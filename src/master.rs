use std::{
    fs,
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{
    PhotaraError, Result, asset::downstream_basename, config::PhotaraConfig, project::ProjectRecord,
};

const MANIFEST_NAME: &str = "photara-master-manifest.json";
const REPORT_NAME: &str = "photara-photoshop-report.json";
const SCRIPT_NAME: &str = "Build Photara Masters.psjs";
const PHOTOSHOP_SCRIPT: &str = include_str!("../photoshop/Build Photara Masters.psjs");
const FLATTENING_MANIFEST_NAME: &str = "photara-flattening-manifest.json";
const FLATTENING_REPORT_NAME: &str = "photara-flattening-report.json";
const FLATTENING_HANDOFF_MANIFEST_NAME: &str = "Photara Flattening Manifest.json";
const FLATTENING_HANDOFF_REPORT_NAME: &str = "Photara Flattening Report.json";
const FLATTENING_SCRIPT_NAME: &str = "Flatten Photara Masters.psjs";
const FLATTENING_SCRIPT: &str = include_str!("../photoshop/Flatten Photara Masters.psjs");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MasterManifest {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub document_contract: DocumentContract,
    pub staging_root: PathBuf,
    pub incoming_directory: PathBuf,
    pub output_directory: PathBuf,
    pub photoshop_script: PathBuf,
    pub items: Vec<MasterManifestItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentContract {
    pub bits_per_channel: u8,
    pub color_profile_family: String,
    pub require_hdr: bool,
    pub require_embedded_smart_object: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MasterManifestItem {
    pub asset_id: Uuid,
    pub source_key: String,
    pub camera_raw_path: PathBuf,
    pub original_filename: String,
    pub downstream_basename: String,
    pub dng_filename: String,
    pub psb_filename: String,
    pub dng_relative_path: PathBuf,
    pub psb_relative_path: PathBuf,
    pub canary: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MasterStagingStatus {
    pub schema_version: u32,
    pub project: String,
    pub staging_root: PathBuf,
    pub expected_count: usize,
    pub ready_dng_count: usize,
    pub existing_psb_count: usize,
    pub missing_dngs: Vec<String>,
    pub unexpected_dngs: Vec<String>,
    pub ready: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PhotoshopReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub items: Vec<PhotoshopReportItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PhotoshopReportItem {
    pub asset_id: Uuid,
    pub dng_filename: String,
    pub psb_filename: String,
    pub state: String,
    #[serde(default)]
    pub bits_per_channel: Option<u8>,
    #[serde(default)]
    pub color_profile: Option<String>,
    #[serde(default)]
    pub smart_object: Option<bool>,
    #[serde(default)]
    pub linked: Option<bool>,
    #[serde(default)]
    pub file_reference: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MasterVerification {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub verified_count: usize,
    pub items: Vec<VerifiedMaster>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerifiedMaster {
    pub asset_id: Uuid,
    pub dng_path: PathBuf,
    pub psb_path: PathBuf,
    pub psb_sha256: String,
    pub psb_byte_size: u64,
    pub bits_per_channel: u8,
    pub color_profile: String,
    pub embedded_smart_object: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct MasterPromotion {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub confirmed: bool,
    pub total_byte_size: u64,
    pub items: Vec<PromotedMaster>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PromotedMaster {
    pub asset_id: Uuid,
    pub source_psb: PathBuf,
    pub destination_psb: PathBuf,
    pub logical_location: String,
    pub psb_sha256: String,
    pub psb_byte_size: u64,
    pub color_profile: String,
    pub workflow_state: String,
    pub action: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MasterCheckpoint {
    pub schema_version: u32,
    pub project: String,
    pub target_state: String,
    pub recorded: bool,
    pub items: Vec<CheckpointedMaster>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckpointedMaster {
    pub asset_id: Uuid,
    pub psb_path: PathBuf,
    pub psb_sha256: String,
    pub psb_byte_size: u64,
    pub bits_per_channel: u8,
    pub previous_state: String,
    pub target_state: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlatteningManifest {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub project_root: PathBuf,
    pub images_root: PathBuf,
    pub output_directory: PathBuf,
    pub photoshop_script: PathBuf,
    pub items: Vec<FlatteningManifestItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlatteningManifestItem {
    pub asset_id: Uuid,
    pub layered_file_id: Uuid,
    pub psb_filename: String,
    pub psb_relative_path: PathBuf,
    pub psb_sha256: String,
    pub psb_byte_size: u64,
    pub tiff_filename: String,
    pub tiff_relative_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlatteningReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub items: Vec<FlatteningReportItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlatteningReportItem {
    pub asset_id: Uuid,
    pub psb_filename: String,
    pub tiff_filename: String,
    pub state: String,
    #[serde(default)]
    pub bits_per_channel: Option<u8>,
    #[serde(default)]
    pub color_profile: Option<String>,
    #[serde(default)]
    pub layer_count: Option<usize>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FlatteningVerification {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub verified_count: usize,
    pub items: Vec<VerifiedFlattenedMaster>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerifiedFlattenedMaster {
    pub asset_id: Uuid,
    pub layered_file_id: Uuid,
    pub psb_path: PathBuf,
    pub tiff_path: PathBuf,
    pub tiff_sha256: String,
    pub tiff_byte_size: u64,
    pub bits_per_channel: u8,
    pub color_profile: String,
    pub layer_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct FlatteningRegistration {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub confirmed: bool,
    pub items: Vec<RegisteredFlattenedMaster>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegisteredFlattenedMaster {
    pub asset_id: Uuid,
    pub tiff_path: PathBuf,
    pub logical_location: String,
    pub tiff_sha256: String,
    pub tiff_byte_size: u64,
    pub workflow_state: String,
    pub action: String,
}

pub async fn prepare(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    canary_filename: Option<&str>,
) -> Result<MasterManifest> {
    validate_project_slug(&project.slug)?;
    let root = config.settings.lightroom_inbox.clone();
    let workspace = root.join(".photara");
    let incoming = root.clone();
    let output = workspace.join("output");
    create_directory(&root)?;
    create_directory(&workspace)?;
    create_directory(&incoming)?;
    create_directory(&output)?;

    let rows = sqlx::query(
        "SELECT asset.id, asset.original_filename, asset.original_stem, asset.capture_date, \
                asset.author_code, asset.original_sha256, file.location \
         FROM project_asset_decisions AS decision \
         JOIN assets AS asset ON asset.id = decision.asset_id \
         JOIN asset_files AS file ON file.asset_id = asset.id \
           AND file.representation = 'camera-raw' AND file.state = 'current' \
         WHERE decision.project_id = $1 AND decision.decision = 'photographer-final' \
           AND decision.selected \
         ORDER BY asset.capture_date, asset.original_filename",
    )
    .bind(project.id)
    .fetch_all(database.pool())
    .await?;
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project {:?} has no current Photographer Final assets",
            project.slug
        )));
    }

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let asset_id: Uuid = row.try_get("id")?;
        let original_filename: String = row.try_get("original_filename")?;
        let original_stem: String = row.try_get("original_stem")?;
        let capture_date: NaiveDate = row.try_get("capture_date")?;
        let author_code: String = row.try_get("author_code")?;
        let original_sha256: String = row.try_get("original_sha256")?;
        let source_key: String = row.try_get("location")?;
        let camera_raw_path = resolve_source_key(&config.settings.images_root, &source_key)?;
        inspect_regular_file(&camera_raw_path, "camera RAW")?;
        let downstream_basename = downstream_basename(
            &original_stem,
            capture_date,
            &author_code,
            &original_sha256,
            false,
        )?;
        let dng_filename = format!("{downstream_basename}.DNG");
        let psb_filename = format!("{downstream_basename}.PSB");
        let is_canary = canary_filename.is_some_and(|value| {
            value.eq_ignore_ascii_case(&dng_filename)
                || value.eq_ignore_ascii_case(&original_filename)
                || value.eq_ignore_ascii_case(&original_stem)
        });
        let dng_relative_path = PathBuf::from(&dng_filename);
        let psb_relative_path = PathBuf::from(".photara/output").join(&psb_filename);
        items.push(MasterManifestItem {
            asset_id,
            source_key: source_key.clone(),
            camera_raw_path,
            original_filename,
            downstream_basename,
            dng_filename,
            psb_filename,
            dng_relative_path,
            psb_relative_path,
            canary: is_canary,
        });
    }
    if let Some(requested) = canary_filename
        && !items.iter().any(|item| item.canary)
    {
        return Err(PhotaraError::Configuration(format!(
            "canary {requested:?} is not a current Photographer Final asset"
        )));
    }

    let batch_id = match read_json::<MasterManifest>(&workspace.join(MANIFEST_NAME)) {
        Ok(existing)
            if existing.project == project.slug
                && existing.items.len() == items.len()
                && existing
                    .items
                    .iter()
                    .map(|item| item.asset_id)
                    .eq(items.iter().map(|item| item.asset_id)) =>
        {
            existing.batch_id
        }
        Ok(_) => Uuid::new_v4(),
        Err(PhotaraError::Filesystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Uuid::new_v4()
        }
        Err(error) => return Err(error),
    };
    let manifest = MasterManifest {
        schema_version: 2,
        batch_id,
        project: project.slug.clone(),
        document_contract: DocumentContract {
            bits_per_channel: 16,
            color_profile_family: "P3 PQ".into(),
            require_hdr: true,
            require_embedded_smart_object: true,
        },
        staging_root: root.clone(),
        incoming_directory: incoming,
        output_directory: output,
        photoshop_script: root.join(SCRIPT_NAME),
        items,
    };
    write_json_atomic(workspace.join(MANIFEST_NAME), &manifest)?;
    write_atomic(root.join(SCRIPT_NAME), PHOTOSHOP_SCRIPT.as_bytes())?;
    Ok(manifest)
}

pub fn status(config: &PhotaraConfig, project: &str) -> Result<MasterStagingStatus> {
    let manifest = load_manifest(config, project)?;
    let mut missing_dngs = Vec::new();
    let mut existing_psb_count = 0;
    for item in &manifest.items {
        if !manifest
            .staging_root
            .join(&item.dng_relative_path)
            .is_file()
        {
            missing_dngs.push(item.dng_filename.clone());
        }
        if manifest
            .staging_root
            .join(&item.psb_relative_path)
            .is_file()
        {
            existing_psb_count += 1;
        }
    }
    let expected = manifest
        .items
        .iter()
        .map(|item| item.dng_filename.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    let mut unexpected_dngs = fs::read_dir(&manifest.incoming_directory)
        .map_err(|source| {
            PhotaraError::filesystem("read Lightroom inbox", &manifest.incoming_directory, source)
        })?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let is_dng = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dng"));
            let filename = path.file_name()?.to_str()?.to_owned();
            (path.is_file() && is_dng && !expected.contains(&filename.to_ascii_lowercase()))
                .then_some(filename)
        })
        .collect::<Vec<_>>();
    unexpected_dngs.sort();
    let ready_dng_count = manifest.items.len() - missing_dngs.len();
    let ready = missing_dngs.is_empty() && unexpected_dngs.is_empty();
    Ok(MasterStagingStatus {
        schema_version: 2,
        project: manifest.project,
        staging_root: manifest.staging_root,
        expected_count: manifest.items.len(),
        ready_dng_count,
        existing_psb_count,
        ready,
        missing_dngs,
        unexpected_dngs,
    })
}

pub fn verify(config: &PhotaraConfig, project: &str) -> Result<MasterVerification> {
    let manifest = load_manifest(config, project)?;
    let report_path = manifest.staging_root.join(".photara").join(REPORT_NAME);
    let report: PhotoshopReport = read_json(&report_path)?;
    if report.batch_id != manifest.batch_id || report.project != manifest.project {
        return Err(PhotaraError::Configuration(
            "Photoshop report does not belong to the current master manifest".into(),
        ));
    }
    let mut verified = Vec::with_capacity(manifest.items.len());
    for item in &manifest.items {
        let evidence = report
            .items
            .iter()
            .find(|entry| entry.asset_id == item.asset_id)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "Photoshop report has no entry for {}",
                    item.dng_filename
                ))
            })?;
        if evidence.state != "verified"
            || evidence.bits_per_channel != Some(16)
            || evidence.smart_object != Some(true)
            || evidence.linked != Some(false)
            || evidence.file_reference.as_deref() != Some(item.dng_filename.as_str())
        {
            return Err(PhotaraError::Configuration(format!(
                "Photoshop did not verify the master contract for {}: {:?}",
                item.dng_filename, evidence.error
            )));
        }
        let profile = evidence.color_profile.as_deref().unwrap_or_default();
        if !is_p3_pq(profile) {
            return Err(PhotaraError::Configuration(format!(
                "{} uses unexpected profile {profile:?}; expected a P3 PQ profile",
                item.psb_filename
            )));
        }
        let dng_path = manifest.staging_root.join(&item.dng_relative_path);
        let psb_path = manifest.staging_root.join(&item.psb_relative_path);
        inspect_regular_file(&dng_path, "DNG")?;
        let (psb_size, psb_bits) = inspect_psb(&psb_path)?;
        if psb_bits != 16 {
            return Err(PhotaraError::Configuration(format!(
                "{} is {psb_bits} bits per channel; the initial embedded-DNG build must be 16-bit",
                psb_path.display()
            )));
        }
        verified.push(VerifiedMaster {
            asset_id: item.asset_id,
            dng_path,
            psb_sha256: sha256(&psb_path)?,
            psb_byte_size: psb_size,
            psb_path,
            bits_per_channel: 16,
            color_profile: profile.into(),
            embedded_smart_object: true,
        });
    }
    Ok(MasterVerification {
        schema_version: 1,
        batch_id: manifest.batch_id,
        project: manifest.project,
        verified_count: verified.len(),
        items: verified,
    })
}

pub async fn promote(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    confirmed: bool,
) -> Result<MasterPromotion> {
    let manifest = load_manifest(config, &project.slug)?;
    for item in &manifest.items {
        if let Err(error) = inspect_regular_file(&item.camera_raw_path, "camera RAW") {
            return Err(PhotaraError::Configuration(format!(
                "archive preflight failed for {} using images root {}: {error}. Update \
                 images_root/PHOTARA_IMAGES_ROOT and rerun `photara masters prepare {}`",
                item.original_filename,
                config.settings.images_root.display(),
                project.slug
            )));
        }
    }
    let verification = verify(config, &project.slug)?;
    let mut items = Vec::with_capacity(verification.items.len());
    let mut total_byte_size = 0_u64;

    for verified in verification.items {
        let manifest_item = manifest
            .items
            .iter()
            .find(|item| item.asset_id == verified.asset_id)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "verified asset {} is missing from the master manifest",
                    verified.asset_id
                ))
            })?;
        let destination = manifest_item
            .camera_raw_path
            .parent()
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "camera RAW {} has no parent directory",
                    manifest_item.camera_raw_path.display()
                ))
            })?
            .join(&manifest_item.psb_filename);
        let logical_location =
            layered_psb_location(&manifest_item.source_key, &manifest_item.psb_filename)?;
        let destination_matches = if destination.is_file() {
            inspect_psb(&destination)?.0 == verified.psb_byte_size
                && sha256(&destination)? == verified.psb_sha256
        } else {
            false
        };
        if destination.exists() && !destination_matches {
            return Err(PhotaraError::Configuration(format!(
                "destination {} already exists with different contents",
                destination.display()
            )));
        }

        let existing = sqlx::query(
            "SELECT file.id, file.location, file.sha256, file.byte_size, document.workflow_state \
             FROM asset_files AS file \
             JOIN layered_master_documents AS document ON document.asset_file_id = file.id \
             WHERE file.asset_id = $1 AND file.representation = 'layered-psb' \
               AND file.authoritative AND file.state = 'current'",
        )
        .bind(verified.asset_id)
        .fetch_optional(database.pool())
        .await?;
        let already_registered = if let Some(row) = existing {
            let location: String = row.try_get("location")?;
            let hash: Option<String> = row.try_get("sha256")?;
            let bytes: Option<i64> = row.try_get("byte_size")?;
            if location != logical_location
                || hash.as_deref() != Some(&verified.psb_sha256)
                || bytes != Some(to_i64(verified.psb_byte_size, "PSB")?)
                || !destination_matches
            {
                return Err(PhotaraError::Configuration(format!(
                    "registered layered PSB for {} does not match the verified destination",
                    manifest_item.original_filename
                )));
            }
            true
        } else {
            false
        };
        total_byte_size = total_byte_size
            .checked_add(verified.psb_byte_size)
            .ok_or_else(|| PhotaraError::Configuration("PSB batch size overflowed".into()))?;
        items.push(PromotedMaster {
            asset_id: verified.asset_id,
            source_psb: verified.psb_path,
            destination_psb: destination,
            logical_location,
            psb_sha256: verified.psb_sha256,
            psb_byte_size: verified.psb_byte_size,
            color_profile: verified.color_profile,
            workflow_state: "editing".into(),
            action: if already_registered {
                "already-promoted".into()
            } else if destination_matches {
                "register-existing-copy".into()
            } else {
                "copy-and-register".into()
            },
        });
    }

    if confirmed {
        for item in &mut items {
            if item.action == "already-promoted" {
                remove_redundant_source(&item.source_psb, &item.destination_psb)?;
                continue;
            }
            if !item.destination_psb.is_file() {
                copy_verified(
                    &item.source_psb,
                    &item.destination_psb,
                    &item.psb_sha256,
                    item.psb_byte_size,
                    verification.batch_id,
                )?;
            }
            register_promotion(database, project, &manifest, item, verification.batch_id).await?;
            remove_redundant_source(&item.source_psb, &item.destination_psb)?;
            item.action = "promoted".into();
        }
    }

    Ok(MasterPromotion {
        schema_version: 1,
        batch_id: verification.batch_id,
        project: project.slug.clone(),
        confirmed,
        total_byte_size,
        items,
    })
}

pub async fn checkpoint(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    ready: bool,
    confirmed: bool,
) -> Result<MasterCheckpoint> {
    if ready && !confirmed {
        return checkpoint_plan(database, config, project, ready, false).await;
    }
    checkpoint_plan(database, config, project, ready, true).await
}

pub async fn prepare_flattening(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
) -> Result<FlatteningManifest> {
    validate_project_slug(&project.slug)?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let workspace = project_root.join(".photara");
    let output = project_root.join("masters").join("flattened");
    create_directory(&project_root)?;
    create_directory(&workspace)?;
    create_directory(&output)?;

    let rows = sqlx::query(
        "SELECT file.id, file.asset_id, file.location, file.sha256, file.byte_size, \
                document.bits_per_channel, document.color_profile \
         FROM layered_master_documents AS document \
         JOIN asset_files AS file ON file.id = document.asset_file_id \
         WHERE document.project_id = $1 AND document.workflow_state = 'ready-for-flattening' \
           AND file.representation = 'layered-psb' AND file.authoritative \
           AND file.state = 'current' ORDER BY file.location",
    )
    .bind(project.id)
    .fetch_all(database.pool())
    .await?;
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project {:?} has no 32-bit layered masters ready for flattening",
            project.slug
        )));
    }

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let layered_file_id: Uuid = row.try_get("id")?;
        let asset_id: Uuid = row.try_get("asset_id")?;
        let location: String = row.try_get("location")?;
        let recorded_hash: Option<String> = row.try_get("sha256")?;
        let recorded_size: Option<i64> = row.try_get("byte_size")?;
        let recorded_bits: i16 = row.try_get("bits_per_channel")?;
        let profile: String = row.try_get("color_profile")?;
        let psb_path = resolve_source_key(&config.settings.images_root, &location)?;
        let (psb_byte_size, bits_per_channel) = inspect_psb(&psb_path)?;
        let psb_sha256 = sha256(&psb_path)?;
        if bits_per_channel != 32 || recorded_bits != 32 {
            return Err(PhotaraError::Configuration(format!(
                "{} is not recorded and verified as a 32-bit layered master; run `photara masters mark-ready {} --confirm` after saving it",
                psb_path.display(),
                project.slug
            )));
        }
        if !is_p3_pq(&profile) {
            return Err(PhotaraError::Configuration(format!(
                "{} has unexpected recorded profile {profile:?}; expected P3 PQ",
                psb_path.display()
            )));
        }
        if recorded_hash.as_deref() != Some(&psb_sha256)
            || recorded_size != Some(to_i64(psb_byte_size, "PSB")?)
        {
            return Err(PhotaraError::Configuration(format!(
                "{} changed after its last readiness checkpoint; rerun `photara masters mark-ready {} --confirm`",
                psb_path.display(),
                project.slug
            )));
        }
        let psb_filename = psb_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| PhotaraError::Configuration("layered PSB has no filename".into()))?
            .to_owned();
        let stem = psb_path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| PhotaraError::Configuration("layered PSB has no file stem".into()))?;
        let relative = location.strip_prefix("images:").ok_or_else(|| {
            PhotaraError::Configuration(format!("unsupported layered PSB location {location:?}"))
        })?;
        let tiff_filename = format!("{stem}.TIF");
        items.push(FlatteningManifestItem {
            asset_id,
            layered_file_id,
            psb_filename,
            psb_relative_path: PathBuf::from(relative),
            psb_sha256,
            psb_byte_size,
            tiff_relative_path: PathBuf::from("masters")
                .join("flattened")
                .join(&tiff_filename),
            tiff_filename,
        });
    }

    let manifest_path = workspace.join(FLATTENING_MANIFEST_NAME);
    let batch_id = match read_json::<FlatteningManifest>(&manifest_path) {
        Ok(existing)
            if existing.project == project.slug
                && existing.items.len() == items.len()
                && existing.items.iter().zip(&items).all(|(left, right)| {
                    left.asset_id == right.asset_id && left.psb_sha256 == right.psb_sha256
                }) =>
        {
            existing.batch_id
        }
        Ok(_) => Uuid::new_v4(),
        Err(PhotaraError::Filesystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Uuid::new_v4()
        }
        Err(error) => return Err(error),
    };
    let scripts_root = config
        .settings
        .lightroom_inbox
        .parent()
        .ok_or_else(|| {
            PhotaraError::Configuration(
                "lightroom_inbox must have a parent directory for Photoshop scripts".into(),
            )
        })?
        .join("Scripts");
    create_directory(&scripts_root)?;
    let photoshop_script = scripts_root.join(FLATTENING_SCRIPT_NAME);
    let manifest = FlatteningManifest {
        schema_version: 1,
        batch_id,
        project: project.slug.clone(),
        project_root: project_root.clone(),
        images_root: config.settings.images_root.clone(),
        output_directory: output,
        photoshop_script: photoshop_script.clone(),
        items,
    };
    write_json_atomic(manifest_path, &manifest)?;
    write_json_atomic(
        project_root.join(FLATTENING_HANDOFF_MANIFEST_NAME),
        &manifest,
    )?;
    write_atomic(photoshop_script, FLATTENING_SCRIPT.as_bytes())?;
    Ok(manifest)
}

pub fn verify_flattening(config: &PhotaraConfig, project: &str) -> Result<FlatteningVerification> {
    let manifest = load_flattening_manifest(config, project)?;
    let handoff_report = manifest.project_root.join(FLATTENING_HANDOFF_REPORT_NAME);
    let internal_report = manifest
        .project_root
        .join(".photara")
        .join(FLATTENING_REPORT_NAME);
    let report: FlatteningReport = match read_json(&handoff_report) {
        Ok(report) => report,
        Err(PhotaraError::Filesystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            read_json(&internal_report)?
        }
        Err(error) => return Err(error),
    };
    if report.batch_id != manifest.batch_id || report.project != manifest.project {
        return Err(PhotaraError::Configuration(
            "Photoshop flattening report does not belong to the current manifest".into(),
        ));
    }
    let mut verified = Vec::with_capacity(manifest.items.len());
    for item in &manifest.items {
        let evidence = report
            .items
            .iter()
            .find(|entry| entry.asset_id == item.asset_id)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "Photoshop flattening report has no entry for {}",
                    item.psb_filename
                ))
            })?;
        let profile = evidence.color_profile.as_deref().unwrap_or_default();
        if evidence.state != "verified"
            || evidence.bits_per_channel != Some(32)
            || evidence.layer_count != Some(1)
            || !is_display_p3_linear(profile)
        {
            return Err(PhotaraError::Configuration(format!(
                "Photoshop did not verify the flattened contract for {}: {:?}",
                item.tiff_filename, evidence.error
            )));
        }
        let psb_path = manifest.images_root.join(&item.psb_relative_path);
        let (psb_size, psb_bits) = inspect_psb(&psb_path)?;
        if psb_bits != 32 || psb_size != item.psb_byte_size || sha256(&psb_path)? != item.psb_sha256
        {
            return Err(PhotaraError::Configuration(format!(
                "layered source {} changed during flattening",
                psb_path.display()
            )));
        }
        let tiff_path = manifest.project_root.join(&item.tiff_relative_path);
        let (tiff_byte_size, tiff_bits) = inspect_tiff(&tiff_path)?;
        if tiff_bits != 32 {
            return Err(PhotaraError::Configuration(format!(
                "{} is {tiff_bits} bits per channel; expected a flattened 32-bit TIFF",
                tiff_path.display()
            )));
        }
        verified.push(VerifiedFlattenedMaster {
            asset_id: item.asset_id,
            layered_file_id: item.layered_file_id,
            psb_path,
            tiff_sha256: sha256(&tiff_path)?,
            tiff_byte_size,
            tiff_path,
            bits_per_channel: 32,
            color_profile: profile.into(),
            layer_count: 1,
        });
    }
    Ok(FlatteningVerification {
        schema_version: 1,
        batch_id: manifest.batch_id,
        project: manifest.project,
        verified_count: verified.len(),
        items: verified,
    })
}

pub async fn register_flattening(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    confirmed: bool,
) -> Result<FlatteningRegistration> {
    let verification = verify_flattening(config, &project.slug)?;
    let mut items = Vec::with_capacity(verification.items.len());
    for verified in verification.items {
        let logical_location = format!(
            "projects:{}/masters/flattened/{}",
            project.slug,
            verified
                .tiff_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| PhotaraError::Configuration("TIFF has no filename".into()))?
        );
        let existing = sqlx::query(
            "SELECT id, location, sha256, byte_size FROM asset_files \
             WHERE asset_id = $1 AND representation = 'flattened-tiff' \
               AND authoritative AND state = 'current'",
        )
        .bind(verified.asset_id)
        .fetch_optional(database.pool())
        .await?;
        let already_registered = if let Some(row) = existing {
            let location: String = row.try_get("location")?;
            let hash: Option<String> = row.try_get("sha256")?;
            let bytes: Option<i64> = row.try_get("byte_size")?;
            if location != logical_location
                || hash.as_deref() != Some(&verified.tiff_sha256)
                || bytes != Some(to_i64(verified.tiff_byte_size, "TIFF")?)
            {
                return Err(PhotaraError::Configuration(format!(
                    "asset {} already has a different authoritative flattened TIFF",
                    verified.asset_id
                )));
            }
            true
        } else {
            false
        };
        let mut action = if already_registered {
            "already-registered".to_owned()
        } else {
            "register".to_owned()
        };
        if confirmed && !already_registered {
            register_flattened_master(
                database,
                project,
                &verified,
                &logical_location,
                verification.batch_id,
            )
            .await?;
            action = "registered".into();
        }
        items.push(RegisteredFlattenedMaster {
            asset_id: verified.asset_id,
            tiff_path: verified.tiff_path,
            logical_location,
            tiff_sha256: verified.tiff_sha256,
            tiff_byte_size: verified.tiff_byte_size,
            workflow_state: "flattened".into(),
            action,
        });
    }
    Ok(FlatteningRegistration {
        schema_version: 1,
        batch_id: verification.batch_id,
        project: project.slug.clone(),
        confirmed,
        items,
    })
}

async fn checkpoint_plan(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    ready: bool,
    record: bool,
) -> Result<MasterCheckpoint> {
    let rows = sqlx::query(
        "SELECT file.id, file.asset_id, file.location, file.sha256, file.byte_size, \
                document.workflow_state \
         FROM layered_master_documents AS document \
         JOIN asset_files AS file ON file.id = document.asset_file_id \
         WHERE document.project_id = $1 AND file.representation = 'layered-psb' \
           AND file.authoritative AND file.state = 'current' \
         ORDER BY file.location",
    )
    .bind(project.id)
    .fetch_all(database.pool())
    .await?;
    if rows.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project {:?} has no promoted layered masters",
            project.slug
        )));
    }
    let target_state = if ready {
        "ready-for-flattening"
    } else {
        "editing"
    };
    let event_type = if ready {
        "marked-ready"
    } else {
        "checkpointed"
    };
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let file_id: Uuid = row.try_get("id")?;
        let asset_id: Uuid = row.try_get("asset_id")?;
        let location: String = row.try_get("location")?;
        let previous_hash: Option<String> = row.try_get("sha256")?;
        let previous_size: Option<i64> = row.try_get("byte_size")?;
        let previous_state: String = row.try_get("workflow_state")?;
        if previous_state == "flattened" {
            return Err(PhotaraError::Configuration(format!(
                "layered master {location:?} is already flattened"
            )));
        }
        let psb_path = resolve_source_key(&config.settings.images_root, &location)?;
        let (psb_byte_size, bits_per_channel) = inspect_psb(&psb_path)?;
        if ready && bits_per_channel != 32 {
            return Err(PhotaraError::Configuration(format!(
                "{} is {bits_per_channel} bits per channel; a master must be 32-bit before flattening",
                psb_path.display()
            )));
        }
        let psb_sha256 = sha256(&psb_path)?;
        let changed = previous_hash.as_deref() != Some(&psb_sha256)
            || previous_size != Some(to_i64(psb_byte_size, "PSB")?)
            || previous_state != target_state;
        if record {
            let mut transaction = database.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(asset_id.to_string())
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "UPDATE asset_files SET sha256 = $2, byte_size = $3 \
                 WHERE id = $1 AND representation = 'layered-psb' \
                   AND authoritative AND state = 'current'",
            )
            .bind(file_id)
            .bind(&psb_sha256)
            .bind(to_i64(psb_byte_size, "PSB")?)
            .execute(&mut *transaction)
            .await?;
            sqlx::query(
                "UPDATE layered_master_documents \
                 SET workflow_state = $2, bits_per_channel = $3, updated_at = now() \
                 WHERE asset_file_id = $1",
            )
            .bind(file_id)
            .bind(target_state)
            .bind(i16::from(bits_per_channel))
            .execute(&mut *transaction)
            .await?;
            sqlx::query(
                "INSERT INTO layered_master_events \
                 (id, asset_file_id, event_type, sha256, byte_size, note) \
                 SELECT $1, $2, $3, $4, $5, $6 \
                 WHERE NOT EXISTS ( \
                   SELECT 1 FROM layered_master_events \
                   WHERE asset_file_id = $2 AND event_type = $3 \
                     AND sha256 = $4 AND byte_size = $5 \
                 )",
            )
            .bind(Uuid::new_v4())
            .bind(file_id)
            .bind(event_type)
            .bind(&psb_sha256)
            .bind(to_i64(psb_byte_size, "PSB")?)
            .bind(if ready {
                "Photographer confirmed raster master is ready for flattening"
            } else {
                "Layered raster-edit checkpoint"
            })
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
        }
        items.push(CheckpointedMaster {
            asset_id,
            psb_path,
            psb_sha256,
            psb_byte_size,
            bits_per_channel,
            previous_state,
            target_state: target_state.into(),
            changed,
        });
    }
    Ok(MasterCheckpoint {
        schema_version: 1,
        project: project.slug.clone(),
        target_state: target_state.into(),
        recorded: record,
        items,
    })
}

async fn register_promotion(
    database: &Database,
    project: &ProjectRecord,
    manifest: &MasterManifest,
    promoted: &PromotedMaster,
    batch_id: Uuid,
) -> Result<()> {
    let manifest_item = manifest
        .items
        .iter()
        .find(|item| item.asset_id == promoted.asset_id)
        .ok_or_else(|| PhotaraError::Configuration("promotion manifest item disappeared".into()))?;
    let dng_path = manifest.staging_root.join(&manifest_item.dng_relative_path);
    let dng_size = inspect_regular_file(&dng_path, "DNG")?;
    let dng_hash = sha256(&dng_path)?;
    let dng_location = format!("master-inbox:{batch_id}/{}", manifest_item.dng_filename);
    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(promoted.asset_id.to_string())
        .execute(&mut *transaction)
        .await?;

    let raw_file_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM asset_files WHERE asset_id = $1 \
         AND representation = 'camera-raw' AND state = 'current'",
    )
    .bind(promoted.asset_id)
    .fetch_one(&mut *transaction)
    .await?;
    let source_file_id = if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM asset_files WHERE location = $1 AND state = 'current'",
    )
    .bind(&dng_location)
    .fetch_optional(&mut *transaction)
    .await?
    {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO asset_files \
             (id, asset_id, representation, location, sha256, byte_size, authoritative) \
             VALUES ($1, $2, 'working-dng', $3, $4, $5, false)",
        )
        .bind(id)
        .bind(promoted.asset_id)
        .bind(&dng_location)
        .bind(&dng_hash)
        .bind(to_i64(dng_size, "DNG")?)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO asset_file_origins (source_file_id, derived_file_id, operation) \
             VALUES ($1, $2, 'lightroom-cloud-original-plus-settings-export') \
             ON CONFLICT DO NOTHING",
        )
        .bind(raw_file_id)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        id
    };

    let psb_file_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO asset_files \
         (id, asset_id, representation, location, sha256, byte_size, authoritative) \
         VALUES ($1, $2, 'layered-psb', $3, $4, $5, true)",
    )
    .bind(psb_file_id)
    .bind(promoted.asset_id)
    .bind(&promoted.logical_location)
    .bind(&promoted.psb_sha256)
    .bind(to_i64(promoted.psb_byte_size, "PSB")?)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO asset_file_origins (source_file_id, derived_file_id, operation) \
         VALUES ($1, $2, 'photoshop-embedded-camera-raw-smart-object')",
    )
    .bind(source_file_id)
    .bind(psb_file_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO layered_master_documents \
         (asset_file_id, project_id, source_file_id, build_batch_id, bits_per_channel, \
          color_profile, smart_object_source, smart_object_embedded, workflow_state, verified_at) \
         VALUES ($1, $2, $3, $4, 16, $5, $6, true, 'editing', now())",
    )
    .bind(psb_file_id)
    .bind(project.id)
    .bind(source_file_id)
    .bind(batch_id)
    .bind(&promoted.color_profile)
    .bind(&manifest_item.dng_filename)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO layered_master_events \
         (id, asset_file_id, event_type, sha256, byte_size, note) \
         VALUES ($1, $2, 'promoted', $3, $4, 'UXP-verified embedded DNG master')",
    )
    .bind(Uuid::new_v4())
    .bind(psb_file_id)
    .bind(&promoted.psb_sha256)
    .bind(to_i64(promoted.psb_byte_size, "PSB")?)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn register_flattened_master(
    database: &Database,
    project: &ProjectRecord,
    verified: &VerifiedFlattenedMaster,
    logical_location: &str,
    batch_id: Uuid,
) -> Result<()> {
    let mut transaction = database.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(verified.asset_id.to_string())
        .execute(&mut *transaction)
        .await?;
    let state: String = sqlx::query_scalar(
        "SELECT workflow_state FROM layered_master_documents WHERE asset_file_id = $1",
    )
    .bind(verified.layered_file_id)
    .fetch_one(&mut *transaction)
    .await?;
    if state != "ready-for-flattening" {
        return Err(PhotaraError::Configuration(format!(
            "layered master {} is in state {state:?}, not ready-for-flattening",
            verified.psb_path.display()
        )));
    }
    let tiff_file_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO asset_files \
         (id, asset_id, representation, location, sha256, byte_size, authoritative) \
         VALUES ($1, $2, 'flattened-tiff', $3, $4, $5, true)",
    )
    .bind(tiff_file_id)
    .bind(verified.asset_id)
    .bind(logical_location)
    .bind(&verified.tiff_sha256)
    .bind(to_i64(verified.tiff_byte_size, "TIFF")?)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO asset_file_origins (source_file_id, derived_file_id, operation) \
         VALUES ($1, $2, 'photoshop-flatten-32-bit-tiff')",
    )
    .bind(verified.layered_file_id)
    .bind(tiff_file_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO flattened_master_documents \
         (asset_file_id, project_id, source_file_id, build_batch_id, bits_per_channel, \
          color_profile, layer_count, verified_at) \
         VALUES ($1, $2, $3, $4, 32, $5, 1, now())",
    )
    .bind(tiff_file_id)
    .bind(project.id)
    .bind(verified.layered_file_id)
    .bind(batch_id)
    .bind(&verified.color_profile)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE layered_master_documents \
         SET workflow_state = 'flattened', color_profile = $2, updated_at = now() \
         WHERE asset_file_id = $1",
    )
    .bind(verified.layered_file_id)
    .bind(&verified.color_profile)
    .execute(&mut *transaction)
    .await?;
    let (psb_byte_size, _) = inspect_psb(&verified.psb_path)?;
    let psb_sha256 = sha256(&verified.psb_path)?;
    sqlx::query(
        "INSERT INTO layered_master_events \
         (id, asset_file_id, event_type, sha256, byte_size, note) \
         VALUES ($1, $2, 'flattened', $3, $4, 'Verified 32-bit flattened TIFF registered')",
    )
    .bind(Uuid::new_v4())
    .bind(verified.layered_file_id)
    .bind(psb_sha256)
    .bind(to_i64(psb_byte_size, "PSB")?)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

fn layered_psb_location(source_key: &str, psb_filename: &str) -> Result<String> {
    let (directory, _) = source_key.rsplit_once('/').ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "camera RAW source key {source_key:?} has no directory"
        ))
    })?;
    Ok(format!("{directory}/{psb_filename}"))
}

fn copy_verified(
    source: &Path,
    destination: &Path,
    expected_hash: &str,
    expected_size: u64,
    batch_id: Uuid,
) -> Result<()> {
    inspect_regular_file(source, "staged PSB")?;
    let parent = destination.parent().ok_or_else(|| {
        PhotaraError::Configuration(format!("{} has no parent directory", destination.display()))
    })?;
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            PhotaraError::Configuration("layered PSB destination has no filename".into())
        })?;
    let temporary = parent.join(format!(".{filename}.{batch_id}.photara-tmp"));
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|source| {
            PhotaraError::filesystem("remove stale PSB temporary file", &temporary, source)
        })?;
    }
    fs::copy(source, &temporary)
        .map_err(|source| PhotaraError::filesystem("copy verified PSB", &temporary, source))?;
    sync_if_supported(&temporary)?;
    if inspect_psb(&temporary)?.0 != expected_size || sha256(&temporary)? != expected_hash {
        return Err(PhotaraError::Configuration(format!(
            "copied PSB {} failed size or SHA-256 verification",
            temporary.display()
        )));
    }
    fs::rename(&temporary, destination)
        .map_err(|source| PhotaraError::filesystem("promote verified PSB", destination, source))
}

fn sync_if_supported(path: &Path) -> Result<()> {
    let file = fs::File::open(path)
        .map_err(|source| PhotaraError::filesystem("open copied PSB", path, source))?;
    match file.sync_all() {
        Ok(()) => Ok(()),
        Err(error)
            if error.kind() == std::io::ErrorKind::Unsupported
                || matches!(error.raw_os_error(), Some(45 | 95)) =>
        {
            // SMB and some other network filesystems do not expose fsync. The
            // mandatory size and SHA-256 readback below remains the promotion
            // integrity gate before the same-filesystem atomic rename.
            Ok(())
        }
        Err(source) => Err(PhotaraError::filesystem("sync verified PSB", path, source)),
    }
}

fn remove_redundant_source(source: &Path, destination: &Path) -> Result<()> {
    if source != destination && source.exists() {
        fs::remove_file(source).map_err(|error| {
            PhotaraError::filesystem("remove promoted staging PSB", source, error)
        })?;
    }
    Ok(())
}

fn to_i64(value: u64, kind: &str) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        PhotaraError::Configuration(format!("{kind} byte size exceeds PostgreSQL bigint range"))
    })
}

pub fn load_manifest(config: &PhotaraConfig, project: &str) -> Result<MasterManifest> {
    validate_project_slug(project)?;
    read_json(
        &config
            .settings
            .lightroom_inbox
            .join(".photara")
            .join(MANIFEST_NAME),
    )
}

fn load_flattening_manifest(config: &PhotaraConfig, project: &str) -> Result<FlatteningManifest> {
    validate_project_slug(project)?;
    read_json(
        &config
            .settings
            .projects_root
            .join(project)
            .join(".photara")
            .join(FLATTENING_MANIFEST_NAME),
    )
}

fn validate_project_slug(project: &str) -> Result<()> {
    if project.is_empty()
        || !project
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(PhotaraError::Configuration(
            "master project slug must use lowercase ASCII letters, digits, and hyphens".into(),
        ));
    }
    Ok(())
}

fn resolve_source_key(images_root: &Path, source_key: &str) -> Result<PathBuf> {
    let relative = source_key.strip_prefix("images:").ok_or_else(|| {
        PhotaraError::Configuration(format!("unsupported camera RAW source key {source_key:?}"))
    })?;
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(PhotaraError::Configuration(format!(
            "unsafe camera RAW source key {source_key:?}"
        )));
    }
    Ok(images_root.join(relative))
}

fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|source| PhotaraError::filesystem("create master staging directory", path, source))
}

fn write_json_atomic(path: PathBuf, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

fn write_atomic(path: PathBuf, contents: &[u8]) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary)
        .map_err(|source| PhotaraError::filesystem("create temporary file", &temporary, source))?;
    file.write_all(contents)
        .map_err(|source| PhotaraError::filesystem("write temporary file", &temporary, source))?;
    if let Err(source) = file.sync_all()
        && source.raw_os_error() != Some(45)
    {
        return Err(PhotaraError::filesystem(
            "sync temporary file",
            &temporary,
            source,
        ));
    }
    drop(file);
    fs::rename(&temporary, &path)
        .map_err(|source| PhotaraError::filesystem("replace file", path, source))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .map_err(|source| PhotaraError::filesystem("read JSON file", path, source))?;
    serde_json::from_str(&text).map_err(Into::into)
}

fn inspect_regular_file(path: &Path, kind: &str) -> Result<u64> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| PhotaraError::filesystem("inspect master file", path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(PhotaraError::Configuration(format!(
            "{kind} {} must be a non-empty regular file, not a symlink",
            path.display()
        )));
    }
    Ok(metadata.len())
}

fn inspect_psb(path: &Path) -> Result<(u64, u8)> {
    let size = inspect_regular_file(path, "PSB")?;
    let mut header = [0_u8; 26];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|source| PhotaraError::filesystem("read PSB header", path, source))?;
    if &header[..4] != b"8BPS" || u16::from_be_bytes([header[4], header[5]]) != 2 {
        return Err(PhotaraError::Configuration(format!(
            "{} is not a Photoshop large document (PSB)",
            path.display()
        )));
    }
    let bits = u16::from_be_bytes([header[22], header[23]]);
    if !matches!(bits, 16 | 32) {
        return Err(PhotaraError::Configuration(format!(
            "{} has unsupported {bits}-bit channels; expected 16 or 32",
            path.display(),
        )));
    }
    Ok((size, bits as u8))
}

fn inspect_tiff(path: &Path) -> Result<(u64, u8)> {
    let size = inspect_regular_file(path, "TIFF")?;
    let mut file = fs::File::open(path)
        .map_err(|source| PhotaraError::filesystem("open TIFF", path, source))?;
    let mut header = [0_u8; 8];
    file.read_exact(&mut header)
        .map_err(|source| PhotaraError::filesystem("read TIFF header", path, source))?;
    let little = match &header[..2] {
        b"II" => true,
        b"MM" => false,
        _ => {
            return Err(PhotaraError::Configuration(format!(
                "{} has no TIFF byte-order marker",
                path.display()
            )));
        }
    };
    let decode_u16 = |value: [u8; 2]| {
        if little {
            u16::from_le_bytes(value)
        } else {
            u16::from_be_bytes(value)
        }
    };
    let decode_u32 = |value: [u8; 4]| {
        if little {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        }
    };
    if decode_u16([header[2], header[3]]) != 42 {
        return Err(PhotaraError::Configuration(format!(
            "{} is not a classic TIFF file",
            path.display()
        )));
    }
    let ifd = u64::from(decode_u32([header[4], header[5], header[6], header[7]]));
    file.seek(SeekFrom::Start(ifd))
        .map_err(|source| PhotaraError::filesystem("seek TIFF IFD", path, source))?;
    let mut count_bytes = [0_u8; 2];
    file.read_exact(&mut count_bytes)
        .map_err(|source| PhotaraError::filesystem("read TIFF IFD count", path, source))?;
    let count = usize::from(decode_u16(count_bytes));
    for index in 0..count {
        file.seek(SeekFrom::Start(ifd + 2 + (index as u64) * 12))
            .map_err(|source| PhotaraError::filesystem("seek TIFF IFD entry", path, source))?;
        let mut entry = [0_u8; 12];
        file.read_exact(&mut entry)
            .map_err(|source| PhotaraError::filesystem("read TIFF IFD entry", path, source))?;
        if decode_u16([entry[0], entry[1]]) != 258 {
            continue;
        }
        let field_type = decode_u16([entry[2], entry[3]]);
        let values = usize::try_from(decode_u32([entry[4], entry[5], entry[6], entry[7]]))
            .map_err(|_| PhotaraError::Configuration("TIFF value count overflowed".into()))?;
        if field_type != 3 || values == 0 {
            break;
        }
        let inline = [entry[8], entry[9], entry[10], entry[11]];
        let data_offset = if values <= 2 {
            None
        } else {
            Some(u64::from(decode_u32(inline)))
        };
        if let Some(offset) = data_offset {
            file.seek(SeekFrom::Start(offset)).map_err(|source| {
                PhotaraError::filesystem("seek TIFF BitsPerSample", path, source)
            })?;
        }
        let mut bits = None;
        for value_index in 0..values {
            let mut value_bytes = [0_u8; 2];
            if data_offset.is_none() {
                value_bytes.copy_from_slice(&inline[value_index * 2..value_index * 2 + 2]);
            } else {
                file.read_exact(&mut value_bytes).map_err(|source| {
                    PhotaraError::filesystem("read TIFF BitsPerSample", path, source)
                })?;
            }
            let value = decode_u16(value_bytes);
            if let Some(expected) = bits
                && expected != value
            {
                return Err(PhotaraError::Configuration(format!(
                    "{} has mixed TIFF channel depths",
                    path.display()
                )));
            }
            bits = Some(value);
        }
        let bits = bits.unwrap_or_default();
        if bits > u16::from(u8::MAX) {
            break;
        }
        return Ok((size, bits as u8));
    }
    Err(PhotaraError::Configuration(format!(
        "{} has no readable TIFF BitsPerSample tag",
        path.display()
    )))
}

fn sha256(path: &Path) -> Result<String> {
    let file = fs::File::open(path)
        .map_err(|source| PhotaraError::filesystem("open master for hashing", path, source))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| PhotaraError::filesystem("hash master", path, source))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn is_p3_pq(profile: &str) -> bool {
    let normalized = profile.to_ascii_lowercase();
    normalized.contains("p3") && normalized.contains("pq")
}

fn is_display_p3_linear(profile: &str) -> bool {
    let normalized = profile.to_ascii_lowercase();
    normalized.contains("p3") && normalized.contains("linear")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn accepts_adobe_p3_pq_profile_names() {
        assert!(is_p3_pq("P3 PQ"));
        assert!(is_p3_pq("P3D65 PQ Display Full 12-16-0-1"));
        assert!(!is_p3_pq("Display P3"));
        assert!(!is_p3_pq("Rec.2100 PQ"));
    }

    #[test]
    fn accepts_photoshop_32_bit_display_p3_linear_profile() {
        assert!(is_display_p3_linear("Display P3 (Linear RGB Profile)"));
        assert!(is_display_p3_linear("Display P3 Linear"));
        assert!(!is_display_p3_linear("P3 PQ"));
        assert!(!is_display_p3_linear("Linear ProPhoto RGB"));
    }

    #[test]
    fn source_keys_resolve_below_images_root() {
        assert_eq!(
            resolve_source_key(Path::new("/Pictures/Images"), "images:2021/06/a.ARW").unwrap(),
            PathBuf::from("/Pictures/Images/2021/06/a.ARW")
        );
        assert!(resolve_source_key(Path::new("/Pictures/Images"), "images:../a.ARW").is_err());
    }

    #[test]
    fn layered_psb_location_stays_beside_the_camera_raw() {
        assert_eq!(
            layered_psb_location(
                "images:2021/2021-06/2021-06-11/DSC05217.ARW",
                "DSC05217_2021_06_11_SUHAIL.PSB",
            )
            .unwrap(),
            "images:2021/2021-06/2021-06-11/DSC05217_2021_06_11_SUHAIL.PSB"
        );
    }

    #[test]
    fn accepts_32_bit_psb_header() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("master.PSB");
        let mut header = [0_u8; 26];
        header[..4].copy_from_slice(b"8BPS");
        header[4..6].copy_from_slice(&2_u16.to_be_bytes());
        header[22..24].copy_from_slice(&32_u16.to_be_bytes());
        fs::write(&path, header).unwrap();
        assert_eq!(inspect_psb(&path).unwrap(), (26, 32));
    }

    #[test]
    fn reads_32_bit_tiff_samples_without_loading_pixels() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("master.TIFF");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"II");
        bytes.extend_from_slice(&42_u16.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&258_u16.to_le_bytes());
        bytes.extend_from_slice(&3_u16.to_le_bytes());
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&26_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        for _ in 0..3 {
            bytes.extend_from_slice(&32_u16.to_le_bytes());
        }
        fs::write(&path, &bytes).unwrap();
        assert_eq!(inspect_tiff(&path).unwrap(), (bytes.len() as u64, 32));
    }
}
