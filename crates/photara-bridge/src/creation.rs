//! Native UI1 adapter. Only local database paths and public cloud command bytes cross FFI.
// UniFFI exports own strings/byte buffers across the language boundary.
#![allow(clippy::needless_pass_by_value)]
use crate::BridgeError;
use photara_core::{
    contracts as ct,
    creation::{InitialProject, project_name},
};
use photara_library::gen2::{
    CreateProjectCloudReceipt, CreateProjectCommand, CreateProjectRequest, CreationActor,
    LocalIdentity, LocalLibraryStore, ProjectCreation, Timestamp, initial_creation_package,
};
use photara_store::package::creation::DirectoryPin;
use std::result::Result;
use std::{
    future::Future,
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

fn fail(error: impl std::fmt::Display) -> BridgeError {
    BridgeError::Store {
        message: format!("Project creation: {error}."),
    }
}
fn now() -> Result<Timestamp, BridgeError> {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(fail)?;
    Timestamp::try_from(i64::try_from(t.as_millis()).map_err(fail)?).map_err(fail)
}
fn uuid(value: &str) -> Result<Uuid, BridgeError> {
    let id = Uuid::parse_str(value).map_err(fail)?;
    if id.is_nil() {
        return Err(fail("invalid identity"));
    }
    Ok(id)
}

/// Pin the chosen host directory before the user submits the sheet.
/// # Errors
/// Refuses missing, symlinked and inaccessible directories.
#[uniffi::export]
pub fn inspect_creation_destination(path: String) -> Result<String, BridgeError> {
    let pin = DirectoryPin::inspect(Path::new(&path)).map_err(fail)?;
    String::from_utf8(photara_core::canonical_json(&pin).map_err(fail)?).map_err(fail)
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeProjectCreation {
    pub operation_id: String,
    pub library_id: String,
    pub project_id: String,
    pub graph_id: String,
    pub title: String,
    pub package_path: String,
    pub commit_sha256: String,
    pub state: String,
    pub is_cloud: bool,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub environment: Option<String>,
    pub device_id: String,
}
fn dto(c: ProjectCreation) -> Result<BridgeProjectCreation, BridgeError> {
    let p = &c.request.command.initial;
    let (issuer, subject, environment) = match &c.request.actor {
        CreationActor::Local { .. } => (None, None, None),
        CreationActor::Cloud {
            issuer,
            subject,
            environment,
            ..
        } => (
            Some(issuer.clone()),
            Some(subject.clone()),
            Some(environment.clone()),
        ),
    };
    Ok(BridgeProjectCreation {
        operation_id: p.operation_id.to_string(),
        library_id: p.library_id.to_string(),
        project_id: p.project_id.to_string(),
        graph_id: p.graph_id.to_string(),
        title: p.title.clone(),
        package_path: c
            .package_path()
            .to_str()
            .ok_or_else(|| fail("invalid path"))?
            .into(),
        commit_sha256: initial_creation_package(p.clone())
            .map_err(fail)?
            .commit_hex,
        state: serde_json::to_value(c.state)
            .map_err(fail)?
            .as_str()
            .ok_or_else(|| fail("invalid state"))?
            .into(),
        is_cloud: issuer.is_some(),
        issuer,
        subject,
        environment,
        device_id: c.request.command.device_id.to_string(),
    })
}
#[derive(uniffi::Object)]
pub struct PhotaraProjectCreation {
    runtime: Mutex<tokio::runtime::Runtime>,
    store: LocalLibraryStore,
    identity: LocalIdentity,
}
impl PhotaraProjectCreation {
    fn run<T>(
        &self,
        future: impl Future<Output = photara_library::gen2::Result<T>>,
    ) -> Result<T, BridgeError> {
        self.runtime
            .lock()
            .map_err(|_| fail("busy"))?
            .block_on(future)
            .map_err(fail)
    }
}
#[uniffi::export]
impl PhotaraProjectCreation {
    /// # Errors
    /// Refuses foreign or unsupported local state.
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, BridgeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(fail)?;
        let (store, identity) = runtime
            .block_on(LocalLibraryStore::open_app_state(path, now()?))
            .map_err(fail)?;
        Ok(Arc::new(Self {
            runtime: Mutex::new(runtime),
            store,
            identity,
        }))
    }
    /// # Errors
    /// Invalid package names return native-readable diagnostics.
    pub fn validate_name(&self, name: String) -> Result<String, BridgeError> {
        project_name(&name).map_err(fail)
    }
    /// # Errors
    /// Refuses changed input, unavailable destinations and wrong Library authority.
    pub fn prepare(
        &self,
        operation_id: String,
        title: String,
        destination: String,
        destination_pin: Option<String>,
        created_at: String,
    ) -> Result<BridgeProjectCreation, BridgeError> {
        let id = uuid(&operation_id)?;
        let title = project_name(&title).map_err(fail)?;
        let pin = DirectoryPin::inspect(Path::new(&destination)).map_err(fail)?;
        if let Some(expected) = destination_pin {
            let expected: DirectoryPin =
                ct::schema::decode_strict(expected.as_bytes(), 8192).map_err(fail)?;
            if expected != pin {
                return Err(fail(
                    "the selected destination changed; choose its current location again",
                ));
            }
        }
        for existing in self.run(self.store.project_creations())? {
            if existing.request.command.initial.operation_id.uuid() == id {
                if existing.request.command.initial.title != title
                    || existing.request.destination != pin
                {
                    return Err(fail("operation input changed"));
                }
                return dto(existing);
            }
        }
        let mut library = self.identity.library_id;
        let actor = if let Some(binding) = self.run(self.store.cached_cloud_library(library))? {
            let binding = self
                .run(self.store.selected_cloud_library(&binding.principal))?
                .unwrap_or(binding);
            if binding.state != "cached" {
                return Err(fail("sign in to the selected cloud Library"));
            }
            library = binding.library;
            CreationActor::Cloud {
                account: binding.account_id,
                issuer: binding.principal.issuer,
                subject: binding.principal.subject,
                environment: binding.environment_id,
            }
        } else {
            CreationActor::Local {
                principal: self.identity.principal_id,
            }
        };
        let initial = InitialProject {
            operation_id: ct::OperationId::from_uuid(id).map_err(fail)?,
            library_id: ct::LibraryId::from_uuid(library.uuid()).map_err(fail)?,
            project_id: ct::ProjectId::from_uuid(Uuid::new_v4()).map_err(fail)?,
            graph_id: ct::GraphId::from_uuid(Uuid::new_v4()).map_err(fail)?,
            commit_id: ct::CommitId::from_uuid(Uuid::new_v4()).map_err(fail)?,
            title,
            created_at,
        };
        dto(self.run(self.store.prepare_project_creation(
            CreateProjectRequest {
                command: CreateProjectCommand {
                    initial,
                    device_id: self.identity.device_id.uuid(),
                    locator_id: Uuid::new_v4(),
                    observation_id: Uuid::new_v4(),
                },
                destination: pin,
                actor,
            },
            now()?,
        ))?)
    }
    /// # Errors
    /// Reports incomplete work without replacing its identity.
    pub fn advance(&self, operation_id: String) -> Result<BridgeProjectCreation, BridgeError> {
        dto(self.run(
            self.store
                .advance_project_creation(uuid(&operation_id)?, now()?),
        )?)
    }
    /// # Errors
    /// Returns damaged journal errors, never invokes authentication.
    pub fn operations(&self) -> Result<Vec<BridgeProjectCreation>, BridgeError> {
        self.run(self.store.project_creations())?
            .into_iter()
            .map(dto)
            .collect()
    }
    /// # Errors
    /// Requires the staged matching cloud operation.
    pub fn dispatch(&self, operation_id: String) -> Result<Vec<u8>, BridgeError> {
        photara_core::canonical_json(
            &self.run(self.store.dispatch_project_creation(uuid(&operation_id)?))?,
        )
        .map_err(fail)
    }
    /// # Errors
    /// Rejects changed, foreign or malformed authenticated service receipts.
    pub fn receive(&self, operation_id: String, receipt: Vec<u8>) -> Result<(), BridgeError> {
        let value: CreateProjectCloudReceipt =
            ct::schema::decode_strict(&receipt, 65536).map_err(fail)?;
        if photara_core::canonical_json(&value).map_err(fail)? != receipt {
            return Err(fail("noncanonical receipt"));
        }
        self.run(
            self.store
                .receive_project_creation(uuid(&operation_id)?, value),
        )
    }
    /// # Errors
    /// Dispatch/publication cannot be falsely cancelled.
    pub fn cancel(&self, operation_id: String) -> Result<(), BridgeError> {
        self.run(self.store.cancel_project_creation(uuid(&operation_id)?))
    }
    /// # Errors
    /// Reports a poisoned runtime lock.
    pub fn close(&self) -> Result<(), BridgeError> {
        self.runtime
            .lock()
            .map_err(|_| fail("busy"))?
            .block_on(self.store.close());
        Ok(())
    }
}
