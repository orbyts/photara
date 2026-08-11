use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{
    PhotaraError, Result, asset::camera_raw_key, cloud::ADOBE_LIGHTROOM_PROVIDER,
    project::ProjectRecord,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WithdrawalPlan {
    pub schema_version: u32,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub source_key: String,
    pub original_filename: String,
    pub remote_asset_id: String,
    pub remote_filename: String,
    pub current_state: String,
    pub next_step: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PendingWithdrawal {
    pub schema_version: u32,
    pub withdrawal_id: Uuid,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub original_filename: String,
    pub remote_asset_id: String,
    pub remote_filename: String,
    pub state: String,
    pub instructions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerifiedWithdrawal {
    pub schema_version: u32,
    pub withdrawal_id: Uuid,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub source_key: String,
    pub original_filename: String,
    pub remote_asset_id: String,
    pub remote_filename: String,
    pub state: String,
    pub photographer_final_removed: bool,
    pub cloud_presence: String,
    pub keyword_paths_to_remove: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WithdrawalKeywordPlan {
    pub schema_version: u32,
    pub project: String,
    pub verified_count: usize,
    pub keyword_paths_to_remove: Vec<Vec<String>>,
    pub originals: Vec<WithdrawalOriginal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WithdrawalOriginal {
    pub source_key: String,
    pub original_filename: String,
    pub remote_filename: String,
}

pub async fn plan(
    database: &Database,
    project: &ProjectRecord,
    account_label: &str,
    original: &Path,
) -> Result<WithdrawalPlan> {
    let source_key = camera_raw_key(original)?;
    let row = sqlx::query(
        "SELECT asset.original_filename, presence.remote_asset_id, inventory.file_name, \
                withdrawal.id AS withdrawal_id \
         FROM cloud_accounts AS account \
         JOIN asset_cloud_presence AS presence ON presence.account_id = account.id \
         JOIN assets AS asset ON asset.id = presence.asset_id \
         JOIN project_assets AS membership \
           ON membership.project_id = $3 AND membership.asset_id = asset.id \
         JOIN asset_files AS raw \
           ON raw.asset_id = asset.id AND raw.representation = 'camera-raw' \
          AND raw.state = 'current' AND raw.location = $4 \
         JOIN LATERAL ( \
             SELECT id FROM cloud_provider_inventory_runs \
             WHERE account_id = account.id ORDER BY completed_at DESC, id DESC LIMIT 1 \
         ) AS run ON true \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = run.id AND inventory.remote_asset_id = presence.remote_asset_id \
         LEFT JOIN cloud_asset_withdrawals AS withdrawal \
           ON withdrawal.account_id = account.id AND withdrawal.asset_id = asset.id \
          AND withdrawal.state = 'awaiting-user-deletion' \
         WHERE account.provider = $1 AND account.label = $2 \
           AND presence.status = 'present' AND inventory.file_name IS NOT NULL",
    )
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .bind(project.id)
    .bind(&source_key)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "{} is not a verified Cloud-present original in project {:?}",
            original.display(),
            project.slug
        ))
    })?;
    let pending: Option<Uuid> = row.try_get("withdrawal_id")?;
    Ok(WithdrawalPlan {
        schema_version: 1,
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        project: project.slug.clone(),
        source_key,
        original_filename: row.try_get("original_filename")?,
        remote_asset_id: row.try_get("remote_asset_id")?,
        remote_filename: row.try_get("file_name")?,
        current_state: if pending.is_some() {
            "awaiting-user-deletion".into()
        } else {
            "present".into()
        },
        next_step: if let Some(id) = pending {
            format!(
                "delete the exact DNG in Lightroom Desktop, then run cloud verify-withdrawal {id}"
            )
        } else {
            "run cloud begin-withdrawal with --confirm to record the intent before deleting".into()
        },
    })
}

pub async fn begin(
    database: &Database,
    project: &ProjectRecord,
    account_label: &str,
    original: &Path,
    reason: Option<&str>,
) -> Result<PendingWithdrawal> {
    let planned = plan(database, project, account_label, original).await?;
    let reason = reason.map(str::trim).filter(|value| !value.is_empty());
    let id = Uuid::new_v4();
    let row = sqlx::query(
        "WITH target AS ( \
             SELECT account.id AS account_id, presence.asset_id, run.id AS run_id \
             FROM cloud_accounts AS account \
             JOIN asset_cloud_presence AS presence ON presence.account_id = account.id \
             JOIN asset_files AS raw ON raw.asset_id = presence.asset_id \
               AND raw.representation = 'camera-raw' AND raw.state = 'current' \
               AND raw.location = $4 \
             JOIN LATERAL ( \
                 SELECT id FROM cloud_provider_inventory_runs \
                 WHERE account_id = account.id ORDER BY completed_at DESC, id DESC LIMIT 1 \
             ) AS run ON true \
             WHERE account.provider = $1 AND account.label = $2 \
               AND presence.status = 'present' \
         ), inserted AS ( \
             INSERT INTO cloud_asset_withdrawals \
               (id, account_id, project_id, asset_id, remote_asset_id, remote_filename, \
                state, reason, planned_inventory_run_id) \
             SELECT $5, target.account_id, $3, target.asset_id, $6, $7, \
                    'awaiting-user-deletion', $8, target.run_id FROM target \
             ON CONFLICT (account_id, asset_id) \
               WHERE state = 'awaiting-user-deletion' DO NOTHING \
             RETURNING id \
         ) \
         SELECT id FROM inserted \
         UNION ALL \
         SELECT withdrawal.id FROM cloud_asset_withdrawals AS withdrawal \
         JOIN target ON target.account_id = withdrawal.account_id \
                    AND target.asset_id = withdrawal.asset_id \
         WHERE withdrawal.state = 'awaiting-user-deletion' \
         LIMIT 1",
    )
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .bind(project.id)
    .bind(&planned.source_key)
    .bind(id)
    .bind(&planned.remote_asset_id)
    .bind(&planned.remote_filename)
    .bind(reason)
    .fetch_one(database.pool())
    .await?;
    let withdrawal_id: Uuid = row.try_get("id")?;
    Ok(PendingWithdrawal {
        schema_version: 1,
        withdrawal_id,
        provider: planned.provider,
        account_label: planned.account_label,
        project: planned.project,
        original_filename: planned.original_filename,
        remote_asset_id: planned.remote_asset_id,
        remote_filename: planned.remote_filename.clone(),
        state: "awaiting-user-deletion".into(),
        instructions: vec![
            format!(
                "In Lightroom Desktop All Photos, delete only {}",
                planned.remote_filename
            ),
            "Open Deleted and permanently delete that same photo".into(),
            format!(
                "Run photara cloud verify-withdrawal {withdrawal_id} --account {account_label}"
            ),
        ],
    })
}

pub async fn verify(
    database: &Database,
    withdrawal_id: Uuid,
    account_label: &str,
) -> Result<VerifiedWithdrawal> {
    let mut transaction = database.begin().await?;
    let row = sqlx::query(
        "SELECT withdrawal.project_id, withdrawal.asset_id, withdrawal.remote_asset_id, \
                withdrawal.remote_filename, withdrawal.state, withdrawal.requested_at, \
                project.slug, asset.original_filename, raw.location AS source_key, \
                account.id AS account_id \
         FROM cloud_asset_withdrawals AS withdrawal \
         JOIN cloud_accounts AS account ON account.id = withdrawal.account_id \
         JOIN projects AS project ON project.id = withdrawal.project_id \
         JOIN assets AS asset ON asset.id = withdrawal.asset_id \
         JOIN asset_files AS raw ON raw.asset_id = asset.id \
           AND raw.representation = 'camera-raw' AND raw.state = 'current' \
         WHERE withdrawal.id = $1 AND account.provider = $2 AND account.label = $3 \
         FOR UPDATE OF withdrawal",
    )
    .bind(withdrawal_id)
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!("withdrawal {withdrawal_id} was not found"))
    })?;
    let state: String = row.try_get("state")?;
    let account_id: Uuid = row.try_get("account_id")?;
    let asset_id: Uuid = row.try_get("asset_id")?;
    let project_id: Uuid = row.try_get("project_id")?;
    let remote_asset_id: String = row.try_get("remote_asset_id")?;
    let requested_at: DateTime<Utc> = row.try_get("requested_at")?;
    if state == "verified-removed" {
        return verified_from_row(withdrawal_id, account_label, &row, false);
    }
    if state != "awaiting-user-deletion" {
        return Err(PhotaraError::Configuration(format!(
            "withdrawal {withdrawal_id} is {state}, not awaiting verification"
        )));
    }
    let inventory = sqlx::query(
        "SELECT id, completed_at FROM cloud_provider_inventory_runs \
         WHERE account_id = $1 ORDER BY completed_at DESC, id DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_one(&mut *transaction)
    .await?;
    let run_id: Uuid = inventory.try_get("id")?;
    let completed_at: DateTime<Utc> = inventory.try_get("completed_at")?;
    if completed_at < requested_at {
        return Err(PhotaraError::Configuration(
            "the latest Adobe inventory predates this withdrawal; refresh inventory and retry"
                .into(),
        ));
    }
    let still_present: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM cloud_provider_inventory_assets \
         WHERE run_id = $1 AND remote_asset_id = $2)",
    )
    .bind(run_id)
    .bind(&remote_asset_id)
    .fetch_one(&mut *transaction)
    .await?;
    if still_present {
        return Err(PhotaraError::Configuration(format!(
            "Adobe still reports {} ({remote_asset_id}) in Cloud; delete it from All Photos and permanently from Deleted, then retry",
            row.try_get::<String, _>("remote_filename")?
        )));
    }

    sqlx::query(
        "UPDATE asset_cloud_presence SET status = 'removed', evidence_kind = 'provider-api', \
                last_verified_at = now() WHERE account_id = $1 AND asset_id = $2",
    )
    .bind(account_id)
    .bind(asset_id)
    .execute(&mut *transaction)
    .await?;
    let previous: Option<bool> = sqlx::query_scalar(
        "SELECT selected FROM project_asset_decisions \
         WHERE project_id = $1 AND asset_id = $2 AND decision = 'photographer-final' FOR UPDATE",
    )
    .bind(project_id)
    .bind(asset_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let photographer_final_removed = previous == Some(true);
    if photographer_final_removed {
        sqlx::query(
            "UPDATE project_asset_decisions SET selected = false, decided_at = now() \
             WHERE project_id = $1 AND asset_id = $2 AND decision = 'photographer-final'",
        )
        .bind(project_id)
        .bind(asset_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO project_asset_decision_events \
               (project_id, asset_id, decision, selected, source, note) \
             VALUES ($1, $2, 'photographer-final', false, 'cloud-withdrawal', \
                     'Removed after Adobe inventory verified the working DNG absent')",
        )
        .bind(project_id)
        .bind(asset_id)
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query(
        "UPDATE cloud_asset_withdrawals SET state = 'verified-removed', \
                verified_inventory_run_id = $2, verified_at = now() WHERE id = $1",
    )
    .bind(withdrawal_id)
    .bind(run_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    verified_from_row(
        withdrawal_id,
        account_label,
        &row,
        photographer_final_removed,
    )
}

fn verified_from_row(
    withdrawal_id: Uuid,
    account_label: &str,
    row: &sqlx::postgres::PgRow,
    photographer_final_removed: bool,
) -> Result<VerifiedWithdrawal> {
    Ok(VerifiedWithdrawal {
        schema_version: 1,
        withdrawal_id,
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        project: row.try_get("slug")?,
        source_key: row.try_get("source_key")?,
        original_filename: row.try_get("original_filename")?,
        remote_asset_id: row.try_get("remote_asset_id")?,
        remote_filename: row.try_get("remote_filename")?,
        state: "verified-removed".into(),
        photographer_final_removed,
        cloud_presence: "removed".into(),
        keyword_paths_to_remove: keyword_paths(),
    })
}

pub async fn keyword_plan(
    database: &Database,
    project: &ProjectRecord,
    paths: &[PathBuf],
) -> Result<WithdrawalKeywordPlan> {
    if paths.is_empty() {
        return Err(PhotaraError::Configuration(
            "select one or more camera originals".into(),
        ));
    }
    let mut originals = Vec::with_capacity(paths.len());
    for path in paths {
        let source_key = camera_raw_key(path)?;
        let row = sqlx::query(
            "SELECT asset.original_filename, withdrawal.remote_filename \
             FROM cloud_asset_withdrawals AS withdrawal \
             JOIN assets AS asset ON asset.id = withdrawal.asset_id \
             JOIN asset_files AS raw ON raw.asset_id = asset.id \
               AND raw.representation = 'camera-raw' AND raw.state = 'current' \
             WHERE withdrawal.project_id = $1 AND withdrawal.state = 'verified-removed' \
               AND raw.location = $2 \
             ORDER BY withdrawal.verified_at DESC LIMIT 1",
        )
        .bind(project.id)
        .bind(&source_key)
        .fetch_optional(database.pool())
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "{} does not have a provider-verified Cloud withdrawal",
                path.display()
            ))
        })?;
        originals.push(WithdrawalOriginal {
            source_key,
            original_filename: row.try_get("original_filename")?,
            remote_filename: row.try_get("remote_filename")?,
        });
    }
    Ok(WithdrawalKeywordPlan {
        schema_version: 1,
        project: project.slug.clone(),
        verified_count: originals.len(),
        keyword_paths_to_remove: keyword_paths(),
        originals,
    })
}

fn keyword_paths() -> Vec<Vec<String>> {
    vec![
        vec![
            "workflow".into(),
            "selection".into(),
            "photographer-final".into(),
        ],
        vec!["workflow".into(), "cloud".into(), "present".into()],
    ]
}
