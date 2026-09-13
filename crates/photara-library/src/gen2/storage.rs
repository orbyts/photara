//! Logical storage and device-local bindings. Host I/O runs outside transactions.
use super::local::local_authority;
use super::*;
use photara_core::contracts::resource::{RelativeComponents, ResourceRights};
use photara_core::contracts::schema::LocalName;
use photara_core::contracts::{HostBindingId, LocalPrincipalId, StorageSlotId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LocationKindSpec {
    Filesystem,
    Provider {
        provider_id: photara_core::contracts::schema::QualifiedName,
    },
}
#[derive(Clone, Debug)]
pub struct LocationSpec {
    pub root: StorageRoot,
    pub kind: LocationKindSpec,
    pub rights: ResourceRights,
}
#[derive(Clone, Debug)]
pub struct SlotSpec {
    pub id: StorageSlotId,
    pub library_id: LibraryId,
    pub name: LocalName,
    pub display_name: String,
    pub root_id: StorageRootId,
    pub revision: Revision,
    pub tombstoned: bool,
}
/// Device evidence only; deliberately no serialization or path-bearing Debug.
#[derive(Clone)]
pub struct PathBinding {
    pub id: HostBindingId,
    pub library_id: LibraryId,
    pub root_id: StorageRootId,
    pub path: std::path::PathBuf,
    pub revision: Revision,
    pub generation: Revision,
    pub available: bool,
}
impl std::fmt::Debug for PathBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PathBinding([redacted])")
    }
}
/// Host adapter owns containment, permissions, content verification and lease expiry.
pub trait BindingHost {
    type Handle;
    /// # Errors
    /// Returns denied/unavailable/changed-evidence errors without exposing secrets.
    fn resolve(
        &self,
        binding: &PathBinding,
        relative: &RelativeComponents,
        rights: &ResourceRights,
    ) -> Result<Self::Handle>;
}
impl LocalLibraryStore {
    /// Writes classification and logical root as one CAS aggregate.
    /// # Errors
    /// Rejects authority, rights, immutable kind/provider or stale revisions.
    pub async fn put_location_spec(
        &self,
        actor: LocalPrincipalId,
        spec: &LocationSpec,
        expected: Option<Revision>,
    ) -> Result<()> {
        validation::name(&spec.root.display_name, 512)?;
        validation::identifier(&spec.root.purpose)?;
        if spec.rights.is_empty() {
            return Err(Error::Invalid);
        }
        let m = &spec.root.meta;
        let mut tx = self.write().await?;
        local_authority(&mut tx, m.library_id, actor).await?;
        check_meta(&mut tx, Table::Root, m, expected).await?;
        sqlx::query(if expected.is_none() {"INSERT INTO storage_roots(storage_root_id,library_id,display_name,label_key,purpose,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms) VALUES(?,?,?,?,?,?,?,?,?,?)"} else {"UPDATE storage_roots SET storage_root_id=?1,library_id=?2,display_name=?3,label_key=?4,purpose=?5,local_revision=?6,created_at_ms=?7,updated_at_ms=?8,state=?9,retired_at_ms=?10 WHERE storage_root_id=?1 AND library_id=?2"}).bind(m.id.bytes()).bind(m.library_id.bytes()).bind(&spec.root.display_name).bind(normalize_term(&spec.root.display_name)?).bind(&spec.root.purpose).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).execute(&mut *tx).await?;
        let (kind, provider) = match &spec.kind {
            LocationKindSpec::Filesystem => ("filesystem", None),
            LocationKindSpec::Provider { provider_id } => ("provider", Some(provider_id.as_str())),
        };
        let result=sqlx::query(if expected.is_none(){"INSERT INTO storage_location_specs VALUES(?1,?2,?3,?4,?5,1,?6,?7,?8)"}else{"UPDATE storage_location_specs SET storage_kind=?3,provider_id=?4,supported_rights_json=?5,local_revision=?6,updated_at=?8 WHERE library_id=?1 AND storage_root_id=?2 AND local_revision=?9 AND created_at=?7"}).bind(m.library_id.bytes()).bind(m.id.bytes()).bind(kind).bind(provider).bind(canonical(&spec.rights)?).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(expected.map(Revision::get)).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        finish(
            &mut tx,
            self.info.device_id,
            Table::Root,
            m,
            expected,
            &serde_json::json!({"root":spec.root,"kind":spec.kind,"rights":spec.rights}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Names remain reserved forever; rename and retarget use a single parent CAS.
    /// # Errors
    /// Rejects stale revisions, collisions, unclassified roots and retirement revival.
    pub async fn put_slot(
        &self,
        actor: LocalPrincipalId,
        slot: &SlotSpec,
        expected: Option<Revision>,
        at: Timestamp,
    ) -> Result<()> {
        validation::name(&slot.display_name, 512)?;
        if slot.revision != expected.map_or(Ok(Revision::INITIAL), Revision::next)?
            || expected.is_none() && slot.tombstoned
        {
            return Err(Error::Conflict);
        }
        let mut tx = self.write().await?;
        local_authority(&mut tx, slot.library_id, actor).await?;
        let result=sqlx::query(if expected.is_none(){"INSERT INTO storage_slots VALUES(?1,?2,?3,?4,?5,?6,?7,1,?8,?9,?9)"}else{"UPDATE storage_slots SET current_name=?3,display_name=?4,storage_root_id=?5,state=?6,retired_at=?7,local_revision=?8,updated_at=?9 WHERE slot_id=?1 AND library_id=?2 AND local_revision=?10"}).bind(slot.id.uuid().as_bytes().to_vec()).bind(slot.library_id.bytes()).bind(slot.name.as_str()).bind(&slot.display_name).bind(slot.root_id.bytes()).bind(if slot.tombstoned{"tombstoned"}else{"active"}).bind(slot.tombstoned.then_some(at.get())).bind(slot.revision.get()).bind(at.get()).bind(expected.map(Revision::get)).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO storage_slot_names VALUES(?,?,?,?) ON CONFLICT(library_id,name) DO NOTHING").bind(slot.library_id.bytes()).bind(slot.name.as_str()).bind(slot.id.uuid().as_bytes().to_vec()).bind(at.get()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Installs host-verified evidence. A path or availability change advances the
    /// generation; an unchanged observation advances only the revision.
    /// # Errors
    /// Rejects device/Library scope, stale tokens, unsafe paths or wrong generations.
    pub async fn put_path_binding(
        &self,
        actor: LocalPrincipalId,
        binding: &PathBinding,
        expected: Option<Revision>,
        at: Timestamp,
    ) -> Result<()> {
        if !binding.path.is_absolute()
            || binding
                .path
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(Error::Invalid);
        }
        let path = binding.path.to_str().ok_or(Error::Invalid)?;
        let mut tx = self.write().await?;
        local_authority(&mut tx, binding.library_id, actor).await?;
        let old=sqlx::query("SELECT host_path,availability,generation,local_revision FROM host_bindings WHERE binding_id=? AND device_id=? AND library_id=? AND storage_root_id=?").bind(binding.id.uuid().as_bytes().to_vec()).bind(self.info.device_id.bytes()).bind(binding.library_id.bytes()).bind(binding.root_id.bytes()).fetch_optional(&mut *tx).await?;
        let availability = if binding.available {
            "available"
        } else {
            "unavailable"
        };
        match (old, expected) {
            (None, None)
                if binding.revision == Revision::INITIAL
                    && binding.generation == Revision::INITIAL => {}
            (Some(row), Some(base)) => {
                let changed = row.try_get::<String, _>("host_path")? != path
                    || row.try_get::<String, _>("availability")? != availability;
                let generation = Revision::try_from(row.try_get::<i64, _>("generation")?)?;
                if row.try_get::<i64, _>("local_revision")? != base.get()
                    || binding.revision != base.next()?
                    || binding.generation
                        != if changed {
                            generation.next()?
                        } else {
                            generation
                        }
                {
                    return Err(Error::Conflict);
                }
            }
            _ => return Err(Error::Conflict),
        }
        let host = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        sqlx::query(if expected.is_none(){"INSERT INTO host_bindings VALUES(?1,?2,?3,?4,?5,'path',?6,NULL,'verified',?7,?8,'{}',?9,NULL,1,?10,?9,?9)"}else{"UPDATE host_bindings SET host_path=?6,generation=?7,availability=?8,verified_at=?9,updated_at=?9,local_revision=?10 WHERE binding_id=?1 AND device_id=?2 AND library_id=?3 AND storage_root_id=?4 AND host_kind=?5 AND local_revision=?11"}).bind(binding.id.uuid().as_bytes().to_vec()).bind(self.info.device_id.bytes()).bind(binding.library_id.bytes()).bind(binding.root_id.bytes()).bind(host).bind(path).bind(binding.generation.get()).bind(availability).bind(at.get()).bind(binding.revision.get()).bind(expected.map(Revision::get)).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Explicit selection uses a separate CAS token from binding observations.
    /// # Errors
    /// Rejects stale selection and candidates from another device or logical root.
    pub async fn select_path_binding(
        &self,
        actor: LocalPrincipalId,
        binding: &PathBinding,
        expected: Option<Revision>,
        at: Timestamp,
    ) -> Result<Revision> {
        let next = expected.map_or(Ok(Revision::INITIAL), Revision::next)?;
        let mut tx = self.write().await?;
        local_authority(&mut tx, binding.library_id, actor).await?;
        let result=sqlx::query(if expected.is_none(){"INSERT INTO host_binding_selections VALUES(?1,?2,?3,?4,?5,?6)"}else{"UPDATE host_binding_selections SET binding_id=?4,selection_revision=?5,updated_at=?6 WHERE device_id=?1 AND library_id=?2 AND storage_root_id=?3 AND selection_revision=?7"}).bind(self.info.device_id.bytes()).bind(binding.library_id.bytes()).bind(binding.root_id.bytes()).bind(binding.id.uuid().as_bytes().to_vec()).bind(next.get()).bind(at.get()).bind(expected.map(Revision::get)).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.commit().await?;
        Ok(next)
    }
    /// Resolves only the selected exact generation. The host callback cannot hold
    /// the database lock; a concurrent rebind discards its returned handle.
    /// # Errors
    /// Rejects missing selection, denied rights, host failure or a concurrent rebind.
    pub async fn resolve_binding<H: BindingHost>(
        &self,
        actor: LocalPrincipalId,
        binding: &PathBinding,
        relative: &RelativeComponents,
        rights: &ResourceRights,
        host: &H,
    ) -> Result<H::Handle> {
        self.check_resolution(actor, binding, rights).await?;
        let handle = host.resolve(binding, relative, rights)?;
        self.check_resolution(actor, binding, rights).await?;
        Ok(handle)
    }
    async fn check_resolution(
        &self,
        actor: LocalPrincipalId,
        binding: &PathBinding,
        rights: &ResourceRights,
    ) -> Result<()> {
        let mut tx = self.read().await?;
        local_authority(&mut tx, binding.library_id, actor).await?;
        let row=sqlx::query("SELECT b.host_path,l.supported_rights_json FROM host_bindings b JOIN host_binding_selections s USING(device_id,library_id,storage_root_id,binding_id) JOIN storage_location_specs l USING(library_id,storage_root_id) JOIN storage_roots r USING(library_id,storage_root_id) WHERE b.binding_id=? AND b.device_id=? AND b.library_id=? AND b.storage_root_id=? AND b.generation=? AND b.state='verified' AND b.availability='available' AND r.state='active' AND l.storage_kind='filesystem'").bind(binding.id.uuid().as_bytes().to_vec()).bind(self.info.device_id.bytes()).bind(binding.library_id.bytes()).bind(binding.root_id.bytes()).bind(binding.generation.get()).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
        let allowed: ResourceRights =
            serde_json::from_str(&row.try_get::<String, _>("supported_rights_json")?)
                .map_err(|_| Error::Corrupt)?;
        if row.try_get::<String, _>("host_path")? != binding.path.to_str().ok_or(Error::Invalid)?
            || !allowed.permits(rights)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FakeHost;
    impl BindingHost for FakeHost {
        type Handle = usize;
        fn resolve(
            &self,
            _: &PathBinding,
            relative: &RelativeComponents,
            _: &ResourceRights,
        ) -> Result<usize> {
            Ok(relative.parts().len())
        }
    }
    #[tokio::test]
    async fn storage_selection_rebind_rights_and_slot_cas() {
        use photara_core::contracts::resource::ResourceRight;
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let at = Timestamp::try_from(1_789_142_400_000).unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("state.sqlite"), at)
            .await
            .unwrap();
        let root = StorageRoot {
            meta: Metadata::new(StorageRootId::new(), id.library_id, at),
            display_name: "Sources".into(),
            purpose: "photara.source".into(),
        };
        let rights = ResourceRights::new(vec![ResourceRight::Read]).unwrap();
        store
            .put_location_spec(
                id.principal_id,
                &LocationSpec {
                    root: root.clone(),
                    kind: LocationKindSpec::Filesystem,
                    rights: rights.clone(),
                },
                None,
            )
            .await
            .unwrap();
        let mut slot = SlotSpec {
            id: StorageSlotId::from_uuid(Uuid::new_v4()).unwrap(),
            library_id: id.library_id,
            name: LocalName::parse("sources").unwrap(),
            display_name: "Sources".into(),
            root_id: root.meta.id,
            revision: Revision::INITIAL,
            tombstoned: false,
        };
        store
            .put_slot(id.principal_id, &slot, None, at)
            .await
            .unwrap();
        slot.revision = Revision::try_from(2).unwrap();
        slot.name = LocalName::parse("photos").unwrap();
        store
            .put_slot(id.principal_id, &slot, Some(Revision::INITIAL), at)
            .await
            .unwrap();
        let mut binding = PathBinding {
            id: HostBindingId::from_uuid(Uuid::new_v4()).unwrap(),
            library_id: id.library_id,
            root_id: root.meta.id,
            path: "/private/tmp/fake-host".into(),
            revision: Revision::INITIAL,
            generation: Revision::INITIAL,
            available: true,
        };
        store
            .put_path_binding(id.principal_id, &binding, None, at)
            .await
            .unwrap();
        let relative = RelativeComponents::new(vec!["image.jpg".into()]).unwrap();
        assert!(
            store
                .resolve_binding(id.principal_id, &binding, &relative, &rights, &FakeHost)
                .await
                .is_err()
        );
        store
            .select_path_binding(id.principal_id, &binding, None, at)
            .await
            .unwrap();
        assert_eq!(
            store
                .resolve_binding(id.principal_id, &binding, &relative, &rights, &FakeHost)
                .await
                .unwrap(),
            1
        );
        let stale = binding.clone();
        binding.path = "/private/tmp/fake-rebind".into();
        binding.revision = Revision::try_from(2).unwrap();
        binding.generation = Revision::try_from(2).unwrap();
        store
            .put_path_binding(id.principal_id, &binding, Some(Revision::INITIAL), at)
            .await
            .unwrap();
        assert!(
            store
                .resolve_binding(id.principal_id, &stale, &relative, &rights, &FakeHost)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .resolve_binding(id.principal_id, &binding, &relative, &rights, &FakeHost)
                .await
                .unwrap(),
            1
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}
