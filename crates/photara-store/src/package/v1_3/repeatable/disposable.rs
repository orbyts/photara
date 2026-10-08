//! Explicitly controller-owned disposable session admission; no general path registrar.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Private fixed-fixture helpers operate on validated registration and reader proofs"
)]
use super::wire::{encode, hash, reference};
use super::{
    Allocation, AssociationId, Attempt, Corridor, EffectFailure, ExecutionError, Limits,
    Observation, OriginalPackage, PlannerInputs, RepeatableIo, RepeatablePlan, RestartContext,
    RoleAllocation, SelectedStage, SelectorIds, TreeGeometry, VerifiedOriginal, execute, journal,
    layout, plan, recovery, restart, restore_plan, wire,
};
use crate::package::{self, v1_3};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[path = "native_test.rs"]
mod native;
#[path = "disposable_support.rs"]
mod support;
/// One explicitly registered disposable workspace. Stored activation metadata
/// never grants a package lease; every attachment passes the native registrar.
pub struct ControlledWorkspace {
    inner: native::activation_host::Coordinator,
}
impl ControlledWorkspace {
    /// # Errors
    /// Refuses unregistered namespaces, scope mismatch and competing writers.
    pub fn open(
        manifest: &std::path::Path,
        binding: Value,
        target_bindings: Value,
    ) -> std::io::Result<Self> {
        Ok(Self {
            inner: native::activation_host::Coordinator::open(manifest, binding, target_bindings)?,
        })
    }
    #[must_use]
    pub fn state(&self) -> Value {
        self.inner.snapshot()
    }
    pub fn execute(&mut self, request: ControlledRequest) -> Value {
        match serde_json::to_value(request) {Ok(value)=>self.inner.execute(&value),Err(_)=>json!({"error":"invalid request"})}
    }
    pub fn prepare(
        &mut self,
        id: &str,
        target: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> Value {
        self.inner.prepare(id, target, view, epoch, generation)
    }
    pub fn confirm(&mut self, id: &str) -> Value {
        self.inner.confirm(id)
    }
    pub fn cancel(&mut self, id: &str) -> Value {
        self.inner.cancel(id)
    }
    pub fn retry(&mut self, id: &str) -> Value {
        self.inner.retry(id)
    }
    /// One-shot failure cuts, available only when the independent disposable
    /// registrar explicitly enabled them before admission.
    /// # Errors
    /// Refuses unregistered fault injection and unknown cut names.
    pub fn inject_faults(&mut self, points: &[String]) -> std::io::Result<()> {
        self.inner.inject_faults(points)
    }
}
/// Not constructible outside this registrar module. Minted only after profile,
/// controller namespace and pinned image verification.
pub(crate) struct DisposablePermit {
    volume: package::Sha256Hex,
    epoch: package::planning::OwnerEpoch,
}
impl DisposablePermit {
    pub(crate) fn volume(&self) -> package::Sha256Hex {
        self.volume.clone()
    }
    pub(crate) fn epoch(&self) -> package::planning::OwnerEpoch {
        self.epoch
    }
}
fn uid(n: u64) -> String {
    format!("a6000000-0000-4000-8000-{n:012}")
}
fn aid(n: u64) -> String {
    format!("96000000-0000-4000-8000-{:012}", 1000 + n)
}
fn budget() -> Limits {
    Limits {
        semantic: support::limits().semantic,
        max_frames: 65536,
        max_total_allocation_bytes: 64 << 20,
        max_entries: 16384,
        max_objects: 16384,
        max_allocation_growth: 1 << 20,
        max_allocation_bytes: 16 << 20,
    }
}
fn frames() -> v1_3::FrameLimits {
    v1_3::FrameLimits {
        json: support::limits().json,
        max_allocation_bytes: 16 << 20,
        max_frames: 65536,
    }
}
fn package(f: &support::Fixture) -> OriginalPackage {
    let boot = &f.data["bootstrap"];
    OriginalPackage {
        manifest: serde_json::from_str(boot["manifest"].as_str().unwrap()).unwrap(),
        head: serde_json::from_str(boot["head"].as_str().unwrap()).unwrap(),
        commit: serde_json::from_str(boot["commit"].as_str().unwrap()).unwrap(),
        loose: f
            .loose
            .iter()
            .map(|(r, b)| (r.sha256.as_str().to_owned(), b.clone()))
            .collect(),
        allocations: f
            .allocations
            .iter()
            .map(|(id, a)| {
                (
                    id.to_string(),
                    Allocation {
                        bytes: a.bytes.clone(),
                        arena: serde_json::to_value(a.arena)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .into(),
                        witness: json!({"device":a.device.to_string(),"inode":a.inode.to_string()}),
                    },
                )
            })
            .collect(),
    }
}
fn identity(p: &OriginalPackage) -> v1_3::SelectionIdentity {
    v1_3::SelectionIdentity {
        project: package::PackageUuid::parse(p.manifest["project_id"].as_str().unwrap()).unwrap(),
        library: package::PackageUuid::parse(p.commit["root_set"]["library_id"].as_str().unwrap())
            .unwrap(),
        bootstrap_sha256: support::hash(&encode(&p.manifest)),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Existing fixed disposable topology with fallible validation"
)]
fn attempt(old: &VerifiedOriginal, n: u64) -> std::io::Result<Attempt> {
    let invalid = || std::io::Error::from(std::io::ErrorKind::InvalidData);
    if n > 999_999_998_000 || old.proof.roles.len() != 3 || old.package.allocations.len() != 8 {
        return Err(invalid());
    }
    for i in 1..=8 {
        if !old.package.allocations.contains_key(&aid(i)) {
            return Err(invalid());
        }
    }
    let pool = layout::original_objects(
        &old.package,
        &PlannerInputs {
            codec: String::new(),
            active_root_id: String::new(),
            associations: vec![],
            roles: vec![],
            root_placement: String::new(),
            shared_sealed: vec![],
            geometry: TreeGeometry {
                semantic_leaf: 4,
                operation_leaf: 2,
                ownership_leaf: 4,
                physical_leaf: 4,
                branch: 4,
            },
            corridors: vec![],
        },
        budget(),
    )
    .map_err(|_| invalid())?;
    let active = old.proof.roles.get("active").ok_or_else(invalid)?;
    let resource = pool
        .get(&wire::key(&active.state["resource_state"]).map_err(|_| invalid())?)
        .ok_or_else(invalid)?;
    let sources = pool
        .get(&wire::key(&resource["retention_sources"]).map_err(|_| invalid())?)
        .ok_or_else(invalid)?;
    let associations = sources["entries"]
        .as_array()
        .filter(|v| v.len() <= 64)
        .ok_or_else(invalid)?
        .iter()
        .enumerate()
        .map(|(i, e)| {
            Ok(AssociationId {
                original: e["id"].as_str().ok_or_else(invalid)?.into(),
                replacement: uid(n + 100 + i as u64),
            })
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let envelope = old
        .package
        .loose(&old.package.commit["root_set"]["placement"]["accounting"])
        .map_err(|_| invalid())?;
    let standing = old
        .package
        .loose(&envelope["ledger"])
        .map_err(|_| invalid())?["standing_control"]
        .as_str()
        .ok_or_else(invalid)?
        .to_owned();
    let pinned = old
        .proof
        .roles
        .keys()
        .find(|s| s.starts_with("pin:"))
        .ok_or_else(invalid)?;
    let roles = [("active", 2), ("recovery", 4), (pinned.as_str(), 6)]
        .into_iter()
        .map(|(role, a)| RoleAllocation {
            role: role.into(),
            data: aid(a),
            locator: aid(a + 1),
        })
        .collect();
    let corridors = old
        .package
        .allocations
        .iter()
        .filter(|(id, _)| **id != aid(1))
        .map(|(id, a)| Corridor {
            allocation_id: id.clone(),
            arena: a.arena.clone(),
            witness: a.witness.clone(),
            original_end: a.bytes.len().to_string(),
            original_sha256: hash(&a.bytes),
            maximum_end: (a.bytes.len() + 262_144).to_string(),
        })
        .collect();
    Ok(Attempt {
        token: (n + 1).to_string(),
        nonce: hash(&n.to_le_bytes()),
        selectors: (0..7)
            .map(|i| SelectorIds {
                commit_id: uid(n + 200 + i * 2),
                write_id: uid(n + 201 + i * 2),
            })
            .collect(),
        planner: PlannerInputs {
            codec: "photara.codec.ps2-repeatable-layout-v1".into(),
            active_root_id: uid(n),
            associations,
            roles,
            root_placement: aid(8),
            shared_sealed: vec![aid(1)],
            geometry: TreeGeometry {
                semantic_leaf: 4,
                operation_leaf: 2,
                ownership_leaf: 4,
                physical_leaf: 4,
                branch: 4,
            },
            corridors,
        },
        project_limit: "16777216".into(),
        cleanup_bound: "4096".into(),
        charge_unit: "4096".into(),
        aggregate_control_bytes: standing,
        aggregate_control_count: "64".into(),
        phase_bytes: "16384".into(),
        finalization_bytes: "32768".into(),
    })
}

fn request() -> (Value, Value) {
    serde_json::from_str(r#"[{"bootstrap_sha256":"e5cfd9582ed8bd0ee07890d933e4dac52dff15a80fc7345c8494e1b2baac1a5d","boundary":"single","command":{"envelope":{"command":{"kind":"set-node-position","node_id":"62000000-0000-4000-8000-000000020103","x":13,"y":14},"command_id":"50000000-0000-4000-8000-000000000040","expected_revision":2,"graph_id":"62000000-0000-4000-8000-000000020018"},"kind":"graph"},"domain":"photara.package.operation-intent.v1","expected":{"authored_digest":"1f8e9536722aaa1fd85e0baa1ddb874170eb0f67514e81954efbbd45ebe1b039","graphs":{"62000000-0000-4000-8000-000000020018":{"envelope_digest":"a642cd4c56712e57dc68a0171b1fe8d8c84df7da2bd3777f146d7ab0ae01216f","payload_digest":"3431e94ad706b16aa457a234394fb95a81f2674abf32c90bbf5e1d1ede6b07da","revision":"2","semantic_digest":"3431e94ad706b16aa457a234394fb95a81f2674abf32c90bbf5e1d1ede6b07da"}},"revision":"3"},"library_id":"10000000-0000-4000-8000-000000000002","operation_id":"50000000-0000-4000-8000-000000000040","project_id":"10000000-0000-4000-8000-000000000001","undo_group_id":null,"updated_at":"2026-09-27T00:00:00.000Z","version":1},{"acceptance_ordinal":"4","after":{"digest":"57736c507c91b2bb99ae31b12ac93560c1446ccb39ac78a1006a4d1d797b0cf3","revision":"4"},"before":{"digest":"1f8e9536722aaa1fd85e0baa1ddb874170eb0f67514e81954efbbd45ebe1b039","revision":"3"},"bootstrap_sha256":"e5cfd9582ed8bd0ee07890d933e4dac52dff15a80fc7345c8494e1b2baac1a5d","extensions":{},"journal_id":"50000000-0000-4000-8000-000000000001","journal_sequence":"19","library_id":"10000000-0000-4000-8000-000000000002","operation_id":"50000000-0000-4000-8000-000000000040","outcome":"accepted","project_id":"10000000-0000-4000-8000-000000000001","provenance":{"actor":{"actor_id":"50000000-0000-4000-8000-000000000051","kind":"photara.gui"},"effective_scope":{"actions":71,"project_id":"10000000-0000-4000-8000-000000000001"},"grant_ref":{"kind":"photara.project-grant","reference_id":"50000000-0000-4000-8000-000000000052"},"grantor":{"account_id":"50000000-0000-4000-8000-000000000050","kind":"account"},"policy_decision_sha256":"fd922ec9dce398d773fb5cb500ec930113dd333039b244f200e8943b4ee6a85a","principal":{"account_id":"50000000-0000-4000-8000-000000000050","kind":"account"}},"request_sha256":"6b2a5b540e4ca555d6fb9a4bbf657bb0b566ac2b6ada51e1eb1bde641a45186d","schema":{"id":"photara.package.operation-receipt","version":1},"undo_group_id":null}]"#).expect("fixed disposable command template")
}

fn snapshot(base: &support::Fixture, p: &OriginalPackage) -> support::Fixture {
    let mut f = support::Fixture::from_value(base.data.clone())
        .expect("previously validated immutable registration");
    for (id, a) in &p.allocations {
        let dest = f
            .allocations
            .get_mut(&package::PackageUuid::parse(id).unwrap())
            .unwrap();
        dest.bytes.clone_from(&a.bytes);
        dest.charge = (a.bytes.len() as u64).max(1).div_ceil(4096) * 4096;
    }
    f.loose = p
        .loose
        .values()
        .map(|b| {
            (
                serde_json::from_value(reference(&serde_json::from_slice::<Value>(b).unwrap()))
                    .unwrap(),
                b.clone(),
            )
        })
        .collect();
    f.data["bootstrap"] = json!({"manifest":String::from_utf8(encode(&p.manifest)).unwrap(),"head":String::from_utf8(encode(&p.head)).unwrap(),"commit":String::from_utf8(encode(&p.commit)).unwrap()});
    f
}

fn selected(plan: &RepeatablePlan, stage: usize) -> OriginalPackage {
    let mut p = plan.old.clone();
    p.head = plan.stages[stage].head.clone();
    p.commit = plan.stages[stage].commit.clone();
    p.loose = plan.stages[stage].loose.clone();
    for (id, b) in &plan.files {
        p.allocations.get_mut(id).unwrap().bytes.clone_from(b);
    }
    p
}

/// Serialized handle to one explicitly registered private image. Drop releases
/// the lock but never reports a successful flush; `Close` must succeed first.
pub struct ControlledSession {
    inner: native::session_host::OpenSession,
}
impl ControlledSession {
    /// Opens only an existing controller-owned private disposable registration.
    /// # Errors
    /// Refuses unknown/provider-managed storage, malformed registration, changed
    /// image identity, a competing writer, and unsupported session state.
    pub fn open(manifest: &std::path::Path, binding: Value) -> std::io::Result<Self> {
        if manifest.as_os_str().len() > 4096 || encode(&binding).len() > 65536 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            inner: native::open_controlled(manifest, binding)?,
        })
    }
    /// Executes one serialized, typed client request through the shared session.
    pub fn execute(&mut self, request: ControlledRequest) -> Value {
        self.inner
            .execute(&serde_json::to_value(request).unwrap_or(Value::Null))
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlledCommand {
    Snapshot,
    Submit,
    Undo,
    Redo,
    Complete,
    Barrier,
    Close,
    Retry,
    Revalidate,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledRequest {
    pub id: String,
    pub command: ControlledCommand,
    pub node_id: Option<String>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub expected_owner_epoch: Option<String>,
    pub expected_attachment_generation: Option<u64>,
}
#[cfg(test)]
pub(super) fn run_test() {
    native::native_repeatable_phase();
}

#[cfg(test)]
mod tests {
    use super::{ControlledSession, aid, support};
    use serde_json::{Value, json};
    #[test]
    fn controlled_registration_refuses_malformed_input_without_io() {
        let f = support::Fixture::load("integrated");
        assert!(support::Fixture::from_value(f.data.clone()).is_ok());
        for bad in [Value::Null, json!({"bootstrap":{}})] {
            assert!(support::Fixture::from_value(bad).is_err());
        }
        let mut bad = f.data.clone();
        bad["allocations"][aid(1)]["hex"] = json!("zz");
        assert!(support::Fixture::from_value(bad).is_err());
        let mut bad = f.data.clone();
        bad["bootstrap"]["commit"] = json!("{}");
        assert!(support::Fixture::from_value(bad).is_err());
        let mut bad = f.data;
        bad["source_files"] = json!({"../escape":"00"});
        assert!(support::Fixture::from_value(bad).is_err());
    }
    #[test]
    fn controlled_admission_refuses_general_paths_without_creating_files() {
        let root = tempfile::tempdir().unwrap();
        assert!(ControlledSession::open(&root.path().join("manifest.json"), json!({})).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        assert!(
            ControlledSession::open(std::path::Path::new("/tmp/unknown-project"), json!({}))
                .is_err()
        );
    }
}
