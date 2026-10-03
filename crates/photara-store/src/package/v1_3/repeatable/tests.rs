//! Existing integrated corpus, exercised through the shared repeatable path.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::wire::{encode, hash, reference};
use super::*;
use crate as photara_store;
#[path = "../../../../tests/ps2_reader_support/mod.rs"]
mod support;
use crate::package::{self, v1_3};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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
fn verify(f: &support::Fixture) -> VerifiedOriginal {
    let p = package(f);
    v1_3::verify_original(
        p.clone(),
        identity(&p),
        f,
        &f.registration,
        v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        frames(),
        &support::limits(),
    )
    .unwrap()
}
#[expect(
    clippy::too_many_lines,
    reason = "Explicit fixture planner input schedule"
)]
fn attempt(old: &VerifiedOriginal, n: u64) -> Attempt {
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
    .unwrap();
    let resource = &pool[&wire::key(&old.proof.roles["active"].state["resource_state"]).unwrap()];
    let sources = &pool[&wire::key(&resource["retention_sources"]).unwrap()];
    let associations = sources["entries"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, e)| AssociationId {
            original: e["id"].as_str().unwrap().into(),
            replacement: uid(n + 100 + i as u64),
        })
        .collect();
    let envelope = old
        .package
        .loose(&old.package.commit["root_set"]["placement"]["accounting"])
        .unwrap();
    let standing = old.package.loose(&envelope["ledger"]).unwrap()["standing_control"]
        .as_str()
        .unwrap()
        .to_owned();
    let pinned = old
        .proof
        .roles
        .keys()
        .find(|s| s.starts_with("pin:"))
        .unwrap();
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
    Attempt {
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
    }
}
/// Explicit test-only initial registration: the original 131072-byte standing
/// control fixture correctly refuses op2's 151552-byte adjacent control peak.
/// Charge another 131072 before verifying the original; no real library change.
fn fixture() -> support::Fixture {
    let base = support::Fixture::load("integrated");
    let mut p = package(&base);
    let old_envelope = p
        .loose(&p.commit["root_set"]["placement"]["accounting"])
        .unwrap();
    let mut ledger = p.loose(&old_envelope["ledger"]).unwrap();
    let old_ledger = reference(&ledger);
    let old_env = reference(&old_envelope);
    ledger["standing_control"] = json!("262144");
    ledger["total_charge"] =
        json!((wire::number(&ledger["total_charge"]).unwrap() + 131_072).to_string());
    let mut envelope = old_envelope;
    envelope["ledger"] = reference(&ledger);
    let mut overlay = p.loose(&p.commit["root_set"]["inventory"]).unwrap();
    for r in overlay["controls"].as_array_mut().unwrap() {
        if *r == old_ledger {
            *r = reference(&ledger);
        } else if *r == old_env {
            *r = reference(&envelope);
        }
    }
    overlay["controls"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|r| wire::key(r).unwrap());
    p.commit["root_set"]["placement"]["accounting"] = reference(&envelope);
    p.commit["root_set"]["inventory"] = reference(&overlay);
    p.head["commit_sha256"] = json!(hash(&encode(&p.commit)));
    let mut data = base.data;
    data["loose"] = json!({"ledger":String::from_utf8(encode(&ledger)).unwrap(),"envelope":String::from_utf8(encode(&envelope)).unwrap(),"overlay":String::from_utf8(encode(&overlay)).unwrap()});
    data["bootstrap"]["head"] = json!(String::from_utf8(encode(&p.head)).unwrap());
    data["bootstrap"]["commit"] = json!(String::from_utf8(encode(&p.commit)).unwrap());
    support::Fixture::from_value(data)
}
fn request() -> (Value, Value) {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../../../docs/architecture/proposals/ps2/route/operation-four.json"
    ))
    .unwrap();
    (
        v["records"]["intent-4"]["input"].clone(),
        v["records"]["receipt-4"]["input"].clone(),
    )
}
#[test]
fn repeatable_original_compiles_deterministically() {
    let fixture = fixture();
    let original = verify(&fixture);
    let (intent, receipt) = request();
    let inputs = attempt(&original, 1000);
    let plan = plan::compile_inner(&original, &intent, &receipt, inputs.clone(), budget()).unwrap();
    let repeated = plan::compile_inner(&original, &intent, &receipt, inputs, budget()).unwrap();
    assert_eq!(plan.original_bytes(), repeated.original_bytes());
    assert_eq!(plan.target_allocations(), repeated.target_allocations());
    assert_eq!(plan.stages().len(), 7);
    for (a, b) in plan.stages().iter().zip(repeated.stages()) {
        assert_eq!(a.head, b.head);
        assert_eq!(a.commit, b.commit);
        assert_eq!(a.loose, b.loose);
    }
    assert_eq!(
        plan.original()["original_codec"],
        "photara.codec.ps2-repeatable-admission-v1"
    );
    assert_eq!(reference(&intent), plan.original()["request"]["intent"]);
}
fn snapshot(base: &support::Fixture, p: &OriginalPackage) -> support::Fixture {
    let mut f = support::Fixture::from_value(base.data.clone());
    for (id, a) in &p.allocations {
        let dest = f
            .allocations
            .get_mut(&package::PackageUuid::parse(id).unwrap())
            .unwrap();
        dest.bytes = a.bytes.clone();
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
fn vector(plan: &RepeatablePlan) -> String {
    let stages=plan.stages.iter().map(|s|json!({"head":hash(&encode(&s.head)),"commit":hash(&encode(&s.commit)),"controls":s.loose.iter().map(|(sha,b)|(sha.clone(),b.len().to_string())).collect::<BTreeMap<_,_>>()})).collect::<Vec<_>>();
    hash(&encode(
        &json!({"original_sha256":hash(&plan.original_bytes()),"stages":stages,"allocations":plan.files.iter().map(|(id,b)|(id.clone(),hash(b))).collect::<BTreeMap<_,_>>()}),
    ))
}
fn selected(plan: &RepeatablePlan, stage: usize) -> OriginalPackage {
    let mut p = plan.old.clone();
    p.head = plan.stages[stage].head.clone();
    p.commit = plan.stages[stage].commit.clone();
    p.loose = plan.stages[stage].loose.clone();
    for (id, b) in &plan.files {
        p.allocations.get_mut(id).unwrap().bytes = b.clone();
    }
    p
}
struct Mock {
    base: support::Fixture,
    current: OriginalPackage,
    receipts: BTreeMap<String, Value>,
    journals: BTreeMap<String, Value>,
    interrupt_append: bool,
    interrupt_select: Option<(String, bool)>,
    fail_stabilize: bool,
    effects: usize,
}
type Lease = crate::package::planning::io::RegisteredCooperativeLease<()>;
impl Mock {
    fn check(
        &self,
        p: &OriginalPackage,
        plan: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), ExecutionError> {
        let observed = snapshot(&self.base, p);
        v1_3::reader::inspect_operation_package(
            p,
            identity(p),
            &observed,
            &self.base.registration,
            frames(),
            &support::limits(),
            plan,
            stage,
        )
        .map(|_| ())
        .map_err(|_| ExecutionError::InvalidObservation)
    }
}
impl RepeatableIo for Mock {
    type Lease = ();
    fn authorize(
        &mut self,
        _: &Lease,
        original: &Value,
        receipt: &Value,
    ) -> Result<(), ExecutionError> {
        // Explicit synthetic test principal; never a production authorization path.
        let (_, expected) = request();
        if receipt["provenance"] != expected["provenance"]
            || original["request"]["receipt"] != reference(receipt)
        {
            return Err(ExecutionError::Authorization);
        }
        Ok(())
    }
    fn observe(&mut self, _: &Lease) -> Result<Observation, ExecutionError> {
        Ok(Observation {
            head: self.current.head.clone(),
            allocations: self
                .current
                .allocations
                .iter()
                .map(|(id, a)| (id.clone(), a.bytes.clone()))
                .collect(),
        })
    }
    fn verify_progress(
        &mut self,
        _: &Lease,
        p: &RepeatablePlan,
        stage: Option<usize>,
    ) -> Result<(), ExecutionError> {
        let (commit, loose) = stage.map_or((&p.old.commit, &p.old.loose), |n| {
            (&p.stages[n].commit, &p.stages[n].loose)
        });
        if &self.current.commit != commit || &self.current.loose != loose {
            return Err(ExecutionError::InvalidObservation);
        }
        if stage.is_some_and(|n| n >= 1)
            && self
                .journals
                .get(p.prepared.receipt["operation_id"].as_str().unwrap())
                != Some(&plan::journal_frame(&p.prepared.receipt))
        {
            return Err(ExecutionError::InvalidObservation);
        }
        if stage == Some(6)
            && self
                .receipts
                .get(p.prepared.receipt["operation_id"].as_str().unwrap())
                != Some(&p.prepared.receipt)
        {
            return Err(ExecutionError::InvalidObservation);
        }
        Ok(())
    }
    fn stabilize_progress(
        &mut self,
        _: &Lease,
        _: &RepeatablePlan,
        _: usize,
    ) -> Result<(), EffectFailure> {
        if self.fail_stabilize {
            Err(EffectFailure::OutcomeUnknown)
        } else {
            Ok(())
        }
    }
    fn verify_prospective(&mut self, _: &Lease, p: &RepeatablePlan) -> Result<(), ExecutionError> {
        self.check(&selected(p, 6), p, 6)
    }
    fn select(
        &mut self,
        _: &Lease,
        expected: &Value,
        stage: &SelectedStage,
    ) -> Result<(), EffectFailure> {
        if &self.current.head != expected {
            return Err(EffectFailure::NotPerformed);
        }
        let cut = self
            .interrupt_select
            .as_ref()
            .is_some_and(|(id, _)| stage.commit["commit_id"] == *id);
        let after = cut && self.interrupt_select.as_ref().unwrap().1;
        if cut {
            self.interrupt_select = None;
            if !after {
                return Err(EffectFailure::OutcomeUnknown);
            }
        }
        self.effects += 1;
        self.current.head = stage.head.clone();
        self.current.commit = stage.commit.clone();
        self.current.loose = stage.loose.clone();
        if after {
            Err(EffectFailure::OutcomeUnknown)
        } else {
            Ok(())
        }
    }
    fn journal(&mut self, _: &Lease, frame: &Value) -> Result<(), EffectFailure> {
        let id = frame["operation_id"].as_str().unwrap().to_owned();
        if let Some(old) = self.journals.get(&id) {
            assert_eq!(old, frame);
        }
        self.journals.insert(id, frame.clone());
        self.effects += 1;
        Ok(())
    }
    fn append(
        &mut self,
        _: &Lease,
        id: &str,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), EffectFailure> {
        let a = self.current.allocations.get_mut(id).unwrap();
        assert_eq!(a.bytes.len(), offset);
        self.effects += 1;
        if self.interrupt_append {
            self.interrupt_append = false;
            a.bytes.extend_from_slice(&bytes[..bytes.len() / 2]);
            return Err(EffectFailure::OutcomeUnknown);
        }
        a.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn verify_selected(
        &mut self,
        _: &Lease,
        p: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), ExecutionError> {
        self.check(&self.current, p, stage)
    }
    fn record_receipt(
        &mut self,
        _: &Lease,
        original: &Value,
        receipt: &Value,
    ) -> Result<(), EffectFailure> {
        let id = receipt["operation_id"].as_str().unwrap().to_owned();
        assert_eq!(original["request"]["receipt"], reference(receipt));
        if let Some(old) = self.receipts.get(&id) {
            assert_eq!(old, receipt);
        }
        self.receipts.insert(id, receipt.clone());
        Ok(())
    }
    fn cleanup_controls(&mut self, _: &Lease, _: &RepeatablePlan) -> Result<(), EffectFailure> {
        self.effects += 1;
        Ok(())
    }
}
fn lease(p: &OriginalPackage) -> Lease {
    crate::package::planning::io::test_lease(
        (),
        &encode(&p.manifest),
        &encode(&p.head),
        crate::package::planning::IncarnationId::parse("96000000-0000-4000-8000-000000000001")
            .unwrap(),
        package::DecimalU64::parse(p.commit["package_revision"].as_str().unwrap()).unwrap(),
    )
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One continuous pair of real operations and persisted restart boundaries"
)]
fn two_consecutive_operations_resume_the_same_original_after_partial_append() {
    let base = fixture();
    let original = verify(&base);
    let (intent, receipt) = request();
    let plan = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    assert_eq!(
        vector(&plan),
        "b56de6aa7dfa328f00b55d2263c1912e13146a4537ef110d5a985542c9ce9e82"
    );
    let mut io = Mock {
        current: original.package.clone(),
        base,
        receipts: BTreeMap::new(),
        journals: BTreeMap::new(),
        interrupt_append: true,
        interrupt_select: None,
        fail_stabilize: false,
        effects: 0,
    };
    let first_lease = lease(&io.current);
    let interrupted = execute(&mut io, &first_lease, &plan);
    assert!(
        matches!(
            interrupted,
            Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
        ),
        "unexpected execution result: {:?}",
        interrupted.err()
    );
    assert!(io.receipts.is_empty());
    let observed = snapshot(&io.base, &io.current);
    let id = identity(&io.current);
    let reader = support::limits();
    let context = RestartContext {
        inspection: &observed,
        registration: &io.base.registration,
        directory: v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        identity: &id,
        frames: frames(),
        reader: &reader,
        planner: budget(),
        max_originals: 2,
    };
    let retry = restore_plan(&io.current, &context).unwrap();
    assert_eq!(plan.original_bytes(), retry.original_bytes());
    let done = execute(&mut io, &first_lease, &retry).unwrap();
    assert_eq!(done.receipt(), &receipt);
    let observed = snapshot(&io.base, &io.current);
    let next = v1_3::reader::verify_operation_original(
        io.current.clone(),
        identity(&io.current),
        &observed,
        &io.base.registration,
        v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        frames(),
        &support::limits(),
        &retry,
        6,
    )
    .unwrap();
    let inputs = attempt(&next, 2000);
    let coordinate = core::original_coordinate(&next, &inputs.planner, budget()).unwrap();
    let mut intent2 = intent.clone();
    intent2["expected"] = coordinate.clone();
    intent2["operation_id"] = json!(uid(3000));
    intent2["command"]["envelope"]["command_id"] = json!(uid(3000));
    let graph = intent2["command"]["envelope"]["graph_id"]
        .as_str()
        .unwrap()
        .to_owned();
    intent2["command"]["envelope"]["expected_revision"] = json!(
        coordinate["graphs"][&graph]["revision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    );
    let mut receipt2 = receipt.clone();
    receipt2["operation_id"] = intent2["operation_id"].clone();
    receipt2["before"] = receipt["after"].clone();
    receipt2["after"] = receipt["after"].clone();
    receipt2["acceptance_ordinal"] = json!("5");
    receipt2["journal_sequence"] = json!("20");
    receipt2["request_sha256"] = json!(hash(&encode(&intent2)));
    let second = plan::compile_inner(&next, &intent2, &receipt2, inputs, budget()).unwrap();
    assert_eq!(
        vector(&second),
        "fdbbf81b9a6f5bd6472a2b515f45b1f029fa91576401745a576dd49c447e1915"
    );
    let second_lease = lease(&io.current);
    io.interrupt_append = true;
    assert!(matches!(
        execute(&mut io, &second_lease, &second),
        Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
    ));
    let observed = snapshot(&io.base, &io.current);
    let id = identity(&io.current);
    let context = RestartContext {
        inspection: &observed,
        registration: &io.base.registration,
        directory: v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        identity: &id,
        frames: frames(),
        reader: &reader,
        planner: budget(),
        max_originals: 2,
    };
    let restarted = restore_plan(&io.current, &context).unwrap();
    assert_eq!(restarted.original_bytes(), second.original_bytes());
    let done = execute(&mut io, &second_lease, &restarted).unwrap();
    assert_eq!(done.receipt(), &receipt2);
    assert_eq!(io.receipts.len(), 2);
    assert_eq!(io.journals.len(), 2);
    assert_ne!(
        plan.original()["semantic_target"]["active"],
        second.original()["semantic_target"]["active"]
    );
    let effects = io.effects;
    execute(&mut io, &second_lease, &second).unwrap();
    assert_eq!(io.effects, effects);
    let observed = snapshot(&io.base, &io.current);
    let last = second
        .verify_next_original(
            io.current.clone(),
            identity(&io.current),
            &observed,
            &io.base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &support::limits(),
        )
        .unwrap();
    let retry = admit(&last, &intent, &receipt, attempt(&last, 4000), budget()).unwrap();
    assert!(matches!(retry,Admission::ExistingReceipt(r) if r==receipt));
    let mut changed = intent.clone();
    changed["updated_at"] = json!("different");
    assert!(admit(&last, &changed, &receipt, attempt(&last, 4000), budget()).is_err());
}
#[test]
fn repeatable_preflight_refuses_changed_policy_collisions_and_corridors() {
    let fixture = fixture();
    let original = verify(&fixture);
    let (intent, receipt) = request();
    let input = attempt(&original, 1000);
    let mut wrong = input.clone();
    wrong.charge_unit = "1".into();
    assert!(compile(&original, &intent, &receipt, wrong, budget()).is_err());
    let mut wrong = input.clone();
    wrong.planner.active_root_id = original.proof.roles["active"].state["root_id"]
        .as_str()
        .unwrap()
        .into();
    assert!(compile(&original, &intent, &receipt, wrong, budget()).is_err());
    let mut wrong = input.clone();
    wrong.planner.associations[0].replacement = wrong.planner.associations[0].original.clone();
    assert!(compile(&original, &intent, &receipt, wrong, budget()).is_err());
    let mut wrong = input.clone();
    wrong.planner.corridors[0].original_sha256 = "0".repeat(64);
    assert!(compile(&original, &intent, &receipt, wrong, budget()).is_err());
    let mut wrong = input;
    wrong.project_limit = "1".into();
    assert!(compile(&original, &intent, &receipt, wrong, budget()).is_err());
}

#[test]
fn selector_outcome_unknown_reopens_the_exact_candidate_before_success() {
    let base = fixture();
    let original = verify(&base);
    let (intent, receipt) = request();
    let p = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    for (stage, after) in [(5, false), (5, true), (6, false), (6, true)] {
        let mut io = Mock {
            base: fixture(),
            current: original.package.clone(),
            receipts: BTreeMap::new(),
            journals: BTreeMap::new(),
            interrupt_append: false,
            interrupt_select: Some((
                p.stages[stage].commit["commit_id"].as_str().unwrap().into(),
                after,
            )),
            effects: 0,
            fail_stabilize: false,
        };
        let held = lease(&io.current);
        assert!(matches!(
            execute(&mut io, &held, &p),
            Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
        ));
        assert_eq!(io.receipts.is_empty(), stage == 5);
        let mut bad = io.current.clone();
        bad.loose.clear();
        std::mem::swap(&mut bad, &mut io.current);
        assert!(matches!(
            execute(&mut io, &held, &p),
            Err(ExecutionError::InvalidObservation)
        ));
        std::mem::swap(&mut bad, &mut io.current);
        io.fail_stabilize = true;
        assert!(matches!(
            execute(&mut io, &held, &p),
            Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
        ));
        io.fail_stabilize = false;
        assert_eq!(execute(&mut io, &held, &p).unwrap().receipt(), &receipt);
    }
}

#[cfg(target_os = "macos")]
#[path = "native_test.rs"]
mod native_test;

#[test]
fn native_evidence_charge_stays_inside_original_standing_allowance() {
    let fixture = fixture();
    let original = verify(&fixture);
    let (intent, receipt) = request();
    let p = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    let retained = EvidenceAllocationCharge {
        identity: EvidenceAllocationId::Registered {
            device: 7,
            inode: 90_001,
        },
        current_extent: 32,
        maximum_extent: 32,
        registered_charge: 16_384,
    };
    let journal = EvidenceAllocationCharge {
        identity: EvidenceAllocationId::Reserved { slot: 1 },
        current_extent: 0,
        maximum_extent: 5000,
        registered_charge: 0,
    };
    let receipt = EvidenceAllocationCharge {
        identity: EvidenceAllocationId::Reserved { slot: 2 },
        current_extent: 0,
        maximum_extent: 400,
        registered_charge: 0,
    };
    let proof = check_evidence_charge(&p, &[retained, journal, receipt], 4096).unwrap();
    assert_eq!(proof.evidence_peak(), 16_384 + 8192 + 4096 + 4096);
    assert_eq!(proof.total_peak(), p.control_peak() + proof.evidence_peak());
    assert_eq!(proof.remaining_standing() + proof.total_peak(), 262_144);
    assert_eq!(proof.allocation_count(), p.control_role_peak + 3);
    // Neither matching contents nor a smaller current extent refunds captured charge.
    assert_eq!(
        check_evidence_charge(&p, &[retained], 0)
            .unwrap()
            .evidence_peak(),
        16_384
    );
    assert!(check_evidence_charge(&p, &[retained, retained], 0).is_err());
    let huge = EvidenceAllocationCharge {
        maximum_extent: 262_144,
        ..journal
    };
    assert_eq!(
        check_evidence_charge(&p, &[huge], 0).err(),
        Some(package::PackageError::Limit)
    );
    let undercharged = EvidenceAllocationCharge {
        registered_charge: 0,
        ..retained
    };
    assert!(check_evidence_charge(&p, &[undercharged], 0).is_err());
    let shrinking = EvidenceAllocationCharge {
        maximum_extent: 0,
        ..retained
    };
    assert!(check_evidence_charge(&p, &[shrinking], 0).is_err());
    let allocations = (0..64)
        .map(|slot| EvidenceAllocationCharge {
            identity: EvidenceAllocationId::Reserved { slot },
            ..receipt
        })
        .collect::<Vec<_>>();
    assert_eq!(
        check_evidence_charge(&p, &allocations, 0).err(),
        Some(package::PackageError::Limit)
    );
}
