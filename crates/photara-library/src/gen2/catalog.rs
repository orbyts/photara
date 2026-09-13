use super::*;

impl LocalLibraryStore {
    /// # Errors
    /// Returns validation, CAS, label collision or live-locator errors.
    pub async fn put_storage_root(
        &self,
        value: &StorageRoot,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::name(&value.display_name, 512)?;
        validation::identifier(&value.purpose)?;
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Root, m, expected).await?;
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO storage_roots(storage_root_id,library_id,display_name,label_key,purpose,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms) VALUES(?,?,?,?,?,?,?,?,?,?)"} else {"UPDATE storage_roots SET storage_root_id=?1,library_id=?2,display_name=?3,label_key=?4,purpose=?5,local_revision=?6,created_at_ms=?7,updated_at_ms=?8,state=?9,retired_at_ms=?10 WHERE storage_root_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(&value.display_name).bind(normalize_term(&value.display_name)?).bind(&value.purpose).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).execute(&mut *tx).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Root,
            m,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Changes discovery visibility only. Device selection has a separate method.
    /// # Errors
    /// Rejects invalid selection, CAS or Library state.
    pub async fn put_catalog(
        &self,
        value: &CatalogEntry,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Catalog, m, expected).await?;
        let current = catalog_row(&mut tx, m.library_id, m.id).await?;
        if value.active_locator_id != current.as_ref().and_then(|r| r.active_locator_id)
            || value.selected_observation_id
                != current.as_ref().and_then(|r| r.selected_observation_id)
        {
            return Err(Error::Invalid);
        }
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO project_catalog(library_id,project_id,visibility,local_revision,created_at_ms,updated_at_ms) VALUES(?,?,?,?,?,?)"} else {"UPDATE project_catalog SET library_id=?1,project_id=?2,visibility=?3,local_revision=?4,created_at_ms=?5,updated_at_ms=?6 WHERE project_id=?2 AND library_id=?1"})
            .bind(m.library_id.bytes()).bind(m.id.bytes()).bind(value.visibility.sql()).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).execute(&mut *tx).await?;
        let portable = serde_json::json!({"meta":m,"visibility":value.visibility});
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Catalog,
            m,
            expected,
            &portable,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Rooted locators carry safe relative hints; no filesystem resolution occurs.
    /// Tombstoning maps to the locator-specific `retired` state.
    /// # Errors
    /// Returns path, CAS, same-Library root/project or selected-locator errors.
    pub async fn put_locator(
        &self,
        value: &ProjectLocator,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        if let Some(root) = &value.rooted {
            validation::relative_path(&root.relative_path)?;
        }
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Locator, m, expected).await?;
        if let Some(old) = locator_row(&mut tx, m.library_id, m.id).await?
            && old.project_id != value.project_id
        {
            return Err(Error::Invalid);
        }
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO project_locators(locator_id,library_id,project_id,storage_root_id,relative_path,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms) VALUES(?,?,?,?,?,?,?,?,?,?)"} else {"UPDATE project_locators SET locator_id=?1,library_id=?2,project_id=?3,storage_root_id=?4,relative_path=?5,local_revision=?6,created_at_ms=?7,updated_at_ms=?8,state=?9,retired_at_ms=?10 WHERE locator_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(value.project_id.bytes()).bind(value.rooted.as_ref().map(|r|r.storage_root_id.bytes())).bind(value.rooted.as_ref().map(|r|&r.relative_path)).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(if m.state==Lifecycle::Active {"active"} else {"retired"}).bind(m.retired_at()).execute(&mut *tx).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Locator,
            m,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// # Errors
    /// Returns storage or typed decoding failure.
    pub async fn catalog_entry(
        &self,
        library: LibraryId,
        project: ProjectId,
    ) -> Result<Option<CatalogEntry>> {
        let mut tx = self.read().await?;
        catalog_row(&mut tx, library, project).await
    }
    /// # Errors
    /// Returns storage or typed decoding failure.
    pub async fn locator(
        &self,
        library: LibraryId,
        id: LocatorId,
    ) -> Result<Option<ProjectLocator>> {
        let mut tx = self.read().await?;
        locator_row(&mut tx, library, id).await
    }
    /// # Errors
    /// Returns invalid bounds or storage/typed decoding failure.
    pub async fn catalog_entries(
        &self,
        library: LibraryId,
        after: Option<ProjectId>,
        limit: u32,
    ) -> Result<Vec<CatalogEntry>> {
        page(0, limit)?;
        sqlx::query("SELECT *, 'active' AS state FROM project_catalog WHERE library_id=? AND project_id>? ORDER BY project_id LIMIT ?")
            .bind(library.bytes()).bind(after.map_or_else(Vec::new,ProjectId::bytes)).bind(limit).fetch_all(self.db.pool()).await?.iter().map(decode_catalog).collect()
    }
    /// # Errors
    /// Returns invalid bounds or storage/typed decoding failure.
    pub async fn storage_roots(
        &self,
        library: LibraryId,
        after: Option<StorageRootId>,
        limit: u32,
        retired: bool,
    ) -> Result<Vec<StorageRoot>> {
        page(0, limit)?;
        sqlx::query("SELECT * FROM storage_roots WHERE library_id=? AND storage_root_id>? AND (? OR state='active') ORDER BY storage_root_id LIMIT ?")
            .bind(library.bytes()).bind(after.map_or_else(Vec::new,StorageRootId::bytes)).bind(retired).bind(limit).fetch_all(self.db.pool()).await?.iter().map(|row|Ok(StorageRoot {meta:metadata(row,"storage_root_id")?,display_name:row.try_get("display_name")?,purpose:row.try_get("purpose")?})).collect()
    }
    /// Writes only a device-local path or opaque secure-store reference, with CAS.
    /// The host validates mount/security-scoped access separately; nothing is opened.
    /// # Errors
    /// Rejects invalid paths, stale revision or inactive/foreign roots.
    pub async fn put_root_binding(
        &self,
        value: &RootBinding,
        expected: Option<Revision>,
    ) -> Result<()> {
        let (kind, path, secure) = match &value.binding {
            DeviceBinding::Path(path) => {
                if !path.is_absolute()
                    || path
                        .components()
                        .any(|p| !matches!(p, Component::RootDir | Component::Normal(_)))
                {
                    return Err(Error::Invalid);
                }
                let text = path.to_str().ok_or(Error::Invalid)?;
                validation::bounded(text, 4096)?;
                ("path", Some(text.to_owned()), None)
            }
            DeviceBinding::Bookmark(id) => ("bookmark", None, Some(id.bytes())),
            DeviceBinding::Provider(id) => ("provider", None, Some(id.bytes())),
        };
        let mut tx = self.write().await?;
        if !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM storage_roots r JOIN libraries w ON w.library_id=r.library_id WHERE r.library_id=? AND r.storage_root_id=? AND r.state='active' AND w.state='active')").bind(value.library_id.bytes()).bind(value.storage_root_id.bytes()).fetch_one(&mut *tx).await? {return Err(Error::Constraint);}
        let previous:Option<i64>=sqlx::query_scalar("SELECT local_revision FROM device_root_bindings WHERE device_id=? AND storage_root_id=?").bind(self.info.device_id.bytes()).bind(value.storage_root_id.bytes()).fetch_optional(&mut *tx).await?;
        if previous != expected.map(Revision::get)
            || value.revision != expected.map_or(Ok(Revision::INITIAL), Revision::next)?
        {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO device_root_bindings(device_id,library_id,storage_root_id,binding_kind,host_path,secure_handle_ref,local_revision,updated_at_ms) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(device_id,storage_root_id) DO UPDATE SET binding_kind=excluded.binding_kind,host_path=excluded.host_path,secure_handle_ref=excluded.secure_handle_ref,local_revision=excluded.local_revision,updated_at_ms=excluded.updated_at_ms")
            .bind(self.info.device_id.bytes()).bind(value.library_id.bytes()).bind(value.storage_root_id.bytes()).bind(kind).bind(path).bind(secure).bind(value.revision.get()).bind(value.updated_at.get()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// # Errors
    /// Returns storage or typed decoding failure.
    pub async fn root_binding(
        &self,
        library: LibraryId,
        root: StorageRootId,
    ) -> Result<Option<RootBinding>> {
        let row=sqlx::query("SELECT * FROM device_root_bindings WHERE device_id=? AND library_id=? AND storage_root_id=?").bind(self.info.device_id.bytes()).bind(library.bytes()).bind(root.bytes()).fetch_optional(self.db.pool()).await?;
        row.map(|row| {
            let binding = match row.try_get::<String, _>("binding_kind")?.as_str() {
                "path" => DeviceBinding::Path(std::path::PathBuf::from(
                    row.try_get::<String, _>("host_path")?,
                )),
                "bookmark" => DeviceBinding::Bookmark(SecureHandleId::from_bytes(
                    &row.try_get::<Vec<u8>, _>("secure_handle_ref")?,
                )?),
                "provider" => DeviceBinding::Provider(SecureHandleId::from_bytes(
                    &row.try_get::<Vec<u8>, _>("secure_handle_ref")?,
                )?),
                _ => return Err(Error::Unsupported),
            };
            Ok(RootBinding {
                library_id: library,
                storage_root_id: root,
                binding,
                revision: Revision::try_from(row.try_get::<i64, _>("local_revision")?)?,
                updated_at: Timestamp::try_from(row.try_get::<i64, _>("updated_at_ms")?)?,
            })
        })
        .transpose()
    }
    /// Validates a read-only directory package with L1 before projecting its summary.
    /// This never saves the package or auto-selects a duplicate locator. The caller
    /// supplies an explicitly authorized root; verification runs outside the DB lock.
    /// # Errors
    /// Rejects unverified bytes, wrong identity, retired locator or database failure.
    pub async fn observe_package(
        &self,
        library: LibraryId,
        locator: LocatorId,
        root: impl AsRef<Path>,
        at: Timestamp,
    ) -> Result<ProjectObservation> {
        let root = root.as_ref().to_owned();
        let initial = self
            .locator(library, locator)
            .await?
            .ok_or(Error::Constraint)?;
        let rooted = initial.rooted.as_ref().ok_or(Error::Unsupported)?;
        let root_binding = self
            .root_binding(library, rooted.storage_root_id)
            .await?
            .ok_or(Error::Constraint)?;
        let DeviceBinding::Path(host_root) = &root_binding.binding else {
            return Err(Error::Unsupported);
        };
        if host_root.join(&rooted.relative_path) != root {
            return Err(Error::Invalid);
        }
        let package = tokio::task::spawn_blocking(move || {
            photara_store::package::validate_directory(
                root,
                photara_store::package::PackageLimits::default(),
            )
        })
        .await
        .map_err(|_| Error::Storage)?
        .map_err(|_| Error::Corrupt)?;
        let mut tx = self.write().await?;
        let binding = locator_row(&mut tx, library, locator)
            .await?
            .ok_or(Error::Constraint)?;
        if binding != initial {
            return Err(Error::Conflict);
        }
        let current_revision:Option<i64>=sqlx::query_scalar("SELECT local_revision FROM device_root_bindings WHERE device_id=? AND library_id=? AND storage_root_id=?").bind(self.info.device_id.bytes()).bind(library.bytes()).bind(rooted.storage_root_id.bytes()).fetch_optional(&mut *tx).await?;
        if current_revision != Some(root_binding.revision.get()) {
            return Err(Error::Conflict);
        }
        if binding.meta.state != Lifecycle::Active
            || binding.project_id.uuid() != package.authored.project_id.as_uuid()
        {
            return Err(Error::Constraint);
        }
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM libraries WHERE library_id=? AND state='active')",
        )
        .bind(library.bytes())
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(Error::Constraint);
        }
        let inventory = package
            .objects
            .get(&package.authored.asset_inventory.sha256)
            .ok_or(Error::Corrupt)?;
        let asset_count = u64::try_from(
            inventory
                .value
                .get("assets")
                .and_then(serde_json::Value::as_array)
                .ok_or(Error::Corrupt)?
                .len(),
        )
        .map_err(|_| Error::Limit)?;
        let commit = package.commits.first().ok_or(Error::Corrupt)?;
        let commit_id = CommitId::try_from(
            Uuid::parse_str(&commit.commit_id.to_string()).map_err(|_| Error::Corrupt)?,
        )?;
        let digest = unhex(package.head.commit_sha256.as_str())?;
        if let Some(row)=sqlx::query("SELECT * FROM project_observations WHERE locator_id=? AND commit_id=? AND commit_sha256=? AND index_schema=1").bind(locator.bytes()).bind(commit_id.bytes()).bind(digest.to_vec()).fetch_optional(&mut *tx).await? {return decode_observation(&row);}
        let observation = ProjectObservation {
            id: ObservationId::new(),
            library_id: library,
            project_id: binding.project_id,
            locator_id: locator,
            commit_id,
            commit_sha256: digest,
            package_revision: commit.package_revision.get(),
            title: package.authored.title.clone(),
            lifecycle: package.authored.lifecycle.clone(),
            asset_count,
            graph_count: u64::try_from(package.graphs.len()).map_err(|_| Error::Limit)?,
            observed_at: at,
        };
        sqlx::query("INSERT INTO project_observations(observation_id,library_id,project_id,locator_id,commit_id,commit_sha256,package_revision,title,project_lifecycle,asset_count,graph_count,observed_at_ms,index_schema) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,1)")
            .bind(observation.id.bytes()).bind(library.bytes()).bind(binding.project_id.bytes()).bind(locator.bytes()).bind(commit_id.bytes()).bind(digest.to_vec()).bind(observation.package_revision.to_string()).bind(&observation.title).bind(&observation.lifecycle).bind(i64::try_from(asset_count).map_err(|_|Error::Limit)?).bind(i64::try_from(observation.graph_count).map_err(|_|Error::Limit)?).bind(at.get()).execute(&mut *tx).await?;
        for graph in &package.graphs {
            let graph_id =
                Uuid::parse_str(&graph.graph_id.to_string()).map_err(|_| Error::Corrupt)?;
            let graph_bytes = canonical(&graph.graph)?;
            sqlx::query("INSERT INTO project_graph_projection(observation_id,graph_id,graph_name,graph_digest,graph_revision) VALUES(?,?,?,?,?)").bind(observation.id.bytes()).bind(graph_id.as_bytes().to_vec()).bind(&graph.name).bind(sha(graph_bytes.as_bytes()).to_vec()).bind(graph.graph.revision.get().to_string()).execute(&mut *tx).await?;
        }
        project_context(&mut tx, &package, observation.id).await?;
        // Available is a historical verified identity observation, never a promise
        // of continuing access. Device paths and cloud reports are not inferred.
        sqlx::query("INSERT INTO device_project_bindings(device_id,library_id,project_id,locator_id,availability,verified_project_id,last_commit_id,last_commit_sha256,checked_at_ms) VALUES(?,?,?,?,'available',?,?,?,?) ON CONFLICT(device_id,locator_id) DO UPDATE SET availability=excluded.availability,verified_project_id=excluded.verified_project_id,last_commit_id=excluded.last_commit_id,last_commit_sha256=excluded.last_commit_sha256,checked_at_ms=excluded.checked_at_ms")
            .bind(self.info.device_id.bytes()).bind(library.bytes()).bind(binding.project_id.bytes()).bind(locator.bytes()).bind(binding.project_id.bytes()).bind(commit_id.bytes()).bind(digest.to_vec()).bind(at.get()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(observation)
    }
    /// Explicitly selects or clears a verified observation at catalog CAS.
    /// No timestamp/newest-wins rule; this local pointer never enters mutations.
    /// # Errors
    /// Returns stale CAS, cross-project/locator or retired-reference errors.
    pub async fn select_observation(
        &self,
        library: LibraryId,
        project: ProjectId,
        expected: Revision,
        observation: Option<ObservationId>,
        at: Timestamp,
    ) -> Result<CatalogEntry> {
        let mut tx = self.write().await?;
        let mut entry = catalog_row(&mut tx, library, project)
            .await?
            .ok_or(Error::Conflict)?;
        if entry.meta.revision != expected {
            return Err(Error::Conflict);
        }
        entry.meta.advance(at)?;
        check_meta(&mut tx, Table::Catalog, &entry.meta, Some(expected)).await?;
        let locator = if let Some(observation) = observation {
            let bytes:Vec<u8>=sqlx::query_scalar("SELECT locator_id FROM project_observations WHERE library_id=? AND project_id=? AND observation_id=?").bind(library.bytes()).bind(project.bytes()).bind(observation.bytes()).fetch_optional(&mut *tx).await?.ok_or(Error::Constraint)?;
            Some(LocatorId::from_bytes(&bytes)?)
        } else {
            None
        };
        sqlx::query("UPDATE project_catalog SET active_locator_id=?,selected_observation_id=?,local_revision=?,updated_at_ms=? WHERE library_id=? AND project_id=? AND local_revision=?")
            .bind(locator.map(LocatorId::bytes)).bind(observation.map(ObservationId::bytes)).bind(entry.meta.revision.get()).bind(at.get()).bind(library.bytes()).bind(project.bytes()).bind(expected.get()).execute(&mut *tx).await?;
        entry.active_locator_id = locator;
        entry.selected_observation_id = observation;
        tx.commit().await?;
        Ok(entry)
    }
}
async fn catalog_row(
    conn: &mut SqliteConnection,
    library: LibraryId,
    project: ProjectId,
) -> Result<Option<CatalogEntry>> {
    sqlx::query(
        "SELECT *, 'active' AS state FROM project_catalog WHERE library_id=? AND project_id=?",
    )
    .bind(library.bytes())
    .bind(project.bytes())
    .fetch_optional(conn)
    .await?
    .as_ref()
    .map(decode_catalog)
    .transpose()
}
fn decode_catalog(row: &sqlx::sqlite::SqliteRow) -> Result<CatalogEntry> {
    Ok(CatalogEntry {
        meta: metadata(row, "project_id")?,
        visibility: Visibility::parse(&row.try_get::<String, _>("visibility")?)?,
        active_locator_id: row
            .try_get::<Option<Vec<u8>>, _>("active_locator_id")?
            .map(|b| LocatorId::from_bytes(&b))
            .transpose()?,
        selected_observation_id: row
            .try_get::<Option<Vec<u8>>, _>("selected_observation_id")?
            .map(|b| ObservationId::from_bytes(&b))
            .transpose()?,
    })
}
async fn locator_row(
    conn: &mut SqliteConnection,
    library: LibraryId,
    id: LocatorId,
) -> Result<Option<ProjectLocator>> {
    sqlx::query("SELECT * FROM project_locators WHERE library_id=? AND locator_id=?")
        .bind(library.bytes())
        .bind(id.bytes())
        .fetch_optional(conn)
        .await?
        .map(|row| {
            let rooted = match (
                row.try_get::<Option<Vec<u8>>, _>("storage_root_id")?,
                row.try_get::<Option<String>, _>("relative_path")?,
            ) {
                (Some(id), Some(relative_path)) => Some(RootedPath {
                    storage_root_id: StorageRootId::from_bytes(&id)?,
                    relative_path,
                }),
                (None, None) => None,
                _ => return Err(Error::Corrupt),
            };
            Ok(ProjectLocator {
                meta: metadata(&row, "locator_id")?,
                project_id: ProjectId::from_bytes(&row.try_get::<Vec<u8>, _>("project_id")?)?,
                rooted,
            })
        })
        .transpose()
}
fn decode_observation(row: &sqlx::sqlite::SqliteRow) -> Result<ProjectObservation> {
    Ok(ProjectObservation {
        id: ObservationId::from_bytes(&row.try_get::<Vec<u8>, _>("observation_id")?)?,
        library_id: LibraryId::from_bytes(&row.try_get::<Vec<u8>, _>("library_id")?)?,
        project_id: ProjectId::from_bytes(&row.try_get::<Vec<u8>, _>("project_id")?)?,
        locator_id: LocatorId::from_bytes(&row.try_get::<Vec<u8>, _>("locator_id")?)?,
        commit_id: CommitId::from_bytes(&row.try_get::<Vec<u8>, _>("commit_id")?)?,
        commit_sha256: row
            .try_get::<Vec<u8>, _>("commit_sha256")?
            .try_into()
            .map_err(|_| Error::Corrupt)?,
        package_revision: row
            .try_get::<String, _>("package_revision")?
            .parse()
            .map_err(|_| Error::Corrupt)?,
        title: row.try_get("title")?,
        lifecycle: row.try_get("project_lifecycle")?,
        asset_count: u64::try_from(row.try_get::<i64, _>("asset_count")?)
            .map_err(|_| Error::Corrupt)?,
        graph_count: u64::try_from(row.try_get::<i64, _>("graph_count")?)
            .map_err(|_| Error::Corrupt)?,
        observed_at: Timestamp::try_from(row.try_get::<i64, _>("observed_at_ms")?)?,
    })
}
fn unhex(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 || !value.is_ascii() {
        return Err(Error::Corrupt);
    }
    let mut result = [0; 32];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| Error::Corrupt)?;
    }
    Ok(result)
}

async fn project_context(
    conn: &mut SqliteConnection,
    package: &photara_store::package::ValidatedPackage,
    id: ObservationId,
) -> Result<()> {
    let object = |reference: &serde_json::Value| -> Result<&serde_json::Value> {
        let r: photara_store::package::ObjectRef =
            serde_json::from_value(reference.clone()).map_err(|_| Error::Corrupt)?;
        package
            .objects
            .get(&r.sha256)
            .map(|o| &o.value)
            .ok_or(Error::Corrupt)
    };
    let parties = &package
        .objects
        .get(&package.authored.party_assignments.sha256)
        .ok_or(Error::Corrupt)?
        .value;
    for reference in parties["assignments"].as_array().ok_or(Error::Corrupt)? {
        let assignment = object(reference)?;
        let snapshot = object(&assignment["party_snapshot"])?;
        let source = &snapshot["source"];
        sqlx::query("INSERT INTO project_party_projection(observation_id,assignment_id,source_library_id,source_kind,source_record_id,source_revision,display_name_snapshot,roles_json) VALUES(?,?,?,?,?,?,?,?)")
            .bind(id.bytes()).bind(json_uuid(&assignment["assignment_id"])?).bind(json_uuid(&source["library_id"])?).bind(json_text(&source["kind"])?).bind(json_uuid(&source["record_id"])?).bind(canonical(&source["revision"])?).bind(json_text(&snapshot["display_name"])?).bind(canonical(&assignment["roles"])?).execute(&mut *conn).await?;
    }
    let locations = &package
        .objects
        .get(&package.authored.location_assignments.sha256)
        .ok_or(Error::Corrupt)?
        .value;
    for reference in locations["assignments"].as_array().ok_or(Error::Corrupt)? {
        let assignment = object(reference)?;
        let location = object(&assignment["location_snapshot"])?;
        let kind = object(&assignment["location_kind_snapshot"])?;
        let schedule = assignment
            .get("schedule")
            .filter(|v| !v.is_null())
            .map(canonical)
            .transpose()?;
        sqlx::query("INSERT INTO project_location_projection(observation_id,assignment_id,source_library_id,location_id,location_kind_id,location_name_snapshot,kind_name_snapshot,schedule_json) VALUES(?,?,?,?,?,?,?,?)")
            .bind(id.bytes()).bind(json_uuid(&assignment["assignment_id"])?).bind(json_uuid(&location["source"]["library_id"])?).bind(json_uuid(&location["source"]["record_id"])?).bind(json_uuid(&kind["source"]["record_id"])?).bind(json_text(&location["display_name"])?).bind(json_text(&kind["display_name"])?).bind(schedule).execute(&mut *conn).await?;
    }
    Ok(())
}
fn json_text(value: &serde_json::Value) -> Result<&str> {
    value.as_str().ok_or(Error::Corrupt)
}
fn json_uuid(value: &serde_json::Value) -> Result<Vec<u8>> {
    Ok(Uuid::parse_str(json_text(value)?)
        .map_err(|_| Error::Corrupt)?
        .as_bytes()
        .to_vec())
}
