#![allow(clippy::wildcard_imports, reason = "Integration test API coverage")]
//! Production reader composition against already-frozen canonical packages.
#[expect(
    clippy::single_component_path_imports,
    reason = "Shared support also imports an explicit local crate alias in unit tests"
)]
use photara_store;
use photara_store::package::v1_3::*;
use photara_store::package::*;
use serde_json::Value;
#[path = "ps2_reader_support/mod.rs"]
mod support;
use support::*;
#[test]
fn existing_integrated_and_d19_have_complete_production_proofs() {
    for name in ["integrated", "blob"] {
        let f = Fixture::load(name);
        let proof = f.verify().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(proof.roles().contains_key("active"));
        assert!(proof.roles().contains_key("recovery"));
        assert!(!proof.global().is_empty());
    }
}
#[test]
fn selected_registration_substitution_and_unknown_controls_refuse() {
    let mut f = Fixture::load("integrated");
    let id = *f.allocations.keys().next().unwrap();
    f.allocations.get_mut(&id).unwrap().inode += 1;
    assert!(f.verify().is_err());
    let mut f = Fixture::load("integrated");
    let b = bytes(&serde_json::json!({"unknown":true}));
    f.loose.insert(
        ObjectRef {
            kind: ObjectKind::Json,
            sha256: hash(&b),
            byte_length: DecimalU64::parse(&b.len().to_string()).unwrap(),
        },
        b,
    );
    assert!(f.verify().is_err());
}
#[test]
fn recovery_uses_only_its_selected_physical_closure() {
    let mut f = Fixture::load("integrated");
    let e = f.envelope();
    let active = e.open_role(&f, SelectedRole::Active, tree()).unwrap();
    f.allocations.remove(&active.locator.allocation_id);
    assert!(f.verify().is_err());
    assert!(inspect_recovery(&e, &f, &f, &f, &limits()).is_ok());
}

#[test]
fn existing_selected_origins_use_complete_production_authority() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/origins/linked.json"
    ))
    .unwrap();
    for label in ["selected", "rotated"] {
        let f = Fixture::from_value(corpus["worlds"][label].clone());
        f.verify().unwrap_or_else(|e| panic!("{label}: {e:?}"));
        let e = f.envelope();
        inspect_recovery(&e, &f, &f, &f, &limits()).unwrap();
    }
}
#[test]
fn exact_frozen_coherent_integrated_negatives_refuse() {
    let variants: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/integrated/negatives.json"
    ))
    .unwrap();
    for (name, value) in variants.as_object().unwrap() {
        let mut data = Fixture::load("integrated").data;
        for key in ["bootstrap", "loose", "source_files", "source_witnesses"] {
            data[key] = value[key].clone();
        }
        for (id, changes) in value["allocations"].as_object().unwrap() {
            let mut raw = unhex(data["allocations"][id]["hex"].as_str().unwrap());
            for patch in changes["patches"].as_array().unwrap() {
                let at = usize::try_from(patch["offset"].as_u64().unwrap()).unwrap();
                let bytes = unhex(patch["hex"].as_str().unwrap());
                raw[at..at + bytes.len()].copy_from_slice(&bytes);
            }
            data["allocations"][id]["hex"] =
                Value::String(raw.iter().fold(String::new(), |mut s, b| {
                    use std::fmt::Write as _;
                    write!(&mut s, "{b:02x}").unwrap();
                    s
                }));
        }
        let f = Fixture::from_value(data);
        let boot = &f.data["bootstrap"];
        let commit: Value = serde_json::from_str(boot["commit"].as_str().unwrap()).unwrap();
        let envelope = SelectedEnvelope::parse(
            boot["manifest"].as_str().unwrap().as_bytes(),
            boot["head"].as_str().unwrap().as_bytes(),
            boot["commit"].as_str().unwrap().as_bytes(),
            SelectionIdentity {
                project: PackageUuid::parse(commit["project_id"].as_str().unwrap()).unwrap(),
                library: PackageUuid::parse(commit["root_set"]["library_id"].as_str().unwrap())
                    .unwrap(),
                bootstrap_sha256: Sha256Hex::parse(commit["bootstrap_sha256"].as_str().unwrap())
                    .unwrap(),
            },
            limits().json,
        );
        let original = Fixture::load("integrated");
        let result = envelope
            .and_then(|e| inspect_settled(&e, &f, &f, &f, &original.registration, &limits()));
        assert!(result.is_err(), "accepted {name}");
    }
}
