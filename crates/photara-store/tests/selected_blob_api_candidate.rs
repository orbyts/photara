//! Pure helper tests; the shared target separately exercises the complete driver.
#![allow(dead_code)]
#[path = "selected_blob_candidate/context.rs"]
mod context;
#[path = "selected_blob_candidate/keys.rs"]
mod keys;
#[allow(
    dead_code,
    reason = "Original frozen media helper depends on canonical utilities; no legacy package setup in these API tests"
)]
#[path = "selected_blob_candidate/legacy.rs"]
mod legacy;
#[path = "selected_blob_candidate/media.rs"]
mod media;
#[path = "selected_blob_candidate/media_adapter.rs"]
mod media_adapter;
#[path = "selected_blob_candidate/package.rs"]
mod package;
#[path = "selected_blob_candidate/provider.rs"]
mod provider;
use keys::{MixedObjectKey, TreeKey};
use media_adapter::InstrumentedMedia;
use provider::{RawAudit, RawMetadata};
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
    ))
    .unwrap()
}
#[test]
fn mixed_keys_preserve_json_api_and_kind_identity_without_encoded_hash_tags() {
    let digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let json_key = (digest.to_owned(), 5);
    let j = MixedObjectKey::from_json_key(&json_key).unwrap();
    let b =
        MixedObjectKey::parse(&json!({"kind":"blob","sha256":digest,"byte_length":"5"})).unwrap();
    assert_ne!(j, b);
    assert!(b < j);
    assert_eq!(j.json_key().unwrap(), json_key);
    assert_eq!(b.json_key(), Err("Blob cannot become JSON key"));
    assert_eq!(MixedObjectKey::parse(&j.value().unwrap()).unwrap(), j);
    assert_eq!(j.object_ref().sha256.as_str(), digest);
    let empty =
        MixedObjectKey::parse(&json!({"kind":"blob","sha256":digest,"byte_length":"0"})).unwrap();
    assert_eq!(empty.object_ref().byte_length.get(), 0);
    assert!(MixedObjectKey::from_json_key(&(digest.into(), 0)).is_err());
    for value in [
        json!({"kind":"blob","sha256":digest,"byte_length":"05"}),
        json!({"kind":"blob","sha256":digest,"byte_length":"5","offset":"0"}),
        json!({"kind":"other","sha256":digest,"byte_length":"5"}),
    ] {
        assert!(MixedObjectKey::parse(&value).is_err());
    }
    assert_eq!(
        MixedObjectKey::parse(&json!({"kind":"blob","sha256":digest,"byte_length":"4294967296"}))
            .unwrap()
            .object_ref()
            .byte_length
            .get(),
        4_294_967_296
    );
}
#[test]
fn tree_keys_keep_numeric_and_uuid_order_without_string_padding_conventions() {
    assert!(TreeKey::ordinal(&json!("9")).unwrap() < TreeKey::ordinal(&json!("10")).unwrap());
    assert!(TreeKey::ordinal(&json!("01")).is_err());
    assert!(TreeKey::ordinal(&json!("18446744073709551616")).is_err());
    assert!(
        TreeKey::id(&json!("97000000-0000-4000-8000-000000000001")).unwrap()
            < TreeKey::id(&json!("97000000-0000-4000-8000-000000000002")).unwrap()
    );
    assert!(TreeKey::id(&json!("00000000-0000-0000-0000-000000000000")).is_err());
    let c = corpus();
    assert_eq!(
        TreeKey::object(&c["expected"]["blob"]).unwrap(),
        TreeKey::Object(MixedObjectKey::parse(&c["expected"]["blob"]).unwrap())
    );
}
#[test]
fn structural_provider_interface_exposes_metadata_while_explicit_audit_reads_private_bytes() {
    let c = corpus();
    let r = MixedObjectKey::parse(&c["expected"]["blob"]).unwrap();
    let id = c["expected"]["blob_allocation"].as_str().unwrap();
    let mut p = InstrumentedMedia::load(&c).unwrap();
    let registered = p.describe(id).unwrap();
    assert_eq!(
        p.allocation_ids().unwrap(),
        std::collections::BTreeSet::from([id.to_owned()])
    );
    provider::observe_blob(&p, r.object_ref(), id, &registered).unwrap();
    assert_eq!(p.counters(), (0, 0));
    p.corrupt_byte(id).unwrap();
    provider::observe_blob(&p, r.object_ref(), id, &registered).unwrap();
    assert_eq!(p.counters(), (0, 0));
    assert_eq!(
        p.audit(r.object_ref(), id, &registered),
        Err("explicit raw audit digest")
    );
    assert_eq!(p.counters(), (5, 1));
    let mut replaced = InstrumentedMedia::load(&c).unwrap();
    replaced.replace_inode(id).unwrap();
    assert_eq!(
        provider::observe_blob(&replaced, r.object_ref(), id, &registered),
        Err("raw registered extent/witness")
    );
    assert_eq!(replaced.counters(), (0, 0));
    assert!(
        provider::observe_blob(
            &replaced,
            r.object_ref(),
            "97000000-0000-4000-8000-000000009999",
            &registered
        )
        .is_err()
    );
}

#[test]
fn trusted_context_uses_original_registration_and_exact_zero_null_dispatch() {
    let c = corpus();
    let context = context::D19Context::trusted().unwrap();
    let commit: Value = serde_json::from_str(c["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    let root = &commit["root_set"];
    let resolve = |r: &Value| -> legacy::Result<Value> {
        let sha = r["sha256"].as_str().ok_or("test ref")?;
        let raw = c["records"][sha]["canonical"]
            .as_str()
            .ok_or("test object")?;
        legacy::parse(raw.as_bytes())
    };
    let state = resolve(&root["active"]).unwrap();
    let envelope: Value = serde_json::from_str(
        c["loose"][root["placement"]["accounting"]["sha256"].as_str().unwrap()]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let ledger: Value = serde_json::from_str(
        c["loose"][envelope["ledger"]["sha256"].as_str().unwrap()]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(context.empty_index(&state, resolve).unwrap().len(), 3);
    assert_eq!(
        context
            .null_selectors(root, &state, &ledger, 0, resolve)
            .unwrap()
            .len(),
        3
    );
    assert!(
        context
            .null_selectors(root, &state, &ledger, 1, resolve)
            .is_err()
    );
    let mut wrong = state.clone();
    wrong["accepted"]["through_ordinal"] = json!("1");
    assert_eq!(
        context.empty_index(&wrong, resolve).err(),
        Some("context exact zero operation state")
    );
    let provider = InstrumentedMedia::load(&c).unwrap();
    let raw_id = c["expected"]["blob_allocation"].as_str().unwrap();
    let registration = context.registration();
    let frames = registration
        .allocations
        .iter()
        .filter(|(_, r)| r.layout == "framed-json")
        .map(|(id, r)| (id.clone(), (r.arena.clone(), r.description.clone())))
        .collect();
    context.observe_allocations(&frames, &provider).unwrap();
    let semantic = context
        .semantic_members(&state, resolve, |r| {
            Ok(provider::observe_blob(
                &provider,
                r,
                raw_id,
                &registration.allocations[raw_id].description,
            )?
            .extent)
        })
        .unwrap();
    assert!(!semantic.json.is_empty());
    assert_eq!(semantic.blobs.len(), 1);
    assert_eq!(provider.counters(), (0, 0));
    let mut changed = c.clone();
    changed["allocations"][raw_id]["witness"]["inode"] = json!("999999");
    changed["original_evidence"]["allocations"][raw_id]["witness"]["inode"] = json!("999999");
    let substituted = InstrumentedMedia::load(&changed).unwrap();
    assert_eq!(
        context.observe_allocations(&frames, &substituted),
        Err("context original raw registration")
    );
    assert_eq!(context.identity().project, commit["project_id"]);
    assert_eq!(
        context.identity().manifest_bytes,
        c["bootstrap"]["manifest"].as_str().unwrap().as_bytes()
    );
    assert_eq!(
        context
            .required_features()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        serde_json::from_value::<Vec<String>>(commit["required_features"].clone()).unwrap()
    );
    assert_eq!(
        registration.profile,
        "example.ps2.synthetic-local-profile-v1"
    );
    assert_eq!(
        registration.incarnation,
        "97000000-0000-4000-8000-000000000001"
    );
    assert_eq!(registration.standing_control, 65_536);
    assert_eq!(registration.directory_allowance, 16_384);
    assert_eq!(registration.allocations[raw_id].registered_charge, 4096);
}
