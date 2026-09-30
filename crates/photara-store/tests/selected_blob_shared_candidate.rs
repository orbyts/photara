//! Same D19 project selected by the shared HEAD driver. Unfrozen private fixture only.
#![allow(dead_code)]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "integrated_wire_candidate/resources.rs"]
mod resources;
#[path = "integrated_wire_candidate/wire.rs"]
mod wire;
use wire::{keys, package, provider};
#[path = "selected_blob_candidate/context.rs"]
mod context;
#[path = "selected_blob_candidate/legacy.rs"]
mod legacy;
#[path = "selected_blob_candidate/media.rs"]
mod media;
#[path = "selected_blob_candidate/media_adapter.rs"]
mod media_adapter;
use serde_json::Value;
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
    ))
    .unwrap()
}
#[test]
fn original_d19_shared_head_structural_open_reads_no_media() {
    let c = corpus();
    let trusted = context::D19Context::trusted().unwrap();
    let media = media_adapter::InstrumentedMedia::load(&c).unwrap();
    let w = wire::World::load_blob(&c, &media).unwrap();
    let p = wire::verify_package(&w, &trusted.package(&media)).unwrap();
    assert_eq!(p.total, 348_160);
    assert_eq!(p.roles.len(), 2);
    assert_eq!(p.blob_global.len(), 1);
    assert!(
        p.roles
            .values()
            .all(|r| r.receipts.is_empty() && r.blob_semantic.len() == 1)
    );
    let recovered = wire::recovery_package(&w, &trusted.package(&media)).unwrap();
    assert_eq!(recovered.semantic, p.roles["recovery"].semantic);
    assert_eq!(media.counters(), (0, 0));
}
#[test]
fn current_media_corruption_is_audit_only_but_same_byte_inode_replacement_refuses() {
    let c = corpus();
    let trusted = context::D19Context::trusted().unwrap();
    let id = c["expected"]["blob_allocation"].as_str().unwrap();
    let mut media = media_adapter::InstrumentedMedia::load(&c).unwrap();
    let w = wire::World::load_blob(&c, &media).unwrap();
    wire::audit_package(&w, &trusted.package(&media), &media).unwrap();
    assert_eq!(media.counters(), (5, 1));
    media.corrupt_byte(id).unwrap();
    wire::verify_package(&w, &trusted.package(&media)).unwrap();
    assert_eq!(media.counters(), (5, 1));
    assert!(wire::audit_package(&w, &trusted.package(&media), &media).is_err());
    assert_eq!(media.counters(), (10, 2));
    let mut replacement = media_adapter::InstrumentedMedia::load(&c).unwrap();
    replacement.replace_inode(id).unwrap();
    assert!(wire::verify_package(&w, &trusted.package(&replacement)).is_err());
    assert_eq!(replacement.counters(), (0, 0));
}
#[test]
fn trusted_scope_empty_dispatch_and_original_registration_cannot_be_rebound() {
    let c = corpus();
    let trusted = context::D19Context::trusted().unwrap();
    let media = media_adapter::InstrumentedMedia::load(&c).unwrap();
    let w = wire::World::load_blob(&c, &media).unwrap();
    assert!(wire::verify(&w).is_err());
    assert!(wire::verify_transition(&w, &w, &std::collections::BTreeMap::default()).is_err());
    let mut wrong = w.clone();
    wrong.commit["required_features"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f != "photara.whole-blob-storage.v1");
    wrong.rehash_head();
    assert_eq!(
        wire::verify_package(&wrong, &trusted.package(&media)).err(),
        Some("required capability/floor dispatch")
    );
    let mut wrong = w.clone();
    wrong.commit["root_set"]["conversion_source"] = wrong.commit["root_set"]["active"].clone();
    wrong.rehash_head();
    assert!(wire::verify_package(&wrong, &trusted.package(&media)).is_err());
    let mut wrong = w.clone();
    wrong.allocations.values_mut().next().unwrap().witness["inode"] = serde_json::json!("9999");
    assert_eq!(
        wire::verify_package(&wrong, &trusted.package(&media)).err(),
        Some("original registered allocation identity/extent")
    );
    assert_eq!(media.counters(), (0, 0));
}
#[test]
fn cross_role_whole_allocation_identity_cannot_alias_but_exact_sharing_is_valid() {
    use std::collections::BTreeMap;
    let c = corpus();
    let original = keys::MixedObjectKey::parse(&c["expected"]["blob"]).unwrap();
    let mut other = c["expected"]["blob"].clone();
    other["sha256"] = serde_json::json!("a".repeat(64));
    let other = keys::MixedObjectKey::parse(&other).unwrap();
    let id = c["expected"]["blob_allocation"]
        .as_str()
        .unwrap()
        .to_owned();
    let a = BTreeMap::from([(original, id.clone())]);
    let b = BTreeMap::from([(other, id)]);
    let mut identities = BTreeMap::new();
    wire::merge_blob_identities(&mut identities, &a).unwrap();
    wire::merge_blob_identities(&mut identities, &a).unwrap();
    assert_eq!(
        wire::merge_blob_identities(&mut identities, &b),
        Err("whole Blob identity across roles")
    );
}
#[test]
fn rebuilt_hash_correct_metadata_negatives_reach_their_intended_invariants() {
    let negatives: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/shared-negatives.json"
    ))
    .unwrap();
    let trusted = context::D19Context::trusted().unwrap();
    for (name, delta) in negatives.as_object().unwrap() {
        let mut c = corpus();
        for k in ["bootstrap", "loose"] {
            c[k] = delta[k].clone();
        }
        let media = media_adapter::InstrumentedMedia::load(&c).unwrap();
        let mut w = wire::World::load_blob(&c, &media).unwrap();
        for (id, patches) in delta["patches"].as_object().unwrap() {
            let bytes = &mut w.allocations.get_mut(id).unwrap().bytes;
            for patch in patches.as_array().unwrap() {
                let start = usize::try_from(patch["offset"].as_u64().unwrap()).unwrap();
                let data = wire::unhex(patch["hex"].as_str().unwrap()).unwrap();
                bytes[start..start + data.len()].copy_from_slice(&data);
            }
        }
        let expected = match name.as_str() {
            "cross-role-raw-alias" => "whole Blob identity across roles",
            "state-version" => "schema/project dispatch",
            "state-unknown" => "exact fields",
            "empty-prefix" => "exact empty operation prefix/journal",
            "raw-claim-layout" => "ownership full extent/layout",
            "raw-claim-not-sealed" => "whole Blob ownership classification",
            "raw-charge-understated" => "accounting exact sealed observation identity",
            _ => unreachable!(),
        };
        assert_eq!(
            wire::verify_package(&w, &trusted.package(&media)).err(),
            Some(expected),
            "{name}"
        );
        assert_eq!(media.counters(), (0, 0));
    }
}
