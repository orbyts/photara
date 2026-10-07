//! Explicit disposable admission, independent of Library/provider production routes.
use photara_store::package::v1_3::repeatable::disposable::{
    ControlledCommand, ControlledRequest, ControlledSession,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, uniffi::Enum)]
pub enum ControlledSessionCommand {
    Snapshot,
    Submit,
    Undo,
    Redo,
    Complete,
    Barrier,
    Close,
    Retry,
    Revalidate,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ControlledSessionRequest {
    pub id: String,
    pub command: ControlledSessionCommand,
    pub node_id: Option<String>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub expected_owner_epoch: Option<String>,
    pub expected_attachment_generation: Option<u64>,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct ControlledSessionResponse {
    pub id: String,
    pub snapshot_json: Option<String>,
    pub result_json: Option<String>,
    pub acknowledgement_json: Option<String>,
    pub error: Option<String>,
}
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ControlledSessionError {
    #[error("Controlled disposable admission refused")]
    AdmissionRefused,
    #[error("Session closed or unavailable")]
    Unavailable,
    #[error("Invalid controlled request")]
    InvalidRequest,
}
/// A serialized session owns its OS lock until successful close or handle drop.
/// Drop cannot acknowledge Saved and performs no implicit flush.
#[derive(uniffi::Object)]
pub struct ControlledDisposableSession {
    inner: Mutex<Option<ControlledSession>>,
}
#[uniffi::export]
impl ControlledDisposableSession {
    /// Opens only an independently verified controlled disposable image.
    /// # Errors
    /// Refuses malformed controller evidence, unsafe or unsupported storage,
    /// changed registration, and an existing writer lease.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI owns constructor arguments across the native boundary"
    )]
    #[uniffi::constructor]
    pub fn open(
        manifest_path: String,
        binding_json: String,
    ) -> Result<Arc<Self>, ControlledSessionError> {
        if manifest_path.len() > 4096 || binding_json.len() > 65536 {
            return Err(ControlledSessionError::AdmissionRefused);
        }
        let binding = serde_json::from_str(&binding_json)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        let inner = ControlledSession::open(Path::new(&manifest_path), binding)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Some(inner)),
        }))
    }
    /// Runs one request while retaining the exclusive package lease.
    /// # Errors
    /// Refuses invalid request bounds and closed or poisoned handles. Storage
    /// and checkpoint refusals are returned with the last proven snapshot.
    pub fn execute(
        &self,
        request: ControlledSessionRequest,
    ) -> Result<ControlledSessionResponse, ControlledSessionError> {
        if request.id.len() > 36
            || request.node_id.as_ref().is_some_and(|s| s.len() > 36)
            || request
                .expected_owner_epoch
                .as_ref()
                .is_some_and(|s| s.len() > 36)
        {
            return Err(ControlledSessionError::InvalidRequest);
        }
        let command = match request.command {
            ControlledSessionCommand::Snapshot => ControlledCommand::Snapshot,
            ControlledSessionCommand::Submit => ControlledCommand::Submit,
            ControlledSessionCommand::Undo => ControlledCommand::Undo,
            ControlledSessionCommand::Redo => ControlledCommand::Redo,
            ControlledSessionCommand::Complete => ControlledCommand::Complete,
            ControlledSessionCommand::Barrier => ControlledCommand::Barrier,
            ControlledSessionCommand::Close => ControlledCommand::Close,
            ControlledSessionCommand::Retry => ControlledCommand::Retry,
            ControlledSessionCommand::Revalidate => ControlledCommand::Revalidate,
        };
        let mut held = self
            .inner
            .lock()
            .map_err(|_| ControlledSessionError::Unavailable)?;
        let value = held
            .as_mut()
            .ok_or(ControlledSessionError::Unavailable)?
            .execute(ControlledRequest {
                id: request.id.clone(),
                command,
                node_id: request.node_id,
                x: request.x,
                y: request.y,
                expected_owner_epoch: request.expected_owner_epoch,
                expected_attachment_generation: request.expected_attachment_generation,
            });
        let field = |name: &str| {
            if value[name].is_null() {
                None
            } else {
                Some(value[name].to_string())
            }
        };
        let response = ControlledSessionResponse {
            id: request.id,
            snapshot_json: field("snapshot"),
            result_json: field("result"),
            acknowledgement_json: field("acknowledgement"),
            error: value["error"].as_str().map(str::to_owned),
        };
        if value["snapshot"]["closed"] == true && response.error.is_none() {
            held.take();
        }
        Ok(response)
    }
}
