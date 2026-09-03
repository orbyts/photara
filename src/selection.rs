use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{PhotaraError, Result, metadata::KeywordPath, project::ProjectRecord};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionKind {
    ClientFavorite,
    ClientShortlist,
    Hero,
}

impl SelectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClientFavorite => "client-favorite",
            Self::ClientShortlist => "client-shortlist",
            Self::Hero => "hero",
        }
    }

    fn pixieset_name(self) -> &'static str {
        match self {
            Self::ClientFavorite => "Client Favorites",
            Self::ClientShortlist => "Client Shortlist",
            Self::Hero => "Hero",
        }
    }

    fn keyword(self) -> KeywordPath {
        KeywordPath {
            path: vec!["workflow".into(), "selection".into(), self.as_str().into()],
        }
    }
}

impl FromStr for SelectionKind {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "client-favorite" => Ok(Self::ClientFavorite),
            "client-shortlist" => Ok(Self::ClientShortlist),
            "hero" => Ok(Self::Hero),
            _ => Err(format!(
                "unknown selection kind {value:?}; expected client-favorite, client-shortlist, or hero"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionAction {
    Add,
    Remove,
}

impl SelectionAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SelectionSource {
    pub kind: SelectionKind,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImportReport {
    pub project: String,
    pub provider: String,
    pub direct_counts: BTreeMap<String, usize>,
    pub effective_counts: BTreeMap<String, usize>,
    pub source_checksums: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionPlan {
    pub schema_version: u32,
    pub project: SelectionProject,
    pub managed_keywords: Vec<KeywordPath>,
    pub assignments: Vec<SelectionAssignment>,
    pub direct_counts: BTreeMap<String, usize>,
    pub effective_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionProject {
    pub id: String,
    pub slug: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionAssignment {
    pub original_filename: String,
    pub keywords: Vec<KeywordPath>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionCorrectionReport {
    pub schema_version: u32,
    pub project: String,
    pub asset: SelectionAsset,
    pub action: SelectionAction,
    pub requested_kind: SelectionKind,
    pub cascade: bool,
    pub dry_run: bool,
    pub changed_kinds: Vec<SelectionKind>,
    pub direct_before: Vec<SelectionKind>,
    pub direct_after: Vec<SelectionKind>,
    pub effective_before: Vec<SelectionKind>,
    pub effective_after: Vec<SelectionKind>,
}

#[derive(Clone, Copy, Debug)]
pub struct SelectionCorrection<'a> {
    pub asset_reference: &'a str,
    pub kind: SelectionKind,
    pub action: SelectionAction,
    pub reason: &'a str,
    pub cascade: bool,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionStatus {
    pub schema_version: u32,
    pub project: String,
    pub asset: SelectionAsset,
    pub provider_direct: Vec<SelectionKind>,
    pub overrides: Vec<SelectionOverride>,
    pub direct: Vec<SelectionKind>,
    pub effective: Vec<SelectionKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionHistory {
    pub schema_version: u32,
    pub project: String,
    pub asset: SelectionAsset,
    pub events: Vec<SelectionOverrideEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionAsset {
    pub id: Uuid,
    pub source_key: String,
    pub original_filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionOverride {
    pub kind: SelectionKind,
    pub action: SelectionAction,
    pub reason: String,
    pub source: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectionOverrideEvent {
    pub kind: SelectionKind,
    pub action: SelectionAction,
    pub reason: String,
    pub source: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug)]
struct ParsedSelection {
    kind: SelectionKind,
    source_name: String,
    source_sha256: String,
    source_contents: String,
    collection_name: String,
    favorite_name: String,
    client_email: Option<String>,
    entries: Vec<ParsedEntry>,
}

#[derive(Debug)]
struct ParsedEntry {
    proof_filename: String,
    original_filename: String,
    note: Option<String>,
    photo_set: Option<String>,
    provider_created_at: Option<String>,
}

pub async fn import_pixieset(
    database: &Database,
    project: &ProjectRecord,
    source_root: &Path,
    sources: &[SelectionSource],
) -> Result<ImportReport> {
    let expected = BTreeSet::from([
        SelectionKind::ClientFavorite,
        SelectionKind::ClientShortlist,
        SelectionKind::Hero,
    ]);
    let supplied: BTreeSet<_> = sources.iter().map(|source| source.kind).collect();
    if supplied != expected || sources.len() != expected.len() {
        return Err(PhotaraError::Configuration(
            "Pixieset import requires exactly one Client Favorites, Client Shortlist, and Hero CSV"
                .into(),
        ));
    }

    let raw_by_stem = raw_manifest(source_root)?;
    let mut parsed = Vec::with_capacity(sources.len());
    for source in sources {
        parsed.push(parse_pixieset(source, project, &raw_by_stem)?);
    }

    let mut transaction = database.begin().await?;
    for selection in &parsed {
        let import_id: Uuid = sqlx::query_scalar(
            "INSERT INTO selection_imports \
             (id, project_id, provider, selection_kind, source_name, source_sha256, \
              collection_name, favorite_name, client_email, source_contents) \
             VALUES ($1, $2, 'pixieset', $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (project_id, provider, selection_kind, source_sha256) \
             DO UPDATE SET source_name = EXCLUDED.source_name \
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(project.id)
        .bind(selection.kind.as_str())
        .bind(&selection.source_name)
        .bind(&selection.source_sha256)
        .bind(&selection.collection_name)
        .bind(&selection.favorite_name)
        .bind(&selection.client_email)
        .bind(&selection.source_contents)
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query("DELETE FROM selection_import_entries WHERE import_id = $1")
            .bind(import_id)
            .execute(&mut *transaction)
            .await?;
        for entry in &selection.entries {
            sqlx::query(
                "INSERT INTO selection_import_entries \
                 (import_id, proof_filename, original_filename, note, photo_set, provider_created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(import_id)
            .bind(&entry.proof_filename)
            .bind(&entry.original_filename)
            .bind(&entry.note)
            .bind(&entry.photo_set)
            .bind(&entry.provider_created_at)
            .execute(&mut *transaction)
            .await?;
        }

        sqlx::query(
            "DELETE FROM project_selection_memberships \
             WHERE project_id = $1 AND selection_kind = $2",
        )
        .bind(project.id)
        .bind(selection.kind.as_str())
        .execute(&mut *transaction)
        .await?;
        for entry in &selection.entries {
            sqlx::query(
                "INSERT INTO project_selection_memberships \
                 (project_id, original_filename, selection_kind, import_id) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(project.id)
            .bind(&entry.original_filename)
            .bind(selection.kind.as_str())
            .bind(import_id)
            .execute(&mut *transaction)
            .await?;
        }
    }
    transaction.commit().await?;

    let direct = database_direct_memberships(database, project.id).await?;
    let effective = effective_memberships(&direct);
    Ok(ImportReport {
        project: project.slug.clone(),
        provider: "pixieset".into(),
        direct_counts: counts(&direct),
        effective_counts: counts(&effective),
        source_checksums: parsed
            .iter()
            .map(|selection| {
                (
                    selection.kind.as_str().to_owned(),
                    selection.source_sha256.clone(),
                )
            })
            .collect(),
    })
}

pub async fn plan(database: &Database, project: &ProjectRecord) -> Result<SelectionPlan> {
    let direct = database_direct_memberships(database, project.id).await?;
    let effective = effective_memberships(&direct);
    let direct_counts = counts(&direct);
    let mut by_filename: BTreeMap<String, Vec<KeywordPath>> = BTreeMap::new();
    for (kind, filenames) in &effective {
        for filename in filenames {
            by_filename
                .entry(filename.clone())
                .or_default()
                .push(kind.keyword());
        }
    }
    Ok(SelectionPlan {
        schema_version: 1,
        project: SelectionProject {
            id: project.id.to_string(),
            slug: project.slug.clone(),
            display_name: project.display_name.clone(),
        },
        managed_keywords: vec![
            SelectionKind::ClientFavorite.keyword(),
            SelectionKind::ClientShortlist.keyword(),
            SelectionKind::Hero.keyword(),
        ],
        assignments: by_filename
            .into_iter()
            .map(|(original_filename, keywords)| SelectionAssignment {
                original_filename,
                keywords,
            })
            .collect(),
        direct_counts,
        effective_counts: counts(&effective),
    })
}

pub async fn correct(
    database: &Database,
    project: &ProjectRecord,
    correction: SelectionCorrection<'_>,
) -> Result<SelectionCorrectionReport> {
    let reason = correction.reason.trim();
    if reason.is_empty() {
        return Err(PhotaraError::Configuration(
            "selection correction requires a non-empty reason".into(),
        ));
    }
    if correction.action == SelectionAction::Add && correction.cascade {
        return Err(PhotaraError::Configuration(
            "--cascade applies only to selection removal".into(),
        ));
    }
    let asset = resolve_asset(database, project.id, correction.asset_reference).await?;
    let direct_before_map = database_direct_memberships(database, project.id).await?;
    let effective_before_map = effective_memberships(&direct_before_map);
    let direct_before = kinds_for(&direct_before_map, &asset.original_filename);
    let effective_before = kinds_for(&effective_before_map, &asset.original_filename);

    if correction.action == SelectionAction::Remove && !correction.cascade {
        let blocking = match correction.kind {
            SelectionKind::ClientFavorite => effective_before
                .iter()
                .copied()
                .filter(|kind| *kind != SelectionKind::ClientFavorite)
                .collect::<Vec<_>>(),
            SelectionKind::ClientShortlist => effective_before
                .iter()
                .copied()
                .filter(|kind| *kind == SelectionKind::Hero)
                .collect::<Vec<_>>(),
            SelectionKind::Hero => Vec::new(),
        };
        if !blocking.is_empty() {
            return Err(PhotaraError::Configuration(format!(
                "cannot remove {} while {} remains effective; retry with --cascade",
                correction.kind.as_str(),
                blocking
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    let affected_kinds = affected_kinds(correction.kind, correction.action, correction.cascade);
    let mut direct_after_map = direct_before_map.clone();
    let mut changed_kinds = Vec::new();
    for kind in &affected_kinds {
        let members = direct_after_map.entry(*kind).or_default();
        let changed = match correction.action {
            SelectionAction::Add => members.insert(asset.original_filename.clone()),
            SelectionAction::Remove => members.remove(&asset.original_filename),
        };
        if changed {
            changed_kinds.push(*kind);
        }
    }
    let effective_after_map = effective_memberships(&direct_after_map);

    if !correction.dry_run && !changed_kinds.is_empty() {
        let mut transaction = database.begin().await?;
        for kind in &changed_kinds {
            sqlx::query(
                "INSERT INTO project_selection_overrides \
                 (project_id, asset_id, selection_kind, action, reason, source, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, 'operator-command', now()) \
                 ON CONFLICT (project_id, asset_id, selection_kind) DO UPDATE SET \
                   action = EXCLUDED.action, reason = EXCLUDED.reason, \
                   source = EXCLUDED.source, updated_at = now()",
            )
            .bind(project.id)
            .bind(asset.id)
            .bind(kind.as_str())
            .bind(correction.action.as_str())
            .bind(reason)
            .execute(&mut *transaction)
            .await?;
            sqlx::query(
                "INSERT INTO project_selection_override_events \
                 (project_id, asset_id, selection_kind, action, reason, source) \
                 VALUES ($1, $2, $3, $4, $5, 'operator-command')",
            )
            .bind(project.id)
            .bind(asset.id)
            .bind(kind.as_str())
            .bind(correction.action.as_str())
            .bind(reason)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
    }

    let direct_after = kinds_for(&direct_after_map, &asset.original_filename);
    let effective_after = kinds_for(&effective_after_map, &asset.original_filename);
    Ok(SelectionCorrectionReport {
        schema_version: 1,
        project: project.slug.clone(),
        asset,
        action: correction.action,
        requested_kind: correction.kind,
        cascade: correction.cascade,
        dry_run: correction.dry_run,
        changed_kinds,
        direct_before,
        direct_after,
        effective_before,
        effective_after,
    })
}

pub async fn status(
    database: &Database,
    project: &ProjectRecord,
    asset_reference: &str,
) -> Result<SelectionStatus> {
    let asset = resolve_asset(database, project.id, asset_reference).await?;
    let provider = provider_direct_memberships(database, project.id).await?;
    let direct = database_direct_memberships(database, project.id).await?;
    let effective = effective_memberships(&direct);
    let overrides = load_overrides(database, project.id, Some(asset.id)).await?;
    Ok(SelectionStatus {
        schema_version: 1,
        project: project.slug.clone(),
        provider_direct: kinds_for(&provider, &asset.original_filename),
        direct: kinds_for(&direct, &asset.original_filename),
        effective: kinds_for(&effective, &asset.original_filename),
        overrides,
        asset,
    })
}

pub async fn history(
    database: &Database,
    project: &ProjectRecord,
    asset_reference: &str,
) -> Result<SelectionHistory> {
    let asset = resolve_asset(database, project.id, asset_reference).await?;
    let events = sqlx::query(
        "SELECT selection_kind, action, reason, source, changed_at \
         FROM project_selection_override_events \
         WHERE project_id = $1 AND asset_id = $2 ORDER BY changed_at, id",
    )
    .bind(project.id)
    .bind(asset.id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(SelectionOverrideEvent {
            kind: parse_kind(row.try_get("selection_kind")?)?,
            action: parse_action(row.try_get("action")?)?,
            reason: row.try_get("reason")?,
            source: row.try_get("source")?,
            changed_at: row.try_get("changed_at")?,
        })
    })
    .collect::<Result<Vec<_>>>()?;
    Ok(SelectionHistory {
        schema_version: 1,
        project: project.slug.clone(),
        asset,
        events,
    })
}

async fn resolve_asset(
    database: &Database,
    project_id: Uuid,
    asset_reference: &str,
) -> Result<SelectionAsset> {
    let asset_reference = asset_reference.trim();
    if asset_reference.is_empty() {
        return Err(PhotaraError::Configuration(
            "selection correction requires an asset filename or source key".into(),
        ));
    }
    let rows = sqlx::query(
        "SELECT DISTINCT asset.id, asset.original_filename, file.location \
         FROM project_assets AS membership \
         JOIN assets AS asset ON asset.id = membership.asset_id \
         JOIN asset_files AS file ON file.asset_id = asset.id \
           AND file.representation = 'camera-raw' AND file.state = 'current' \
         WHERE membership.project_id = $1 \
           AND (file.location = $2 OR lower(asset.original_filename) = lower($2)) \
         ORDER BY file.location",
    )
    .bind(project_id)
    .bind(asset_reference)
    .fetch_all(database.pool())
    .await?;
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project asset {asset_reference:?} was not found"
        )));
    }
    if rows.len() > 1 {
        return Err(PhotaraError::Configuration(format!(
            "project asset filename {asset_reference:?} is ambiguous; use its canonical source key"
        )));
    }
    let row = &rows[0];
    Ok(SelectionAsset {
        id: row.try_get("id")?,
        original_filename: row.try_get("original_filename")?,
        source_key: row.try_get("location")?,
    })
}

async fn provider_direct_memberships(
    database: &Database,
    project_id: Uuid,
) -> Result<BTreeMap<SelectionKind, BTreeSet<String>>> {
    let rows = sqlx::query(
        "SELECT original_filename, selection_kind FROM project_selection_memberships \
         WHERE project_id = $1 ORDER BY original_filename, selection_kind",
    )
    .bind(project_id)
    .fetch_all(database.pool())
    .await?;
    let mut direct = BTreeMap::from([
        (SelectionKind::ClientFavorite, BTreeSet::new()),
        (SelectionKind::ClientShortlist, BTreeSet::new()),
        (SelectionKind::Hero, BTreeSet::new()),
    ]);
    for row in rows {
        direct
            .entry(parse_kind(row.try_get("selection_kind")?)?)
            .or_insert_with(BTreeSet::new)
            .insert(row.try_get("original_filename")?);
    }
    Ok(direct)
}

async fn database_direct_memberships(
    database: &Database,
    project_id: Uuid,
) -> Result<BTreeMap<SelectionKind, BTreeSet<String>>> {
    let mut direct = provider_direct_memberships(database, project_id).await?;
    let rows = sqlx::query(
        "SELECT asset.original_filename, correction.selection_kind, correction.action \
         FROM project_selection_overrides AS correction \
         JOIN assets AS asset ON asset.id = correction.asset_id \
         WHERE correction.project_id = $1 \
         ORDER BY asset.original_filename, correction.selection_kind",
    )
    .bind(project_id)
    .fetch_all(database.pool())
    .await?;
    for row in rows {
        let filename: String = row.try_get("original_filename")?;
        let kind = parse_kind(row.try_get("selection_kind")?)?;
        match parse_action(row.try_get("action")?)? {
            SelectionAction::Add => {
                direct.entry(kind).or_default().insert(filename);
            }
            SelectionAction::Remove => {
                direct.entry(kind).or_default().remove(&filename);
            }
        }
    }
    Ok(direct)
}

async fn load_overrides(
    database: &Database,
    project_id: Uuid,
    asset_id: Option<Uuid>,
) -> Result<Vec<SelectionOverride>> {
    sqlx::query(
        "SELECT selection_kind, action, reason, source, updated_at \
         FROM project_selection_overrides \
         WHERE project_id = $1 AND ($2::uuid IS NULL OR asset_id = $2) \
         ORDER BY selection_kind",
    )
    .bind(project_id)
    .bind(asset_id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(SelectionOverride {
            kind: parse_kind(row.try_get("selection_kind")?)?,
            action: parse_action(row.try_get("action")?)?,
            reason: row.try_get("reason")?,
            source: row.try_get("source")?,
            updated_at: row.try_get("updated_at")?,
        })
    })
    .collect()
}

fn affected_kinds(
    requested: SelectionKind,
    action: SelectionAction,
    cascade: bool,
) -> Vec<SelectionKind> {
    if action != SelectionAction::Remove || !cascade {
        return vec![requested];
    }
    match requested {
        SelectionKind::ClientFavorite => vec![
            SelectionKind::ClientFavorite,
            SelectionKind::ClientShortlist,
            SelectionKind::Hero,
        ],
        SelectionKind::ClientShortlist => {
            vec![SelectionKind::ClientShortlist, SelectionKind::Hero]
        }
        SelectionKind::Hero => vec![SelectionKind::Hero],
    }
}

fn kinds_for(
    memberships: &BTreeMap<SelectionKind, BTreeSet<String>>,
    filename: &str,
) -> Vec<SelectionKind> {
    [
        SelectionKind::ClientFavorite,
        SelectionKind::ClientShortlist,
        SelectionKind::Hero,
    ]
    .into_iter()
    .filter(|kind| {
        memberships
            .get(kind)
            .is_some_and(|members| members.contains(filename))
    })
    .collect()
}

fn parse_action(value: &str) -> Result<SelectionAction> {
    match value {
        "add" => Ok(SelectionAction::Add),
        "remove" => Ok(SelectionAction::Remove),
        _ => Err(PhotaraError::Configuration(format!(
            "unknown selection override action {value:?}"
        ))),
    }
}

fn parse_pixieset(
    source: &SelectionSource,
    project: &ProjectRecord,
    raw_by_stem: &BTreeMap<String, String>,
) -> Result<ParsedSelection> {
    let bytes = fs::read(&source.path)
        .map_err(|error| PhotaraError::filesystem("read selection CSV", &source.path, error))?;
    let contents = String::from_utf8(bytes.clone()).map_err(|_| {
        PhotaraError::Configuration(format!(
            "selection CSV {} is not UTF-8",
            source.path.display()
        ))
    })?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes.as_slice());
    let mut records = reader.records();
    let metadata = records.next().transpose()?.ok_or_else(|| {
        PhotaraError::Configuration(format!("selection CSV {} is empty", source.path.display()))
    })?;
    let collection_name = prefixed(metadata.get(0), "Collection: ")?;
    let favorite_name = prefixed(metadata.get(1), "Favorite: ")?;
    let client_email = optional_prefixed(metadata.get(2), "Email ");
    if collection_name != project.display_name {
        return Err(PhotaraError::Configuration(format!(
            "{} belongs to Pixieset collection {:?}, expected {:?}",
            source.path.display(),
            collection_name,
            project.display_name
        )));
    }
    if favorite_name != source.kind.pixieset_name() {
        return Err(PhotaraError::Configuration(format!(
            "{} contains favorite list {:?}, expected {:?}",
            source.path.display(),
            favorite_name,
            source.kind.pixieset_name()
        )));
    }
    let header = records.next().transpose()?.ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "selection CSV {} has no header",
            source.path.display()
        ))
    })?;
    if header.iter().collect::<Vec<_>>() != ["Name", "Note", "Photo Set", "Created at"] {
        return Err(PhotaraError::Configuration(format!(
            "selection CSV {} has an unsupported header",
            source.path.display()
        )));
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for record in records {
        let record = record?;
        let proof_filename = record.get(0).unwrap_or_default().trim().to_owned();
        if proof_filename.is_empty() {
            continue;
        }
        if !seen.insert(proof_filename.clone()) {
            return Err(PhotaraError::Configuration(format!(
                "{} contains duplicate proof filename {:?}",
                source.path.display(),
                proof_filename
            )));
        }
        let stem = Path::new(&proof_filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                PhotaraError::Configuration(format!("invalid proof filename {proof_filename:?}"))
            })?;
        let original_filename = raw_by_stem.get(stem).ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "proof {proof_filename:?} has no unique RAW in {}",
                source.path.display()
            ))
        })?;
        entries.push(ParsedEntry {
            proof_filename,
            original_filename: original_filename.clone(),
            note: optional(record.get(1)),
            photo_set: optional(record.get(2)),
            provider_created_at: optional(record.get(3)),
        });
    }
    Ok(ParsedSelection {
        kind: source.kind,
        source_name: source
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("selection.csv")
            .to_owned(),
        source_sha256: format!("{:x}", Sha256::digest(&bytes)),
        source_contents: contents,
        collection_name,
        favorite_name,
        client_email,
        entries,
    })
}

fn raw_manifest(source_root: &Path) -> Result<BTreeMap<String, String>> {
    let entries = fs::read_dir(source_root).map_err(|error| {
        PhotaraError::filesystem("read RAW source directory", source_root, error)
    })?;
    let mut manifest = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PhotaraError::filesystem("read RAW source directory entry", source_root, error)
        })?;
        let path = entry.path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("arw"))
        {
            continue;
        }
        let stem = path.file_stem().and_then(|value| value.to_str()).unwrap();
        let filename = path.file_name().and_then(|value| value.to_str()).unwrap();
        if manifest
            .insert(stem.to_owned(), filename.to_owned())
            .is_some()
        {
            return Err(PhotaraError::Configuration(format!(
                "RAW source {} contains duplicate stem {:?}",
                source_root.display(),
                stem
            )));
        }
    }
    if manifest.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "RAW source {} contains no ARW files",
            source_root.display()
        )));
    }
    Ok(manifest)
}

fn effective_memberships(
    direct: &BTreeMap<SelectionKind, BTreeSet<String>>,
) -> BTreeMap<SelectionKind, BTreeSet<String>> {
    let heroes = direct
        .get(&SelectionKind::Hero)
        .cloned()
        .unwrap_or_default();
    let mut shortlist = direct
        .get(&SelectionKind::ClientShortlist)
        .cloned()
        .unwrap_or_default();
    shortlist.extend(heroes.iter().cloned());
    let mut favorites = direct
        .get(&SelectionKind::ClientFavorite)
        .cloned()
        .unwrap_or_default();
    favorites.extend(shortlist.iter().cloned());
    BTreeMap::from([
        (SelectionKind::ClientFavorite, favorites),
        (SelectionKind::ClientShortlist, shortlist),
        (SelectionKind::Hero, heroes),
    ])
}

fn counts(values: &BTreeMap<SelectionKind, BTreeSet<String>>) -> BTreeMap<String, usize> {
    values
        .iter()
        .map(|(kind, filenames)| (kind.as_str().to_owned(), filenames.len()))
        .collect()
}

fn parse_kind(value: &str) -> Result<SelectionKind> {
    match value {
        "client-favorite" => Ok(SelectionKind::ClientFavorite),
        "client-shortlist" => Ok(SelectionKind::ClientShortlist),
        "hero" => Ok(SelectionKind::Hero),
        _ => Err(PhotaraError::Configuration(format!(
            "unknown selection kind {value:?}"
        ))),
    }
}

fn prefixed(value: Option<&str>, prefix: &str) -> Result<String> {
    value
        .and_then(|value| value.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| PhotaraError::Configuration(format!("missing CSV metadata {prefix:?}")))
}

fn optional_prefixed(value: Option<&str>, prefix: &str) -> Option<String> {
    value
        .and_then(|value| value.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_implies_shortlist_and_favorite() {
        let direct = BTreeMap::from([
            (
                SelectionKind::ClientFavorite,
                BTreeSet::from(["A.ARW".to_owned()]),
            ),
            (
                SelectionKind::ClientShortlist,
                BTreeSet::from(["B.ARW".to_owned()]),
            ),
            (SelectionKind::Hero, BTreeSet::from(["C.ARW".to_owned()])),
        ]);
        let effective = effective_memberships(&direct);
        assert_eq!(effective[&SelectionKind::Hero].len(), 1);
        assert_eq!(effective[&SelectionKind::ClientShortlist].len(), 2);
        assert_eq!(effective[&SelectionKind::ClientFavorite].len(), 3);
    }

    #[test]
    fn cascade_removal_preserves_the_selection_hierarchy() {
        assert_eq!(
            affected_kinds(SelectionKind::ClientFavorite, SelectionAction::Remove, true),
            vec![
                SelectionKind::ClientFavorite,
                SelectionKind::ClientShortlist,
                SelectionKind::Hero,
            ]
        );
        assert_eq!(
            affected_kinds(
                SelectionKind::ClientShortlist,
                SelectionAction::Remove,
                true
            ),
            vec![SelectionKind::ClientShortlist, SelectionKind::Hero]
        );
        assert_eq!(
            affected_kinds(SelectionKind::Hero, SelectionAction::Remove, true),
            vec![SelectionKind::Hero]
        );
    }

    #[test]
    fn additions_need_only_the_requested_direct_membership() {
        assert_eq!(
            affected_kinds(SelectionKind::Hero, SelectionAction::Add, false),
            vec![SelectionKind::Hero]
        );
    }
}
