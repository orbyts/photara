//! UI1 durable creation coordinator. `SQLite` serializes transitions across processes.
use super::*;
use photara_core::{
    contracts::LocalPrincipalId,
    creation::{InitialProject, PackageExtension},
};
use photara_store::package::{
    PackageLimits,
    creation::{DirectoryPin, InitialPackage},
    v1_1,
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectCommand {
    pub initial: InitialProject,
    pub device_id: Uuid,
    pub locator_id: Uuid,
    pub observation_id: Uuid,
}
pub struct InitialCreationPackage {
    pub commit_sha256: [u8; 32],
    pub commit_hex: String,
}
/// Compute portable initial package evidence without filesystem access.
/// # Errors
/// Rejects invalid authored coordinates.
pub fn initial_creation_package(initial: InitialProject) -> Result<InitialCreationPackage> {
    let p = InitialPackage::build(initial).map_err(|_| Error::Invalid)?;
    Ok(InitialCreationPackage {
        commit_sha256: super::catalog::unhex(p.commit_sha256.as_str())?,
        commit_hex: p.commit_sha256.as_str().into(),
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CreationActor {
    Local {
        principal: LocalPrincipalId,
    },
    Cloud {
        account: Uuid,
        issuer: String,
        subject: String,
        environment: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectRequest {
    // Local-only immutable naming policy. Missing field means a pre-BR0 request;
    // never reinterpret an interrupted operation using a new brand's extension.
    #[serde(
        default = "PackageExtension::legacy_creation_alias",
        skip_serializing_if = "PackageExtension::is_legacy_creation_alias"
    )]
    pub package_extension: PackageExtension,
    pub command: CreateProjectCommand,
    pub destination: DirectoryPin,
    pub actor: CreationActor,
}
/// Immutable service evidence; host paths and credentials are deliberately absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectCloudReceipt {
    pub command_sha256: String,
    pub operation_id: Uuid,
    pub account_id: Uuid,
    pub account_name: String,
    pub account_revision: i64,
    pub grant_id: Uuid,
    pub generation: i64,
    pub commit_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CreationState {
    Prepared,
    Staged,
    Dispatching,
    RemoteCommitted,
    Published,
    Complete,
    Cancelling,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectCreation {
    pub request: CreateProjectRequest,
    pub state: CreationState,
    pub stage: Option<DirectoryPin>,
    pub cloud_receipt: Option<CreateProjectCloudReceipt>,
}
impl ProjectCreation {
    #[must_use]
    pub fn package_path(&self) -> PathBuf {
        // Request admission and persisted-request decoding validate the complete
        // filename before any filesystem operation. This accessor is a locator.
        self.request.destination.path.join(format!(
            "{}.{}",
            self.request.command.initial.title,
            self.request.package_extension.as_str()
        ))
    }
}
fn package(request: &CreateProjectRequest) -> Result<InitialPackage> {
    request
        .package_extension
        .filename(&request.command.initial.title)
        .map_err(|_| Error::Invalid)?;
    InitialPackage::build(request.command.initial.clone()).map_err(|_| Error::Invalid)
}
fn bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}
fn library(request: &CreateProjectRequest) -> Result<LibraryId> {
    LibraryId::try_from(request.command.initial.library_id.uuid())
}
fn operation(request: &CreateProjectRequest) -> Vec<u8> {
    bytes(request.command.initial.operation_id.uuid())
}

async fn authority(tx: &mut SqliteConnection, request: &CreateProjectRequest) -> Result<()> {
    let l = library(request)?;
    match &request.actor {
        CreationActor::Local { principal } => {
            super::local::local_authority(tx, l, *principal).await
        }
        CreationActor::Cloud {
            account,
            issuer,
            subject,
            environment,
        } => {
            let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM library_cloud_bindings b JOIN library_contract_state c USING(library_id) JOIN libraries l USING(library_id) WHERE b.library_id=? AND b.account_id=? AND b.issuer=? AND b.subject=? AND b.environment_id=? AND b.state='cached' AND c.authority_mode='cloud-member' AND l.state='active')").bind(l.bytes()).bind(bytes(*account)).bind(issuer).bind(subject).bind(environment).fetch_one(&mut *tx).await?;
            if allowed { Ok(()) } else { Err(Error::Invalid) }
        }
    }
}
async fn read(tx: &mut SqliteConnection, id: Uuid) -> Result<Option<ProjectCreation>> {
    let row = sqlx::query("SELECT * FROM project_creation_intents WHERE operation_id=?")
        .bind(bytes(id))
        .fetch_optional(&mut *tx)
        .await?;
    row.map(|r| {
        let command: String = r.try_get("request_canonical")?;
        if sha(command.as_bytes()).as_slice() != r.try_get::<Vec<u8>, _>("request_sha256")? {
            return Err(Error::Corrupt);
        }
        let parse = |s: String| serde_json::from_str(&s).map_err(|_| Error::Corrupt);
        let request: CreateProjectRequest = parse(command)?;
        package(&request).map_err(|_| Error::Corrupt)?;
        Ok(ProjectCreation {
            request,
            state: serde_json::from_value(serde_json::json!(r.try_get::<String, _>("state")?))
                .map_err(|_| Error::Corrupt)?,
            stage: r
                .try_get::<Option<String>, _>("stage_pin")?
                .map(|s| serde_json::from_str(&s).map_err(|_| Error::Corrupt))
                .transpose()?,
            cloud_receipt: r
                .try_get::<Option<String>, _>("cloud_receipt")?
                .map(|s| serde_json::from_str(&s).map_err(|_| Error::Corrupt))
                .transpose()?,
        })
    })
    .transpose()
}
impl LocalLibraryStore {
    /// Persist an immutable request before any filesystem/network effects.
    /// # Errors
    /// Rejects authority, invalid package/destination, identity or operation collisions.
    pub async fn prepare_project_creation(
        &self,
        request: CreateProjectRequest,
        at: Timestamp,
    ) -> Result<ProjectCreation> {
        let _ = package(&request)?;
        if request.command.device_id != self.info.device_id.uuid()
            || request.command.locator_id.is_nil()
            || request.command.observation_id.is_nil()
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.write().await?;
        authority(&mut tx, &request).await?;
        if let Some(existing) = read(&mut tx, request.command.initial.operation_id.uuid()).await? {
            if existing.request != request {
                return Err(Error::Conflict);
            }
            return Ok(existing);
        }
        if DirectoryPin::inspect(&request.destination.path).map_err(Error::from)?
            != request.destination
        {
            return Err(Error::Conflict);
        }
        let path = request.destination.path.join(
            request
                .package_extension
                .filename(&request.command.initial.title)
                .map_err(|_| Error::Invalid)?,
        );
        if std::fs::symlink_metadata(&path).is_ok() {
            return Err(Error::Conflict);
        }
        let collision:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_ownership WHERE project_id=?) OR EXISTS(SELECT 1 FROM project_catalog WHERE project_id=?)").bind(bytes(request.command.initial.project_id.uuid())).bind(bytes(request.command.initial.project_id.uuid())).fetch_one(&mut *tx).await?;
        if collision {
            return Err(Error::Conflict);
        }
        let body = canonical(&request)?;
        let key = format!(
            "{}:{}:{}",
            request.destination.device,
            request.destination.inode,
            normalize_term(&request.command.initial.title)?
        );
        sqlx::query(
            "INSERT INTO project_creation_intents VALUES(?,?,?,?,?,?,'prepared',NULL,NULL,?)",
        )
        .bind(operation(&request))
        .bind(library(&request)?.bytes())
        .bind(bytes(request.command.initial.project_id.uuid()))
        .bind(&body)
        .bind(sha(body.as_bytes()).to_vec())
        .bind(key)
        .bind(at.get())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ProjectCreation {
            request,
            state: CreationState::Prepared,
            stage: None,
            cloud_receipt: None,
        })
    }
    /// Reads retained operations for restart/recovery without invoking authentication.
    /// # Errors
    /// Rejects damaged journals or database errors.
    pub async fn project_creations(&self) -> Result<Vec<ProjectCreation>> {
        let mut tx = self.read().await?;
        let ids: Vec<Vec<u8>> = sqlx::query_scalar(
            "SELECT operation_id FROM project_creation_intents ORDER BY created_at_ms,operation_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut out = Vec::new();
        for id in ids {
            out.push(
                read(&mut tx, Uuid::from_slice(&id).map_err(|_| Error::Corrupt)?)
                    .await?
                    .ok_or(Error::Corrupt)?,
            );
        }
        Ok(out)
    }
    /// Advances local staging/publication. Cloud dispatch is explicitly separate
    /// and must journal Dispatching before the native HTTP adapter sends bytes.
    /// # Errors
    /// Failures retain the intent; no unrelated destination is overwritten.
    pub async fn advance_project_creation(
        &self,
        id: Uuid,
        at: Timestamp,
    ) -> Result<ProjectCreation> {
        let mut tx = self.write().await?;
        let mut c = read(&mut tx, id).await?.ok_or(Error::Invalid)?;
        if c.state == CreationState::Cancelled {
            return Ok(c);
        }
        if c.state == CreationState::Cancelling {
            tx.rollback().await?;
            self.cancel_project_creation(id).await?;
            let mut tx = self.read().await?;
            return read(&mut tx, id).await?.ok_or(Error::Corrupt);
        }
        authority(&mut tx, &c.request).await?;
        let p = package(&c.request)?;
        if c.stage.is_none() {
            let stage = c
                .request
                .destination
                .create_stage(&p)
                .map_err(Error::from)?;
            sqlx::query("UPDATE project_creation_intents SET stage_pin=? WHERE operation_id=?")
                .bind(canonical(&stage)?)
                .bind(bytes(id))
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            // The pin is durable before package writes. All later calls resume it.
            tx = self.write().await?;
            c = read(&mut tx, id).await?.ok_or(Error::Corrupt)?;
            if matches!(
                c.state,
                CreationState::Cancelled | CreationState::Cancelling
            ) {
                tx.rollback().await?;
                self.cancel_project_creation(id).await?;
                let mut tx = self.read().await?;
                return read(&mut tx, id).await?.ok_or(Error::Corrupt);
            }
            authority(&mut tx, &c.request).await?;
        }
        let stage = c.stage.as_ref().ok_or(Error::Corrupt)?;
        if c.state == CreationState::Prepared {
            stage.materialize(&p).map_err(Error::from)?;
            sqlx::query("UPDATE project_creation_intents SET state='staged' WHERE operation_id=?")
                .bind(bytes(id))
                .execute(&mut *tx)
                .await?;
            c.state = CreationState::Staged;
        }
        if matches!(c.request.actor, CreationActor::Cloud { .. }) && c.cloud_receipt.is_none() {
            tx.commit().await?;
            return Ok(c);
        }
        if c.state != CreationState::Complete {
            let path = c.package_path();
            match std::fs::symlink_metadata(&path) {
                Ok(_) => {
                    // Reconcile a crash after exclusive rename, never adopt by title.
                    let pin = DirectoryPin::inspect(&path).map_err(Error::from)?;
                    if pin.device != stage.device || pin.inode != stage.inode {
                        return Err(Error::Conflict);
                    }
                    verify_created(&c, &p)?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    c.request
                        .destination
                        .publish(stage, &p, &c.request.package_extension)
                        .map_err(Error::from)?;
                }
                Err(_) => return Err(Error::Io),
            }
            sqlx::query(
                "UPDATE project_creation_intents SET state='published' WHERE operation_id=?",
            )
            .bind(bytes(id))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            tx = self.write().await?;
            c = read(&mut tx, id).await?.ok_or(Error::Corrupt)?;
            authority(&mut tx, &c.request).await?;
            verify_created(&c, &p)?;
            if c.state != CreationState::Complete {
                install_catalog(&mut tx, self.info.device_id, &c, &p, at).await?;
                sqlx::query(
                    "UPDATE project_creation_intents SET state='complete' WHERE operation_id=?",
                )
                .bind(bytes(id))
                .execute(&mut *tx)
                .await?;
                c.state = CreationState::Complete;
            }
        }
        verify_created(&c, &p)?;
        tx.commit().await?;
        Ok(c)
    }
    /// Persist unknown-outcome state before handing immutable command bytes to HTTP.
    /// # Errors
    /// Requires a staged cloud operation and the current matching native binding.
    pub async fn dispatch_project_creation(&self, id: Uuid) -> Result<CreateProjectCommand> {
        let mut tx = self.write().await?;
        let c = read(&mut tx, id).await?.ok_or(Error::Invalid)?;
        authority(&mut tx, &c.request).await?;
        if !matches!(c.request.actor, CreationActor::Cloud { .. })
            || !matches!(c.state, CreationState::Staged | CreationState::Dispatching)
        {
            return Err(Error::Invalid);
        }
        sqlx::query("UPDATE project_creation_intents SET state='dispatching' WHERE operation_id=?")
            .bind(bytes(id))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(c.request.command)
    }
    /// Apply exact service receipt only through the authenticated native adapter.
    /// # Errors
    /// Rejects wrong command, actor, commit, or changed receipt bytes.
    pub async fn receive_project_creation(
        &self,
        id: Uuid,
        receipt: CreateProjectCloudReceipt,
    ) -> Result<()> {
        let mut tx = self.write().await?;
        let c = read(&mut tx, id).await?.ok_or(Error::Invalid)?;
        authority(&mut tx, &c.request).await?;
        let CreationActor::Cloud { account, .. } = c.request.actor else {
            return Err(Error::Invalid);
        };
        let p = package(&c.request)?;
        if receipt.operation_id != id
            || receipt.account_id != account
            || receipt.command_sha256 != hex(&sha(canonical(&c.request.command)?.as_bytes()))
            || receipt.commit_sha256 != p.commit_sha256.as_str()
            || receipt.grant_id.is_nil()
            || receipt.generation < 1
            || receipt.account_revision < 1
        {
            return Err(Error::Invalid);
        }
        if let Some(previous) = c.cloud_receipt {
            return if previous == receipt {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        if c.state != CreationState::Dispatching {
            return Err(Error::Invalid);
        }
        sqlx::query("UPDATE project_creation_intents SET state='remote-committed',cloud_receipt=? WHERE operation_id=?").bind(canonical(&receipt)?).bind(bytes(id)).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Cancel only before publication/dispatch. Unknown remote outcomes must retry.
    /// # Errors
    /// Reports noncancellable or incomplete cleanup without fabricating cancellation.
    pub async fn cancel_project_creation(&self, id: Uuid) -> Result<()> {
        let mut tx = self.write().await?;
        let c = read(&mut tx, id).await?.ok_or(Error::Invalid)?;
        if c.state == CreationState::Cancelled {
            return Ok(());
        }
        if !matches!(
            c.state,
            CreationState::Prepared | CreationState::Staged | CreationState::Cancelling
        ) {
            return Err(Error::Conflict);
        }
        if c.state != CreationState::Cancelling {
            sqlx::query(
                "UPDATE project_creation_intents SET state='cancelling' WHERE operation_id=?",
            )
            .bind(bytes(id))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            tx = self.write().await?;
        }
        if let Some(stage) = &c.stage {
            if DirectoryPin::inspect(&c.request.destination.path).map_err(Error::from)?
                != c.request.destination
            {
                return Err(Error::DestinationChanged);
            }
            match std::fs::symlink_metadata(&stage.path) {
                Ok(_) => stage
                    .discard_stage(&c.request.destination, &package(&c.request)?)
                    .map_err(Error::from)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if std::fs::symlink_metadata(c.package_path()).is_ok() {
                        return Err(Error::Conflict);
                    }
                }
                Err(_) => return Err(Error::Io),
            }
        }
        sqlx::query("UPDATE project_creation_intents SET state='cancelled' WHERE operation_id=?")
            .bind(bytes(id))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn verify_created(c: &ProjectCreation, p: &InitialPackage) -> Result<()> {
    if DirectoryPin::inspect(&c.request.destination.path).map_err(Error::from)?
        != c.request.destination
    {
        return Err(Error::Conflict);
    }
    let v = v1_1::validate_directory(c.package_path(), PackageLimits::default())
        .map_err(Error::from)?;
    if v.head.commit_sha256 != p.commit_sha256
        || v.head.project_id.as_uuid() != p.spec.project_id.uuid()
        || v.graphs.len() != 1
        || v.graphs[0].value["graph_id"] != p.spec.graph_id.to_string()
    {
        return Err(Error::Corrupt);
    }
    Ok(())
}
async fn install_catalog(
    tx: &mut SqliteConnection,
    device: DeviceId,
    c: &ProjectCreation,
    p: &InitialPackage,
    at: Timestamp,
) -> Result<()> {
    let command = &c.request.command;
    let l = library(&c.request)?;
    let project = ProjectId::try_from(p.spec.project_id.uuid())?;
    let registered = RegisteredProject {
        library_id: l,
        project_id: project,
        association_commit_id: CommitId::try_from(p.spec.commit_id.uuid())?,
        association_sha256: super::catalog::unhex(p.commit_sha256.as_str())?,
        source_origin_library_id: None,
    };
    match &c.request.actor {
        CreationActor::Local { principal } => {
            super::local::register_local_project(tx, device, *principal, &registered, at).await?;
        }
        CreationActor::Cloud { account, .. } => {
            let receipt = c.cloud_receipt.as_ref().ok_or(Error::Invalid)?;
            sqlx::query("INSERT INTO account_cache VALUES(?,?,'active',?,?) ON CONFLICT(account_id) DO NOTHING").bind(bytes(*account)).bind(&receipt.account_name).bind(receipt.account_revision.to_string()).bind(at.get()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO project_ownership VALUES(?,?,'active',?,?,NULL,'photara.package.v1.1',1,1,?,?)").bind(project.bytes()).bind(l.bytes()).bind(registered.association_commit_id.bytes()).bind(registered.association_sha256.to_vec()).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
            sqlx::query(
                "INSERT INTO project_access_policies VALUES(?,?,'restricted',0,0,0,0,?,1,1,?,?)",
            )
            .bind(l.bytes())
            .bind(project.bytes())
            .bind(receipt.generation)
            .bind(at.get())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO project_access_grants VALUES(?,?,?,?,NULL,255,'active',NULL,1,1,?,?)",
            )
            .bind(bytes(receipt.grant_id))
            .bind(l.bytes())
            .bind(project.bytes())
            .bind(bytes(*account))
            .bind(at.get())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO project_catalog(library_id,project_id,visibility,local_revision,created_at_ms,updated_at_ms) VALUES(?,?,'visible',1,?,?)").bind(l.bytes()).bind(project.bytes()).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
        }
    }
    sqlx::query("INSERT INTO project_locators VALUES(?,?,?,NULL,NULL,1,?,?,'active',NULL)")
        .bind(bytes(command.locator_id))
        .bind(l.bytes())
        .bind(project.bytes())
        .bind(at.get())
        .bind(at.get())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO project_observations VALUES(?,?,?,?,?,?,'1',?,'active',0,1,?,1)")
        .bind(bytes(command.observation_id))
        .bind(l.bytes())
        .bind(project.bytes())
        .bind(bytes(command.locator_id))
        .bind(registered.association_commit_id.bytes())
        .bind(registered.association_sha256.to_vec())
        .bind(&p.spec.title)
        .bind(at.get())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO project_graph_projection(observation_id,graph_id,graph_name,graph_digest,graph_revision) VALUES(?,?,'Graph 1',?,'0')").bind(bytes(command.observation_id)).bind(bytes(p.spec.graph_id.uuid())).bind(super::catalog::unhex(p.graph_sha256.as_str())?.to_vec()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO device_project_bindings(device_id,library_id,project_id,locator_id,direct_host_path,availability,verified_project_id,last_commit_id,last_commit_sha256,checked_at_ms) VALUES(?,?,?,?,?,'available',?,?,?,?)").bind(device.bytes()).bind(l.bytes()).bind(project.bytes()).bind(bytes(command.locator_id)).bind(c.package_path().to_str().ok_or(Error::Invalid)?).bind(project.bytes()).bind(registered.association_commit_id.bytes()).bind(registered.association_sha256.to_vec()).bind(at.get()).execute(&mut *tx).await?;
    sqlx::query("UPDATE project_catalog SET active_locator_id=?,selected_observation_id=?,local_revision=local_revision+1 WHERE library_id=? AND project_id=?").bind(bytes(command.locator_id)).bind(bytes(command.observation_id)).bind(l.bytes()).bind(project.bytes()).execute(&mut *tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use photara_core::contracts as ct;
    fn at() -> Timestamp {
        Timestamp::try_from(1_789_387_200_000).unwrap()
    }
    fn request(id: &LocalIdentity, destination: &Path) -> CreateProjectRequest {
        CreateProjectRequest {
            package_extension: PackageExtension::legacy_creation_alias(),
            command: CreateProjectCommand {
                initial: InitialProject {
                    operation_id: ct::OperationId::from_uuid(Uuid::new_v4()).unwrap(),
                    library_id: ct::LibraryId::from_uuid(id.library_id.uuid()).unwrap(),
                    project_id: ct::ProjectId::from_uuid(Uuid::new_v4()).unwrap(),
                    graph_id: ct::GraphId::from_uuid(Uuid::new_v4()).unwrap(),
                    commit_id: ct::CommitId::from_uuid(Uuid::new_v4()).unwrap(),
                    title: "Created Project".into(),
                    created_at: "2026-09-14T12:00:00.000Z".into(),
                },
                device_id: id.device_id.uuid(),
                locator_id: Uuid::new_v4(),
                observation_id: Uuid::new_v4(),
            },
            destination: DirectoryPin::inspect(destination).unwrap(),
            actor: CreationActor::Local {
                principal: id.principal_id,
            },
        }
    }
    #[tokio::test]
    async fn local_creation_restarts_with_one_catalog_graph_and_receipt() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = root.path().join("state.sqlite");
        let (store, id) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let req = request(&id, root.path());
        let op = req.command.initial.operation_id.uuid();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        let done = store.advance_project_creation(op, at()).await.unwrap();
        assert_eq!(done.state, CreationState::Complete);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM project_catalog")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM project_graph_projection")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
        store.verify_integrity().await.unwrap();
        store.close().await;
        let (store, again) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        assert_eq!(id, again);
        assert_eq!(
            store
                .advance_project_creation(op, at())
                .await
                .unwrap()
                .request,
            req
        );
        assert_eq!(store.project_creations().await.unwrap().len(), 1);
        let mut changed = req;
        changed.command.initial.title = "Changed".into();
        assert_eq!(
            store
                .prepare_project_creation(changed, at())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        store.close().await;
    }
    #[tokio::test]
    async fn synthetic_extension_restarts_and_legacy_request_keeps_original_policy() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = root.path().join("state.sqlite");
        let (store, id) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let mut req = request(&id, root.path());
        let legacy = req.clone();
        let mut old_json = serde_json::to_value(&legacy).unwrap();
        old_json
            .as_object_mut()
            .unwrap()
            .remove("package_extension");
        assert_eq!(
            serde_json::from_value::<CreateProjectRequest>(old_json).unwrap(),
            legacy
        );
        let portable = initial_creation_package(req.command.initial.clone())
            .unwrap()
            .commit_hex;
        req.package_extension = PackageExtension::try_from("jprtest".to_owned()).unwrap();
        assert_eq!(
            initial_creation_package(req.command.initial.clone())
                .unwrap()
                .commit_hex,
            portable
        );
        let op = req.command.initial.operation_id.uuid();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        let done = store.advance_project_creation(op, at()).await.unwrap();
        assert_eq!(done.package_path().extension().unwrap(), "jprtest");
        assert!(!root.path().join("Created Project.photara").exists());
        let expected = package(&req).unwrap().files;
        store.close().await;
        let (store, _) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let reopened = store.advance_project_creation(op, at()).await.unwrap();
        assert_eq!(reopened.request, req);
        for (name, bytes) in expected {
            assert_eq!(
                std::fs::read(reopened.package_path().join(name)).unwrap(),
                bytes
            );
        }
        let mut changed = req;
        changed.package_extension = PackageExtension::legacy_creation_alias();
        assert_eq!(
            store
                .prepare_project_creation(changed, at())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        store.close().await;
    }

    #[tokio::test]
    async fn cancellation_and_competing_destination_requests_are_retained() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(root.path().join("state.sqlite"), at())
            .await
            .unwrap();
        let req = request(&id, root.path());
        let op = req.command.initial.operation_id.uuid();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        assert_eq!(
            store
                .prepare_project_creation(request(&id, root.path()), at())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        store.cancel_project_creation(op).await.unwrap();
        assert_eq!(
            store
                .advance_project_creation(op, at())
                .await
                .unwrap()
                .state,
            CreationState::Cancelled
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_catalog")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            0
        );
        store
            .prepare_project_creation(request(&id, root.path()), at())
            .await
            .unwrap();
        store.close().await;
    }
    #[tokio::test]
    async fn published_package_survives_catalog_rollback_and_reconciles() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(root.path().join("state.sqlite"), at())
            .await
            .unwrap();
        let req = request(&id, root.path());
        let op = req.command.initial.operation_id.uuid();
        store.prepare_project_creation(req, at()).await.unwrap();
        sqlx::query("CREATE TRIGGER fail_ui1 BEFORE INSERT ON project_graph_projection BEGIN SELECT RAISE(ABORT,'test'); END").execute(store.db.pool()).await.unwrap();
        assert!(store.advance_project_creation(op, at()).await.is_err());
        let c = store.project_creations().await.unwrap().remove(0);
        assert_eq!(c.state, CreationState::Published);
        assert!(c.package_path().exists());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_catalog")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            0
        );
        sqlx::query("DROP TRIGGER fail_ui1")
            .execute(store.db.pool())
            .await
            .unwrap();
        assert_eq!(
            store
                .advance_project_creation(op, at())
                .await
                .unwrap()
                .state,
            CreationState::Complete
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
    #[tokio::test]
    async fn unjournaled_stage_and_independent_pool_replays_recover_without_adoption() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = root.path().join("state.sqlite");
        let (store, id) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let req = request(&id, root.path());
        let op = req.command.initial.operation_id.uuid();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        sqlx::query("CREATE TRIGGER fail_pin BEFORE UPDATE OF stage_pin ON project_creation_intents BEGIN SELECT RAISE(ABORT,'simulated crash before pin commit'); END").execute(store.db.pool()).await.unwrap();
        assert!(store.advance_project_creation(op, at()).await.is_err());
        let orphan = std::fs::read_dir(root.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".project-create-")
            })
            .unwrap();
        assert_eq!(std::fs::read_dir(&orphan).unwrap().count(), 0);
        assert!(store.project_creations().await.unwrap()[0].stage.is_none());
        sqlx::query("DROP TRIGGER fail_pin")
            .execute(store.db.pool())
            .await
            .unwrap();
        let (other, _) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let (left, right) = tokio::join!(
            store.advance_project_creation(op, at()),
            other.advance_project_creation(op, at())
        );
        for done in [left.unwrap(), right.unwrap()] {
            assert_eq!(done.state, CreationState::Complete);
            assert_eq!(done.request, req);
            assert_ne!(done.stage.unwrap().path, orphan);
        }
        assert!(orphan.is_dir());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_catalog")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_graph_projection")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            1
        );
        store.verify_integrity().await.unwrap();
        other.close().await;
        store.close().await;
    }

    #[tokio::test]
    async fn interrupted_cancellation_finishes_without_publishing() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(root.path().join("state.sqlite"), at())
            .await
            .unwrap();
        let req = request(&id, root.path());
        let op = req.command.initial.operation_id.uuid();
        store
            .prepare_project_creation(req.clone(), at())
            .await
            .unwrap();
        let p = package(&req).unwrap();
        let stage = req.destination.create_stage(&p).unwrap();
        stage.materialize(&p).unwrap();
        sqlx::query("UPDATE project_creation_intents SET stage_pin=?,state='cancelling' WHERE operation_id=?")
            .bind(canonical(&stage).unwrap()).bind(bytes(op)).execute(store.db.pool()).await.unwrap();
        // Crash after all owned stage contents were removed, before cancellation receipt.
        stage.discard_stage(&req.destination, &p).unwrap();
        let done = store.advance_project_creation(op, at()).await.unwrap();
        assert_eq!(done.state, CreationState::Cancelled);
        assert!(!done.package_path().exists());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_catalog")
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            0
        );
        store.cancel_project_creation(op).await.unwrap();
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}
