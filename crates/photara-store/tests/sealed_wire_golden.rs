//! Candidate PS2 wire bytes only. This never reads or publishes a package.

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn candidate_sealed_wire_bytes_are_stable() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/sealed-wire-golden.json"
    ))
    .expect("golden fixture JSON");
    assert_eq!(fixture["status"], "candidate-review-bytes-only");
    let vectors = fixture["vectors"].as_array().expect("vectors array");
    assert_eq!(vectors.len(), 19);

    for vector in vectors {
        let name = vector["name"].as_str().expect("vector name");
        let expected = vector["canonical"].as_str().expect("canonical bytes");
        let digest = vector["sha256"].as_str().expect("digest");
        let bytes = photara_core::canonical_json(&vector["value"]).expect("canonical JSON");
        assert_eq!(bytes, expected.as_bytes(), "canonical bytes for {name}");
        assert!(!bytes.ends_with(b"\n"), "no trailing newline for {name}");
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            digest,
            "digest for {name}"
        );
    }
}

#[test]
fn portable_working_binding_has_no_host_observation_or_authority() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/sealed-wire-golden.json"
    ))
    .expect("golden fixture JSON");
    let binding = fixture["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "portable-working-binding")
        .expect("working binding");
    let record = &binding["value"];
    let location = &record["location"];
    assert_eq!(location["coordinate"]["kind"], "filesystem");
    assert!(location["storage_location_id"].is_string());
    for forbidden in [
        "host_binding_id",
        "absolute_path",
        "inode",
        "mtime",
        "file_size",
        "bookmark",
        "credential",
        "mount_state",
        "access_grant",
    ] {
        assert!(
            record.get(forbidden).is_none(),
            "record excludes {forbidden}"
        );
        assert!(
            location.get(forbidden).is_none(),
            "location excludes {forbidden}"
        );
    }
}

#[test]
fn sample_commitments_bind_the_same_original_operation() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/sealed-wire-golden.json"
    ))
    .expect("golden fixture JSON");
    let vectors = fixture["vectors"].as_array().expect("vectors array");
    let get = |name: &str| {
        vectors
            .iter()
            .find(|v| v["name"] == name)
            .expect("named vector")
    };
    let p0 = get("accepted-prefix-zero")["sha256"].as_str().unwrap();
    let intent = get("semantic-intent")["sha256"].as_str().unwrap();
    let receipt = get("operation-receipt")["sha256"].as_str().unwrap();
    let commit = get("outer-commit-v1-reader-1.2");
    assert_eq!(
        get("unchanged-head-v1")["value"]["commit_sha256"],
        commit["sha256"]
    );
    assert_eq!(commit["value"]["minimum_reader"]["major"], 1);
    assert_eq!(commit["value"]["minimum_reader"]["minor"], 2);
    assert_eq!(get("state-root")["value"]["accepted"]["prefix_sha256"], p0);
    assert_eq!(
        get("empty-operation-index")["value"]["accepted"]["prefix_sha256"],
        p0
    );
    assert_eq!(get("operation-receipt")["value"]["request_sha256"], intent);
    assert_eq!(
        get("accepted-prefix-link-one")["value"]["previous_sha256"],
        p0
    );
    assert_eq!(
        get("accepted-prefix-link-one")["value"]["request_sha256"],
        intent
    );
    assert_eq!(
        get("accepted-prefix-link-one")["value"]["receipt_sha256"],
        receipt
    );
    assert_eq!(
        get("accepted-journal-frame")["value"]["receipt_sha256"],
        receipt
    );
}
