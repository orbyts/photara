use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;

use crate::{
    PhotaraError, Result,
    asset::{RegisterOriginal, camera_raw_key, register_original},
    config::PhotaraConfig,
    metadata::KeywordPath,
    project::ProjectRecord,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionValue {
    Selected,
    Rejected,
}

impl DecisionValue {
    fn selected(self) -> bool {
        self == Self::Selected
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionReport {
    pub schema_version: u32,
    pub project: String,
    pub decision: String,
    pub selected: bool,
    pub affected_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub keyword_path: Vec<String>,
    pub originals: Vec<DecisionOriginal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionHistory {
    pub schema_version: u32,
    pub project: String,
    pub decision: String,
    pub event_count: usize,
    pub events: Vec<DecisionEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionEvent {
    pub original_filename: String,
    pub selected: bool,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionOriginal {
    pub source_key: String,
    pub original_path: String,
    pub original_filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionPlan {
    pub schema_version: u32,
    pub project: String,
    pub decision: String,
    pub selected_count: usize,
    pub keyword: KeywordPath,
    pub source_keys: Vec<String>,
}

pub async fn update(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    value: DecisionValue,
    paths: &[PathBuf],
) -> Result<DecisionReport> {
    if paths.is_empty() {
        return Err(PhotaraError::Configuration(
            "select one or more camera originals".into(),
        ));
    }
    let mut originals = Vec::with_capacity(paths.len());
    let mut asset_ids = Vec::with_capacity(paths.len());
    for path in paths {
        if !path.is_file() {
            return Err(PhotaraError::Configuration(format!(
                "camera original {} is not a file",
                path.display()
            )));
        }
        let metadata = path.metadata().map_err(|source| {
            PhotaraError::filesystem("read camera original metadata", path, source)
        })?;
        let record = register_original(
            database,
            RegisterOriginal {
                project_id: project.id,
                original_path: path.clone(),
                capture_date: capture_date(path)?,
                author_code: config.settings.default_author_code.clone(),
                sha256: sha256(path)?,
                byte_size: i64::try_from(metadata.len()).ok(),
            },
        )
        .await?;
        asset_ids.push(record.id);
        originals.push(DecisionOriginal {
            source_key: camera_raw_key(path)?,
            original_path: path.to_string_lossy().into_owned(),
            original_filename: record.original_filename,
        });
    }

    let mut transaction = database.begin().await?;
    let mut changed_count = 0;
    for asset_id in asset_ids {
        let previous: Option<bool> = sqlx::query_scalar(
            "SELECT selected FROM project_asset_decisions \
             WHERE project_id = $1 AND asset_id = $2 AND decision = 'photographer-final' \
             FOR UPDATE",
        )
        .bind(project.id)
        .bind(asset_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if previous == Some(value.selected()) {
            continue;
        }
        sqlx::query(
            "INSERT INTO project_asset_decisions \
             (project_id, asset_id, decision, selected, decided_at) \
             VALUES ($1, $2, 'photographer-final', $3, now()) \
             ON CONFLICT (project_id, asset_id, decision) DO UPDATE SET \
               selected = EXCLUDED.selected, decided_at = EXCLUDED.decided_at",
        )
        .bind(project.id)
        .bind(asset_id)
        .bind(value.selected())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO project_asset_decision_events \
             (project_id, asset_id, decision, selected, source) \
             VALUES ($1, $2, 'photographer-final', $3, 'operator-command')",
        )
        .bind(project.id)
        .bind(asset_id)
        .bind(value.selected())
        .execute(&mut *transaction)
        .await?;
        changed_count += 1;
    }
    transaction.commit().await?;

    Ok(DecisionReport {
        schema_version: 1,
        project: project.slug.clone(),
        decision: "photographer-final".into(),
        selected: value.selected(),
        affected_count: originals.len(),
        changed_count,
        unchanged_count: originals.len() - changed_count,
        keyword_path: keyword().path,
        originals,
    })
}

pub async fn history(database: &Database, project: &ProjectRecord) -> Result<DecisionHistory> {
    let events = sqlx::query(
        "SELECT asset.original_filename, event.selected, event.source, event.note, \
                event.changed_at \
         FROM project_asset_decision_events AS event \
         JOIN assets AS asset ON asset.id = event.asset_id \
         WHERE event.project_id = $1 AND event.decision = 'photographer-final' \
         ORDER BY event.changed_at, event.id",
    )
    .bind(project.id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(DecisionEvent {
            original_filename: row.try_get("original_filename")?,
            selected: row.try_get("selected")?,
            source: row.try_get("source")?,
            note: row.try_get("note")?,
            changed_at: row.try_get("changed_at")?,
        })
    })
    .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    Ok(DecisionHistory {
        schema_version: 1,
        project: project.slug.clone(),
        decision: "photographer-final".into(),
        event_count: events.len(),
        events,
    })
}

pub async fn plan(database: &Database, project: &ProjectRecord) -> Result<DecisionPlan> {
    let rows = sqlx::query(
        "SELECT file.location FROM project_asset_decisions AS decision \
         JOIN asset_files AS file ON file.asset_id = decision.asset_id \
           AND file.representation = 'camera-raw' AND file.state = 'current' \
         WHERE decision.project_id = $1 AND decision.decision = 'photographer-final' \
           AND decision.selected \
         ORDER BY file.location",
    )
    .bind(project.id)
    .fetch_all(database.pool())
    .await?;
    let source_keys = rows
        .into_iter()
        .map(|row| row.try_get("location"))
        .collect::<std::result::Result<Vec<String>, sqlx::Error>>()?;
    Ok(DecisionPlan {
        schema_version: 1,
        project: project.slug.clone(),
        decision: "photographer-final".into(),
        selected_count: source_keys.len(),
        keyword: keyword(),
        source_keys,
    })
}

fn keyword() -> KeywordPath {
    KeywordPath {
        path: vec![
            "workflow".into(),
            "selection".into(),
            "photographer-final".into(),
        ],
    }
}

fn capture_date(path: &Path) -> Result<NaiveDate> {
    let date = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "camera original {} has no dated parent folder",
                path.display()
            ))
        })?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
        PhotaraError::Configuration(format!(
            "camera original {} is not under a YYYY-MM-DD folder",
            path.display()
        ))
    })
}

fn sha256(path: &Path) -> Result<String> {
    let file = File::open(path)
        .map_err(|source| PhotaraError::filesystem("open camera original", path, source))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| PhotaraError::filesystem("hash camera original", path, source))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_date_comes_from_canonical_archive_folder() {
        assert_eq!(
            capture_date(Path::new("/Images/2021/2021-06/2021-06-11/DSC05181.ARW")).unwrap(),
            NaiveDate::from_ymd_opt(2021, 6, 11).unwrap()
        );
    }
}
