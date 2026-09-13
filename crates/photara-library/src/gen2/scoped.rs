//! Scoped local projections. Current Project access is checked independently of
//! media digests. Local-only state never claims an online authorization lease.
use super::local::{local_authority, project_authority};
use super::*;
use photara_core::contracts::{LocalPrincipalId, access::ProjectAction};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectMediaPurpose {
    Cover,
    AssignedSnapshot,
}
impl ProjectMediaPurpose {
    const fn sql(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::AssignedSnapshot => "assigned-snapshot",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ProjectMediaProjection {
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub sha256: [u8; 32],
    pub purpose: ProjectMediaPurpose,
    pub source_commit_id: CommitId,
    pub source_commit_sha256: [u8; 32],
    pub consent_projection_sha256: [u8; 32],
    pub media_type: String,
    pub byte_length: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAuthoritySnapshot {
    pub library_id: LibraryId,
    pub principal_id: LocalPrincipalId,
    pub revision: Revision,
    pub authorization_generation: Revision,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopedTransportReadiness {
    LocalOnly,
    OnlineBootstrapRequired,
    ReconciliationRequired,
}
impl LocalLibraryStore {
    /// Returns the explicit local owner principal, not a fabricated Account member.
    /// # Errors
    /// Returns denied scope, stale/corrupt authority or storage failure.
    pub async fn local_membership(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
    ) -> Result<LocalAuthoritySnapshot> {
        let mut tx = self.read().await?;
        local_authority(&mut tx, library, actor).await?;
        let row=sqlx::query("SELECT local_revision,authorization_generation FROM library_contract_state WHERE library_id=?").bind(library.bytes()).fetch_one(&mut *tx).await?;
        Ok(LocalAuthoritySnapshot {
            library_id: library,
            principal_id: actor,
            revision: Revision::try_from(row.try_get::<i64, _>("local_revision")?)?,
            authorization_generation: Revision::try_from(
                row.try_get::<i64, _>("authorization_generation")?,
            )?,
        })
    }
    /// Verifies content bytes before linking a Project-specific projection. No
    /// object upload, URL issuance or disk-cache availability is inferred.
    /// # Errors
    /// Rejects changed bytes, denied edit rights, conflicting scope or invalid media.
    pub async fn link_project_media(
        &self,
        actor: LocalPrincipalId,
        projection: &ProjectMediaProjection,
        verified_bytes: &[u8],
        at: Timestamp,
    ) -> Result<()> {
        if verified_bytes.len() > 64 * 1024 * 1024
            || u64::try_from(verified_bytes.len()).map_err(|_| Error::Limit)?
                != projection.byte_length
            || sha(verified_bytes) != projection.sha256
        {
            return Err(Error::Invalid);
        }
        if projection.media_type.len() > 128
            || !projection.media_type.contains('/')
            || projection.media_type.chars().any(char::is_control)
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.write().await?;
        project_authority(
            &mut tx,
            projection.library_id,
            projection.project_id,
            actor,
            ProjectAction::Edit,
        )
        .await?;
        sqlx::query("INSERT INTO library_media VALUES(?,?,?,?,NULL,NULL,?) ON CONFLICT(library_id,sha256) DO NOTHING").bind(projection.library_id.bytes()).bind(projection.sha256.to_vec()).bind(&projection.media_type).bind(i64::try_from(projection.byte_length).map_err(|_|Error::Limit)?).bind(at.get()).execute(&mut *tx).await?;
        let row = sqlx::query(
            "SELECT media_type,byte_length FROM library_media WHERE library_id=? AND sha256=?",
        )
        .bind(projection.library_id.bytes())
        .bind(projection.sha256.to_vec())
        .fetch_one(&mut *tx)
        .await?;
        if row.try_get::<String, _>("media_type")? != projection.media_type
            || row.try_get::<i64, _>("byte_length")?
                != i64::try_from(projection.byte_length).map_err(|_| Error::Limit)?
        {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO project_media_links VALUES(?,?,?,?,?,?,?,'active',NULL,1,1,?,?)")
            .bind(projection.library_id.bytes())
            .bind(projection.project_id.bytes())
            .bind(projection.sha256.to_vec())
            .bind(projection.purpose.sql())
            .bind(projection.source_commit_id.bytes())
            .bind(projection.source_commit_sha256.to_vec())
            .bind(projection.consent_projection_sha256.to_vec())
            .bind(at.get())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    /// A digest grants no access: requires current read and an exact active link.
    /// # Errors
    /// Masks missing links and inaccessible Project scope consistently.
    pub async fn authorize_project_media(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        project: ProjectId,
        digest: [u8; 32],
        purpose: ProjectMediaPurpose,
    ) -> Result<()> {
        let mut tx = self.read().await?;
        project_authority(&mut tx, library, project, actor, ProjectAction::Read).await?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_media_links WHERE library_id=? AND project_id=? AND sha256=? AND purpose=? AND state='active')").bind(library.bytes()).bind(project.bytes()).bind(digest.to_vec()).bind(purpose.sql()).fetch_one(&mut *tx).await?;
        if exists { Ok(()) } else { Err(Error::Invalid) }
    }
    /// Reports whether transport can be activated. No local-only controller can
    /// synthesize Account authentication or settle sealed/unknown network work.
    /// # Errors
    /// Rejects inaccessible Library state and storage failure.
    pub async fn scoped_transport_readiness(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
    ) -> Result<ScopedTransportReadiness> {
        let mut tx = self.read().await?;
        local_authority(&mut tx, library, actor).await?;
        let unsettled:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM scoped_sync_operations o JOIN scoped_sync_channels c USING(channel_id) WHERE c.library_id=? AND o.state IN ('sealed','unknown','acknowledged')) OR EXISTS(SELECT 1 FROM sync_outbox WHERE library_id=? AND state IN ('sending','acknowledged'))").bind(library.bytes()).bind(library.bytes()).fetch_one(&mut *tx).await?;
        if unsettled {
            return Ok(ScopedTransportReadiness::ReconciliationRequired);
        }
        let channels: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM scoped_sync_channels WHERE library_id=?)",
        )
        .bind(library.bytes())
        .fetch_one(&mut *tx)
        .await?;
        Ok(if channels {
            ScopedTransportReadiness::OnlineBootstrapRequired
        } else {
            ScopedTransportReadiness::LocalOnly
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn media_access_is_project_and_purpose_scoped() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let at = Timestamp::try_from(1_789_142_400_000).unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("state.sqlite"), at)
            .await
            .unwrap();
        let p = RegisteredProject {
            library_id: id.library_id,
            project_id: ProjectId::new(),
            association_commit_id: CommitId::new(),
            association_sha256: [9; 32],
            source_origin_library_id: None,
        };
        store
            .register_project(id.principal_id, &p, at)
            .await
            .unwrap();
        let media = ProjectMediaProjection {
            library_id: id.library_id,
            project_id: p.project_id,
            sha256: sha(b"verified"),
            purpose: ProjectMediaPurpose::Cover,
            source_commit_id: p.association_commit_id,
            source_commit_sha256: p.association_sha256,
            consent_projection_sha256: [7; 32],
            media_type: "image/png".into(),
            byte_length: 8,
        };
        assert!(
            store
                .link_project_media(id.principal_id, &media, b"tampered", at)
                .await
                .is_err()
        );
        store
            .link_project_media(id.principal_id, &media, b"verified", at)
            .await
            .unwrap();
        store
            .authorize_project_media(
                id.principal_id,
                id.library_id,
                p.project_id,
                media.sha256,
                ProjectMediaPurpose::Cover,
            )
            .await
            .unwrap();
        assert!(
            store
                .authorize_project_media(
                    id.principal_id,
                    id.library_id,
                    p.project_id,
                    media.sha256,
                    ProjectMediaPurpose::AssignedSnapshot
                )
                .await
                .is_err()
        );
        assert!(
            store
                .authorize_project_media(
                    id.principal_id,
                    id.library_id,
                    ProjectId::new(),
                    media.sha256,
                    ProjectMediaPurpose::Cover
                )
                .await
                .is_err()
        );
        assert_eq!(
            store
                .scoped_transport_readiness(id.principal_id, id.library_id)
                .await
                .unwrap(),
            ScopedTransportReadiness::LocalOnly
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}

/// A retained local intent is not a network request or authorization lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedVariableIntent {
    pub operation_id: photara_core::contracts::OperationId,
    pub channel_id: ScopedChannelId,
    pub library_id: LibraryId,
    pub authorization_generation: Revision,
    pub channel_revision: Revision,
    pub variable: photara_core::context::variable::VariableAggregate,
}
impl LocalLibraryStore {
    /// Captures the exact current Library variable as an offline working overlay.
    /// Scope, generation and base are checked under one writer reservation; only
    /// a later online adapter may seal or dispatch this retained intent.
    /// # Errors
    /// Rejects changed IDs/bytes, Project channels, stale facts or mismatched post-state.
    pub async fn queue_scoped_variable(
        &self,
        actor: LocalPrincipalId,
        intent: &ScopedVariableIntent,
        at: Timestamp,
    ) -> Result<()> {
        let mut tx = self.write().await?;
        local_authority(&mut tx, intent.library_id, actor).await?;
        let channel=sqlx::query("SELECT scope_kind,authorization_generation,local_revision,state FROM scoped_sync_channels WHERE channel_id=? AND library_id=?").bind(intent.channel_id.bytes()).bind(intent.library_id.bytes()).fetch_optional(&mut *tx).await?.ok_or(Error::Invalid)?;
        if channel.try_get::<String, _>("scope_kind")? != "library"
            || channel.try_get::<i64, _>("authorization_generation")?
                != intent.authorization_generation.get()
            || channel.try_get::<i64, _>("local_revision")? != intent.channel_revision.get()
            || !["active", "paused"].contains(&channel.try_get::<String, _>("state")?.as_str())
        {
            return Err(Error::Conflict);
        }
        let values = super::context::read_variables(&mut tx, intent.library_id).await?;
        if !values.contains(&intent.variable) {
            return Err(Error::Conflict);
        }
        let value = intent.variable.spec();
        let post = canonical(&intent.variable)?.into_bytes();
        let body=canonical(&serde_json::json!({"schema":"photara.scoped-variable-intent.v2","channel_id":intent.channel_id,"library_id":intent.library_id,"authorization_generation":intent.authorization_generation,"variable":intent.variable}))?.into_bytes();
        let existing:Option<Vec<u8>>=sqlx::query_scalar("SELECT local_intent_canonical FROM scoped_sync_operations WHERE operation_id=? AND channel_id=?").bind(intent.operation_id.uuid().as_bytes().to_vec()).bind(intent.channel_id.bytes()).fetch_optional(&mut *tx).await?;
        if let Some(existing) = existing {
            return if existing == body {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        let predecessor:Option<Vec<u8>>=sqlx::query_scalar("SELECT o.operation_id FROM scoped_sync_operation_roots r JOIN scoped_sync_operations o USING(channel_id,operation_id) WHERE r.channel_id=? AND r.entity_kind='library-variable' AND r.entity_id=? AND o.state NOT IN ('superseded','discarded','rejected','conflict') ORDER BY o.created_at DESC,o.operation_id DESC LIMIT 1").bind(intent.channel_id.bytes()).bind(value.variable_id.uuid().as_bytes().to_vec()).fetch_optional(&mut *tx).await?;
        let base: Option<String> = if predecessor.is_none() {
            sqlx::query_scalar("SELECT server_revision FROM scoped_sync_base WHERE channel_id=? AND entity_kind='library-variable' AND entity_id=?").bind(intent.channel_id.bytes()).bind(value.variable_id.uuid().as_bytes().to_vec()).fetch_optional(&mut *tx).await?
        } else {
            None
        };
        sqlx::query("INSERT INTO scoped_sync_operations VALUES(?,?,'put-library-variable',?,?,NULL,NULL,NULL,NULL,'queued',NULL,?,?)").bind(intent.operation_id.uuid().as_bytes().to_vec()).bind(intent.channel_id.bytes()).bind(&body).bind(sha(&body).to_vec()).bind(at.get()).bind(at.get()).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO scoped_sync_operation_roots VALUES(?,?,'library-variable',?,?,?,?,?,?)",
        )
        .bind(intent.channel_id.bytes())
        .bind(intent.operation_id.uuid().as_bytes().to_vec())
        .bind(value.variable_id.uuid().as_bytes().to_vec())
        .bind(base)
        .bind(predecessor)
        .bind(&post)
        .bind(sha(&post).to_vec())
        .bind(value.revision.get())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod queue_tests {
    use super::*;
    #[tokio::test]
    async fn scoped_intent_retries_and_predecessors_keep_original_bytes() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let at = Timestamp::try_from(1_789_142_400_000).unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("state.sqlite"), at)
            .await
            .unwrap();
        let variable = super::super::context::tests::variable(id.library_id);
        store
            .put_library_variable(id.principal_id, &variable, None)
            .await
            .unwrap();
        // Fake cached channel, created only inside this disposable transport test.
        let channel = ScopedChannelId::new();
        let account = Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO account_cache VALUES(?,'Fake','active','sr1.fake',?)")
            .bind(&account)
            .bind(at.get())
            .execute(store.db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO scoped_sync_channels VALUES(?,?,?,'fake','library',NULL,1,?,NULL,'paused',1,?)").bind(channel.bytes()).bind(id.library_id.bytes()).bind(account).bind(Uuid::new_v4().as_bytes().to_vec()).bind(at.get()).execute(store.db.pool()).await.unwrap();
        let mut intent = ScopedVariableIntent {
            operation_id: photara_core::contracts::OperationId::from_uuid(Uuid::new_v4()).unwrap(),
            channel_id: channel,
            library_id: id.library_id,
            authorization_generation: Revision::INITIAL,
            channel_revision: Revision::INITIAL,
            variable,
        };
        store
            .queue_scoped_variable(id.principal_id, &intent, at)
            .await
            .unwrap();
        store
            .queue_scoped_variable(id.principal_id, &intent, at)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM scoped_sync_operations")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
        let first = intent.operation_id;
        intent.operation_id =
            photara_core::contracts::OperationId::from_uuid(Uuid::new_v4()).unwrap();
        store
            .queue_scoped_variable(id.principal_id, &intent, at)
            .await
            .unwrap();
        let predecessor: Vec<u8> = sqlx::query_scalar(
            "SELECT predecessor_operation_id FROM scoped_sync_operation_roots WHERE operation_id=?",
        )
        .bind(intent.operation_id.uuid().as_bytes().to_vec())
        .fetch_one(store.db.pool())
        .await
        .unwrap();
        assert_eq!(predecessor, first.uuid().as_bytes());
        intent.authorization_generation = Revision::try_from(2).unwrap();
        assert!(matches!(
            store
                .queue_scoped_variable(id.principal_id, &intent, at)
                .await,
            Err(Error::Conflict)
        ));
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}
