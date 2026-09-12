//! Immutable captures and private device facts supplied by a host, without queries.
use super::expression::{Coordinate, Dependency};
use super::value::{Type, TypedValue, Value};
use super::{EXPANDED_DEPENDENCIES, ErrorCode, Result, SNAPSHOT_LIMIT, bytes, digest};
use crate::contracts::{
    dto::{RevisionCoordinate, ScopeRef},
    ids::{
        ContextSnapshotId, DeviceId, HostBindingId, LibraryId, OperationId, ProjectId, RunId,
        VariableValueId,
    },
    resource::{HostPlace, ResourceDescriptor},
    schema::{Digest, LocalName, Version},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sensitivity {
    Ordinary,
    Personal,
    Restricted,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Portability {
    Portable,
    CaptureConsentRequired,
    HostOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectiveSource {
    Explicit,
    Default,
    RunOverride,
    Captured,
    Input,
    Query,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Freshness {
    Captured,
    LivePreview,
    Stale,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum FactValue {
    Present(Value),
    Absent,
    Unavailable(ErrorCode),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideCoordinate {
    pub run_id: RunId,
    pub override_id: crate::contracts::ids::RunOverrideId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedFact {
    pub dependency: Dependency,
    pub ty: Type,
    pub revision: RevisionCoordinate,
    pub value_id: Option<VariableValueId>,
    pub source: EffectiveSource,
    pub run_override: Option<OverrideCoordinate>,
    pub ast_digest: Option<Digest>,
    pub value: FactValue,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
    pub freshness: Freshness,
    pub required: bool,
    pub dependencies: Vec<Dependency>,
}
impl CapturedFact {
    pub fn validate(&self) -> Result<()> {
        self.ty.validate(matches!(
            self.dependency.coordinate,
            Coordinate::Variable { .. }
        ))?;
        if (self.source == EffectiveSource::RunOverride) != self.run_override.is_some() {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        if self.dependency.projection.len() > 32
            || self.dependencies.len() > super::DIRECT_DEPENDENCIES
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        ordered_dependencies(&self.dependencies)?;
        if let Coordinate::Slot {
            library_id,
            slot_id,
        } = &self.dependency.coordinate
            && let FactValue::Present(value) = &self.value
        {
            let Value::SlotResource(slot) = value else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            if slot.capture.library_id != *library_id
                || slot.capture.slot_id != *slot_id
                || self.revision
                    != (RevisionCoordinate::Local {
                        revision: slot.capture.slot_revision,
                    })
            {
                return Err(ErrorCode::Stale.into());
            }
        }
        match &self.value {
            FactValue::Present(v) => {
                self.ty.check(v)?;
            }
            FactValue::Absent => {
                if !matches!(self.ty.shape, super::value::Shape::Optional { .. }) {
                    return Err(ErrorCode::TypeMismatch.into());
                }
            }
            FactValue::Unavailable(code) => {
                if !matches!(
                    code,
                    ErrorCode::Unknown
                        | ErrorCode::Forbidden
                        | ErrorCode::Revoked
                        | ErrorCode::Tombstoned
                        | ErrorCode::Unavailable
                        | ErrorCode::Stale
                        | ErrorCode::StaleFingerprint
                        | ErrorCode::Ambiguous
                        | ErrorCode::Conflict
                        | ErrorCode::TypeMismatch
                        | ErrorCode::UnsupportedVersion
                        | ErrorCode::LimitExceeded
                ) {
                    return Err(ErrorCode::InvalidCoordinate.into());
                }
            }
        }
        Ok(())
    }
    pub fn resolved(&self) -> Result<TypedValue> {
        if self.freshness == Freshness::Stale {
            return Err(ErrorCode::Stale.into());
        }
        let value = match &self.value {
            FactValue::Present(v) => v.clone(),
            FactValue::Absent => Value::Optional(None),
            FactValue::Unavailable(c) => return Err((*c).into()),
        };
        Ok(TypedValue {
            ty: self.ty.clone(),
            value,
        })
    }
}
pub fn ordered_dependencies(deps: &[Dependency]) -> Result<()> {
    let keys = deps
        .iter()
        .map(Dependency::key)
        .collect::<Result<Vec<_>>>()?;
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(ErrorCode::Conflict.into());
    }
    Ok(())
}
pub fn sorted_dependencies(deps: impl IntoIterator<Item = Dependency>) -> Result<Vec<Dependency>> {
    let mut out = BTreeMap::new();
    for d in deps {
        out.insert(d.key()?, d);
    }
    Ok(out.into_values().collect())
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Versions {
    pub context: Version,
    pub expression: Version,
    pub interpreter: Version,
    pub query: Version,
}
impl Default for Versions {
    fn default() -> Self {
        Self {
            context: Version::FIRST,
            expression: Version::FIRST,
            interpreter: Version::FIRST,
            query: Version::FIRST,
        }
    }
}
impl Versions {
    pub fn validate(&self) -> Result<()> {
        if self == &Self::default() {
            Ok(())
        } else {
            Err(ErrorCode::UnsupportedVersion.into())
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audience {
    pub project_id: ProjectId,
    pub policy_digest: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureConsent {
    pub audience: Audience,
    pub projection_digest: Digest,
    pub sensitivity_ceiling: Sensitivity,
    pub granted: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureDestination {
    LocalProject,
    SharedProject,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Completeness {
    Complete,
    Incomplete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Replayability {
    Portable,
    DeviceDependent,
    Blocked,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotSpec {
    pub snapshot_id: ContextSnapshotId,
    pub owning_library_id: LibraryId,
    pub project_id: ProjectId,
    pub captured_at: super::value::Timestamp,
    pub versions: Versions,
    pub audience: Audience,
    pub destination: CaptureDestination,
    pub consent: Option<CaptureConsent>,
    pub roots: Vec<Dependency>,
    pub entries: Vec<CapturedFact>,
    pub device_required: bool,
    pub secret_requirements: Vec<LocalName>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContextSnapshot {
    spec: SnapshotSpec,
    content_digest: Digest,
    completeness: Completeness,
    replayability: Replayability,
}
impl ContextSnapshot {
    pub fn build(spec: SnapshotSpec) -> Result<Self> {
        spec.versions.validate()?;
        if spec.audience.project_id != spec.project_id {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        if spec.entries.len() > EXPANDED_DEPENDENCIES
            || spec.roots.len() > EXPANDED_DEPENDENCIES
            || spec.secret_requirements.len() > 256
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        ordered_dependencies(&spec.roots)?;
        crate::contracts::schema::ordered_unique(&spec.secret_requirements)?;
        let (sensitivity, needs_consent, incomplete) = validate_entries(&spec)?;
        let projection = projection_digest(&spec.entries)?;
        if needs_consent {
            let c = spec.consent.as_ref().ok_or(ErrorCode::ConsentRequired)?;
            if !c.granted
                || c.audience != spec.audience
                || c.projection_digest != projection
                || c.sensitivity_ceiling < sensitivity
            {
                return Err(ErrorCode::ConsentRequired.into());
            }
        }
        if spec.destination == CaptureDestination::SharedProject
            && sensitivity == Sensitivity::Restricted
        {
            return Err(ErrorCode::Privacy.into());
        }
        let content_digest = digest(
            &serde_json::json!({"domain":"photara.context-content.v1","library_id":spec.owning_library_id,"project_id":spec.project_id,"versions":spec.versions,"roots":spec.roots,"entries":spec.entries,"device_required":spec.device_required,"secret_requirements":spec.secret_requirements}),
            SNAPSHOT_LIMIT,
        )?;
        let completeness = if incomplete {
            Completeness::Incomplete
        } else {
            Completeness::Complete
        };
        let replayability = if incomplete || !spec.secret_requirements.is_empty() {
            Replayability::Blocked
        } else if spec.device_required {
            Replayability::DeviceDependent
        } else {
            Replayability::Portable
        };
        let value = Self {
            spec,
            content_digest,
            completeness,
            replayability,
        };
        bytes(&value, SNAPSHOT_LIMIT)?;
        Ok(value)
    }
    #[must_use]
    pub fn spec(&self) -> &SnapshotSpec {
        &self.spec
    }
    #[must_use]
    pub const fn content_digest(&self) -> Digest {
        self.content_digest
    }
    #[must_use]
    pub const fn completeness(&self) -> Completeness {
        self.completeness
    }
    #[must_use]
    pub const fn replayability(&self) -> Replayability {
        self.replayability
    }
    pub fn from_json(input: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            spec: SnapshotSpec,
            content_digest: Digest,
            completeness: Completeness,
            replayability: Replayability,
        }
        let w: Wire = crate::contracts::schema::decode_strict(input, SNAPSHOT_LIMIT)?;
        let snapshot = Self::build(w.spec)?;
        if snapshot.content_digest != w.content_digest
            || snapshot.completeness != w.completeness
            || snapshot.replayability != w.replayability
        {
            return Err(ErrorCode::SourceAstMismatch.into());
        }
        Ok(snapshot)
    }
    pub fn subset(&self, roots: &[Dependency]) -> Result<Vec<CapturedFact>> {
        ordered_dependencies(roots)?;
        let map = self
            .spec
            .entries
            .iter()
            .map(|f| Ok((f.dependency.key()?, f)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        closure(roots, &map)?
            .iter()
            .map(|k| {
                map.get(k)
                    .map(|f| (*f).clone())
                    .ok_or_else(|| ErrorCode::Incomplete.into())
            })
            .collect()
    }
    pub fn subset_digest(&self, roots: &[Dependency]) -> Result<Digest> {
        digest(
            &serde_json::json!({"domain":"photara.context-subset.v1","versions":self.spec.versions,"entries":self.subset(roots)?}),
            SNAPSHOT_LIMIT,
        )
    }
    pub fn validate_audience(&self, current: &Audience) -> Result<()> {
        if &self.spec.audience != current {
            return Err(ErrorCode::ConsentRequired.into());
        }
        Ok(())
    }
}
fn validate_entries(spec: &SnapshotSpec) -> Result<(Sensitivity, bool, bool)> {
    let mut keys = BTreeMap::new();
    let mut previous = None;
    let mut sensitivity = Sensitivity::Ordinary;
    let mut needs_consent = false;
    let mut incomplete = false;
    let mut override_run = None;
    for fact in &spec.entries {
        fact.validate()?;
        if let Some(source) = fact.run_override {
            if override_run.is_some_and(|run| run != source.run_id) {
                return Err(ErrorCode::ScopeMismatch.into());
            }
            override_run = Some(source.run_id);
        }
        let key = fact.dependency.key()?;
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(ErrorCode::Conflict.into());
        }
        previous = Some(key.clone());
        keys.insert(key, fact);
        if matches!(fact.dependency.coordinate, Coordinate::HostPlace { .. })
            || fact.portability == Portability::HostOnly
            || contains_host_value(&fact.value)
        {
            return Err(ErrorCode::Privacy.into());
        }
        validate_capture_scope(&fact.dependency, spec.project_id, spec.owning_library_id)?;
        if let FactValue::Present(value) = &fact.value {
            validate_value_project(value, spec.project_id)?;
        }
        validate_metadata_coordinate(fact)?;
        if fact.freshness == Freshness::LivePreview {
            return Err(ErrorCode::Stale.into());
        }
        sensitivity = sensitivity.max(fact.sensitivity);
        needs_consent |= fact.portability == Portability::CaptureConsentRequired
            || fact.sensitivity == Sensitivity::Restricted;
        incomplete |= fact.freshness == Freshness::Stale
            || matches!(fact.value, FactValue::Unavailable(_))
            || (fact.required && matches!(fact.value, FactValue::Absent));
    }
    let closure = closure(&spec.roots, &keys)?;
    if closure.len() != keys.len() {
        return Err(ErrorCode::Incomplete.into());
    }
    for fact in &spec.entries {
        if let (
            Coordinate::MetadataQuery {
                project_id,
                graph_id,
                node_id,
                port_id,
                ..
            },
            FactValue::Present(Value::Metadata(metadata)),
        ) = (&fact.dependency.coordinate, &fact.value)
        {
            let input = Dependency {
                coordinate: Coordinate::Input {
                    project_id: *project_id,
                    graph_id: *graph_id,
                    node_id: *node_id,
                    port_id: port_id.clone(),
                },
                projection: vec![],
            };
            let fact = keys.get(&input.key()?).ok_or(ErrorCode::Incomplete)?;
            if fact.resolved()?.value != Value::AssetSet(metadata.spec().input.descriptor()?) {
                return Err(ErrorCode::StaleFingerprint.into());
            }
        }
        for dep in &fact.dependencies {
            let d = keys.get(&dep.key()?).ok_or(ErrorCode::Incomplete)?;
            if fact.sensitivity < d.sensitivity || fact.portability < d.portability {
                return Err(ErrorCode::Privacy.into());
            }
        }
    }
    Ok((sensitivity, needs_consent, incomplete))
}
pub fn projection_digest(facts: &[CapturedFact]) -> Result<Digest> {
    digest(
        &serde_json::json!({"domain":"photara.context-projection.v1","entries":facts}),
        SNAPSHOT_LIMIT,
    )
}
fn validate_capture_scope(dep: &Dependency, p: ProjectId, l: LibraryId) -> Result<()> {
    let valid = match dep.coordinate {
        Coordinate::Variable { scope, .. } => match scope {
            ScopeRef::Library { library_id } => library_id == l,
            _ => super::expression::project(scope) == Some(p),
        },
        Coordinate::Slot { library_id, .. } => library_id == l,
        Coordinate::MetadataQuery { project_id, .. }
        | Coordinate::Input { project_id, .. }
        | Coordinate::AssetMetadata { project_id, .. }
        | Coordinate::ProjectRoot { project_id }
        | Coordinate::ProjectArtifacts { project_id }
        | Coordinate::RunOverrides { project_id, .. } => project_id == p,
        Coordinate::HostPlace { .. } => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ErrorCode::ScopeMismatch.into())
    }
}
fn contains_host_value(f: &FactValue) -> bool {
    fn host(v: &Value) -> bool {
        match v {
            Value::Resource(r) => matches!(r.descriptor, ResourceDescriptor::HostPlace { .. }),
            Value::List(v) => v.iter().any(host),
            Value::Record(v) => v.values().any(host),
            Value::Optional(Some(v)) => host(v),
            Value::Metadata(m) => m.spec().members.iter().any(
                |row| matches!(&row.outcome, super::metadata::Observation::Value(v) if host(v)),
            ),
            _ => false,
        }
    }
    matches!(f,FactValue::Present(v)if host(v))
}
/// Iterative topological closure includes every default/fallback/branch dependency.
pub fn closure(
    roots: &[Dependency],
    facts: &BTreeMap<String, &CapturedFact>,
) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    let mut pending = roots
        .iter()
        .map(Dependency::key)
        .collect::<Result<Vec<_>>>()?;
    while let Some(key) = pending.pop() {
        if seen.insert(key.clone()) {
            if seen.len() > EXPANDED_DEPENDENCIES {
                return Err(ErrorCode::LimitExceeded.into());
            }
            let f = facts.get(&key).ok_or(ErrorCode::Incomplete)?;
            pending.extend(
                f.dependencies
                    .iter()
                    .map(Dependency::key)
                    .collect::<Result<Vec<_>>>()?,
            );
        }
    }
    let mut remaining = seen.clone();
    let mut done = BTreeSet::new();
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .filter(|key| {
                facts.get(*key).is_some_and(|f| {
                    f.dependencies
                        .iter()
                        .all(|d| d.key().is_ok_and(|k| done.contains(&k)))
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(ErrorCode::Cyclic.into());
        }
        for k in ready {
            remaining.remove(&k);
            done.insert(k);
        }
    }
    Ok(seen)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceAvailability {
    Ready,
    NotBound,
    Unavailable,
    Denied,
    Stale,
    Ambiguous,
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeviceFact {
    pub place: HostPlace,
    pub binding_id: HostBindingId,
    pub generation: u64,
    pub availability: DeviceAvailability,
    pub temp_lease: Option<OperationId>,
}
/// Device-only capture has no serialization API; its digest excludes paths and grants.
#[derive(Clone, Eq, PartialEq)]
pub struct DeviceContextSnapshot {
    device_id: DeviceId,
    run_id: RunId,
    facts: Vec<DeviceFact>,
    digest: Digest,
}
impl std::fmt::Debug for DeviceContextSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceContextSnapshot(<private>)")
    }
}
impl DeviceContextSnapshot {
    pub fn build(device_id: DeviceId, run_id: RunId, facts: Vec<DeviceFact>) -> Result<Self> {
        if facts.len() > 6 || facts.windows(2).any(|w| w[0].place >= w[1].place) {
            return Err(ErrorCode::Conflict.into());
        }
        for f in &facts {
            if f.generation == 0
                || f.generation > i64::MAX as u64
                || (f.place == HostPlace::Temp) != f.temp_lease.is_some()
            {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
        }
        let digest = digest(
            &serde_json::json!({"domain":"photara.device-context.v1","device_id":device_id,"facts":facts,"temp_run_id":facts.iter().any(|f|f.place==HostPlace::Temp).then_some(run_id)}),
            SNAPSHOT_LIMIT,
        )?;
        Ok(Self {
            device_id,
            run_id,
            facts,
            digest,
        })
    }
    #[must_use]
    pub const fn digest(&self) -> Digest {
        self.digest
    }
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }
    pub(crate) fn resolve(&self, place: HostPlace) -> Result<Value> {
        let f = self
            .facts
            .iter()
            .find(|f| f.place == place)
            .ok_or(ErrorCode::Unavailable)?;
        if f.availability != DeviceAvailability::Ready {
            return Err(match f.availability {
                DeviceAvailability::Denied => ErrorCode::Revoked,
                DeviceAvailability::Stale => ErrorCode::Stale,
                DeviceAvailability::Ambiguous => ErrorCode::Ambiguous,
                DeviceAvailability::Unsupported => ErrorCode::UnsupportedVersion,
                _ => ErrorCode::Unavailable,
            }
            .into());
        }
        Ok(Value::Resource(Box::new(super::value::ResourceValue {
            descriptor: ResourceDescriptor::HostPlace { place },
            components: vec![],
        })))
    }
}
/// Frozen permitted subset. No serde or mutation API; values cannot refresh during evaluation.
#[derive(Clone)]
pub struct FrozenContext {
    owner: ScopeRef,
    library_id: LibraryId,
    run_id: Option<RunId>,
    expression_digest: Digest,
    pub(crate) facts: BTreeMap<String, CapturedFact>,
    pub(crate) device: Option<DeviceContextSnapshot>,
}
impl std::fmt::Debug for FrozenContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FrozenContext(<redacted>)")
    }
}
impl FrozenContext {
    pub fn for_expression(
        snapshot: &ContextSnapshot,
        expression: &super::expression::Expression,
        device: Option<DeviceContextSnapshot>,
    ) -> Result<Self> {
        let e = expression.record();
        if super::expression::project(e.environment.owner) != Some(snapshot.spec.project_id)
            || e.environment.owning_library_id != snapshot.spec.owning_library_id
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        let roots = e
            .dependencies
            .iter()
            .filter(|d| !matches!(d.coordinate, Coordinate::HostPlace { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let facts = snapshot.subset(&roots)?;
        if let Some(d) = &device
            && e.environment.run_id != Some(d.run_id())
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        let context = Self::from_supplied(
            expression,
            facts
                .into_iter()
                .map(|f| Ok((f.dependency.key()?, f)))
                .collect::<Result<_>>()?,
            device,
        )?;
        for d in &e.dependencies {
            context.get(d)?;
        }
        Ok(context)
    }
    pub(crate) fn from_supplied(
        expression: &super::expression::Expression,
        facts: BTreeMap<String, CapturedFact>,
        device: Option<DeviceContextSnapshot>,
    ) -> Result<Self> {
        let e = expression.record();
        if device
            .as_ref()
            .is_some_and(|d| Some(d.run_id()) != e.environment.run_id)
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        for (key, fact) in &facts {
            fact.validate()?;
            if fact
                .run_override
                .is_some_and(|source| Some(source.run_id) != e.environment.run_id)
            {
                return Err(ErrorCode::ScopeMismatch.into());
            }
            if key != &fact.dependency.key()? {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
        }
        Ok(Self {
            owner: e.environment.owner,
            library_id: e.environment.owning_library_id,
            run_id: e.environment.run_id,
            expression_digest: e.ast_digest,
            facts,
            device,
        })
    }
    pub(crate) fn validate_for(&self, expression: &super::expression::Expression) -> Result<()> {
        let e = expression.record();
        if self.owner != e.environment.owner
            || self.library_id != e.environment.owning_library_id
            || self.run_id != e.environment.run_id
            || self.expression_digest != e.ast_digest
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        Ok(())
    }
    pub fn dependency_digest(&self) -> Result<Digest> {
        digest(
            &serde_json::json!({"domain":"photara.frozen-evaluation.v1","facts":self.facts.values().collect::<Vec<_>>(),"device":self.device.as_ref().map(DeviceContextSnapshot::digest)}),
            SNAPSHOT_LIMIT,
        )
    }
    pub(crate) fn get(&self, dep: &Dependency) -> Result<TypedValue> {
        if let Coordinate::HostPlace { place } = dep.coordinate {
            let d = self.device.as_ref().ok_or(ErrorCode::Unavailable)?;
            return Ok(TypedValue {
                ty: Type::builtin(super::value::Shape::Resource),
                value: d.resolve(place)?,
            });
        }
        self.facts
            .get(&dep.key()?)
            .ok_or(ErrorCode::Incomplete)?
            .resolved()
    }
    #[must_use]
    pub fn sensitivity(&self) -> Sensitivity {
        self.facts
            .values()
            .map(|f| f.sensitivity)
            .max()
            .unwrap_or(Sensitivity::Ordinary)
    }
    #[must_use]
    pub fn portability(&self) -> Portability {
        if self.device.is_some() {
            Portability::HostOnly
        } else {
            self.facts
                .values()
                .map(|f| f.portability)
                .max()
                .unwrap_or(Portability::Portable)
        }
    }
}

fn validate_value_project(value: &Value, project_id: ProjectId) -> Result<()> {
    match value {
        Value::AssetSet(s) if s.project_id != project_id => {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        Value::Metadata(m) if m.spec().input.project_id() != project_id => {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        Value::Resource(r) if r.descriptor.project_id().is_some_and(|p| p != project_id) => {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        Value::Reference(super::value::TypedReference::Asset { project_id: p, .. })
            if *p != project_id =>
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        Value::List(values) => {
            for v in values {
                validate_value_project(v, project_id)?;
            }
        }
        Value::Record(values) => {
            for v in values.values() {
                validate_value_project(v, project_id)?;
            }
        }
        Value::Optional(Some(v)) => validate_value_project(v, project_id)?,
        _ => {}
    }
    Ok(())
}

fn validate_metadata_coordinate(fact: &CapturedFact) -> Result<()> {
    if let Coordinate::MetadataQuery {
        project_id,
        graph_id,
        node_id,
        port_id,
        field,
        selector,
    } = &fact.dependency.coordinate
    {
        let dependency = Dependency {
            coordinate: Coordinate::Input {
                project_id: *project_id,
                graph_id: *graph_id,
                node_id: *node_id,
                port_id: port_id.clone(),
            },
            projection: vec![],
        };
        if !fact.dependencies.contains(&dependency) {
            return Err(ErrorCode::Incomplete.into());
        }
        if let FactValue::Present(Value::Metadata(m)) = &fact.value {
            if &m.spec().field != field || &m.spec().selector != selector {
                return Err(ErrorCode::ScopeMismatch.into());
            }
        } else {
            return Err(ErrorCode::TypeMismatch.into());
        }
    }
    Ok(())
}
