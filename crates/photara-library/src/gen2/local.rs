//! D19 local controller. Identity is read only from a verified database;
//! local authority is explicit and never inferred from a catalog or path.
use super::*;
use photara_core::contracts::access::{ActionMask, ProjectAction, ProjectPolicy};
use photara_core::contracts::{LocalPrincipalId, ProjectAccessGrantId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalIdentity {
    pub database_id: DatabaseId,
    pub device_id: DeviceId,
    pub library_id: LibraryId,
    pub principal_id: LocalPrincipalId,
}

#[derive(Clone, Debug)]
pub struct RegisteredProject {
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub association_commit_id: CommitId,
    pub association_sha256: [u8; 32],
    pub source_origin_library_id: Option<LibraryId>,
}

impl LocalLibraryStore {
    /// Initialize an explicitly chosen local app-state path. Existing files are
    /// inspected read-only before opening; unknown files are never adopted.
    /// # Errors
    /// Rejects unsafe paths, unknown databases, schema floors and damaged identity.
    pub async fn open_app_state(
        path: impl AsRef<Path>,
        at: Timestamp,
    ) -> Result<(Self, LocalIdentity)> {
        let path = path.as_ref();
        check_path(path)?;
        let (mode, device) = match std::fs::symlink_metadata(path) {
            Ok(_) => {
                preflight(path)?;
                let connection = rusqlite::Connection::open_with_flags(
                    path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .map_err(|_| Error::ForeignDatabase)?;
                let (family, reader, writer, epoch): (String, i64, i64, i64) = connection.query_row("SELECT schema_family,minimum_reader,minimum_writer,schema_epoch FROM schema_metadata WHERE singleton=1", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_| Error::ForeignDatabase)?;
                if family != "photara.local.g2"
                    || epoch != 1
                    || !(1..=2).contains(&reader)
                    || writer != reader
                {
                    return Err(Error::Unsupported);
                }
                let bytes: Vec<u8> = connection
                    .query_row(
                        "SELECT device_id FROM local_device WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|_| Error::Corrupt)?;
                (OpenMode::OpenExisting, DeviceId::from_bytes(&bytes)?)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let parent = path.parent().ok_or(Error::Invalid)?;
                std::fs::create_dir_all(parent).map_err(|_| Error::Io)?;
                check_path(path)?;
                (OpenMode::CreateNew, DeviceId::new())
            }
            Err(_) => return Err(Error::Io),
        };
        let store = Self::open(path, mode, device, at).await?;
        let identity = store.ensure_default_library(at).await?;
        Ok((store, identity))
    }

    /// Creates the default once, using database-scoped stable identities. Renaming
    /// the Library does not cause a replacement on the next launch.
    /// # Errors
    /// Returns identity collisions, authority mismatches or storage errors.
    pub async fn ensure_default_library(&self, at: Timestamp) -> Result<LocalIdentity> {
        let stable = |domain: &[u8]| {
            let mut hash = Sha256::new();
            hash.update(domain);
            hash.update(self.info.database_id.bytes());
            let digest = hash.finalize();
            let mut bytes = [0; 16];
            bytes.copy_from_slice(&digest[..16]);
            Uuid::from_bytes(bytes)
        };
        let id = LibraryId::try_from(stable(b"photara.default-library.v2"))?;
        let principal = LocalPrincipalId::from_uuid(stable(b"photara.local-principal.v2"))
            .map_err(|_| Error::Invalid)?;
        let mut tx = self.write().await?;
        let existing: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM libraries WHERE library_id=?)")
                .bind(id.bytes())
                .fetch_one(&mut *tx)
                .await?;
        if !existing {
            sqlx::query("INSERT INTO libraries VALUES(?,'My Library','active',1,?,?,NULL,1,'{}')")
                .bind(id.bytes())
                .bind(at.get())
                .bind(at.get())
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO library_contract_state VALUES(?,1,'local-only',?,1,1,1,?,?)")
                .bind(id.bytes())
                .bind(principal.uuid().as_bytes().to_vec())
                .bind(at.get())
                .bind(at.get())
                .execute(&mut *tx)
                .await?;
        }
        let bytes: Vec<u8> = sqlx::query_scalar("SELECT local_principal_id FROM library_contract_state WHERE library_id=? AND authority_mode='local-only'").bind(id.bytes()).fetch_one(&mut *tx).await?;
        let principal =
            LocalPrincipalId::from_uuid(Uuid::from_slice(&bytes).map_err(|_| Error::Corrupt)?)
                .map_err(|_| Error::Corrupt)?;
        local_authority(&mut tx, id, principal).await?;
        tx.commit().await?;
        Ok(LocalIdentity {
            database_id: self.info.database_id,
            device_id: self.info.device_id,
            library_id: id,
            principal_id: principal,
        })
    }

    /// Registers an explicitly verified package association with restricted policy
    /// and an explicit creator manager in the same transaction.
    /// # Errors
    /// Returns authority, association collision or storage errors.
    pub async fn register_project(
        &self,
        actor: LocalPrincipalId,
        project: &RegisteredProject,
        at: Timestamp,
    ) -> Result<()> {
        let mut tx = self.write().await?;
        local_authority(&mut tx, project.library_id, actor).await?;
        sqlx::query("INSERT INTO project_ownership VALUES(?,?,'active',?,?,?,'photara.package.v1.1',1,1,?,?)")
            .bind(project.project_id.bytes()).bind(project.library_id.bytes()).bind(project.association_commit_id.bytes()).bind(project.association_sha256.to_vec()).bind(project.source_origin_library_id.map(LibraryId::bytes)).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO project_access_policies VALUES(?,?,'restricted',0,0,0,0,1,1,1,?,?)",
        )
        .bind(project.library_id.bytes())
        .bind(project.project_id.bytes())
        .bind(at.get())
        .bind(at.get())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO project_access_grants VALUES(?,?,?,NULL,?,255,'active',NULL,1,1,?,?)",
        )
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .bind(project.library_id.bytes())
        .bind(project.project_id.bytes())
        .bind(actor.uuid().as_bytes().to_vec())
        .bind(at.get())
        .bind(at.get())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO project_catalog(library_id,project_id,visibility,local_revision,created_at_ms,updated_at_ms) VALUES(?,?,'visible',1,?,?)").bind(project.library_id.bytes()).bind(project.project_id.bytes()).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
        bump_authority(&mut tx, project.library_id, at).await?;
        audit_control(&mut tx,self.info.device_id,actor,project.library_id,"register-project",&serde_json::json!({"project_id":project.project_id,"association_commit_id":project.association_commit_id,"association_sha256":hex(&project.association_sha256)}),at).await?;
        assert_manager(&mut tx, project.library_id, project.project_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Checks current local facts on every request, including a retained revoked
    /// grant. Library authority alone cannot bypass Project permissions.
    /// # Errors
    /// Returns a masked authority error or storage failure.
    pub async fn authorize_project(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        project: ProjectId,
        action: ProjectAction,
    ) -> Result<()> {
        let mut tx = self.read().await?;
        project_authority(&mut tx, library, project, actor, action).await
    }

    /// Atomically changes policy under explicit manager authority and exact CAS.
    /// # Errors
    /// Rejects stale revisions, denied actions and storage errors.
    pub async fn set_project_policy(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        project: ProjectId,
        expected: Revision,
        policy: &ProjectPolicy,
        at: Timestamp,
    ) -> Result<()> {
        self.change_project_policy(
            actor,
            &ProjectPolicyChange {
                library_id: library,
                project_id: project,
                expected,
                policy: policy.clone(),
                disclosure: None,
            },
            at,
        )
        .await
    }
    /// Applies an explicit policy command with retained disclosure evidence when
    /// permissions widen. The host supplies verified package/projection coordinates.
    /// # Errors
    /// Rejects missing or mismatched disclosure, denied authority and stale CAS.
    pub async fn change_project_policy(
        &self,
        actor: LocalPrincipalId,
        change: &ProjectPolicyChange,
        at: Timestamp,
    ) -> Result<()> {
        let (library, project, expected, policy) = (
            change.library_id,
            change.project_id,
            change.expected,
            &change.policy,
        );
        let mut tx = self.write().await?;
        project_authority(
            &mut tx,
            library,
            project,
            actor,
            ProjectAction::ManageAccess,
        )
        .await?;
        let p = policy.spec();
        let old=sqlx::query("SELECT owner_mask,admin_mask,editor_mask,viewer_mask FROM project_access_policies WHERE library_id=? AND project_id=?").bind(library.bytes()).bind(project.bytes()).fetch_one(&mut *tx).await?;
        let masks = [
            ("owner_mask", p.owner),
            ("admin_mask", p.admin),
            ("editor_mask", p.editor),
            ("viewer_mask", p.viewer),
        ];
        let mut widening = false;
        for (name, mask) in masks {
            widening |= i64::from(mask.bits()) & !old.try_get::<i64, _>(name)? != 0;
        }
        if widening {
            let ack = change.disclosure.as_ref().ok_or(Error::Invalid)?;
            if ack.actor != actor
                || ack.project_id != project
                || ack.proposed_policy_sha256 != sha(canonical(policy)?.as_bytes())
            {
                return Err(Error::Invalid);
            }
        }
        let visibility = match p.visibility {
            photara_core::contracts::access::VisibilityPolicy::Restricted => "restricted",
            photara_core::contracts::access::VisibilityPolicy::LibraryVisible => "library-visible",
        };
        let changed=sqlx::query("UPDATE project_access_policies SET visibility_policy=?,owner_mask=?,admin_mask=?,editor_mask=?,viewer_mask=?,authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND project_id=? AND local_revision=?").bind(visibility).bind(i64::from(p.owner.bits())).bind(i64::from(p.admin.bits())).bind(i64::from(p.editor.bits())).bind(i64::from(p.viewer.bits())).bind(at.get()).bind(library.bytes()).bind(project.bytes()).bind(expected.get()).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        bump_authority(&mut tx, library, at).await?;
        audit_control(
            &mut tx,
            self.info.device_id,
            actor,
            library,
            "set-project-policy",
            &serde_json::json!({"project_id":project,"expected":expected,"policy":policy,"disclosure_ack":change.disclosure}),
            at,
        )
        .await?;
        assert_manager(&mut tx, library, project).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Dedicated grant CAS; a revoked grant requires the explicit regrant flag.
    /// # Errors
    /// Rejects denied commands, last-manager loss, stale revisions and implicit revival.
    pub async fn change_local_grant(
        &self,
        actor: LocalPrincipalId,
        change: &LocalGrantChange,
        at: Timestamp,
    ) -> Result<()> {
        let mut tx = self.write().await?;
        project_authority(
            &mut tx,
            change.library_id,
            change.project_id,
            actor,
            ProjectAction::ManageAccess,
        )
        .await?;
        let old: String=sqlx::query_scalar("SELECT state FROM project_access_grants WHERE grant_id=? AND library_id=? AND project_id=?").bind(change.grant_id.uuid().as_bytes().to_vec()).bind(change.library_id.bytes()).bind(change.project_id.bytes()).fetch_optional(&mut *tx).await?.ok_or(Error::Invalid)?;
        if old == "revoked" && !change.revoke && !change.regrant {
            return Err(Error::Invalid);
        }
        let result=sqlx::query("UPDATE project_access_grants SET action_mask=?,state=?,revoked_at=?,local_revision=local_revision+1,updated_at=? WHERE grant_id=? AND library_id=? AND project_id=? AND local_revision=?")
            .bind(i64::from(change.actions.bits())).bind(if change.revoke {"revoked"} else {"active"}).bind(change.revoke.then_some(at.get())).bind(at.get()).bind(change.grant_id.uuid().as_bytes().to_vec()).bind(change.library_id.bytes()).bind(change.project_id.bytes()).bind(change.expected.get()).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE project_access_policies SET authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND project_id=?").bind(at.get()).bind(change.library_id.bytes()).bind(change.project_id.bytes()).execute(&mut *tx).await?;
        bump_authority(&mut tx, change.library_id, at).await?;
        audit_control(&mut tx,self.info.device_id,actor,change.library_id,"change-local-grant",&serde_json::json!({"project_id":change.project_id,"grant_id":change.grant_id,"expected":change.expected,"actions":change.actions,"revoke":change.revoke,"regrant":change.regrant}),at).await?;
        assert_manager(&mut tx, change.library_id, change.project_id).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LocalGrantChange {
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub grant_id: ProjectAccessGrantId,
    pub expected: Revision,
    pub actions: ActionMask,
    pub revoke: bool,
    pub regrant: bool,
}

pub(super) async fn local_authority(
    conn: &mut SqliteConnection,
    library: LibraryId,
    actor: LocalPrincipalId,
) -> Result<()> {
    let allowed: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM library_contract_state a JOIN libraries l USING(library_id) WHERE a.library_id=? AND a.authority_mode='local-only' AND a.local_principal_id=? AND l.state='active')").bind(library.bytes()).bind(actor.uuid().as_bytes().to_vec()).fetch_one(&mut *conn).await?;
    if allowed { Ok(()) } else { Err(Error::Invalid) }
}
pub(super) async fn project_authority(
    conn: &mut SqliteConnection,
    library: LibraryId,
    project: ProjectId,
    actor: LocalPrincipalId,
    action: ProjectAction,
) -> Result<()> {
    local_authority(conn, library, actor).await?;
    let bits:Option<i64>=sqlx::query_scalar("SELECT g.action_mask FROM project_access_grants g JOIN project_ownership o USING(library_id,project_id) WHERE g.library_id=? AND g.project_id=? AND g.local_principal_id=? AND g.state='active' AND o.registration_state='active'").bind(library.bytes()).bind(project.bytes()).bind(actor.uuid().as_bytes().to_vec()).fetch_optional(&mut *conn).await?;
    let mask = bits
        .and_then(|v| u16::try_from(v).ok())
        .and_then(|v| ActionMask::new(v).ok())
        .ok_or(Error::Invalid)?;
    if mask.allows(action) {
        Ok(())
    } else {
        Err(Error::Invalid)
    }
}
async fn assert_manager(
    conn: &mut SqliteConnection,
    library: LibraryId,
    project: ProjectId,
) -> Result<()> {
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_access_grants g JOIN library_contract_state a USING(library_id) WHERE g.library_id=? AND g.project_id=? AND g.state='active' AND g.action_mask=255 AND g.local_principal_id=a.local_principal_id AND a.authority_mode='local-only')").bind(library.bytes()).bind(project.bytes()).fetch_one(&mut *conn).await?;
    if exists {
        Ok(())
    } else {
        Err(Error::Constraint)
    }
}
async fn bump_authority(
    conn: &mut SqliteConnection,
    library: LibraryId,
    at: Timestamp,
) -> Result<()> {
    let result=sqlx::query("UPDATE library_contract_state SET authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=?").bind(at.get()).bind(library.bytes()).execute(&mut *conn).await?;
    if result.rows_affected() == 1 {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at() -> Timestamp {
        Timestamp::try_from(1_789_142_400_000).unwrap()
    }
    #[tokio::test]
    async fn app_initialization_is_idempotent_and_checks_integrity() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("State/photara-local-v2.sqlite");
        let (store, first) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        assert_eq!(store.info().migration_count, 12);
        assert_eq!(store.libraries(None, 100, false).await.unwrap().len(), 1);
        store.verify_integrity().await.unwrap();
        store.close().await;
        let (store, second) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        assert_eq!(first, second);
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
    #[tokio::test]
    async fn unknown_database_is_untouched() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("unknown.sqlite");
        std::fs::write(&path, b"unknown user data").unwrap();
        assert!(matches!(
            LocalLibraryStore::open_app_state(&path, at()).await,
            Err(Error::ForeignDatabase)
        ));
        assert_eq!(std::fs::read(path).unwrap(), b"unknown user data");
    }
    #[tokio::test]
    async fn registration_permissions_cas_and_last_manager_rollback() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("state.sqlite"), at())
            .await
            .unwrap();
        let project = RegisteredProject {
            library_id: id.library_id,
            project_id: ProjectId::new(),
            association_commit_id: CommitId::new(),
            association_sha256: [42; 32],
            source_origin_library_id: Some(id.library_id),
        };
        store
            .register_project(id.principal_id, &project, at())
            .await
            .unwrap();
        store
            .authorize_project(
                id.principal_id,
                id.library_id,
                project.project_id,
                ProjectAction::Run,
            )
            .await
            .unwrap();
        let stranger = LocalPrincipalId::from_uuid(Uuid::new_v4()).unwrap();
        assert!(
            store
                .authorize_project(
                    stranger,
                    id.library_id,
                    project.project_id,
                    ProjectAction::Read
                )
                .await
                .is_err()
        );
        store
            .set_project_policy(
                id.principal_id,
                id.library_id,
                project.project_id,
                Revision::INITIAL,
                &ProjectPolicy::restricted(),
                at(),
            )
            .await
            .unwrap();
        assert!(matches!(
            store
                .set_project_policy(
                    id.principal_id,
                    id.library_id,
                    project.project_id,
                    Revision::INITIAL,
                    &ProjectPolicy::restricted(),
                    at()
                )
                .await,
            Err(Error::Conflict)
        ));
        let bytes: Vec<u8> = sqlx::query_scalar("SELECT grant_id FROM project_access_grants")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        let grant = ProjectAccessGrantId::from_uuid(Uuid::from_slice(&bytes).unwrap()).unwrap();
        let change = LocalGrantChange {
            library_id: id.library_id,
            project_id: project.project_id,
            grant_id: grant,
            expected: Revision::INITIAL,
            actions: ActionMask::NONE,
            revoke: true,
            regrant: false,
        };
        assert!(matches!(
            store
                .change_local_grant(id.principal_id, &change, at())
                .await,
            Err(Error::Constraint)
        ));
        store
            .authorize_project(
                id.principal_id,
                id.library_id,
                project.project_id,
                ProjectAction::ManageAccess,
            )
            .await
            .unwrap();
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}

impl LocalLibraryStore {
    /// Transfers the explicit local controller and all Projects it currently
    /// manages in one transaction. Any inaccessible active Project blocks transfer.
    /// # Errors
    /// Rejects stale authority CAS, hidden Project access or incomplete manager transfer.
    pub async fn transfer_local_controller(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        next: LocalPrincipalId,
        expected: Revision,
        at: Timestamp,
    ) -> Result<()> {
        if actor == next {
            return Err(Error::Invalid);
        }
        let mut tx = self.write().await?;
        local_authority(&mut tx, library, actor).await?;
        let projects:Vec<Vec<u8>>=sqlx::query_scalar("SELECT project_id FROM project_ownership WHERE library_id=? AND registration_state='active' ORDER BY project_id").bind(library.bytes()).fetch_all(&mut *tx).await?;
        for bytes in &projects {
            let project = ProjectId::from_bytes(bytes)?;
            project_authority(
                &mut tx,
                library,
                project,
                actor,
                ProjectAction::ManageAccess,
            )
            .await?;
            sqlx::query("UPDATE project_access_grants SET state='revoked',revoked_at=?,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND project_id=? AND local_principal_id=? AND state='active'").bind(at.get()).bind(at.get()).bind(library.bytes()).bind(bytes).bind(actor.uuid().as_bytes().to_vec()).execute(&mut *tx).await?;
        }
        let changed=sqlx::query("UPDATE library_contract_state SET local_principal_id=?,authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND local_revision=?").bind(next.uuid().as_bytes().to_vec()).bind(at.get()).bind(library.bytes()).bind(expected.get()).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        for bytes in &projects {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_access_grants WHERE library_id=? AND project_id=? AND local_principal_id=?)").bind(library.bytes()).bind(bytes).bind(next.uuid().as_bytes().to_vec()).fetch_one(&mut *tx).await?;
            if exists {
                sqlx::query("UPDATE project_access_grants SET state='active',revoked_at=NULL,action_mask=255,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND project_id=? AND local_principal_id=?").bind(at.get()).bind(library.bytes()).bind(bytes).bind(next.uuid().as_bytes().to_vec()).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO project_access_grants VALUES(?,?,?,NULL,?,255,'active',NULL,1,1,?,?)").bind(Uuid::new_v4().as_bytes().to_vec()).bind(library.bytes()).bind(bytes).bind(next.uuid().as_bytes().to_vec()).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE project_access_policies SET authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=? AND project_id=?").bind(at.get()).bind(library.bytes()).bind(bytes).execute(&mut *tx).await?;
            assert_manager(&mut tx, library, ProjectId::from_bytes(bytes)?).await?;
        }
        audit_control(&mut tx,self.info.device_id,actor,library,"transfer-local-controller",&serde_json::json!({"next_principal_id":next,"expected":expected,"projects":projects.iter().map(|p|hex(p)).collect::<Vec<_>>()}),at).await?;
        tx.commit().await?;
        Ok(())
    }
}
async fn audit_control(
    conn: &mut SqliteConnection,
    device: DeviceId,
    actor: LocalPrincipalId,
    library: LibraryId,
    command: &str,
    body: &serde_json::Value,
    at: Timestamp,
) -> Result<()> {
    let envelope = canonical(
        &serde_json::json!({"schema":"photara.local-control.v2","actor":actor,"command":command,"body":body}),
    )?;
    sqlx::query("INSERT INTO mutations VALUES(?,?,?,'local',2,'library',?,?,?,?)")
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .bind(library.bytes())
        .bind(device.bytes())
        .bind(library.bytes())
        .bind(at.get())
        .bind(&envelope)
        .bind(sha(envelope.as_bytes()).to_vec())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use storexa::SqliteDatabaseConfig;
    fn at() -> Timestamp {
        Timestamp::try_from(1_789_142_400_000).unwrap()
    }
    async fn baseline(path: &Path) -> DeviceId {
        std::fs::File::create(path).unwrap();
        let db = SqliteDatabase::connect(
            SqliteDatabaseConfig::from_path(path)
                .unwrap()
                .with_max_connections(1)
                .with_journal_mode(Some(SqliteJournalMode::Wal)),
        )
        .await
        .unwrap();
        baseline_migrator().run(db.pool()).await.unwrap();
        let device = DeviceId::new();
        sqlx::query("INSERT INTO schema_metadata VALUES(1,'photara.local.g2',?,1,1,1,'photara.canonical-json.v1',?)").bind(DatabaseId::new().bytes()).bind(at().get()).execute(db.pool()).await.unwrap();
        sqlx::query("INSERT INTO local_device VALUES(1,?,'Local device',?)")
            .bind(device.bytes())
            .bind(at().get())
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO normalization_policies VALUES(1,'16.0.0',?,?)")
            .bind(sha(validation::POLICY.as_bytes()).to_vec())
            .bind(validation::POLICY)
            .execute(db.pool())
            .await
            .unwrap();
        db.close().await;
        device
    }
    #[tokio::test]
    async fn empty_0006_upgrade_and_activation_failure_rollback() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("state.sqlite");
        baseline(&path).await;
        let (store, _) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        assert_eq!(store.info().migration_count, 12);
        store.verify_integrity().await.unwrap();
        store.close().await;
        let bad = dir.path().join("failure.sqlite");
        let device = baseline(&bad).await;
        let connection = rusqlite::Connection::open(&bad).unwrap();
        connection.execute_batch("CREATE TRIGGER fail_floor BEFORE UPDATE ON schema_metadata BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        drop(connection);
        assert!(matches!(
            LocalLibraryStore::open(&bad, OpenMode::OpenExisting, device, at()).await,
            Err(Error::Migration)
        ));
        let connection =
            rusqlite::Connection::open_with_flags(&bad, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM _sqlx_migrations", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            6
        );
        assert_eq!(
            connection
                .query_row("SELECT minimum_writer FROM schema_metadata", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='library_contract_state'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn populated_0006_is_not_adopted_and_newer_floor_is_untouched() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("state.sqlite");
        baseline(&path).await;
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "INSERT INTO libraries VALUES(?,'Existing','active',1,0,0,NULL,1,'{}')",
                [LibraryId::new().bytes()],
            )
            .unwrap();
        drop(connection);
        assert!(matches!(
            LocalLibraryStore::open_app_state(&path, at()).await,
            Err(Error::Unsupported)
        ));
        let fresh = dir.path().join("newer.sqlite");
        let (store, _) = LocalLibraryStore::open_app_state(&fresh, at())
            .await
            .unwrap();
        store.close().await;
        let connection = rusqlite::Connection::open(&fresh).unwrap();
        connection
            .execute(
                "UPDATE schema_metadata SET minimum_reader=3,minimum_writer=3",
                [],
            )
            .unwrap();
        drop(connection);
        let bytes = std::fs::read(&fresh).unwrap();
        assert!(matches!(
            LocalLibraryStore::open_app_state(&fresh, at()).await,
            Err(Error::Unsupported)
        ));
        assert_eq!(std::fs::read(fresh).unwrap(), bytes);
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)] // One end-to-end transaction/restart scenario.
    async fn independent_pools_policy_cas_and_atomic_controller_transfer() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("state.sqlite");
        let (store, id) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let p = RegisteredProject {
            library_id: id.library_id,
            project_id: ProjectId::new(),
            association_commit_id: CommitId::new(),
            association_sha256: [42; 32],
            source_origin_library_id: Some(id.library_id),
        };
        store
            .register_project(id.principal_id, &p, at())
            .await
            .unwrap();
        let (other, _) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        let policy = ProjectPolicy::restricted();
        let (a, b) = tokio::join!(
            store.set_project_policy(
                id.principal_id,
                id.library_id,
                p.project_id,
                Revision::INITIAL,
                &policy,
                at()
            ),
            other.set_project_policy(
                id.principal_id,
                id.library_id,
                p.project_id,
                Revision::INITIAL,
                &policy,
                at()
            )
        );
        assert_ne!(a.is_ok(), b.is_ok());
        let new = LocalPrincipalId::from_uuid(Uuid::new_v4()).unwrap();
        assert!(matches!(
            store
                .transfer_local_controller(
                    id.principal_id,
                    id.library_id,
                    new,
                    Revision::INITIAL,
                    at()
                )
                .await,
            Err(Error::Conflict)
        ));
        store
            .authorize_project(
                id.principal_id,
                id.library_id,
                p.project_id,
                ProjectAction::Run,
            )
            .await
            .unwrap();
        let revision: i64 = sqlx::query_scalar("SELECT local_revision FROM library_contract_state")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        store
            .transfer_local_controller(
                id.principal_id,
                id.library_id,
                new,
                Revision::try_from(revision).unwrap(),
                at(),
            )
            .await
            .unwrap();
        assert!(
            store
                .authorize_project(
                    id.principal_id,
                    id.library_id,
                    p.project_id,
                    ProjectAction::Read
                )
                .await
                .is_err()
        );
        store
            .authorize_project(
                new,
                id.library_id,
                p.project_id,
                ProjectAction::ManageAccess,
            )
            .await
            .unwrap();
        store.verify_integrity().await.unwrap();
        store.close().await;
        other.close().await;
        let (store, reopened) = LocalLibraryStore::open_app_state(&path, at())
            .await
            .unwrap();
        assert_eq!(reopened.principal_id, new);
        store.close().await;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DisclosureAcknowledgement {
    pub actor: LocalPrincipalId,
    pub project_id: ProjectId,
    pub commit_id: CommitId,
    pub commit_sha256: [u8; 32],
    pub projection_sha256: [u8; 32],
    pub proposed_policy_sha256: [u8; 32],
}
#[derive(Clone, Debug)]
pub struct ProjectPolicyChange {
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub expected: Revision,
    pub policy: ProjectPolicy,
    pub disclosure: Option<DisclosureAcknowledgement>,
}
