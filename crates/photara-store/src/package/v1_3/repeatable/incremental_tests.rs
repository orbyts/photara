//! Build-gated v2 tests: actual Core, shared reader and synthetic IO only.
use super::*;
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One v1-to-v2-to-v2 chain verifies exact restart and independent selected closure"
)]
fn v2_selected_chain_preserves_v1_and_exact_restart() {
    let base = fixture();
    let mut old = verify(&base);
    let (mut intent, mut receipt) = request();
    let mut io = Mock {
        current: old.package.clone(),
        base,
        receipts: BTreeMap::new(),
        journals: BTreeMap::new(),
        interrupt_append: false,
        interrupt_select: None,
        fail_stabilize: false,
        effects: 0,
    };
    let mut vectors = Vec::new();
    let mut memo = restart::CompiledPlans::default();
    let mut decoded = Vec::new();
    for n in 0..3u64 {
        let mut inputs = attempt(&old, 1000 + n * 2000);
        if n > 0 {
            inputs.planner.codec = "photara.codec.ps2-repeatable-layout-v2".into();
        }
        if n > 0 {
            let coordinate = core::original_coordinate(&old, &inputs.planner, budget()).unwrap();
            intent["expected"] = coordinate.clone();
            intent["operation_id"] = json!(uid(20000 + n));
            intent["command"]["envelope"]["command_id"] = intent["operation_id"].clone();
            let graph = intent["command"]["envelope"]["graph_id"]
                .as_str()
                .unwrap()
                .to_owned();
            intent["command"]["envelope"]["expected_revision"] = json!(
                coordinate["graphs"][&graph]["revision"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
            receipt["operation_id"] = intent["operation_id"].clone();
            receipt["before"] = receipt["after"].clone();
            receipt["acceptance_ordinal"] = json!((4 + n).to_string());
            receipt["journal_sequence"] = json!((19 + n).to_string());
            receipt["request_sha256"] = json!(hash(&encode(&intent)));
        }
        layout::take_load_work();
        let plan = memo
            .compile(&old, &intent, &receipt, inputs.clone(), budget(), 8)
            .unwrap_or_else(|e| panic!("operation {n}: {e}"));
        let cached_decodes = layout::take_load_work().1;
        let uncached = compile(&old, &intent, &receipt, inputs, budget()).unwrap();
        let uncached_decodes = layout::take_load_work().1;
        assert_eq!(vector(&plan), vector(&uncached));
        if n > 0 {
            assert!(cached_decodes < uncached_decodes);
        }
        decoded.push((cached_decodes, uncached_decodes));
        if n == 0 {
            assert_eq!(
                vector(&plan),
                "b56de6aa7dfa328f00b55d2263c1912e13146a4537ef110d5a985542c9ce9e82"
            );
        } else {
            assert!(
                plan.original["payload_recipe"]
                    .get("retained_controls")
                    .is_none()
            );
            assert_eq!(
                plan.original["payload_recipe"]["retained_control_inventory"]["base"]["kind"],
                if n == 1 { "legacy" } else { "tree" }
            );
        }
        vectors.push(vector(&plan));
        let held = lease(&io.current);
        io.interrupt_append = n > 0;
        if n > 0 {
            assert!(execute(&mut io, &held, &plan).is_err());
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
                max_originals: 8,
            };
            let restored = restore_plan(&io.current, &context).unwrap();
            assert_eq!(vector(&restored), vector(&plan));
            execute(&mut io, &held, &restored).unwrap();
        } else {
            execute(&mut io, &held, &plan).unwrap();
        }
        let observed = snapshot(&io.base, &io.current);
        old = plan
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
    }
    assert_eq!(
        vectors,
        [
            "b56de6aa7dfa328f00b55d2263c1912e13146a4537ef110d5a985542c9ce9e82",
            "8b3d1c1fb1aa79868568350f67dabb46e916b2ec8aa104470bc020733999a305",
            "71070d53e6f2a34408bc1f4f396b63dade03055bc3fec498907e5a8c4539276c",
        ]
    );
    eprintln!("v2 chain vectors: {vectors:?}; canonical decoded per operation {decoded:?}");
    v1_3::retained_inventory::audit_regressions();
}

#[test]
fn v2_inventory_vectors_splits_removals_and_seed_refusals() {
    use layout::incremental::{Spec, audit, tree};
    let base = fixture();
    let old = verify(&base);
    let inputs = attempt(&old, 1000);
    let mut b = layout::Builder::load(&old.package, &inputs.planner, budget()).unwrap();
    let name = "photara.storage.retained-control-inventory";
    let spec = || Spec {
        name,
        field: "record",
        capacity: 1,
        allocation: None,
    };
    let a = json!({"record":{"kind":"json","sha256":"a".repeat(64),"byte_length":"1"},"membership":"ownership"});
    let z = json!({"record":{"kind":"json","sha256":"b".repeat(64),"byte_length":"2"},"membership":"semantic"});
    let (empty, _) = tree(&mut b, None, vec![], spec()).unwrap();
    assert_eq!(
        empty["sha256"],
        "cc2273172590226214739fca9d25ea8d9c2e1e9d288229b5c053d8750fb08c5a"
    );
    let (one, _) = tree(&mut b, Some(&empty), vec![a.clone()], spec()).unwrap();
    assert_eq!(
        one["sha256"],
        "9589cf84a0d58cb4f401f20642f008a18d640bf54c0a1dc7fee1a66071dc6549"
    );
    let (two, _) = tree(&mut b, Some(&one), vec![a.clone(), z.clone()], spec()).unwrap();
    assert_eq!(
        two["sha256"],
        "221400d6eb5f4c6f3cef37fb37f59577bd127663170bd6520450122597386ecb"
    );
    let (back, _) = tree(&mut b, Some(&two), vec![a], spec()).unwrap();
    assert_eq!(back, one);
    let (none, _) = tree(&mut b, Some(&back), vec![], spec()).unwrap();
    assert_eq!(none, empty);
    let mut malformed = b.pool[&wire::key(&two).unwrap()].clone();
    malformed["extensions"] = json!({"unknown":true});
    let r = reference(&malformed);
    b.pool.insert(wire::key(&r).unwrap(), malformed);
    assert!(audit(&b, &r, spec()).is_err());
    let mut malformed = b.pool[&wire::key(&two).unwrap()].clone();
    malformed["children"].as_array_mut().unwrap().reverse();
    let r = reference(&malformed);
    b.pool.insert(wire::key(&r).unwrap(), malformed);
    assert!(audit(&b, &r, spec()).is_err());
    let target=(0..17u64).map(|n|json!({"record":{"kind":"json","sha256":format!("{n:064x}"),"byte_length":"1"},"membership":"ownership"})).collect::<Vec<_>>();
    let mut narrow = inputs.planner.clone();
    narrow.geometry.branch = 2;
    let mut b = layout::Builder::load(&old.package, &narrow, budget()).unwrap();
    let (grown, _) = tree(&mut b, None, target.clone(), spec()).unwrap();
    assert_eq!(audit(&b, &grown, spec()).unwrap(), target);
    let (same, unchanged) = tree(&mut b, Some(&grown), target.clone(), spec()).unwrap();
    assert_eq!(same, grown);
    assert!(!unchanged.is_empty());
    let (shrunk, _) = tree(&mut b, Some(&grown), target[..2].to_vec(), spec()).unwrap();
    assert_eq!(audit(&b, &shrunk, spec()).unwrap(), target[..2]);
    let physical = &old.package.commit["root_set"]["placement"]["root_placements"];
    assert!(
        audit(
            &b,
            physical,
            Spec {
                name: "photara.storage.root-placement",
                field: "root",
                capacity: 4,
                allocation: Some(&aid(3))
            }
        )
        .is_err()
    );
}

#[test]
fn v2_decoding_memo_preserves_changed_bytes_and_late_limits() {
    let base = fixture();
    let old = verify(&base);
    let inputs = attempt(&old, 1000);
    let mut memo = canonical_memo::CanonicalMemo::default();
    layout::take_load_work();
    let first =
        layout::Builder::load_cached(&old.package, &inputs.planner, budget(), Some(&mut memo))
            .unwrap();
    let cold = layout::take_load_work().1;
    let warm =
        layout::Builder::load_cached(&old.package, &inputs.planner, budget(), Some(&mut memo))
            .unwrap();
    assert_eq!(layout::take_load_work().1, 0);
    assert!(cold > 0);
    assert_eq!(first.pool, warm.pool);
    for change in 0..4 {
        let mut limits = budget();
        match change {
            0 => limits.semantic.json.max_depth = 1,
            1 => limits.semantic.json.max_members = 1,
            2 => limits.semantic.json.max_array_elements = 1,
            _ => limits.semantic.max_total_json_bytes = 1,
        }
        assert!(
            layout::Builder::load_cached(&old.package, &inputs.planner, limits, Some(&mut memo))
                .is_err()
        );
        assert!(layout::Builder::load(&old.package, &inputs.planner, limits).is_err());
    }
    let mut changed = old.package.clone();
    changed.loose.values_mut().next().unwrap()[0] = b'!';
    assert!(
        layout::Builder::load_cached(&changed, &inputs.planner, budget(), Some(&mut memo)).is_err()
    );
    memo.set_maximum(1);
    let bounded =
        layout::Builder::load_cached(&old.package, &inputs.planner, budget(), Some(&mut memo))
            .unwrap();
    assert_eq!(bounded.pool, first.pool);
    assert_eq!(memo.bytes, 0);
}
