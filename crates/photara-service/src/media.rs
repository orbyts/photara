//! Fake storage port. Upload verification and URL issuance run outside DB transactions.
use crate::runtime::{record, retry};
use crate::{
    Actor, ControlReceipt, Disclosure, Request, Result, Scope, Service, ServiceError, canonical,
    hash,
};
use photara_core::contracts::access::{
    ActionMask, PolicySpec, ProjectAction, ProjectPolicy, VisibilityPolicy,
};
use photara_core::contracts::{OperationId, ProjectId};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use std::{collections::BTreeMap, sync::RwLock};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MediaPurpose {
    Cover,
    AssignedSnapshot,
}
impl MediaPurpose {
    fn sql(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::AssignedSnapshot => "assigned-snapshot",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateUpload {
    pub operation: OperationId,
    pub digest: [u8; 32],
    pub media_type: String,
    pub byte_length: i64,
    pub purpose: Option<MediaPurpose>,
    pub consent: Option<Disclosure>,
}
/// Production adapters must verify immutable uploaded bytes and issue short-lived scoped capabilities.
pub trait MediaAdapter: Send + Sync {
    /// # Errors
    /// Rejects unavailable bytes, mismatched digests or lengths.
    fn verify(&self, upload: Uuid, digest: [u8; 32], length: i64) -> Result<()>;
    /// # Errors
    /// Rejects unavailable capability issuance.
    fn issue(
        &self,
        scope: Scope,
        digest: [u8; 32],
        purpose: Option<MediaPurpose>,
    ) -> Result<String>;
}
#[derive(Default)]
pub struct FakeMedia {
    bytes: RwLock<BTreeMap<Uuid, Vec<u8>>>,
}
impl FakeMedia {
    /// # Errors
    /// Rejects replacement of staged bytes or an oversized test fixture.
    pub fn stage(&self, upload: Uuid, bytes: Vec<u8>) -> Result<()> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(ServiceError::Invalid);
        }
        let mut items = self.bytes.write().map_err(|_| ServiceError::Storage)?;
        if items.contains_key(&upload) {
            return Err(ServiceError::Conflict);
        }
        items.insert(upload, bytes);
        Ok(())
    }
}
impl MediaAdapter for FakeMedia {
    fn verify(&self, upload: Uuid, digest: [u8; 32], length: i64) -> Result<()> {
        let items = self.bytes.read().map_err(|_| ServiceError::Storage)?;
        let bytes = items.get(&upload).ok_or(ServiceError::Invalid)?;
        if hash(bytes) != digest
            || i64::try_from(bytes.len()).map_err(|_| ServiceError::Invalid)? != length
        {
            return Err(ServiceError::Integrity);
        }
        Ok(())
    }
    fn issue(
        &self,
        _scope: Scope,
        _digest: [u8; 32],
        _purpose: Option<MediaPurpose>,
    ) -> Result<String> {
        Ok(format!("https://fake-media.invalid/{}", Uuid::new_v4()))
    }
}
impl Service {
    /// # Errors
    /// Requires current edit rights and explicit Project consent; stores no secret object capability in a shared feed.
    pub async fn create_upload(
        &self,
        actor: &Actor,
        r: Request,
        c: &CreateUpload,
    ) -> Result<ControlReceipt> {
        if c.byte_length < 0
            || c.byte_length > 16 * 1024 * 1024
            || !matches!(
                c.media_type.as_str(),
                "image/jpeg" | "image/png" | "image/webp"
            )
            || c.digest == [0; 32]
            || r.scope.project().is_some() != c.purpose.is_some()
            || r.scope.project().is_some() != c.consent.is_some()
        {
            return Err(ServiceError::Invalid);
        }
        if let Some(consent) = &c.consent
            && (consent.actor != actor.account
                || consent.commit_sha256 == [0; 32]
                || consent.projection_sha256 == [0; 32])
        {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self.begin(actor, r, ProjectAction::Edit, &[]).await?;
        let body = (r.scope, "create-upload", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        if let Some(consent) = &c.consent {
            verify_consent(&mut tx, r, consent).await?;
        }
        sqlx::query("INSERT INTO photara.library_media VALUES($1,$2,$3,$4,NULL,NULL,CURRENT_TIMESTAMP) ON CONFLICT(library_id,sha256) DO NOTHING").bind(r.scope.library().uuid()).bind(c.digest.to_vec()).bind(&c.media_type).bind(c.byte_length).execute(&mut *tx).await?;
        let compatible:bool=sqlx::query_scalar("SELECT byte_length=$3 AND media_type=$4 FROM photara.library_media WHERE library_id=$1 AND sha256=$2").bind(r.scope.library().uuid()).bind(c.digest.to_vec()).bind(c.byte_length).bind(&c.media_type).fetch_one(&mut *tx).await?;
        if !compatible {
            return Err(ServiceError::Conflict);
        }
        sqlx::query("INSERT INTO photara_private.media_objects VALUES($1,$2,$3,'pending',NULL,CURRENT_TIMESTAMP) ON CONFLICT(library_id,sha256) DO NOTHING").bind(r.scope.library().uuid()).bind(c.digest.to_vec()).bind(Uuid::new_v4().to_string()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.media_upload_sessions(upload_id,library_id,sha256,staging_object_key,state,revision,created_at,updated_at,expires_at,project_id,actor_account_id,project_media_purpose,authorization_generation) VALUES($1,$2,$3,$4,'pending',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+interval '1 hour',$5,$6,$7,$8)").bind(c.operation.uuid()).bind(r.scope.library().uuid()).bind(c.digest.to_vec()).bind(c.operation.to_string()).bind(r.scope.project().map(ProjectId::uuid)).bind(actor.account.uuid()).bind(c.purpose.map(MediaPurpose::sql)).bind(r.generation).execute(&mut *tx).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation: r.generation,
            target: c.operation.uuid(),
        };
        record(&mut tx, actor, r, &body, "create-upload", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// # Errors
    /// Rechecks actor, scope, generation, consent and session revision after external verification.
    pub async fn finalize_upload(
        &self,
        actor: &Actor,
        r: Request,
        upload: OperationId,
        operation: OperationId,
        media: &dyn MediaAdapter,
    ) -> Result<ControlReceipt> {
        let mut tx = self.begin(actor, r, ProjectAction::Edit, &[]).await?;
        let body = (r.scope, "finalize-upload", upload, operation);
        if let Some(v) = retry(&mut tx, actor, r, operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        let c = upload_request(&mut tx, actor, r, upload).await?;
        tx.commit().await?;
        media.verify(upload.uuid(), c.digest, c.byte_length)?;
        let mut tx = self.begin(actor, r, ProjectAction::Edit, &[]).await?;
        if let Some(v) = retry(&mut tx, actor, r, operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        let current = upload_request(&mut tx, actor, r, upload).await?;
        if canonical(&current)? != canonical(&c)? {
            return Err(ServiceError::Integrity);
        }
        if let Some(consent) = &c.consent {
            verify_consent(&mut tx, r, consent).await?;
        }
        let n=sqlx::query("UPDATE photara_private.media_upload_sessions SET state='complete',revision=revision+1,updated_at=CURRENT_TIMESTAMP,completed_at=CURRENT_TIMESTAMP WHERE upload_id=$1 AND library_id=$2 AND actor_account_id=$3 AND authorization_generation=$4 AND revision=1 AND state='pending' AND expires_at>CURRENT_TIMESTAMP").bind(upload.uuid()).bind(r.scope.library().uuid()).bind(actor.account.uuid()).bind(r.generation).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(ServiceError::Conflict);
        }
        sqlx::query("UPDATE photara_private.media_objects SET state='available',verified_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND sha256=$2 AND state IN ('pending','available')").bind(r.scope.library().uuid()).bind(c.digest.to_vec()).execute(&mut *tx).await?;
        if let (Some(project), Some(purpose), Some(consent)) =
            (r.scope.project(), c.purpose, c.consent)
        {
            sqlx::query("INSERT INTO photara.project_media_links VALUES($1,$2,$3,$4,$5,$6,$7,'active',NULL,1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.digest.to_vec()).bind(purpose.sql()).bind(consent.commit.uuid()).bind(consent.commit_sha256.to_vec()).bind(consent.projection_sha256.to_vec()).execute(&mut *tx).await?;
        }
        let receipt = ControlReceipt {
            operation,
            generation: r.generation,
            target: upload.uuid(),
        };
        record(&mut tx, actor, r, &body, "finalize-upload", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// # Errors
    /// Requires exact Project digest/purpose consent or Library read; never exposes object keys.
    pub async fn media_url(
        &self,
        actor: &Actor,
        r: Request,
        digest: [u8; 32],
        purpose: Option<MediaPurpose>,
        media: &dyn MediaAdapter,
    ) -> Result<String> {
        if r.scope.project().is_some() != purpose.is_some() {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self.begin(actor, r, ProjectAction::Read, &[]).await?;
        if let (Some(project), Some(purpose)) = (r.scope.project(), purpose) {
            let allowed: bool =
                sqlx::query_scalar("SELECT photara_private.can_project_media($1,$2,$3,$4)")
                    .bind(r.scope.library().uuid())
                    .bind(project.uuid())
                    .bind(digest.to_vec())
                    .bind(purpose.sql())
                    .fetch_one(&mut *tx)
                    .await?;
            if !allowed {
                return Err(ServiceError::Forbidden);
            }
        }
        let available:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photara_private.media_objects WHERE library_id=$1 AND sha256=$2 AND state='available')").bind(r.scope.library().uuid()).bind(digest.to_vec()).fetch_one(&mut *tx).await?;
        if !available {
            return Err(ServiceError::Forbidden);
        }
        tx.commit().await?;
        media.issue(r.scope, digest, purpose)
    }
    /// Entitlements are independent of Project role masks. Unknown capabilities are disabled.
    /// # Errors
    /// Requires current scoped read before returning a bounded capability result.
    pub async fn entitlement(
        &self,
        actor: &Actor,
        r: Request,
        key: &str,
    ) -> Result<(bool, Option<i64>, bool)> {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self.begin(actor, r, ProjectAction::Read, &[]).await?;
        let row=sqlx::query("SELECT enabled,quota_limit,photara_private.developer_capability($1) AS developer FROM photara_private.library_capability($1)").bind(key).fetch_one(&mut *tx).await?;
        let value = (
            row.try_get("enabled")?,
            row.try_get("quota_limit")?,
            row.try_get("developer")?,
        );
        tx.commit().await?;
        Ok(value)
    }
}
async fn upload_request(
    tx: &mut storexa::Transaction,
    actor: &Actor,
    r: Request,
    upload: OperationId,
) -> Result<CreateUpload> {
    let row=sqlx::query("SELECT u.actor_account_id,u.authorization_generation,u.project_id,u.project_media_purpose,u.state,u.expires_at>CURRENT_TIMESTAMP AS fresh,r.request_canonical,r.request_sha256 FROM photara_private.media_upload_sessions u JOIN photara_private.scoped_command_receipts r ON r.library_id=u.library_id AND r.operation_id=u.upload_id WHERE u.library_id=$1 AND u.upload_id=$2 FOR UPDATE OF u").bind(r.scope.library().uuid()).bind(upload.uuid()).fetch_optional(&mut **tx).await?.ok_or(ServiceError::Forbidden)?;
    if row.try_get::<Option<Uuid>, _>("actor_account_id")? != Some(actor.account.uuid())
        || row.try_get::<Option<i64>, _>("authorization_generation")? != Some(r.generation)
        || row.try_get::<Option<Uuid>, _>("project_id")? != r.scope.project().map(ProjectId::uuid)
        || row.try_get::<String, _>("state")? != "pending"
        || !row.try_get::<bool, _>("fresh")?
    {
        return Err(ServiceError::Forbidden);
    }
    let bytes: Vec<u8> = row.try_get("request_canonical")?;
    if row.try_get::<Vec<u8>, _>("request_sha256")? != hash(&bytes) {
        return Err(ServiceError::Integrity);
    }
    let (scope, kind, c): (Scope, String, CreateUpload) =
        serde_json::from_slice(&bytes).map_err(|_| ServiceError::Integrity)?;
    if scope != r.scope
        || kind != "create-upload"
        || c.operation != upload
        || row.try_get::<Option<String>, _>("project_media_purpose")?
            != c.purpose.map(|p| p.sql().into())
    {
        return Err(ServiceError::Integrity);
    }
    Ok(c)
}
async fn verify_consent(tx: &mut storexa::Transaction, r: Request, c: &Disclosure) -> Result<()> {
    let p = r.scope.project().ok_or(ServiceError::Invalid)?;
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photara.package_observations WHERE library_id=$1 AND project_id=$2 AND commit_id=$3 AND commit_sha256=$4 AND projection_sha256=$5)").bind(r.scope.library().uuid()).bind(p.uuid()).bind(c.commit.uuid()).bind(c.commit_sha256.to_vec()).bind(c.projection_sha256.to_vec()).fetch_one(&mut **tx).await?;
    if !exists {
        return Err(ServiceError::Forbidden);
    }
    let row=sqlx::query("SELECT visibility_policy,owner_mask,admin_mask,editor_mask,viewer_mask FROM photara.project_access_policies WHERE library_id=$1 AND project_id=$2").bind(r.scope.library().uuid()).bind(p.uuid()).fetch_one(&mut **tx).await?;
    let mask = |key| -> Result<ActionMask> {
        ActionMask::new(
            u16::try_from(row.try_get::<i32, _>(key)?).map_err(|_| ServiceError::Integrity)?,
        )
        .map_err(|_| ServiceError::Integrity)
    };
    let policy = ProjectPolicy::try_from(PolicySpec {
        visibility: match row.try_get::<String, _>("visibility_policy")?.as_str() {
            "restricted" => VisibilityPolicy::Restricted,
            "library-visible" => VisibilityPolicy::LibraryVisible,
            _ => return Err(ServiceError::Integrity),
        },
        owner: mask("owner_mask")?,
        admin: mask("admin_mask")?,
        editor: mask("editor_mask")?,
        viewer: mask("viewer_mask")?,
    })
    .map_err(|_| ServiceError::Integrity)?;
    if hash(&canonical(&policy)?) != c.policy_sha256 {
        return Err(ServiceError::Forbidden);
    }
    Ok(())
}
