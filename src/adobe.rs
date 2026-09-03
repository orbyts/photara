use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::random;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    PhotaraError, Result,
    cloud::ADOBE_LIGHTROOM_PROVIDER,
    cloud_collection::CloudCollectionPlan,
    credentials::{CredentialStore, SecretId, SystemCredentialStore},
};

const AUTHORIZE_ENDPOINT: &str = "https://ims-na1.adobelogin.com/ims/authorize/v2";
const TOKEN_ENDPOINT: &str = "https://ims-na1.adobelogin.com/ims/token/v3";
const CATALOG_ENDPOINT: &str = "https://lr.adobe.io/v2/catalog";
const ACCOUNT_ENDPOINT: &str = "https://lr.adobe.io/v2/account";
const SCOPES: &str = "AdobeID,openid,offline_access,lr_partner_apis,lr_partner_rendition_apis";

#[derive(Clone, Debug)]
pub struct AdobeOAuthConfig {
    client_id: String,
    redirect_uri: Url,
}

#[derive(Clone, Debug)]
struct PkceRequest {
    verifier: String,
    state: String,
    authorization_url: Url,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CatalogResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    id: String,
    entitlement: AdobeEntitlement,
}

#[derive(Debug, Deserialize)]
struct AdobeEntitlement {
    status: String,
    storage: AdobeStorage,
}

#[derive(Debug, Deserialize)]
struct AdobeStorage {
    used: u64,
    limit: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdobeAsset {
    pub id: String,
    pub subtype: String,
    #[serde(default)]
    pub payload: AdobeAssetPayload,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AdobeAssetPayload {
    #[serde(rename = "captureDate")]
    pub capture_date: Option<String>,
    #[serde(rename = "importSource")]
    pub import_source: Option<AdobeImportSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdobeImportSource {
    #[serde(rename = "fileName")]
    pub file_name: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AdobeInventory {
    pub catalog_id: String,
    pub assets: Vec<AdobeAsset>,
}

#[derive(Debug, Deserialize)]
struct AssetPage {
    base: String,
    #[serde(default)]
    resources: Vec<AdobeAsset>,
    #[serde(default)]
    links: AdobePageLinks,
}

#[derive(Debug, Default, Deserialize)]
struct AdobePageLinks {
    next: Option<AdobeLink>,
}

#[derive(Debug, Deserialize)]
struct AdobeLink {
    href: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdobeAlbum {
    id: String,
    subtype: String,
    #[serde(rename = "serviceId")]
    service_id: Option<String>,
    #[serde(default)]
    payload: AdobeAlbumPayload,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AdobeAlbumPayload {
    name: Option<String>,
    parent: Option<AdobeResourceId>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdobeResourceId {
    id: String,
}

#[derive(Debug, Deserialize)]
struct AdobeAlbumAssetPage {
    base: String,
    #[serde(default)]
    resources: Vec<AdobeAlbumAsset>,
    #[serde(default)]
    links: AdobePageLinks,
}

#[derive(Debug, Deserialize)]
struct AdobeAlbumAsset {
    asset: AdobeResourceId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeProbeReport {
    pub provider: String,
    pub catalog_id: String,
    pub access_token_expires_in_seconds: u64,
    pub refresh_token_issued: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeLoginReport {
    pub provider: String,
    pub account_label: String,
    pub catalog_id: String,
    pub access_token_expires_in_seconds: u64,
    pub refresh_token_stored: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeLogoutReport {
    pub provider: String,
    pub account_label: String,
    pub credential_removed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeStatusReport {
    pub provider: String,
    pub account_label: String,
    pub configuration_valid: bool,
    pub refresh_token_stored: bool,
    pub client_id_fingerprint: String,
    pub credential_client_id_fingerprint: Option<String>,
    pub credential_matches_client_id: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeUploadPreflight {
    pub provider: String,
    pub account_label: String,
    pub catalog_id: String,
    pub entitlement_status: String,
    pub storage_used_bytes: u64,
    pub storage_limit_bytes: u64,
    pub storage_available_bytes: u64,
    pub required_bytes: u64,
    pub ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeUploadReport {
    pub provider: String,
    pub account_label: String,
    pub catalog_id: String,
    pub remote_asset_id: String,
    pub filename: String,
    pub byte_size: u64,
    pub asset_reused: bool,
    pub master_uploaded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdobeCollectionSyncReport {
    pub provider: String,
    pub account_label: String,
    pub catalog_id: String,
    pub collection_count: usize,
    pub created_collection_count: usize,
    pub verified_collection_count: usize,
    pub leaf_album_count: usize,
    pub cloud_asset_count: usize,
    pub verified_membership_count: usize,
}

impl AdobeOAuthConfig {
    pub fn from_environment() -> Result<Self> {
        let client_id = required_environment("PHOTARA_ADOBE_CLIENT_ID")?;
        let redirect_uri = Url::parse(&required_environment("PHOTARA_ADOBE_REDIRECT_URI")?)?;
        if !redirect_uri.scheme().starts_with("adobe+") {
            return Err(PhotaraError::Configuration(
                "PHOTARA_ADOBE_REDIRECT_URI must be Adobe's native adobe+ callback URI".into(),
            ));
        }
        if redirect_uri.query().is_some() || redirect_uri.fragment().is_some() {
            return Err(PhotaraError::Configuration(
                "PHOTARA_ADOBE_REDIRECT_URI must not contain a query or fragment".into(),
            ));
        }
        Ok(Self {
            client_id,
            redirect_uri,
        })
    }

    fn authorization_request(&self) -> Result<PkceRequest> {
        let verifier = random_urlsafe(32);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_urlsafe(32);
        let mut authorization_url = Url::parse(AUTHORIZE_ENDPOINT)?;
        authorization_url
            .query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", self.redirect_uri.as_str())
            .append_pair("scope", SCOPES)
            .append_pair("response_type", "code")
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);
        Ok(PkceRequest {
            verifier,
            state,
            authorization_url,
        })
    }

    fn authorization_code(&self, callback: &str, expected_state: &str) -> Result<String> {
        let callback = Url::parse(callback.trim())?;
        if callback.scheme() != self.redirect_uri.scheme()
            || callback.host_str() != self.redirect_uri.host_str()
            || callback.path() != self.redirect_uri.path()
        {
            return Err(PhotaraError::Configuration(
                "Adobe callback URI does not match PHOTARA_ADOBE_REDIRECT_URI".into(),
            ));
        }
        let values = callback
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        if let Some(error) = values.get("error") {
            let description = values
                .get("error_description")
                .map_or("authorization was rejected", |value| value.as_ref());
            return Err(PhotaraError::Configuration(format!(
                "Adobe authorization failed ({error}): {description}"
            )));
        }
        let state = values.get("state").ok_or_else(|| {
            PhotaraError::Configuration("Adobe callback did not contain OAuth state".into())
        })?;
        if state.as_ref() != expected_state {
            return Err(PhotaraError::Configuration(
                "Adobe callback OAuth state did not match the authorization request".into(),
            ));
        }
        values
            .get("code")
            .map(|value| value.to_string())
            .ok_or_else(|| {
                PhotaraError::Configuration(
                    "Adobe callback did not contain an authorization code".into(),
                )
            })
    }
}

pub async fn probe() -> Result<AdobeProbeReport> {
    let config = AdobeOAuthConfig::from_environment()?;
    let client = Client::new();
    let token = authorize(&client, &config).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    show_browser_success()?;
    Ok(AdobeProbeReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        catalog_id: catalog.id,
        access_token_expires_in_seconds: token.expires_in,
        refresh_token_issued: token.refresh_token.is_some(),
    })
}

pub async fn login(account_label: &str) -> Result<AdobeLoginReport> {
    let refresh_id = refresh_token_id(account_label)?;
    let config = AdobeOAuthConfig::from_environment()?;
    let client = Client::new();
    let token = authorize(&client, &config).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    let refresh_token = token.refresh_token.as_deref().ok_or_else(|| {
        PhotaraError::Configuration(
            "Adobe did not issue a refresh token; confirm offline_access is enabled and consented"
                .into(),
        )
    })?;
    SystemCredentialStore.save(&refresh_id, refresh_token.as_bytes())?;
    SystemCredentialStore.save(
        &client_fingerprint_id(account_label)?,
        client_id_fingerprint(&config.client_id).as_bytes(),
    )?;
    show_browser_success()?;
    Ok(AdobeLoginReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        catalog_id: catalog.id,
        access_token_expires_in_seconds: token.expires_in,
        refresh_token_stored: true,
    })
}

pub async fn verify(account_label: &str) -> Result<AdobeLoginReport> {
    let (client, config, token) = refreshed_session(account_label).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    Ok(AdobeLoginReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        catalog_id: catalog.id,
        access_token_expires_in_seconds: token.expires_in,
        refresh_token_stored: true,
    })
}

pub async fn inventory(account_label: &str) -> Result<AdobeInventory> {
    let (client, config, token) = refreshed_session(account_label).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    let mut url = Url::parse(&format!(
        "https://lr.adobe.io/v2/catalogs/{}/assets",
        catalog.id
    ))?;
    url.query_pairs_mut()
        .append_pair("limit", "500")
        .append_pair("subtype", "image")
        .append_pair("exclude", "incomplete");
    let mut assets = Vec::new();
    let mut asset_ids = HashSet::new();
    let mut visited_pages = HashSet::new();
    loop {
        validate_lightroom_url(&url)?;
        if !visited_pages.insert(url.as_str().to_owned()) {
            return Err(PhotaraError::Configuration(
                "Adobe asset pagination returned a repeated page".into(),
            ));
        }
        let response = client
            .get(url.clone())
            .header("X-API-Key", &config.client_id)
            .bearer_auth(&token.access_token)
            .send()
            .await?
            .error_for_status()?;
        let body = response.text().await?;
        let page: AssetPage = serde_json::from_str(strip_lightroom_json_prefix(&body))?;
        for asset in page.resources {
            if asset_ids.insert(asset.id.clone()) {
                assets.push(asset);
            }
        }
        let Some(next) = page.links.next else {
            break;
        };
        url = Url::parse(&page.base)?.join(&next.href)?;
    }
    assets.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(AdobeInventory {
        catalog_id: catalog.id,
        assets,
    })
}

pub async fn upload_preflight(
    account_label: &str,
    required_bytes: u64,
) -> Result<AdobeUploadPreflight> {
    let (client, config, token) = refreshed_session(account_label).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    let response = client
        .get(ACCOUNT_ENDPOINT)
        .header("X-API-Key", &config.client_id)
        .bearer_auth(&token.access_token)
        .send()
        .await?
        .error_for_status()?;
    let body = response.text().await?;
    let account: AccountResponse = serde_json::from_str(strip_lightroom_json_prefix(&body))?;
    let entitled = matches!(account.entitlement.status.as_str(), "subscriber" | "trial");
    let available = account
        .entitlement
        .storage
        .limit
        .saturating_sub(account.entitlement.storage.used);
    Ok(AdobeUploadPreflight {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        catalog_id: catalog.id,
        entitlement_status: account.entitlement.status,
        storage_used_bytes: account.entitlement.storage.used,
        storage_limit_bytes: account.entitlement.storage.limit,
        storage_available_bytes: available,
        required_bytes,
        ready: entitled && available >= required_bytes,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn upload_asset(
    account_label: &str,
    remote_asset_id: &str,
    filename: &str,
    sha256: &str,
    byte_size: u64,
    capture_date: chrono::NaiveDate,
    path: &Path,
) -> Result<AdobeUploadReport> {
    if remote_asset_id.len() != 32
        || !remote_asset_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PhotaraError::Configuration(
            "Adobe asset ID must be 32 lowercase hexadecimal characters".into(),
        ));
    }
    let metadata = fs::metadata(path)
        .map_err(|source| PhotaraError::filesystem("inspect Adobe upload", path, source))?;
    if metadata.len() != byte_size {
        return Err(PhotaraError::Configuration(
            "Adobe upload file size changed after preflight".into(),
        ));
    }
    let (client, config, token) = refreshed_session(account_label).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    let account_response = client
        .get(ACCOUNT_ENDPOINT)
        .header("X-API-Key", &config.client_id)
        .bearer_auth(&token.access_token)
        .send()
        .await?
        .error_for_status()?;
    let account_body = account_response.text().await?;
    let account: AccountResponse =
        serde_json::from_str(strip_lightroom_json_prefix(&account_body))?;
    let asset_url = format!(
        "https://lr.adobe.io/v2/catalogs/{}/assets/{remote_asset_id}",
        catalog.id
    );
    let existing = client
        .get(&asset_url)
        .header("X-API-Key", &config.client_id)
        .bearer_auth(&token.access_token)
        .send()
        .await?;
    let asset_reused = match existing.status() {
        StatusCode::OK => {
            let body = existing.text().await?;
            let asset: AdobeAsset = serde_json::from_str(strip_lightroom_json_prefix(&body))?;
            if asset
                .payload
                .import_source
                .as_ref()
                .and_then(|source| source.sha256.as_deref())
                == Some(sha256)
            {
                return Ok(AdobeUploadReport {
                    provider: ADOBE_LIGHTROOM_PROVIDER.into(),
                    account_label: account_label.into(),
                    catalog_id: catalog.id,
                    remote_asset_id: remote_asset_id.into(),
                    filename: filename.into(),
                    byte_size,
                    asset_reused: true,
                    master_uploaded: false,
                });
            }
            true
        }
        StatusCode::NOT_FOUND => false,
        status => {
            let body = existing.text().await.unwrap_or_default();
            return Err(PhotaraError::Configuration(format!(
                "Adobe asset lookup failed with {status}: {}",
                body.trim()
            )));
        }
    };
    if !asset_reused {
        let now = chrono::Utc::now().to_rfc3339();
        let response = client
            .put(&asset_url)
            .header("X-API-Key", &config.client_id)
            .header("If-None-Match", sha256)
            .bearer_auth(&token.access_token)
            .json(&serde_json::json!({
                "subtype": "image",
                "payload": {
                    "captureDate": format!("{capture_date}T00:00:00"),
                    "importSource": {
                        "fileName": filename,
                        "importedOnDevice": config.client_id,
                        "importedBy": account.id,
                        "importTimestamp": now,
                    }
                }
            }))
            .send()
            .await?;
        if response.status() != StatusCode::CREATED {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotaraError::Configuration(format!(
                "Adobe asset creation failed with {status}: {}",
                body.trim()
            )));
        }
    }
    let bytes = fs::read(path)
        .map_err(|source| PhotaraError::filesystem("read Adobe upload", path, source))?;
    let response = client
        .put(format!("{asset_url}/master"))
        .header("X-API-Key", &config.client_id)
        .bearer_auth(&token.access_token)
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await?;
    if response.status() != StatusCode::CREATED {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(PhotaraError::Configuration(format!(
            "Adobe master upload failed with {status}: {}",
            body.trim()
        )));
    }
    Ok(AdobeUploadReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        catalog_id: catalog.id,
        remote_asset_id: remote_asset_id.into(),
        filename: filename.into(),
        byte_size,
        asset_reused,
        master_uploaded: true,
    })
}

pub async fn sync_collections(
    account_label: &str,
    plan: &CloudCollectionPlan,
) -> Result<AdobeCollectionSyncReport> {
    if plan.provider != ADOBE_LIGHTROOM_PROVIDER || plan.account_label != account_label {
        return Err(PhotaraError::Configuration(
            "Cloud collection plan does not match the requested Adobe account".into(),
        ));
    }
    let (client, config, token) = refreshed_session(account_label).await?;
    let catalog = catalog(&client, &config, &token.access_token).await?;
    let mut created_collection_count = 0;
    let mut verified_collection_count = 0;
    let mut verified_membership_count = 0;
    for node in &plan.nodes {
        if ensure_album(&client, &config, &token.access_token, &catalog.id, node).await? {
            created_collection_count += 1;
        }
        verified_collection_count += 1;
        if node.node_kind == "album" {
            add_album_assets(
                &client,
                &config,
                &token.access_token,
                &catalog.id,
                &node.remote_id,
                &plan.assets,
            )
            .await?;
            let actual = list_album_asset_ids(
                &client,
                &config,
                &token.access_token,
                &catalog.id,
                &node.remote_id,
            )
            .await?;
            for asset in &plan.assets {
                if !actual.contains(&asset.remote_asset_id) {
                    return Err(PhotaraError::Configuration(format!(
                        "Adobe album {:?} does not contain verified asset {} after synchronization",
                        node.display_name, asset.remote_filename
                    )));
                }
                verified_membership_count += 1;
            }
        }
    }
    Ok(AdobeCollectionSyncReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        catalog_id: catalog.id,
        collection_count: plan.collection_count,
        created_collection_count,
        verified_collection_count,
        leaf_album_count: plan.leaf_album_count,
        cloud_asset_count: plan.cloud_asset_count,
        verified_membership_count,
    })
}

async fn ensure_album(
    client: &Client,
    config: &AdobeOAuthConfig,
    access_token: &str,
    catalog_id: &str,
    node: &crate::cloud_collection::CloudCollectionNode,
) -> Result<bool> {
    let url = format!(
        "https://lr.adobe.io/v2/catalogs/{catalog_id}/albums/{}",
        node.remote_id
    );
    let response = client
        .get(&url)
        .header("X-API-Key", &config.client_id)
        .bearer_auth(access_token)
        .send()
        .await?;
    match response.status() {
        StatusCode::OK => {
            let body = response.text().await?;
            let album: AdobeAlbum = serde_json::from_str(strip_lightroom_json_prefix(&body))?;
            let expected_subtype = if node.node_kind == "set" {
                "project_set"
            } else {
                "project"
            };
            let actual_parent = album
                .payload
                .parent
                .as_ref()
                .map(|parent| parent.id.as_str());
            if album.id != node.remote_id
                || album.subtype != expected_subtype
                || album.service_id.as_deref() != Some(config.client_id.as_str())
                || album.payload.name.as_deref() != Some(node.display_name.as_str())
                || actual_parent != node.parent_remote_id.as_deref()
            {
                return Err(PhotaraError::Configuration(format!(
                    "Adobe collection ID {} exists with metadata not owned by this Photara plan",
                    node.remote_id
                )));
            }
            Ok(false)
        }
        StatusCode::NOT_FOUND => {
            let now = chrono::Utc::now().to_rfc3339();
            let mut payload = serde_json::json!({
                "userCreated": now,
                "userUpdated": now,
                "name": node.display_name,
                "publishInfo": {
                    "version": 3,
                    "created": now,
                    "updated": now,
                    "remoteId": node.semantic_path,
                    "servicePayload": "photara-cloud-collection-v1"
                }
            });
            if let Some(parent) = &node.parent_remote_id {
                payload["parent"] = serde_json::json!({ "id": parent });
            }
            let subtype = if node.node_kind == "set" {
                "project_set"
            } else {
                "project"
            };
            let created = client
                .put(&url)
                .header("X-API-Key", &config.client_id)
                .bearer_auth(access_token)
                .json(&serde_json::json!({
                    "subtype": subtype,
                    "serviceId": config.client_id,
                    "payload": payload
                }))
                .send()
                .await?;
            if created.status() != StatusCode::CREATED {
                let status = created.status();
                let body = created.text().await.unwrap_or_default();
                return Err(PhotaraError::Configuration(format!(
                    "Adobe collection creation failed for {:?} with {status}: {}",
                    node.semantic_path,
                    body.trim()
                )));
            }
            Ok(true)
        }
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(PhotaraError::Configuration(format!(
                "Adobe collection lookup failed with {status}: {}",
                body.trim()
            )))
        }
    }
}

async fn add_album_assets(
    client: &Client,
    config: &AdobeOAuthConfig,
    access_token: &str,
    catalog_id: &str,
    album_id: &str,
    assets: &[crate::cloud_collection::CloudCollectionAsset],
) -> Result<()> {
    for chunk in assets.chunks(50) {
        let resources = chunk
            .iter()
            .map(|asset| {
                serde_json::json!({
                    "id": asset.remote_asset_id,
                    "payload": {
                        "cover": false,
                        "publishInfo": {
                            "remoteId": asset.asset_id.to_string(),
                            "servicePayload": "photara-project-membership-v1"
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        let response = client
            .put(format!(
                "https://lr.adobe.io/v2/catalogs/{catalog_id}/albums/{album_id}/assets"
            ))
            .header("X-API-Key", &config.client_id)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "resources": resources }))
            .send()
            .await?;
        if response.status() != StatusCode::CREATED {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotaraError::Configuration(format!(
                "Adobe album membership synchronization failed with {status}: {}",
                body.trim()
            )));
        }
    }
    Ok(())
}

async fn list_album_asset_ids(
    client: &Client,
    config: &AdobeOAuthConfig,
    access_token: &str,
    catalog_id: &str,
    album_id: &str,
) -> Result<HashSet<String>> {
    let mut url = Url::parse(&format!(
        "https://lr.adobe.io/v2/catalogs/{catalog_id}/albums/{album_id}/assets"
    ))?;
    let mut ids = HashSet::new();
    let mut visited_pages = HashSet::new();
    loop {
        validate_lightroom_url(&url)?;
        if !visited_pages.insert(url.as_str().to_owned()) {
            return Err(PhotaraError::Configuration(
                "Adobe album pagination returned a repeated page".into(),
            ));
        }
        let response = client
            .get(url.clone())
            .header("X-API-Key", &config.client_id)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?;
        let body = response.text().await?;
        let page: AdobeAlbumAssetPage = serde_json::from_str(strip_lightroom_json_prefix(&body))?;
        ids.extend(page.resources.into_iter().map(|item| item.asset.id));
        let Some(next) = page.links.next else {
            break;
        };
        url = Url::parse(&page.base)?.join(&next.href)?;
    }
    Ok(ids)
}

async fn refreshed_session(
    account_label: &str,
) -> Result<(Client, AdobeOAuthConfig, TokenResponse)> {
    let refresh_id = refresh_token_id(account_label)?;
    let store = SystemCredentialStore;
    let refresh_token = store.load(&refresh_id)?.ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "no Adobe credential is stored for account {account_label:?}; run `photara cloud adobe-login --account {account_label}`"
        ))
    })?;
    let refresh_token = String::from_utf8(refresh_token).map_err(|_| {
        PhotaraError::Credential("stored Adobe refresh token is not valid UTF-8".into())
    })?;
    let config = AdobeOAuthConfig::from_environment()?;
    let fingerprint_id = client_fingerprint_id(account_label)?;
    let configured_fingerprint = client_id_fingerprint(&config.client_id);
    if let Some(stored_fingerprint) = store.load(&fingerprint_id)? {
        let stored_fingerprint = String::from_utf8(stored_fingerprint).map_err(|_| {
            PhotaraError::Credential("stored Adobe client-ID fingerprint is not valid UTF-8".into())
        })?;
        if stored_fingerprint != configured_fingerprint {
            return Err(PhotaraError::Configuration(format!(
                "Adobe credential for account {account_label:?} was issued for a different client ID; run `photara cloud adobe-login --account {account_label}`"
            )));
        }
    }
    let client = Client::new();
    let token = refresh_access_token(&client, &config, &refresh_token, account_label).await?;
    if let Some(rotated) = token.refresh_token.as_deref() {
        store.save(&refresh_id, rotated.as_bytes())?;
    }
    store.save(&fingerprint_id, configured_fingerprint.as_bytes())?;
    Ok((client, config, token))
}

pub fn logout(account_label: &str) -> Result<AdobeLogoutReport> {
    let removed = SystemCredentialStore.delete(&refresh_token_id(account_label)?)?;
    SystemCredentialStore.delete(&client_fingerprint_id(account_label)?)?;
    Ok(AdobeLogoutReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        credential_removed: removed,
    })
}

pub fn status(account_label: &str) -> Result<AdobeStatusReport> {
    let config = AdobeOAuthConfig::from_environment()?;
    let store = SystemCredentialStore;
    let refresh_token_stored = store.load(&refresh_token_id(account_label)?)?.is_some();
    let configured = client_id_fingerprint(&config.client_id);
    let stored = store
        .load(&client_fingerprint_id(account_label)?)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|_| {
            PhotaraError::Credential("stored Adobe client-ID fingerprint is not valid UTF-8".into())
        })?;
    Ok(AdobeStatusReport {
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        configuration_valid: true,
        refresh_token_stored,
        client_id_fingerprint: configured.clone(),
        credential_matches_client_id: stored.as_ref().map(|value| value == &configured),
        credential_client_id_fingerprint: stored,
    })
}

async fn authorize(client: &Client, config: &AdobeOAuthConfig) -> Result<TokenResponse> {
    let request = config.authorization_request()?;
    let receiver = CallbackReceiver::prepare(&config.redirect_uri)?;
    println!(
        "Open this Adobe authorization URL:\n{}",
        request.authorization_url
    );
    if let Err(error) = webbrowser::open(request.authorization_url.as_str()) {
        tracing::warn!(%error, "could not open the system browser automatically");
    }
    eprintln!("Waiting for Adobe to return authorization to Photara...");
    let callback = receiver.wait().await?;
    let code = config.authorization_code(&callback, &request.state)?;
    let response = client
        .post(TOKEN_ENDPOINT)
        .query(&[("client_id", config.client_id.as_str())])
        .form(&[
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", request.verifier.as_str()),
        ])
        .send()
        .await?;
    token_response(response, None).await
}

async fn refresh_access_token(
    client: &Client,
    config: &AdobeOAuthConfig,
    refresh_token: &str,
    account_label: &str,
) -> Result<TokenResponse> {
    let response = client
        .post(TOKEN_ENDPOINT)
        .query(&[("client_id", config.client_id.as_str())])
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;
    token_response(response, Some(account_label)).await
}

async fn catalog(
    client: &Client,
    config: &AdobeOAuthConfig,
    access_token: &str,
) -> Result<CatalogResponse> {
    let response = client
        .get(CATALOG_ENDPOINT)
        .header("X-API-Key", &config.client_id)
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;
    let body = response.text().await?;
    Ok(serde_json::from_str(strip_lightroom_json_prefix(&body))?)
}

fn refresh_token_id(account_label: &str) -> Result<SecretId> {
    SecretId::new(ADOBE_LIGHTROOM_PROVIDER, account_label, "refresh-token")
}

fn client_fingerprint_id(account_label: &str) -> Result<SecretId> {
    SecretId::new(
        ADOBE_LIGHTROOM_PROVIDER,
        account_label,
        "oauth-client-id-sha256",
    )
}

fn client_id_fingerprint(client_id: &str) -> String {
    format!("{:x}", Sha256::digest(client_id.as_bytes()))[..12].to_owned()
}

async fn token_response(
    response: reqwest::Response,
    account_label: Option<&str>,
) -> Result<TokenResponse> {
    let status = response.status();
    if status.is_success() {
        return Ok(response.json().await?);
    }
    let body = response.text().await.unwrap_or_default();
    Err(PhotaraError::Configuration(oauth_failure_message(
        status,
        &body,
        account_label,
    )))
}

fn oauth_failure_message(status: StatusCode, body: &str, account_label: Option<&str>) -> String {
    let parsed = serde_json::from_str::<OAuthErrorResponse>(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|error| error.error.as_deref())
        .unwrap_or("unknown_error");
    let description = parsed
        .as_ref()
        .and_then(|error| error.error_description.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let detail = description
        .map(|value| format!(": {value}"))
        .unwrap_or_default();
    match (code, account_label) {
        ("invalid_grant", Some(account)) => format!(
            "Adobe authorization is expired, revoked, or no longer valid ({status}, {code}){detail}; run `photara cloud adobe-login --account {account}`"
        ),
        ("invalid_client", Some(account)) => format!(
            "Adobe rejected the configured client ID ({status}, {code}){detail}; verify PHOTARA_ADOBE_CLIENT_ID and then run `photara cloud adobe-login --account {account}`"
        ),
        _ => format!("Adobe OAuth token request failed ({status}, {code}){detail}"),
    }
}

fn validate_lightroom_url(url: &Url) -> Result<()> {
    if url.scheme() != "https" || url.host_str() != Some("lr.adobe.io") {
        return Err(PhotaraError::Configuration(format!(
            "refusing to send Adobe credentials to unexpected pagination URL {url}"
        )));
    }
    Ok(())
}

struct CallbackReceiver {
    callback_path: PathBuf,
}

impl CallbackReceiver {
    #[cfg(target_os = "macos")]
    fn prepare(redirect_uri: &Url) -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;

        let root = cache_root()?.join("photara").join("oauth");
        fs::create_dir_all(&root)
            .map_err(|source| PhotaraError::filesystem("create OAuth cache", &root, source))?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .map_err(|source| PhotaraError::filesystem("secure OAuth cache", &root, source))?;
        let callback_path = root.join("adobe-callback");
        remove_if_present(&callback_path)?;

        let script_path = root.join("callback.applescript");
        let application_path = root.join("Photara OAuth Callback.app");
        match fs::remove_dir_all(&application_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(PhotaraError::filesystem(
                    "replace OAuth callback helper",
                    &application_path,
                    source,
                ));
            }
        }
        let escaped_path = callback_path.to_string_lossy().replace('"', "\\\"");
        let script = format!(
            "on open location callbackUrl\n\
             do shell script \"umask 077; /usr/bin/printf %s \" & quoted form of callbackUrl & \" > \" & quoted form of \"{escaped_path}\"\n\
             end open location\n"
        );
        fs::write(&script_path, script).map_err(|source| {
            PhotaraError::filesystem("write OAuth callback helper", &script_path, source)
        })?;
        run_command(
            Command::new("/usr/bin/osacompile")
                .arg("-o")
                .arg(&application_path)
                .arg(&script_path),
            "compile OAuth callback helper",
        )?;
        let plist = application_path.join("Contents/Info.plist");
        let url_types = serde_json::json!([{
            "CFBundleURLName": "Adobe OAuth callback",
            "CFBundleURLSchemes": [redirect_uri.scheme()]
        }]);
        run_command(
            Command::new("/usr/bin/plutil")
                .arg("-insert")
                .arg("CFBundleURLTypes")
                .arg("-json")
                .arg(url_types.to_string())
                .arg(&plist),
            "configure OAuth callback URL scheme",
        )?;
        run_command(
            Command::new("/usr/bin/plutil")
                .arg("-insert")
                .arg("LSUIElement")
                .arg("-bool")
                .arg("YES")
                .arg(&plist),
            "configure OAuth callback helper",
        )?;
        run_command(
            Command::new("/usr/bin/codesign")
                .arg("--force")
                .arg("--sign")
                .arg("-")
                .arg(&application_path),
            "sign OAuth callback helper",
        )?;
        run_command(
            Command::new(
                "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
            )
            .arg("-f")
            .arg(&application_path),
            "register OAuth callback helper",
        )?;
        Ok(Self { callback_path })
    }

    #[cfg(not(target_os = "macos"))]
    fn prepare(_redirect_uri: &Url) -> Result<Self> {
        Err(PhotaraError::Configuration(
            "automatic Adobe native callback capture currently requires macOS".into(),
        ))
    }

    async fn wait(self) -> Result<String> {
        for _ in 0..600 {
            match fs::read_to_string(&self.callback_path) {
                Ok(callback) if !callback.trim().is_empty() => {
                    remove_if_present(&self.callback_path)?;
                    return Ok(callback);
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(PhotaraError::filesystem(
                        "read Adobe callback",
                        &self.callback_path,
                        source,
                    ));
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(PhotaraError::Configuration(
            "timed out waiting five minutes for Adobe authorization".into(),
        ))
    }
}

fn cache_root() -> Result<PathBuf> {
    if let Some(root) = env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(root));
    }
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".cache"))
        .ok_or_else(|| {
            PhotaraError::Configuration(
                "XDG_CACHE_HOME or HOME is required for Adobe OAuth callback state".into(),
            )
        })
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(PhotaraError::filesystem(
            "remove stale OAuth callback",
            path,
            source,
        )),
    }
}

fn run_command(command: &mut Command, action: &'static str) -> Result<()> {
    let output = command.output().map_err(|source| {
        PhotaraError::filesystem(action, PathBuf::from(command.get_program()), source)
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(PhotaraError::Configuration(format!(
        "could not {action}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn required_environment(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PhotaraError::Configuration(format!("{name} is not configured")))
}

fn random_urlsafe(bytes: usize) -> String {
    let mut random_bytes = vec![0_u8; bytes];
    for chunk in random_bytes.chunks_mut(32) {
        let generated: [u8; 32] = random();
        chunk.copy_from_slice(&generated[..chunk.len()]);
    }
    URL_SAFE_NO_PAD.encode(random_bytes)
}

fn strip_lightroom_json_prefix(value: &str) -> &str {
    let value = value.trim_start();
    if value.starts_with("while") {
        return value
            .find('}')
            .map_or(value, |end| value[end + 1..].trim_start());
    }
    value
}

fn show_browser_success() -> Result<()> {
    let root = cache_root()?.join("photara").join("oauth");
    fs::create_dir_all(&root)
        .map_err(|source| PhotaraError::filesystem("create OAuth cache", &root, source))?;
    let page = root.join("adobe-connected.html");
    fs::write(
        &page,
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Photara connected to Adobe</title>
  <style>
    :root { color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, sans-serif; }
    body { min-height: 100vh; margin: 0; display: grid; place-items: center; background: #111; color: #f5f5f5; }
    main { max-width: 36rem; padding: 3rem; text-align: center; }
    h1 { font-size: 2rem; margin-bottom: 1rem; }
    p { color: #b8b8b8; font-size: 1.1rem; line-height: 1.6; }
  </style>
</head>
<body><main>
  <h1>Photara is connected to Adobe</h1>
  <p>The Lightroom catalog connection was verified successfully. You can now close this tab.</p>
</main></body>
</html>
"#,
    )
    .map_err(|source| PhotaraError::filesystem("write OAuth success page", &page, source))?;
    let url = Url::from_file_path(&page).map_err(|()| {
        PhotaraError::Configuration("could not create the OAuth success page URL".into())
    })?;
    if let Err(error) = webbrowser::open(url.as_str()) {
        tracing::warn!(%error, "could not open the OAuth success page");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AdobeOAuthConfig {
        AdobeOAuthConfig {
            client_id: "client-id".into(),
            redirect_uri: Url::parse("adobe+client://adobeid/callback").unwrap(),
        }
    }

    #[test]
    fn authorization_request_uses_pkce_and_required_scopes() {
        let request = config().authorization_request().unwrap();
        let query = request
            .authorization_url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert!(query.get("scope").unwrap().contains("lr_partner_apis"));
        assert!(request.verifier.len() >= 43);
    }

    #[test]
    fn callback_requires_matching_state_and_redirect() {
        assert_eq!(
            config()
                .authorization_code(
                    "adobe+client://adobeid/callback?code=one-time&state=expected",
                    "expected",
                )
                .unwrap(),
            "one-time"
        );
        assert!(
            config()
                .authorization_code(
                    "adobe+client://adobeid/callback?code=one-time&state=wrong",
                    "expected",
                )
                .is_err()
        );
    }

    #[test]
    fn strips_lightroom_json_protection_prefix() {
        assert_eq!(
            strip_lightroom_json_prefix("while(1){} {\"id\":\"catalog\"}"),
            "{\"id\":\"catalog\"}"
        );
        assert_eq!(
            strip_lightroom_json_prefix("  while (1) {}\n{\"id\":\"catalog\"}"),
            "{\"id\":\"catalog\"}"
        );
    }

    #[test]
    fn invalid_grant_explains_how_to_reauthenticate_without_echoing_secrets() {
        let message = oauth_failure_message(
            StatusCode::BAD_REQUEST,
            r#"{"error":"invalid_grant","error_description":"Refresh token expired"}"#,
            Some("personal"),
        );
        assert!(message.contains("adobe-login --account personal"));
        assert!(message.contains("invalid_grant"));
        assert!(!message.contains("refresh_token="));
    }

    #[test]
    fn client_id_fingerprint_is_stable_and_non_revealing() {
        let fingerprint = client_id_fingerprint("private-client-identifier");
        assert_eq!(fingerprint.len(), 12);
        assert_ne!(fingerprint, "private-client-identifier");
        assert_eq!(
            fingerprint,
            client_id_fingerprint("private-client-identifier")
        );
    }
}
