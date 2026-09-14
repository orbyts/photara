//! Typed native enrollment boundary. Rust owns journals, canonical bytes and
//! local authority. The caller supplies verified provider observations, never SQL
//! or secrets. This object does not perform network or Keychain operations.
use crate::{BridgeError, BridgeLocalState};
use photara_core::contracts::LocalPrincipalId;
use photara_library::gen2::{self, LocalLibraryStore, Timestamp};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeEnrollmentPrincipal {
    pub issuer: String,
    pub subject: String,
}
impl From<BridgeEnrollmentPrincipal> for gen2::EnrollmentPrincipal {
    fn from(value: BridgeEnrollmentPrincipal) -> Self {
        Self {
            issuer: value.issuer,
            subject: value.subject,
        }
    }
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgePrepareEnrollment {
    pub operation_id: String,
    pub library_id: String,
    pub local_principal_id: String,
    pub environment_id: String,
    pub issuer: String,
    pub credential_reference: String,
    pub device_commitment_sha256: String,
    pub device_display_name: String,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeEnrollmentIntent {
    pub operation_id: String,
    pub requested_library_id: String,
    pub device_id: String,
    pub environment_id: String,
    pub command_canonical: Vec<u8>,
    pub command_sha256: String,
    pub challenge_canonical: Vec<u8>,
    pub credential_reference: String,
    pub principal: Option<BridgeEnrollmentPrincipal>,
    pub state: String,
    pub replaces_operation_id: Option<String>,
}
impl From<gen2::EnrollmentIntent> for BridgeEnrollmentIntent {
    fn from(value: gen2::EnrollmentIntent) -> Self {
        Self {
            operation_id: value.operation.to_string(),
            requested_library_id: value.command.requested_library_id.uuid().to_string(),
            device_id: value.command.device_id.uuid().to_string(),
            environment_id: value.command.environment_id,
            command_canonical: value.command_canonical,
            command_sha256: value.command_sha256,
            challenge_canonical: value.challenge_canonical,
            credential_reference: value.credential_reference,
            principal: value.principal.map(|p| BridgeEnrollmentPrincipal {
                issuer: p.issuer,
                subject: p.subject,
            }),
            state: value.state,
            replaces_operation_id: value.replaces_operation.map(|id| id.to_string()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum BridgeEnrollmentApplication {
    Applied { library_id: String },
    LibraryChoiceRequired { library_id: String },
    ReconciliationRequired,
}
impl From<gen2::EnrollmentApplication> for BridgeEnrollmentApplication {
    fn from(value: gen2::EnrollmentApplication) -> Self {
        match value {
            gen2::EnrollmentApplication::Applied { library } => Self::Applied {
                library_id: library.uuid().to_string(),
            },
            gen2::EnrollmentApplication::LibraryChoiceRequired { library } => {
                Self::LibraryChoiceRequired {
                    library_id: library.uuid().to_string(),
                }
            }
            gen2::EnrollmentApplication::ReconciliationRequired => Self::ReconciliationRequired,
        }
    }
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeCachedCloudLibrary {
    pub library_id: String,
    pub environment_id: String,
    pub principal: BridgeEnrollmentPrincipal,
    pub account_id: String,
    pub state: String,
}
impl From<gen2::CachedCloudLibrary> for BridgeCachedCloudLibrary {
    fn from(value: gen2::CachedCloudLibrary) -> Self {
        Self {
            library_id: value.library.uuid().to_string(),
            environment_id: value.environment_id,
            principal: BridgeEnrollmentPrincipal {
                issuer: value.principal.issuer,
                subject: value.principal.subject,
            },
            account_id: value.account_id.to_string(),
            state: value.state,
        }
    }
}

fn failure() -> BridgeError {
    BridgeError::Store {
        message: "Local onboarding operation failed; retained evidence was not discarded.".into(),
    }
}
fn invalid() -> BridgeError {
    BridgeError::InvalidArgument {
        message: "Invalid onboarding coordinate.".into(),
    }
}
fn uuid(value: &str) -> Result<Uuid, BridgeError> {
    let parsed = Uuid::parse_str(value).map_err(|_| invalid())?;
    if parsed.is_nil() {
        return Err(invalid());
    }
    Ok(parsed)
}
fn library(value: &str) -> Result<gen2::LibraryId, BridgeError> {
    gen2::LibraryId::try_from(uuid(value)?).map_err(|_| invalid())
}
fn now() -> Result<Timestamp, BridgeError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| failure())?;
    Timestamp::try_from(i64::try_from(elapsed.as_millis()).map_err(|_| failure())?)
        .map_err(|_| failure())
}

/// Serialize synchronous FFI operations on one runtime. Native must invoke
/// storage work off the UI thread. Independent processes serialize in `SQLite`.
#[derive(uniffi::Object)]
pub struct PhotaraLocalOnboarding {
    runtime: Mutex<tokio::runtime::Runtime>,
    store: LocalLibraryStore,
}
impl PhotaraLocalOnboarding {
    fn run<T>(&self, future: impl Future<Output = gen2::Result<T>>) -> Result<T, BridgeError> {
        self.runtime
            .lock()
            .map_err(|_| failure())?
            .block_on(future)
            .map_err(|_| failure())
    }
}
#[uniffi::export]
// UniFFI owns the buffers/strings crossing the native ABI.
#[allow(clippy::needless_pass_by_value)]
impl PhotaraLocalOnboarding {
    /// # Errors
    /// Refuses invalid paths, unsupported floors and corrupt journals.
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, BridgeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| failure())?;
        let (store, _) = runtime
            .block_on(LocalLibraryStore::open_app_state(path, now()?))
            .map_err(|_| failure())?;
        runtime
            .block_on(store.verify_integrity())
            .map_err(|_| failure())?;
        Ok(Arc::new(Self {
            runtime: Mutex::new(runtime),
            store,
        }))
    }
    /// # Errors
    /// Refuses damaged association evidence; never returns historical local authority as current.
    pub fn local_state(&self) -> Result<BridgeLocalState, BridgeError> {
        let id = self.run(self.store.ensure_default_library(now()?))?;
        Ok(BridgeLocalState {
            database_id: id.database_id.uuid().to_string(),
            device_id: id.device_id.uuid().to_string(),
            library_id: id.library_id.uuid().to_string(),
            principal_id: id.principal_id.to_string(),
            authority_mode: id.authority_mode,
        })
    }
    /// # Errors
    /// Rejects populated Libraries, unresolved operations and invalid public coordinates.
    pub fn prepare(
        &self,
        input: BridgePrepareEnrollment,
    ) -> Result<BridgeEnrollmentIntent, BridgeError> {
        let request = gen2::PrepareEnrollment {
            operation: uuid(&input.operation_id)?,
            library: library(&input.library_id)?,
            actor: LocalPrincipalId::from_uuid(uuid(&input.local_principal_id)?)
                .map_err(|_| invalid())?,
            environment_id: input.environment_id,
            issuer: input.issuer,
            credential_reference: input.credential_reference,
            device_commitment: input.device_commitment_sha256,
            device_display_name: input.device_display_name,
        };
        self.run(self.store.prepare_enrollment(&request, now()?))
            .map(Into::into)
    }
    /// # Errors
    /// Refuses malformed principal or generation overflow. Call before cancel/account switch.
    pub fn advance_session(
        &self,
        principal: Option<BridgeEnrollmentPrincipal>,
    ) -> Result<i64, BridgeError> {
        let principal = principal.map(Into::into);
        self.run(self.store.advance_enrollment_session(principal.as_ref()))
    }
    /// # Errors
    /// Refuses stale session/changed preflight. Commits unknown outcome before network dispatch.
    pub fn dispatch(
        &self,
        operation_id: String,
        principal: BridgeEnrollmentPrincipal,
        generation: i64,
    ) -> Result<BridgeEnrollmentIntent, BridgeError> {
        self.run(self.store.dispatch_enrollment(
            uuid(&operation_id)?,
            &principal.into(),
            generation,
        ))
        .map(Into::into)
    }
    /// # Errors
    /// Refuses access to a journal bound to another principal.
    pub fn pending(
        &self,
        library_id: String,
        environment_id: String,
        principal: Option<BridgeEnrollmentPrincipal>,
    ) -> Result<Option<BridgeEnrollmentIntent>, BridgeError> {
        let principal = principal.map(Into::into);
        self.run(self.store.pending_enrollment(
            library(&library_id)?,
            &environment_id,
            principal.as_ref(),
        ))
        .map(|v| v.map(Into::into))
    }
    /// # Errors
    /// Returns an opaque Keychain locator only; never a principal or command.
    pub fn pending_credential(
        &self,
        library_id: String,
        environment_id: String,
    ) -> Result<Option<String>, BridgeError> {
        self.run(
            self.store
                .pending_enrollment_credential(library(&library_id)?, &environment_id),
        )
    }
    /// # Errors
    /// Refuses invalid coordinates or damaged terminal evidence.
    pub fn expired_credential(
        &self,
        library_id: String,
        environment_id: String,
    ) -> Result<Option<String>, BridgeError> {
        self.run(
            self.store
                .expired_enrollment_credential(library(&library_id)?, &environment_id),
        )
    }
    /// # Errors
    /// Rejects corrupt local binding evidence. Presentation is not live access.
    pub fn saved_state(
        &self,
        library_id: String,
        environment_id: String,
    ) -> Result<Option<String>, BridgeError> {
        self.run(
            self.store
                .saved_enrollment_state(library(&library_id)?, &environment_id),
        )
    }
    /// # Errors
    /// Rejects corrupt cached bindings. Returns only an opaque credential locator.
    pub fn cached_credential(
        &self,
        library_id: String,
        environment_id: String,
    ) -> Result<Option<String>, BridgeError> {
        self.run(
            self.store
                .cached_enrollment_credential(library(&library_id)?, &environment_id),
        )
    }
    /// # Errors
    /// Requires the same natively verified principal as the original binding.
    pub fn bound_intent(
        &self,
        library_id: String,
        environment_id: String,
        principal: BridgeEnrollmentPrincipal,
    ) -> Result<BridgeEnrollmentIntent, BridgeError> {
        self.run(self.store.bound_enrollment_intent(
            library(&library_id)?,
            &environment_id,
            &principal.into(),
        ))
        .map(Into::into)
    }
    /// # Errors
    /// Refuses stale principal/generation, non-404 evidence or a recorded receipt.
    pub fn abandon_expired(
        &self,
        operation_id: String,
        principal: BridgeEnrollmentPrincipal,
        generation: i64,
        absence_canonical: Vec<u8>,
    ) -> Result<(), BridgeError> {
        self.run(self.store.abandon_expired_enrollment(
            uuid(&operation_id)?,
            &principal.into(),
            generation,
            &absence_canonical,
            true,
            now()?,
        ))
    }
    /// # Errors
    /// Submitted/unknown operations cannot be cancelled away.
    pub fn cancel_prepared(&self, operation_id: String) -> Result<(), BridgeError> {
        self.run(self.store.cancel_prepared_enrollment(uuid(&operation_id)?))
    }
    /// # Errors
    /// Rejects noncanonical/mismatched receipt bytes. No session requirement for late retention.
    pub fn record_receipt(
        &self,
        operation_id: String,
        envelope_canonical: Vec<u8>,
    ) -> Result<(), BridgeError> {
        self.run(self.store.record_enrollment_receipt(
            uuid(&operation_id)?,
            &envelope_canonical,
            now()?,
        ))
    }
    /// # Errors
    /// Requires fresh canonical current-session DTO and exact active session generation.
    pub fn apply(
        &self,
        operation_id: String,
        principal: BridgeEnrollmentPrincipal,
        generation: i64,
        session_canonical: Vec<u8>,
    ) -> Result<BridgeEnrollmentApplication, BridgeError> {
        self.run(self.store.apply_enrollment(
            uuid(&operation_id)?,
            &principal.into(),
            generation,
            &session_canonical,
            now()?,
        ))
        .map(Into::into)
    }
    /// # Errors
    /// Requires verified identity and fresh active access to the receipt's Library.
    pub fn library_choice(
        &self,
        operation_id: String,
        principal: BridgeEnrollmentPrincipal,
        session_canonical: Vec<u8>,
    ) -> Result<String, BridgeError> {
        self.run(self.store.enrollment_library_choice(
            uuid(&operation_id)?,
            &principal.into(),
            &session_canonical,
            now()?,
        ))
        .map(|id| id.uuid().to_string())
    }
    /// # Errors
    /// Requires explicit user choice; preserves the independent original local Library.
    pub fn open_existing_cloud_library(
        &self,
        operation_id: String,
        principal: BridgeEnrollmentPrincipal,
        generation: i64,
        session_canonical: Vec<u8>,
    ) -> Result<BridgeEnrollmentApplication, BridgeError> {
        self.run(self.store.open_existing_cloud_library(
            uuid(&operation_id)?,
            &principal.into(),
            generation,
            &session_canonical,
            now()?,
        ))
        .map(Into::into)
    }
    /// # Errors
    /// Refuses corrupt caches. This result is presentation data, not online authority.
    pub fn selected_cloud_library(
        &self,
        principal: BridgeEnrollmentPrincipal,
    ) -> Result<Option<BridgeCachedCloudLibrary>, BridgeError> {
        self.run(self.store.selected_cloud_library(&principal.into()))
            .map(|v| v.map(Into::into))
    }
    /// # Errors
    /// Fails if the runtime lock is poisoned.
    pub fn close(&self) -> Result<(), BridgeError> {
        self.runtime
            .lock()
            .map_err(|_| failure())?
            .block_on(self.store.close());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_bridge_prepares_recovers_and_cancels_without_provider_or_keychain() {
        let directory = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("photara-onboarding-bridge-{}", Uuid::new_v4()));
        let path = directory
            .join("local.sqlite")
            .to_string_lossy()
            .into_owned();
        let bridge = PhotaraLocalOnboarding::open(path.clone()).unwrap();
        let id = bridge.local_state().unwrap();
        assert_eq!(id.authority_mode, "local-only");
        let operation_id = Uuid::new_v4().to_string();
        let prepared = bridge
            .prepare(BridgePrepareEnrollment {
                operation_id: operation_id.clone(),
                library_id: id.library_id.clone(),
                local_principal_id: id.principal_id,
                environment_id: "synthetic".into(),
                issuer: "https://synthetic.invalid/".into(),
                credential_reference: format!("keychain:{}", Uuid::new_v4()),
                device_commitment_sha256: "a".repeat(64),
                device_display_name: "Synthetic Mac".into(),
            })
            .unwrap();
        assert_eq!(prepared.state, "prepared");
        let decoded: gen2::NativeBootstrapCommand =
            serde_json::from_slice(&prepared.command_canonical).unwrap();
        assert_eq!(
            decoded.requested_library_id.uuid().to_string(),
            id.library_id
        );
        let challenge: gen2::NativeChallengeCommand =
            serde_json::from_slice(&prepared.challenge_canonical).unwrap();
        assert_eq!(challenge.schema, "photara.onboarding.v1");
        assert_eq!(challenge.action, "bootstrap");
        assert_eq!(challenge.operation_id.to_string(), operation_id);
        assert_eq!(challenge.request_sha256, prepared.command_sha256);
        assert_eq!(challenge.device_id.uuid().to_string(), prepared.device_id);
        assert_eq!(challenge.device_credential_sha256, "a".repeat(64));
        assert!(!prepared.challenge_canonical.contains(&b' '));
        bridge.close().unwrap();
        drop(bridge);
        let reopened = PhotaraLocalOnboarding::open(path).unwrap();
        assert_eq!(reopened.local_state().unwrap().library_id, id.library_id);
        assert_eq!(
            reopened
                .pending(id.library_id.clone(), "synthetic".into(), None)
                .unwrap()
                .unwrap()
                .command_canonical,
            prepared.command_canonical
        );
        reopened.cancel_prepared(operation_id).unwrap();
        assert!(
            reopened
                .pending(id.library_id, "synthetic".into(), None)
                .unwrap()
                .is_none()
        );
        reopened.close().unwrap();
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
