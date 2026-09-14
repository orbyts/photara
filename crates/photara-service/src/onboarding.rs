//! Host-independent onboarding controllers. No provider I/O occurs under locks.
#![allow(clippy::needless_pass_by_value)] // Helpers consume committed byte/row ownership.
use crate::{
    Actor, ClaimLibrary, Request, Result, Scope, Service, ServiceError, auth::VerifiedClaims,
    canonical, hash,
};
use base64::Engine as _;
use photara_core::contracts::{AccountId, DeviceId, LibraryId, OperationId};
use ring::rand::SecureRandom as _;
use serde::{Deserialize, Serialize};
use sqlx::{Row as _, postgres::PgRow};
use storexa::Transaction;
use uuid::Uuid;

pub const SCHEMA: &str = "photara.onboarding.v1";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRequest {
    pub schema: String,
    pub operation_id: OperationId,
    pub environment_id: String,
    pub device_id: DeviceId,
    pub device_credential_sha256: String,
    pub device_display_name: String,
    pub requested_library_id: LibraryId,
    pub library_display_name: String,
    pub intent: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRequest {
    pub schema: String,
    pub action: String,
    pub operation_id: OperationId,
    pub request_sha256: String,
    pub device_id: DeviceId,
    pub device_credential_sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeResponse {
    pub schema: String,
    pub challenge_id: Uuid,
    pub nonce: String,
    pub expires_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCommand {
    pub schema: String,
    pub operation_id: OperationId,
    pub device_id: DeviceId,
    pub expected_credential_revision: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionReceipt {
    pub schema: String,
    pub operation_id: OperationId,
    pub request_sha256: String,
    pub action: String,
    pub credential_state: String,
    pub credential_revision: String,
    pub completed_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionResponse {
    pub schema: String,
    pub issuer: String,
    pub subject: String,
    pub account_id: AccountId,
    pub identity_id: Uuid,
    pub device_id: DeviceId,
    pub default_library_id: LibraryId,
    pub account_state: String,
    pub identity_state: String,
    pub device_state: String,
    pub credential_state: String,
    pub credential_revision: String,
    pub membership_id: Uuid,
    pub membership_role: String,
    pub membership_revision: String,
    pub authorization_generation: String,
    pub observed_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapReceipt {
    pub schema: String,
    pub environment_id: String,
    pub operation_id: OperationId,
    pub request_sha256: String,
    pub issuer: String,
    pub subject: String,
    pub account_id: AccountId,
    pub identity_id: Uuid,
    pub device_id: DeviceId,
    pub device_credential_sha256: String,
    pub outcome: String,
    pub requested_library_id: LibraryId,
    pub default_library_id: LibraryId,
    pub membership_id: Uuid,
    pub contract_version: String,
    pub authorization_generation: String,
    pub stream_id: Uuid,
    pub stream_epoch: Uuid,
    pub initial_high_water: String,
    pub completed_at: String,
}
/// Digest covers the enclosed canonical receipt bytes, not its own envelope.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptEnvelope {
    pub receipt: serde_json::Value,
    pub receipt_sha256: String,
}
pub(crate) struct Proof {
    pub challenge_id: Uuid,
    pub nonce_sha256: Vec<u8>,
    pub issued_ms: i64,
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| {
            [
                char::from(HEX[usize::from(b >> 4)]),
                char::from(HEX[usize::from(b & 15)]),
            ]
        })
        .collect()
}
pub(crate) fn digest(value: &str) -> Result<Vec<u8>> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(ServiceError::Invalid);
    }
    (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| ServiceError::Invalid))
        .collect()
}
pub(crate) fn command<T: Serialize + serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value: T = photara_core::contracts::schema::decode_strict(bytes, 65536)
        .map_err(|_| ServiceError::Invalid)?;
    if canonical(&value)? != bytes {
        return Err(ServiceError::Invalid);
    }
    Ok(value)
}
fn name(s: &str) -> bool {
    !s.is_empty() && s.trim() == s && s.len() <= 512 && !s.chars().any(char::is_control)
}
fn envelope(bytes: Vec<u8>, expected: Vec<u8>) -> Result<Vec<u8>> {
    if hash(&bytes).as_slice() != expected {
        return Err(ServiceError::Integrity);
    }
    canonical(&ReceiptEnvelope {
        receipt: serde_json::from_slice(&bytes).map_err(|_| ServiceError::Integrity)?,
        receipt_sha256: hex(&expected),
    })
}

impl Service {
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub(crate) fn onboarding_checkpoint(&self, phase: u8) -> Result<()> {
        let _ = phase;
        #[cfg(test)]
        if self.fault_phase.load(std::sync::atomic::Ordering::SeqCst) == phase {
            return Err(ServiceError::Storage);
        }
        Ok(())
    }
    pub(crate) async fn challenge(&self, c: &ChallengeRequest) -> Result<ChallengeResponse> {
        if c.schema != SCHEMA {
            return Err(ServiceError::Unsupported);
        }
        if !["bootstrap", "resume"].contains(&c.action.as_str()) {
            return Err(ServiceError::Invalid);
        }
        let request_hash = digest(&c.request_sha256)?;
        let commitment = digest(&c.device_credential_sha256)?;
        if commitment == [0; 32] {
            return Err(ServiceError::Invalid);
        }
        let mut random = [0; 32];
        ring::rand::SystemRandom::new()
            .fill(&mut random)
            .map_err(|_| ServiceError::Storage)?;
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random);
        let challenge_id = Uuid::new_v4();
        let now = self.clock.now_ms();
        let mut tx = self.control.begin().await?;
        onboarding_floor(&mut tx).await?;
        // Bounded retention cleanup; authoritative receipts never reference a
        // challenge for their lifetime and are never removed by this path.
        sqlx::query("WITH expired AS (SELECT challenge_id FROM photara_private.onboarding_challenges WHERE expires_at<clock_timestamp()-interval '24 hours' ORDER BY expires_at LIMIT 100 FOR UPDATE SKIP LOCKED) DELETE FROM photara_private.onboarding_challenges c USING expired e WHERE c.challenge_id=e.challenge_id").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.onboarding_challenges(challenge_id,nonce_sha256,action,operation_id,request_sha256,device_id,device_commitment,issued_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,to_timestamp($8::double precision/1000),to_timestamp($8::double precision/1000)+interval '5 minutes')")
            .bind(challenge_id).bind(hash(nonce.as_bytes()).to_vec()).bind(&c.action).bind(c.operation_id.uuid()).bind(request_hash).bind(c.device_id.uuid()).bind(commitment).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(ChallengeResponse {
            schema: SCHEMA.into(),
            challenge_id,
            nonce,
            expires_at: (now + 300_000).to_string(),
        })
    }
    pub(crate) async fn proof_facts(&self, challenge_id: Uuid) -> Result<Proof> {
        let row = sqlx::query("SELECT nonce_sha256,(extract(epoch FROM issued_at)*1000)::bigint AS issued_ms FROM photara_private.onboarding_challenges WHERE challenge_id=$1 AND consumed_at IS NULL AND expires_at>to_timestamp($2::double precision/1000)")
            .bind(challenge_id).bind(self.clock.now_ms()).fetch_optional(self.control.pool()).await?.ok_or(ServiceError::Forbidden)?;
        Ok(Proof {
            challenge_id,
            nonce_sha256: row.try_get("nonce_sha256")?,
            issued_ms: row.try_get("issued_ms")?,
        })
    }
    pub(crate) async fn principal_transaction(
        &self,
        claims: &VerifiedClaims,
    ) -> Result<Transaction> {
        if claims.expires_ms <= self.clock.now_ms()
            || claims.subject.is_empty()
            || claims.subject.len() > 255
        {
            return Err(ServiceError::Forbidden);
        }
        let mut tx = self.control.begin().await?;
        sqlx::query("SET LOCAL lock_timeout='3s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        onboarding_floor(&mut tx).await?;
        // JSON tuple encoding avoids ambiguous concatenation; hash is only a lock key.
        let key = hash(&canonical(&(&claims.issuer, &claims.subject))?);
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(i64::from_be_bytes(
                key[..8].try_into().map_err(|_| ServiceError::Integrity)?,
            ))
            .execute(&mut *tx)
            .await?;
        if claims.expires_ms <= self.clock.now_ms() {
            return Err(ServiceError::Forbidden);
        }
        Ok(tx)
    }
    pub(crate) async fn bootstrap(
        &self,
        claims: &VerifiedClaims,
        environment: &str,
        bytes: &[u8],
        credential: &[u8],
        proof: Option<&Proof>,
    ) -> Result<Vec<u8>> {
        for attempt in 0..3 {
            let result = self
                .bootstrap_once(claims, environment, bytes, credential, proof)
                .await;
            if !matches!(result, Err(ServiceError::RetryTransaction)) || attempt == 2 {
                return result;
            }
            retry_delay().await?;
        }
        Err(ServiceError::RetryTransaction)
    }
    #[allow(clippy::too_many_lines)] // One visible transaction boundary for the aggregate.
    async fn bootstrap_once(
        &self,
        claims: &VerifiedClaims,
        environment: &str,
        bytes: &[u8],
        credential: &[u8],
        proof: Option<&Proof>,
    ) -> Result<Vec<u8>> {
        let c: BootstrapRequest = command(bytes)?;
        if c.schema != SCHEMA {
            return Err(ServiceError::Unsupported);
        }
        if c.environment_id != environment
            || c.intent != "enroll-default"
            || !name(&c.device_display_name)
            || !name(&c.library_display_name)
            || credential.len() != 32
            || !crate::secret_eq(&digest(&c.device_credential_sha256)?, &hash(credential))
        {
            return Err(ServiceError::Invalid);
        }
        let commitment = hash(credential);
        let mut tx = self.principal_transaction(claims).await?;
        let mut actor = identity(&mut tx, claims).await?;
        if actor.is_none() {
            if proof.is_none() {
                return Err(ServiceError::Forbidden);
            }
            let account =
                AccountId::from_uuid(Uuid::new_v4()).map_err(|_| ServiceError::Integrity)?;
            let identity = Uuid::new_v4();
            sqlx::query("INSERT INTO photara_identity.accounts VALUES($1,'Photara Account','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(account.uuid()).execute(&mut *tx).await?;
            self.onboarding_checkpoint(1)?;
            sqlx::query("INSERT INTO photara_identity.account_identities VALUES($1,$2,$3,$4,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(identity).bind(account.uuid()).bind(&claims.issuer).bind(&claims.subject).execute(&mut *tx).await?;
            self.onboarding_checkpoint(2)?;
            actor = Some(Actor {
                account,
                identity,
                expires_ms: claims.expires_ms,
            });
        }
        let actor = actor.ok_or(ServiceError::Integrity)?;
        let device = device(&mut tx, &actor, c.device_id, &commitment, false).await?;
        let default = default_library(&mut tx, &actor).await?;
        if let Some(row) = receipt(&mut tx, claims, c.operation_id).await? {
            if device.is_none() || default.is_none() {
                return Err(ServiceError::Forbidden);
            }
            let response = replay(row, bytes, c.device_id, &commitment, "bootstrap")?;
            tx.commit().await?;
            return Ok(response);
        }
        let proof = proof.ok_or(ServiceError::Forbidden)?;
        if device.is_none() {
            sqlx::query("INSERT INTO photara_identity.devices VALUES($1,$2,$3,'active',CURRENT_TIMESTAMP,NULL)").bind(actor.account.uuid()).bind(c.device_id.uuid()).bind(&c.device_display_name).execute(&mut *tx).await?;
            self.onboarding_checkpoint(3)?;
            sqlx::query("INSERT INTO photara_identity.device_credentials VALUES($1,$2,$3,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(actor.account.uuid()).bind(c.device_id.uuid()).bind(commitment.to_vec()).execute(&mut *tx).await?;
            self.onboarding_checkpoint(4)?;
        }
        let (library, outcome) = if let Some(library) = default {
            (
                library,
                if library == c.requested_library_id {
                    "recovered-default"
                } else {
                    "existing-default-requires-local-choice"
                },
            )
        } else {
            let r = Request {
                scope: Scope::Library {
                    library: c.requested_library_id,
                },
                device: c.device_id,
                generation: 1,
            };
            crate::runtime::set_context(&mut tx, &actor, r).await?;
            self.claim_in_transaction(
                &mut tx,
                &actor,
                r,
                &ClaimLibrary {
                    operation: c.operation_id,
                    display_name: c.library_display_name.clone(),
                },
            )
            .await?;
            sqlx::query("INSERT INTO photara_identity.account_defaults VALUES($1,$2,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(actor.account.uuid()).bind(c.requested_library_id.uuid()).execute(&mut *tx).await?;
            self.onboarding_checkpoint(5)?;
            (c.requested_library_id, "claimed-default")
        };
        let row = coordinates(&mut tx, &actor, library).await?;
        consume(
            &mut tx,
            claims,
            &c.operation_id,
            c.device_id,
            &commitment,
            bytes,
            "bootstrap",
            proof,
            self.clock.now_ms(),
        )
        .await?;
        let response = BootstrapReceipt {
            // Challenge consumption has occurred, but no receipt has committed.
            schema: SCHEMA.into(),
            environment_id: environment.into(),
            operation_id: c.operation_id,
            request_sha256: hex(&hash(bytes)),
            issuer: claims.issuer.clone(),
            subject: claims.subject.clone(),
            account_id: actor.account,
            identity_id: actor.identity,
            device_id: c.device_id,
            device_credential_sha256: c.device_credential_sha256,
            outcome: outcome.into(),
            requested_library_id: c.requested_library_id,
            default_library_id: library,
            membership_id: row.try_get("membership_id")?,
            contract_version: row.try_get::<i32, _>("contract_version")?.to_string(),
            authorization_generation: row
                .try_get::<i64, _>("authorization_generation")?
                .to_string(),
            stream_id: row.try_get("stream_id")?,
            stream_epoch: row.try_get("stream_epoch")?,
            initial_high_water: row.try_get::<i64, _>("high_water")?.to_string(),
            completed_at: self.clock.now_ms().to_string(),
        };
        let response = canonical(&response)?;
        self.onboarding_checkpoint(6)?;
        save(
            &mut tx,
            claims,
            &actor,
            c.operation_id,
            c.device_id,
            &commitment,
            "bootstrap",
            bytes,
            &response,
        )
        .await?;
        if claims.expires_ms <= self.clock.now_ms() {
            return Err(ServiceError::Forbidden);
        }
        self.onboarding_checkpoint(7)?;
        tx.commit().await?;
        self.onboarding_checkpoint(8)?;
        envelope(response.clone(), hash(&response).to_vec())
    }
    pub(crate) async fn operation(
        &self,
        claims: &VerifiedClaims,
        operation: OperationId,
        credential: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        self.onboarding_checkpoint(9)?;
        let mut tx = self.principal_transaction(claims).await?;
        let Some(actor) = identity(&mut tx, claims).await? else {
            return Ok(None);
        };
        // Read only the device coordinate before locking it; immutable receipt is locked later.
        let row = receipt(&mut tx, claims, operation).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let id =
            DeviceId::from_uuid(row.try_get("device_id")?).map_err(|_| ServiceError::Integrity)?;
        let commitment = hash(credential);
        if credential.len() != 32 {
            return Err(ServiceError::Forbidden);
        }
        let is_logout = row.try_get::<String, _>("action")? == "logout";
        let (state, revision) = device(&mut tx, &actor, id, &commitment, is_logout)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        if state == "suspended" {
            let stored: SessionReceipt =
                command(&row.try_get::<Vec<u8>, _>("response_canonical")?)?;
            if stored.action != "logout"
                || stored.credential_state != "suspended"
                || stored.credential_revision != revision.to_string()
            {
                return Err(ServiceError::Forbidden);
            }
        }
        default_library(&mut tx, &actor)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        if !crate::secret_eq(
            &row.try_get::<Vec<u8>, _>("device_commitment")?,
            &commitment,
        ) {
            return Err(ServiceError::Forbidden);
        }
        let response = envelope(
            row.try_get("response_canonical")?,
            row.try_get("response_sha256")?,
        )?;
        tx.commit().await?;
        Ok(Some(response))
    }
    pub(crate) async fn session_change(
        &self,
        claims: &VerifiedClaims,
        bytes: &[u8],
        credential: &[u8],
        action: &str,
        proof: Option<&Proof>,
    ) -> Result<Vec<u8>> {
        for attempt in 0..3 {
            let result = self
                .session_change_once(claims, bytes, credential, action, proof)
                .await;
            if !matches!(result, Err(ServiceError::RetryTransaction)) || attempt == 2 {
                return result;
            }
            retry_delay().await?;
        }
        Err(ServiceError::RetryTransaction)
    }
    async fn session_change_once(
        &self,
        claims: &VerifiedClaims,
        bytes: &[u8],
        credential: &[u8],
        action: &str,
        proof: Option<&Proof>,
    ) -> Result<Vec<u8>> {
        let c: SessionCommand = command(bytes)?;
        if c.schema != SCHEMA {
            return Err(ServiceError::Unsupported);
        }
        let revision = c
            .expected_credential_revision
            .parse::<i64>()
            .map_err(|_| ServiceError::Invalid)?;
        if revision < 1
            || revision.to_string() != c.expected_credential_revision
            || credential.len() != 32
            || !["resume", "logout"].contains(&action)
        {
            return Err(ServiceError::Invalid);
        }
        let commitment = hash(credential);
        let mut tx = self.principal_transaction(claims).await?;
        let actor = identity(&mut tx, claims)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let (state, current) = device(&mut tx, &actor, c.device_id, &commitment, true)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        default_library(&mut tx, &actor)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        if let Some(row) = receipt(&mut tx, claims, c.operation_id).await? {
            if state != "active" && action != "logout" {
                return Err(ServiceError::Forbidden);
            }
            if action == "logout"
                && (state != "suspended"
                    || current != revision.checked_add(1).ok_or(ServiceError::Conflict)?)
            {
                return Err(ServiceError::Conflict);
            }
            let response = replay(row, bytes, c.device_id, &commitment, action)?;
            tx.commit().await?;
            return Ok(response);
        }
        if current != revision {
            return Err(ServiceError::Conflict);
        }
        if action == "resume" {
            consume(
                &mut tx,
                claims,
                &c.operation_id,
                c.device_id,
                &commitment,
                bytes,
                action,
                proof.ok_or(ServiceError::Forbidden)?,
                self.clock.now_ms(),
            )
            .await?;
        } else if state != "active" {
            return Err(ServiceError::Forbidden);
        }
        let target = if action == "resume" {
            "active"
        } else {
            "suspended"
        };
        let next = if state == target {
            current
        } else {
            current.checked_add(1).ok_or(ServiceError::Conflict)?
        };
        if next != current {
            sqlx::query("UPDATE photara_identity.device_credentials SET state=$3,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1 AND device_id=$2").bind(actor.account.uuid()).bind(c.device_id.uuid()).bind(target).execute(&mut *tx).await?;
        }
        let response = canonical(&SessionReceipt {
            schema: SCHEMA.into(),
            operation_id: c.operation_id,
            request_sha256: hex(&hash(bytes)),
            action: action.into(),
            credential_state: target.into(),
            credential_revision: next.to_string(),
            completed_at: self.clock.now_ms().to_string(),
        })?;
        save(
            &mut tx,
            claims,
            &actor,
            c.operation_id,
            c.device_id,
            &commitment,
            action,
            bytes,
            &response,
        )
        .await?;
        if claims.expires_ms <= self.clock.now_ms() {
            return Err(ServiceError::Forbidden);
        }
        tx.commit().await?;
        envelope(response.clone(), hash(&response).to_vec())
    }
    pub(crate) async fn session(
        &self,
        claims: &VerifiedClaims,
        id: DeviceId,
        credential: &[u8],
    ) -> Result<Vec<u8>> {
        if credential.len() != 32 {
            return Err(ServiceError::Forbidden);
        }
        let mut tx = self.principal_transaction(claims).await?;
        let actor = identity(&mut tx, claims)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let (_, revision) = device(&mut tx, &actor, id, &hash(credential), false)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let library = default_library(&mut tx, &actor)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let row = coordinates(&mut tx, &actor, library).await?;
        let result = canonical(&SessionResponse {
            schema: SCHEMA.into(),
            issuer: claims.issuer.clone(),
            subject: claims.subject.clone(),
            account_id: actor.account,
            identity_id: actor.identity,
            device_id: id,
            default_library_id: library,
            account_state: "active".into(),
            identity_state: "active".into(),
            device_state: "active".into(),
            credential_state: "active".into(),
            credential_revision: revision.to_string(),
            membership_id: row.try_get("membership_id")?,
            membership_role: "owner".into(),
            membership_revision: row.try_get::<i64, _>("membership_revision")?.to_string(),
            authorization_generation: row
                .try_get::<i64, _>("authorization_generation")?
                .to_string(),
            observed_at: self.clock.now_ms().to_string(),
        })?;
        tx.commit().await?;
        Ok(result)
    }
}
async fn retry_delay() -> Result<()> {
    let mut byte = [0];
    ring::rand::SystemRandom::new()
        .fill(&mut byte)
        .map_err(|_| ServiceError::Storage)?;
    tokio::time::sleep(std::time::Duration::from_millis(
        10 + u64::from(byte[0] % 32),
    ))
    .await;
    Ok(())
}
async fn onboarding_floor(tx: &mut Transaction) -> Result<()> {
    let floor: i32 = sqlx::query_scalar(
        "SELECT minimum_api FROM photara.schema_metadata WHERE singleton FOR SHARE",
    )
    .fetch_one(&mut **tx)
    .await?;
    if floor != 3 {
        return Err(ServiceError::Unsupported);
    }
    Ok(())
}
async fn identity(tx: &mut Transaction, claims: &VerifiedClaims) -> Result<Option<Actor>> {
    let row=sqlx::query("SELECT identity_id,account_id FROM photara_identity.account_identities WHERE issuer=$1 AND subject=$2").bind(&claims.issuer).bind(&claims.subject).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let account: Uuid = row.try_get("account_id")?;
    let identity: Uuid = row.try_get("identity_id")?;
    let active: String = sqlx::query_scalar(
        "SELECT state FROM photara_identity.accounts WHERE account_id=$1 FOR UPDATE",
    )
    .bind(account)
    .fetch_one(&mut **tx)
    .await?;
    let state: String = sqlx::query_scalar(
        "SELECT state FROM photara_identity.account_identities WHERE identity_id=$1 FOR UPDATE",
    )
    .bind(identity)
    .fetch_one(&mut **tx)
    .await?;
    if active != "active" || state != "active" {
        return Err(ServiceError::Forbidden);
    }
    Ok(Some(Actor {
        account: AccountId::from_uuid(account).map_err(|_| ServiceError::Integrity)?,
        identity,
        expires_ms: claims.expires_ms,
    }))
}
async fn device(
    tx: &mut Transaction,
    actor: &Actor,
    id: DeviceId,
    commitment: &[u8],
    suspended: bool,
) -> Result<Option<(String, i64)>> {
    let state:Option<String>=sqlx::query_scalar("SELECT state FROM photara_identity.devices WHERE account_id=$1 AND device_id=$2 FOR UPDATE").bind(actor.account.uuid()).bind(id.uuid()).fetch_optional(&mut **tx).await?;
    let Some(state) = state else {
        return Ok(None);
    };
    if state != "active" {
        return Err(ServiceError::Forbidden);
    }
    let row=sqlx::query("SELECT commitment,state,revision FROM photara_identity.device_credentials WHERE account_id=$1 AND device_id=$2 FOR UPDATE").bind(actor.account.uuid()).bind(id.uuid()).fetch_optional(&mut **tx).await?.ok_or(ServiceError::Forbidden)?;
    let state: String = row.try_get("state")?;
    if !crate::secret_eq(&row.try_get::<Vec<u8>, _>("commitment")?, commitment)
        || (!suspended && state != "active")
    {
        return Err(ServiceError::Forbidden);
    }
    Ok(Some((state, row.try_get("revision")?)))
}
async fn default_library(tx: &mut Transaction, actor: &Actor) -> Result<Option<LibraryId>> {
    let library: Option<Uuid> = sqlx::query_scalar(
        "SELECT library_id FROM photara_identity.account_defaults WHERE account_id=$1 FOR UPDATE",
    )
    .bind(actor.account.uuid())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(library) = library else {
        return Ok(None);
    };
    let library = LibraryId::from_uuid(library).map_err(|_| ServiceError::Integrity)?;
    coordinates(tx, actor, library).await?;
    Ok(Some(library))
}
async fn coordinates(tx: &mut Transaction, actor: &Actor, library: LibraryId) -> Result<PgRow> {
    sqlx::query("SELECT set_config('photara.library_id',$1,true)")
        .bind(library.to_string())
        .execute(&mut **tx)
        .await?;
    let row=sqlx::query("SELECT l.state,m.membership_id,m.role,m.state AS membership_state,m.revision AS membership_revision,c.contract_version,c.authorization_generation,s.stream_id,s.epoch AS stream_epoch,s.last_sequence AS high_water FROM photara.libraries l JOIN photara_identity.memberships m USING(library_id) JOIN photara.library_contract_state c USING(library_id) JOIN photara_private.scoped_streams s USING(library_id) WHERE l.library_id=$1 AND m.account_id=$2 AND s.scope_kind='library' FOR UPDATE OF l,m,c,s").bind(library.uuid()).bind(actor.account.uuid()).fetch_optional(&mut **tx).await?.ok_or(ServiceError::Forbidden)?;
    if row.try_get::<String, _>("state")? != "active"
        || row.try_get::<String, _>("membership_state")? != "active"
        || row.try_get::<String, _>("role")? != "owner"
    {
        return Err(ServiceError::Forbidden);
    }
    Ok(row)
}
async fn receipt(
    tx: &mut Transaction,
    claims: &VerifiedClaims,
    operation: OperationId,
) -> Result<Option<PgRow>> {
    Ok(sqlx::query("SELECT * FROM photara_private.onboarding_receipts WHERE issuer=$1 AND subject=$2 AND operation_id=$3").bind(&claims.issuer).bind(&claims.subject).bind(operation.uuid()).fetch_optional(&mut **tx).await?)
}
fn replay(
    row: PgRow,
    bytes: &[u8],
    device: DeviceId,
    commitment: &[u8],
    action: &str,
) -> Result<Vec<u8>> {
    if row.try_get::<Vec<u8>, _>("request_canonical")? != bytes
        || row.try_get::<Vec<u8>, _>("request_sha256")? != hash(bytes)
        || row.try_get::<Uuid, _>("device_id")? != device.uuid()
        || !crate::secret_eq(&row.try_get::<Vec<u8>, _>("device_commitment")?, commitment)
        || row.try_get::<String, _>("action")? != action
    {
        return Err(ServiceError::Conflict);
    }
    envelope(
        row.try_get("response_canonical")?,
        row.try_get("response_sha256")?,
    )
}
#[allow(clippy::too_many_arguments)]
async fn consume(
    tx: &mut Transaction,
    claims: &VerifiedClaims,
    operation: &OperationId,
    device: DeviceId,
    commitment: &[u8],
    bytes: &[u8],
    action: &str,
    proof: &Proof,
    now: i64,
) -> Result<()> {
    let affected=sqlx::query("UPDATE photara_private.onboarding_challenges SET consumed_at=to_timestamp($1::double precision/1000),consuming_issuer=$2,consuming_subject=$3 WHERE challenge_id=$4 AND nonce_sha256=$5 AND action=$6 AND operation_id=$7 AND request_sha256=$8 AND device_id=$9 AND device_commitment=$10 AND issued_at=to_timestamp($11::double precision/1000) AND consumed_at IS NULL AND expires_at>to_timestamp($1::double precision/1000)").bind(now).bind(&claims.issuer).bind(&claims.subject).bind(proof.challenge_id).bind(&proof.nonce_sha256).bind(action).bind(operation.uuid()).bind(hash(bytes).to_vec()).bind(device.uuid()).bind(commitment).bind(proof.issued_ms).execute(&mut **tx).await?.rows_affected();
    if affected != 1 {
        return Err(ServiceError::Forbidden);
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
async fn save(
    tx: &mut Transaction,
    claims: &VerifiedClaims,
    actor: &Actor,
    operation: OperationId,
    device: DeviceId,
    commitment: &[u8],
    action: &str,
    request: &[u8],
    response: &[u8],
) -> Result<()> {
    sqlx::query("INSERT INTO photara_private.onboarding_receipts VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,CURRENT_TIMESTAMP)").bind(&claims.issuer).bind(&claims.subject).bind(operation.uuid()).bind(actor.identity).bind(actor.account.uuid()).bind(device.uuid()).bind(commitment).bind(action).bind(request).bind(hash(request).to_vec()).bind(response).bind(hash(response).to_vec()).execute(&mut **tx).await?;
    Ok(())
}
