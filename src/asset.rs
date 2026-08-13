use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{PhotaraError, Result};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Representation {
    CameraRaw,
    XmpSidecar,
    WorkingDng,
    LayeredPsb,
    FlattenedHdrTiff,
    FlattenedSdrTiff,
    DeliveryRendition,
    PixiesetProof,
}

impl Representation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CameraRaw => "camera-raw",
            Self::XmpSidecar => "xmp-sidecar",
            Self::WorkingDng => "working-dng",
            Self::LayeredPsb => "layered-psb",
            Self::FlattenedHdrTiff => "flattened-hdr-tiff",
            Self::FlattenedSdrTiff => "flattened-sdr-tiff",
            Self::DeliveryRendition => "delivery-rendition",
            Self::PixiesetProof => "pixieset-proof",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegisterOriginal {
    pub project_id: Uuid,
    pub original_path: PathBuf,
    pub capture_date: NaiveDate,
    pub author_code: String,
    pub sha256: String,
    pub byte_size: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRecord {
    pub id: Uuid,
    pub original_filename: String,
    pub original_stem: String,
    pub capture_date: NaiveDate,
    pub author_code: String,
    pub original_sha256: String,
}

pub async fn register_original(
    database: &Database,
    input: RegisterOriginal,
) -> Result<AssetRecord> {
    let original_filename = filename(&input.original_path)?;
    let original_stem = stem(&input.original_path)?;
    validate_author_code(&input.author_code)?;
    validate_sha256(&input.sha256)?;
    if !input.original_path.is_absolute() {
        return Err(PhotaraError::Configuration(
            "original RAW location must be an absolute path".into(),
        ));
    }
    let location = camera_raw_key(&input.original_path)?;

    let mut transaction = database.begin().await?;
    let asset_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO assets \
         (id, original_filename, original_stem, capture_date, author_code, original_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (original_sha256) DO NOTHING",
    )
    .bind(asset_id)
    .bind(&original_filename)
    .bind(&original_stem)
    .bind(input.capture_date)
    .bind(&input.author_code)
    .bind(&input.sha256)
    .execute(&mut *transaction)
    .await?;

    let record = find_by_sha256_on(&mut transaction, &input.sha256)
        .await?
        .ok_or_else(|| PhotaraError::Configuration("asset insert did not persist".into()))?;
    if record.original_filename != original_filename
        || record.capture_date != input.capture_date
        || record.author_code != input.author_code
    {
        return Err(PhotaraError::Configuration(format!(
            "fingerprint {} is already registered with different source metadata",
            input.sha256
        )));
    }

    sqlx::query(
        "INSERT INTO project_assets (project_id, asset_id) VALUES ($1, $2) \
         ON CONFLICT (project_id, asset_id) DO NOTHING",
    )
    .bind(input.project_id)
    .bind(record.id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO asset_files \
         (id, asset_id, representation, location, sha256, byte_size, authoritative) \
         VALUES ($1, $2, $3, $4, $5, $6, true) \
         ON CONFLICT (location) WHERE state = 'current' DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(record.id)
    .bind(Representation::CameraRaw.as_str())
    .bind(&location)
    .bind(&input.sha256)
    .bind(input.byte_size)
    .execute(&mut *transaction)
    .await?;

    let raw_asset_id: Uuid = sqlx::query_scalar(
        "SELECT asset_id FROM asset_files \
         WHERE location = $1 AND representation = 'camera-raw' AND state = 'current'",
    )
    .bind(&location)
    .fetch_one(&mut *transaction)
    .await?;
    if raw_asset_id != record.id {
        return Err(PhotaraError::Configuration(format!(
            "RAW location {} is already owned by a different asset",
            input.original_path.display()
        )));
    }

    transaction.commit().await?;
    Ok(record)
}

pub fn camera_raw_key(path: &Path) -> Result<String> {
    let components = path.components().collect::<Vec<_>>();
    let images = components
        .iter()
        .position(|component| {
            matches!(component, std::path::Component::Normal(value) if value.eq_ignore_ascii_case("images"))
        })
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "camera RAW path {} has no Images component",
                path.display()
            ))
        })?;
    let relative = components[images + 1..]
        .iter()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    if relative.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "camera RAW path {} has nothing below Images",
            path.display()
        )));
    }
    Ok(format!("images:{relative}"))
}

pub async fn find_by_sha256(database: &Database, sha256: &str) -> Result<Option<AssetRecord>> {
    validate_sha256(sha256)?;
    let mut connection = database.acquire().await?;
    find_by_sha256_on(&mut connection, sha256).await
}

async fn find_by_sha256_on(
    connection: &mut sqlx::PgConnection,
    sha256: &str,
) -> Result<Option<AssetRecord>> {
    let row = sqlx::query(
        "SELECT id, original_filename, original_stem, capture_date, author_code, original_sha256 \
         FROM assets WHERE original_sha256 = $1",
    )
    .bind(sha256)
    .fetch_optional(connection)
    .await?;
    row.map(|row| {
        Ok(AssetRecord {
            id: row.try_get("id")?,
            original_filename: row.try_get("original_filename")?,
            original_stem: row.try_get("original_stem")?,
            capture_date: row.try_get("capture_date")?,
            author_code: row.try_get("author_code")?,
            original_sha256: row.try_get("original_sha256")?,
        })
    })
    .transpose()
}

pub fn downstream_basename(
    original_stem: &str,
    capture_date: NaiveDate,
    author_code: &str,
    sha256: &str,
    collision: bool,
) -> Result<String> {
    if original_stem.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "original stem must not be empty".into(),
        ));
    }
    validate_author_code(author_code)?;
    validate_sha256(sha256)?;
    let base = format!(
        "{original_stem}_{}_{author_code}",
        capture_date.format("%Y_%m_%d")
    );
    if collision {
        Ok(format!("{base}_{}", &sha256[..12]))
    } else {
        Ok(base)
    }
}

fn filename(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| PhotaraError::Configuration("RAW path has no UTF-8 filename".into()))
}

fn stem(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| PhotaraError::Configuration("RAW path has no UTF-8 file stem".into()))
}

fn validate_author_code(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(PhotaraError::Configuration(
            "author code must contain only uppercase ASCII letters, digits, and hyphens".into(),
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PhotaraError::Configuration(
            "SHA-256 must be 64 lowercase hexadecimal characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn downstream_name_preserves_camera_stem_and_expands_only_on_request() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        assert_eq!(
            downstream_basename("_SUH5082", date, "SUHAIL", HASH, false).unwrap(),
            "_SUH5082_2026_08_06_SUHAIL"
        );
        assert_eq!(
            downstream_basename("_SUH5082", date, "SUHAIL", HASH, true).unwrap(),
            "_SUH5082_2026_08_06_SUHAIL_0123456789ab"
        );
    }

    #[test]
    fn rejects_noncanonical_fingerprints() {
        assert!(validate_sha256("ABC").is_err());
        assert!(validate_sha256(HASH).is_ok());
    }

    #[test]
    fn camera_raw_identity_is_independent_of_volume_root() {
        assert_eq!(
            camera_raw_key(Path::new(
                "/Volumes/whisk/work/ml/datasets/proetus/images/2021/2021-06/2021-06-11/DSC05181.ARW"
            ))
            .unwrap(),
            "images:2021/2021-06/2021-06-11/DSC05181.ARW"
        );
    }
}
