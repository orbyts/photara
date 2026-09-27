//! UNFROZEN resource factoring, pure metadata projection; no production dispatch.
#[allow(
    dead_code,
    reason = "Frozen legacy byte helpers are shared without modifying their independent integration target"
)]
#[path = "resource_conversion_candidate/wire.rs"]
mod legacy;
#[path = "resource_factored_candidate/wire.rs"]
mod wire;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/resource-factored/linked.json"
    ))
    .unwrap()
}
#[test]
fn independent_factored_canonical_bytes_are_exact() {
    let c = corpus();
    for scenario in c["scenarios"]
        .as_object()
        .unwrap()
        .values()
        .chain(c["scales"].as_object().unwrap().values())
        .chain(std::iter::once(&c["source_scale"]))
    {
        for (sha, record) in scenario["records"].as_object().unwrap() {
            let bytes = photara_core::canonical_json(&record["input"]).unwrap();
            assert_eq!(bytes, record["canonical"].as_str().unwrap().as_bytes());
            assert_eq!(legacy::hash(&bytes), *sha);
            assert_eq!(record["byte_length"], bytes.len());
            assert_eq!(legacy::parse(&bytes).unwrap(), record["input"]);
        }
    }
}
#[test]
fn distinct_roots_share_all_requirements_and_only_bounded_local_bytes_change() {
    let c = corpus();
    let mut maxima = 0;
    for n in ["1", "16", "64"] {
        let s = wire::Store::load(&c["scales"][n]).unwrap();
        let (a, b) = wire::verify_pair(&s).unwrap();
        assert!(a.supported && b.supported);
        assert_ne!(s.roots["a"]["root_id"], s.roots["b"]["root_id"]);
        assert_eq!(a.requirements, b.requirements);
        assert_eq!(a.requirements.len(), n.parse::<usize>().unwrap());
        assert_eq!(
            a.closure
                .difference(&b.closure)
                .cloned()
                .collect::<BTreeSet<_>>(),
            a.root_local
        );
        assert_eq!(
            b.closure
                .difference(&a.closure)
                .cloned()
                .collect::<BTreeSet<_>>(),
            b.root_local
        );
        assert!(b.root_local.len() <= 4);
        let changed = b.root_local.iter().map(|(_, n)| n).sum::<u64>();
        assert!(changed <= 3072);
        maxima = maxima.max(changed);
        let mut only_b = s.clone();
        for (sha, _) in a.closure.difference(&b.closure) {
            only_b.records.remove(sha);
        }
        assert_eq!(
            wire::retained(&only_b, "b").unwrap().requirements,
            b.requirements
        );
        wire::representation(&s, "a").unwrap();
        wire::representation(&s, "b").unwrap();
        println!(
            "resources={n} shared={} changed_records={} changed_bytes={changed}",
            a.closure.intersection(&b.closure).count(),
            b.root_local.len()
        );
    }
    assert!(maxima > 0);
}
#[test]
fn rehashed_factored_negatives_reach_typed_invariants() {
    let c = corpus();
    for (name, error) in [
        ("wrong-root", "resolved selected source association"),
        ("unresolved-origin", "resolved selected source association"),
        ("uncovered-requirement", "exact source coverage"),
        ("missing-backing", "required backing selected"),
        ("zero-copies", "positive obligation copies"),
        ("unknown-field", "exact fields"),
        ("old-requirement-version", "schema/project"),
        ("old-state-version", "schema/project"),
        ("wrong-tree-kind", "tree selection kind"),
        ("unknown-tree-field", "exact fields"),
        ("capture-policy-pointer", "exact fields"),
        (
            "conflicting-source-requirement",
            "source subset exact shared requirement",
        ),
        ("wrong-tree-range", "exact child range/count"),
        ("duplicate-source-id", "selection unique order"),
        ("changed-source-id", "selected target identity"),
        ("source-alias", "tree cycle/alias/depth bound"),
    ] {
        let s = wire::Store::load(&c["scenarios"][name]).unwrap();
        assert_eq!(wire::verify(&s, "b").err(), Some(error), "{name}");
    }
}
#[test]
fn unsatisfied_requirements_remain_structurally_readable() {
    let c = corpus();
    for name in ["two-copies", "wrong-qualification"] {
        let s = wire::Store::load(&c["scenarios"][name]).unwrap();
        assert!(!wire::verify(&s, "b").unwrap().supported);
        assert_eq!(
            wire::retained(&s, "b").err(),
            Some("fixture requirement unsupported")
        );
    }
}
#[test]
fn capability_and_v3_dispatch_are_explicit() {
    let c = corpus();
    let mut s = wire::Store::load(&c["scenarios"]["valid"]).unwrap();
    s.roots["a"]["required_features"] = json!(["photara.resource-backings.v1"]);
    assert_eq!(wire::verify(&s, "a").err(), Some("factored capability"));
}

#[test]
fn representation_v3_remains_explicit_and_opaque_extensions_stay_nonedges() {
    let c = corpus();
    let mut s = wire::Store::load(&c["scenarios"]["valid"]).unwrap();
    wire::representation(&s, "a").unwrap();
    let proof = wire::verify(&s, "a").unwrap();
    assert!(!proof.closure.iter().any(|(h, _)| h == &"0".repeat(64)));
    let mut value =
        legacy::parse(&s.records[s.representation["sha256"].as_str().unwrap()]).unwrap();
    value["schema"]["version"] = json!(2);
    let raw = photara_core::canonical_json(&value).unwrap();
    let sha = legacy::hash(&raw);
    s.representation = json!({"kind":"json","sha256":sha,"byte_length":raw.len().to_string()});
    s.records.insert(sha, raw);
    assert_eq!(wire::representation(&s, "a").err(), Some("schema/project"));
}

#[test]
fn existing_selected_v1_metadata_and_v3_binding_bytes_are_unchanged() {
    let old: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/resource-conversion/linked.json"
    ))
    .unwrap();
    let old = &old["scenarios"]["valid"];
    let context = &old["records"][old["roots"]["active"]["sha256"].as_str().unwrap()]["input"];
    let c = corpus();
    let s = wire::Store::load(&c["scenarios"]["valid"]).unwrap();
    let mut count = 0;
    for r in context["inventory"].as_array().unwrap() {
        let h = r["sha256"].as_str().unwrap();
        let record = &old["records"][h];
        let schema = record["input"]["schema"]["id"].as_str().unwrap();
        if matches!(
            schema,
            "photara.resource.state" | "photara.resource.retention-obligation"
        ) {
            continue;
        }
        assert_eq!(
            s.records[h],
            record["canonical"].as_str().unwrap().as_bytes()
        );
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn changed_root_origin_requires_a_new_immutable_association_identity() {
    let c = corpus();
    let s = wire::Store::load(&c["scenarios"]["reused-source-id"]).unwrap();
    wire::verify(&s, "a").unwrap();
    wire::verify(&s, "b").unwrap();
    assert_eq!(
        wire::verify_pair(&s).err(),
        Some("immutable association identity across roots")
    );
}

#[test]
fn source_association_selection_uses_real_branches_beyond_former_array_bound() {
    let c = corpus();
    let s = wire::Store::load(&c["source_scale"]).unwrap();
    let (a, b) = wire::verify_pair(&s).unwrap();
    assert!(a.supported && b.supported);
    assert_eq!(a.associations.len(), 16);
    assert_eq!(a.requirements, b.requirements);
    assert!(a.root_local.len() > 17);
    println!(
        "source_associations=16 root_local_records={} changed_bytes={}",
        b.root_local.len(),
        b.root_local.iter().map(|(_, n)| n).sum::<u64>()
    );
}
