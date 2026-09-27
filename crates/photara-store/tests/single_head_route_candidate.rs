//! Original-token route candidate. New files only; no production code or wire freeze.
#[path = "single_head_route_candidate/contract.rs"]
mod contract;
#[path = "single_head_route_candidate/control.rs"]
mod control;
#[path = "scalable_joined_candidate/joined.rs"]
#[allow(dead_code, reason = "Shared actual selected typed traversal")]
mod joined;
#[path = "scalable_operation_candidate/operations.rs"]
#[allow(
    dead_code,
    reason = "Validate original portable receipt/index semantics"
)]
mod operations;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(dead_code, reason = "Reuse candidate exact frame parser")]
mod packed;
#[path = "resource_conversion_candidate/wire.rs"]
#[allow(dead_code, reason = "Selected resources and retained conversion")]
mod resource;
#[path = "single_head_route_candidate/route.rs"]
mod route;
#[path = "scalable_branch_candidate/trees.rs"]
#[allow(dead_code, reason = "Typed multi-level indexes")]
mod trees;
use photara_core::{
    NodeDefinitionRegistry, ValueTypeRegistry, canonical_json, creation::PackageExtension,
};
use photara_store::package::{MemoryPackage, PackageLimits, planning::*};
use serde_json::{Value, json};
const OPERATIONS: &str =
    include_str!("../../../docs/architecture/proposals/ps2/route/operation-four.json");
fn corpus() -> Value {
    serde_json::from_str(OPERATIONS).unwrap()
}
fn base(c: &Value) -> VerifiedClosure {
    let files = c["base_files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().as_bytes().to_vec()))
        .collect();
    VerifiedClosure::verify(
        MemoryPackage::new(files, PackageLimits::default()).unwrap(),
        IncarnationId::parse("50000000-0000-4000-8000-000000000090").unwrap(),
    )
    .unwrap()
}
fn apply(base: &VerifiedClosure, intent: &Value, suffix: u64) -> PlanOutcome {
    assert_eq!(
        serde_json::to_value(base.coordinate()).unwrap(),
        intent["expected"]
    );
    let mutation = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone()).unwrap(),
        expected: base.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(
                serde_json::from_value(intent["command"]["envelope"].clone()).unwrap(),
            ),
        },
        updated_at: intent["updated_at"].as_str().unwrap().into(),
    };
    let naming = PackageNamingPolicy {
        write_extension: PackageExtension::try_from("jprtest".to_string()).unwrap(),
        legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
    };
    plan(
        base,
        PlanRequest {
            mutation: &mutation,
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
#[test]
fn fourth_real_core_operation_continues_exact_settled_authored_coordinate() {
    let c = corpus();
    for row in c["records"].as_object().unwrap().values() {
        let bytes = canonical_json(&row["input"]).unwrap();
        assert_eq!(bytes, row["canonical"].as_str().unwrap().as_bytes());
        assert_eq!(packed::hash(&bytes), row["sha256"]);
    }
    let initial = base(&c);
    let PlanOutcome::Checkpoint(first) = apply(&initial, &c["records"]["intent-1"]["input"], 100)
    else {
        panic!("first checkpoint")
    };
    let PlanOutcome::Unchanged(_) =
        apply(first.candidate(), &c["records"]["intent-2"]["input"], 200)
    else {
        panic!("authored noop")
    };
    let PlanOutcome::Checkpoint(third) =
        apply(first.candidate(), &c["records"]["intent-3"]["input"], 300)
    else {
        panic!("third checkpoint")
    };
    let PlanOutcome::Checkpoint(fourth) =
        apply(third.candidate(), &c["records"]["intent-4"]["input"], 400)
    else {
        panic!("fourth checkpoint")
    };
    assert_eq!(
        serde_json::to_value(fourth.candidate().coordinate()).unwrap(),
        c["expected_coordinates"][3]
    );
    for name in ["graph-3", "authored-3"] {
        let row = &c["records"][name];
        let path = format!(
            "objects/json/sha256/{}.json",
            row["sha256"].as_str().unwrap()
        );
        assert_eq!(
            fourth.candidate().files().files()[&path].as_ref(),
            row["canonical"].as_str().unwrap().as_bytes()
        );
    }
    for (coordinate, field) in [
        (fourth.receipt().before(), "before"),
        (fourth.receipt().after(), "after"),
    ] {
        let coordinate = serde_json::to_value(coordinate).unwrap();
        assert_eq!(
            json!({"revision":coordinate["revision"],"digest":coordinate["authored_digest"]}),
            c["records"]["receipt-4"]["input"][field]
        );
    }
    let mut store = operations::Store::load(&c).unwrap();
    store
        .journal
        .push(c["records"]["journal-frame-4"]["input"].clone());
    let (active, recovery) = operations::verify(&store).unwrap();
    assert_eq!(active.records.len(), 4);
    assert_eq!(recovery.records.len(), 2);
    assert_eq!(active.records[3]["journal_sequence"], "19");
    assert_eq!(active.records[3]["acceptance_ordinal"], "4");
    for n in 1..=3 {
        assert_eq!(
            operations::retry(&store, &c["records"][format!("intent-{n}")]["input"]).unwrap(),
            c["records"][format!("receipt-{n}")]["input"]
        );
    }
}
