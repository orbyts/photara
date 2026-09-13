//! Typed Library context aggregates. Project context stays in its package.
use super::local::local_authority;
use super::*;
use photara_core::context::{
    expression::{Coordinate, FieldMode},
    value::Type,
    variable::{
        Binding, VariableAggregate, VariableSpec, VariableState, validate_transition,
        variable_order,
    },
};
use photara_core::contracts::{LocalPrincipalId, VariableId, dto::ScopeRef, schema::LocalRevision};
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
struct VariableProvenance {
    ty: Type,
    definition_version: photara_core::contracts::schema::Version,
    owner_revision: photara_core::contracts::dto::RevisionCoordinate,
    created_at: photara_core::context::value::Timestamp,
    updated_at: photara_core::context::value::Timestamp,
    origin: photara_core::context::variable::ValueOrigin,
}
impl LocalLibraryStore {
    /// Reads complete checked aggregates, including reserved names and current values.
    /// # Errors
    /// Returns denied scope, bounds or corrupt stored aggregates.
    pub async fn library_variables(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
    ) -> Result<Vec<VariableAggregate>> {
        let mut tx = self.read().await?;
        local_authority(&mut tx, library, actor).await?;
        read_variables(&mut tx, library).await
    }
    /// Replaces a complete definition/default/value/name/expression aggregate under
    /// one CAS. Dependencies are immutable children created with their expression.
    /// # Errors
    /// Rejects Project scope, stale revisions, name conflicts, cycles and invalid types.
    pub async fn put_library_variable(
        &self,
        actor: LocalPrincipalId,
        value: &VariableAggregate,
        expected: Option<LocalRevision>,
    ) -> Result<()> {
        let library = LibraryId::try_from(value.spec().owning_library_id.uuid())?;
        let mut tx = self.write().await?;
        local_authority(&mut tx, library, actor).await?;
        put_variable(&mut tx, library, value, expected).await?;
        tx.commit().await?;
        Ok(())
    }
}
pub(super) async fn put_variable(
    conn: &mut SqliteConnection,
    library: LibraryId,
    value: &VariableAggregate,
    expected: Option<LocalRevision>,
) -> Result<()> {
    let spec = value.spec();
    if spec.scope
        != (ScopeRef::Library {
            library_id: spec.owning_library_id,
        })
        || spec.owning_library_id.uuid() != library.uuid()
    {
        return Err(Error::Invalid);
    }
    let mut all = read_variables(conn, library).await?;
    let old = all
        .iter()
        .position(|v| v.spec().variable_id == spec.variable_id);
    match (old, expected) {
        (None, None)
            if spec.revision == LocalRevision::INITIAL
                && spec.state == VariableState::Active
                && spec.created_at == spec.updated_at => {}
        (Some(i), Some(base)) => {
            validate_transition(&all[i], value, base, &all[i].spec().owner_revision)
                .map_err(|_| Error::Conflict)?;
            all.remove(i);
        }
        _ => return Err(Error::Conflict),
    }
    all.push(value.clone());
    variable_order(&all).map_err(|_| Error::Constraint)?;
    let created = timestamp_millis(&spec.created_at)?;
    let updated = timestamp_millis(&spec.updated_at)?;
    let provenance = VariableProvenance {
        ty: spec.ty.clone(),
        definition_version: spec.definition_version,
        owner_revision: spec.owner_revision.clone(),
        created_at: spec.created_at.clone(),
        updated_at: spec.updated_at.clone(),
        origin: spec.origin,
    };
    let overrides = if spec.allow_run_override {
        "[\"run\"]"
    } else {
        "[]"
    };
    let result=sqlx::query("INSERT INTO library_variables VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,1,?18,?19,?20) ON CONFLICT(variable_id) DO UPDATE SET current_name=excluded.current_name,display_name=excluded.display_name,description=excluded.description,default_binding_json=excluded.default_binding_json,allowed_overrides_json=excluded.allowed_overrides_json,sensitivity=excluded.sensitivity,portability=excluded.portability,provenance_json=excluded.provenance_json,state=excluded.state,retired_at=excluded.retired_at,local_revision=excluded.local_revision,updated_at=excluded.updated_at WHERE library_variables.library_id=excluded.library_id AND library_variables.local_revision=?21")
        .bind(spec.variable_id.uuid().as_bytes().to_vec()).bind(library.bytes()).bind(spec.namespace.as_str()).bind(spec.name.as_str()).bind(&spec.label).bind(&spec.description).bind(spec.ty.value_type.id.as_str()).bind(i64::from(spec.ty.value_type.version.get())).bind(spec.ty.schema.id.as_str()).bind(i64::from(spec.ty.schema.version.get())).bind(spec.default.as_ref().map(canonical).transpose()?).bind(overrides).bind(enum_name(&spec.sensitivity)?).bind(enum_name(&spec.portability)?).bind(canonical(&provenance)?).bind(if spec.state==VariableState::Active{"active"}else{"tombstoned"}).bind((spec.state==VariableState::Tombstoned).then_some(updated)).bind(spec.revision.get()).bind(created).bind(updated).bind(expected.map(LocalRevision::get)).execute(&mut *conn).await?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    for name in &spec.claimed_names {
        sqlx::query("INSERT INTO library_variable_names VALUES(?,?,?,?) ON CONFLICT(library_id,name) DO NOTHING").bind(library.bytes()).bind(name.as_str()).bind(spec.variable_id.uuid().as_bytes().to_vec()).bind(updated).execute(&mut *conn).await?;
        let target: Vec<u8> = sqlx::query_scalar(
            "SELECT variable_id FROM library_variable_names WHERE library_id=? AND name=?",
        )
        .bind(library.bytes())
        .bind(name.as_str())
        .fetch_one(&mut *conn)
        .await?;
        if target != spec.variable_id.uuid().as_bytes() {
            return Err(Error::Conflict);
        }
    }
    if let Some(id) = spec.value_id {
        sqlx::query("INSERT INTO library_variable_values VALUES(?,?,?,?,?,?,NULL,NULL,NULL,NULL,?) ON CONFLICT(library_id,variable_id) DO UPDATE SET is_present=excluded.is_present,binding_json=excluded.binding_json,origin=excluded.origin,updated_at=excluded.updated_at").bind(library.bytes()).bind(spec.variable_id.uuid().as_bytes().to_vec()).bind(id.uuid().as_bytes().to_vec()).bind(spec.current.is_some()).bind(spec.current.as_ref().map(canonical).transpose()?).bind(enum_name(&spec.origin)?).bind(updated).execute(&mut *conn).await?;
    }
    put_expressions(conn, library, spec, updated).await
}
async fn put_expressions(
    conn: &mut SqliteConnection,
    library: LibraryId,
    spec: &VariableSpec,
    updated: i64,
) -> Result<()> {
    for binding in [&spec.default, &spec.current].into_iter().flatten() {
        if let Binding::Expression(expression) = binding {
            let ast = canonical(&expression.ast)?.into_bytes();
            let existing=sqlx::query("SELECT source_utf8,ast_canonical,library_id,owner_variable_id FROM library_expressions WHERE expression_id=?").bind(expression.expression_id.uuid().as_bytes().to_vec()).fetch_optional(&mut *conn).await?;
            if let Some(row) = existing {
                if row.try_get::<Vec<u8>, _>("source_utf8")? != expression.source.as_bytes()
                    || row.try_get::<Vec<u8>, _>("ast_canonical")? != ast
                    || row.try_get::<Vec<u8>, _>("library_id")? != library.bytes()
                    || row.try_get::<Vec<u8>, _>("owner_variable_id")?
                        != spec.variable_id.uuid().as_bytes()
                {
                    return Err(Error::Conflict);
                }
                continue;
            }
            sqlx::query("INSERT INTO library_expressions VALUES(?,?,?,'photara.expression.v1',?,?,?,?,?,?,?,?,?)").bind(expression.expression_id.uuid().as_bytes().to_vec()).bind(library.bytes()).bind(spec.variable_id.uuid().as_bytes().to_vec()).bind(match expression.mode{FieldMode::Expression=>"expression",FieldMode::Template=>"template",FieldMode::LiteralOnly=>return Err(Error::Invalid)}).bind(expression.compiler_version.get().to_string()).bind(expression.source.as_bytes()).bind(sha(expression.source.as_bytes()).to_vec()).bind(&ast).bind(sha(&ast).to_vec()).bind(spec.ty.value_type.id.as_str()).bind(i64::from(spec.ty.value_type.version.get())).bind(updated).execute(&mut *conn).await?;
            for (ordinal, dep) in expression.dependencies.iter().enumerate() {
                let (kind, variable, slot) = match dep.coordinate {
                    Coordinate::Variable {
                        scope: ScopeRef::Library { library_id },
                        variable_id,
                    } if library_id.uuid() == library.uuid() => (
                        "variable",
                        Some(variable_id.uuid().as_bytes().to_vec()),
                        None,
                    ),
                    Coordinate::Slot {
                        library_id,
                        slot_id,
                    } if library_id.uuid() == library.uuid() => (
                        "storage-slot",
                        None,
                        Some(slot_id.uuid().as_bytes().to_vec()),
                    ),
                    _ => return Err(Error::Invalid),
                };
                let ty = &expression
                    .environment
                    .bindings
                    .iter()
                    .find(|b| b.coordinate == dep.coordinate)
                    .ok_or(Error::Invalid)?
                    .ty;
                sqlx::query("INSERT INTO library_expression_dependencies VALUES(?,?,?,?,?,?,?,?)")
                    .bind(library.bytes())
                    .bind(expression.expression_id.uuid().as_bytes().to_vec())
                    .bind(i64::try_from(ordinal).map_err(|_| Error::Limit)?)
                    .bind(kind)
                    .bind(variable)
                    .bind(slot)
                    .bind(ty.value_type.id.as_str())
                    .bind(i64::from(ty.value_type.version.get()))
                    .execute(&mut *conn)
                    .await?;
            }
        }
    }
    Ok(())
}
pub(super) async fn read_variables(
    conn: &mut SqliteConnection,
    library: LibraryId,
) -> Result<Vec<VariableAggregate>> {
    let rows=sqlx::query("SELECT v.*,c.value_id,c.binding_json FROM library_variables v LEFT JOIN library_variable_values c USING(library_id,variable_id) WHERE v.library_id=? ORDER BY v.variable_id LIMIT 10001").bind(library.bytes()).fetch_all(&mut *conn).await?;
    if rows.len() > 10000 {
        return Err(Error::Limit);
    }
    let mut values = Vec::new();
    for row in rows {
        let id = VariableId::from_uuid(
            Uuid::from_slice(&row.try_get::<Vec<u8>, _>("variable_id")?)
                .map_err(|_| Error::Corrupt)?,
        )
        .map_err(|_| Error::Corrupt)?;
        let names:Vec<String>=sqlx::query_scalar("SELECT name FROM library_variable_names WHERE library_id=? AND variable_id=? ORDER BY name").bind(library.bytes()).bind(id.uuid().as_bytes().to_vec()).fetch_all(&mut *conn).await?;
        let p: VariableProvenance = json(&row.try_get::<String, _>("provenance_json")?)?;
        if p.ty.value_type.id.as_str() != row.try_get::<String, _>("value_type_id")?
            || i64::from(p.ty.value_type.version.get())
                != row.try_get::<i64, _>("value_type_version")?
            || p.ty.schema.id.as_str() != row.try_get::<String, _>("schema_id")?
            || i64::from(p.ty.schema.version.get()) != row.try_get::<i64, _>("schema_version")?
            || timestamp_millis(&p.created_at)? != row.try_get::<i64, _>("created_at")?
            || timestamp_millis(&p.updated_at)? != row.try_get::<i64, _>("updated_at")?
        {
            return Err(Error::Corrupt);
        }
        let core_library = photara_core::contracts::LibraryId::from_uuid(library.uuid())
            .map_err(|_| Error::Corrupt)?;
        let spec = VariableSpec {
            variable_id: id,
            scope: ScopeRef::Library {
                library_id: core_library,
            },
            owning_library_id: core_library,
            definition_version: p.definition_version,
            namespace: photara_core::contracts::schema::QualifiedName::parse(
                row.try_get::<String, _>("namespace")?,
            )
            .map_err(|_| Error::Corrupt)?,
            name: photara_core::contracts::schema::LocalName::parse(
                row.try_get::<String, _>("current_name")?,
            )
            .map_err(|_| Error::Corrupt)?,
            claimed_names: names
                .into_iter()
                .map(|n| {
                    photara_core::contracts::schema::LocalName::parse(n).map_err(|_| Error::Corrupt)
                })
                .collect::<Result<_>>()?,
            label: row.try_get("display_name")?,
            description: row.try_get("description")?,
            ty: p.ty,
            default: row
                .try_get::<Option<String>, _>("default_binding_json")?
                .as_deref()
                .map(json)
                .transpose()?,
            current: row
                .try_get::<Option<String>, _>("binding_json")?
                .as_deref()
                .map(json)
                .transpose()?,
            value_id: row
                .try_get::<Option<Vec<u8>>, _>("value_id")?
                .map(|b| {
                    photara_core::contracts::VariableValueId::from_uuid(
                        Uuid::from_slice(&b).map_err(|_| Error::Corrupt)?,
                    )
                    .map_err(|_| Error::Corrupt)
                })
                .transpose()?,
            revision: LocalRevision::new(row.try_get("local_revision")?)
                .map_err(|_| Error::Corrupt)?,
            owner_revision: p.owner_revision,
            state: parse_enum(row.try_get("state")?)?,
            allow_run_override: match row.try_get::<String, _>("allowed_overrides_json")?.as_str() {
                "[]" => false,
                "[\"run\"]" => true,
                _ => return Err(Error::Corrupt),
            },
            sensitivity: parse_enum(row.try_get("sensitivity")?)?,
            portability: parse_enum(row.try_get("portability")?)?,
            origin: p.origin,
            created_at: p.created_at,
            updated_at: p.updated_at,
        };
        values.push(VariableAggregate::try_from(spec).map_err(|_| Error::Corrupt)?);
    }
    variable_order(&values).map_err(|_| Error::Corrupt)?;
    Ok(values)
}
fn enum_name<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)
        .map_err(|_| Error::Invalid)?
        .as_str()
        .map(str::to_owned)
        .ok_or(Error::Invalid)
}
fn parse_enum<T: serde::de::DeserializeOwned>(value: String) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value)).map_err(|_| Error::Corrupt)
}
/// Exact civil-date conversion; the Core Timestamp has already rejected invalid
/// calendar dates and excess precision. No `SQLite` floating-point time arithmetic.
pub(super) fn timestamp_millis(value: &photara_core::context::value::Timestamp) -> Result<i64> {
    let s = value.as_str();
    let n = |a, z| s[a..z].parse::<i64>().map_err(|_| Error::Invalid);
    let year = n(0, 4)?;
    let month = n(5, 7)?;
    let day = n(8, 10)?;
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Ok(days * 86_400_000
        + n(11, 13)? * 3_600_000
        + n(14, 16)? * 60000
        + n(17, 19)? * 1000
        + n(20, 23)?)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use photara_core::context::{
        snapshot::{Portability, Sensitivity},
        value::{Integer, Shape, TypedValue, Value},
        variable::ValueOrigin,
    };
    use photara_core::contracts::{
        VariableValueId,
        dto::RevisionCoordinate,
        schema::{LocalName, QualifiedName, Version},
    };
    pub(crate) fn variable(library: LibraryId) -> VariableAggregate {
        let core = photara_core::contracts::LibraryId::from_uuid(library.uuid()).unwrap();
        VariableSpec {
            variable_id: VariableId::from_uuid(Uuid::new_v4()).unwrap(),
            scope: ScopeRef::Library { library_id: core },
            owning_library_id: core,
            definition_version: Version::FIRST,
            namespace: QualifiedName::parse("studio.user").unwrap(),
            name: LocalName::parse("count").unwrap(),
            claimed_names: vec![LocalName::parse("count").unwrap()],
            label: "Count".into(),
            description: String::new(),
            ty: Type::builtin(Shape::Integer),
            default: None,
            current: Some(Binding::Literal(Box::new(TypedValue {
                ty: Type::builtin(Shape::Integer),
                value: Value::Integer(Integer::new(7)),
            }))),
            value_id: Some(VariableValueId::from_uuid(Uuid::new_v4()).unwrap()),
            revision: LocalRevision::INITIAL,
            owner_revision: RevisionCoordinate::Local {
                revision: LocalRevision::INITIAL,
            },
            state: VariableState::Active,
            allow_run_override: true,
            sensitivity: Sensitivity::Ordinary,
            portability: Portability::Portable,
            origin: ValueOrigin::Manual,
            created_at: "2026-09-12T00:00:00.000Z".to_owned().try_into().unwrap(),
            updated_at: "2026-09-12T00:00:00.000Z".to_owned().try_into().unwrap(),
        }
        .try_into()
        .unwrap()
    }
    #[tokio::test]
    async fn variable_roundtrip_cas_clear_and_name_retention() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(
            dir.path().join("state.sqlite"),
            Timestamp::try_from(1_789_142_400_000).unwrap(),
        )
        .await
        .unwrap();
        let value = variable(id.library_id);
        store
            .put_library_variable(id.principal_id, &value, None)
            .await
            .unwrap();
        assert_eq!(
            store
                .library_variables(id.principal_id, id.library_id)
                .await
                .unwrap(),
            vec![value.clone()]
        );
        let next = value
            .plan_clear(
                LocalRevision::INITIAL,
                RevisionCoordinate::Local {
                    revision: LocalRevision::new(2).unwrap(),
                },
                value.spec().updated_at.clone(),
            )
            .unwrap();
        store
            .put_library_variable(id.principal_id, &next, Some(LocalRevision::INITIAL))
            .await
            .unwrap();
        assert!(matches!(
            store
                .put_library_variable(id.principal_id, &next, Some(LocalRevision::INITIAL))
                .await,
            Err(Error::Conflict)
        ));
        let actual = store
            .library_variables(id.principal_id, id.library_id)
            .await
            .unwrap();
        assert_eq!(actual, vec![next]);
        assert_eq!(actual[0].spec().value_id, value.spec().value_id);
        let collision = variable(id.library_id);
        assert!(
            store
                .put_library_variable(id.principal_id, &collision, None)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .library_variables(id.principal_id, id.library_id)
                .await
                .unwrap()
                .len(),
            1
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
    #[tokio::test]
    async fn expression_source_and_dependency_children_roundtrip() {
        use photara_core::context::expression::{BindingEnvironment, Expression};
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(
            dir.path().join("state.sqlite"),
            Timestamp::try_from(1_789_142_400_000).unwrap(),
        )
        .await
        .unwrap();
        let mut v = variable(id.library_id).spec().clone();
        let env = BindingEnvironment {
            owner: v.scope,
            owning_library_id: v.owning_library_id,
            owner_revision: v.owner_revision.clone(),
            run_id: None,
            asset_id: None,
            bindings: vec![],
            queries: vec![],
        };
        let expression = Expression::compile(
            photara_core::contracts::ExpressionId::from_uuid(Uuid::new_v4()).unwrap(),
            FieldMode::Expression,
            "`1 + 2`",
            env,
            &v.ty,
        )
        .unwrap();
        v.current = Some(Binding::Expression(Box::new(expression.record().clone())));
        let v = VariableAggregate::try_from(v).unwrap();
        store
            .put_library_variable(id.principal_id, &v, None)
            .await
            .unwrap();
        assert_eq!(
            store
                .library_variables(id.principal_id, id.library_id)
                .await
                .unwrap(),
            vec![v]
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}

/// A persisted private-device observation, not a serialized live context or lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceContextEvidence {
    pub snapshot_id: photara_core::contracts::ContextSnapshotId,
    pub project_id: ProjectId,
    pub run_id: photara_core::contracts::RunId,
    pub canonical_sha256: [u8; 32],
    pub content_digest: photara_core::contracts::schema::Digest,
}
impl LocalLibraryStore {
    /// Retains a bounded device observation with no paths, secrets or live handles.
    /// Captured readiness never grants permission to resolve a resource later.
    /// # Errors
    /// Rejects denied Project access, invalid device facts or a reused snapshot ID.
    pub async fn capture_device_context(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        capture: &DeviceContextCapture,
        at: Timestamp,
    ) -> Result<DeviceContextEvidence> {
        use photara_core::context::snapshot::DeviceContextSnapshot;
        let device = photara_core::contracts::DeviceId::from_uuid(self.info.device_id.uuid())
            .map_err(|_| Error::Invalid)?;
        let checked = DeviceContextSnapshot::build(device, capture.run_id, capture.facts.clone())
            .map_err(|_| Error::Invalid)?;
        let bytes=canonical(&serde_json::json!({"schema":"photara.device-context-observation.v1","device_id":device,"project_id":capture.project_id,"run_id":capture.run_id,"facts":capture.facts}))?.into_bytes();
        let digest = checked.digest();
        let digest_text = digest.to_string();
        let digest_bytes = (0..digest_text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&digest_text[i..i + 2], 16).map_err(|_| Error::Invalid))
            .collect::<Result<Vec<_>>>()?;
        let mut tx = self.write().await?;
        super::local::project_authority(
            &mut tx,
            library,
            capture.project_id,
            actor,
            photara_core::contracts::access::ProjectAction::Read,
        )
        .await?;
        let old:Option<Vec<u8>>=sqlx::query_scalar("SELECT context_canonical FROM device_context_snapshots WHERE snapshot_id=? AND device_id=? AND project_id=? AND run_id=?").bind(capture.snapshot_id.uuid().as_bytes().to_vec()).bind(self.info.device_id.bytes()).bind(capture.project_id.bytes()).bind(capture.run_id.uuid().as_bytes().to_vec()).fetch_optional(&mut *tx).await?;
        if let Some(old) = old {
            if old != bytes {
                return Err(Error::Conflict);
            }
        } else {
            sqlx::query("INSERT INTO device_context_snapshots VALUES(?,?,?,?,?,?,?,?)")
                .bind(capture.snapshot_id.uuid().as_bytes().to_vec())
                .bind(self.info.device_id.bytes())
                .bind(capture.project_id.bytes())
                .bind(capture.run_id.uuid().as_bytes().to_vec())
                .bind(&bytes)
                .bind(sha(&bytes).to_vec())
                .bind(digest_bytes)
                .bind(at.get())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(DeviceContextEvidence {
            snapshot_id: capture.snapshot_id,
            project_id: capture.project_id,
            run_id: capture.run_id,
            canonical_sha256: sha(&bytes),
            content_digest: digest,
        })
    }
}
/// Typed host observations are supplied explicitly; the database does not discover
/// host places or turn captured readiness into a live resource capability.
#[derive(Clone, Debug)]
pub struct DeviceContextCapture {
    pub snapshot_id: photara_core::contracts::ContextSnapshotId,
    pub project_id: ProjectId,
    pub run_id: photara_core::contracts::RunId,
    pub facts: Vec<photara_core::context::snapshot::DeviceFact>,
}
#[cfg(test)]
mod device_tests {
    use super::*;
    #[tokio::test]
    async fn device_observation_is_bound_immutable_and_idempotent() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let at = Timestamp::try_from(1_789_142_400_000).unwrap();
        let (store, id) = LocalLibraryStore::open_app_state(dir.path().join("state.sqlite"), at)
            .await
            .unwrap();
        let p = RegisteredProject {
            library_id: id.library_id,
            project_id: ProjectId::new(),
            association_commit_id: CommitId::new(),
            association_sha256: [8; 32],
            source_origin_library_id: None,
        };
        store
            .register_project(id.principal_id, &p, at)
            .await
            .unwrap();
        let mut capture = DeviceContextCapture {
            snapshot_id: photara_core::contracts::ContextSnapshotId::from_uuid(Uuid::new_v4())
                .unwrap(),
            project_id: p.project_id,
            run_id: photara_core::contracts::RunId::from_uuid(Uuid::new_v4()).unwrap(),
            facts: vec![],
        };
        let first = store
            .capture_device_context(id.principal_id, id.library_id, &capture, at)
            .await
            .unwrap();
        assert_eq!(
            first,
            store
                .capture_device_context(id.principal_id, id.library_id, &capture, at)
                .await
                .unwrap()
        );
        capture.run_id = photara_core::contracts::RunId::from_uuid(Uuid::new_v4()).unwrap();
        assert!(
            store
                .capture_device_context(id.principal_id, id.library_id, &capture, at)
                .await
                .is_err()
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
    }
}
