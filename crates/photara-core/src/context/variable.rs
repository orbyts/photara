//! One-CAS variable aggregates and pure resolution over supplied facts.
use super::expression::{Coordinate, Dependency, Expression, ExpressionRecord};
use super::snapshot::{
    CapturedFact, EffectiveSource, FactValue, Freshness, FrozenContext, Portability, Sensitivity,
    sorted_dependencies,
};
use super::value::{Timestamp, Type, TypedValue};
use super::{EXPANDED_DEPENDENCIES, ErrorCode, Result, SNAPSHOT_LIMIT, digest};
use crate::contracts::{
    dto::{RevisionCoordinate, ScopeRef},
    ids::{LibraryId, RunId, RunOverrideId, VariableId, VariableValueId},
    schema::{Digest, LocalName, LocalRevision, QualifiedName, Version},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Binding {
    Literal(Box<TypedValue>),
    Expression(Box<ExpressionRecord>),
}
impl Binding {
    pub fn validate(&self, ty: &Type, scope: ScopeRef) -> Result<()> {
        match self {
            Self::Literal(v) => {
                v.validate(true)?;
                if &v.ty != ty {
                    return Err(ErrorCode::TypeMismatch.into());
                }
            }
            Self::Expression(r) => {
                let e = Expression::verify(*r.clone())?;
                if e.record().environment.owner != scope || &e.record().ast.ty != ty {
                    return Err(ErrorCode::ScopeMismatch.into());
                }
            }
        }
        Ok(())
    }
    pub fn dependencies(&self) -> Result<Vec<Dependency>> {
        match self {
            Self::Literal(_) => Ok(vec![]),
            Self::Expression(r) => Ok(Expression::verify(*r.clone())?
                .record()
                .dependencies
                .clone()),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VariableState {
    Active,
    Tombstoned,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValueOrigin {
    Manual,
    Command,
    NodeProposal,
    Imported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableSpec {
    pub variable_id: VariableId,
    pub scope: ScopeRef,
    pub owning_library_id: LibraryId,
    pub definition_version: Version,
    pub namespace: QualifiedName,
    pub name: LocalName,
    pub claimed_names: Vec<LocalName>,
    pub label: String,
    pub description: String,
    pub ty: Type,
    pub default: Option<Binding>,
    pub current: Option<Binding>,
    pub value_id: Option<VariableValueId>,
    pub revision: LocalRevision,
    pub owner_revision: RevisionCoordinate,
    pub state: VariableState,
    pub allow_run_override: bool,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
    pub origin: ValueOrigin,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "VariableSpec", into = "VariableSpec")]
pub struct VariableAggregate(VariableSpec);
impl TryFrom<VariableSpec> for VariableAggregate {
    type Error = super::ContextError;
    fn try_from(v: VariableSpec) -> Result<Self> {
        if v.definition_version != Version::FIRST {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        v.ty.validate(true)?;
        if v.namespace.as_str().starts_with("photara.")
            || matches!(
                v.name.as_str(),
                "root" | "artifacts" | "overrides" | "metadata" | "assets"
            )
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        if v.claimed_names.len() > 1024
            || !v.claimed_names.contains(&v.name)
            || v.label.is_empty()
            || v.label.len() > 512
            || v.description.len() > 4096
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        crate::contracts::schema::ordered_unique(&v.claimed_names)?;
        if v.current.is_some() && v.value_id.is_none() {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        if v.updated_at < v.created_at {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        if let ScopeRef::Library { library_id } = v.scope
            && (library_id != v.owning_library_id
                || matches!(v.owner_revision, RevisionCoordinate::Package { .. }))
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        for b in [&v.default, &v.current].into_iter().flatten() {
            b.validate(&v.ty, v.scope)?;
            if let Binding::Expression(e) = b
                && e.environment.owning_library_id != v.owning_library_id
            {
                return Err(ErrorCode::ScopeMismatch.into());
            }
        }
        super::bytes(&v, SNAPSHOT_LIMIT)?;
        Ok(Self(v))
    }
}
impl From<VariableAggregate> for VariableSpec {
    fn from(v: VariableAggregate) -> Self {
        v.0
    }
}
impl VariableAggregate {
    #[must_use]
    pub fn spec(&self) -> &VariableSpec {
        &self.0
    }
    #[must_use]
    pub fn dependency(&self) -> Dependency {
        Dependency {
            coordinate: Coordinate::Variable {
                scope: self.0.scope,
                variable_id: self.0.variable_id,
            },
            projection: vec![],
        }
    }
    pub fn all_dependencies(&self) -> Result<Vec<Dependency>> {
        let mut d = Vec::new();
        for b in [&self.0.default, &self.0.current].into_iter().flatten() {
            d.extend(b.dependencies()?);
        }
        sorted_dependencies(d)
    }
    /// Returns a replacement DTO only; no state is applied or stored.
    pub fn plan_clear(
        &self,
        expected: LocalRevision,
        new_owner_revision: RevisionCoordinate,
        updated_at: Timestamp,
    ) -> Result<Self> {
        if expected != self.0.revision {
            return Err(ErrorCode::Conflict.into());
        }
        if self.0.state != VariableState::Active {
            return Err(ErrorCode::Tombstoned.into());
        }
        let mut next = self.0.clone();
        next.current = None;
        next.revision = next.revision.next()?;
        next.owner_revision = new_owner_revision;
        next.updated_at = updated_at;
        let next = Self::try_from(next)?;
        validate_transition(self, &next, expected, &self.0.owner_revision)?;
        Ok(next)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunOverride {
    pub override_id: RunOverrideId,
    pub run_id: RunId,
    pub target: Dependency,
    pub expected_revision: LocalRevision,
    pub value: TypedValue,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResolvedVariable {
    pub variable_id: VariableId,
    pub fact: CapturedFact,
    pub override_id: Option<RunOverrideId>,
}
/// Validates cycles across both current and default bindings, even when overridden.
pub fn variable_order(variables: &[VariableAggregate]) -> Result<Vec<usize>> {
    if variables.len() > EXPANDED_DEPENDENCIES {
        return Err(ErrorCode::LimitExceeded.into());
    }
    let mut index = BTreeMap::new();
    let mut names = BTreeSet::new();
    for (i, v) in variables.iter().enumerate() {
        if index.insert(v.dependency().key()?, i).is_some() {
            return Err(ErrorCode::Conflict.into());
        }
        for name in &v.0.claimed_names {
            let k = super::bytes(&(v.0.scope, name), super::VALUE_LIMIT)?;
            if !names.insert(k) {
                return Err(ErrorCode::Conflict.into());
            }
        }
    }
    let mut counts = vec![0; variables.len()];
    let mut outgoing: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (i, v) in variables.iter().enumerate() {
        for dep in v.all_dependencies()? {
            let root = Dependency {
                coordinate: dep.coordinate,
                projection: vec![],
            };
            if let Some(&j) = index.get(&root.key()?)
                && outgoing.entry(j).or_default().insert(i)
            {
                counts[i] += 1;
            }
        }
    }
    let mut ready: BTreeSet<_> = counts
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (*c == 0).then_some(i))
        .collect();
    let mut order = Vec::new();
    while let Some(i) = ready.pop_first() {
        order.push(i);
        if let Some(next) = outgoing.get(&i) {
            for &j in next {
                counts[j] -= 1;
                if counts[j] == 0 {
                    ready.insert(j);
                }
            }
        }
    }
    if order.len() != variables.len() {
        return Err(ErrorCode::Cyclic.into());
    }
    Ok(order)
}
/// Values resolve exactly once; caller supplies captured external facts and literal overrides.
pub fn resolve_variables(
    variables: &[VariableAggregate],
    external: &[CapturedFact],
    overrides: &[RunOverride],
    run_id: RunId,
) -> Result<Vec<ResolvedVariable>> {
    let order = variable_order(variables)?;
    if external.len() > EXPANDED_DEPENDENCIES || overrides.len() > EXPANDED_DEPENDENCIES {
        return Err(ErrorCode::LimitExceeded.into());
    }
    let mut facts = BTreeMap::new();
    for f in external {
        f.validate()?;
        if facts.insert(f.dependency.key()?, f.clone()).is_some() {
            return Err(ErrorCode::Conflict.into());
        }
    }
    let vars = variables
        .iter()
        .map(|v| Ok((v.dependency().key()?, v)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut override_map = BTreeMap::new();
    let mut override_ids = BTreeSet::new();
    for o in overrides {
        let v = vars.get(&o.target.key()?).ok_or(ErrorCode::Unknown)?;
        o.value.validate(true)?;
        if o.run_id != run_id || !v.0.allow_run_override {
            return Err(ErrorCode::Forbidden.into());
        }
        if o.expected_revision != v.0.revision {
            return Err(ErrorCode::Conflict.into());
        }
        if o.value.ty != v.0.ty
            || o.sensitivity < v.0.sensitivity
            || o.portability < v.0.portability
        {
            return Err(ErrorCode::Privacy.into());
        }
        if override_map.insert(o.target.key()?, o).is_some() || !override_ids.insert(o.override_id)
        {
            return Err(ErrorCode::Conflict.into());
        }
    }
    let mut results = BTreeMap::new();
    for i in order {
        let variable = &variables[i];
        let key = variable.dependency().key()?;
        let result = resolve_one(variable, &facts, override_map.get(&key).copied())?;
        facts.insert(key.clone(), result.fact.clone());
        results.insert(key, result);
    }
    Ok(results.into_values().collect())
}
fn resolve_one(
    v: &VariableAggregate,
    facts: &BTreeMap<String, CapturedFact>,
    o: Option<&RunOverride>,
) -> Result<ResolvedVariable> {
    let spec = &v.0;
    let key = v.dependency().key()?;
    if facts.contains_key(&key) {
        return Err(ErrorCode::Conflict.into());
    }
    let all_deps = v.all_dependencies()?;

    let binding = spec.current.as_ref().or(spec.default.as_ref());
    let mut sensitivity = spec.sensitivity;
    let mut portability = spec.portability;
    let mut ast_digest = None;
    for dep in &all_deps {
        let f = project_fact(dep, facts)?.ok_or(ErrorCode::Incomplete)?;
        sensitivity = sensitivity.max(f.sensitivity);
        portability = portability.max(f.portability);
    }
    let (value, source) = if spec.state == VariableState::Tombstoned {
        (
            FactValue::Unavailable(ErrorCode::Tombstoned),
            EffectiveSource::Explicit,
        )
    } else if let Some(o) = o {
        sensitivity = sensitivity.max(o.sensitivity);
        portability = portability.max(o.portability);
        (
            FactValue::Present(o.value.value.clone()),
            EffectiveSource::RunOverride,
        )
    } else if let Some(binding) = binding {
        let value = match binding {
            Binding::Literal(v) => v.value.clone(),
            Binding::Expression(r) => {
                let expression = Expression::verify(*r.clone())?;
                ast_digest = Some(expression.record().ast_digest);
                let mut subset = BTreeMap::new();
                for d in &expression.record().dependencies {
                    let f = project_fact(d, facts)?.ok_or(ErrorCode::Unavailable)?;
                    subset.insert(d.key()?, f);
                }
                let context = FrozenContext::from_supplied(&expression, subset, None)?;
                expression.evaluate(&context)?.value
            }
        };
        (
            FactValue::Present(value),
            if spec.current.is_some() {
                EffectiveSource::Explicit
            } else {
                EffectiveSource::Default
            },
        )
    } else {
        (
            FactValue::Unavailable(ErrorCode::Unavailable),
            EffectiveSource::Default,
        )
    };
    let fact = CapturedFact {
        dependency: v.dependency(),
        ty: spec.ty.clone(),
        revision: RevisionCoordinate::Local {
            revision: spec.revision,
        },
        value_id: spec.value_id,
        source,
        run_override: o
            .filter(|_| source == EffectiveSource::RunOverride)
            .map(|o| super::snapshot::OverrideCoordinate {
                run_id: o.run_id,
                override_id: o.override_id,
            }),
        ast_digest,
        value,
        sensitivity,
        portability,
        freshness: Freshness::Captured,
        required: true,
        dependencies: all_deps,
    };
    fact.validate()?;
    Ok(ResolvedVariable {
        variable_id: spec.variable_id,
        fact,
        override_id: o.map(|o| o.override_id),
    })
}
fn project_fact(
    dep: &Dependency,
    facts: &BTreeMap<String, CapturedFact>,
) -> Result<Option<CapturedFact>> {
    if let Some(f) = facts.get(&dep.key()?) {
        return Ok(Some(f.clone()));
    }
    let root = Dependency {
        coordinate: dep.coordinate.clone(),
        projection: vec![],
    };
    let Some(original) = facts.get(&root.key()?) else {
        return Ok(None);
    };
    let mut fact = original.clone();
    for name in &dep.projection {
        let super::value::Shape::Record { fields } = &fact.ty.shape else {
            return Err(ErrorCode::TypeMismatch.into());
        };
        fact.ty = fields.get(name).cloned().ok_or(ErrorCode::Unknown)?;
        if let FactValue::Present(super::value::Value::Record(values)) = &fact.value {
            fact.value = FactValue::Present(values.get(name).cloned().ok_or(ErrorCode::Unknown)?);
        } else if matches!(fact.value, FactValue::Present(_)) {
            return Err(ErrorCode::TypeMismatch.into());
        }
    }
    fact.dependency = dep.clone();
    Ok(Some(fact))
}
/// Stable digest for a complete aggregate's authored definition, default and explicit value.
pub fn variable_digest(variable: &VariableAggregate) -> Result<Digest> {
    digest(variable, SNAPSHOT_LIMIT)
}

/// Validates a proposed aggregate replacement without applying it.
pub fn validate_transition(
    before: &VariableAggregate,
    after: &VariableAggregate,
    expected: LocalRevision,
    owner_base: &RevisionCoordinate,
) -> Result<()> {
    let old = &before.0;
    let new = &after.0;
    if expected != old.revision || owner_base != &old.owner_revision {
        return Err(ErrorCode::Conflict.into());
    }
    if old.state == VariableState::Tombstoned {
        return Err(ErrorCode::Tombstoned.into());
    }
    if old.variable_id != new.variable_id
        || old.scope != new.scope
        || old.owning_library_id != new.owning_library_id
        || old.namespace != new.namespace
        || old.ty != new.ty
        || old.created_at != new.created_at
    {
        return Err(ErrorCode::TypeMismatch.into());
    }
    if new.revision != old.revision.next()?
        || new.updated_at < old.updated_at
        || old
            .claimed_names
            .iter()
            .any(|n| !new.claimed_names.contains(n))
        || old.value_id.is_some() && old.value_id != new.value_id
    {
        return Err(ErrorCode::Conflict.into());
    }
    let advanced = match (&old.owner_revision, &new.owner_revision) {
        (
            RevisionCoordinate::Local { revision: old },
            RevisionCoordinate::Local { revision: new },
        ) => *new == old.next()?,
        (
            RevisionCoordinate::Server { revision: old },
            RevisionCoordinate::Server { revision: new },
        ) => old != new,
        (
            RevisionCoordinate::Package {
                commit_id: old_id,
                commit_sha256: old_hash,
            },
            RevisionCoordinate::Package {
                commit_id: new_id,
                commit_sha256: new_hash,
            },
        ) => old_id != new_id && old_hash != new_hash,
        _ => false,
    };
    if !advanced {
        return Err(ErrorCode::Conflict.into());
    }
    Ok(())
}
