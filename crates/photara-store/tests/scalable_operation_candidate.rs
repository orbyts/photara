//! Linked operation byte candidate, using actual Core and PS1 application.
//! Pure memory only. No production dispatch, publication or permanent wire.
#[path = "scalable_operation_candidate/operations.rs"]
mod operations;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(
    dead_code,
    reason = "Reuse exact candidate frame parser without its bootstrap probe reader"
)]
mod packed;
use photara_core::{
    GraphCommandEnvelope, GraphDocument, NodeDefinitionRegistry, ValueTypeRegistry,
    apply_graph_command, canonical_json, creation::PackageExtension,
};
use photara_store::package::{MemoryPackage, PackageLimits, planning::*};
use serde_json::{Value, json};

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/operations/linked-operations.json"
    ))
    .unwrap()
}
fn bytes(value: &Value) -> Vec<u8> {
    canonical_json(value).unwrap()
}
fn coordinate_fixture(corpus: &Value, index: usize) -> &Value {
    &corpus["expected_coordinates"][index]
}
fn base(corpus: &Value) -> VerifiedClosure {
    let files = corpus["base_files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, v)| (name.clone(), v.as_str().unwrap().as_bytes().to_vec()))
        .collect();
    VerifiedClosure::verify(
        MemoryPackage::new(files, PackageLimits::default()).unwrap(),
        IncarnationId::parse("50000000-0000-4000-8000-000000000090").unwrap(),
    )
    .unwrap()
}
fn mutation(base: &VerifiedClosure, intent: &Value) -> MutationRequest {
    assert_eq!(
        serde_json::to_value(base.coordinate()).unwrap(),
        intent["expected"]
    );
    MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone()).unwrap(),
        expected: base.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(
                serde_json::from_value(intent["command"]["envelope"].clone()).unwrap(),
            ),
        },
        updated_at: intent["updated_at"].as_str().unwrap().into(),
    }
}
fn run(base: &VerifiedClosure, mutation: &MutationRequest, suffix: u64) -> PlanOutcome {
    let naming = PackageNamingPolicy {
        write_extension: PackageExtension::try_from("jprtest".to_owned()).unwrap(),
        legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
    };
    plan(
        base,
        PlanRequest {
            mutation,
            expected_head: base.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse(&format!("50000000-0000-4000-8000-{suffix:012}")).unwrap(),
                commit_id: serde_json::from_value(json!(format!(
                    "50000000-0000-4000-8000-{:012}",
                    suffix + 1
                )))
                .unwrap(),
            },
            naming: &naming,
            observed_extension: &naming.write_extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .unwrap()
}

fn assert_receipt_coordinates(receipt: &PreparedReceipt, expected: &Value) {
    for (actual, field) in [(receipt.before(), "before"), (receipt.after(), "after")] {
        let actual = serde_json::to_value(actual).unwrap();
        assert_eq!(
            json!({"revision":actual["revision"],"digest":actual["authored_digest"]}),
            expected[field]
        );
    }
}

#[test]
fn independent_fixed_bytes_match_core_mutations_and_real_ps1_authored_noop() {
    let corpus = corpus();
    let base = base(&corpus);
    assert_eq!(
        serde_json::to_value(base.coordinate()).unwrap(),
        *coordinate_fixture(&corpus, 0)
    );
    let intent = &corpus["records"]["intent-1"]["input"];
    let request = mutation(&base, intent);
    let PlanOutcome::Checkpoint(first) = run(&base, &request, 100) else {
        panic!("real mutation must checkpoint")
    };
    assert_eq!(
        serde_json::to_value(first.candidate().coordinate()).unwrap(),
        *coordinate_fixture(&corpus, 1)
    );
    for name in ["graph-1", "authored-1"] {
        let expected = &corpus["records"][name];
        let path = format!(
            "objects/json/sha256/{}.json",
            expected["sha256"].as_str().unwrap()
        );
        assert_eq!(
            first.candidate().files().files()[&path].as_ref(),
            expected["canonical"].as_str().unwrap().as_bytes()
        );
    }
    assert_receipt_coordinates(first.receipt(), &corpus["records"]["receipt-1"]["input"]);
    let first_graph: GraphDocument =
        serde_json::from_value(corpus["records"]["graph-1"]["input"]["graph"].clone()).unwrap();
    let noop_intent = &corpus["records"]["intent-2"]["input"];
    let envelope: GraphCommandEnvelope =
        serde_json::from_value(noop_intent["command"]["envelope"].clone()).unwrap();
    let raw = apply_graph_command(
        &first_graph,
        &envelope,
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .unwrap();
    assert_eq!(raw.graph.revision.get(), first_graph.revision.get() + 1);
    assert_ne!(
        canonical_json(&raw.graph).unwrap(),
        canonical_json(&first_graph).unwrap()
    );
    let mut same_revision = raw.graph;
    same_revision.revision = first_graph.revision;
    assert_eq!(
        canonical_json(&same_revision).unwrap(),
        canonical_json(&first_graph).unwrap(),
        "Core changed no authored content apart from its revision result"
    );
    let second_request = mutation(first.candidate(), noop_intent);
    let original_files = first.candidate().files().files().clone();
    let PlanOutcome::Unchanged(noop) = run(first.candidate(), &second_request, 200) else {
        panic!("existing PS1 no-op normalization must return Unchanged")
    };
    assert_eq!(noop.before(), noop.after());
    assert_receipt_coordinates(&noop, &corpus["records"]["receipt-2"]["input"]);
    assert_eq!(
        serde_json::to_value(noop.after()).unwrap(),
        *coordinate_fixture(&corpus, 1)
    );
    assert_eq!(*first.candidate().files().files(), original_files);
    let third_request = mutation(first.candidate(), &corpus["records"]["intent-3"]["input"]);
    let PlanOutcome::Checkpoint(third) = run(first.candidate(), &third_request, 300) else {
        panic!("later mutation")
    };
    assert_eq!(
        serde_json::to_value(third.candidate().coordinate()).unwrap(),
        *coordinate_fixture(&corpus, 2)
    );
    assert_ne!(
        third.candidate().coordinate(),
        first.candidate().coordinate()
    );
    assert_receipt_coordinates(third.receipt(), &corpus["records"]["receipt-3"]["input"]);
    for name in ["graph-2", "authored-2"] {
        let expected = &corpus["records"][name];
        let path = format!(
            "objects/json/sha256/{}.json",
            expected["sha256"].as_str().unwrap()
        );
        assert_eq!(
            third.candidate().files().files()[&path].as_ref(),
            expected["canonical"].as_str().unwrap().as_bytes()
        );
    }
}

#[test]
fn operation_canonical_bytes_and_frames_are_independently_fixed() {
    let corpus = corpus();
    for entry in corpus["records"].as_object().unwrap().values() {
        let expected = entry["canonical"].as_str().unwrap().as_bytes();
        assert_eq!(bytes(&entry["input"]), expected);
        assert_eq!(packed::hash(expected), entry["sha256"]);
        assert_eq!(expected.len() as u64, entry["byte_length"]);
        if let Some(hex) = entry["frame_hex"].as_str() {
            let frame = packed::unhex(hex).unwrap();
            assert_eq!(packed::hash(&frame), entry["frame_sha256"]);
            assert_eq!(&frame[16..], expected);
            assert_eq!(
                packed::frame_ranges(&frame, "data").unwrap(),
                vec![(0, frame.len(), 1)]
            );
        }
    }
}

#[test]
fn typed_operation_indexes_share_original_recovery_prefix_and_distinct_journal_coordinates() {
    let corpus = corpus();
    let store = operations::Store::load(&corpus).unwrap();
    let (active, recovery) = operations::verify(&store).unwrap();
    assert_eq!(active.records.len(), 3);
    assert_eq!(recovery.records.len(), 2);
    assert_eq!(active.records[..2], recovery.records);
    for (n, receipt) in active.records.iter().enumerate() {
        assert_eq!(
            *receipt,
            corpus["records"][format!("receipt-{}", n + 1)]["input"]
        );
        assert_eq!(receipt["acceptance_ordinal"], (n + 1).to_string());
        assert_eq!(receipt["journal_sequence"], ["3", "7", "11"][n]);
    }
    assert_eq!(active.records[1]["before"], active.records[1]["after"]);
    assert_eq!(
        active.accepted_prefix,
        corpus["expected_accepted_prefixes"][3]
    );
    assert_eq!(
        recovery.accepted_prefix,
        corpus["expected_accepted_prefixes"][2]
    );
    let mut independent = store.clone();
    independent.retain(&recovery.closure).unwrap();
    assert_eq!(
        operations::verify_role(&independent, "recovery")
            .unwrap()
            .records,
        recovery.records
    );
    assert!(operations::verify_role(&independent, "active").is_err());
}

#[test]
fn same_id_retry_after_later_real_state_returns_original_result_and_provenance_without_changes() {
    let corpus = corpus();
    let store = operations::Store::load(&corpus).unwrap();
    let before = store.fingerprint();
    let intent = &corpus["records"]["intent-1"]["input"];
    let original = operations::retry(&store, intent).unwrap();
    assert_eq!(original, corpus["records"]["receipt-1"]["input"]);
    assert_eq!(original["provenance"], corpus["original_provenance"]);
    assert_ne!(
        original["after"],
        corpus["records"]["receipt-3"]["input"]["after"]
    );
    assert_eq!(store.fingerprint(), before);
    let mut changed = intent.clone();
    changed["command"]["envelope"]["command"]["x"] = json!(999);
    assert_eq!(
        operations::retry(&store, &changed).err(),
        Some("same ID different semantic intent")
    );
    assert_eq!(store.fingerprint(), before);
}

#[test]
fn ordinal_gaps_duplicates_and_disagreeing_id_index_refuse() {
    let corpus = corpus();
    let original = operations::Store::load(&corpus).unwrap();
    for mode in 0..3 {
        let mut bad = original.clone();
        operations::rewrite_leaf(
            &mut bad,
            if mode == 2 { "by_id" } else { "by_ordinal" },
            |v| match mode {
                0 => {
                    v["entries"][1]["acceptance_ordinal"] = json!("1");
                }
                1 => {
                    v["entries"].as_array_mut().unwrap().remove(1);
                    v["count"] = json!("2");
                }
                _ => {
                    v["entries"][0]["request_sha256"] = json!("f".repeat(64));
                }
            },
        )
        .unwrap();
        assert_eq!(
            operations::verify_role(&bad, "active").err(),
            Some(
                [
                    "contiguous unique ordinals",
                    "two index counts",
                    "two indexes disagree"
                ][mode]
            )
        );
    }
}

#[test]
fn original_provenance_and_journal_projection_cannot_be_rewritten() {
    let corpus = corpus();
    let mut bad = operations::Store::load(&corpus).unwrap();
    operations::replace_original_provenance(&mut bad).unwrap();
    assert_eq!(
        operations::verify_role(&bad, "active").err(),
        Some("journal original receipt binding")
    );
    for mode in 0..3 {
        let mut bad = operations::Store::load(&corpus).unwrap();
        match mode {
            0 => {
                bad.journal[1]["sequence"] = json!("2");
            }
            1 => {
                bad.journal.remove(0);
            }
            _ => {
                bad.roots["active"]["journal_inclusion"]["resulting_authored_sha256"] =
                    json!("e".repeat(64));
            }
        }
        assert!(operations::verify_role(&bad, "active").is_err());
    }
}

#[test]
fn original_before_after_digests_do_not_retain_obsolete_authored_records() {
    let corpus = corpus();
    let mut active_only = operations::Store::load(&corpus).unwrap();
    let active = operations::verify_role(&active_only, "active").unwrap();
    active_only.retain(&active.closure).unwrap();
    for old in ["authored-0", "graph-0", "authored-1", "graph-1"] {
        let v = &corpus["records"][old];
        let reference = json!({"kind":"json","sha256":v["sha256"],"byte_length":v["byte_length"].as_u64().unwrap().to_string()});
        assert!(
            active_only.get(&reference).is_err(),
            "obsolete bytes absent for {old}"
        );
    }
    assert_eq!(
        operations::verify_role(&active_only, "active")
            .unwrap()
            .records,
        active.records
    );
    assert_eq!(
        operations::retry(&active_only, &corpus["records"]["intent-1"]["input"]).unwrap(),
        corpus["records"]["receipt-1"]["input"]
    );
}
