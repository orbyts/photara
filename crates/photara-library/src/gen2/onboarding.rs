//! Durable native onboarding boundary. No provider I/O, credentials or SQL escape.
use super::*;
use photara_core::contracts::LocalPrincipalId;
use serde::Deserialize;

const SCHEMA: &str = "photara.onboarding.v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentPrincipal {
    pub issuer: String,
    pub subject: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBootstrapCommand {
    pub schema: String,
    pub operation_id: Uuid,
    pub environment_id: String,
    pub device_id: DeviceId,
    pub device_credential_sha256: String,
    pub device_display_name: String,
    pub requested_library_id: LibraryId,
    pub library_display_name: String,
    pub intent: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeChallengeCommand {
    pub schema: String,
    pub action: String,
    pub operation_id: Uuid,
    pub request_sha256: String,
    pub device_id: DeviceId,
    pub device_credential_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBootstrapReceipt {
    pub schema: String,
    pub environment_id: String,
    pub operation_id: Uuid,
    pub request_sha256: String,
    pub issuer: String,
    pub subject: String,
    pub account_id: Uuid,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSessionObservation {
    pub schema: String,
    pub issuer: String,
    pub subject: String,
    pub account_id: Uuid,
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

/// All fields are public coordinates/commitments. Native keeps the actual device
/// credential, verifier, access/refresh tokens and ID proof outside `SQLite`.
#[derive(Clone)]
pub struct PrepareEnrollment {
    pub operation: Uuid,
    pub library: LibraryId,
    pub actor: LocalPrincipalId,
    pub environment_id: String,
    pub issuer: String,
    pub credential_reference: String,
    pub device_commitment: String,
    pub device_display_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnrollmentIntent {
    pub operation: Uuid,
    pub command: NativeBootstrapCommand,
    pub command_canonical: Vec<u8>,
    pub command_sha256: String,
    pub challenge_canonical: Vec<u8>,
    pub credential_reference: String,
    pub principal: Option<EnrollmentPrincipal>,
    pub state: String,
    pub replaces_operation: Option<Uuid>,
}

fn challenge(command: &NativeBootstrapCommand, request_sha256: &str) -> Result<Vec<u8>> {
    canonical(&NativeChallengeCommand {
        schema: SCHEMA.into(),
        action: "bootstrap".into(),
        operation_id: command.operation_id,
        request_sha256: request_sha256.into(),
        device_id: command.device_id,
        device_credential_sha256: command.device_credential_sha256.clone(),
    })
    .map(String::into_bytes)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum EnrollmentApplication {
    Applied { library: LibraryId },
    LibraryChoiceRequired { library: LibraryId },
    ReconciliationRequired,
}

#[derive(Clone, Debug, Serialize)]
pub struct CachedCloudLibrary {
    pub library: LibraryId,
    pub environment_id: String,
    pub principal: EnrollmentPrincipal,
    pub account_id: Uuid,
    pub state: String,
}

fn text(value: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn principal(value: &EnrollmentPrincipal) -> Result<()> {
    text(&value.subject, 255)?;
    let url = url::Url::parse(&value.issuer).map_err(|_| Error::Invalid)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.as_str() != value.issuer
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || value.bytes().all(|b| b == b'0')
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn decimal(value: &str, minimum: i64) -> Result<i64> {
    let number: i64 = value.parse().map_err(|_| Error::Invalid)?;
    if number < minimum || number.to_string() != value {
        return Err(Error::Invalid);
    }
    Ok(number)
}
fn strict<T: Serialize + serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value: T = photara_core::contracts::schema::decode_strict(bytes, 65_536)
        .map_err(|_| Error::Invalid)?;
    if canonical(&value)?.as_bytes() != bytes {
        return Err(Error::Invalid);
    }
    Ok(value)
}

async fn session(
    conn: &mut SqliteConnection,
    generation: i64,
    expected: &EnrollmentPrincipal,
) -> Result<()> {
    let matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM onboarding_session WHERE singleton=1 AND generation=? AND issuer=? AND subject=?)")
        .bind(generation).bind(&expected.issuer).bind(&expected.subject).fetch_one(conn).await?;
    if !matches {
        return Err(Error::Conflict);
    }
    Ok(())
}

async fn empty(conn: &mut SqliteConnection, library: LibraryId) -> Result<()> {
    // Retired content is still content. Parent entities cover dependent records;
    // retained recovery/intents are conservatively refused even if completed.
    for table in [
        "people",
        "organizations",
        "social_profiles",
        "location_kinds",
        "locations",
        "storage_roots",
        "project_catalog",
        "project_creation_intents",
        "project_ownership",
        "library_media",
        "library_variables",
        "library_expressions",
        "context_apply_intents",
        "operation_intents",
        "storage_slots",
        "scoped_sync_channels",
        "sync_targets",
    ] {
        let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE library_id=?)");
        // SQL identifiers come exclusively from the literal whitelist above.
        if sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(library.bytes())
            .fetch_one(&mut *conn)
            .await?
        {
            return Err(Error::Constraint);
        }
    }
    // A rename can leave local Library history. It is carried in the immutable
    // command; other local mutations or any prior content transport are refused.
    let populated: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM mutations WHERE library_id=? AND (primary_entity_kind<>'library' OR origin<>'local'))")
        .bind(library.bytes()).fetch_one(conn).await?;
    if populated {
        return Err(Error::Constraint);
    }
    Ok(())
}

async fn intent(
    conn: &mut SqliteConnection,
    operation: Uuid,
) -> Result<(EnrollmentIntent, sqlx::sqlite::SqliteRow)> {
    let row = sqlx::query("SELECT * FROM onboarding_intents WHERE operation_id=?")
        .bind(operation.as_bytes().to_vec())
        .fetch_one(&mut *conn)
        .await?;
    let bytes: Vec<u8> = row.try_get("command_canonical")?;
    let checksum: Vec<u8> = row.try_get("command_sha256")?;
    if sha(&bytes).as_slice() != checksum {
        return Err(Error::Corrupt);
    }
    let command: NativeBootstrapCommand = strict(&bytes)?;
    if command.operation_id != operation
        || command.schema != SCHEMA
        || command.intent != "enroll-default"
        || command.device_id.bytes() != row.try_get::<Vec<u8>, _>("device_id")?
        || command.requested_library_id.bytes() != row.try_get::<Vec<u8>, _>("library_id")?
        || command.environment_id != row.try_get::<String, _>("environment_id")?
        || command.device_credential_sha256 != row.try_get::<String, _>("device_commitment")?
    {
        return Err(Error::Corrupt);
    }
    let issuer: Option<String> = row.try_get("issuer")?;
    let subject: Option<String> = row.try_get("subject")?;
    let principal = match (issuer, subject) {
        (Some(issuer), Some(subject)) => Some(EnrollmentPrincipal { issuer, subject }),
        (None, None) => None,
        _ => return Err(Error::Corrupt),
    };
    let command_sha256 = hex(&checksum);
    let challenge_canonical = challenge(&command, &command_sha256)?;
    let disposition: Option<String> =
        sqlx::query_scalar("SELECT disposition FROM onboarding_dispositions WHERE operation_id=?")
            .bind(operation.as_bytes().to_vec())
            .fetch_optional(&mut *conn)
            .await?;
    let previous: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT previous_operation_id FROM onboarding_replacements WHERE operation_id=?",
    )
    .bind(operation.as_bytes().to_vec())
    .fetch_optional(&mut *conn)
    .await?;
    Ok((
        EnrollmentIntent {
            operation,
            command,
            command_canonical: bytes,
            command_sha256,
            challenge_canonical,
            credential_reference: row.try_get("credential_reference")?,
            principal,
            state: disposition.unwrap_or(row.try_get("state")?),
            replaces_operation: previous
                .map(|bytes| Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt))
                .transpose()?,
        },
        row,
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationAbsence {
    schema: String,
    code: String,
    retryable: bool,
    outcome_known: bool,
    request_id: Uuid,
}
fn check_absence(bytes: &[u8]) -> Result<()> {
    let absence: OperationAbsence = strict(bytes)?;
    if absence.schema != SCHEMA
        || absence.code != "operation-not-yet-found"
        || !absence.retryable
        || absence.outcome_known
        || absence.request_id.is_nil()
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
async fn expired_attempt(
    conn: &mut SqliteConnection,
    library: LibraryId,
    environment: &str,
) -> Result<Option<Uuid>> {
    let operation: Option<Vec<u8>> = sqlx::query_scalar("SELECT i.operation_id FROM onboarding_intents i JOIN onboarding_dispositions d USING(operation_id) WHERE i.library_id=? AND i.environment_id=? ORDER BY d.observed_at DESC,i.created_at DESC,i.operation_id DESC LIMIT 1")
        .bind(library.bytes()).bind(environment).fetch_optional(conn).await?;
    operation
        .map(|bytes| Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt))
        .transpose()
}

impl LocalLibraryStore {
    /// Opaque locator for a previously bound library, including signed-out caches.
    /// # Errors
    /// Rejects corrupt binding/receipt evidence; never returns a principal.
    pub async fn cached_enrollment_credential(
        &self,
        library: LibraryId,
        environment: &str,
    ) -> Result<Option<String>> {
        let mut tx = self.read().await?;
        let result = bound_intent(&mut tx, library, environment)
            .await?
            .map(|i| i.credential_reference);
        tx.commit().await?;
        Ok(result)
    }

    /// Resume the original binding after native verification of a fresh login.
    /// # Errors
    /// Rejects a different principal/environment or damaged binding evidence.
    pub async fn bound_enrollment_intent(
        &self,
        library: LibraryId,
        environment: &str,
        expected: &EnrollmentPrincipal,
    ) -> Result<EnrollmentIntent> {
        principal(expected)?;
        let mut tx = self.read().await?;
        let result = bound_intent(&mut tx, library, environment)
            .await?
            .ok_or(Error::Constraint)?;
        if result.principal.as_ref() != Some(expected) {
            return Err(Error::Constraint);
        }
        tx.commit().await?;
        Ok(result)
    }

    /// Local presentation only; this never establishes current service access.
    /// # Errors
    /// Rejects corrupt binding evidence without disclosing the cached identity.
    pub async fn saved_enrollment_state(
        &self,
        library: LibraryId,
        environment: &str,
    ) -> Result<Option<String>> {
        let mut tx = self.read().await?;
        let binding = opening_binding(&mut tx, library, environment).await?;
        tx.commit().await?;
        Ok(binding
            .filter(|b| b.environment_id == environment)
            .map(|b| b.state))
    }

    /// A terminal disposition preserves original intent/credential/receipt history.
    /// The native caller supplies a just-authenticated exact operation 404 and
    /// confirms its original enrollment proof cannot be submitted anymore.
    /// # Errors
    /// Rejects stale sessions, malformed/non-404 evidence, available proof,
    /// existing receipts and every state other than an unresolved bound attempt.
    pub async fn abandon_expired_enrollment(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        generation: i64,
        absence: &[u8],
        proof_unavailable: bool,
        at: Timestamp,
    ) -> Result<()> {
        principal(expected)?;
        check_absence(absence)?;
        if !proof_unavailable {
            return Err(Error::Constraint);
        }
        let mut tx = self.write().await?;
        session(&mut tx, generation, expected).await?;
        let (stored, _) = intent(&mut tx, operation).await?;
        if stored.principal.as_ref() != Some(expected) {
            return Err(Error::Constraint);
        }
        if stored.state == "abandoned-expired" {
            tx.commit().await?;
            return Ok(());
        }
        if stored.state != "unknown" {
            return Err(Error::Constraint);
        }
        sqlx::query("INSERT INTO onboarding_dispositions VALUES(?,'abandoned-expired',?,?,?,?)")
            .bind(operation.as_bytes().to_vec())
            .bind(absence)
            .bind(sha(absence).to_vec())
            .bind(generation)
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Only an opaque locator is exposed before fresh identity verification.
    /// # Errors
    /// Refuses damaged terminal history; returns no candidate once cloud-bound.
    pub async fn expired_enrollment_credential(
        &self,
        library: LibraryId,
        environment: &str,
    ) -> Result<Option<String>> {
        let mut tx = self.read().await?;
        let local: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM library_contract_state WHERE library_id=? AND authority_mode='local-only')")
            .bind(library.bytes()).fetch_one(&mut *tx).await?;
        let result = if local {
            if let Some(operation) = expired_attempt(&mut tx, library, environment).await? {
                Some(intent(&mut tx, operation).await?.0.credential_reference)
            } else {
                None
            }
        } else {
            None
        };
        tx.commit().await?;
        Ok(result)
    }

    /// Advance the durable cancellation/account-switch generation. No receipt or
    /// credential reference is removed. Native invokes this before cancellation.
    /// # Errors
    /// Rejects malformed identity, generation overflow or storage failure.
    pub async fn advance_enrollment_session(
        &self,
        next: Option<&EnrollmentPrincipal>,
    ) -> Result<i64> {
        if let Some(next) = next {
            principal(next)?;
        }
        let mut tx = self.write().await?;
        let generation: i64 =
            sqlx::query_scalar("SELECT generation FROM onboarding_session WHERE singleton=1")
                .fetch_one(&mut *tx)
                .await?;
        let generation = generation.checked_add(1).ok_or(Error::Limit)?;
        sqlx::query(
            "UPDATE onboarding_session SET generation=?,issuer=?,subject=? WHERE singleton=1",
        )
        .bind(generation)
        .bind(next.map(|p| &p.issuer))
        .bind(next.map(|p| &p.subject))
        .execute(&mut *tx)
        .await?;
        if next.is_none() {
            sqlx::query("UPDATE library_cloud_bindings SET state='signed-out',local_revision=local_revision+1 WHERE state='cached'").execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(generation)
    }

    /// Commit the exact empty-Library intent before any challenge/browser call.
    /// # Errors
    /// Rejects populated content, existing association, identity or request drift.
    pub async fn prepare_enrollment(
        &self,
        input: &PrepareEnrollment,
        at: Timestamp,
    ) -> Result<EnrollmentIntent> {
        if input.operation.is_nil() {
            return Err(Error::Invalid);
        }
        text(&input.environment_id, 255)?;
        text(&input.device_display_name, 512)?;
        let reference = input
            .credential_reference
            .strip_prefix("keychain:")
            .ok_or(Error::Invalid)?;
        if Uuid::parse_str(reference)
            .map_err(|_| Error::Invalid)?
            .is_nil()
        {
            return Err(Error::Invalid);
        }
        principal(&EnrollmentPrincipal {
            issuer: input.issuer.clone(),
            subject: "preflight".into(),
        })?;
        digest(&input.device_commitment)?;
        let mut tx = self.write().await?;
        let row = sqlx::query("SELECT l.display_name,l.local_revision,c.local_revision AS contract_revision,c.local_principal_id FROM libraries l JOIN library_contract_state c USING(library_id) WHERE l.library_id=? AND l.state='active' AND c.authority_mode='local-only'")
            .bind(input.library.bytes()).fetch_one(&mut *tx).await?;
        if row.try_get::<Vec<u8>, _>("local_principal_id")? != input.actor.uuid().as_bytes() {
            return Err(Error::Constraint);
        }
        let command = NativeBootstrapCommand {
            schema: SCHEMA.into(),
            operation_id: input.operation,
            environment_id: input.environment_id.clone(),
            device_id: self.info.device_id,
            device_credential_sha256: input.device_commitment.clone(),
            device_display_name: input.device_display_name.clone(),
            requested_library_id: input.library,
            library_display_name: row.try_get("display_name")?,
            intent: "enroll-default".into(),
        };
        text(&command.library_display_name, 512)?;
        let bytes = canonical(&command)?.into_bytes();
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM onboarding_intents WHERE operation_id=?)",
        )
        .bind(input.operation.as_bytes().to_vec())
        .fetch_one(&mut *tx)
        .await?;
        if exists {
            let (existing, stored) = intent(&mut tx, input.operation).await?;
            if existing.command_canonical != bytes
                || existing.credential_reference != input.credential_reference
                || stored.try_get::<String, _>("expected_issuer")? != input.issuer
            {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(existing);
        }
        empty(&mut tx, input.library).await?;
        let previous = expired_attempt(&mut tx, input.library, &input.environment_id).await?;
        if let Some(previous) = previous {
            let (old, old_row) = intent(&mut tx, previous).await?;
            if old.credential_reference != input.credential_reference
                || old.command.device_credential_sha256 != input.device_commitment
                || old_row.try_get::<String, _>("expected_issuer")? != input.issuer
            {
                return Err(Error::Conflict);
            }
        }
        sqlx::query("INSERT INTO onboarding_intents VALUES(?,?,?,?,?,?,NULL,NULL,?,?,?,?,?,?,?,'prepared',?)")
            .bind(input.operation.as_bytes().to_vec()).bind(self.info.database_id.bytes()).bind(self.info.device_id.bytes())
            .bind(input.library.bytes()).bind(&input.environment_id).bind(&input.issuer).bind(input.actor.uuid().as_bytes().to_vec())
            .bind(row.try_get::<i64, _>("local_revision")?).bind(row.try_get::<i64, _>("contract_revision")?)
            .bind(&bytes).bind(sha(&bytes).to_vec()).bind(&input.credential_reference).bind(&input.device_commitment).bind(at.get())
            .execute(&mut *tx).await?;
        if let Some(previous) = previous {
            sqlx::query("INSERT INTO onboarding_replacements VALUES(?,?)")
                .bind(input.operation.as_bytes().to_vec())
                .bind(previous.as_bytes().to_vec())
                .execute(&mut *tx)
                .await?;
        }
        let (result, _) = intent(&mut tx, input.operation).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Bind an intent once to a natively verified exact principal in the current
    /// session, then mark unknown durably before network dispatch.
    /// # Errors
    /// Rejects stale generation, principal/credential drift and applied intents.
    pub async fn dispatch_enrollment(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        generation: i64,
    ) -> Result<EnrollmentIntent> {
        principal(expected)?;
        let mut tx = self.write().await?;
        session(&mut tx, generation, expected).await?;
        let (stored, row) = intent(&mut tx, operation).await?;
        if row.try_get::<String, _>("expected_issuer")? != expected.issuer
            || stored.principal.as_ref().is_some_and(|p| p != expected)
            || !["prepared", "authenticated", "unknown"].contains(&stored.state.as_str())
        {
            return Err(Error::Conflict);
        }
        if stored.state != "unknown" {
            empty(&mut tx, stored.command.requested_library_id).await?;
            let unchanged: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM libraries l JOIN library_contract_state c USING(library_id) WHERE library_id=? AND l.state='active' AND c.authority_mode='local-only' AND l.local_revision=? AND c.local_revision=? AND c.local_principal_id=?)")
                .bind(stored.command.requested_library_id.bytes()).bind(row.try_get::<i64, _>("library_revision")?)
                .bind(row.try_get::<i64, _>("contract_revision")?).bind(row.try_get::<Vec<u8>, _>("local_principal_id")?).fetch_one(&mut *tx).await?;
            if !unchanged {
                return Err(Error::Conflict);
            }
        }
        if let Some(previous) = stored.replaces_operation {
            let (old, _) = intent(&mut tx, previous).await?;
            if old.principal.as_ref() != Some(expected) {
                return Err(Error::Constraint);
            }
        }
        sqlx::query("INSERT INTO onboarding_credential_references VALUES(?,?,?,?,?,?) ON CONFLICT DO NOTHING")
            .bind(&stored.command.environment_id).bind(&expected.issuer).bind(&expected.subject).bind(self.info.device_id.bytes())
            .bind(&stored.credential_reference).bind(&stored.command.device_credential_sha256).execute(&mut *tx).await?;
        let matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM onboarding_credential_references WHERE environment_id=? AND issuer=? AND subject=? AND device_id=? AND credential_reference=? AND device_commitment=?)")
            .bind(&stored.command.environment_id).bind(&expected.issuer).bind(&expected.subject).bind(self.info.device_id.bytes())
            .bind(&stored.credential_reference).bind(&stored.command.device_credential_sha256).fetch_one(&mut *tx).await?;
        if !matches {
            return Err(Error::Conflict);
        }
        sqlx::query(
            "UPDATE onboarding_intents SET issuer=?,subject=?,state='unknown' WHERE operation_id=?",
        )
        .bind(&expected.issuer)
        .bind(&expected.subject)
        .bind(operation.as_bytes().to_vec())
        .execute(&mut *tx)
        .await?;
        let (result, _) = intent(&mut tx, operation).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Read only this principal's journal. Signing out cannot erase unknown work.
    /// # Errors
    /// Rejects another principal or a corrupt stored command.
    pub async fn enrollment_intent(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
    ) -> Result<EnrollmentIntent> {
        let mut tx = self.read().await?;
        let (stored, _) = intent(&mut tx, operation).await?;
        if stored.principal.as_ref() != Some(expected) {
            return Err(Error::Constraint);
        }
        tx.commit().await?;
        Ok(stored)
    }

    /// Discover a retained operation without regenerating its canonical bytes.
    /// Bound journals require the same verified principal; unbound ones do not.
    /// # Errors
    /// Rejects cross-account access or corrupt stored data.
    pub async fn pending_enrollment(
        &self,
        library: LibraryId,
        environment: &str,
        expected: Option<&EnrollmentPrincipal>,
    ) -> Result<Option<EnrollmentIntent>> {
        let mut tx = self.read().await?;
        let operation: Option<Vec<u8>> = sqlx::query_scalar("SELECT operation_id FROM onboarding_intents i WHERE library_id=? AND environment_id=? AND state NOT IN ('applied','cancelled') AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions d WHERE d.operation_id=i.operation_id)")
            .bind(library.bytes()).bind(environment).fetch_optional(&mut *tx).await?;
        let result = if let Some(operation) = operation {
            let operation = Uuid::from_slice(&operation).map_err(|_| Error::Corrupt)?;
            let (stored, _) = intent(&mut tx, operation).await?;
            if stored.principal.is_some() && stored.principal.as_ref() != expected {
                return Err(Error::Constraint);
            }
            Some(stored)
        } else {
            None
        };
        tx.commit().await?;
        Ok(result)
    }

    /// Return only the opaque credential locator for a bound unresolved attempt.
    /// It grants no journal access or authority: callers must independently
    /// verify the recovered session before calling `pending_enrollment`.
    /// # Errors
    /// Rejects corrupt retained evidence.
    pub async fn pending_enrollment_credential(
        &self,
        library: LibraryId,
        environment: &str,
    ) -> Result<Option<String>> {
        let mut tx = self.read().await?;
        let operation: Option<Vec<u8>> = sqlx::query_scalar("SELECT operation_id FROM onboarding_intents i WHERE library_id=? AND environment_id=? AND issuer IS NOT NULL AND state NOT IN ('applied','cancelled') AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions d WHERE d.operation_id=i.operation_id)")
            .bind(library.bytes()).bind(environment).fetch_optional(&mut *tx).await?;
        let reference = if let Some(operation) = operation {
            let (stored, _) = intent(
                &mut tx,
                Uuid::from_slice(&operation).map_err(|_| Error::Corrupt)?,
            )
            .await?;
            Some(stored.credential_reference)
        } else {
            None
        };
        tx.commit().await?;
        Ok(reference)
    }

    /// Retire only an unsubmitted browser attempt. An unknown cloud operation
    /// cannot be cancelled away or lose its immutable receipt/replay evidence.
    /// # Errors
    /// Rejects submitted operations and stale local cancellation requests.
    pub async fn cancel_prepared_enrollment(&self, operation: Uuid) -> Result<()> {
        let mut tx = self.write().await?;
        let changed = sqlx::query("UPDATE onboarding_intents SET state='cancelled' WHERE operation_id=? AND state='prepared' AND issuer IS NULL")
            .bind(operation.as_bytes().to_vec()).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Verify the canonical envelope and exact request binding before independently
    /// persisting a receipt. Late completion is retained under its original user.
    /// # Errors
    /// Rejects changed bytes/digests, unknown fields, identities and coordinates.
    pub async fn record_enrollment_receipt(
        &self,
        operation: Uuid,
        envelope: &[u8],
        at: Timestamp,
    ) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Envelope {
            receipt: NativeBootstrapReceipt,
            receipt_sha256: String,
        }
        let envelope: Envelope = strict(envelope)?;
        let receipt = envelope.receipt;
        let bytes = canonical(&receipt)?.into_bytes();
        if hex(&sha(&bytes)) != envelope.receipt_sha256 {
            return Err(Error::Invalid);
        }
        let mut tx = self.write().await?;
        let (stored, _) = intent(&mut tx, operation).await?;
        check_receipt(&receipt, &stored)?;
        if let Some(prior) = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT receipt_canonical FROM onboarding_receipts WHERE operation_id=?",
        )
        .bind(operation.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await?
        {
            if prior != bytes {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(());
        }
        if !["unknown", "abandoned-expired"].contains(&stored.state.as_str()) {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO onboarding_receipts VALUES(?,?,?,?)")
            .bind(operation.as_bytes().to_vec())
            .bind(&bytes)
            .bind(sha(&bytes).to_vec())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
        if stored.state != "abandoned-expired" {
            sqlx::query("UPDATE onboarding_intents SET state='receipt-ready' WHERE operation_id=?")
                .bind(operation.as_bytes().to_vec())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

fn check_receipt(receipt: &NativeBootstrapReceipt, stored: &EnrollmentIntent) -> Result<()> {
    let expected = stored.principal.as_ref().ok_or(Error::Constraint)?;
    if receipt.schema != SCHEMA
        || receipt.operation_id != stored.operation
        || receipt.environment_id != stored.command.environment_id
        || receipt.issuer != expected.issuer
        || receipt.subject != expected.subject
        || receipt.request_sha256 != stored.command_sha256
        || receipt.device_id != stored.command.device_id
        || receipt.requested_library_id != stored.command.requested_library_id
        || receipt.device_credential_sha256 != stored.command.device_credential_sha256
        || [
            receipt.account_id,
            receipt.identity_id,
            receipt.membership_id,
            receipt.stream_id,
            receipt.stream_epoch,
        ]
        .iter()
        .any(Uuid::is_nil)
        || receipt.contract_version != "1"
    {
        return Err(Error::Invalid);
    }
    decimal(&receipt.authorization_generation, 1)?;
    decimal(&receipt.initial_high_water, 0)?;
    decimal(&receipt.completed_at, 0)?;
    let same = receipt.requested_library_id == receipt.default_library_id;
    if (same && !["claimed-default", "recovered-default"].contains(&receipt.outcome.as_str()))
        || (!same && receipt.outcome != "existing-default-requires-local-choice")
    {
        return Err(Error::Invalid);
    }
    Ok(())
}

fn check_access(
    access: &NativeSessionObservation,
    receipt: &NativeBootstrapReceipt,
    at: Timestamp,
) -> Result<()> {
    let observed = decimal(&access.observed_at, 0)?;
    if access.schema != SCHEMA
        || access.issuer != receipt.issuer
        || access.subject != receipt.subject
        || access.account_id != receipt.account_id
        || access.identity_id != receipt.identity_id
        || access.device_id != receipt.device_id
        || access.default_library_id != receipt.default_library_id
        || access.membership_id != receipt.membership_id
        || access.membership_role != "owner"
        || [
            &access.account_state,
            &access.identity_state,
            &access.device_state,
            &access.credential_state,
        ]
        .iter()
        .any(|s| s.as_str() != "active")
        || observed < at.get().saturating_sub(60_000)
        || observed > at.get().saturating_add(60_000)
    {
        return Err(Error::Constraint);
    }
    decimal(&access.credential_revision, 1)?;
    decimal(&access.membership_revision, 1)?;
    decimal(&access.authorization_generation, 1)?;
    if decimal(&access.authorization_generation, 1)?
        < decimal(&receipt.authorization_generation, 1)?
    {
        return Err(Error::Constraint);
    }
    Ok(())
}

impl LocalLibraryStore {
    /// Apply the receipt to its original local ID. A different cloud default is
    /// only reported; it cannot replace or rekey an independent local Library.
    /// # Errors
    /// Rejects stale sessions, stale/disabled access and malformed current DTOs.
    pub async fn apply_enrollment(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        generation: i64,
        access_canonical: &[u8],
        at: Timestamp,
    ) -> Result<EnrollmentApplication> {
        self.reconcile_enrollment(operation, expected, generation, access_canonical, false, at)
            .await
    }

    /// Read-only library choice from the current supported default-library
    /// session contract. Validate identity, receipt and active access before UI.
    /// # Errors
    /// Rejects mismatched identities, corrupt evidence and stale/disabled access.
    pub async fn enrollment_library_choice(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        access_bytes: &[u8],
        at: Timestamp,
    ) -> Result<LibraryId> {
        principal(expected)?;
        let access: NativeSessionObservation = strict(access_bytes)?;
        let mut tx = self.read().await?;
        let (stored, _) = intent(&mut tx, operation).await?;
        if stored.principal.as_ref() != Some(expected) || stored.state == "abandoned-expired" {
            return Err(Error::Constraint);
        }
        let saved = sqlx::query(
            "SELECT receipt_canonical,receipt_sha256 FROM onboarding_receipts WHERE operation_id=?",
        )
        .bind(operation.as_bytes().to_vec())
        .fetch_one(&mut *tx)
        .await?;
        let bytes: Vec<u8> = saved.try_get("receipt_canonical")?;
        if sha(&bytes).as_slice() != saved.try_get::<Vec<u8>, _>("receipt_sha256")? {
            return Err(Error::Corrupt);
        }
        let receipt: NativeBootstrapReceipt = strict(&bytes)?;
        check_receipt(&receipt, &stored)?;
        check_access(&access, &receipt, at)?;
        tx.commit().await?;
        Ok(receipt.default_library_id)
    }

    /// Explicit user choice to select the account's existing empty cloud Library.
    /// The original local Library remains intact and available for local recovery.
    /// # Errors
    /// Rejects collisions, stale access or unsupported nonempty cloud content.
    pub async fn open_existing_cloud_library(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        generation: i64,
        access_canonical: &[u8],
        at: Timestamp,
    ) -> Result<EnrollmentApplication> {
        self.reconcile_enrollment(operation, expected, generation, access_canonical, true, at)
            .await
    }

    // Keep the complete authority-changing transaction and its early exits together.
    #[allow(clippy::too_many_lines)]
    async fn reconcile_enrollment(
        &self,
        operation: Uuid,
        expected: &EnrollmentPrincipal,
        generation: i64,
        access_bytes: &[u8],
        explicit_choice: bool,
        at: Timestamp,
    ) -> Result<EnrollmentApplication> {
        principal(expected)?;
        let access: NativeSessionObservation = strict(access_bytes)?;
        let mut tx = self.write().await?;
        session(&mut tx, generation, expected).await?;
        let (stored, row) = intent(&mut tx, operation).await?;
        if stored.principal.as_ref() != Some(expected) || stored.state == "abandoned-expired" {
            return Err(Error::Constraint);
        }
        let saved = sqlx::query(
            "SELECT receipt_canonical,receipt_sha256 FROM onboarding_receipts WHERE operation_id=?",
        )
        .bind(operation.as_bytes().to_vec())
        .fetch_one(&mut *tx)
        .await?;
        let bytes: Vec<u8> = saved.try_get("receipt_canonical")?;
        if sha(&bytes).as_slice() != saved.try_get::<Vec<u8>, _>("receipt_sha256")? {
            return Err(Error::Corrupt);
        }
        let receipt: NativeBootstrapReceipt = strict(&bytes)?;
        check_receipt(&receipt, &stored)?;
        check_access(&access, &receipt, at)?;
        if receipt.initial_high_water != "0" {
            return Err(Error::Unsupported);
        }
        let same = receipt.requested_library_id == receipt.default_library_id;
        let library = receipt.default_library_id;
        if !same && !explicit_choice && stored.state != "applied" {
            if stored.state != "applied" {
                sqlx::query("UPDATE onboarding_intents SET state='library-choice-required' WHERE operation_id=?")
                    .bind(operation.as_bytes().to_vec()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            return Ok(EnrollmentApplication::LibraryChoiceRequired { library });
        }
        if let Some(binding) =
            sqlx::query("SELECT * FROM library_cloud_bindings WHERE library_id=?")
                .bind(library.bytes())
                .fetch_optional(&mut *tx)
                .await?
        {
            cached_binding(&mut tx, library)
                .await?
                .ok_or(Error::Corrupt)?;
            let prior: NativeSessionObservation =
                strict(&binding.try_get::<Vec<u8>, _>("access_canonical")?)?;
            for (new, old) in [
                (&access.observed_at, &prior.observed_at),
                (&access.credential_revision, &prior.credential_revision),
                (&access.membership_revision, &prior.membership_revision),
                (
                    &access.authorization_generation,
                    &prior.authorization_generation,
                ),
            ] {
                if decimal(new, 0)? < decimal(old, 0)? {
                    return Err(Error::Conflict);
                }
            }
            if binding.try_get::<String, _>("environment_id")? != receipt.environment_id
                || binding.try_get::<String, _>("issuer")? != expected.issuer
                || binding.try_get::<String, _>("subject")? != expected.subject
                || binding.try_get::<Vec<u8>, _>("account_id")? != receipt.account_id.as_bytes()
                || binding.try_get::<Vec<u8>, _>("identity_id")? != receipt.identity_id.as_bytes()
                || binding.try_get::<String, _>("state")? == "access-disabled"
            {
                return Err(Error::Constraint);
            }
            sqlx::query("UPDATE library_cloud_bindings SET access_canonical=?,access_sha256=?,observed_at=?,state='cached',local_revision=local_revision+1 WHERE library_id=?")
                .bind(access_bytes).bind(sha(access_bytes).to_vec()).bind(at.get()).bind(library.bytes()).execute(&mut *tx).await?;
        } else {
            if same {
                let facts = sqlx::query("SELECT l.local_revision,c.local_revision AS contract_revision,c.local_principal_id,c.authority_mode,l.state FROM libraries l JOIN library_contract_state c USING(library_id) WHERE library_id=?")
                    .bind(library.bytes()).fetch_one(&mut *tx).await?;
                let eligible = facts.try_get::<i64, _>("local_revision")?
                    == row.try_get::<i64, _>("library_revision")?
                    && facts.try_get::<i64, _>("contract_revision")?
                        == row.try_get::<i64, _>("contract_revision")?
                    && facts.try_get::<Option<Vec<u8>>, _>("local_principal_id")?
                        == Some(row.try_get("local_principal_id")?)
                    && facts.try_get::<String, _>("authority_mode")? == "local-only"
                    && facts.try_get::<String, _>("state")? == "active";
                let content = empty(&mut tx, library).await;
                if let Err(error) = content
                    && error != Error::Constraint
                {
                    return Err(error);
                }
                if !eligible || content.is_err() {
                    sqlx::query("UPDATE onboarding_intents SET state='reconciliation-required' WHERE operation_id=?")
                        .bind(operation.as_bytes().to_vec()).execute(&mut *tx).await?;
                    tx.commit().await?;
                    return Ok(EnrollmentApplication::ReconciliationRequired);
                }
                sqlx::query("UPDATE library_contract_state SET authority_mode='cloud-member',local_principal_id=NULL,authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=?")
                    .bind(at.get()).bind(library.bytes()).execute(&mut *tx).await?;
                sqlx::query("UPDATE libraries SET local_revision=local_revision+1,updated_at_ms=? WHERE library_id=?")
                    .bind(at.get()).bind(library.bytes()).execute(&mut *tx).await?;
            } else {
                let collision: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM libraries WHERE library_id=?)")
                        .bind(library.bytes())
                        .fetch_one(&mut *tx)
                        .await?;
                if collision {
                    return Err(Error::Conflict);
                }
                // This is an empty cache label, not an asserted remote rename.
                // The server's library-name/content snapshot contract is later.
                sqlx::query(
                    "INSERT INTO libraries VALUES(?,'Cloud Library','active',1,?,?,NULL,1,'{}')",
                )
                .bind(library.bytes())
                .bind(at.get())
                .bind(at.get())
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO library_contract_state VALUES(?,1,'cloud-member',NULL,1,1,1,?,?)",
                )
                .bind(library.bytes())
                .bind(at.get())
                .bind(at.get())
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query(
                "INSERT INTO library_cloud_bindings VALUES(?,?,?,?,?,?,?,?,?,?,1,'cached',?)",
            )
            .bind(library.bytes())
            .bind(operation.as_bytes().to_vec())
            .bind(&receipt.environment_id)
            .bind(&expected.issuer)
            .bind(&expected.subject)
            .bind(receipt.account_id.as_bytes().to_vec())
            .bind(receipt.identity_id.as_bytes().to_vec())
            .bind(access_bytes)
            .bind(sha(access_bytes).to_vec())
            .bind(at.get())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE onboarding_intents SET state='applied' WHERE operation_id=?")
            .bind(operation.as_bytes().to_vec())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO onboarding_library_selection VALUES(1,?,?,?) ON CONFLICT(singleton) DO UPDATE SET library_id=excluded.library_id,operation_id=excluded.operation_id,chosen_at=excluded.chosen_at")
            .bind(library.bytes()).bind(operation.as_bytes().to_vec()).bind(at.get()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(EnrollmentApplication::Applied { library })
    }

    /// Cached coordinates are for offline presentation, never server authority.
    /// # Errors
    /// Rejects damaged bindings, receipt bytes or cross-principal observations.
    pub async fn cached_cloud_library(
        &self,
        library: LibraryId,
    ) -> Result<Option<CachedCloudLibrary>> {
        let mut tx = self.read().await?;
        let result = cached_binding(&mut tx, library).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Returning-account selection is scoped to the exact principal. An account
    /// switch cannot select the previous account's cached cloud destination.
    /// # Errors
    /// Returns corruption or storage failure.
    pub async fn selected_cloud_library(
        &self,
        expected: &EnrollmentPrincipal,
    ) -> Result<Option<CachedCloudLibrary>> {
        principal(expected)?;
        let mut tx = self.read().await?;
        let library: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT library_id FROM onboarding_library_selection WHERE singleton=1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let result = if let Some(library) = library {
            cached_binding(&mut tx, LibraryId::from_bytes(&library)?).await?
        } else {
            None
        };
        tx.commit().await?;
        Ok(result.filter(|binding| binding.principal == *expected))
    }
}

pub(super) async fn cached_binding(
    conn: &mut SqliteConnection,
    library: LibraryId,
) -> Result<Option<CachedCloudLibrary>> {
    let Some(row) = sqlx::query("SELECT * FROM library_cloud_bindings WHERE library_id=?")
        .bind(library.bytes())
        .fetch_optional(&mut *conn)
        .await?
    else {
        return Ok(None);
    };
    let bytes: Vec<u8> = row.try_get("access_canonical")?;
    if sha(&bytes).as_slice() != row.try_get::<Vec<u8>, _>("access_sha256")? {
        return Err(Error::Corrupt);
    }
    let access: NativeSessionObservation = strict(&bytes)?;
    let expected = EnrollmentPrincipal {
        issuer: row.try_get("issuer")?,
        subject: row.try_get("subject")?,
    };
    let account_id =
        Uuid::from_slice(&row.try_get::<Vec<u8>, _>("account_id")?).map_err(|_| Error::Corrupt)?;
    if access.default_library_id != library
        || access.issuer != expected.issuer
        || access.subject != expected.subject
        || access.account_id != account_id
    {
        return Err(Error::Corrupt);
    }
    let operation = Uuid::from_slice(&row.try_get::<Vec<u8>, _>("operation_id")?)
        .map_err(|_| Error::Corrupt)?;
    let (stored, _) = intent(conn, operation).await?;
    let saved = sqlx::query(
        "SELECT receipt_canonical,receipt_sha256 FROM onboarding_receipts WHERE operation_id=?",
    )
    .bind(operation.as_bytes().to_vec())
    .fetch_one(&mut *conn)
    .await?;
    let receipt_bytes: Vec<u8> = saved.try_get("receipt_canonical")?;
    if sha(&receipt_bytes).as_slice() != saved.try_get::<Vec<u8>, _>("receipt_sha256")? {
        return Err(Error::Corrupt);
    }
    let receipt: NativeBootstrapReceipt = strict(&receipt_bytes)?;
    check_receipt(&receipt, &stored)?;
    if receipt.default_library_id != library
        || receipt.environment_id != row.try_get::<String, _>("environment_id")?
        || receipt.account_id != account_id
        || access.identity_id != receipt.identity_id
        || row.try_get::<Vec<u8>, _>("identity_id")? != receipt.identity_id.as_bytes()
        || access.device_id != receipt.device_id
        || access.membership_id != receipt.membership_id
        || stored.state != "applied"
    {
        return Err(Error::Corrupt);
    }
    Ok(Some(CachedCloudLibrary {
        library,
        environment_id: row.try_get("environment_id")?,
        principal: expected,
        account_id,
        state: row.try_get("state")?,
    }))
}

async fn bound_intent(
    conn: &mut SqliteConnection,
    library: LibraryId,
    environment: &str,
) -> Result<Option<EnrollmentIntent>> {
    let Some(binding) = opening_binding(conn, library, environment).await? else {
        return Ok(None);
    };
    if binding.environment_id != environment {
        return Ok(None);
    }
    let bytes: Vec<u8> =
        sqlx::query_scalar("SELECT operation_id FROM library_cloud_bindings WHERE library_id=?")
            .bind(binding.library.bytes())
            .fetch_one(&mut *conn)
            .await?;
    let operation = Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt)?;
    Ok(Some(intent(conn, operation).await?.0))
}

// A selection of an existing cloud default leaves the original independent
// local Library intact. Resolve that explicit selection only for its origin.
async fn opening_binding(
    conn: &mut SqliteConnection,
    origin: LibraryId,
    environment: &str,
) -> Result<Option<CachedCloudLibrary>> {
    if let Some(binding) = cached_binding(conn, origin).await? {
        return Ok((binding.environment_id == environment).then_some(binding));
    }
    let selected: Option<Vec<u8>> = sqlx::query_scalar("SELECT s.library_id FROM onboarding_library_selection s JOIN onboarding_intents i USING(operation_id) WHERE i.library_id=? AND i.environment_id=?")
        .bind(origin.bytes()).bind(environment).fetch_optional(&mut *conn).await?;
    let Some(bytes) = selected else {
        return Ok(None);
    };
    let library = LibraryId::try_from(Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt)?)?;
    let binding = cached_binding(conn, library).await?.ok_or(Error::Corrupt)?;
    if binding.environment_id != environment {
        return Err(Error::Corrupt);
    }
    Ok(Some(binding))
}

pub(super) async fn verify_onboarding(conn: &mut SqliteConnection) -> Result<()> {
    let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM library_cloud_bindings b LEFT JOIN library_contract_state c USING(library_id) WHERE c.authority_mode IS NULL OR c.authority_mode<>'cloud-member') OR EXISTS(SELECT 1 FROM onboarding_library_selection s LEFT JOIN library_cloud_bindings b USING(library_id) WHERE b.library_id IS NULL)").fetch_one(&mut *conn).await?;
    if invalid {
        return Err(Error::Corrupt);
    }
    let operations: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT operation_id FROM onboarding_intents")
            .fetch_all(&mut *conn)
            .await?;
    for bytes in operations {
        let operation = Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt)?;
        let (stored, _) = intent(conn, operation).await?;
        if let Some(disposition) = sqlx::query("SELECT absence_canonical,absence_sha256 FROM onboarding_dispositions WHERE operation_id=?")
            .bind(&bytes).fetch_optional(&mut *conn).await? {
            let evidence: Vec<u8> = disposition.try_get("absence_canonical")?;
            if sha(&evidence).as_slice() != disposition.try_get::<Vec<u8>, _>("absence_sha256")?
                || stored.principal.is_none() { return Err(Error::Corrupt); }
            check_absence(&evidence).map_err(|_| Error::Corrupt)?;
        }
        if let Some(previous) = stored.replaces_operation {
            let (old, _) = intent(conn, previous).await?;
            if old.state != "abandoned-expired"
                || old.credential_reference != stored.credential_reference
                || old.command.device_id != stored.command.device_id
                || old.command.requested_library_id != stored.command.requested_library_id
                || old.command.environment_id != stored.command.environment_id
                || old.command.device_credential_sha256 != stored.command.device_credential_sha256
                || stored
                    .principal
                    .as_ref()
                    .is_some_and(|p| Some(p) != old.principal.as_ref())
            {
                return Err(Error::Corrupt);
            }
        }
        let receipt = sqlx::query(
            "SELECT receipt_canonical,receipt_sha256 FROM onboarding_receipts WHERE operation_id=?",
        )
        .bind(bytes)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(receipt) = receipt {
            let bytes: Vec<u8> = receipt.try_get("receipt_canonical")?;
            if sha(&bytes).as_slice() != receipt.try_get::<Vec<u8>, _>("receipt_sha256")? {
                return Err(Error::Corrupt);
            }
            check_receipt(&strict(&bytes)?, &stored)?;
            if ![
                "abandoned-expired",
                "receipt-ready",
                "reconciliation-required",
                "library-choice-required",
                "applied",
            ]
            .contains(&stored.state.as_str())
            {
                return Err(Error::Corrupt);
            }
        } else if ![
            "prepared",
            "cancelled",
            "authenticated",
            "unknown",
            "abandoned-expired",
        ]
        .contains(&stored.state.as_str())
        {
            return Err(Error::Corrupt);
        }
    }
    let rows: Vec<Vec<u8>> = sqlx::query_scalar("SELECT library_id FROM library_cloud_bindings")
        .fetch_all(&mut *conn)
        .await?;
    for bytes in rows {
        cached_binding(conn, LibraryId::from_bytes(&bytes)?).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at() -> Timestamp {
        Timestamp::try_from(1_800_000_000_000).unwrap()
    }
    fn who() -> EnrollmentPrincipal {
        EnrollmentPrincipal {
            issuer: "https://synthetic.invalid/".into(),
            subject: "synthetic-user".into(),
        }
    }
    async fn fixture() -> (tempfile::TempDir, LocalLibraryStore, LocalIdentity) {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
            .await
            .unwrap();
        (dir, store, id)
    }
    fn preparation(id: &LocalIdentity) -> PrepareEnrollment {
        PrepareEnrollment {
            operation: Uuid::new_v4(),
            library: id.library_id,
            actor: id.principal_id,
            environment_id: "synthetic".into(),
            issuer: who().issuer,
            credential_reference: format!("keychain:{}", Uuid::new_v4()),
            device_commitment: "a".repeat(64),
            device_display_name: "Synthetic Mac".into(),
        }
    }
    fn receipt(intent: &EnrollmentIntent, existing: Option<LibraryId>) -> NativeBootstrapReceipt {
        NativeBootstrapReceipt {
            schema: SCHEMA.into(),
            environment_id: intent.command.environment_id.clone(),
            operation_id: intent.operation,
            request_sha256: intent.command_sha256.clone(),
            issuer: who().issuer,
            subject: who().subject,
            account_id: Uuid::new_v4(),
            identity_id: Uuid::new_v4(),
            device_id: intent.command.device_id,
            device_credential_sha256: intent.command.device_credential_sha256.clone(),
            outcome: if existing.is_some() {
                "existing-default-requires-local-choice"
            } else {
                "claimed-default"
            }
            .into(),
            requested_library_id: intent.command.requested_library_id,
            default_library_id: existing.unwrap_or(intent.command.requested_library_id),
            membership_id: Uuid::new_v4(),
            contract_version: "1".into(),
            authorization_generation: "1".into(),
            stream_id: Uuid::new_v4(),
            stream_epoch: Uuid::new_v4(),
            initial_high_water: "0".into(),
            completed_at: at().get().to_string(),
        }
    }
    fn envelope(receipt: &NativeBootstrapReceipt) -> Vec<u8> {
        let bytes = canonical(receipt).unwrap();
        canonical(
            &serde_json::json!({"receipt":receipt,"receipt_sha256":hex(&sha(bytes.as_bytes()))}),
        )
        .unwrap()
        .into_bytes()
    }
    fn access(receipt: &NativeBootstrapReceipt) -> NativeSessionObservation {
        NativeSessionObservation {
            schema: SCHEMA.into(),
            issuer: receipt.issuer.clone(),
            subject: receipt.subject.clone(),
            account_id: receipt.account_id,
            identity_id: receipt.identity_id,
            device_id: receipt.device_id,
            default_library_id: receipt.default_library_id,
            account_state: "active".into(),
            identity_state: "active".into(),
            device_state: "active".into(),
            credential_state: "active".into(),
            credential_revision: "1".into(),
            membership_id: receipt.membership_id,
            membership_role: "owner".into(),
            membership_revision: "1".into(),
            authorization_generation: "1".into(),
            observed_at: at().get().to_string(),
        }
    }
    async fn ready(
        store: &LocalLibraryStore,
        id: &LocalIdentity,
        existing: Option<LibraryId>,
    ) -> (Uuid, i64, NativeBootstrapReceipt) {
        let input = preparation(id);
        let intent = store.prepare_enrollment(&input, at()).await.unwrap();
        assert_eq!(intent.command.requested_library_id, id.library_id);
        assert_eq!(
            store
                .prepare_enrollment(&input, at())
                .await
                .unwrap()
                .command_canonical,
            intent.command_canonical
        );
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        let receipt = receipt(&intent, existing);
        store
            .record_enrollment_receipt(input.operation, &envelope(&receipt), at())
            .await
            .unwrap();
        (input.operation, generation, receipt)
    }
    async fn count(store: &LocalLibraryStore, table: &str) -> i64 {
        assert!(
            [
                "libraries",
                "onboarding_receipts",
                "onboarding_intents",
                "sync_targets",
                "account_cache",
                "library_cloud_bindings"
            ]
            .contains(&table)
        );
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(store.db.pool())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn same_id_atomic_apply_receipt_replay_and_association_aware_restart() {
        let (dir, store, id) = fixture().await;
        assert_eq!(store.info().migration_count, 15);
        let (operation, generation, receipt) = ready(&store, &id, None).await;
        let observed = canonical(&access(&receipt)).unwrap();
        let result = store
            .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
            .await
            .unwrap();
        assert_eq!(
            result,
            EnrollmentApplication::Applied {
                library: id.library_id
            }
        );
        store
            .record_enrollment_receipt(operation, &envelope(&receipt), at())
            .await
            .unwrap();
        assert_eq!(
            store
                .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
                .await
                .unwrap(),
            result
        );
        assert_eq!(count(&store, "libraries").await, 1);
        assert_eq!(count(&store, "onboarding_receipts").await, 1);
        assert_eq!(count(&store, "sync_targets").await, 0);
        assert_eq!(count(&store, "account_cache").await, 0);
        let revision: i64 = sqlx::query_scalar("SELECT local_revision FROM libraries")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(revision, 2);
        store.verify_integrity().await.unwrap();
        store.close().await;
        let (reopened, resumed) =
            LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
                .await
                .unwrap();
        assert_eq!(resumed.library_id, id.library_id);
        assert_eq!(resumed.authority_mode, "cloud-member");
        assert_eq!(resumed.principal_id, id.principal_id);
        assert!(
            reopened
                .register_project(
                    id.principal_id,
                    &RegisteredProject {
                        library_id: id.library_id,
                        project_id: ProjectId::new(),
                        association_commit_id: CommitId::new(),
                        association_sha256: [1; 32],
                        source_origin_library_id: None
                    },
                    at()
                )
                .await
                .is_err()
        );
        assert_eq!(count(&reopened, "libraries").await, 1);
        reopened.verify_integrity().await.unwrap();
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // Keep both original/selected-library rejoin lifecycles together.
    async fn logout_choice_and_rejoin_preserve_library_and_reject_other_identity() {
        for existing in [None, Some(LibraryId::new())] {
            let (_dir, store, id) = fixture().await;
            let (operation, generation, receipt) = ready(&store, &id, existing).await;
            let observed = canonical(&access(&receipt)).unwrap();
            assert_eq!(
                store
                    .enrollment_library_choice(operation, &who(), observed.as_bytes(), at())
                    .await
                    .unwrap(),
                receipt.default_library_id
            );
            assert_eq!(count(&store, "library_cloud_bindings").await, 0);
            store
                .open_existing_cloud_library(
                    operation,
                    &who(),
                    generation,
                    observed.as_bytes(),
                    at(),
                )
                .await
                .unwrap();
            let bound = store
                .bound_enrollment_intent(id.library_id, "synthetic", &who())
                .await
                .unwrap();
            // Use the actual fixture environment rather than a UI label.
            let environment = &bound.command.environment_id;
            store.advance_enrollment_session(None).await.unwrap();
            assert_eq!(
                store
                    .saved_enrollment_state(id.library_id, environment)
                    .await
                    .unwrap()
                    .as_deref(),
                Some("signed-out")
            );
            assert_eq!(
                store
                    .cached_enrollment_credential(id.library_id, environment)
                    .await
                    .unwrap()
                    .as_deref(),
                Some(bound.credential_reference.as_str())
            );
            let other = EnrollmentPrincipal {
                issuer: who().issuer,
                subject: "other".into(),
            };
            assert!(
                store
                    .bound_enrollment_intent(id.library_id, environment, &other)
                    .await
                    .is_err()
            );
            assert!(
                store
                    .enrollment_library_choice(operation, &other, observed.as_bytes(), at())
                    .await
                    .is_err()
            );
            assert!(
                store
                    .cached_enrollment_credential(id.library_id, "wrong-environment")
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                store
                    .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
                    .await
                    .is_err()
            );
            sqlx::query("UPDATE libraries SET display_name='My preserved work',local_revision=local_revision+1 WHERE library_id=?")
                .bind(id.library_id.bytes()).execute(store.db.pool()).await.unwrap();
            let fresh = store
                .advance_enrollment_session(Some(&who()))
                .await
                .unwrap();
            assert_eq!(
                store
                    .apply_enrollment(operation, &who(), fresh, observed.as_bytes(), at())
                    .await
                    .unwrap(),
                EnrollmentApplication::Applied {
                    library: receipt.default_library_id
                }
            );
            let name: String =
                sqlx::query_scalar("SELECT display_name FROM libraries WHERE library_id=?")
                    .bind(id.library_id.bytes())
                    .fetch_one(store.db.pool())
                    .await
                    .unwrap();
            assert_eq!(name, "My preserved work");
            assert_eq!(count(&store, "onboarding_intents").await, 1);
            assert_eq!(count(&store, "onboarding_receipts").await, 1);
            assert_eq!(
                count(&store, "libraries").await,
                if existing.is_some() { 2 } else { 1 }
            );
            store.verify_integrity().await.unwrap();
        }
    }

    #[tokio::test]
    async fn changed_local_content_retains_receipt_without_overwriting() {
        let (_dir, store, id) = fixture().await;
        let (operation, generation, receipt) = ready(&store, &id, None).await;
        sqlx::query(
            "UPDATE libraries SET display_name='User Rename',local_revision=local_revision+1",
        )
        .execute(store.db.pool())
        .await
        .unwrap();
        let result = store
            .apply_enrollment(
                operation,
                &who(),
                generation,
                canonical(&access(&receipt)).unwrap().as_bytes(),
                at(),
            )
            .await
            .unwrap();
        assert_eq!(result, EnrollmentApplication::ReconciliationRequired);
        assert_eq!(count(&store, "onboarding_receipts").await, 1);
        assert_eq!(count(&store, "library_cloud_bindings").await, 0);
        let name: String = sqlx::query_scalar("SELECT display_name FROM libraries")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(name, "User Rename");
    }

    #[tokio::test]
    async fn cancellation_account_isolation_and_late_receipt_recovery() {
        let (_dir, store, id) = fixture().await;
        let input = preparation(&id);
        let intent = store.prepare_enrollment(&input, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        store.advance_enrollment_session(None).await.unwrap();
        let receipt = receipt(&intent, None);
        store
            .record_enrollment_receipt(input.operation, &envelope(&receipt), at())
            .await
            .unwrap();
        let observed = canonical(&access(&receipt)).unwrap();
        assert_eq!(
            store
                .apply_enrollment(
                    input.operation,
                    &who(),
                    generation,
                    observed.as_bytes(),
                    at()
                )
                .await
                .unwrap_err(),
            Error::Conflict
        );
        let other = EnrollmentPrincipal {
            issuer: who().issuer,
            subject: "other-account".into(),
        };
        assert!(
            store
                .enrollment_intent(input.operation, &other)
                .await
                .is_err()
        );
        let switched = store
            .advance_enrollment_session(Some(&other))
            .await
            .unwrap();
        assert!(
            store
                .apply_enrollment(input.operation, &other, switched, observed.as_bytes(), at())
                .await
                .is_err()
        );
        let resumed = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .apply_enrollment(input.operation, &who(), resumed, observed.as_bytes(), at())
            .await
            .unwrap();
        assert_eq!(count(&store, "onboarding_intents").await, 1);
        store.advance_enrollment_session(None).await.unwrap();
        assert_eq!(
            store
                .cached_cloud_library(id.library_id)
                .await
                .unwrap()
                .unwrap()
                .state,
            "signed-out"
        );
    }

    #[tokio::test]
    async fn existing_default_requires_explicit_choice_and_preserves_local_identity() {
        let (dir, store, id) = fixture().await;
        let cloud = LibraryId::new();
        let (operation, generation, receipt) = ready(&store, &id, Some(cloud)).await;
        let observed = canonical(&access(&receipt)).unwrap();
        assert_eq!(
            store
                .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
                .await
                .unwrap(),
            EnrollmentApplication::LibraryChoiceRequired { library: cloud }
        );
        assert_eq!(count(&store, "libraries").await, 1);
        for _ in 0..2 {
            store
                .open_existing_cloud_library(
                    operation,
                    &who(),
                    generation,
                    observed.as_bytes(),
                    at(),
                )
                .await
                .unwrap();
        }
        assert_eq!(count(&store, "libraries").await, 2);
        assert_eq!(
            store
                .selected_cloud_library(&who())
                .await
                .unwrap()
                .unwrap()
                .library,
            cloud
        );
        assert!(
            store
                .selected_cloud_library(&EnrollmentPrincipal {
                    issuer: who().issuer,
                    subject: "other".into()
                })
                .await
                .unwrap()
                .is_none()
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
        let (reopened, local) =
            LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
                .await
                .unwrap();
        assert_eq!(local.library_id, id.library_id);
        assert_eq!(local.authority_mode, "local-only");
        assert_eq!(
            reopened
                .selected_cloud_library(&who())
                .await
                .unwrap()
                .unwrap()
                .library,
            cloud
        );
    }

    #[tokio::test]
    async fn malformed_receipts_disabled_access_and_populated_preflight_fail_closed() {
        let (_dir, store, id) = fixture().await;
        let input = preparation(&id);
        let intent = store.prepare_enrollment(&input, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        let valid = receipt(&intent, None);
        let mut wrong = valid.clone();
        wrong.subject = "other".into();
        assert!(
            store
                .record_enrollment_receipt(input.operation, &envelope(&wrong), at())
                .await
                .is_err()
        );
        let mut wrong = valid.clone();
        wrong.request_sha256 = "b".repeat(64);
        assert!(
            store
                .record_enrollment_receipt(input.operation, &envelope(&wrong), at())
                .await
                .is_err()
        );
        let mut wrong = valid.clone();
        wrong.account_id = Uuid::nil();
        assert!(
            store
                .record_enrollment_receipt(input.operation, &envelope(&wrong), at())
                .await
                .is_err()
        );
        assert_eq!(count(&store, "onboarding_receipts").await, 0);
        store
            .record_enrollment_receipt(input.operation, &envelope(&valid), at())
            .await
            .unwrap();
        let mut wrong = valid.clone();
        wrong.membership_id = Uuid::new_v4();
        assert!(
            store
                .record_enrollment_receipt(input.operation, &envelope(&wrong), at())
                .await
                .is_err()
        );
        for field in [
            "account_state",
            "identity_state",
            "device_state",
            "credential_state",
            "membership_role",
            "observed_at",
        ] {
            let mut value = serde_json::to_value(access(&valid)).unwrap();
            value[field] = serde_json::json!(if field == "observed_at" {
                "1"
            } else {
                "disabled"
            });
            assert!(
                store
                    .apply_enrollment(
                        input.operation,
                        &who(),
                        generation,
                        canonical(&value).unwrap().as_bytes(),
                        at()
                    )
                    .await
                    .is_err()
            );
        }
        assert_eq!(count(&store, "library_cloud_bindings").await, 0);
        let (_dir2, populated, local) = fixture().await;
        sqlx::query("INSERT INTO library_media VALUES(?,zeroblob(32),'image/png',1,NULL,NULL,?)")
            .bind(local.library_id.bytes())
            .bind(at().get())
            .execute(populated.db.pool())
            .await
            .unwrap();
        assert!(
            populated
                .prepare_enrollment(&preparation(&local), at())
                .await
                .is_err()
        );
        assert_eq!(count(&populated, "onboarding_intents").await, 0);
    }

    #[tokio::test]
    async fn prepared_cancel_and_unknown_recovery_preserve_immutable_intents() {
        let (_dir, store, id) = fixture().await;
        let first = preparation(&id);
        let bytes = store
            .prepare_enrollment(&first, at())
            .await
            .unwrap()
            .command_canonical;
        assert_eq!(
            store
                .pending_enrollment(id.library_id, "synthetic", None)
                .await
                .unwrap()
                .unwrap()
                .command_canonical,
            bytes
        );
        store
            .cancel_prepared_enrollment(first.operation)
            .await
            .unwrap();
        assert!(
            store
                .pending_enrollment(id.library_id, "synthetic", None)
                .await
                .unwrap()
                .is_none()
        );
        let second = preparation(&id);
        store.prepare_enrollment(&second, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(second.operation, &who(), generation)
            .await
            .unwrap();
        assert!(
            store
                .cancel_prepared_enrollment(second.operation)
                .await
                .is_err()
        );
        assert!(
            store
                .pending_enrollment(id.library_id, "synthetic", None)
                .await
                .is_err()
        );
        let retained = store
            .pending_enrollment(id.library_id, "synthetic", Some(&who()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retained.state, "unknown");
        sqlx::query(
            "UPDATE libraries SET display_name='Later edit',local_revision=local_revision+1",
        )
        .execute(store.db.pool())
        .await
        .unwrap();
        assert_eq!(
            store
                .dispatch_enrollment(second.operation, &who(), generation)
                .await
                .unwrap()
                .command_canonical,
            retained.command_canonical
        );
        assert!(
            sqlx::query("UPDATE onboarding_intents SET command_sha256=zeroblob(32)")
                .execute(store.db.pool())
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM onboarding_intents")
                .execute(store.db.pool())
                .await
                .is_err()
        );
        assert_eq!(count(&store, "onboarding_intents").await, 2);
    }

    #[tokio::test]
    async fn first_dispatch_rechecks_preflight_and_strict_receipt_bytes() {
        let (_dir, store, id) = fixture().await;
        let input = preparation(&id);
        store.prepare_enrollment(&input, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        sqlx::query("UPDATE libraries SET display_name='Changed',local_revision=local_revision+1")
            .execute(store.db.pool())
            .await
            .unwrap();
        assert!(
            store
                .dispatch_enrollment(input.operation, &who(), generation)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .pending_enrollment(id.library_id, "synthetic", None)
                .await
                .unwrap()
                .unwrap()
                .state,
            "prepared"
        );
        store
            .cancel_prepared_enrollment(input.operation)
            .await
            .unwrap();
        let (operation, _, receipt) = ready(&store, &id, None).await;
        let valid = envelope(&receipt);
        let mut spaced = valid.clone();
        spaced.push(b' ');
        assert!(
            store
                .record_enrollment_receipt(operation, &spaced, at())
                .await
                .is_err()
        );
        let mut extra: serde_json::Value = serde_json::from_slice(&valid).unwrap();
        extra["unexpected"] = true.into();
        assert!(
            store
                .record_enrollment_receipt(operation, canonical(&extra).unwrap().as_bytes(), at())
                .await
                .is_err()
        );
        let duplicate = String::from_utf8(valid).unwrap().replacen(
            '{',
            "{\"receipt_sha256\":\"duplicate\",",
            1,
        );
        assert!(
            store
                .record_enrollment_receipt(operation, duplicate.as_bytes(), at())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn failed_binding_insert_rolls_back_authority_but_retains_receipt() {
        let (_dir, store, id) = fixture().await;
        let (operation, generation, receipt) = ready(&store, &id, None).await;
        sqlx::query("CREATE TRIGGER synthetic_abort BEFORE INSERT ON library_cloud_bindings BEGIN SELECT RAISE(ABORT,'synthetic fault'); END").execute(store.db.pool()).await.unwrap();
        let observed = canonical(&access(&receipt)).unwrap();
        assert!(
            store
                .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
                .await
                .is_err()
        );
        assert_eq!(store.ensure_default_library(at()).await.unwrap(), id);
        assert_eq!(count(&store, "onboarding_receipts").await, 1);
        assert_eq!(count(&store, "library_cloud_bindings").await, 0);
        let revision: i64 = sqlx::query_scalar("SELECT local_revision FROM libraries")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(revision, 1);
        sqlx::query("DROP TRIGGER synthetic_abort")
            .execute(store.db.pool())
            .await
            .unwrap();
        store
            .apply_enrollment(operation, &who(), generation, observed.as_bytes(), at())
            .await
            .unwrap();
        store.verify_integrity().await.unwrap();
    }

    #[tokio::test]
    async fn independent_connections_converge_on_one_journal_and_application() {
        let (dir, store, id) = fixture().await;
        let (other, _) = LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
            .await
            .unwrap();
        let input = preparation(&id);
        let (a, b) = tokio::join!(
            store.prepare_enrollment(&input, at()),
            other.prepare_enrollment(&input, at())
        );
        assert_eq!(a.unwrap().command_canonical, b.unwrap().command_canonical);
        assert!(
            other
                .prepare_enrollment(&preparation(&id), at())
                .await
                .is_err()
        );
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        let pending = store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        let receipt = receipt(&pending, None);
        let bytes = envelope(&receipt);
        let (a, b) = tokio::join!(
            store.record_enrollment_receipt(input.operation, &bytes, at()),
            other.record_enrollment_receipt(input.operation, &bytes, at())
        );
        a.unwrap();
        b.unwrap();
        let observed = canonical(&access(&receipt)).unwrap();
        let principal = who();
        let (a, b) = tokio::join!(
            store.apply_enrollment(
                input.operation,
                &principal,
                generation,
                observed.as_bytes(),
                at()
            ),
            other.apply_enrollment(
                input.operation,
                &principal,
                generation,
                observed.as_bytes(),
                at()
            )
        );
        assert_eq!(a.unwrap(), b.unwrap());
        assert_eq!(count(&store, "libraries").await, 1);
        assert_eq!(count(&store, "onboarding_intents").await, 1);
        store.verify_integrity().await.unwrap();
    }

    fn absence() -> Vec<u8> {
        canonical(&OperationAbsence {
            schema: SCHEMA.into(),
            code: "operation-not-yet-found".into(),
            retryable: true,
            outcome_known: false,
            request_id: Uuid::new_v4(),
        })
        .unwrap()
        .into_bytes()
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn expired_disposition_restart_replacement_and_late_receipt_preserve_history() {
        let (dir, store, id) = fixture().await;
        let mut input = preparation(&id);
        let original = store.prepare_enrollment(&input, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        assert!(
            store
                .abandon_expired_enrollment(
                    input.operation,
                    &who(),
                    generation,
                    &absence(),
                    false,
                    at()
                )
                .await
                .is_err()
        );
        assert!(
            store
                .abandon_expired_enrollment(
                    input.operation,
                    &who(),
                    generation - 1,
                    &absence(),
                    true,
                    at()
                )
                .await
                .is_err()
        );
        assert!(
            store
                .abandon_expired_enrollment(input.operation, &who(), generation, b"{}", true, at())
                .await
                .is_err()
        );
        store
            .abandon_expired_enrollment(input.operation, &who(), generation, &absence(), true, at())
            .await
            .unwrap();
        store
            .abandon_expired_enrollment(input.operation, &who(), generation, &absence(), true, at())
            .await
            .unwrap();
        assert!(
            store
                .pending_enrollment_credential(id.library_id, "synthetic")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .expired_enrollment_credential(id.library_id, "synthetic")
                .await
                .unwrap(),
            Some(input.credential_reference.clone())
        );
        assert!(
            store
                .dispatch_enrollment(input.operation, &who(), generation)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM onboarding_dispositions")
                .execute(store.db.pool())
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE onboarding_intents SET state='receipt-ready'")
                .execute(store.db.pool())
                .await
                .is_err()
        );
        store.close().await;
        let (store, reopened) =
            LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
                .await
                .unwrap();
        assert_eq!(reopened.library_id, id.library_id);
        let saved = store
            .enrollment_intent(original.operation, &who())
            .await
            .unwrap();
        assert_eq!(saved.state, "abandoned-expired");
        assert_eq!(saved.command_canonical, original.command_canonical);
        input.operation = Uuid::new_v4();
        let credential = input.credential_reference.clone();
        input.credential_reference = format!("keychain:{}", Uuid::new_v4());
        assert!(store.prepare_enrollment(&input, at()).await.is_err());
        input.credential_reference = credential;
        let (other, _) = LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
            .await
            .unwrap();
        let mut competing = input.clone();
        competing.operation = Uuid::new_v4();
        let (a, b) = tokio::join!(
            store.prepare_enrollment(&input, at()),
            other.prepare_enrollment(&competing, at())
        );
        assert_ne!(a.is_ok(), b.is_ok());
        let fresh = a.or(b).unwrap();
        assert_eq!(fresh.replaces_operation, Some(original.operation));
        let wrong = EnrollmentPrincipal {
            issuer: who().issuer,
            subject: "different-synthetic-user".into(),
        };
        let wrong_generation = store
            .advance_enrollment_session(Some(&wrong))
            .await
            .unwrap();
        assert!(
            store
                .dispatch_enrollment(fresh.operation, &wrong, wrong_generation)
                .await
                .is_err()
        );
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(fresh.operation, &who(), generation)
            .await
            .unwrap();
        let old_receipt = receipt(&original, None);
        store
            .record_enrollment_receipt(original.operation, &envelope(&old_receipt), at())
            .await
            .unwrap();
        assert!(
            store
                .apply_enrollment(
                    original.operation,
                    &who(),
                    generation,
                    canonical(&access(&old_receipt)).unwrap().as_bytes(),
                    at()
                )
                .await
                .is_err()
        );
        assert!(
            store
                .selected_cloud_library(&who())
                .await
                .unwrap()
                .is_none()
        );
        let mut new_receipt = old_receipt.clone();
        new_receipt.operation_id = fresh.operation;
        new_receipt.request_sha256 = fresh.command_sha256.clone();
        store
            .record_enrollment_receipt(fresh.operation, &envelope(&new_receipt), at())
            .await
            .unwrap();
        store
            .apply_enrollment(
                fresh.operation,
                &who(),
                generation,
                canonical(&access(&new_receipt)).unwrap().as_bytes(),
                at(),
            )
            .await
            .unwrap();
        store
            .record_enrollment_receipt(original.operation, &envelope(&old_receipt), at())
            .await
            .unwrap();
        assert_eq!(
            store
                .enrollment_intent(original.operation, &who())
                .await
                .unwrap()
                .state,
            "abandoned-expired"
        );
        assert_eq!(count(&store, "onboarding_intents").await, 2);
        assert_eq!(count(&store, "onboarding_receipts").await, 2);
        assert_eq!(count(&store, "libraries").await, 1);
        assert_eq!(count(&store, "library_cloud_bindings").await, 1);
        assert_eq!(
            store
                .saved_enrollment_state(id.library_id, "synthetic")
                .await
                .unwrap()
                .as_deref(),
            Some("cached")
        );
        assert_eq!(
            store
                .saved_enrollment_state(id.library_id, "other")
                .await
                .unwrap(),
            None
        );
        store.advance_enrollment_session(None).await.unwrap();
        assert_eq!(
            store
                .saved_enrollment_state(id.library_id, "synthetic")
                .await
                .unwrap()
                .as_deref(),
            Some("signed-out")
        );
        store.verify_integrity().await.unwrap();
        other.close().await;
        store.close().await;
        let (store, reopened) =
            LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
                .await
                .unwrap();
        assert_eq!(reopened.library_id, id.library_id);
        assert!(
            store
                .expired_enrollment_credential(id.library_id, "synthetic")
                .await
                .unwrap()
                .is_none()
        );
        store.verify_integrity().await.unwrap();
    }

    #[tokio::test]
    async fn v13_unknown_upgrade_preserves_intent_and_all_original_migration_checksums() {
        let (dir, store, id) = fixture().await;
        let input = preparation(&id);
        let original = store.prepare_enrollment(&input, at()).await.unwrap();
        let generation = store
            .advance_enrollment_session(Some(&who()))
            .await
            .unwrap();
        store
            .dispatch_enrollment(input.operation, &who(), generation)
            .await
            .unwrap();
        store.close().await;
        {
            // Reconstruct the exact prior schema in this disposable fixture.
            let db = rusqlite::Connection::open(dir.path().join("local.sqlite")).unwrap();
            db.execute_batch("DROP TABLE project_creation_intents; DELETE FROM _sqlx_migrations WHERE version=15; DROP TRIGGER onboarding_disposed_state_immutable; DROP TRIGGER onboarding_one_unresolved_insert; DROP TRIGGER onboarding_one_unresolved_update; DROP TABLE onboarding_replacements; DROP TABLE onboarding_dispositions; DROP INDEX onboarding_unresolved_library; CREATE UNIQUE INDEX onboarding_unresolved_library ON onboarding_intents(library_id,environment_id) WHERE state NOT IN ('applied','cancelled'); DELETE FROM _sqlx_migrations WHERE version=14; UPDATE schema_metadata SET minimum_reader=3,minimum_writer=3;").unwrap();
        }
        let (store, reopened) =
            LocalLibraryStore::open_app_state(dir.path().join("local.sqlite"), at())
                .await
                .unwrap();
        assert_eq!(reopened.library_id, id.library_id);
        let retained = store
            .enrollment_intent(input.operation, &who())
            .await
            .unwrap();
        assert_eq!(retained.state, "unknown");
        assert_eq!(retained.command_canonical, original.command_canonical);
        assert_eq!(retained.credential_reference, input.credential_reference);
        for migration in MIGRATOR.iter().filter(|m| m.version <= 13) {
            let checksum: Vec<u8> =
                sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=?")
                    .bind(migration.version)
                    .fetch_one(store.db.pool())
                    .await
                    .unwrap();
            assert_eq!(checksum, migration.checksum.as_ref());
        }
        store
            .abandon_expired_enrollment(input.operation, &who(), generation, &absence(), true, at())
            .await
            .unwrap();
        store.verify_integrity().await.unwrap();
    }

    #[tokio::test]
    async fn additive_v12_upgrade_preserves_existing_identity_content_and_ledger() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("local.sqlite");
        let device = DeviceId::new();
        let database = DatabaseId::new();
        let library = LibraryId::new();
        let local = Uuid::new_v4();
        {
            let db = rusqlite::Connection::open(&path).unwrap();
            db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT NOT NULL,installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,success BOOLEAN NOT NULL,checksum BLOB NOT NULL,execution_time BIGINT NOT NULL);").unwrap();
            for migration in MIGRATOR.iter().filter(|m| m.version <= 12) {
                db.execute_batch(migration.sql.as_str()).unwrap();
                db.execute("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(?,?,1,?,0)", rusqlite::params![migration.version, migration.description.as_ref(), migration.checksum.as_ref()]).unwrap();
            }
            db.execute("INSERT INTO schema_metadata VALUES(1,'photara.local.g2',?,1,2,2,'photara.canonical-json.v1',0)", [database.bytes()]).unwrap();
            db.execute(
                "INSERT INTO local_device VALUES(1,?,'Original Mac',0)",
                [device.bytes()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO normalization_policies VALUES(1,'16.0.0',?,?)",
                rusqlite::params![
                    sha(validation::POLICY.as_bytes()).to_vec(),
                    validation::POLICY
                ],
            )
            .unwrap();
            db.execute("INSERT INTO libraries VALUES(?,'Existing independent library','active',1,0,0,NULL,1,'{}')", [library.bytes()]).unwrap();
            db.execute(
                "INSERT INTO library_contract_state VALUES(?,1,'local-only',?,1,1,1,0,0)",
                rusqlite::params![library.bytes(), local.as_bytes().to_vec()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO library_media VALUES(?,zeroblob(32),'image/png',42,NULL,NULL,0)",
                [library.bytes()],
            )
            .unwrap();
        }
        let store = LocalLibraryStore::open(&path, OpenMode::OpenExisting, device, at())
            .await
            .unwrap();
        assert_eq!(store.info().database_id, database);
        assert_eq!(store.info().device_id, device);
        let name: String =
            sqlx::query_scalar("SELECT display_name FROM libraries WHERE library_id=?")
                .bind(library.bytes())
                .fetch_one(store.db.pool())
                .await
                .unwrap();
        assert_eq!(name, "Existing independent library");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT byte_length FROM library_media")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            42
        );
        for migration in MIGRATOR.iter().filter(|m| m.version <= 12) {
            let checksum: Vec<u8> =
                sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=?")
                    .bind(migration.version)
                    .fetch_one(store.db.pool())
                    .await
                    .unwrap();
            assert_eq!(checksum, migration.checksum.as_ref());
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT minimum_reader FROM schema_metadata")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            5
        );
        assert_eq!(count(&store, "libraries").await, 1);
        store.verify_integrity().await.unwrap();
    }
}
