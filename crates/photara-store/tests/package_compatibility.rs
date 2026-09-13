//! Pure mapping facts, with no migration or persistence effects.
use photara_core::{canonical_json, contracts::schema::Digest};
use photara_store::package::{ObjectRef, compatibility::*};
use serde_json::{Value, json};
fn id(n: u32) -> String {
    format!("62000000-0000-4000-8000-{n:012}")
}
fn from<T: serde::de::DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
fn vectors() -> Value {
    let hash = Digest::of_bytes(b"verified source");
    json!({"schema":{"id":"photara.fixture.d19-compatibility","version":1},"codec":"photara.canonical-json.v1",
        "library_mapping":{"source":id(22000),"destination":id(22000)},
        "location_mapping":{"source":id(22001),"destination":id(22001)},
        "association":{"project_id":id(22002),"source_object":{"kind":"json","sha256":hash,"byte_length":"15"},"source_digest":hash,"origin_library_id":null,"chosen_library_id":id(22000)},
        "unresolved":{"state":"unresolved","project_id":id(22002),"source_resolver_id":id(22003),"source_root_id":null},
        "authority":"inspection-only","failure_boundary":"No migration, registration, access grant, host binding or publisher"})
}
#[test]
fn explicit_ids_and_null_origin() {
    let v = vectors();
    let lib: LibraryIdentityMapping = from(v["library_mapping"].clone());
    lib.validate().unwrap();
    let location: LocationIdentityMapping = from(v["location_mapping"].clone());
    location.validate().unwrap();
    let association: ProjectAssociationMapping = from(v["association"].clone());
    association
        .validate(association.project_id, &association.source_object, None)
        .unwrap();
    assert_eq!(association.origin_library_id, None);
    assert!(
        association
            .validate(
                association.project_id,
                &association.source_object,
                Some(from(json!(id(22009))))
            )
            .is_err()
    );
}
#[test]
fn changed_identity_or_source_rejected() {
    let v = vectors();
    let mut lib: LibraryIdentityMapping = from(v["library_mapping"].clone());
    lib.destination = from(json!(id(22009)));
    assert!(lib.validate().is_err());
    let association: ProjectAssociationMapping = from(v["association"].clone());
    let other: ObjectRef =
        from(json!({"kind":"json","sha256":Digest::of_bytes(b"other"),"byte_length":"5"}));
    assert!(
        association
            .validate(association.project_id, &other, None)
            .is_err()
    );
}
#[test]
fn unresolved_root_never_becomes_device_binding() {
    let mapping: ExternalResolverMapping = from(vectors()["unresolved"].clone());
    assert!(matches!(
        mapping,
        ExternalResolverMapping::Unresolved {
            source_root_id: None,
            ..
        }
    ));
    let mut v = vectors()["unresolved"].clone();
    v["host_binding_id"] = json!(id(22003));
    assert!(serde_json::from_value::<ExternalResolverMapping>(v).is_err());
}
#[test]
fn explicit_selection_has_no_inventory_fallback() {
    let pin = json!({"package_id":"example.review","package_version":"1.2.3","definition_id":"example.review.inspect","definition_version":3});
    let mapping: SourceSelectionMapping = from(
        json!({"project_id":id(22002),"graph_id":id(22004),"node_id":id(22005),"old_pin":pin,"chosen_pin":pin,"snapshot":{"schema_version":1,"snapshot_id":id(22006),"project_id":id(22002),"members":[]},"output_port":"assets","migration_digest":Digest::of_bytes(b"selection")}),
    );
    assert!(mapping.validate().is_err());
    let mut selected = mapping;
    selected.chosen_pin.definition_version = photara_core::NodeDefinitionVersion::new(4).unwrap();
    selected.validate().unwrap();
    assert!(selected.snapshot.members().is_empty());
}
#[test]
#[ignore = "Explicit fixture generation only"]
fn generate_compatibility_golden() {
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/fixtures/generation-two/d19-compatibility.json"),
        canonical_json(&vectors()).unwrap(),
    )
    .unwrap();
}
#[test]
fn compatibility_golden_is_exact_rust_canonical_bytes() {
    assert_eq!(
        canonical_json(&vectors()).unwrap(),
        include_bytes!("../../../docs/fixtures/generation-two/d19-compatibility.json")
    );
}
