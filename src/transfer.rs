use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use chrono::NaiveDate;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{
    PhotaraError, Result, asset::downstream_basename, cloud::ADOBE_LIGHTROOM_PROVIDER,
    project::ProjectRecord,
};

#[derive(Clone, Debug)]
struct SelectedAsset {
    id: Uuid,
    source_key: String,
    original_filename: String,
    original_stem: String,
    capture_date: NaiveDate,
    author_code: String,
    original_sha256: String,
}

#[derive(Clone, Debug)]
struct ExistingPresence {
    remote_asset_id: String,
    remote_filename: String,
    evidence_import_id: Option<Uuid>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferPlan {
    pub schema_version: u32,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub inventory_snapshot_sha256: String,
    pub manifest_sha256: String,
    pub photographer_final_count: usize,
    pub planned_count: usize,
    pub skipped_already_present_count: usize,
    pub items: Vec<TransferPlanItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferPlanItem {
    pub asset_id: Uuid,
    pub source_key: String,
    pub original_filename: String,
    pub planned_filename: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_asset_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferReservation {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub manifest_sha256: String,
    pub state: String,
    pub expected_upload_count: usize,
    pub skipped_already_present_count: usize,
    pub reused_existing_batch: bool,
    pub no_transfer_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportBatch {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub state: String,
    pub staging_directory: PathBuf,
    pub pending_count: usize,
    pub exported_count: usize,
    pub items: Vec<ExportBatchItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportBatchItem {
    pub asset_id: Uuid,
    pub source_key: String,
    pub planned_filename: String,
    pub state: String,
    pub staged_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportRecord {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub asset_id: Uuid,
    pub planned_filename: String,
    pub sha256: String,
    pub byte_size: u64,
    pub state: String,
    pub reused_existing_record: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportCompletion {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub state: String,
    pub exported_count: usize,
    pub skipped_already_present_count: usize,
    pub staging_directory: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UploadRequirements {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub state: String,
    pub upload_count: usize,
    pub skipped_already_present_count: usize,
    pub required_bytes: u64,
    pub canary_asset_id: Uuid,
    pub canary_filename: String,
}

#[derive(Clone, Debug)]
pub struct CanaryUpload {
    pub batch_id: Uuid,
    pub asset_id: Uuid,
    pub remote_asset_id: String,
    pub filename: String,
    pub staged_path: PathBuf,
    pub sha256: String,
    pub byte_size: u64,
    pub capture_date: NaiveDate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanaryVerification {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub asset_id: Uuid,
    pub remote_asset_id: String,
    pub filename: String,
    pub sha256: String,
    pub state: String,
    pub remaining_exported_count: usize,
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema_version: u32,
    provider: &'a str,
    account_label: &'a str,
    project: &'a str,
    inventory_snapshot_sha256: &'a str,
    items: &'a [TransferPlanItem],
}

pub async fn plan(
    database: &Database,
    project: &ProjectRecord,
    account_label: &str,
) -> Result<TransferPlan> {
    let account_id: Uuid =
        sqlx::query_scalar("SELECT id FROM cloud_accounts WHERE provider = $1 AND label = $2")
            .bind(ADOBE_LIGHTROOM_PROVIDER)
            .bind(account_label)
            .fetch_optional(database.pool())
            .await?
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "Adobe Lightroom Cloud account {account_label:?} was not found"
                ))
            })?;
    let inventory = sqlx::query(
        "SELECT id, snapshot_sha256, asset_count FROM cloud_provider_inventory_runs \
         WHERE account_id = $1 ORDER BY completed_at DESC, id DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "Adobe Lightroom Cloud account {account_label:?} has no inventory"
        ))
    })?;
    let run_id: Uuid = inventory.try_get("id")?;
    let inventory_count: i32 = inventory.try_get("asset_count")?;
    let actual_inventory_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_provider_inventory_assets WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_one(database.pool())
    .await?;
    if i64::from(inventory_count) != actual_inventory_count {
        return Err(PhotaraError::Configuration(format!(
            "latest Adobe inventory is incomplete: expected {inventory_count}, stored {actual_inventory_count}"
        )));
    }

    let selected = selected_assets(database, project.id).await?;
    if selected.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project {:?} has no Photographer Final assets",
            project.slug
        )));
    }
    let remote_filenames: BTreeSet<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT file_name FROM cloud_provider_inventory_assets WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .flatten()
    .map(|name| name.to_ascii_lowercase())
    .collect();

    let mut base_names = BTreeMap::<String, usize>::new();
    let mut proposed = Vec::with_capacity(selected.len());
    for asset in &selected {
        let filename = format!(
            "{}.DNG",
            downstream_basename(
                &asset.original_stem,
                asset.capture_date,
                &asset.author_code,
                &asset.original_sha256,
                false,
            )?
        );
        *base_names.entry(filename.to_ascii_lowercase()).or_default() += 1;
        proposed.push(filename);
    }

    let mut items = Vec::with_capacity(selected.len());
    for (asset, proposed_filename) in selected.iter().zip(proposed) {
        if let Some(presence) = existing_presence(database, account_id, run_id, asset).await? {
            items.push(TransferPlanItem {
                asset_id: asset.id,
                source_key: asset.source_key.clone(),
                original_filename: asset.original_filename.clone(),
                planned_filename: presence.remote_filename,
                state: "skipped-already-present".into(),
                remote_asset_id: Some(presence.remote_asset_id),
            });
            continue;
        }
        let lower = proposed_filename.to_ascii_lowercase();
        let collision = base_names.get(&lower).is_some_and(|count| *count > 1)
            || remote_filenames.contains(&lower);
        let planned_filename = if collision {
            format!(
                "{}.DNG",
                downstream_basename(
                    &asset.original_stem,
                    asset.capture_date,
                    &asset.author_code,
                    &asset.original_sha256,
                    true,
                )?
            )
        } else {
            proposed_filename
        };
        if remote_filenames.contains(&planned_filename.to_ascii_lowercase()) {
            return Err(PhotaraError::Configuration(format!(
                "collision-safe DNG filename {planned_filename:?} already exists in Adobe inventory"
            )));
        }
        items.push(TransferPlanItem {
            asset_id: asset.id,
            source_key: asset.source_key.clone(),
            original_filename: asset.original_filename.clone(),
            planned_filename,
            state: "planned".into(),
            remote_asset_id: None,
        });
    }
    items.sort_by(|left, right| left.source_key.cmp(&right.source_key));
    let inventory_snapshot_sha256: String = inventory.try_get("snapshot_sha256")?;
    let manifest = Manifest {
        schema_version: 1,
        provider: ADOBE_LIGHTROOM_PROVIDER,
        account_label,
        project: &project.slug,
        inventory_snapshot_sha256: &inventory_snapshot_sha256,
        items: &items,
    };
    let manifest_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&manifest)?));
    let planned_count = items.iter().filter(|item| item.state == "planned").count();
    let skipped_already_present_count = items.len() - planned_count;
    Ok(TransferPlan {
        schema_version: 1,
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        project: project.slug.clone(),
        inventory_snapshot_sha256,
        manifest_sha256,
        photographer_final_count: items.len(),
        planned_count,
        skipped_already_present_count,
        items,
    })
}

pub async fn reserve(
    database: &Database,
    project: &ProjectRecord,
    account_label: &str,
) -> Result<TransferReservation> {
    let plan = plan(database, project, account_label).await?;
    let account_id: Uuid =
        sqlx::query_scalar("SELECT id FROM cloud_accounts WHERE provider = $1 AND label = $2")
            .bind(ADOBE_LIGHTROOM_PROVIDER)
            .bind(account_label)
            .fetch_one(database.pool())
            .await?;
    let manifest = serde_json::to_value(&plan)?;
    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(&plan.manifest_sha256)
        .execute(&mut *transaction)
        .await?;
    let inventory_run_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM cloud_provider_inventory_runs \
         WHERE account_id = $1 AND snapshot_sha256 = $2",
    )
    .bind(account_id)
    .bind(&plan.inventory_snapshot_sha256)
    .fetch_one(&mut *transaction)
    .await?;
    let existing: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, state FROM cloud_transfer_batches \
         WHERE account_id = $1 AND project_id = $2 AND manifest_sha256 = $3",
    )
    .bind(account_id)
    .bind(project.id)
    .bind(&plan.manifest_sha256)
    .fetch_optional(&mut *transaction)
    .await?;
    let (batch_id, state, reused_existing_batch) = if let Some((id, state)) = existing {
        (id, state, true)
    } else {
        let id = Uuid::new_v4();
        let initial_state = if plan.planned_count == 0 {
            "complete"
        } else {
            "planned"
        };
        sqlx::query(
            "INSERT INTO cloud_transfer_batches \
             (id, account_id, project_id, mode, state, manifest_sha256, expected_count, manifest) \
             VALUES ($1, $2, $3, 'api', $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(account_id)
        .bind(project.id)
        .bind(initial_state)
        .bind(&plan.manifest_sha256)
        .bind(i32::try_from(plan.planned_count).map_err(|_| {
            PhotaraError::Configuration("transfer plan exceeds PostgreSQL integer range".into())
        })?)
        .bind(&manifest)
        .execute(&mut *transaction)
        .await?;
        for item in &plan.items {
            sqlx::query(
                "INSERT INTO cloud_transfer_items \
                 (batch_id, asset_id, state, planned_filename) VALUES ($1, $2, $3, $4)",
            )
            .bind(id)
            .bind(item.asset_id)
            .bind(&item.state)
            .bind(&item.planned_filename)
            .execute(&mut *transaction)
            .await?;
        }
        (id, initial_state.into(), false)
    };

    for item in plan
        .items
        .iter()
        .filter(|item| item.state == "skipped-already-present")
    {
        let presence = existing_presence_for_snapshot(
            &mut transaction,
            account_id,
            inventory_run_id,
            &item.source_key,
            item.asset_id,
        )
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "verified Cloud presence disappeared for {}",
                item.source_key
            ))
        })?;
        sqlx::query(
            "INSERT INTO asset_cloud_presence \
             (account_id, asset_id, status, evidence_kind, evidence_import_id, remote_asset_id, \
              first_confirmed_at, last_verified_at) \
             VALUES ($1, $2, 'present', 'provider-api', $3, $4, now(), now()) \
             ON CONFLICT (account_id, asset_id) DO UPDATE SET \
               status = 'present', evidence_kind = 'provider-api', \
               evidence_import_id = EXCLUDED.evidence_import_id, \
               remote_asset_id = EXCLUDED.remote_asset_id, last_verified_at = now()",
        )
        .bind(account_id)
        .bind(item.asset_id)
        .bind(presence.evidence_import_id)
        .bind(&presence.remote_asset_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(TransferReservation {
        schema_version: 1,
        batch_id,
        manifest_sha256: plan.manifest_sha256,
        state,
        expected_upload_count: plan.planned_count,
        skipped_already_present_count: plan.skipped_already_present_count,
        reused_existing_batch,
        no_transfer_required: plan.planned_count == 0,
    })
}

pub async fn begin_export(database: &Database, batch_id: Uuid) -> Result<ExportBatch> {
    let staging_directory = staging_directory(batch_id)?;
    fs::create_dir_all(&staging_directory).map_err(|source| {
        PhotaraError::filesystem(
            "create transfer staging directory",
            &staging_directory,
            source,
        )
    })?;

    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(batch_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let state: String =
        sqlx::query_scalar("SELECT state FROM cloud_transfer_batches WHERE id = $1 FOR UPDATE")
            .bind(batch_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| {
                PhotaraError::Configuration(format!("transfer batch {batch_id} was not found"))
            })?;
    if !matches!(state.as_str(), "planned" | "exporting") {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} cannot export while in state {state:?}"
        )));
    }
    if state == "planned" {
        sqlx::query(
            "UPDATE cloud_transfer_batches SET state = 'exporting', updated_at = now() WHERE id = $1",
        )
        .bind(batch_id)
        .execute(&mut *transaction)
        .await?;
    }
    let rows = sqlx::query(
        "SELECT item.asset_id, file.location AS source_key, item.planned_filename, item.state \
         FROM cloud_transfer_items AS item \
         JOIN asset_files AS file ON file.asset_id = item.asset_id \
           AND file.representation = 'camera-raw' AND file.state = 'current' \
         WHERE item.batch_id = $1 AND item.state IN ('planned', 'exported') \
         ORDER BY file.location",
    )
    .bind(batch_id)
    .fetch_all(&mut *transaction)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let planned_filename: String = row.try_get("planned_filename")?;
        let state: String = row.try_get("state")?;
        items.push(ExportBatchItem {
            asset_id: row.try_get("asset_id")?,
            source_key: row.try_get("source_key")?,
            staged_path: staging_directory.join(&planned_filename),
            planned_filename,
            state,
        });
    }
    let pending_count = items.iter().filter(|item| item.state == "planned").count();
    let exported_count = items.len() - pending_count;
    transaction.commit().await?;
    Ok(ExportBatch {
        schema_version: 1,
        batch_id,
        state: "exporting".into(),
        staging_directory,
        pending_count,
        exported_count,
        items,
    })
}

pub async fn record_export(
    database: &Database,
    batch_id: Uuid,
    asset_id: Uuid,
    staged_path: &Path,
) -> Result<ExportRecord> {
    let staging_directory = staging_directory(batch_id)?;
    let planned_filename: String = sqlx::query_scalar(
        "SELECT planned_filename FROM cloud_transfer_items \
         WHERE batch_id = $1 AND asset_id = $2 AND state IN ('planned', 'exported')",
    )
    .bind(batch_id)
    .bind(asset_id)
    .fetch_optional(database.pool())
    .await?
    .flatten()
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "asset {asset_id} is not pending or exported in transfer batch {batch_id}"
        ))
    })?;
    let expected_path = staging_directory.join(&planned_filename);
    if staged_path != expected_path {
        return Err(PhotaraError::Configuration(format!(
            "staged DNG path must be exactly {}",
            expected_path.display()
        )));
    }
    let (sha256, byte_size) = inspect_dng(staged_path)?;
    let byte_size_i64 = i64::try_from(byte_size).map_err(|_| {
        PhotaraError::Configuration("staged DNG exceeds PostgreSQL bigint range".into())
    })?;
    let logical_location = format!("staging:{batch_id}/{planned_filename}");

    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(batch_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let row = sqlx::query(
        "SELECT state, working_file_id FROM cloud_transfer_items \
         WHERE batch_id = $1 AND asset_id = $2 FOR UPDATE",
    )
    .bind(batch_id)
    .bind(asset_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "asset {asset_id} is not part of transfer batch {batch_id}"
        ))
    })?;
    let state: String = row.try_get("state")?;
    if !matches!(state.as_str(), "planned" | "exported") {
        return Err(PhotaraError::Configuration(format!(
            "asset {asset_id} cannot record an export while in state {state:?}"
        )));
    }
    let existing_file_id: Option<Uuid> = row.try_get("working_file_id")?;
    let (working_file_id, reused_existing_record) = if let Some(file_id) = existing_file_id {
        let existing =
            sqlx::query("SELECT location, sha256, byte_size FROM asset_files WHERE id = $1")
                .bind(file_id)
                .fetch_one(&mut *transaction)
                .await?;
        let existing_location: String = existing.try_get("location")?;
        let existing_sha256: Option<String> = existing.try_get("sha256")?;
        let existing_byte_size: Option<i64> = existing.try_get("byte_size")?;
        if existing_location != logical_location
            || existing_sha256.as_deref() != Some(&sha256)
            || existing_byte_size != Some(byte_size_i64)
        {
            return Err(PhotaraError::Configuration(format!(
                "recorded DNG for asset {asset_id} does not match the staged file"
            )));
        }
        (file_id, true)
    } else {
        let file_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO asset_files \
             (id, asset_id, representation, location, sha256, byte_size, authoritative) \
             VALUES ($1, $2, 'working-dng', $3, $4, $5, false)",
        )
        .bind(file_id)
        .bind(asset_id)
        .bind(&logical_location)
        .bind(&sha256)
        .bind(byte_size_i64)
        .execute(&mut *transaction)
        .await?;
        let source_file_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM asset_files WHERE asset_id = $1 \
             AND representation = 'camera-raw' AND state = 'current'",
        )
        .bind(asset_id)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO asset_file_origins (source_file_id, derived_file_id, operation) \
             VALUES ($1, $2, 'lightroom-classic-dng-export')",
        )
        .bind(source_file_id)
        .bind(file_id)
        .execute(&mut *transaction)
        .await?;
        (file_id, false)
    };
    sqlx::query(
        "UPDATE cloud_transfer_items SET working_file_id = $3, state = 'exported', \
         error_message = NULL, updated_at = now() WHERE batch_id = $1 AND asset_id = $2",
    )
    .bind(batch_id)
    .bind(asset_id)
    .bind(working_file_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(ExportRecord {
        schema_version: 1,
        batch_id,
        asset_id,
        planned_filename,
        sha256,
        byte_size,
        state: "exported".into(),
        reused_existing_record,
    })
}

pub async fn finish_export(database: &Database, batch_id: Uuid) -> Result<ExportCompletion> {
    let staging_directory = staging_directory(batch_id)?;
    let rows = sqlx::query(
        "SELECT item.state, item.planned_filename, file.sha256, file.byte_size \
         FROM cloud_transfer_items AS item \
         LEFT JOIN asset_files AS file ON file.id = item.working_file_id \
         WHERE item.batch_id = $1 ORDER BY item.planned_filename",
    )
    .bind(batch_id)
    .fetch_all(database.pool())
    .await?;
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} was not found or contains no items"
        )));
    }
    let mut exported_count = 0;
    let mut skipped_count = 0;
    for row in &rows {
        let state: String = row.try_get("state")?;
        match state.as_str() {
            "exported" => {
                let filename: String = row.try_get("planned_filename")?;
                let expected_sha256: String = row.try_get("sha256")?;
                let expected_size: i64 = row.try_get("byte_size")?;
                let (actual_sha256, actual_size) = inspect_dng(&staging_directory.join(filename))?;
                if actual_sha256 != expected_sha256
                    || u64::try_from(expected_size).ok() != Some(actual_size)
                {
                    return Err(PhotaraError::Configuration(
                        "a staged DNG changed after it was recorded".into(),
                    ));
                }
                exported_count += 1;
            }
            "skipped-already-present" => skipped_count += 1,
            other => {
                return Err(PhotaraError::Configuration(format!(
                    "transfer batch {batch_id} still contains an item in state {other:?}"
                )));
            }
        }
    }
    let mut transaction = database.begin().await?;
    let updated = sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'awaiting-user-confirmation', updated_at = now() \
         WHERE id = $1 AND state = 'exporting'",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} is not currently exporting"
        )));
    }
    transaction.commit().await?;
    Ok(ExportCompletion {
        schema_version: 1,
        batch_id,
        state: "awaiting-user-confirmation".into(),
        exported_count,
        skipped_already_present_count: skipped_count,
        staging_directory,
    })
}

pub async fn upload_requirements(
    database: &Database,
    batch_id: Uuid,
) -> Result<UploadRequirements> {
    let batch =
        sqlx::query("SELECT state, expected_count FROM cloud_transfer_batches WHERE id = $1")
            .bind(batch_id)
            .fetch_optional(database.pool())
            .await?
            .ok_or_else(|| {
                PhotaraError::Configuration(format!("transfer batch {batch_id} was not found"))
            })?;
    let state: String = batch.try_get("state")?;
    if !matches!(state.as_str(), "awaiting-user-confirmation" | "uploading") {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} is not ready for upload preflight; state is {state:?}"
        )));
    }
    let expected_count: i32 = batch.try_get("expected_count")?;
    let rows = sqlx::query(
        "SELECT item.asset_id, item.state, item.planned_filename, file.byte_size \
         FROM cloud_transfer_items AS item \
         LEFT JOIN asset_files AS file ON file.id = item.working_file_id \
         WHERE item.batch_id = $1 ORDER BY item.planned_filename",
    )
    .bind(batch_id)
    .fetch_all(database.pool())
    .await?;
    let mut required_bytes = 0_u64;
    let mut exported = Vec::new();
    let mut uploaded_or_verified_count = 0_usize;
    let mut skipped_count = 0;
    for row in rows {
        let item_state: String = row.try_get("state")?;
        match item_state.as_str() {
            "exported" => {
                let byte_size: i64 = row.try_get("byte_size")?;
                let byte_size = u64::try_from(byte_size).map_err(|_| {
                    PhotaraError::Configuration("recorded DNG has an invalid byte size".into())
                })?;
                required_bytes = required_bytes.checked_add(byte_size).ok_or_else(|| {
                    PhotaraError::Configuration("transfer byte total overflowed".into())
                })?;
                exported.push((
                    row.try_get::<Uuid, _>("asset_id")?,
                    row.try_get::<String, _>("planned_filename")?,
                ));
            }
            "skipped-already-present" => skipped_count += 1,
            "uploaded" | "verified" => uploaded_or_verified_count += 1,
            other => {
                return Err(PhotaraError::Configuration(format!(
                    "transfer batch {batch_id} contains an item in state {other:?}"
                )));
            }
        }
    }
    if exported.len() + uploaded_or_verified_count
        != usize::try_from(expected_count).unwrap_or(usize::MAX)
    {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch expected {expected_count} uploads but has {} eligible items",
            exported.len() + uploaded_or_verified_count
        )));
    }
    let (canary_asset_id, canary_filename) = exported
        .first()
        .cloned()
        .ok_or_else(|| PhotaraError::Configuration("transfer batch has no DNG to upload".into()))?;
    Ok(UploadRequirements {
        schema_version: 1,
        batch_id,
        state,
        upload_count: exported.len(),
        skipped_already_present_count: skipped_count,
        required_bytes,
        canary_asset_id,
        canary_filename,
    })
}

pub async fn prepare_canary_upload(
    database: &Database,
    batch_id: Uuid,
    account_label: &str,
) -> Result<CanaryUpload> {
    let staging_directory = staging_directory(batch_id)?;
    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(batch_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let batch_state: String = sqlx::query_scalar(
        "SELECT batch.state FROM cloud_transfer_batches AS batch \
         JOIN cloud_accounts AS account ON account.id = batch.account_id \
         WHERE batch.id = $1 AND account.provider = $2 AND account.label = $3 FOR UPDATE",
    )
    .bind(batch_id)
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "transfer batch {batch_id} does not belong to Adobe account {account_label:?}"
        ))
    })?;
    if !matches!(
        batch_state.as_str(),
        "awaiting-user-confirmation" | "uploading"
    ) {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} cannot upload a canary while in state {batch_state:?}"
        )));
    }
    let row = sqlx::query(
        "SELECT item.asset_id, item.planned_filename, item.remote_asset_id, \
                file.sha256, file.byte_size, asset.capture_date \
         FROM cloud_transfer_items AS item \
         JOIN asset_files AS file ON file.id = item.working_file_id \
         JOIN assets AS asset ON asset.id = item.asset_id \
         WHERE item.batch_id = $1 AND item.state = 'exported' \
         ORDER BY item.planned_filename LIMIT 1 FOR UPDATE OF item",
    )
    .bind(batch_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "transfer batch {batch_id} has no exported DNG awaiting upload"
        ))
    })?;
    let asset_id: Uuid = row.try_get("asset_id")?;
    let filename: String = row.try_get("planned_filename")?;
    let sha256: String = row.try_get("sha256")?;
    let byte_size: i64 = row.try_get("byte_size")?;
    let byte_size = u64::try_from(byte_size)
        .map_err(|_| PhotaraError::Configuration("canary has an invalid byte size".into()))?;
    let staged_path = staging_directory.join(&filename);
    let (actual_sha256, actual_size) = inspect_dng(&staged_path)?;
    if actual_sha256 != sha256 || actual_size != byte_size {
        return Err(PhotaraError::Configuration(
            "canary DNG changed after export validation".into(),
        ));
    }
    let remote_asset_id = row
        .try_get::<Option<String>, _>("remote_asset_id")?
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());
    sqlx::query(
        "UPDATE cloud_transfer_items SET remote_asset_id = $3, updated_at = now() \
         WHERE batch_id = $1 AND asset_id = $2",
    )
    .bind(batch_id)
    .bind(asset_id)
    .bind(&remote_asset_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'uploading', updated_at = now() WHERE id = $1",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(CanaryUpload {
        batch_id,
        asset_id,
        remote_asset_id,
        filename,
        staged_path,
        sha256,
        byte_size,
        capture_date: row.try_get("capture_date")?,
    })
}

pub async fn mark_canary_uploaded(database: &Database, canary: &CanaryUpload) -> Result<()> {
    let mut transaction = database.begin().await?;
    let updated = sqlx::query(
        "UPDATE cloud_transfer_items SET state = 'uploaded', uploaded_at = now(), updated_at = now() \
         WHERE batch_id = $1 AND asset_id = $2 AND remote_asset_id = $3 AND state = 'exported'",
    )
    .bind(canary.batch_id)
    .bind(canary.asset_id)
    .bind(&canary.remote_asset_id)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(PhotaraError::Configuration(
            "canary transfer state changed before upload could be recorded".into(),
        ));
    }
    sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'uploading', updated_at = now() WHERE id = $1",
    )
    .bind(canary.batch_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn begin_batch_verification(database: &Database, batch_id: Uuid) -> Result<()> {
    let remaining_exported: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_transfer_items WHERE batch_id = $1 AND state = 'exported'",
    )
    .bind(batch_id)
    .fetch_one(database.pool())
    .await?;
    if remaining_exported != 0 {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} still has {remaining_exported} DNGs awaiting upload"
        )));
    }
    let updated = sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'verifying', updated_at = now() \
         WHERE id = $1 AND state = 'uploading'",
    )
    .bind(batch_id)
    .execute(database.pool())
    .await?;
    if updated.rows_affected() != 1 {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} is not uploading"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BatchVerification {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub state: String,
    pub verified_upload_count: usize,
    pub skipped_already_present_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CleanupReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub state: String,
    pub removed_file_count: usize,
    pub already_missing_count: usize,
    pub staging_directory: PathBuf,
    pub already_complete: bool,
}

pub async fn cleanup_batch(
    database: &Database,
    batch_id: Uuid,
    confirmed: bool,
) -> Result<CleanupReport> {
    if !confirmed {
        return Err(PhotaraError::Configuration(
            "cleanup requires explicit confirmation; pass --confirm after reviewing the batch"
                .into(),
        ));
    }
    let staging_directory = staging_directory(batch_id)?;
    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(batch_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let batch = sqlx::query(
        "SELECT batch.state, batch.expected_count, batch.account_id, \
                batch.cleanup_started_at IS NOT NULL AS cleanup_started, \
                batch.cleaned_at IS NOT NULL AS cleanup_complete, inventory.id AS run_id \
         FROM cloud_transfer_batches AS batch \
         JOIN LATERAL ( \
             SELECT id FROM cloud_provider_inventory_runs \
             WHERE account_id = batch.account_id AND completed_at IS NOT NULL \
             ORDER BY completed_at DESC, id DESC LIMIT 1 \
         ) AS inventory ON true \
         WHERE batch.id = $1 FOR UPDATE OF batch",
    )
    .bind(batch_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!("transfer batch {batch_id} was not found"))
    })?;
    let state: String = batch.try_get("state")?;
    if state != "complete" {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} cannot be cleaned while in state {state:?}"
        )));
    }
    let expected_count: i32 = batch.try_get("expected_count")?;
    let run_id: Uuid = batch.try_get("run_id")?;
    let cleanup_started: bool = batch.try_get("cleanup_started")?;
    let cleanup_complete: bool = batch.try_get("cleanup_complete")?;
    match fs::symlink_metadata(&staging_directory) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(PhotaraError::Configuration(format!(
                "refusing cleanup because staging path {} is not a real directory",
                staging_directory.display()
            )));
        }
        Ok(_) => {}
        Err(source)
            if source.kind() == std::io::ErrorKind::NotFound
                && (cleanup_started || cleanup_complete) => {}
        Err(source) => {
            return Err(PhotaraError::filesystem(
                "inspect transfer staging directory",
                &staging_directory,
                source,
            ));
        }
    }
    let rows = sqlx::query(
        "SELECT item.asset_id, item.planned_filename, item.remote_asset_id, \
                item.verified_at IS NOT NULL AS verified, file.id AS file_id, \
                file.sha256, file.byte_size, file.state AS file_state, \
                inventory.file_name AS remote_filename, inventory.sha256 AS remote_sha256 \
         FROM cloud_transfer_items AS item \
         JOIN asset_files AS file ON file.id = item.working_file_id \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $2 AND inventory.remote_asset_id = item.remote_asset_id \
         WHERE item.batch_id = $1 AND item.state = 'verified' \
         ORDER BY item.planned_filename FOR UPDATE OF item, file",
    )
    .bind(batch_id)
    .bind(run_id)
    .fetch_all(&mut *transaction)
    .await?;
    if rows.len() != usize::try_from(expected_count).unwrap_or(usize::MAX) {
        return Err(PhotaraError::Configuration(format!(
            "cleanup expected {expected_count} verified uploads but found {} in the latest Adobe inventory",
            rows.len()
        )));
    }

    let mut files = Vec::with_capacity(rows.len());
    let mut already_missing_count = 0;
    for row in rows {
        let filename: String = row.try_get("planned_filename")?;
        if !is_plain_staged_filename(&filename) {
            return Err(PhotaraError::Configuration(format!(
                "refusing cleanup because {filename:?} is not a plain staged filename"
            )));
        }
        let verified: bool = row.try_get("verified")?;
        let expected_sha256: String = row.try_get("sha256")?;
        let expected_size: i64 = row.try_get("byte_size")?;
        let remote_filename: Option<String> = row.try_get("remote_filename")?;
        let remote_sha256: Option<String> = row.try_get("remote_sha256")?;
        if !verified
            || remote_filename.as_deref() != Some(&filename)
            || remote_sha256.as_deref() != Some(&expected_sha256)
        {
            return Err(PhotaraError::Configuration(format!(
                "refusing cleanup because Adobe verification no longer matches {filename}"
            )));
        }
        let path = staging_directory.join(&filename);
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let (actual_sha256, actual_size) = inspect_dng(&path)?;
                if actual_sha256 != expected_sha256
                    || u64::try_from(expected_size).ok() != Some(actual_size)
                {
                    return Err(PhotaraError::Configuration(format!(
                        "refusing cleanup because staged DNG {filename} changed after validation"
                    )));
                }
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound && cleanup_started => {
                already_missing_count += 1;
            }
            Err(source) => {
                return Err(PhotaraError::filesystem(
                    "inspect cleanup candidate",
                    &path,
                    source,
                ));
            }
        }
        let file_id: Uuid = row.try_get("file_id")?;
        let file_state: String = row.try_get("file_state")?;
        if !matches!(file_state.as_str(), "current" | "removed") {
            return Err(PhotaraError::Configuration(format!(
                "refusing cleanup because {filename} has unexpected file state {file_state:?}"
            )));
        }
        files.push((file_id, path));
    }

    if cleanup_complete {
        if files.iter().any(|(_, path)| path_lexists(path)) || path_lexists(&staging_directory) {
            return Err(PhotaraError::Configuration(format!(
                "batch {batch_id} is recorded as cleaned but its staging path still exists"
            )));
        }
        transaction.commit().await?;
        return Ok(CleanupReport {
            schema_version: 1,
            batch_id,
            state: "clean".into(),
            removed_file_count: 0,
            already_missing_count: files.len(),
            staging_directory,
            already_complete: true,
        });
    }

    sqlx::query(
        "UPDATE cloud_transfer_batches SET cleanup_started_at = COALESCE(cleanup_started_at, now()), \
         updated_at = now() WHERE id = $1",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    for (file_id, _) in &files {
        sqlx::query(
            "UPDATE asset_files SET state = 'removed', removed_at = COALESCE(removed_at, now()) \
             WHERE id = $1",
        )
        .bind(file_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    let mut removed_file_count = 0;
    for (_, path) in &files {
        match fs::remove_file(path) {
            Ok(()) => removed_file_count += 1,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(PhotaraError::filesystem(
                    "remove verified staged DNG",
                    path,
                    source,
                ));
            }
        }
    }
    match fs::remove_dir(&staging_directory) {
        Ok(()) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(PhotaraError::filesystem(
                "remove empty transfer staging directory",
                &staging_directory,
                source,
            ));
        }
    }

    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(batch_id.to_string())
        .execute(&mut *transaction)
        .await?;
    if files.iter().any(|(_, path)| path_lexists(path)) || path_lexists(&staging_directory) {
        return Err(PhotaraError::Configuration(
            "cleanup did not remove every verified staging path".into(),
        ));
    }
    sqlx::query(
        "UPDATE cloud_transfer_batches SET cleaned_at = now(), updated_at = now() \
         WHERE id = $1 AND state = 'complete' AND cleanup_started_at IS NOT NULL",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(CleanupReport {
        schema_version: 1,
        batch_id,
        state: "clean".into(),
        removed_file_count,
        already_missing_count,
        staging_directory,
        already_complete: false,
    })
}

pub async fn verify_uploaded_batch(
    database: &Database,
    batch_id: Uuid,
) -> Result<BatchVerification> {
    let mut transaction = database.begin().await?;
    let batch = sqlx::query(
        "SELECT state, expected_count, account_id FROM cloud_transfer_batches \
         WHERE id = $1 FOR UPDATE",
    )
    .bind(batch_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!("transfer batch {batch_id} was not found"))
    })?;
    let state: String = batch.try_get("state")?;
    if state != "verifying" {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch {batch_id} cannot verify while in state {state:?}"
        )));
    }
    let account_id: Uuid = batch.try_get("account_id")?;
    let expected_count: i32 = batch.try_get("expected_count")?;
    let rows = sqlx::query(
        "SELECT item.asset_id, item.remote_asset_id, item.planned_filename, item.state, file.sha256 \
         FROM cloud_transfer_items AS item \
         JOIN asset_files AS file ON file.id = item.working_file_id \
         WHERE item.batch_id = $1 AND item.state IN ('uploaded', 'verified') \
         ORDER BY item.planned_filename FOR UPDATE OF item",
    )
    .bind(batch_id)
    .fetch_all(&mut *transaction)
    .await?;
    if rows.len() != usize::try_from(expected_count).unwrap_or(usize::MAX) {
        return Err(PhotaraError::Configuration(format!(
            "transfer batch expected {expected_count} verified uploads but found {}",
            rows.len()
        )));
    }
    for row in &rows {
        let asset_id: Uuid = row.try_get("asset_id")?;
        let remote_asset_id: String = row.try_get("remote_asset_id")?;
        let filename: String = row.try_get("planned_filename")?;
        let sha256: String = row.try_get("sha256")?;
        let observed = sqlx::query(
            "SELECT inventory.file_name, inventory.sha256 \
             FROM cloud_provider_inventory_assets AS inventory \
             JOIN cloud_provider_inventory_runs AS run ON run.id = inventory.run_id \
             WHERE run.account_id = $1 AND run.completed_at IS NOT NULL \
               AND inventory.remote_asset_id = $2 \
             ORDER BY run.completed_at DESC LIMIT 1",
        )
        .bind(account_id)
        .bind(&remote_asset_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "fresh Adobe inventory does not contain {filename}"
            ))
        })?;
        let observed_filename: Option<String> = observed.try_get("file_name")?;
        let observed_sha256: Option<String> = observed.try_get("sha256")?;
        if observed_filename.as_deref() != Some(&filename)
            || observed_sha256.as_deref() != Some(&sha256)
        {
            return Err(PhotaraError::Configuration(format!(
                "Adobe inventory does not match validated DNG {filename}"
            )));
        }
        sqlx::query(
            "UPDATE cloud_transfer_items SET state = 'verified', verified_at = COALESCE(verified_at, now()), \
             updated_at = now() WHERE batch_id = $1 AND asset_id = $2",
        )
        .bind(batch_id)
        .bind(asset_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO asset_cloud_presence \
             (account_id, asset_id, status, evidence_kind, remote_asset_id, first_confirmed_at, last_verified_at) \
             VALUES ($1, $2, 'present', 'provider-api', $3, now(), now()) \
             ON CONFLICT (account_id, asset_id) DO UPDATE SET status = 'present', \
               evidence_kind = 'provider-api', remote_asset_id = EXCLUDED.remote_asset_id, \
               last_verified_at = now()",
        )
        .bind(account_id)
        .bind(asset_id)
        .bind(&remote_asset_id)
        .execute(&mut *transaction)
        .await?;
    }
    let skipped_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_transfer_items \
         WHERE batch_id = $1 AND state = 'skipped-already-present'",
    )
    .bind(batch_id)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'complete', confirmed_at = now(), updated_at = now() \
         WHERE id = $1",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(BatchVerification {
        schema_version: 1,
        batch_id,
        state: "complete".into(),
        verified_upload_count: rows.len(),
        skipped_already_present_count: usize::try_from(skipped_count).unwrap_or(usize::MAX),
    })
}

pub async fn verify_canary(database: &Database, batch_id: Uuid) -> Result<CanaryVerification> {
    let mut transaction = database.begin().await?;
    let row = sqlx::query(
        "SELECT item.asset_id, item.remote_asset_id, item.planned_filename, file.sha256, \
                batch.account_id \
         FROM cloud_transfer_items AS item \
         JOIN cloud_transfer_batches AS batch ON batch.id = item.batch_id \
         JOIN asset_files AS file ON file.id = item.working_file_id \
         WHERE item.batch_id = $1 AND item.state = 'uploaded' \
         ORDER BY item.planned_filename LIMIT 1 FOR UPDATE OF item",
    )
    .bind(batch_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "transfer batch {batch_id} has no uploaded canary awaiting verification"
        ))
    })?;
    let account_id: Uuid = row.try_get("account_id")?;
    let asset_id: Uuid = row.try_get("asset_id")?;
    let remote_asset_id: String = row.try_get("remote_asset_id")?;
    let filename: String = row.try_get("planned_filename")?;
    let sha256: String = row.try_get("sha256")?;
    let observed = sqlx::query(
        "SELECT inventory.file_name, inventory.sha256 \
         FROM cloud_provider_inventory_assets AS inventory \
         JOIN cloud_provider_inventory_runs AS run ON run.id = inventory.run_id \
         WHERE run.account_id = $1 AND run.completed_at IS NOT NULL \
           AND inventory.remote_asset_id = $2 \
         ORDER BY run.completed_at DESC LIMIT 1",
    )
    .bind(account_id)
    .bind(&remote_asset_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(
            "fresh Adobe inventory does not yet contain the uploaded canary".into(),
        )
    })?;
    let observed_filename: Option<String> = observed.try_get("file_name")?;
    let observed_sha256: Option<String> = observed.try_get("sha256")?;
    if observed_filename.as_deref() != Some(&filename)
        || observed_sha256.as_deref() != Some(&sha256)
    {
        return Err(PhotaraError::Configuration(
            "Adobe canary metadata does not match the validated staged DNG".into(),
        ));
    }
    sqlx::query(
        "UPDATE cloud_transfer_items SET state = 'verified', verified_at = now(), updated_at = now() \
         WHERE batch_id = $1 AND asset_id = $2",
    )
    .bind(batch_id)
    .bind(asset_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO asset_cloud_presence \
         (account_id, asset_id, status, evidence_kind, remote_asset_id, first_confirmed_at, last_verified_at) \
         VALUES ($1, $2, 'present', 'provider-api', $3, now(), now()) \
         ON CONFLICT (account_id, asset_id) DO UPDATE SET status = 'present', \
           evidence_kind = 'provider-api', remote_asset_id = EXCLUDED.remote_asset_id, \
           last_verified_at = now()",
    )
    .bind(account_id)
    .bind(asset_id)
    .bind(&remote_asset_id)
    .execute(&mut *transaction)
    .await?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_transfer_items WHERE batch_id = $1 AND state = 'exported'",
    )
    .bind(batch_id)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE cloud_transfer_batches SET state = 'awaiting-user-confirmation', updated_at = now() \
         WHERE id = $1",
    )
    .bind(batch_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(CanaryVerification {
        schema_version: 1,
        batch_id,
        asset_id,
        remote_asset_id,
        filename,
        sha256,
        state: "verified".into(),
        remaining_exported_count: usize::try_from(remaining).unwrap_or(usize::MAX),
    })
}

fn staging_directory(batch_id: Uuid) -> Result<PathBuf> {
    let root = if let Some(value) = env::var_os("PHOTARA_STAGING_ROOT") {
        PathBuf::from(value)
    } else if let Some(value) = env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(value).join("photara/transfers")
    } else if let Some(value) = env::var_os("HOME") {
        PathBuf::from(value).join(".cache/photara/transfers")
    } else {
        return Err(PhotaraError::Configuration(
            "set PHOTARA_STAGING_ROOT, XDG_CACHE_HOME, or HOME for transfer staging".into(),
        ));
    };
    if !root.is_absolute() {
        return Err(PhotaraError::Configuration(
            "transfer staging root must be absolute".into(),
        ));
    }
    Ok(root.join(batch_id.to_string()))
}

fn is_plain_staged_filename(filename: &str) -> bool {
    let path = Path::new(filename);
    path.components().count() == 1 && matches!(path.components().next(), Some(Component::Normal(_)))
}

fn path_lexists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn inspect_dng(path: &Path) -> Result<(String, u64)> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| PhotaraError::filesystem("inspect staged DNG", path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(PhotaraError::Configuration(format!(
            "staged DNG {} must be a non-empty regular file, not a symlink",
            path.display()
        )));
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("dng"))
    {
        return Err(PhotaraError::Configuration(format!(
            "staged file {} must use the DNG extension",
            path.display()
        )));
    }
    let mut file = File::open(path)
        .map_err(|source| PhotaraError::filesystem("open staged DNG", path, source))?;
    let mut header = [0_u8; 4];
    file.read_exact(&mut header)
        .map_err(|source| PhotaraError::filesystem("read staged DNG header", path, source))?;
    if !matches!(
        header,
        [b'I', b'I', 42, 0] | [b'M', b'M', 0, 42] | [b'I', b'I', 43, 0] | [b'M', b'M', 0, 43]
    ) {
        return Err(PhotaraError::Configuration(format!(
            "staged file {} does not have a TIFF/DNG header",
            path.display()
        )));
    }
    let mut digest = Sha256::new();
    digest.update(header);
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| PhotaraError::filesystem("hash staged DNG", path, source))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok((format!("{:x}", digest.finalize()), metadata.len()))
}

async fn selected_assets(database: &Database, project_id: Uuid) -> Result<Vec<SelectedAsset>> {
    sqlx::query(
        "SELECT asset.id, asset.original_filename, asset.original_stem, asset.capture_date, \
                asset.author_code, asset.original_sha256, file.location \
         FROM project_asset_decisions AS decision \
         JOIN assets AS asset ON asset.id = decision.asset_id \
         JOIN asset_files AS file ON file.asset_id = asset.id \
           AND file.representation = 'camera-raw' AND file.state = 'current' \
         WHERE decision.project_id = $1 AND decision.decision = 'photographer-final' \
           AND decision.selected \
         ORDER BY file.location",
    )
    .bind(project_id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(SelectedAsset {
            id: row.try_get("id")?,
            source_key: row.try_get("location")?,
            original_filename: row.try_get("original_filename")?,
            original_stem: row.try_get("original_stem")?,
            capture_date: row.try_get("capture_date")?,
            author_code: row.try_get("author_code")?,
            original_sha256: row.try_get("original_sha256")?,
        })
    })
    .collect::<std::result::Result<Vec<_>, sqlx::Error>>()
    .map_err(Into::into)
}

async fn existing_presence(
    database: &Database,
    account_id: Uuid,
    run_id: Uuid,
    asset: &SelectedAsset,
) -> Result<Option<ExistingPresence>> {
    if let Some(row) = sqlx::query(
        "SELECT presence.remote_asset_id, inventory.file_name \
         FROM asset_cloud_presence AS presence \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $3 AND inventory.remote_asset_id = presence.remote_asset_id \
         WHERE presence.account_id = $1 AND presence.asset_id = $2 \
           AND presence.status = 'present' AND inventory.file_name IS NOT NULL",
    )
    .bind(account_id)
    .bind(asset.id)
    .bind(run_id)
    .fetch_optional(database.pool())
    .await?
    {
        return Ok(Some(ExistingPresence {
            remote_asset_id: row.try_get("remote_asset_id")?,
            remote_filename: row.try_get("file_name")?,
            evidence_import_id: None,
        }));
    }
    let row = sqlx::query(
        "SELECT evidence.remote_asset_id, inventory.file_name, evidence.import_id \
         FROM cloud_evidence_entries AS evidence \
         JOIN cloud_evidence_imports AS evidence_import ON evidence_import.id = evidence.import_id \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $3 AND inventory.remote_asset_id = evidence.remote_asset_id \
         WHERE evidence_import.account_id = $1 AND evidence.source_key = $2 \
           AND inventory.file_name IS NOT NULL",
    )
    .bind(account_id)
    .bind(&asset.source_key)
    .bind(run_id)
    .fetch_optional(database.pool())
    .await?;
    row.map(|row| {
        Ok(ExistingPresence {
            remote_asset_id: row.try_get("remote_asset_id")?,
            remote_filename: row.try_get("file_name")?,
            evidence_import_id: Some(row.try_get("import_id")?),
        })
    })
    .transpose()
}

async fn existing_presence_for_snapshot(
    connection: &mut sqlx::PgConnection,
    account_id: Uuid,
    run_id: Uuid,
    source_key: &str,
    asset_id: Uuid,
) -> Result<Option<ExistingPresence>> {
    if let Some(row) = sqlx::query(
        "SELECT presence.remote_asset_id, inventory.file_name, \
                presence.evidence_import_id \
         FROM asset_cloud_presence AS presence \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $3 AND inventory.remote_asset_id = presence.remote_asset_id \
         WHERE presence.account_id = $1 AND presence.asset_id = $2 \
           AND presence.status = 'present' AND inventory.file_name IS NOT NULL",
    )
    .bind(account_id)
    .bind(asset_id)
    .bind(run_id)
    .fetch_optional(&mut *connection)
    .await?
    {
        return Ok(Some(ExistingPresence {
            remote_asset_id: row.try_get("remote_asset_id")?,
            remote_filename: row.try_get("file_name")?,
            evidence_import_id: row.try_get("evidence_import_id")?,
        }));
    }
    let row = sqlx::query(
        "SELECT evidence.remote_asset_id, inventory.file_name, evidence.import_id \
         FROM cloud_evidence_entries AS evidence \
         JOIN cloud_evidence_imports AS evidence_import ON evidence_import.id = evidence.import_id \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $3 AND inventory.remote_asset_id = evidence.remote_asset_id \
         WHERE evidence_import.account_id = $1 AND evidence.source_key = $2 \
           AND evidence.remote_asset_id IS NOT NULL AND inventory.file_name IS NOT NULL",
    )
    .bind(account_id)
    .bind(source_key)
    .bind(run_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(ref row) = row {
        let import_id: Uuid = row.try_get("import_id")?;
        let updated = sqlx::query(
            "UPDATE cloud_evidence_entries SET matched_asset_id = $1 \
             WHERE import_id = $2 AND source_key = $3 \
               AND (matched_asset_id IS NULL OR matched_asset_id = $1)",
        )
        .bind(asset_id)
        .bind(import_id)
        .bind(source_key)
        .execute(&mut *connection)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(PhotaraError::Configuration(format!(
                "Cloud evidence for {source_key:?} is already linked to a different asset"
            )));
        }
    }
    row.map(|row| {
        Ok(ExistingPresence {
            remote_asset_id: row.try_get("remote_asset_id")?,
            remote_filename: row.try_get("file_name")?,
            evidence_import_id: Some(row.try_get("import_id")?),
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_states_are_provider_neutral() {
        let item = TransferPlanItem {
            asset_id: Uuid::nil(),
            source_key: "images:2021/a.ARW".into(),
            original_filename: "a.ARW".into(),
            planned_filename: "a_2021_01_01_AUTHOR.DNG".into(),
            state: "planned".into(),
            remote_asset_id: None,
        };
        assert_eq!(item.state, "planned");
    }

    #[test]
    fn cleanup_accepts_only_plain_staged_filenames() {
        assert!(is_plain_staged_filename("DSC00001_2026_01_02_SUHAIL.DNG"));
        assert!(!is_plain_staged_filename("../escape.DNG"));
        assert!(!is_plain_staged_filename("nested/escape.DNG"));
        assert!(!is_plain_staged_filename("/absolute/escape.DNG"));
        assert!(!is_plain_staged_filename("."));
    }
}
