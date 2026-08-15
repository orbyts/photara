use std::{fs, path::Path};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use url::Url;
use uuid::Uuid;

use crate::{
    PhotaraError, Result,
    config::{PhotaraConfig, validate_slug},
    layout::{PostPlatform, show_post},
    project::ProjectRecord,
};

#[derive(Clone, Debug, Serialize)]
pub struct ManualPublicationEvidence {
    pub schema_version: u32,
    pub publication_id: Option<Uuid>,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub provider: String,
    pub account_label: String,
    pub publication_method: String,
    pub source_specification_sha256: String,
    pub external_url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub evidence_note: String,
    pub recorded: bool,
    pub action: String,
}

pub struct ManualPublicationInput<'a> {
    pub account_label: &'a str,
    pub external_url: Option<&'a str>,
    pub published_at: Option<DateTime<Utc>>,
    pub evidence_note: &'a str,
    pub confirmed: bool,
}

pub async fn confirm_manual(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    input: ManualPublicationInput<'_>,
) -> Result<ManualPublicationEvidence> {
    validate_slug(post_name).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post name {post_name:?}: {message}"))
    })?;
    let account_label = input.account_label.trim();
    if account_label.is_empty() {
        return Err(PhotaraError::Configuration(
            "publication account label must not be empty".into(),
        ));
    }
    let evidence_note = input.evidence_note.trim();
    if evidence_note.is_empty() {
        return Err(PhotaraError::Configuration(
            "manual publication evidence requires a non-empty note".into(),
        ));
    }
    let external_url = input
        .external_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(validate_external_url)
        .transpose()?;
    let shown = show_post(config, project, post_name, platform)?;
    let source_specification_sha256 = sha256_file(&shown.path)?;
    let provider = platform.as_str().to_string();
    if !input.confirmed {
        return Ok(ManualPublicationEvidence {
            schema_version: 1,
            publication_id: None,
            project: project.slug.clone(),
            post: post_name.into(),
            platform,
            provider,
            account_label: account_label.into(),
            publication_method: "manual-confirmation".into(),
            source_specification_sha256,
            external_url,
            published_at: input.published_at,
            confirmed_at: None,
            evidence_note: evidence_note.into(),
            recorded: false,
            action: "confirm-with---confirm".into(),
        });
    }
    let id = Uuid::new_v4();
    let row = sqlx::query(
        "INSERT INTO post_publications \
         (id, project_id, post_name, platform, provider, account_label, \
          publication_method, source_specification_sha256, external_url, \
          published_at, evidence_note) \
         VALUES ($1, $2, $3, $4, $5, $6, 'manual-confirmation', $7, $8, $9, $10) \
         ON CONFLICT (project_id, post_name, platform, provider, account_label, \
                      source_specification_sha256, publication_method) \
         DO UPDATE SET \
           external_url = COALESCE(post_publications.external_url, EXCLUDED.external_url), \
           published_at = COALESCE(post_publications.published_at, EXCLUDED.published_at), \
           evidence_note = CASE \
             WHEN post_publications.evidence_note = EXCLUDED.evidence_note \
             THEN post_publications.evidence_note \
             ELSE post_publications.evidence_note || E'\\n' || EXCLUDED.evidence_note \
           END \
         RETURNING id, confirmed_at, external_url, published_at",
    )
    .bind(id)
    .bind(project.id)
    .bind(post_name)
    .bind(platform.as_str())
    .bind(&provider)
    .bind(account_label)
    .bind(&source_specification_sha256)
    .bind(external_url.as_deref())
    .bind(input.published_at)
    .bind(evidence_note)
    .fetch_one(database.pool())
    .await?;
    Ok(ManualPublicationEvidence {
        schema_version: 1,
        publication_id: Some(row.try_get("id")?),
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        provider,
        account_label: account_label.into(),
        publication_method: "manual-confirmation".into(),
        source_specification_sha256,
        external_url: row.try_get("external_url")?,
        published_at: row.try_get("published_at")?,
        confirmed_at: Some(row.try_get("confirmed_at")?),
        evidence_note: evidence_note.into(),
        recorded: true,
        action: "recorded".into(),
    })
}

fn validate_external_url(value: &str) -> Result<String> {
    let parsed = Url::parse(value).map_err(|error| {
        PhotaraError::Configuration(format!("invalid publication URL {value:?}: {error}"))
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(PhotaraError::Configuration(
            "publication URL must use http or https".into(),
        ));
    }
    Ok(value.into())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|source| PhotaraError::filesystem("read post specification", path, source))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
