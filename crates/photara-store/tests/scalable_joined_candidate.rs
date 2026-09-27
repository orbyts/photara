//! Joined settled candidate. Experimental, pure memory; no production wire reader.
#[path = "scalable_joined_candidate/joined.rs"]
mod joined;
#[path = "scalable_operation_candidate/operations.rs"]
#[allow(dead_code, reason = "Original real operation oracle")]
mod operations;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(dead_code, reason = "Reuse exact frame parser")]
mod packed;
#[path = "resource_conversion_candidate/wire.rs"]
#[allow(
    dead_code,
    reason = "Reuse selected resource and exact conversion validators"
)]
mod resource;
#[path = "scalable_branch_candidate/trees.rs"]
#[allow(dead_code, reason = "Reuse typed branch traversal")]
mod trees;
use joined::World;
use serde_json::{Value, json};
use std::fmt::Write as _;
const CORPUS: &str =
    include_str!("../../../docs/architecture/proposals/ps2/joined/linked-joined.json");
const OPS: &str =
    include_str!("../../../docs/architecture/proposals/ps2/operations/linked-operations.json");
const RESOURCE: &str =
    include_str!("../../../docs/architecture/proposals/ps2/resource-conversion/linked.json");
fn corpus() -> Value {
    serde_json::from_str(CORPUS).unwrap()
}
fn ops() -> Value {
    serde_json::from_str(OPS).unwrap()
}
fn profile() -> Value {
    serde_json::from_str::<Value>(RESOURCE).unwrap()["scenarios"]["valid"]["fixture_profile"]
        .clone()
}
fn setup() -> World {
    World::load(&corpus()).unwrap()
}
#[test]
fn joined_selected_bootstrap_resolves_all_typed_bytes_and_counts_each_allocation_once() {
    let c = corpus();
    for row in c["records"].as_object().unwrap().values() {
        let b = photara_core::canonical_json(&row["input"]).unwrap();
        assert_eq!(b, row["canonical"].as_str().unwrap().as_bytes());
        assert_eq!(packed::hash(&b), row["sha256"]);
        assert_eq!(b.len() as u64, row["byte_length"].as_u64().unwrap());
    }
    let w = setup();
    assert_eq!(joined::verify(&w, &ops(), &profile()).unwrap(), 761_856);
    assert_eq!(w.allocations.len(), 7);
    assert_eq!(w.source.len(), 18);
}
#[test]
fn joined_recovery_reads_its_complete_state_without_active_only_allocations() {
    let mut w = setup();
    let proof = joined::verify_role(&w, &ops(), &profile(), "recovery").unwrap();
    assert_eq!(proof.claims.len(), 5);
    assert!(proof.resource_supported);
    assert_eq!(proof.state["accepted"]["through_ordinal"], "2");
    assert_eq!(proof.physical_objects.len(), 124);
    w.allocations.retain(|k, _| proof.used.contains(k));
    joined::verify_role(&w, &ops(), &profile(), "recovery").unwrap();
    assert!(joined::verify_role(&w, &ops(), &profile(), "active").is_err());
}
#[test]
fn joined_wrong_selector_capability_and_active_projection_refuse() {
    for n in 0..4 {
        let mut w = setup();
        match n {
            0 => w.commit["root_set"]["active"] = w.commit["root_set"]["recovery"].clone(),
            1 => {
                w.commit["required_features"].as_array_mut().unwrap().pop();
            }
            2 => w.commit["authored"] = w.commit["history"].clone(),
            _ => {
                w.commit["root_set"]["pinned_roots"] =
                    json!([w.commit["root_set"]["recovery"].clone()]);
            }
        }
        w.rehash_head();
        assert!(joined::verify(&w, &ops(), &profile()).is_err(), "{n}");
    }
}
#[test]
fn joined_absent_sealed_bytes_same_inode_substitution_and_conversion_loss_refuse() {
    let mut w = setup();
    w.allocations.remove("80000000-0000-4000-8000-000000000001");
    assert!(joined::verify(&w, &ops(), &profile()).is_err());
    let mut w = setup();
    w.allocations
        .get_mut("80000000-0000-4000-8000-000000000004")
        .unwrap()
        .witness["inode"] = json!("999");
    assert!(joined::verify(&w, &ops(), &profile()).is_err());
    let mut w = setup();
    let name = w.source.keys().last().unwrap().clone();
    w.source.remove(&name);
    assert!(joined::verify(&w, &ops(), &profile()).is_err());
}
fn negative(name: &str) -> World {
    let deltas: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/joined/joined-negatives.json"
    ))
    .unwrap();
    let delta = &deltas[name];
    let mut c = corpus();
    for field in ["bootstrap", "loose", "source_files", "source_witnesses"] {
        c[field] = delta[field].clone();
    }
    for (allocation, changes) in delta["allocations"].as_object().unwrap() {
        let mut raw = packed::unhex(c["allocations"][allocation]["hex"].as_str().unwrap()).unwrap();
        for patch in changes["patches"].as_array().unwrap() {
            let start = usize::try_from(patch["offset"].as_u64().unwrap()).unwrap();
            let bytes = packed::unhex(patch["hex"].as_str().unwrap()).unwrap();
            raw[start..start + bytes.len()].copy_from_slice(&bytes);
        }
        c["allocations"][allocation]["hex"] =
            json!(raw.iter().fold(String::new(), |mut out, b| {
                write!(out, "{b:02x}").unwrap();
                out
            }));
        c["allocations"][allocation]["sha256"] = changes["sha256"].clone();
    }
    World::load(&c).unwrap()
}
#[test]
fn coherently_reframed_resource_and_conversion_membership_refuse_semantic_edge_rule() {
    for mode in ["wrong-resource-membership", "wrong-conversion-membership"] {
        let w = negative(mode);
        assert_eq!(
            joined::verify(&w, &ops(), &profile()).err(),
            Some("selected edge membership"),
            "{mode}"
        );
    }
}
#[test]
fn valid_alternate_legacy_snapshot_cannot_replace_original_selected_bootstrap() {
    let w = negative("alternate-source-bootstrap");
    let memory = photara_store::package::MemoryPackage::new(
        resource::legacy_internal(&w.source),
        photara_store::package::PackageLimits::default(),
    )
    .unwrap();
    photara_store::package::v1_1::validate_memory(&memory).unwrap();
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("retained source is original selected bootstrap")
    );
}
#[test]
fn unchanged_outer_commit_bootstrap_and_allocation_arena_are_checked() {
    let mut w = setup();
    w.commit["bootstrap_sha256"] = json!("e".repeat(64));
    w.rehash_head();
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("outer commit dispatch")
    );
    let mut w = setup();
    w.commit.as_object_mut().unwrap().remove("bootstrap_sha256");
    w.rehash_head();
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("exact fields")
    );
    let mut w = setup();
    w.commit["created_at"] = json!("not a timestamp");
    w.rehash_head();
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("commit timestamp")
    );
    let mut c = corpus();
    c["allocations"]["80000000-0000-4000-8000-000000000001"]["arena"] = json!("unknown");
    assert_eq!(World::load(&c).err(), Some("allocation arena dispatch"));
}
#[test]
fn actual_unchanged_legacy_reader_refuses_joined_commit_before_packed_reads() {
    let w = setup();
    let mut files = resource::legacy_internal(&w.source);
    files.insert("HEAD.json".into(), trees::encode(&w.head));
    files.insert(
        format!("commits/{}.json", w.commit["commit_id"].as_str().unwrap()),
        trees::encode(&w.commit),
    );
    let memory = photara_store::package::MemoryPackage::new(
        files,
        photara_store::package::PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        photara_store::package::v1_1::validate_memory(&memory).err(),
        Some(photara_store::package::PackageError::UnsupportedFeature)
    );
}
fn rewrite_accounting(w: &mut World, edit: impl FnOnce(&mut Value)) {
    let original = w.commit["root_set"]["placement"]["accounting"].clone();
    let mut accounting: Value =
        serde_json::from_slice(&w.loose[original["sha256"].as_str().unwrap()]).unwrap();
    edit(&mut accounting);
    let bytes = trees::encode(&accounting);
    let r = trees::reference(&accounting);
    w.loose.remove(original["sha256"].as_str().unwrap());
    w.loose.insert(r["sha256"].as_str().unwrap().into(), bytes);
    w.commit["root_set"]["placement"]["accounting"] = r;
    w.rehash_head();
}
#[test]
fn sealed_and_growable_replacements_cannot_inherit_original_registered_charge() {
    for (allocation, error) in [
        (
            "80000000-0000-4000-8000-000000000001",
            "original sealed allocation witness",
        ),
        (
            "80000000-0000-4000-8000-000000000004",
            "original tip witness/extent",
        ),
    ] {
        let mut w = setup();
        w.allocations.get_mut(allocation).unwrap().witness["inode"] = json!("999");
        assert_eq!(joined::verify(&w, &ops(), &profile()).err(), Some(error));
    }
    let mut w = setup();
    rewrite_accounting(&mut w, |a| {
        a["sealed_observations"][0]["observation_id"] =
            json!("81000000-0000-4000-8000-000000000099");
    });
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("unregistered sealed observation")
    );
}
#[test]
fn coherently_selected_undercharge_and_conversion_refund_refuse_before_inventory_checks() {
    let mut w = setup();
    rewrite_accounting(&mut w, |a| {
        a["tips"][0]["extent"] = json!("126976");
        a["tips"][0]["registered_charge"] = json!("126976");
        a["total_charge"] = json!("757760");
    });
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("original tip witness/extent")
    );
    let mut w = setup();
    rewrite_accounting(&mut w, |a| {
        a["retained_conversion"]["files"][0]["registered_charge"] = json!("0");
        a["retained_conversion"]["registered_charge"] = json!(
            (resource::number(&a["retained_conversion"]["registered_charge"]).unwrap() - 4096)
                .to_string()
        );
        a["total_charge"] = json!("757760");
    });
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("per-path retained conversion charge")
    );
}
#[test]
fn resource_metadata_remains_readable_when_qualification_is_unconfirmed() {
    let w = setup();
    let mut p = profile();
    p["failure_model_sha256"] = json!("e".repeat(64));
    let proof = joined::verify_role(&w, &ops(), &p, "active").unwrap();
    assert!(!proof.resource_supported);
    joined::verify(&w, &ops(), &p).unwrap();
}

#[test]
fn identical_conversion_file_replacement_retains_no_old_charge_authority() {
    let mut w = setup();
    let path = w.source.keys().next().unwrap().clone();
    w.source_witnesses.get_mut(&path).unwrap()["inode"] = json!("9999");
    assert_eq!(
        joined::verify(&w, &ops(), &profile()).err(),
        Some("original retained file witness")
    );
}
