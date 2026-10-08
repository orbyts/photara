//! Same-Library disposable switching. All durable authority remains in Rust store.
#![expect(
    clippy::needless_pass_by_value,
    reason = "UniFFI transfers owned request and response buffers across the native boundary"
)]
use crate::controlled_session::{
    ControlledSessionCommand, ControlledSessionError, ControlledSessionRequest,
    ControlledSessionResponse,
};
use photara_store::package::v1_3::repeatable::disposable::{
    ControlledCommand, ControlledRequest, ControlledWorkspace,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, uniffi::Record)]
pub struct ControlledActivationResponse {
    pub status: String,
    pub activation_id: Option<String>,
    pub state_json: String,
}
#[derive(uniffi::Object)]
pub struct ControlledDisposableWorkspace {
    inner: Mutex<ControlledWorkspace>,
}
fn response(value: serde_json::Value) -> ControlledActivationResponse {
    ControlledActivationResponse {
        status: value["status"].as_str().unwrap_or("Refused").to_owned(),
        activation_id: value["activation_id"].as_str().map(str::to_owned),
        state_json: value.to_string(),
    }
}
#[uniffi::export]
impl ControlledDisposableWorkspace {
    /// Opens only an independently registered controller-owned activation slot.
    /// # Errors
    /// Refuses unregistered scopes, unsafe storage and competing slot ownership.
    #[uniffi::constructor]
    pub fn open(
        manifest_path: String,
        binding_json: String,
        target_bindings_json: String,
    ) -> Result<Arc<Self>, ControlledSessionError> {
        if manifest_path.len() > 4096
            || binding_json.len() > 65536
            || target_bindings_json.len() > 65536
        {
            return Err(ControlledSessionError::AdmissionRefused);
        }
        let binding = serde_json::from_str(&binding_json)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        let targets = serde_json::from_str(&target_bindings_json)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        let inner = ControlledWorkspace::open(Path::new(&manifest_path), binding, targets)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(inner),
        }))
    }
    /// # Errors
    /// Refuses a poisoned coordinator.
    pub fn state(&self) -> Result<ControlledActivationResponse, ControlledSessionError> {
        Ok(response(
            self.inner
                .lock()
                .map_err(|_| ControlledSessionError::Unavailable)?
                .state(),
        ))
    }
    /// Flushes current work before returning evidence suitable for confirmation.
    /// # Errors
    /// Refuses malformed arguments or unavailable coordinator; storage refusals
    /// retain the current state and are returned in the response.
    pub fn prepare(
        &self,
        activation_id: String,
        target_project_id: String,
        source_view_json: String,
        expected_owner_epoch: Option<String>,
        expected_attachment_generation: Option<u64>,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        if activation_id.len() != 36
            || target_project_id.len() != 36
            || source_view_json.len() > 65536
        {
            return Err(ControlledSessionError::InvalidRequest);
        }
        let view = serde_json::from_str(&source_view_json)
            .map_err(|_| ControlledSessionError::InvalidRequest)?;
        Ok(response(
            self.inner
                .lock()
                .map_err(|_| ControlledSessionError::Unavailable)?
                .prepare(
                    &activation_id,
                    &target_project_id,
                    view,
                    expected_owner_epoch.as_deref(),
                    expected_attachment_generation,
                ),
        ))
    }
    /// # Errors
    /// Refuses an unavailable coordinator; confirmation never changes the original target.
    pub fn confirm(
        &self,
        activation_id: String,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        Ok(response(
            self.inner
                .lock()
                .map_err(|_| ControlledSessionError::Unavailable)?
                .confirm(&activation_id),
        ))
    }
    /// # Errors
    /// Refuses an unavailable coordinator.
    pub fn cancel(
        &self,
        activation_id: String,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        Ok(response(
            self.inner
                .lock()
                .map_err(|_| ControlledSessionError::Unavailable)?
                .cancel(&activation_id),
        ))
    }
    /// # Errors
    /// Refuses an unavailable coordinator. Retry preserves original identity.
    pub fn retry(
        &self,
        activation_id: String,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        Ok(response(
            self.inner
                .lock()
                .map_err(|_| ControlledSessionError::Unavailable)?
                .retry(&activation_id),
        ))
    }
    /// # Errors
    /// Refuses unavailable or malformed requests; pending switches block mutation.
    pub fn execute(
        &self,
        request: ControlledSessionRequest,
    ) -> Result<ControlledSessionResponse, ControlledSessionError> {
        if request.id.len() != 36 || request.node_id.as_ref().is_some_and(|s| s.len() != 36) {
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
        let id = request.id.clone();
        let value = self
            .inner
            .lock()
            .map_err(|_| ControlledSessionError::Unavailable)?
            .execute(ControlledRequest {
                id: request.id,
                command,
                node_id: request.node_id,
                x: request.x,
                y: request.y,
                expected_owner_epoch: request.expected_owner_epoch,
                expected_attachment_generation: request.expected_attachment_generation,
            });
        let field = |name: &str| (!value[name].is_null()).then(|| value[name].to_string());
        Ok(ControlledSessionResponse {
            id,
            snapshot_json: field("snapshot"),
            result_json: field("result"),
            acknowledgement_json: field("acknowledgement"),
            error: value["error"].as_str().map(str::to_owned),
        })
    }
}
