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
    ControlledCommand, ControlledLocalWorkspace, ControlledRequest, ControlledWorkspace,
    controlled_database_config,
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
    inner: Mutex<Workspace>,
}
enum Workspace {
    File(ControlledWorkspace),
    Sql(ControlledLocalWorkspace),
}
impl Workspace {
    fn state(&self) -> serde_json::Value {
        match self {
            Self::File(w) => w.state(),
            Self::Sql(w) => w.state(),
        }
    }
    fn execute(&mut self, r: ControlledRequest) -> serde_json::Value {
        match self {
            Self::File(w) => w.execute(r),
            Self::Sql(w) => w.execute(r),
        }
    }
    fn prepare(
        &mut self,
        id: &str,
        target: &str,
        view: serde_json::Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> serde_json::Value {
        match self {
            Self::File(w) => w.prepare(id, target, view, epoch, generation),
            Self::Sql(w) => w.prepare(id, target, view, epoch, generation),
        }
    }
    fn confirm(&mut self, id: &str) -> serde_json::Value {
        match self {
            Self::File(w) => w.confirm(id),
            Self::Sql(w) => w.confirm(id),
        }
    }
    fn cancel(&mut self, id: &str) -> serde_json::Value {
        match self {
            Self::File(w) => w.cancel(id),
            Self::Sql(w) => w.cancel(id),
        }
    }
    fn retry(&mut self, id: &str) -> serde_json::Value {
        match self {
            Self::File(w) => w.retry(id),
            Self::Sql(w) => w.retry(id),
        }
    }
}
/// Shared app/CLI constructor restricted to the fixed controller-owned SQL config.
/// # Errors
/// Refuses arbitrary paths, scope/pin mismatch, competing ownership or corruption.
pub fn open_controlled_local_workspace(
    manifest: &Path,
    binding: serde_json::Value,
    targets: serde_json::Value,
    configuration: &Path,
) -> std::io::Result<ControlledLocalWorkspace> {
    let config = controlled_database_config(manifest, binding.clone(), configuration)?;
    let registration: photara_library::disposable::Registration =
        serde_json::from_value(config["registration"].clone()).map_err(std::io::Error::other)?;
    let pins: photara_library::disposable::Pins =
        serde_json::from_value(config["pins"].clone()).map_err(std::io::Error::other)?;
    let path = config["path"]
        .as_str()
        .ok_or_else(|| std::io::Error::other("database path missing"))?;
    let database =
        photara_library::disposable::LocalDatabase::reopen(Path::new(path), registration, pins)?;
    ControlledLocalWorkspace::open(manifest, binding, targets, Box::new(database))
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
            inner: Mutex::new(Workspace::File(inner)),
        }))
    }
    /// Opens the approved fresh SQL slot; existing PS4 file slots remain separate.
    /// # Errors
    /// Refuses unregistered paths, stale scope and competing ownership.
    #[uniffi::constructor]
    pub fn open_local(
        manifest_path: String,
        binding_json: String,
        target_bindings_json: String,
        database_registration_path: String,
    ) -> Result<Arc<Self>, ControlledSessionError> {
        if manifest_path.len() > 4096
            || database_registration_path.len() > 4096
            || binding_json.len() > 65536
            || target_bindings_json.len() > 65536
        {
            return Err(ControlledSessionError::AdmissionRefused);
        }
        let binding = serde_json::from_str(&binding_json)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        let targets = serde_json::from_str(&target_bindings_json)
            .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        let inner = open_controlled_local_workspace(
            Path::new(&manifest_path),
            binding,
            targets,
            Path::new(&database_registration_path),
        )
        .map_err(|_| ControlledSessionError::AdmissionRefused)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Workspace::Sql(inner)),
        }))
    }
    /// Selects a Library without implicitly opening a Project or showing a clean-switch sheet.
    /// # Errors
    /// Refuses unsupported route or malformed bounded inputs.
    pub fn select_library(
        &self,
        activation_id: String,
        library_id: String,
        view_json: String,
        expected_owner_epoch: Option<String>,
        expected_attachment_generation: Option<u64>,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        if activation_id.len() != 36 || library_id.len() != 36 || view_json.len() > 65536 {
            return Err(ControlledSessionError::InvalidRequest);
        }
        let view =
            serde_json::from_str(&view_json).map_err(|_| ControlledSessionError::InvalidRequest)?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| ControlledSessionError::Unavailable)?;
        let Workspace::Sql(w) = &mut *guard else {
            return Err(ControlledSessionError::InvalidRequest);
        };
        Ok(response(w.select_library(
            &activation_id,
            &library_id,
            view,
            expected_owner_epoch.as_deref(),
            expected_attachment_generation,
        )))
    }
    /// Returns to the current Library through a fully verified Project flush and SQL publication.
    /// # Errors
    /// Refuses unsupported scope or malformed bounded input.
    pub fn close_project(
        &self,
        activation_id: String,
        view_json: String,
        expected_owner_epoch: Option<String>,
        expected_attachment_generation: Option<u64>,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        if activation_id.len() != 36 || view_json.len() > 65_536 {
            return Err(ControlledSessionError::InvalidRequest);
        }
        let view =
            serde_json::from_str(&view_json).map_err(|_| ControlledSessionError::InvalidRequest)?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| ControlledSessionError::Unavailable)?;
        let Workspace::Sql(workspace) = &mut *guard else {
            return Err(ControlledSessionError::InvalidRequest);
        };
        Ok(response(workspace.close_project(
            &activation_id,
            view,
            expected_owner_epoch.as_deref(),
            expected_attachment_generation,
        )))
    }
    /// Creates using one immutable original request and current verified local controller.
    /// # Errors
    /// Refuses unsupported route or malformed input.
    pub fn create_library(
        &self,
        operation_id: String,
        library_id: String,
        name: String,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        self.library_command(operation_id, library_id, name, None)
    }
    /// Renames through exact revision CAS; retry retains original bytes.
    /// # Errors
    /// Refuses unsupported route or malformed input.
    pub fn rename_library(
        &self,
        operation_id: String,
        library_id: String,
        expected_revision: u64,
        name: String,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        self.library_command(operation_id, library_id, name, Some(expected_revision))
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

impl ControlledDisposableWorkspace {
    fn library_command(
        &self,
        id: String,
        library: String,
        name: String,
        revision: Option<u64>,
    ) -> Result<ControlledActivationResponse, ControlledSessionError> {
        if id.len() != 36 || library.len() != 36 || name.len() > 65536 {
            return Err(ControlledSessionError::InvalidRequest);
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| ControlledSessionError::Unavailable)?;
        let Workspace::Sql(w) = &mut *guard else {
            return Err(ControlledSessionError::InvalidRequest);
        };
        Ok(response(w.library_command(&id, &library, &name, revision)))
    }
}
