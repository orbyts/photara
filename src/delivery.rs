use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use reqwest::{Client, StatusCode, multipart};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use url::Url;
use uuid::Uuid;

use crate::{
    PhotaraError, Result,
    config::{PhotaraConfig, validate_slug},
    credentials::{CredentialStore, SecretId, SystemCredentialStore},
    layout::{PostPlatform, ResolvedPost, resolve_post},
    project::ProjectRecord,
};

const PROVIDER: &str = "cloudinary";
const CREDENTIAL_KIND: &str = "credentials";
const API_KEY_ENV: &str = "CLOUDINARY_API_KEY";
const API_SECRET_ENV: &str = "CLOUDINARY_API_SECRET";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CloudinaryCredentials {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct CloudinaryLoginReport {
    pub schema_version: u32,
    pub provider: &'static str,
    pub account_label: String,
    pub cloud_name: String,
    pub folder_mode: String,
    pub authenticated: bool,
    pub credentials_stored: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloudinaryProbeReport {
    pub schema_version: u32,
    pub provider: &'static str,
    pub account_label: String,
    pub cloud_name: String,
    pub folder_mode: String,
    pub plan: String,
    pub resources: u64,
    pub credits_used: f64,
    pub credits_limit: f64,
    pub root_folders: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeliveryManifest {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub account_label: String,
    pub cloud_name: String,
    pub folder_mode: String,
    pub source_specification: PathBuf,
    pub source_specification_sha256: String,
    pub asset_folder: String,
    pub item_count: usize,
    pub asset_count: usize,
    pub assets: Vec<DeliveryManifestAsset>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeliveryManifestAsset {
    pub asset_index: u32,
    pub item_id: String,
    pub frame_index: u32,
    pub local_relative_path: PathBuf,
    pub source_sha256: String,
    pub source_byte_size: u64,
    pub width: u32,
    pub height: u32,
    pub color_profile: String,
    pub asset_folder: String,
    pub public_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeliveryPreparationReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub asset_count: usize,
    pub reused: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeliveryUploadReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub uploaded: usize,
    pub reused: usize,
    pub remaining: usize,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeliveryVerificationReport {
    pub schema_version: u32,
    pub batch_id: Uuid,
    pub verified: usize,
    pub state: String,
}

#[derive(Clone)]
struct CloudinaryClient {
    http: Client,
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

#[derive(Debug, Deserialize)]
struct ConfigResponse {
    settings: ConfigSettings,
}

#[derive(Debug, Deserialize)]
struct ConfigSettings {
    folder_mode: String,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    plan: String,
    resources: u64,
    credits: UsageCredits,
}

#[derive(Debug, Deserialize)]
struct UsageCredits {
    usage: f64,
    limit: f64,
}

#[derive(Debug, Deserialize)]
struct FoldersResponse {
    folders: Vec<FolderResponse>,
}

#[derive(Debug, Deserialize)]
struct FolderResponse {
    path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ResourceResponse {
    asset_id: String,
    public_id: String,
    version: i64,
    secure_url: String,
    bytes: u64,
    format: String,
    etag: Option<String>,
    context: Option<ResourceContext>,
}

#[derive(Clone, Debug, Deserialize)]
struct ResourceContext {
    custom: BTreeMap<String, String>,
}

impl CloudinaryClient {
    fn from_store(account_label: &str) -> Result<Self> {
        let store = SystemCredentialStore;
        let id = SecretId::new(PROVIDER, account_label, CREDENTIAL_KIND)?;
        let bytes = store.load(&id)?.ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "no Cloudinary credential bundle is stored for account {account_label:?}; run `photara delivery cloudinary-login`"
            ))
        })?;
        let credentials: CloudinaryCredentials = serde_json::from_slice(&bytes).map_err(|_| {
            PhotaraError::Configuration(
                "stored Cloudinary credential bundle is not valid JSON; rerun `photara delivery cloudinary-login`"
                    .into(),
            )
        })?;
        Ok(Self {
            http: http_client()?,
            cloud_name: credentials.cloud_name,
            api_key: credentials.api_key,
            api_secret: credentials.api_secret,
        })
    }

    fn endpoint(&self, action: &str) -> Result<Url> {
        Ok(Url::parse(&format!(
            "https://api.cloudinary.com/v1_1/{}/{action}",
            self.cloud_name
        ))?)
    }

    async fn config(&self) -> Result<ConfigResponse> {
        Ok(self
            .http
            .get(self.endpoint("config")?)
            .query(&[("settings", "true")])
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn resource(&self, public_id: &str) -> Result<Option<ResourceResponse>> {
        let mut url = self.endpoint("resources/image/upload")?;
        url.path_segments_mut()
            .map_err(|_| PhotaraError::Configuration("invalid Cloudinary resource URL".into()))?
            .push(public_id);
        let response = self
            .http
            .get(url)
            .query(&[("context", "true")])
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(response.error_for_status()?.json().await?))
    }

    async fn upload(
        &self,
        project_root: &Path,
        manifest: &DeliveryManifest,
        asset: &DeliveryManifestAsset,
    ) -> Result<(ResourceResponse, bool)> {
        if let Some(existing) = self.resource(&asset.public_id).await? {
            verify_provider_identity(asset, &existing)?;
            return Ok((existing, true));
        }
        let path = project_root.join(&asset.local_relative_path);
        let bytes = fs::read(&path)
            .map_err(|source| PhotaraError::filesystem("read delivery source", &path, source))?;
        let context = format!(
            "photara_sha256={}|photara_project={}|photara_post={}|photara_platform={}|photara_item={}|photara_asset_index={}",
            asset.source_sha256,
            manifest.project,
            manifest.post,
            manifest.platform.as_str(),
            asset.item_id,
            asset.asset_index,
        );
        let tags = format!(
            "photara,{},{}-{},{}",
            manifest.project,
            manifest.platform.as_str(),
            manifest.post,
            asset.item_id
        );
        let timestamp = Utc::now().timestamp().to_string();
        let signature_source = format!(
            "asset_folder={}&context={}&display_name={}&overwrite=false&public_id={}&tags={}&timestamp={}{}",
            asset.asset_folder,
            context,
            asset.display_name,
            asset.public_id,
            tags,
            timestamp,
            self.api_secret,
        );
        let signature = format!("{:x}", Sha1::digest(signature_source.as_bytes()));
        let part = multipart::Part::bytes(bytes)
            .file_name(asset.display_name.clone())
            .mime_str("image/jpeg")?;
        let form = multipart::Form::new()
            .part("file", part)
            .text("public_id", asset.public_id.clone())
            .text("asset_folder", asset.asset_folder.clone())
            .text("display_name", asset.display_name.clone())
            .text("overwrite", "false")
            .text("context", context)
            .text("tags", tags)
            .text("timestamp", timestamp)
            .text("api_key", self.api_key.clone())
            .text("signature", signature);
        let uploaded: ResourceResponse = self
            .http
            .post(self.endpoint("image/upload")?)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        verify_provider_identity(asset, &uploaded)?;
        Ok((uploaded, false))
    }
}

pub async fn login(account_label: &str, cloud_name: &str) -> Result<CloudinaryLoginReport> {
    validate_account(account_label)?;
    let cloud_name = cloud_name.trim();
    if cloud_name.is_empty() {
        return Err(PhotaraError::Configuration(
            "Cloudinary cloud name must not be empty".into(),
        ));
    }
    let api_key = env::var(API_KEY_ENV).map_err(|_| {
        PhotaraError::Configuration(format!("{API_KEY_ENV} is required for Cloudinary login"))
    })?;
    let api_secret = env::var(API_SECRET_ENV).map_err(|_| {
        PhotaraError::Configuration(format!("{API_SECRET_ENV} is required for Cloudinary login"))
    })?;
    let client = CloudinaryClient {
        http: http_client()?,
        cloud_name: cloud_name.into(),
        api_key: api_key.clone(),
        api_secret: api_secret.clone(),
    };
    let config = client.config().await?;
    if !matches!(config.settings.folder_mode.as_str(), "dynamic" | "fixed") {
        return Err(PhotaraError::Configuration(format!(
            "unsupported Cloudinary folder mode {:?}",
            config.settings.folder_mode
        )));
    }
    let store = SystemCredentialStore;
    let credentials = CloudinaryCredentials {
        cloud_name: cloud_name.into(),
        api_key,
        api_secret,
    };
    store.save(
        &SecretId::new(PROVIDER, account_label, CREDENTIAL_KIND)?,
        &serde_json::to_vec(&credentials)?,
    )?;
    Ok(CloudinaryLoginReport {
        schema_version: 1,
        provider: PROVIDER,
        account_label: account_label.into(),
        cloud_name: cloud_name.into(),
        folder_mode: config.settings.folder_mode,
        authenticated: true,
        credentials_stored: true,
    })
}

pub async fn probe(account_label: &str) -> Result<CloudinaryProbeReport> {
    validate_account(account_label)?;
    let client = CloudinaryClient::from_store(account_label)?;
    let config = client.config().await?;
    let usage: UsageResponse = client
        .http
        .get(client.endpoint("usage")?)
        .basic_auth(&client.api_key, Some(&client.api_secret))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let folders: FoldersResponse = client
        .http
        .get(client.endpoint("folders")?)
        .basic_auth(&client.api_key, Some(&client.api_secret))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(CloudinaryProbeReport {
        schema_version: 1,
        provider: PROVIDER,
        account_label: account_label.into(),
        cloud_name: client.cloud_name,
        folder_mode: config.settings.folder_mode,
        plan: usage.plan,
        resources: usage.resources,
        credits_used: usage.credits.usage,
        credits_limit: usage.credits.limit,
        root_folders: folders
            .folders
            .into_iter()
            .map(|folder| folder.path)
            .collect(),
    })
}

pub async fn prepare(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    account_label: &str,
) -> Result<DeliveryPreparationReport> {
    validate_account(account_label)?;
    validate_slug(post_name).map_err(|message| PhotaraError::Configuration(message.to_string()))?;
    let client = CloudinaryClient::from_store(account_label)?;
    let provider_config = client.config().await?;
    let resolved = resolve_post(database, config, project, post_name, platform).await?;
    if !resolved.ready {
        return Err(PhotaraError::Configuration(format!(
            "post is not ready for delivery: {}",
            resolved.requirements.join("; ")
        )));
    }
    let project_root = config.settings.projects_root.join(&project.slug);
    let asset_folder = format!(
        "photara/{}/{}/{}",
        project.slug,
        platform.as_str(),
        post_name
    );
    let assets = resolve_delivery_assets(&project_root, &resolved, &asset_folder)?;
    let batch_id = Uuid::new_v4();
    let manifest = DeliveryManifest {
        schema_version: 1,
        batch_id,
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        account_label: account_label.into(),
        cloud_name: client.cloud_name.clone(),
        folder_mode: provider_config.settings.folder_mode,
        source_specification: resolved.source_path.clone(),
        source_specification_sha256: resolved.source_sha256.clone(),
        asset_folder: asset_folder.clone(),
        item_count: resolved.items.len(),
        asset_count: assets.len(),
        assets,
    };
    let mut manifest_identity = serde_json::to_value(&manifest)?;
    manifest_identity
        .as_object_mut()
        .expect("delivery manifest serializes as an object")
        .remove("batch_id");
    let canonical = serde_json::to_vec(&manifest_identity)?;
    let manifest_sha256 = sha256_bytes(&canonical);
    let existing = sqlx::query(
        "SELECT id FROM cloudinary_delivery_batches \
         WHERE project_id=$1 AND post_name=$2 AND platform=$3 AND account_label=$4 \
           AND source_specification_sha256=$5 AND manifest_sha256=$6",
    )
    .bind(project.id)
    .bind(post_name)
    .bind(platform.as_str())
    .bind(account_label)
    .bind(&resolved.source_sha256)
    .bind(&manifest_sha256)
    .fetch_optional(database.pool())
    .await?;
    let (batch_id, reused) =
        if let Some(row) = existing {
            (row.try_get("id")?, true)
        } else {
            let mut transaction = database.begin().await?;
            sqlx::query(
                "INSERT INTO cloudinary_delivery_batches \
             (id,project_id,post_name,platform,account_label,cloud_name,folder_mode, \
              source_specification_sha256,manifest_sha256,asset_folder,item_count, \
              asset_count) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            )
            .bind(batch_id)
            .bind(project.id)
            .bind(post_name)
            .bind(platform.as_str())
            .bind(account_label)
            .bind(&client.cloud_name)
            .bind(&manifest.folder_mode)
            .bind(&resolved.source_sha256)
            .bind(&manifest_sha256)
            .bind(&asset_folder)
            .bind(
                i32::try_from(manifest.item_count)
                    .map_err(|_| PhotaraError::Configuration("item count overflow".into()))?,
            )
            .bind(
                i32::try_from(manifest.asset_count)
                    .map_err(|_| PhotaraError::Configuration("asset count overflow".into()))?,
            )
            .execute(&mut *transaction)
            .await?;
            for asset in &manifest.assets {
                sqlx::query(
                    "INSERT INTO cloudinary_delivery_assets \
                 (batch_id,asset_index,item_id,frame_index,local_relative_path,source_sha256, \
                  source_byte_size,width,height,color_profile,asset_folder,public_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
                )
                .bind(batch_id)
                .bind(i32::try_from(asset.asset_index).unwrap())
                .bind(&asset.item_id)
                .bind(i32::try_from(asset.frame_index).unwrap())
                .bind(asset.local_relative_path.to_string_lossy().as_ref())
                .bind(&asset.source_sha256)
                .bind(i64::try_from(asset.source_byte_size).map_err(|_| {
                    PhotaraError::Configuration("delivery byte size overflow".into())
                })?)
                .bind(i32::try_from(asset.width).unwrap())
                .bind(i32::try_from(asset.height).unwrap())
                .bind(&asset.color_profile)
                .bind(&asset.asset_folder)
                .bind(&asset.public_id)
                .execute(&mut *transaction)
                .await?;
            }
            transaction.commit().await?;
            (batch_id, false)
        };
    let manifest_path = project_root
        .join("manifests")
        .join("delivery")
        .join(platform.as_str())
        .join(format!("{post_name}-{batch_id}.json"));
    let mut stored_manifest = manifest;
    stored_manifest.batch_id = batch_id;
    write_json_atomic(&manifest_path, &stored_manifest)?;
    Ok(DeliveryPreparationReport {
        schema_version: 1,
        batch_id,
        manifest_path,
        manifest_sha256,
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        asset_count: stored_manifest.asset_count,
        reused,
    })
}

pub async fn upload_canary(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
    confirmed: bool,
) -> Result<DeliveryUploadReport> {
    if !confirmed {
        return Err(PhotaraError::Configuration(
            "upload-canary creates one Cloudinary asset; inspect the delivery manifest, then retry with --confirm".into(),
        ));
    }
    upload_range(database, config, batch_id, true).await
}

pub async fn upload_remaining(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
    confirmed: bool,
) -> Result<DeliveryUploadReport> {
    if !confirmed {
        return Err(PhotaraError::Configuration(
            "upload-remaining creates Cloudinary assets; verify the canary, then retry with --confirm".into(),
        ));
    }
    upload_range(database, config, batch_id, false).await
}

pub async fn verify_canary(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
) -> Result<DeliveryVerificationReport> {
    let manifest = load_manifest_for_batch(database, config, batch_id).await?;
    let asset = manifest.assets.first().ok_or_else(|| {
        PhotaraError::Configuration("delivery manifest contains no backup assets".into())
    })?;
    let state: String = sqlx::query_scalar(
        "SELECT state FROM cloudinary_delivery_assets WHERE batch_id=$1 AND asset_index=$2",
    )
    .bind(batch_id)
    .bind(i32::try_from(asset.asset_index).unwrap())
    .fetch_one(database.pool())
    .await?;
    if !matches!(state.as_str(), "uploaded" | "verified") {
        return Err(PhotaraError::Configuration(
            "upload the canary before verifying it".into(),
        ));
    }
    let client = CloudinaryClient::from_store(&manifest.account_label)?;
    verify_remote_asset(&client, asset).await?;
    mark_asset_verified(database, batch_id, asset.asset_index).await?;
    Ok(DeliveryVerificationReport {
        schema_version: 1,
        batch_id,
        verified: 1,
        state: "canary-verified".into(),
    })
}

pub async fn verify(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
) -> Result<DeliveryVerificationReport> {
    let manifest = load_manifest_for_batch(database, config, batch_id).await?;
    let client = CloudinaryClient::from_store(&manifest.account_label)?;
    let uploaded_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloudinary_delivery_assets \
         WHERE batch_id=$1 AND state IN ('uploaded','verified')",
    )
    .bind(batch_id)
    .fetch_one(database.pool())
    .await?;
    if usize::try_from(uploaded_count).unwrap_or(0) != manifest.assets.len() {
        return Err(PhotaraError::Configuration(
            "delivery batch is incomplete; upload all remaining assets before verification".into(),
        ));
    }
    let mut verified = 0usize;
    for asset in &manifest.assets {
        verify_remote_asset(&client, asset).await?;
        mark_asset_verified(database, batch_id, asset.asset_index).await?;
        verified += 1;
    }
    if verified == manifest.assets.len() {
        sqlx::query(
            "UPDATE cloudinary_delivery_batches SET state='verified',updated_at=now(),verified_at=now() WHERE id=$1",
        )
        .bind(batch_id)
        .execute(database.pool())
        .await?;
    }
    Ok(DeliveryVerificationReport {
        schema_version: 1,
        batch_id,
        verified,
        state: if verified == manifest.assets.len() {
            "verified"
        } else {
            "incomplete"
        }
        .into(),
    })
}

async fn upload_range(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
    canary_only: bool,
) -> Result<DeliveryUploadReport> {
    let manifest = load_manifest_for_batch(database, config, batch_id).await?;
    let project_root = config.settings.projects_root.join(&manifest.project);
    let client = CloudinaryClient::from_store(&manifest.account_label)?;
    let selected = if canary_only {
        &manifest.assets[..1]
    } else {
        &manifest.assets[..]
    };
    let mut uploaded = 0;
    let mut reused = 0;
    for asset in selected {
        let state: String = sqlx::query_scalar(
            "SELECT state FROM cloudinary_delivery_assets WHERE batch_id=$1 AND asset_index=$2",
        )
        .bind(batch_id)
        .bind(i32::try_from(asset.asset_index).unwrap())
        .fetch_one(database.pool())
        .await?;
        if matches!(state.as_str(), "uploaded" | "verified") {
            reused += 1;
            continue;
        }
        verify_local_asset(&project_root, asset)?;
        let (resource, was_reused) = client.upload(&project_root, &manifest, asset).await?;
        record_upload(database, batch_id, asset.asset_index, &resource).await?;
        if was_reused {
            reused += 1;
        } else {
            uploaded += 1;
        }
    }
    let uploaded_total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloudinary_delivery_assets WHERE batch_id=$1 AND state IN ('uploaded','verified')",
    )
    .bind(batch_id)
    .fetch_one(database.pool())
    .await?;
    let remaining = manifest
        .assets
        .len()
        .saturating_sub(usize::try_from(uploaded_total).unwrap_or(0));
    let proposed_state = if remaining == 0 {
        "uploaded"
    } else {
        "canary-uploaded"
    };
    sqlx::query(
        "UPDATE cloudinary_delivery_batches SET state=$2,updated_at=now() WHERE id=$1 AND state <> 'verified'",
    )
    .bind(batch_id)
    .bind(proposed_state)
    .execute(database.pool())
    .await?;
    let state: String =
        sqlx::query_scalar("SELECT state FROM cloudinary_delivery_batches WHERE id=$1")
            .bind(batch_id)
            .fetch_one(database.pool())
            .await?;
    Ok(DeliveryUploadReport {
        schema_version: 1,
        batch_id,
        uploaded,
        reused,
        remaining,
        state,
    })
}

async fn verify_remote_asset(
    client: &CloudinaryClient,
    asset: &DeliveryManifestAsset,
) -> Result<()> {
    let resource = client.resource(&asset.public_id).await?.ok_or_else(|| {
        PhotaraError::Configuration(format!("Cloudinary asset {:?} is missing", asset.public_id))
    })?;
    verify_provider_identity(asset, &resource)?;
    let response = client
        .http
        .get(&resource.secure_url)
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    if sha256_bytes(&bytes) != asset.source_sha256 {
        return Err(PhotaraError::Configuration(format!(
            "Cloudinary original {:?} differs from the WSP source",
            asset.public_id
        )));
    }
    Ok(())
}

async fn mark_asset_verified(database: &Database, batch_id: Uuid, asset_index: u32) -> Result<()> {
    let result = sqlx::query(
        "UPDATE cloudinary_delivery_assets SET state='verified', verified_at=now() \
         WHERE batch_id=$1 AND asset_index=$2 AND state IN ('uploaded','verified')",
    )
    .bind(batch_id)
    .bind(i32::try_from(asset_index).unwrap())
    .execute(database.pool())
    .await?;
    if result.rows_affected() != 1 {
        return Err(PhotaraError::Configuration(format!(
            "delivery evidence for backup asset {asset_index} could not be marked verified"
        )));
    }
    Ok(())
}

async fn record_upload(
    database: &Database,
    batch_id: Uuid,
    asset_index: u32,
    resource: &ResourceResponse,
) -> Result<()> {
    sqlx::query(
        "UPDATE cloudinary_delivery_assets SET state='uploaded',cloudinary_asset_id=$3, \
         cloudinary_version=$4,secure_url=$5,provider_byte_size=$6,provider_format=$7, \
         provider_etag=$8,uploaded_at=now() WHERE batch_id=$1 AND asset_index=$2",
    )
    .bind(batch_id)
    .bind(i32::try_from(asset_index).unwrap())
    .bind(&resource.asset_id)
    .bind(resource.version)
    .bind(&resource.secure_url)
    .bind(
        i64::try_from(resource.bytes)
            .map_err(|_| PhotaraError::Configuration("provider byte size overflow".into()))?,
    )
    .bind(&resource.format)
    .bind(&resource.etag)
    .execute(database.pool())
    .await?;
    Ok(())
}

async fn load_manifest_for_batch(
    database: &Database,
    config: &PhotaraConfig,
    batch_id: Uuid,
) -> Result<DeliveryManifest> {
    let row = sqlx::query(
        "SELECT p.slug,b.post_name,b.platform FROM cloudinary_delivery_batches b \
         JOIN projects p ON p.id=b.project_id WHERE b.id=$1",
    )
    .bind(batch_id)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!("delivery batch {batch_id} was not found"))
    })?;
    let project: String = row.try_get("slug")?;
    let post: String = row.try_get("post_name")?;
    let platform = match row.try_get::<String, _>("platform")?.as_str() {
        "instagram" => PostPlatform::Instagram,
        "threads" => PostPlatform::Threads,
        value => {
            return Err(PhotaraError::Configuration(format!(
                "delivery batch has unsupported platform {value:?}"
            )));
        }
    };
    let path = config
        .settings
        .projects_root
        .join(&project)
        .join("manifests/delivery")
        .join(platform.as_str())
        .join(format!("{post}-{batch_id}.json"));
    let bytes = fs::read(&path)
        .map_err(|source| PhotaraError::filesystem("read delivery manifest", &path, source))?;
    let manifest: DeliveryManifest = serde_json::from_slice(&bytes)?;
    if manifest.batch_id != batch_id
        || manifest.project != project
        || manifest.post != post
        || manifest.platform != platform
    {
        return Err(PhotaraError::Configuration(format!(
            "delivery manifest {} has the wrong identity",
            path.display()
        )));
    }
    Ok(manifest)
}

fn resolve_delivery_assets(
    project_root: &Path,
    resolved: &ResolvedPost,
    asset_folder: &str,
) -> Result<Vec<DeliveryManifestAsset>> {
    let export_root = PathBuf::from("workspace/exports")
        .join(resolved.platform.as_str())
        .join(&resolved.name);
    let absolute_root = project_root.join(&export_root);
    let entries = fs::read_dir(&absolute_root).map_err(|source| {
        PhotaraError::filesystem("read WSP export directory", &absolute_root, source)
    })?;
    let mut jpeg_paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| {
            PhotaraError::filesystem("read WSP export entry", &absolute_root, source)
        })?;
    jpeg_paths.retain(|path| {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("jpg"))
    });
    jpeg_paths.sort();
    let expected = canonical_backup_frames(resolved);
    if jpeg_paths.len() != expected.len() {
        return Err(PhotaraError::Configuration(format!(
            "WSP export directory {} contains {} JPEGs; expected {}",
            absolute_root.display(),
            jpeg_paths.len(),
            expected.len()
        )));
    }
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    let mut mapped = Vec::with_capacity(jpeg_paths.len());
    for (position, path) in jpeg_paths.into_iter().enumerate() {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| PhotaraError::Configuration("WSP filename is not UTF-8".into()))?;
        let logical_stem = if resolved.platform == PostPlatform::Instagram {
            let (prefix, remainder) = stem.split_once('_').ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "Instagram WSP export {stem:?} has no ordinal prefix"
                ))
            })?;
            prefix.parse::<u32>().map_err(|_| {
                PhotaraError::Configuration(format!(
                    "Instagram WSP export {stem:?} has an invalid ordinal prefix"
                ))
            })?;
            remainder
        } else {
            stem
        };
        let (item_id, frame_index) = parse_delivery_stem(logical_stem);
        if !expected_set.contains(&(item_id.clone(), frame_index)) {
            return Err(PhotaraError::Configuration(format!(
                "WSP export {stem:?} does not map to a delivery frame in the post specification"
            )));
        }
        let bytes = fs::read(&path)
            .map_err(|source| PhotaraError::filesystem("read WSP JPEG", &path, source))?;
        let (width, height) = jpeg_dimensions(&bytes)?;
        let source_sha256 = sha256_bytes(&bytes);
        let display_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let public_leaf = stem.to_string();
        mapped.push(DeliveryManifestAsset {
            asset_index: u32::try_from(position + 1).unwrap(),
            item_id,
            frame_index,
            local_relative_path: export_root.join(&display_name),
            source_sha256,
            source_byte_size: u64::try_from(bytes.len()).unwrap(),
            width,
            height,
            color_profile: if bytes.windows(10).any(|window| window == b"Display P3") {
                "Display P3"
            } else {
                "embedded-or-unspecified"
            }
            .into(),
            asset_folder: asset_folder.into(),
            public_id: format!("{asset_folder}/{public_leaf}"),
            display_name,
        });
    }
    let actual_set = mapped
        .iter()
        .map(|asset| (asset.item_id.clone(), asset.frame_index))
        .collect::<BTreeSet<_>>();
    if actual_set != expected_set {
        return Err(PhotaraError::Configuration(
            "WSP delivery exports contain duplicate or missing post frames".into(),
        ));
    }
    Ok(mapped)
}

fn canonical_backup_frames(resolved: &ResolvedPost) -> Vec<(String, u32)> {
    resolved
        .items
        .iter()
        .flat_map(|item| {
            let count = item
                .template
                .template
                .surface
                .as_ref()
                .map(|surface| surface.frame_count)
                .unwrap_or(1);
            (1..=count).map(move |frame| (item.id.clone(), frame))
        })
        .collect()
}

fn parse_delivery_stem(stem: &str) -> (String, u32) {
    if let Some((item, frame)) = stem.rsplit_once("_col")
        && let Ok(frame) = frame.parse::<u32>()
    {
        return (item.into(), frame);
    }
    (stem.into(), 1)
}

fn verify_local_asset(project_root: &Path, asset: &DeliveryManifestAsset) -> Result<()> {
    let path = project_root.join(&asset.local_relative_path);
    let bytes = fs::read(&path)
        .map_err(|source| PhotaraError::filesystem("read delivery source", &path, source))?;
    if u64::try_from(bytes.len()).unwrap() != asset.source_byte_size
        || sha256_bytes(&bytes) != asset.source_sha256
    {
        return Err(PhotaraError::Configuration(format!(
            "delivery source {} changed after manifest preparation",
            path.display()
        )));
    }
    Ok(())
}

fn verify_provider_identity(
    asset: &DeliveryManifestAsset,
    resource: &ResourceResponse,
) -> Result<()> {
    let provider_sha = resource
        .context
        .as_ref()
        .and_then(|context| context.custom.get("photara_sha256"));
    if resource.public_id != asset.public_id
        || resource.bytes != asset.source_byte_size
        || provider_sha != Some(&asset.source_sha256)
    {
        return Err(PhotaraError::Configuration(format!(
            "Cloudinary public ID {:?} already exists with different evidence",
            asset.public_id
        )));
    }
    Ok(())
}

fn validate_account(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains(':') {
        return Err(PhotaraError::Configuration(
            "Cloudinary account label must be non-empty and cannot contain ':'".into(),
        ));
    }
    Ok(())
}

fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()?)
}

fn jpeg_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(PhotaraError::Configuration(
            "delivery source is not a JPEG".into(),
        ));
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x00 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if offset + 2 > bytes.len() {
            break;
        }
        let length = usize::from(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
        if length < 2 || offset + length > bytes.len() {
            break;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) && length >= 7 {
            let height = u32::from(u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]));
            let width = u32::from(u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]));
            return Ok((width, height));
        }
        offset += length;
    }
    Err(PhotaraError::Configuration(
        "could not read JPEG dimensions".into(),
    ))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        PhotaraError::Configuration(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|source| {
        PhotaraError::filesystem("create delivery manifest directory", parent, source)
    })?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        Uuid::new_v4()
    ));
    let contents = format!("{}\n", serde_json::to_string_pretty(value)?);
    fs::write(&temporary, contents).map_err(|source| {
        PhotaraError::filesystem("write delivery manifest", &temporary, source)
    })?;
    fs::rename(&temporary, path)
        .map_err(|source| PhotaraError::filesystem("install delivery manifest", path, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_and_multi_frame_export_names() {
        assert_eq!(parse_delivery_stem("hero"), ("hero".into(), 1));
        assert_eq!(
            parse_delivery_stem("panorama-05382_col2"),
            ("panorama-05382".into(), 2)
        );
        assert_eq!(
            parse_delivery_stem("subject_colophon"),
            ("subject_colophon".into(), 1)
        );
    }

    #[test]
    fn reads_dimensions_from_jpeg_start_of_frame() {
        let bytes = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x01, 0x2c, 0x02, 0x80, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xd9,
        ];
        assert_eq!(jpeg_dimensions(&bytes).unwrap(), (640, 300));
    }

    #[test]
    fn rejects_non_jpeg_sources() {
        assert!(jpeg_dimensions(b"not a jpeg").is_err());
    }
}
