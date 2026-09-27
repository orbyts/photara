//! Proposed-coordinate settled package byte specimen. Unfrozen; no production reader.
#![allow(dead_code)]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "integrated_wire_candidate/resources.rs"]
mod resources;
#[path = "integrated_wire_candidate/wire.rs"]
mod wire;

fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/integrated/linked.json"
    ))
    .unwrap()
}
#[test]
fn actual_head_selects_three_independent_roots_and_exact_accounting() {
    let c = corpus();
    let w = wire::World::load(&c).unwrap();
    let p = wire::verify(&w).unwrap();
    assert_eq!(p.total.to_string(), c["expected"]["total_charge"]);
    assert_eq!(p.roles.len(), 3);
    assert_eq!(p.roles["active"].receipts.len(), 3);
    assert_eq!(p.roles["recovery"].receipts.len(), 2);
    assert!(p.roles.values().all(|p| p.supported));
}
fn variant(name: &str) -> wire::World {
    let mut c = corpus();
    let variants: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/integrated/negatives.json"
    ))
    .unwrap();
    let delta = &variants[name];
    for k in ["bootstrap", "loose", "source_files", "source_witnesses"] {
        c[k] = delta[k].clone();
    }
    for (id, changes) in delta["allocations"].as_object().unwrap() {
        let mut bytes = wire::unhex(c["allocations"][id]["hex"].as_str().unwrap()).unwrap();
        for patch in changes["patches"].as_array().unwrap() {
            let offset = usize::try_from(patch["offset"].as_u64().unwrap()).unwrap();
            let data = wire::unhex(patch["hex"].as_str().unwrap()).unwrap();
            bytes[offset..offset + data.len()].copy_from_slice(&data);
        }
        c["allocations"][id]["hex"] =
            serde_json::json!(bytes.iter().fold(String::new(), |mut s, b| {
                use std::fmt::Write;
                write!(s, "{b:02x}").unwrap();
                s
            }));
    }
    wire::World::load(&c).unwrap()
}
#[test]
fn coherent_wrong_selectors_ownership_membership_and_charges_refuse() {
    for (name, error) in [
        ("missing-global-member", "exact global inventory union"),
        (
            "missing-shared-ownership",
            "exact per-root owned allocations",
        ),
        ("borrowed-pin-placement", "missing located object"),
        ("undercharge", "accounting exact aggregate settlement"),
        (
            "conflicting-association-id",
            "immutable association identity across roots",
        ),
        ("wrong-resource-membership", "typed membership"),
        (
            "wrong-history-edge",
            "StateRoot authored/history typed edges",
        ),
        ("missing-capability", "required capability/floor dispatch"),
        ("invented-origin", "resolved selected source association"),
        ("missing-resource-locator", "missing located object"),
        (
            "changed-original-conversion",
            "accounting unchanged original ConversionSource bytes",
        ),
        ("duplicate-root-id", "unique retained root UUID"),
        ("sealed-as-tip", "selected growable claim classification"),
        (
            "alternate-sealed-claim",
            "exact selected sealed charge reference",
        ),
        (
            "unknown-legacy-history-version",
            "StateRoot authored/history typed edges",
        ),
    ] {
        let got = wire::verify(&variant(name)).err();
        assert_eq!(got, Some(error), "{name}: {got:?}");
    }
}
#[test]
fn original_semantic_receipt_conversion_bytes_and_fixed_vectors_are_preserved() {
    let c = corpus();
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/operations/linked-operations.json"
    ))
    .unwrap();
    for row in c["records"].as_object().unwrap().values() {
        let v = wire::parse(row["canonical"].as_str().unwrap().as_bytes()).unwrap();
        assert_eq!(wire::reference(&v)["sha256"], row["sha256"]);
        assert_eq!(
            wire::reference(&v)["byte_length"],
            row["byte_length"].as_u64().unwrap().to_string()
        );
    }
    let w = wire::World::load(&c).unwrap();
    let proof = wire::verify(&w).unwrap();
    for (i, r) in proof.roles["active"].receipts.iter().enumerate() {
        assert_eq!(
            wire::encode(r),
            original["records"][format!("receipt-{}", i + 1)]["canonical"]
                .as_str()
                .unwrap()
                .as_bytes()
        );
    }
    assert_eq!(
        wire::encode(&w.manifest),
        original["records"]["manifest"]["canonical"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    let conversion: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/resource-conversion/linked.json"
    ))
    .unwrap();
    assert_eq!(
        c["source_files"],
        conversion["scenarios"]["valid"]["source_files"]
    );
    let a = &proof.roles["active"].receipts;
    assert_eq!(a[1]["before"], a[1]["after"]);
    assert_ne!(a[0]["acceptance_ordinal"], a[0]["journal_sequence"]);
    assert_eq!(
        a[0]["provenance"],
        original["records"]["receipt-1"]["input"]["provenance"]
    );
}
#[test]
fn bounded_parser_and_optional_extension_nonedge_dispatch() {
    assert_eq!(
        wire::parse(br#"{"a":1,"a":2}"#).unwrap_err(),
        "bounded canonical JSON"
    );
    assert_eq!(
        wire::parse(br#"{ "a":1}"#).unwrap_err(),
        "bounded canonical JSON"
    );
    let mut w = wire::World::load(&corpus()).unwrap();
    w.commit["root_set"]["extensions"] = serde_json::json!({"example.opaque":{"kind":"json","sha256":"f".repeat(64),"byte_length":"1"}});
    w.rehash_head();
    wire::verify(&w).unwrap();
    w.commit["root_set"]["unknown_required"] = serde_json::json!(true);
    w.rehash_head();
    assert_eq!(wire::verify(&w).err(), Some("exact fields"));
    let mut c = corpus();
    c["loose"].as_object_mut().unwrap().insert(
        "f".repeat(64),
        serde_json::json!("x".repeat(1024 * 1024 + 1)),
    );
    assert_eq!(wire::World::load(&c).err().unwrap(), "loose input bound");
    let mut c = corpus();
    c["source_files"]["large"] = serde_json::json!("00".repeat(16 * 1024 * 1024));
    assert_eq!(
        wire::World::load(&c).err().unwrap(),
        "source aggregate bound"
    );
}
#[test]
fn supplied_bytes_and_original_local_identity_are_independent_checks() {
    let original = wire::World::load(&corpus()).unwrap();
    for suffix in [1001, 1002, 1008] {
        let mut w = original.clone();
        let id = format!("96000000-0000-4000-8000-{suffix:012}");
        w.allocations.get_mut(&id).unwrap().witness["inode"] = serde_json::json!("999999");
        assert_eq!(
            wire::verify(&w).err(),
            Some("accounting exact original allocation evidence")
        );
    }
    let mut w = original.clone();
    w.allocations
        .get_mut("96000000-0000-4000-8000-000000001001")
        .unwrap()
        .bytes[17] ^= 1;
    assert_eq!(wire::verify(&w).err(), Some("physical frame hash"));
    let mut w = original;
    w.loose.insert("f".repeat(64), b"{}".to_vec());
    assert_eq!(wire::verify(&w).err(), Some("exact loose control keep-set"));
}

#[test]
fn bounded_fixture_entry_and_same_id_original_result() {
    let raw = include_bytes!("../../../docs/architecture/proposals/ps2/integrated/linked.json");
    let w = wire::World::load_bytes(raw).unwrap();
    let p = wire::verify(&w).unwrap();
    let original = &p.roles["active"].receipts[0];
    assert_eq!(
        &wire::receipt_for(
            &p,
            original["operation_id"].as_str().unwrap(),
            original["request_sha256"].as_str().unwrap()
        )
        .unwrap(),
        original
    );
    assert_eq!(
        wire::receipt_for(
            &p,
            original["operation_id"].as_str().unwrap(),
            &"f".repeat(64)
        )
        .err(),
        Some("same-ID request conflict")
    );
    let oversized = vec![b' '; 8 * 1024 * 1024 + 1];
    assert_eq!(
        wire::World::load_bytes(&oversized).err().unwrap(),
        "bounded fixture input"
    );
}

#[test]
fn unchanged_legacy_reader_refuses_before_new_storage_dependencies() {
    let w = wire::World::load(&corpus()).unwrap();
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
fn recovery_reads_its_own_selected_closure_without_other_roles() {
    let mut w = wire::World::load(&corpus()).unwrap();
    for n in [1002, 1003, 1006, 1007] {
        w.allocations
            .remove(&format!("96000000-0000-4000-8000-{n:012}"));
    }
    assert_eq!(wire::recovery(&w).unwrap().receipts.len(), 2);
    assert!(wire::verify(&w).is_err());
    let mut changed = w.clone();
    changed
        .allocations
        .get_mut("96000000-0000-4000-8000-000000001004")
        .unwrap()
        .witness["inode"] = serde_json::json!("99999");
    assert_eq!(
        wire::recovery(&changed).err().unwrap(),
        "supplied owned allocation original witness"
    );
    let mut changed = w;
    let r = changed.commit["root_set"]["inventory"].clone();
    let mut overlay = changed.loose(&r).unwrap();
    overlay["controls"] = serde_json::json!([]);
    let rr = wire::reference(&overlay);
    changed.loose.remove(r["sha256"].as_str().unwrap());
    changed.loose.insert(
        rr["sha256"].as_str().unwrap().into(),
        wire::encode(&overlay),
    );
    changed.commit["root_set"]["inventory"] = rr;
    changed.rehash_head();
    assert_eq!(
        wire::recovery(&changed).err().unwrap(),
        "exact overlay controls"
    );
}
