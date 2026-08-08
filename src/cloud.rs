use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{
    Connection, Postgres, QueryBuilder, Row, SqliteConnection, sqlite::SqliteConnectOptions,
};
use storexa::Database;
use uuid::Uuid;

use crate::{PhotaraError, Result, adobe::AdobeInventory};

pub const ADOBE_LIGHTROOM_PROVIDER: &str = "adobe-lightroom";
const IMAGES_ROOT_KEY: &str = "images";

#[derive(Clone, Debug)]
pub struct ProetusImport {
    pub database_path: PathBuf,
    pub account_label: String,
    pub confirmed_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvidenceImportReport {
    pub provider: String,
    pub account_label: String,
    pub source_system: String,
    pub source_sha256: String,
    pub evidence_entries: usize,
    pub matched_assets: usize,
    pub evidence_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudStatus {
    pub provider: String,
    pub account_label: String,
    pub evidence_imports: i64,
    pub evidence_entries: i64,
    pub matched_evidence_entries: i64,
    pub confirmed_assets: i64,
    pub transfer_batches: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StorageAuditReport {
    pub storage_roots: i64,
    pub evidence_entries: i64,
    pub canonical_source_keys: i64,
    pub entries_with_empty_text: i64,
    pub entries_without_dng_filename: i64,
    pub payloads_with_legacy_paths: i64,
    pub legacy_path_columns_present: bool,
    pub clean: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudPresencePlan {
    pub schema_version: u32,
    pub provider: String,
    pub account_label: String,
    pub remote_catalog_id: String,
    pub inventory_snapshot_sha256: String,
    pub inventory_asset_count: usize,
    pub verified_count: usize,
    pub unmapped_inventory_count: usize,
    pub keyword_path: Vec<String>,
    pub originals: Vec<VerifiedCloudOriginal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerifiedCloudOriginal {
    pub source_key: String,
    pub original_relative_path: String,
    pub original_filename: String,
    pub dng_filename: String,
    pub remote_asset_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventoryReconciliationReport {
    pub provider: String,
    pub account_label: String,
    pub remote_catalog_id: String,
    pub remote_asset_count: usize,
    pub remote_assets_with_filename: usize,
    pub remote_assets_with_sha256: usize,
    pub expected_evidence_count: usize,
    pub expected_without_recorded_dng_filename: usize,
    pub matched_by_recorded_filename: usize,
    pub matched_by_derived_name: usize,
    pub uniquely_matched_expected: usize,
    pub missing_expected: usize,
    pub ambiguous_expected: usize,
    pub unmatched_remote: usize,
    pub all_expected_present: bool,
    pub snapshot_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct ProetusRow {
    original_relative_path: String,
    file_name: String,
    dng_filename: Option<String>,
    version: String,
    part: String,
    group_name: String,
    group_num: i64,
    alt_text: String,
    is_monochrome: bool,
    upload_status: String,
    uploaded_at: Option<String>,
    delivery_status: String,
    delivered_at: Option<String>,
    delivery_batch: Option<String>,
    lifecycle_status: String,
    removed_at: Option<String>,
    removed_version: Option<String>,
    removal_note: Option<String>,
    last_seen_at: String,
}

#[derive(Clone, Debug)]
struct ExpectedEvidence {
    source_key: String,
    original_relative_path: String,
    original_filename: String,
    dng_filename: Option<String>,
}

pub async fn import_proetus_evidence(
    database: &Database,
    input: &ProetusImport,
) -> Result<EvidenceImportReport> {
    if !input.confirmed_present {
        return Err(PhotaraError::Configuration(
            "legacy Cloud evidence requires explicit --confirmed-present attestation".into(),
        ));
    }
    if input.account_label.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "Cloud account label cannot be empty".into(),
        ));
    }

    let before_hash = file_sha256(&input.database_path)?;
    let rows = read_proetus(&input.database_path).await?;
    let after_hash = file_sha256(&input.database_path)?;
    if before_hash != after_hash {
        return Err(PhotaraError::Configuration(
            "Proetus database changed while it was being imported; retry from a stable snapshot"
                .into(),
        ));
    }
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(
            "Proetus database contains no packaged assets".into(),
        ));
    }

    let mut transaction = database.begin().await?;
    let account_id: Uuid = sqlx::query_scalar(
        "INSERT INTO cloud_accounts (id, provider, label) VALUES ($1, $2, $3) \
         ON CONFLICT (provider, label) DO UPDATE SET updated_at = now() \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(input.account_label.trim())
    .fetch_one(&mut *transaction)
    .await?;

    let source_name = input
        .database_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("proetus.sqlite3");
    let import_id: Uuid = sqlx::query_scalar(
        "INSERT INTO cloud_evidence_imports \
         (id, account_id, source_system, evidence_kind, source_name, source_sha256, row_count, metadata) \
         VALUES ($1, $2, 'proetus-sqlite', 'user-confirmed', $3, $4, $5, $6) \
         ON CONFLICT (account_id, source_system, source_sha256) DO UPDATE SET \
             source_name = EXCLUDED.source_name, \
             row_count = EXCLUDED.row_count, \
             metadata = EXCLUDED.metadata \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(account_id)
    .bind(source_name)
    .bind(&before_hash)
    .bind(i32::try_from(rows.len()).map_err(|_| {
        PhotaraError::Configuration("Proetus evidence row count exceeds PostgreSQL integer".into())
    })?)
    .bind(serde_json::json!({
        "attestation": "user-confirmed-present",
        "required_status": {
            "upload": "uploaded",
            "delivery": "approved",
            "lifecycle": "removed"
        }
    }))
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query("DELETE FROM cloud_evidence_entries WHERE import_id = $1")
        .bind(import_id)
        .execute(&mut *transaction)
        .await?;
    let payloads = rows
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut insert = QueryBuilder::<Postgres>::new(
        "INSERT INTO cloud_evidence_entries \
         (import_id, source_key, storage_root_key, original_relative_path, original_filename, \
          dng_filename, source_payload) ",
    );
    insert.push_values(rows.iter().zip(&payloads), |mut values, (row, payload)| {
        let source_key = format!("{IMAGES_ROOT_KEY}:{}", row.original_relative_path);
        values
            .push_bind(import_id)
            .push_bind(source_key)
            .push_bind(IMAGES_ROOT_KEY)
            .push_bind(&row.original_relative_path)
            .push_bind(&row.file_name)
            .push_bind(&row.dng_filename)
            .push_bind(payload);
    });
    insert.build().execute(&mut *transaction).await?;
    transaction.commit().await?;

    Ok(EvidenceImportReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: input.account_label.trim().into(),
        source_system: "proetus-sqlite".into(),
        source_sha256: before_hash,
        evidence_entries: rows.len(),
        matched_assets: 0,
        evidence_kind: "user-confirmed".into(),
    })
}

pub async fn status(database: &Database, account_label: &str) -> Result<CloudStatus> {
    let row = sqlx::query("SELECT id FROM cloud_accounts WHERE provider = $1 AND label = $2")
        .bind(ADOBE_LIGHTROOM_PROVIDER)
        .bind(account_label)
        .fetch_optional(database.pool())
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "Adobe Lightroom Cloud account {account_label:?} was not found"
            ))
        })?;
    let account_id: Uuid = row.try_get("id")?;
    let counts = sqlx::query(
        "SELECT \
           (SELECT COUNT(*) FROM cloud_evidence_imports WHERE account_id = $1) AS imports, \
           (SELECT COUNT(*) FROM cloud_evidence_entries entry \
              JOIN cloud_evidence_imports evidence_import ON evidence_import.id = entry.import_id \
             WHERE evidence_import.account_id = $1) AS entries, \
           (SELECT COUNT(*) FROM cloud_evidence_entries entry \
              JOIN cloud_evidence_imports evidence_import ON evidence_import.id = entry.import_id \
             WHERE evidence_import.account_id = $1 AND entry.matched_asset_id IS NOT NULL) AS matched, \
           (SELECT COUNT(*) FROM asset_cloud_presence WHERE account_id = $1 \
             AND status = 'present') AS present, \
           (SELECT COUNT(*) FROM cloud_transfer_batches WHERE account_id = $1) AS batches",
    )
    .bind(account_id)
    .fetch_one(database.pool())
    .await?;
    Ok(CloudStatus {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        evidence_imports: counts.try_get("imports")?,
        evidence_entries: counts.try_get("entries")?,
        matched_evidence_entries: counts.try_get("matched")?,
        confirmed_assets: counts.try_get("present")?,
        transfer_batches: counts.try_get("batches")?,
    })
}

pub async fn storage_audit(database: &Database) -> Result<StorageAuditReport> {
    let row = sqlx::query(
        "SELECT \
           (SELECT COUNT(*) FROM storage_roots) AS storage_roots, \
           COUNT(*) AS evidence_entries, \
           COUNT(*) FILTER (WHERE source_key = storage_root_key || ':' || original_relative_path) \
               AS canonical_source_keys, \
           COUNT(*) FILTER (WHERE source_key = '' OR storage_root_key = '' \
               OR original_relative_path = '' OR original_filename = '' OR dng_filename = '') \
               AS entries_with_empty_text, \
           COUNT(*) FILTER (WHERE dng_filename IS NULL) AS entries_without_dng_filename, \
           COUNT(*) FILTER (WHERE source_payload ?| ARRAY['source_path', 'dng_path']) \
               AS payloads_with_legacy_paths, \
           EXISTS ( \
               SELECT 1 FROM information_schema.columns \
               WHERE table_schema = current_schema() \
                 AND table_name = 'cloud_evidence_entries' \
                 AND column_name IN ('original_path', 'dng_path') \
           ) AS legacy_path_columns_present \
         FROM cloud_evidence_entries",
    )
    .fetch_one(database.pool())
    .await?;
    let evidence_entries: i64 = row.try_get("evidence_entries")?;
    let canonical_source_keys: i64 = row.try_get("canonical_source_keys")?;
    let entries_with_empty_text: i64 = row.try_get("entries_with_empty_text")?;
    let entries_without_dng_filename: i64 = row.try_get("entries_without_dng_filename")?;
    let payloads_with_legacy_paths: i64 = row.try_get("payloads_with_legacy_paths")?;
    let legacy_path_columns_present: bool = row.try_get("legacy_path_columns_present")?;
    Ok(StorageAuditReport {
        storage_roots: row.try_get("storage_roots")?,
        evidence_entries,
        canonical_source_keys,
        entries_with_empty_text,
        entries_without_dng_filename,
        payloads_with_legacy_paths,
        legacy_path_columns_present,
        clean: canonical_source_keys == evidence_entries
            && entries_with_empty_text == 0
            && entries_without_dng_filename == 0
            && payloads_with_legacy_paths == 0
            && !legacy_path_columns_present,
    })
}

pub async fn presence_plan(database: &Database, account_label: &str) -> Result<CloudPresencePlan> {
    let account = sqlx::query(
        "SELECT id, remote_catalog_id FROM cloud_accounts WHERE provider = $1 AND label = $2",
    )
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "Adobe Lightroom Cloud account {account_label:?} was not found"
        ))
    })?;
    let account_id: Uuid = account.try_get("id")?;
    let remote_catalog_id: String = account
        .try_get::<Option<String>, _>("remote_catalog_id")?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "Adobe Lightroom Cloud account {account_label:?} has no verified catalog"
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
            "Adobe Lightroom Cloud account {account_label:?} has no inventory; run adobe-inventory"
        ))
    })?;
    let run_id: Uuid = inventory.try_get("id")?;
    let asset_count: i32 = inventory.try_get("asset_count")?;
    let evidence_import = sqlx::query(
        "SELECT id FROM cloud_evidence_imports \
         WHERE account_id = $1 AND source_system = 'proetus-sqlite' \
         ORDER BY imported_at DESC, id DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "Adobe Lightroom Cloud account {account_label:?} has no Proetus evidence import"
        ))
    })?;
    let import_id: Uuid = evidence_import.try_get("id")?;
    let originals = sqlx::query(
        "WITH legacy AS ( \
             SELECT evidence.source_key, evidence.original_relative_path, \
                    evidence.original_filename, evidence.dng_filename, evidence.remote_asset_id \
             FROM cloud_evidence_entries AS evidence \
             JOIN cloud_provider_inventory_assets AS inventory \
               ON inventory.run_id = $1 AND inventory.remote_asset_id = evidence.remote_asset_id \
             WHERE evidence.import_id = $2 \
               AND evidence.dng_filename IS NOT NULL \
               AND evidence.remote_asset_id IS NOT NULL \
         ), registered AS ( \
             SELECT raw.location AS source_key, \
                    substr(raw.location, length('images:') + 1) AS original_relative_path, \
                    asset.original_filename, inventory.file_name AS dng_filename, \
                    presence.remote_asset_id \
             FROM asset_cloud_presence AS presence \
             JOIN assets AS asset ON asset.id = presence.asset_id \
             JOIN asset_files AS raw \
               ON raw.asset_id = asset.id \
              AND raw.representation = 'camera-raw' \
              AND raw.authoritative \
              AND raw.state = 'current' \
             JOIN cloud_provider_inventory_assets AS inventory \
               ON inventory.run_id = $1 AND inventory.remote_asset_id = presence.remote_asset_id \
             WHERE presence.account_id = $3 \
               AND presence.status = 'present' \
               AND inventory.file_name IS NOT NULL \
               AND NOT EXISTS ( \
                   SELECT 1 FROM legacy \
                   WHERE legacy.remote_asset_id = presence.remote_asset_id \
               ) \
         ) \
         SELECT * FROM legacy \
         UNION ALL \
         SELECT * FROM registered \
         ORDER BY original_relative_path",
    )
    .bind(run_id)
    .bind(import_id)
    .bind(account_id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(VerifiedCloudOriginal {
            source_key: row.try_get("source_key")?,
            original_relative_path: row.try_get("original_relative_path")?,
            original_filename: row.try_get("original_filename")?,
            dng_filename: row.try_get("dng_filename")?,
            remote_asset_id: row.try_get("remote_asset_id")?,
        })
    })
    .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    let unique_remote_ids = originals
        .iter()
        .map(|original| original.remote_asset_id.as_str())
        .collect::<HashSet<_>>();
    if unique_remote_ids.len() != originals.len() {
        return Err(PhotaraError::Configuration(
            "Cloud presence plan contains duplicate Adobe asset IDs".into(),
        ));
    }
    let unique_source_keys = originals
        .iter()
        .map(|original| original.source_key.as_str())
        .collect::<HashSet<_>>();
    if unique_source_keys.len() != originals.len() {
        return Err(PhotaraError::Configuration(
            "Cloud presence plan maps more than one Adobe asset to the same camera original; no \
             Lightroom metadata was changed"
                .into(),
        ));
    }
    let inventory_asset_count = usize::try_from(asset_count).map_err(|_| {
        PhotaraError::Configuration("Adobe inventory reported a negative asset count".into())
    })?;
    let unmapped_inventory_count = inventory_asset_count.saturating_sub(originals.len());
    Ok(CloudPresencePlan {
        schema_version: 1,
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        remote_catalog_id,
        inventory_snapshot_sha256: inventory.try_get("snapshot_sha256")?,
        inventory_asset_count,
        verified_count: originals.len(),
        unmapped_inventory_count,
        keyword_path: vec!["workflow".into(), "cloud".into(), "present".into()],
        originals,
    })
}

pub async fn register_remote_catalog(
    database: &Database,
    account_label: &str,
    remote_catalog_id: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cloud_accounts (id, provider, label, remote_catalog_id) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (provider, label) DO UPDATE SET \
             remote_catalog_id = EXCLUDED.remote_catalog_id, updated_at = now()",
    )
    .bind(Uuid::new_v4())
    .bind(ADOBE_LIGHTROOM_PROVIDER)
    .bind(account_label)
    .bind(remote_catalog_id)
    .execute(database.pool())
    .await?;
    Ok(())
}

pub async fn record_adobe_inventory(
    database: &Database,
    account_label: &str,
    inventory: &AdobeInventory,
) -> Result<InventoryReconciliationReport> {
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
    let serialized = serde_json::to_vec(&inventory.assets)?;
    let snapshot_sha256 = format!("{:x}", Sha256::digest(&serialized));
    let remote_assets_with_filename = inventory
        .assets
        .iter()
        .filter(|asset| {
            asset
                .payload
                .import_source
                .as_ref()
                .and_then(|source| source.file_name.as_deref())
                .is_some()
        })
        .count();
    let remote_assets_with_sha256 = inventory
        .assets
        .iter()
        .filter(|asset| {
            asset
                .payload
                .import_source
                .as_ref()
                .and_then(|source| source.sha256.as_deref())
                .is_some()
        })
        .count();

    let mut transaction = database.begin().await?;
    let run_id: Uuid = sqlx::query_scalar(
        "INSERT INTO cloud_provider_inventory_runs \
         (id, account_id, remote_catalog_id, snapshot_sha256, asset_count, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (account_id, snapshot_sha256) DO UPDATE SET \
             remote_catalog_id = EXCLUDED.remote_catalog_id, \
             asset_count = EXCLUDED.asset_count, metadata = EXCLUDED.metadata, \
             completed_at = now() \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(account_id)
    .bind(&inventory.catalog_id)
    .bind(&snapshot_sha256)
    .bind(i32::try_from(inventory.assets.len()).map_err(|_| {
        PhotaraError::Configuration("Adobe inventory exceeds PostgreSQL integer range".into())
    })?)
    .bind(serde_json::json!({
        "subtype": "image",
        "exclude": "incomplete",
        "page_limit": 500
    }))
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM cloud_provider_inventory_assets WHERE run_id = $1")
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
    if !inventory.assets.is_empty() {
        let payloads = inventory
            .assets
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut insert = QueryBuilder::<Postgres>::new(
            "INSERT INTO cloud_provider_inventory_assets \
             (run_id, remote_asset_id, subtype, file_name, sha256, capture_date, source_payload) ",
        );
        insert.push_values(
            inventory.assets.iter().zip(&payloads),
            |mut values, (asset, payload)| {
                let import = asset.payload.import_source.as_ref();
                values
                    .push_bind(run_id)
                    .push_bind(&asset.id)
                    .push_bind(&asset.subtype)
                    .push_bind(import.and_then(|source| source.file_name.as_deref()))
                    .push_bind(import.and_then(|source| source.sha256.as_deref()))
                    .push_bind(asset.payload.capture_date.as_deref())
                    .push_bind(payload);
            },
        );
        insert.build().execute(&mut *transaction).await?;
    }

    let evidence_import_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM cloud_evidence_imports \
         WHERE account_id = $1 AND source_system = 'proetus-sqlite' \
         ORDER BY imported_at DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let mut expected: Vec<ExpectedEvidence> = Vec::new();
    if let Some(import_id) = evidence_import_id {
        expected = sqlx::query(
            "SELECT source_key, original_relative_path, original_filename, dng_filename \
             FROM cloud_evidence_entries \
             WHERE import_id = $1 ORDER BY source_key",
        )
        .bind(import_id)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ExpectedEvidence {
                source_key: row.try_get("source_key")?,
                original_relative_path: row.try_get("original_relative_path")?,
                original_filename: row.try_get("original_filename")?,
                dng_filename: row.try_get("dng_filename")?,
            })
        })
        .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    }

    let mut remote_by_filename: HashMap<&str, Vec<&str>> = HashMap::new();
    for asset in &inventory.assets {
        if let Some(filename) = asset
            .payload
            .import_source
            .as_ref()
            .and_then(|source| source.file_name.as_deref())
        {
            remote_by_filename
                .entry(filename)
                .or_default()
                .push(&asset.id);
        }
    }
    let mut expected_filename_counts: HashMap<&str, usize> = HashMap::new();
    for evidence in &expected {
        if let Some(filename) = evidence.dng_filename.as_deref() {
            *expected_filename_counts.entry(filename).or_default() += 1;
        }
    }
    let mut matched = Vec::new();
    let mut used_remote_ids = HashSet::new();
    let mut ambiguous_expected = 0;
    for evidence in &expected {
        let Some(filename) = evidence.dng_filename.as_deref() else {
            continue;
        };
        let remote = remote_by_filename.get(filename);
        if expected_filename_counts.get(filename) == Some(&1)
            && remote.is_some_and(|assets| assets.len() == 1)
        {
            matched.push((evidence.source_key.as_str(), remote.unwrap()[0]));
            used_remote_ids.insert(remote.unwrap()[0]);
        } else if remote.is_some()
            || expected_filename_counts
                .get(filename)
                .is_some_and(|n| *n > 1)
        {
            ambiguous_expected += 1;
        }
    }
    let matched_by_recorded_filename = matched.len();
    let derived_prefixes = expected
        .iter()
        .filter(|evidence| evidence.dng_filename.is_none())
        .map(|evidence| {
            derived_dng_prefix(
                &evidence.original_relative_path,
                &evidence.original_filename,
            )
            .map(|prefix| (evidence.source_key.as_str(), prefix))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut derived_prefix_counts: HashMap<&str, usize> = HashMap::new();
    for (_, prefix) in &derived_prefixes {
        *derived_prefix_counts.entry(prefix).or_default() += 1;
    }
    for (source_key, prefix) in &derived_prefixes {
        let candidates = remote_by_filename
            .iter()
            .filter(|(filename, assets)| {
                filename.starts_with(prefix.as_str())
                    && filename.to_ascii_lowercase().ends_with(".dng")
                    && assets.len() == 1
                    && !used_remote_ids.contains(assets[0])
            })
            .map(|(_, assets)| assets[0])
            .collect::<Vec<_>>();
        if derived_prefix_counts.get(prefix.as_str()) == Some(&1) && candidates.len() == 1 {
            matched.push((source_key, candidates[0]));
            used_remote_ids.insert(candidates[0]);
        } else if !candidates.is_empty()
            || derived_prefix_counts
                .get(prefix.as_str())
                .is_some_and(|count| *count > 1)
        {
            ambiguous_expected += 1;
        }
    }
    let matched_by_derived_name = matched.len() - matched_by_recorded_filename;
    if let Some(import_id) = evidence_import_id {
        sqlx::query(
            "UPDATE cloud_evidence_entries SET remote_asset_id = NULL WHERE import_id = $1",
        )
        .bind(import_id)
        .execute(&mut *transaction)
        .await?;
        if !matched.is_empty() {
            let mut update = QueryBuilder::<Postgres>::new(
                "UPDATE cloud_evidence_entries AS evidence SET remote_asset_id = matched.remote_id \
                 FROM (",
            );
            update.push_values(&matched, |mut values, (source_key, remote_id)| {
                values.push_bind(source_key).push_bind(remote_id);
            });
            update.push(
                ") AS matched(source_key, remote_id) \
                 WHERE evidence.import_id = ",
            );
            update.push_bind(import_id);
            update.push(" AND evidence.source_key = matched.source_key");
            update.build().execute(&mut *transaction).await?;
        }
    }
    transaction.commit().await?;

    let matched_remote_ids: HashSet<_> = matched.iter().map(|(_, remote)| *remote).collect();
    let missing_expected = expected.len().saturating_sub(matched.len());
    Ok(InventoryReconciliationReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        remote_catalog_id: inventory.catalog_id.clone(),
        remote_asset_count: inventory.assets.len(),
        remote_assets_with_filename,
        remote_assets_with_sha256,
        expected_evidence_count: expected.len(),
        expected_without_recorded_dng_filename: expected
            .iter()
            .filter(|evidence| evidence.dng_filename.is_none())
            .count(),
        matched_by_recorded_filename,
        matched_by_derived_name,
        uniquely_matched_expected: matched.len(),
        missing_expected,
        ambiguous_expected,
        unmatched_remote: inventory
            .assets
            .len()
            .saturating_sub(matched_remote_ids.len()),
        all_expected_present: !expected.is_empty()
            && missing_expected == 0
            && ambiguous_expected == 0,
        snapshot_sha256,
    })
}

fn derived_dng_prefix(original_relative_path: &str, original_filename: &str) -> Result<String> {
    let date = Path::new(original_relative_path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .filter(|value| {
            value.len() == 10
                && value.as_bytes()[4] == b'-'
                && value.as_bytes()[7] == b'-'
                && value
                    .bytes()
                    .enumerate()
                    .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
        })
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "legacy path {original_relative_path:?} has no canonical dated parent"
            ))
        })?;
    let stem = Path::new(original_filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "legacy filename {original_filename:?} has no stem"
            ))
        })?;
    Ok(format!("{stem}_{}_", date.replace('-', "_")))
}

async fn read_proetus(path: &Path) -> Result<Vec<ProetusRow>> {
    if !path.is_file() {
        return Err(PhotaraError::Configuration(format!(
            "Proetus database {} is not a file",
            path.display()
        )));
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);
    let mut connection = SqliteConnection::connect_with(&options).await?;
    let records = sqlx::query(
        "SELECT source_path, version, dng_path, file_name, part, group_name, group_num, \
                alt_text, is_monochrome, upload_status, uploaded_at, delivery_status, \
                delivered_at, delivery_batch, lifecycle_status, removed_at, \
                removed_version, removal_note, last_seen_at \
         FROM packaged_assets ORDER BY source_path",
    )
    .fetch_all(&mut connection)
    .await?;
    connection.close().await?;

    records
        .into_iter()
        .map(|record| {
            let source_path: String = record.try_get("source_path")?;
            let upload_status: String = record.try_get("upload_status")?;
            let delivery_status: String = record.try_get("delivery_status")?;
            let lifecycle_status: String = record.try_get("lifecycle_status")?;
            if upload_status != "uploaded"
                || delivery_status != "approved"
                || lifecycle_status != "removed"
            {
                return Err(PhotaraError::Configuration(format!(
                    "legacy asset {source_path:?} is not uploaded/approved/removed"
                )));
            }
            let dng_path: Option<String> = record.try_get("dng_path")?;
            Ok(ProetusRow {
                original_relative_path: archive_relative_path(Path::new(&source_path))?,
                dng_filename: dng_path.as_deref().and_then(path_filename),
                file_name: record.try_get("file_name")?,
                version: record.try_get("version")?,
                part: record.try_get("part")?,
                group_name: record.try_get("group_name")?,
                group_num: record.try_get("group_num")?,
                alt_text: record.try_get("alt_text")?,
                is_monochrome: record.try_get::<i64, _>("is_monochrome")? != 0,
                upload_status,
                uploaded_at: record.try_get("uploaded_at")?,
                delivery_status,
                delivered_at: record.try_get("delivered_at")?,
                delivery_batch: record.try_get("delivery_batch")?,
                lifecycle_status,
                removed_at: record.try_get("removed_at")?,
                removed_version: record.try_get("removed_version")?,
                removal_note: record.try_get("removal_note")?,
                last_seen_at: record.try_get("last_seen_at")?,
            })
        })
        .collect()
}

fn archive_relative_path(path: &Path) -> Result<String> {
    let components: Vec<_> = path.components().collect();
    let images = components
        .iter()
        .position(|component| {
            matches!(component, Component::Normal(value) if value.eq_ignore_ascii_case("images"))
        })
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "legacy source path {} has no Images component",
                path.display()
            ))
        })?;
    let relative: PathBuf = components[images + 1..].iter().collect();
    if relative.as_os_str().is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "legacy source path {} has nothing below Images",
            path.display()
        )));
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn path_filename(value: &str) -> Option<String> {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|error| PhotaraError::filesystem("read legacy Cloud evidence", path, error))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::{archive_relative_path, derived_dng_prefix};
    use std::path::Path;

    #[test]
    fn derives_portable_archive_path_below_images_root() {
        assert_eq!(
            archive_relative_path(Path::new(
                "/Volumes/Orion/Pictures/Images/2020/2020-08/2020-08-08/DSC00009.ARW"
            ))
            .unwrap(),
            "2020/2020-08/2020-08-08/DSC00009.ARW"
        );
    }

    #[test]
    fn derives_legacy_dng_prefix_without_assuming_author() {
        assert_eq!(
            derived_dng_prefix("2017/2017-07/2017-07-01/DSC02717.ARW", "DSC02717.ARW").unwrap(),
            "DSC02717_2017_07_01_"
        );
    }
}
