//! Unfrozen HEAD-selected origin evidence. No pending codec or release authority.
#[allow(
    dead_code,
    reason = "Reuse settled accounting proof entry points unchanged"
)]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "selected_origin_candidate/origins.rs"]
mod origins;
#[allow(
    dead_code,
    reason = "Reuse direct resource metadata proof entry points"
)]
#[path = "integrated_wire_candidate/resources.rs"]
mod resources;
#[allow(
    dead_code,
    reason = "Reuse selected physical and global closure implementation"
)]
#[path = "integrated_wire_candidate/wire.rs"]
mod wire;
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/origins/linked.json"
    ))
    .unwrap()
}
fn world(role: &str) -> wire::World {
    wire::World::load(&corpus()["worlds"][role]).unwrap()
}
#[test]
fn actual_head_selects_four_origins_and_retains_exact_receipts_and_snapshot() {
    for label in ["selected", "rotated"] {
        let w = world(label);
        let p = wire::verify_with(&w, &origins::Origins).unwrap();
        assert_eq!(p.total, 2_076_672);
        assert_eq!(p.roles.len(), 3);
        assert_eq!(p.roles["active"].receipts.len(), 3);
        assert!(p.roles.values().all(|r| r.supported));
    }
    let initial = world("selected");
    let rotated = world("rotated");
    let p = wire::verify_with(&initial, &origins::Origins).unwrap();
    let r = wire::verify_with(&rotated, &origins::Origins).unwrap();
    assert_eq!(
        p.roles["recovery"].state,
        r.roles["pin:96000000-0000-4000-8000-000000005001"].state
    );
    assert_eq!(initial.source, rotated.source);
    assert_eq!(p.roles["active"].receipts, r.roles["active"].receipts);
    assert_eq!(r.roles["recovery"].receipts.len(), 1);
}
#[test]
fn independent_recovery_uses_own_subset_of_shared_policy_without_foreign_payloads() {
    for (label, remove) in [
        ("selected", [1002, 1003, 1006, 1007]),
        ("rotated", [1002, 1003, 1004, 1005]),
    ] {
        let mut w = world(label);
        for n in remove {
            w.allocations
                .remove(&format!("96000000-0000-4000-8000-{n:012}"));
        }
        let p = wire::recovery_with(&w, &origins::Origins).unwrap();
        assert!(p.supported);
        assert_eq!(p.receipts.len(), if label == "selected" { 2 } else { 1 });
        assert!(wire::verify_with(&w, &origins::Origins).is_err());
    }
}
fn negative(name: &str) -> wire::World {
    let variants: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/origins/negatives.json"
    ))
    .unwrap();
    let variant = &variants[name];
    let mut c = corpus()["worlds"][variant["base"].as_str().unwrap()].clone();
    let delta = &variant["delta"];
    for key in ["bootstrap", "loose", "source_files", "source_witnesses"] {
        c[key] = delta[key].clone();
    }
    for (id, changes) in delta["allocations"].as_object().unwrap() {
        let mut b = wire::unhex(c["allocations"][id]["hex"].as_str().unwrap()).unwrap();
        for patch in changes["patches"].as_array().unwrap() {
            let at = usize::try_from(patch["offset"].as_u64().unwrap()).unwrap();
            let bytes = wire::unhex(patch["hex"].as_str().unwrap()).unwrap();
            b[at..at + bytes.len()].copy_from_slice(&bytes);
        }
        let text = b.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write;
            write!(s, "{b:02x}").unwrap();
            s
        });
        c["allocations"][id]["hex"] = json!(text);
    }
    wire::World::load(&c).unwrap()
}
#[test]
fn coherent_origin_metadata_cannot_substitute_selected_authority() {
    for (name, error) in [
        ("wrong-policy-id", "origin evidence key identity"),
        ("wrong-policy-subset", "origin exact selected subset union"),
        ("noncanonical-policy-revision", "canonical decimal"),
        ("unknown-policy-field", "exact fields"),
        ("wrong-history-root", "origin context root identity"),
        ("wrong-history-projection", "origin exact history promise"),
        ("missing-history-context", "origin unresolved evidence key"),
        ("wrong-recovery-pin", "origin retained recovery role"),
        ("duplicate-evidence-key", "origin strict tree order"),
        ("wrong-evidence-count", "origin tree count"),
        ("unknown-evidence-version", "schema/project dispatch"),
        (
            "pending-unsupported",
            "pending/unknown resource origin unsupported",
        ),
        ("wrong-owning-root", "origin owning root identity"),
        (
            "conflicting-association",
            "immutable association identity across roots",
        ),
        (
            "conflicting-requirement",
            "origin shared requirement identity",
        ),
        ("missing-capability", "required capability/floor dispatch"),
        (
            "missing-shared-ownership",
            "exact per-root owned allocations",
        ),
        ("missing-policy-body", "missing located object"),
        ("wrong-rotation-pin-reason", "origin retained recovery role"),
    ] {
        assert_eq!(
            wire::verify_with(&negative(name), &origins::Origins).err(),
            Some(error),
            "{name}"
        );
    }
}

#[test]
fn unchanged_legacy_reader_refuses_before_new_storage_dependencies() {
    let w = world("selected");
    let mut files = w
        .source
        .iter()
        .filter(|(p, _)| {
            p.as_str() == "HEAD.json"
                || p.as_str() == "manifest.json"
                || p.starts_with("commits/")
                || p.starts_with("objects/")
        })
        .map(|(p, b)| (p.clone(), b.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    files.insert("HEAD.json".into(), wire::encode(&w.head));
    files.insert(
        format!("commits/{}.json", w.commit["commit_id"].as_str().unwrap()),
        wire::encode(&w.commit),
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

#[test]
fn canonical_records_preserve_original_receipts_and_source_bytes() {
    let c = corpus();
    let original: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/integrated/linked.json"
    ))
    .unwrap();
    for role in ["selected", "rotated"] {
        let specimen = &c["worlds"][role];
        assert_eq!(specimen["source_files"], original["source_files"]);
        for (digest, record) in specimen["records"].as_object().unwrap() {
            let bytes = record["canonical"].as_str().unwrap().as_bytes();
            let value: Value = serde_json::from_slice(bytes).unwrap();
            assert_eq!(wire::encode(&value), bytes);
            assert_eq!(wire::hash(bytes), *digest);
            assert_eq!(record["sha256"], *digest);
            assert_eq!(
                record["byte_length"].as_u64().unwrap(),
                u64::try_from(bytes.len()).unwrap()
            );
        }
        let mut preserved = 0;
        for (digest, record) in original["records"].as_object().unwrap() {
            let value: Value = serde_json::from_str(record["canonical"].as_str().unwrap()).unwrap();
            if matches!(
                value["schema"]["id"].as_str(),
                Some("photara.package.operation-receipt" | "photara.package.conversion-source")
            ) {
                assert_eq!(specimen["records"][digest], *record);
                preserved += 1;
            }
        }
        assert_eq!(preserved, 4);
        let records = specimen["records"].as_object().unwrap();
        let values = records
            .values()
            .map(|r| serde_json::from_str::<Value>(r["canonical"].as_str().unwrap()).unwrap())
            .collect::<Vec<_>>();
        let policy = values
            .iter()
            .find(|v| v["schema"]["id"] == "photara.resource.retention-policy")
            .unwrap();
        let recovery_source = values
            .iter()
            .find(|v| {
                v["schema"]["id"] == "photara.resource.retention-source"
                    && v["association_id"] == "96000000-0000-4000-8000-000000010021"
            })
            .unwrap();
        let count = |r: &Value| {
            let v: Value = serde_json::from_str(
                records[r["sha256"].as_str().unwrap()]["canonical"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            wire::number(&v["count"]).unwrap()
        };
        assert_eq!(count(&policy["requirements"]), 8);
        assert_eq!(count(&recovery_source["requirements"]), 4);
    }
}
