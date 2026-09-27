//! Linked metadata candidate only. No native package or media effects.
#[path = "resource_conversion_candidate/wire.rs"]
mod wire;
use photara_core::canonical_json;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Intent, Result, Store, hash, reference, verify_resources};
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/resource-conversion/linked.json"
    ))
    .unwrap()
}
fn load() -> Store {
    Store::load(&corpus()["scenarios"]["valid"]).unwrap()
}
fn replace(s: &mut Store, v: &Value) -> Value {
    let b = canonical_json(v).unwrap();
    let r = json!({"kind":"json","sha256":hash(&b),"byte_length":b.len().to_string()});
    s.records.insert(hash(&b), b);
    r
}
#[test]
fn independent_canonical_bytes_hashes_and_lengths_are_exact() -> Result<()> {
    let corpus = corpus();
    assert_eq!(corpus["base_file_count"], 15);
    for scenario in corpus["scenarios"].as_object().unwrap().values() {
        for (sha, v) in scenario["records"].as_object().unwrap() {
            let b = canonical_json(&v["input"]).unwrap();
            assert_eq!(b, v["canonical"].as_str().unwrap().as_bytes());
            assert_eq!(hash(&b), *sha);
            assert_eq!(v["sha256"], *sha);
            assert_eq!(v["byte_length"], b.len());
            assert_eq!(wire::parse(&b)?, v["input"]);
        }
    }
    Ok(())
}
#[test]
fn independent_resource_closures_keep_versions_immutable_across_relocation() -> Result<()> {
    let s = load();
    let a = verify_resources(&s, "active")?;
    let r = verify_resources(&s, "recovery")?;
    assert_ne!(a, r);
    let common = a.intersection(&r).cloned().collect::<BTreeSet<_>>();
    let mut version_count = 0;
    for (h, _) in &common {
        let v = wire::parse(&s.records[h])?;
        if v["schema"]["id"] == "photara.resource.captured-version" {
            version_count += 1;
            assert!(v.get("backing").is_none());
            assert!(v.get("retention").is_none());
        }
    }
    assert_eq!(version_count, 1);
    for (role, own, other) in [("active", &a, &r), ("recovery", &r, &a)] {
        let mut isolated = s.clone();
        for (h, _) in other.difference(own) {
            isolated.records.remove(h);
        }
        assert_eq!(verify_resources(&isolated, role)?, *own);
        assert!(
            verify_resources(
                &isolated,
                if role == "active" {
                    "recovery"
                } else {
                    "active"
                }
            )
            .is_err()
        );
    }
    // Both scopes traverse small metadata only despite an 8 GB captured length.
    assert!(a.iter().map(|(_, n)| n).sum::<u64>() < 16 * 1024);
    Ok(())
}
#[test]
fn linked_rehashed_negatives_reach_semantic_checks() -> Result<()> {
    let c = corpus();
    let cases = [
        (
            "capture-qualified",
            "capture has no retention qualification",
        ),
        ("version-policy-pointer", "exact fields"),
        ("binding-version", "binding selected version"),
        ("binding-length", "binding media/fingerprint"),
        ("binding-digest", "binding media/fingerprint"),
        ("binding-v2", "schema/project"),
        (
            "capture-as-publication",
            "backing needs retained publication",
        ),
        ("wrong-profile", "fixture obligation unsupported"),
        ("old-profile", "fixture obligation unsupported"),
        ("wrong-failure-model", "fixture obligation unsupported"),
        ("wrong-destination", "publication destination"),
        ("backing-version", "backing selected version"),
        ("missing-pin", "resolved typed pin source"),
        ("unknown-pin", "resolved typed pin source"),
        ("zero-copies", "positive obligation copies"),
        ("two-copies", "fixture obligation unsupported"),
        ("missing-required-backing", "required backing selected"),
        ("selection-id", "selected target ID"),
        ("duplicate-selection", "selection order/unique"),
        ("duplicate-working", "one working binding per resource"),
        ("host-observation", "exact fields"),
        ("unsafe-coordinate", "portable coordinate"),
        ("missing-feature", "resource feature before lookup"),
        ("surplus-inventory", "exact resource inventory"),
        ("missing-inventory", "exact resource inventory"),
    ];
    for (name, e) in cases {
        let s = Store::load(&c["scenarios"][name])?;
        assert_eq!(verify_resources(&s, "active").err(), Some(e), "{name}");
    }
    Ok(())
}
#[test]
fn extension_refs_are_opaque_and_missing_feature_precedes_lookup() -> Result<()> {
    let s = load();
    let root = s.root("active")?;
    let state = s.get(&root["resource_state"], &mut BTreeSet::new())?;
    let original = canonical_json(&state["extensions"]).unwrap();
    let closure = verify_resources(&s, "active")?;
    assert!(!closure.iter().any(|(h, _)| h == &"0".repeat(64)));
    assert_eq!(
        canonical_json(&s.get(&root["resource_state"], &mut BTreeSet::new())?["extensions"])
            .unwrap(),
        original
    );
    let c = corpus();
    let mut bad = Store::load(&c["scenarios"]["missing-feature"])?;
    let keep = reference(&bad.roots["active"])?.0;
    bad.records.retain(|k, _| *k == keep);
    assert_eq!(
        verify_resources(&bad, "active").err(),
        Some("resource feature before lookup")
    );
    Ok(())
}
#[test]
fn every_selected_resource_dependency_is_required_and_canonical() -> Result<()> {
    let s = load();
    for (key, _) in verify_resources(&s, "active")? {
        let mut missing = s.clone();
        missing.records.remove(&key);
        assert_eq!(
            verify_resources(&missing, "active").err(),
            Some("missing object")
        );
        let mut corrupt = s.clone();
        corrupt.records.get_mut(&key).unwrap().push(b' ');
        assert_eq!(
            verify_resources(&corrupt, "active").err(),
            Some("referenced bytes")
        );
    }
    for b in [
        b"{\"x\":1,\"x\":1}".as_slice(),
        b"{ \"x\":1}",
        b"{\"x\":1}\n",
        b"\xff",
    ] {
        assert!(wire::parse(b).is_err());
    }
    Ok(())
}
#[test]
fn conversion_preserves_actual_legacy_reader_and_unknown_bytes_at_every_copy_cut() -> Result<()> {
    let s = load();
    let intent = Intent::new(&s)?;
    intent.verify(&s)?;
    assert_eq!(s.source.len(), 18);
    let base: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/operations/linked-operations.json"
    ))
    .unwrap();
    for (p, b) in base["base_files"].as_object().unwrap() {
        assert_eq!(s.source[p], b.as_str().unwrap().as_bytes());
    }
    let original = s.source.clone();
    for cut in 0..=original.len() {
        let partial = original
            .iter()
            .take(cut)
            .map(|(p, b)| (p.clone(), b.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut current = original.clone();
        current.insert(intent.stable_lock.clone(), b"registered lock".to_vec());
        current.insert(intent.staging.clone(), b"same immutable attempt".to_vec());
        for (p, b) in &partial {
            current.insert(format!("{}/{p}", intent.namespace), b.clone());
        }
        let resumed = intent.resume(&s, &current, &partial, Some(&intent.registration()))?;
        assert_eq!(resumed, original);
        assert_eq!(s.source, original);
        let reopened = photara_store::package::MemoryPackage::new(
            wire::legacy_internal(&resumed),
            photara_store::package::PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(
            photara_store::package::v1_1::validate_memory(&reopened)
                .unwrap()
                .commits
                .len(),
            1
        );
    }
    assert_eq!(
        s.source["optional/opaque.bin"],
        b"\x00\xffunknown optional bytes\n"
    );
    Ok(())
}
#[test]
fn conversion_refuses_mismatch_occupancy_source_change_and_broad_exclusion_without_effects()
-> Result<()> {
    let s = load();
    let intent = Intent::new(&s)?;
    for mode in 0..7 {
        let mut current = s.source.clone();
        let mut partial = BTreeMap::new();
        match mode {
            0 => {
                partial.insert("HEAD.json".into(), vec![0]);
                current.insert(format!("{}/HEAD.json", intent.namespace), vec![0]);
            }
            1 => {
                partial.insert("unknown".into(), vec![]);
                current.insert(format!("{}/unknown", intent.namespace), vec![]);
            }
            2 => {
                current.insert("staging/other-attempt/new".into(), vec![]);
            }
            3 => {
                current.insert("conversion-sources/other/package/new".into(), vec![]);
            }
            4 => {
                current.get_mut("optional/opaque.bin").unwrap().push(1);
            }
            5 => {
                current.remove("staging/other-attempt/keep.bin");
            }
            _ => {
                current.insert(format!("{}/unreported", intent.namespace), vec![]);
            }
        }
        let before = (current.clone(), partial.clone());
        assert!(
            intent
                .resume(&s, &current, &partial, Some(&intent.registration()))
                .is_err(),
            "mode{mode}"
        );
        assert_eq!((current, partial), before);
    }
    let mut replacement = s.clone();
    let mut d = intent.descriptor.clone();
    d["conversion_id"] = json!("71000000-0000-4000-8000-000000009999");
    replacement.conversion = replace(&mut replacement, &d);
    assert_eq!(
        intent.verify(&replacement).err(),
        Some("immutable conversion intent")
    );
    Ok(())
}
#[test]
fn conversion_manifest_semantics_are_checked_beyond_hashes() -> Result<()> {
    let s = load();
    for mode in 0..9 {
        let mut bad = s.clone();
        let mut d = Intent::new(&s)?.descriptor;
        match mode {
            0 => d["files"]
                .as_array_mut()
                .unwrap()
                .pop()
                .map(|_| ())
                .unwrap(),
            1 => {
                d["files"][0]["components"] = json!(["..", "HEAD.json"]);
            }
            2 => {
                let duplicate = d["files"][0].clone();
                d["files"].as_array_mut().unwrap().insert(1, duplicate);
            }
            3 => d["files"][0]["byte_length"] = json!("01"),
            4 => d["source_head_sha256"] = json!("0".repeat(64)),
            5 => d["source_bootstrap_sha256"] = json!("0".repeat(64)),
            6 => d["snapshot_directory"] = json!(["conversion-sources", "wrong", "package"]),
            7 => d["source_format_version"]["minor"] = json!(2),
            _ => d["source_commit_id"] = json!("71000000-0000-4000-8000-000000009999"),
        }
        bad.conversion = replace(&mut bad, &d);
        let intent = Intent::new(&bad)?;
        assert!(intent.verify(&bad).is_err(), "mode{mode}");
    }
    Ok(())
}

#[test]
fn structurally_valid_unsatisfied_obligations_remain_readable() -> Result<()> {
    let c = corpus();
    for name in [
        "two-copies",
        "wrong-profile",
        "old-profile",
        "wrong-failure-model",
    ] {
        let s = Store::load(&c["scenarios"][name])?;
        let (_, supported) = wire::verify_resources_structural(&s, "active")?;
        assert!(!supported, "{name}");
    }
    let s = load();
    let root = s.root("active")?;
    let (state, supported) =
        wire::verify_selected_state(&s, &root["resource_state"], &root["root_id"])?;
    assert!(supported);
    let projection = verify_resources(&s, "active")?;
    assert_eq!(projection.difference(&state).count(), 1);
    assert!(
        wire::verify_selected_state(
            &s,
            &root["resource_state"],
            &json!("71000000-0000-4000-8000-000000009999")
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn captured_binding_does_not_enter_existing_context_value_codec() -> Result<()> {
    let s = load();
    let root = s.root("active")?;
    let rep = s.get(&root["representation"], &mut BTreeSet::new())?;
    let candidate = json!({"descriptor":rep["binding"],"components":[]});
    assert!(
        serde_json::from_value::<photara_core::context::value::ResourceValue>(candidate).is_err()
    );
    Ok(())
}

#[test]
fn byte_identical_snapshot_requires_original_attempt_registration() -> Result<()> {
    let s = load();
    let intent = Intent::new(&s)?;
    let partial = s.source.clone();
    let mut current = s.source.clone();
    for (p, b) in &partial {
        current.insert(format!("{}/{p}", intent.namespace), b.clone());
    }
    let before = current.clone();
    assert_eq!(
        intent.resume(&s, &current, &partial, None).err(),
        Some("same-attempt snapshot registration")
    );
    for mode in 0..3 {
        let mut registration = intent.registration();
        match mode {
            0 => registration.conversion_id = json!("71000000-0000-4000-8000-000000009999"),
            1 => registration.descriptor_sha256 = "0".repeat(64),
            _ => registration.namespace = "conversion-sources/foreign/package".into(),
        }
        assert_eq!(
            intent
                .resume(&s, &current, &partial, Some(&registration))
                .err(),
            Some("same-attempt snapshot registration")
        );
        assert_eq!(current, before);
    }
    assert_eq!(
        intent.resume(&s, &current, &partial, Some(&intent.registration()))?,
        partial
    );
    Ok(())
}

#[test]
fn unrepresentable_original_components_refuse_without_rewriting() -> Result<()> {
    let s = load();
    for component in [
        "CON",
        "trailing.",
        "trailing ",
        "bad\nname",
        "bad?name",
        "..",
        "",
    ] {
        let mut bad = s.clone();
        let mut descriptor = Intent::new(&s)?.descriptor;
        descriptor["files"][0]["components"] = json!([component]);
        bad.conversion = replace(&mut bad, &descriptor);
        let intent = Intent::new(&bad)?;
        let before = bad.source.clone();
        assert_eq!(
            intent.verify(&bad).err(),
            Some("portable path"),
            "{component:?}"
        );
        assert_eq!(bad.source, before);
        descriptor["snapshot_directory"] = json!(["conversion-sources", component, "package"]);
        bad.conversion = replace(&mut bad, &descriptor);
        assert_eq!(Intent::new(&bad).err(), Some("portable path"));
    }
    Ok(())
}
