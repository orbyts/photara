use super::wire::{Result, ensure, key, number, uuid};
use crate::package::{JsonLimits, PackageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct Allocation {
    pub bytes: Vec<u8>,
    pub arena: String,
    pub witness: Value,
}
/// Supplied bytes are not an authority. The entry-point validator must establish
/// complete selected closure before this input can be compiled or executed.
#[derive(Clone)]
pub struct OriginalPackage {
    pub manifest: Value,
    pub head: Value,
    pub commit: Value,
    pub loose: BTreeMap<String, Vec<u8>>,
    pub allocations: BTreeMap<String, Allocation>,
}
impl OriginalPackage {
    pub(crate) fn loose(&self, r: &Value) -> Result<Value> {
        let k = key(r)?;
        let bytes = self.loose.get(&k.0).ok_or("missing original control")?;
        ensure(
            bytes.len() as u64 == k.1 && super::wire::hash(bytes) == k.0,
            "original control commitment",
        )?;
        super::wire::parse(bytes)
    }
}
#[derive(Clone)]
pub struct Role {
    pub state: Value,
    pub semantic: BTreeSet<(String, u64)>,
    pub receipts: Vec<Value>,
}
#[derive(Clone)]
pub struct OriginalProof {
    pub roles: BTreeMap<String, Role>,
    pub global: BTreeSet<(String, u64)>,
}
#[derive(Clone)]
pub(crate) struct PreparedChange {
    pub authored: BTreeMap<(String, u64), Value>,
    pub receipt: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationId {
    pub original: String,
    pub replacement: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleAllocation {
    pub role: String,
    pub data: String,
    pub locator: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeGeometry {
    pub semantic_leaf: u32,
    pub operation_leaf: u32,
    pub ownership_leaf: u32,
    pub physical_leaf: u32,
    pub branch: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corridor {
    pub allocation_id: String,
    pub arena: String,
    pub witness: Value,
    pub original_end: String,
    pub original_sha256: String,
    pub maximum_end: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerInputs {
    pub codec: String,
    pub active_root_id: String,
    pub associations: Vec<AssociationId>,
    pub roles: Vec<RoleAllocation>,
    pub root_placement: String,
    pub shared_sealed: Vec<String>,
    pub geometry: TreeGeometry,
    pub corridors: Vec<Corridor>,
}
impl PlannerInputs {
    #[expect(
        clippy::too_many_lines,
        reason = "Single fail-closed validation of persisted layout input"
    )]
    pub(crate) fn validate(
        &self,
        old: &OriginalPackage,
        proof: &OriginalProof,
        limits: super::Limits,
    ) -> Result<()> {
        ensure(
            self.codec == "photara.codec.ps2-repeatable-layout-v1",
            "repeatable layout codec",
        )?;
        uuid(&Value::String(self.active_root_id.clone()))?;
        let roots = proof
            .roles
            .values()
            .map(|r| r.state["root_id"].as_str().ok_or("original root UUID"))
            .collect::<Result<BTreeSet<_>>>()?;
        ensure(
            !roots.contains(self.active_root_id.as_str()),
            "new active root collision",
        )?;
        ensure(
            self.geometry.semantic_leaf > 0
                && self.geometry.operation_leaf > 0
                && self.geometry.ownership_leaf > 0
                && self.geometry.physical_leaf > 0
                && self.geometry.branch >= 2,
            "positive deterministic geometry",
        )?;
        for size in [
            self.geometry.semantic_leaf,
            self.geometry.operation_leaf,
            self.geometry.ownership_leaf,
            self.geometry.physical_leaf,
            self.geometry.branch,
        ] {
            ensure(size as usize <= limits.max_entries, "geometry work budget")?;
        }
        ensure(
            self.roles.len() == proof.roles.len() && self.roles.len() <= limits.max_entries,
            "exact role count",
        )?;
        let mut roles = BTreeSet::new();
        let mut writable = BTreeSet::new();
        for role in &self.roles {
            ensure(
                proof.roles.contains_key(&role.role) && roles.insert(&role.role),
                "exact original role map",
            )?;
            for (id, arena) in [(&role.data, "data"), (&role.locator, "metadata")] {
                uuid(&Value::String(id.clone()))?;
                ensure(
                    writable.insert(id.clone())
                        && old.allocations.get(id).is_some_and(|a| a.arena == arena),
                    "distinct registered role allocation",
                )?;
            }
        }
        ensure(
            writable.insert(self.root_placement.clone())
                && old
                    .allocations
                    .get(&self.root_placement)
                    .is_some_and(|a| a.arena == "metadata"),
            "shared root-placement allocation",
        )?;
        let mut sealed = BTreeSet::new();
        for id in &self.shared_sealed {
            uuid(&Value::String(id.clone()))?;
            ensure(
                sealed.insert(id.clone())
                    && !writable.contains(id)
                    && old.allocations.get(id).is_some_and(|a| a.arena == "data"),
                "registered sealed allocation",
            )?;
        }
        let all = writable.union(&sealed).cloned().collect::<BTreeSet<_>>();
        ensure(
            all == old.allocations.keys().cloned().collect(),
            "unsupported unassigned allocation topology",
        )?;
        ensure(
            self.corridors.len() == writable.len(),
            "exact writable corridors",
        )?;
        let mut seen = BTreeSet::new();
        for c in &self.corridors {
            let a = old
                .allocations
                .get(&c.allocation_id)
                .ok_or("corridor allocation")?;
            let start = number(&Value::String(c.original_end.clone()))?;
            let end = number(&Value::String(c.maximum_end.clone()))?;
            ensure(
                writable.contains(&c.allocation_id)
                    && seen.insert(&c.allocation_id)
                    && c.arena == a.arena
                    && c.witness == a.witness
                    && start == a.bytes.len() as u64
                    && c.original_sha256 == super::wire::hash(&a.bytes)
                    && end >= start
                    && end - start <= limits.max_allocation_growth
                    && end <= limits.max_allocation_bytes,
                "original finite corridor",
            )?;
        }
        let mut oldids = BTreeSet::new();
        let mut newids = BTreeSet::new();
        for a in &self.associations {
            uuid(&Value::String(a.original.clone()))?;
            uuid(&Value::String(a.replacement.clone()))?;
            ensure(
                oldids.insert(&a.original)
                    && newids.insert(&a.replacement)
                    && a.original != a.replacement,
                "fresh association map",
            )?;
        }
        Ok(())
    }
}
/// Parsing is bounded before allocation and retains exact canonical input bytes.
/// # Errors
/// Refuses noncanonical JSON, unknown fields, and caller JSON budget violations.
pub fn parse_inputs(
    bytes: &[u8],
    limits: JsonLimits,
) -> std::result::Result<PlannerInputs, PackageError> {
    serde_json::from_value(crate::package::parse_canonical_json(bytes, limits)?)
        .map_err(|_| PackageError::Record)
}
