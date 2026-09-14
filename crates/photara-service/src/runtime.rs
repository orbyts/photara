use crate::{
    MIGRATOR, Result, ServiceError,
    auth::{Clock, IdentityVerifier},
    canonical, hash,
};
use photara_core::contracts::access::ProjectAction;
use photara_core::contracts::{AccountId, DeviceId, LibraryId, ProjectId};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use std::sync::Arc;
use storexa::{Database, DatabaseConfig, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct Actor {
    pub(crate) account: AccountId,
    pub(crate) identity: Uuid,
    pub(crate) expires_ms: i64,
}
impl Actor {
    #[must_use]
    pub const fn account(&self) -> AccountId {
        self.account
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Library {
        library: LibraryId,
    },
    Project {
        library: LibraryId,
        project: ProjectId,
    },
}
impl Scope {
    #[must_use]
    pub const fn library(self) -> LibraryId {
        match self {
            Self::Library { library } | Self::Project { library, .. } => library,
        }
    }
    #[must_use]
    pub const fn project(self) -> Option<ProjectId> {
        match self {
            Self::Library { .. } => None,
            Self::Project { project, .. } => Some(project),
        }
    }
    pub(crate) const fn kind(self) -> &'static str {
        match self {
            Self::Library { .. } => "library",
            Self::Project { .. } => "project",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub scope: Scope,
    pub device: DeviceId,
    pub generation: i64,
}
/// Holds separate least-privileged Storexa pools. No raw connection is exported.
pub struct Service {
    #[cfg(test)]
    pub(crate) fault_phase: std::sync::atomic::AtomicU8,
    pub(crate) api: Database,
    pub(crate) control: Database,
    pub(crate) auth: Database,
    pub(crate) clock: Arc<dyn Clock>,
    verifier: Arc<dyn IdentityVerifier>,
    issuer: String,
    audience: String,
    pub(crate) cursor_key: [u8; 32],
}
impl Service {
    /// # Errors
    /// Rechecks exact ledger/floor and runtime-role drift without exposing connection data.
    pub async fn readiness(&self) -> Result<()> {
        for (db, role) in [
            (&self.api, "photara_api"),
            (&self.control, "photara_control"),
            (&self.auth, "photara_auth_read"),
        ] {
            validate_schema(db).await?;
            let safe:bool=sqlx::query_scalar("SELECT rolcanlogin AND NOT rolsuper AND NOT rolbypassrls AND NOT rolcreaterole AND NOT rolcreatedb AND NOT rolreplication AND NOT pg_has_role(current_user,'photara_owner','MEMBER') AND pg_has_role(current_user,$1,'MEMBER') AND NOT EXISTS(SELECT 1 FROM pg_roles r WHERE r.rolname IN ('photara_api','photara_control','photara_auth_read') AND r.rolname<>$1 AND pg_has_role(current_user,r.oid,'MEMBER')) FROM pg_roles WHERE rolname=current_user").bind(role).fetch_one(db.pool()).await?;
            if !safe {
                return Err(ServiceError::Forbidden);
            }
        }
        Ok(())
    }
    pub(crate) fn sign(&self, domain: &str, bytes: &[u8]) -> Result<[u8; 32]> {
        if domain.contains('\0') {
            return Err(ServiceError::Invalid);
        }
        // RFC 2104 HMAC-SHA256 with a fixed 32-byte key and domain separation.
        let mut inner = [0x36u8; 64];
        let mut outer = [0x5cu8; 64];
        for (i, b) in self.cursor_key.iter().enumerate() {
            inner[i] ^= b;
            outer[i] ^= b;
        }
        let mut payload = inner.to_vec();
        payload.extend_from_slice(domain.as_bytes());
        payload.push(0);
        payload.extend_from_slice(bytes);
        let mut envelope = outer.to_vec();
        envelope.extend_from_slice(&hash(&payload));
        Ok(hash(&envelope))
    }
    /// # Errors
    /// Rejects unsafe database roles, unknown floors/ledgers and invalid issuer settings.
    pub async fn connect(
        configs: [DatabaseConfig; 3],
        verifier: Arc<dyn IdentityVerifier>,
        clock: Arc<dyn Clock>,
        issuer: String,
        audience: String,
        cursor_key: [u8; 32],
    ) -> Result<Self> {
        if issuer.is_empty() || audience.is_empty() || cursor_key == [0; 32] {
            return Err(ServiceError::Invalid);
        }
        let [api, control, auth] = configs;
        let api = Database::connect(api).await?;
        let control = Database::connect(control).await?;
        let auth = Database::connect(auth).await?;
        for (db, role) in [
            (&api, "photara_api"),
            (&control, "photara_control"),
            (&auth, "photara_auth_read"),
        ] {
            let safe:bool=sqlx::query_scalar("SELECT rolcanlogin AND NOT rolsuper AND NOT rolbypassrls AND NOT rolcreaterole AND NOT rolcreatedb AND NOT rolreplication AND NOT pg_has_role(current_user,'photara_owner','MEMBER') AND pg_has_role(current_user,$1,'MEMBER') FROM pg_roles WHERE rolname=current_user").bind(role).fetch_one(db.pool()).await?;
            if !safe {
                return Err(ServiceError::Forbidden);
            }
            let other:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname IN ('photara_api','photara_control','photara_auth_read') AND rolname<>$1 AND pg_has_role(current_user,oid,'MEMBER'))").bind(role).fetch_one(db.pool()).await?;
            if other {
                return Err(ServiceError::Forbidden);
            }
            validate_schema(db).await?;
        }
        Ok(Self {
            #[cfg(test)]
            fault_phase: std::sync::atomic::AtomicU8::new(0),
            api,
            control,
            auth,
            clock,
            verifier,
            issuer,
            audience,
            cursor_key,
        })
    }
    /// # Errors
    /// Masks invalid, expired, revoked or unlinked identities identically.
    pub async fn authenticate(&self, token: &str) -> Result<Actor> {
        let c = self.verifier.verify(token)?;
        if c.issuer != self.issuer
            || c.audience != self.audience
            || c.subject.is_empty()
            || c.expires_ms <= self.clock.now_ms()
        {
            return Err(ServiceError::Forbidden);
        }
        let row=sqlx::query("SELECT i.identity_id,i.account_id FROM photara_identity.account_identities i JOIN photara_identity.accounts a USING(account_id) WHERE i.issuer=$1 AND i.subject=$2 AND i.state='active' AND a.state='active'").bind(c.issuer).bind(c.subject).fetch_optional(self.auth.pool()).await?.ok_or(ServiceError::Forbidden)?;
        Ok(Actor {
            account: AccountId::from_uuid(row.try_get("account_id")?)
                .map_err(|_| ServiceError::Integrity)?,
            identity: row.try_get("identity_id")?,
            expires_ms: c.expires_ms,
        })
    }
    pub(crate) async fn context(
        &self,
        actor: &Actor,
        request: Request,
        targets: &[AccountId],
    ) -> Result<Transaction> {
        if actor.expires_ms <= self.clock.now_ms() || request.generation < 1 {
            return Err(ServiceError::Forbidden);
        }
        let mut tx = self.control.begin().await?;
        set_context(&mut tx, actor, request).await?;
        let floor: i32 = sqlx::query_scalar(
            "SELECT minimum_api FROM photara.schema_metadata WHERE singleton FOR SHARE",
        )
        .fetch_one(&mut *tx)
        .await?;
        if floor != 3 {
            return Err(ServiceError::Unsupported);
        }
        // Lock all Accounts first, then all Identities, in stable UUID order.
        let mut ids: Vec<_> = targets
            .iter()
            .map(|a| a.uuid())
            .chain([actor.account.uuid()])
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let accounts=sqlx::query("SELECT account_id,state FROM photara_identity.accounts WHERE account_id=ANY($1) ORDER BY account_id FOR SHARE").bind(&ids).fetch_all(&mut *tx).await?;
        if accounts.len() != ids.len()
            || accounts
                .iter()
                .any(|r| r.get::<String, _>("state") != "active")
        {
            return Err(ServiceError::Forbidden);
        }
        let identities=sqlx::query("SELECT identity_id,account_id,state FROM photara_identity.account_identities WHERE account_id=ANY($1) ORDER BY identity_id FOR SHARE").bind(&ids).fetch_all(&mut *tx).await?;
        if !identities.iter().any(|r| {
            r.get::<Uuid, _>("identity_id") == actor.identity
                && r.get::<Uuid, _>("account_id") == actor.account.uuid()
                && r.get::<String, _>("state") == "active"
        }) {
            return Err(ServiceError::Forbidden);
        }
        for id in ids {
            if !identities.iter().any(|r| {
                r.get::<Uuid, _>("account_id") == id && r.get::<String, _>("state") == "active"
            }) {
                return Err(ServiceError::Forbidden);
            }
        }
        let device:Option<String>=sqlx::query_scalar("SELECT state FROM photara_identity.devices WHERE account_id=$1 AND device_id=$2 FOR SHARE").bind(actor.account.uuid()).bind(request.device.uuid()).fetch_optional(&mut *tx).await?;
        if device.as_deref() != Some("active") {
            return Err(ServiceError::Forbidden);
        }
        Ok(tx)
    }
    pub(crate) async fn begin(
        &self,
        actor: &Actor,
        request: Request,
        action: ProjectAction,
        targets: &[AccountId],
    ) -> Result<Transaction> {
        let mut tx = self.context(actor, request, targets).await?;
        authorize(&mut tx, request.scope, action).await?;
        check_generation(&mut tx, request).await?;
        Ok(tx)
    }
    /// # Errors
    /// Returns only whether the current actor may perform the exact scoped action.
    pub async fn permits(
        &self,
        actor: &Actor,
        request: Request,
        action: ProjectAction,
    ) -> Result<bool> {
        match self.begin(actor, request, action, &[]).await {
            Ok(tx) => {
                tx.commit().await?;
                Ok(true)
            }
            Err(ServiceError::Forbidden) => Ok(false),
            Err(e) => Err(e),
        }
    }
    /// # Errors
    /// Rejects unavailable DB connections.
    pub async fn close(&self) {
        self.api.close().await;
        self.control.close().await;
        self.auth.close().await;
    }
}
pub(crate) async fn validate_schema(db: &Database) -> Result<()> {
    let valid:bool=sqlx::query_scalar("SELECT schema_family='photara.service.g2' AND schema_epoch=1 AND minimum_api=3 AND canonical_codec='photara.canonical-json.v1' FROM photara.schema_metadata WHERE singleton").fetch_one(db.pool()).await?;
    if !valid {
        return Err(ServiceError::Unsupported);
    }
    let ledger = sqlx::query(
        "SELECT version,checksum,success FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(db.pool())
    .await?;
    if ledger.len() != MIGRATOR.iter().count() {
        return Err(ServiceError::Unsupported);
    }
    for (row, migration) in ledger.iter().zip(MIGRATOR.iter()) {
        if row.try_get::<i64, _>("version")? != migration.version
            || !row.try_get::<bool, _>("success")?
            || row.try_get::<Vec<u8>, _>("checksum")?.as_slice() != migration.checksum.as_ref()
        {
            return Err(ServiceError::Unsupported);
        }
    }
    Ok(())
}
pub(crate) async fn set_context(tx: &mut Transaction, actor: &Actor, r: Request) -> Result<()> {
    for (key, value) in [
        ("photara.account_id", actor.account.to_string()),
        ("photara.identity_id", actor.identity.to_string()),
        ("photara.library_id", r.scope.library().to_string()),
        ("photara.scope_kind", r.scope.kind().into()),
        (
            "photara.project_id",
            r.scope
                .project()
                .map_or_else(String::new, |p| p.to_string()),
        ),
        ("photara.device_id", r.device.to_string()),
        ("photara.purpose", String::new()),
        ("photara.authorization_generation", r.generation.to_string()),
    ] {
        sqlx::query("SELECT set_config($1,$2,true)")
            .bind(key)
            .bind(value)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
pub(crate) async fn authorize(
    tx: &mut Transaction,
    scope: Scope,
    action: ProjectAction,
) -> Result<()> {
    let action = serde_json::to_value(action).map_err(|_| ServiceError::Invalid)?;
    let action = action.as_str().ok_or(ServiceError::Invalid)?;
    if let Some(project) = scope.project() {
        sqlx::query("SELECT photara_private.authorize_project($1,$2,$3)")
            .bind(scope.library().uuid())
            .bind(project.uuid())
            .bind(action)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("SELECT photara_private.authorize_library($1,$2)")
            .bind(scope.library().uuid())
            .bind(action)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
pub(crate) async fn check_generation(tx: &mut Transaction, r: Request) -> Result<()> {
    let current: i64 = sqlx::query_scalar(
        "SELECT authorization_generation FROM photara.library_contract_state WHERE library_id=$1",
    )
    .bind(r.scope.library().uuid())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ServiceError::Forbidden)?;
    if current != r.generation {
        return Err(ServiceError::Forbidden);
    }
    Ok(())
}
pub(crate) async fn bump(tx: &mut Transaction, scope: Scope) -> Result<i64> {
    let generation=sqlx::query_scalar("UPDATE photara.library_contract_state SET authorization_generation=authorization_generation+1,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 RETURNING authorization_generation").bind(scope.library().uuid()).fetch_one(&mut **tx).await?;
    sqlx::query("UPDATE photara.project_access_policies SET authorization_generation=authorization_generation+1,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND ($2::uuid IS NULL OR project_id=$2)").bind(scope.library().uuid()).bind(scope.project().map(ProjectId::uuid)).execute(&mut **tx).await?;
    Ok(generation)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlReceipt {
    pub operation: photara_core::contracts::OperationId,
    pub generation: i64,
    pub target: Uuid,
}
pub(crate) async fn retry<T: Serialize>(
    tx: &mut Transaction,
    actor: &Actor,
    r: Request,
    op: photara_core::contracts::OperationId,
    body: &T,
) -> Result<Option<ControlReceipt>> {
    let bytes = canonical(body)?;
    let row=sqlx::query("SELECT actor_account_id,device_id,request_canonical,request_sha256,response_canonical,response_sha256 FROM photara_private.scoped_command_receipts WHERE library_id=$1 AND operation_id=$2").bind(r.scope.library().uuid()).bind(op.uuid()).fetch_optional(&mut **tx).await?;
    if let Some(row) = row {
        if row.try_get::<Uuid, _>("actor_account_id")? != actor.account.uuid()
            || row.try_get::<Uuid, _>("device_id")? != r.device.uuid()
            || row.try_get::<Vec<u8>, _>("request_canonical")? != bytes
        {
            return Err(ServiceError::Conflict);
        }
        if row.try_get::<Vec<u8>, _>("request_sha256")? != hash(&bytes) {
            return Err(ServiceError::Integrity);
        }
        let bytes: Vec<u8> = row.try_get("response_canonical")?;
        if row.try_get::<Vec<u8>, _>("response_sha256")? != hash(&bytes) {
            return Err(ServiceError::Integrity);
        }
        let receipt: ControlReceipt =
            serde_json::from_slice(&bytes).map_err(|_| ServiceError::Integrity)?;
        if canonical(&receipt)? != bytes {
            return Err(ServiceError::Integrity);
        }
        Ok(Some(receipt))
    } else {
        Ok(None)
    }
}
pub(crate) async fn record<T: Serialize>(
    tx: &mut Transaction,
    actor: &Actor,
    r: Request,
    body: &T,
    kind: &str,
    receipt: &ControlReceipt,
) -> Result<()> {
    let bytes = canonical(body)?;
    let response = canonical(receipt)?;
    sqlx::query("INSERT INTO photara_private.scoped_command_receipts(library_id,operation_id,actor_account_id,device_id,scope_kind,project_id,command_kind,request_canonical,request_sha256,outcome,response_canonical,response_sha256,completed_at) VALUES($1,$2,$3,$4,'control',$5,$6,$7,$8,'control-accepted',$9,$10,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).bind(receipt.operation.uuid()).bind(actor.account.uuid()).bind(r.device.uuid()).bind(r.scope.project().map(ProjectId::uuid)).bind(kind).bind(&bytes).bind(hash(&bytes).to_vec()).bind(&response).bind(hash(&response).to_vec()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO photara_private.security_audit VALUES($1,$2,$3,$4,'control',$5,CURRENT_TIMESTAMP,$6)").bind(Uuid::new_v4()).bind(actor.account.uuid()).bind(r.scope.library().uuid()).bind(kind).bind(receipt.target).bind(serde_json::to_value(receipt).map_err(|_|ServiceError::Invalid)?).execute(&mut **tx).await?;
    Ok(())
}
