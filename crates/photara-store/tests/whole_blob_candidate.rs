//! Whole-Blob storage candidate; actual legacy semantics, no production reader.
#[path = "whole_blob_candidate/wire.rs"]
mod wire;
use photara_store::package::{self, MemoryPackage, ObjectKind, ObjectRef, PackageLimits};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/blob/linked.json"
    ))
    .unwrap()
}
fn store() -> wire::Store {
    wire::Store::load(&corpus()).unwrap()
}
fn replace(v: &mut Value, old: &Value, new: &Value) {
    if v == old {
        *v = new.clone();
        return;
    }
    match v {
        Value::Object(m) => {
            for value in m.values_mut() {
                replace(value, old, new);
            }
        }
        Value::Array(a) => {
            for value in a {
                replace(value, old, new);
            }
        }
        _ => {}
    }
}
fn edit(s: &mut wire::Store, schema: &str, change: impl FnOnce(&mut Value)) {
    let (key, raw) = s
        .records
        .iter()
        .find(|(_, b)| serde_json::from_slice::<Value>(b).unwrap()["schema"]["id"] == schema)
        .unwrap();
    let key = key.clone();
    let old: Value = serde_json::from_slice(raw).unwrap();
    let mut v = old.clone();
    change(&mut v);
    s.records.remove(&key);
    s.records
        .insert(wire::hash(&wire::bytes(&v)), wire::bytes(&v));
    let mut mappings = vec![(wire::reference(&old), wire::reference(&v))];
    loop {
        let mut changed = false;
        let old = s.selection.clone();
        for (a, b) in &mappings {
            replace(&mut s.selection, a, b);
        }
        changed |= s.selection != old;
        let entries = s.records.clone();
        let mut next = vec![];
        for (key, raw) in entries {
            let old: Value = serde_json::from_slice(&raw).unwrap();
            let mut v = old.clone();
            for (a, b) in &mappings {
                replace(&mut v, a, b);
            }
            if v != old {
                changed = true;
                s.records.remove(&key);
                s.records
                    .insert(wire::hash(&wire::bytes(&v)), wire::bytes(&v));
                next.push((wire::reference(&old), wire::reference(&v)));
            }
        }
        mappings.extend(next);
        if !changed {
            break;
        }
    }
}
#[test]
fn actual_legacy_managed_blob_and_representation_bytes_are_unchanged() {
    let c = corpus();
    let golden = include_bytes!("../../../docs/fixtures/generation-two/d19-package-specimen.json");
    assert_eq!(c["original_package_sha256"], wire::hash(golden));
    let archive: Value = serde_json::from_slice(golden).unwrap();
    for row in c["records"].as_object().unwrap().values() {
        let canonical = wire::bytes(&row["input"]);
        assert_eq!(canonical, row["canonical"].as_str().unwrap().as_bytes());
        assert_eq!(wire::hash(&canonical), row["sha256"]);
        assert_eq!(canonical.len() as u64, row["byte_length"].as_u64().unwrap());
    }
    let files = c["original_files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(p, v)| (p.clone(), wire::unhex(v.as_str().unwrap()).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let expected = archive["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["path"].as_str().unwrap().to_owned(),
                r["utf8"].as_str().unwrap().as_bytes().to_vec(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(files, expected);
    let memory = MemoryPackage::new(files.clone(), PackageLimits::default()).unwrap();
    let validated = package::v1_1::validate_memory(&memory).unwrap();
    assert_eq!(validated.verified_blobs, 1);
    let s = store();
    for raw in s.records.values() {
        let v: Value = serde_json::from_slice(raw).unwrap();
        if matches!(
            v["schema"]["id"].as_str(),
            Some("photara.project.managed-resource" | "photara.project.representation-content")
        ) {
            let r: ObjectRef = serde_json::from_value(wire::reference(&v)).unwrap();
            assert_eq!(files.get(&r.path()), Some(raw));
        }
    }
}
#[test]
fn whole_blob_and_empty_blob_have_exact_ownership_charge_and_explicit_audit() {
    let s = store();
    let structural = wire::structural(&s).unwrap();
    assert_eq!(structural.blobs.len(), 2);
    assert_eq!(structural.raw_blob_bytes, 5);
    assert_eq!(structural.total_charge, 81_920);
    assert_eq!(structural.controls.len(), 10);
    assert_eq!(wire::audit(&s).unwrap().total_charge, 81_920);
    let empty = structural
        .blobs
        .keys()
        .find(|r| r.byte_length.get() == 0)
        .unwrap();
    assert_eq!(empty.sha256.as_str(), wire::hash(b""));
    assert_eq!(s.allocations[&structural.blobs[empty]].bytes, b"");
    let mut s = s;
    let file = s
        .allocations
        .values_mut()
        .find(|f| !f.bytes.is_empty())
        .unwrap();
    file.bytes[0] = b'P';
    wire::structural(&s).unwrap();
    assert_eq!(wire::audit(&s).err(), Some("actual full Blob digest"));
}
#[test]
fn blob_kind_order_and_u64_lengths_preserve_existing_object_ref_contract() {
    let c = corpus();
    let refs = c["same_digest_kind_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| serde_json::from_value::<ObjectRef>(v.clone()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(refs[0].sha256, refs[1].sha256);
    assert_eq!(refs[0].kind, ObjectKind::Blob);
    assert_eq!(refs[1].kind, ObjectKind::Json);
    assert!(refs[0] < refs[1]);
    let mut large = c["same_digest_kind_order"][0].clone();
    large["byte_length"] = json!("4294967296");
    assert_eq!(
        serde_json::from_value::<ObjectRef>(large)
            .unwrap()
            .byte_length
            .get(),
        4_294_967_296
    );
    let mut s = store();
    edit(&mut s, "example.ps2.whole-blob-selection", |v| {
        v["blobs"][0]["kind"] = json!("json");
    });
    assert_eq!(
        wire::structural(&s).err(),
        Some("exact selected Blob order")
    );
}
#[test]
fn framed_reference_layout_unknown_fields_and_missing_ownership_refuse() {
    let mut s = store();
    edit(&mut s, "photara.storage.locator-leaf", |v| {
        v["entries"][0]["physical"]["kind"] = json!("frame");
    });
    assert_eq!(
        wire::structural(&s).err(),
        Some("whole Blob reference dispatch")
    );
    let mut s = store();
    edit(&mut s, "photara.storage.locator-leaf", |v| {
        v["entries"][0]["physical"]["offset"] = json!("0");
    });
    assert_eq!(wire::structural(&s).err(), Some("exact fields"));
    let mut s = store();
    s.allocations.values_mut().next().unwrap().evidence["layout"] = json!("packed");
    assert_eq!(wire::structural(&s).err(), Some("whole Blob layout/extent"));
    let mut s = store();
    edit(&mut s, "example.ps2.whole-blob-selection", |v| {
        v["claims"].as_array_mut().unwrap().pop();
    });
    assert_eq!(
        wire::structural(&s).err(),
        Some("exact owned and charged Blobs")
    );
}
#[test]
fn original_allocation_identity_and_nonfree_empty_namespace_are_enforced() {
    let mut s = store();
    s.allocations.values_mut().next().unwrap().evidence["physical"]["inode"] = json!("999");
    assert_eq!(
        wire::structural(&s).err(),
        Some("original local allocation observation")
    );
    let mut s = store();
    s.allocations.values_mut().next().unwrap().evidence["incarnation"] =
        json!("95000000-0000-4000-8000-000000000099");
    assert_eq!(
        wire::structural(&s).err(),
        Some("original local allocation observation")
    );
    let mut s = store();
    edit(&mut s, "example.ps2.whole-blob-selection", |v| {
        v["namespace_allowance"] = json!("0");
    });
    assert_eq!(
        wire::structural(&s).err(),
        Some("conservative empty file/namespace allowance")
    );
    let mut s = store();
    edit(&mut s, "example.ps2.whole-blob-selection", |v| {
        v["standing_control"] = json!(u64::MAX.to_string());
    });
    assert_eq!(wire::structural(&s).err(), Some("accounting overflow"));
}
#[test]
fn whole_blob_requires_its_capability_and_extensions_remain_nonedges() {
    let mut s = store();
    edit(&mut s, "example.ps2.whole-blob-selection", |v| {
        v["required_features"] = json!([]);
    });
    assert_eq!(wire::structural(&s).err(), Some("whole Blob capability"));
    let mut s = store();
    edit(&mut s, "photara.storage.locator-leaf", |v| {
        v["extensions"]["example.optional"] =
            json!({"kind":"blob","sha256":"f".repeat(64),"byte_length":"999"});
    });
    wire::audit(&s).unwrap();
}
