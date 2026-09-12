//! All filesystem activity is confined to newly allocated disposable fixtures.
use photara_core::{
    GraphDocument, GraphId, ProjectDocument, ProjectId, canonical_digest, canonical_json,
};
use photara_store::package::{
    self, DecimalU64, JsonLimits, PackageDiagnostic, PackageError, PackageLimits,
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};
use tempfile::TempDir;

const ARCHIVE: &str = include_str!("../../../docs/fixtures/generation-two/package-specimen.json");
const VECTORS: &str = include_str!("../../../docs/fixtures/generation-two/canonical-vectors.json");

fn archive() -> BTreeMap<String, Vec<u8>> {
    serde_json::from_str::<Value>(ARCHIVE).unwrap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap().to_owned(),
                f["utf8"].as_str().unwrap().as_bytes().to_vec(),
            )
        })
        .collect()
}
fn materialize(files: &BTreeMap<String, Vec<u8>>) -> TempDir {
    let root = tempfile::Builder::new()
        .prefix("photara-l1-fixture-")
        .tempdir()
        .unwrap();
    for (path, bytes) in files {
        assert!(
            Path::new(path)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        );
        let target = root.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}
fn canonical(v: &Value) -> Vec<u8> {
    canonical_json(v).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn validate(files: &BTreeMap<String, Vec<u8>>) -> Result<package::ValidatedPackage, PackageError> {
    let root = materialize(files);
    package::validate_directory(root.path(), PackageLimits::default())
}

/// Rewrite a synthetic package with fully repaired checksums so tests reach
/// semantic validation, rather than failing merely on their edited byte hash.
#[expect(
    clippy::items_after_statements,
    reason = "Fixture-only recursive rebuilding helpers stay local to this test builder"
)]
fn rewrite(mut edit: impl FnMut(&str, &mut Value)) -> BTreeMap<String, Vec<u8>> {
    let mut files = archive();
    let originals = files.clone();
    let mut cache = BTreeMap::new();
    fn object(
        old: &Value,
        files: &mut BTreeMap<String, Vec<u8>>,
        originals: &BTreeMap<String, Vec<u8>>,
        cache: &mut BTreeMap<String, Value>,
        edit: &mut impl FnMut(&str, &mut Value),
    ) -> Value {
        if old["kind"] != "json" {
            return old.clone();
        }
        let old_hash = old["sha256"].as_str().unwrap();
        if let Some(new) = cache.get(old_hash) {
            return new.clone();
        }
        let old_path = format!("objects/json/sha256/{old_hash}.json");
        let mut value: Value = serde_json::from_slice(&originals[&old_path]).unwrap();
        let schema = value["schema"]["id"].as_str().unwrap().to_owned();
        edit(&schema, &mut value);
        fn walk(
            v: &mut Value,
            files: &mut BTreeMap<String, Vec<u8>>,
            originals: &BTreeMap<String, Vec<u8>>,
            cache: &mut BTreeMap<String, Value>,
            edit: &mut impl FnMut(&str, &mut Value),
        ) {
            if v.get("kind") == Some(&json!("json")) && v.get("sha256").is_some() {
                *v = object(v, files, originals, cache, edit);
            } else if let Some(a) = v.as_array_mut() {
                for x in a {
                    walk(x, files, originals, cache, edit);
                }
            } else if let Some(m) = v.as_object_mut() {
                for x in m.values_mut() {
                    walk(x, files, originals, cache, edit);
                }
            }
        }
        walk(&mut value, files, originals, cache, edit);
        if schema == "photara.package.inventory" {
            value["objects"].as_array_mut().unwrap().sort_by_key(|r| {
                (
                    r["kind"].as_str().unwrap().to_owned(),
                    r["sha256"].as_str().unwrap().to_owned(),
                )
            });
        }
        let bytes = canonical(&value);
        let digest = hash(&bytes);
        let reference =
            json!({"kind":"json","sha256":digest,"byte_length":bytes.len().to_string()});
        files.insert(format!("objects/json/sha256/{digest}.json"), bytes);
        cache.insert(old_hash.to_owned(), reference.clone());
        reference
    }
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    edit("bootstrap", &mut manifest);
    files.insert("manifest.json".into(), canonical(&manifest));
    let mut commits: Vec<_> = originals
        .iter()
        .filter(|(p, _)| p.starts_with("commits/"))
        .map(|(p, b)| (p.clone(), serde_json::from_slice::<Value>(b).unwrap()))
        .collect();
    commits.sort_by_key(|(_, v)| {
        v["package_revision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    });
    let mut parents = BTreeMap::<String, String>::new();
    for (path, mut commit) in commits {
        commit["bootstrap_sha256"] = json!(hash(&files["manifest.json"]));
        if !commit["parent"].is_null() {
            let parent_id = commit["parent"]["commit_id"].as_str().unwrap();
            commit["parent"]["sha256"] = json!(parents[parent_id]);
        }
        for key in ["authored", "history", "inventory"] {
            commit[key] = object(&commit[key], &mut files, &originals, &mut cache, &mut edit);
        }
        edit("photara.package.commit", &mut commit);
        let bytes = canonical(&commit);
        parents.insert(
            commit["commit_id"].as_str().unwrap().to_owned(),
            hash(&bytes),
        );
        files.insert(path, bytes);
    }
    let mut head: Value = serde_json::from_slice(&files["HEAD.json"]).unwrap();
    head["commit_sha256"] = json!(parents[head["commit_id"].as_str().unwrap()]);
    edit("photara.package.head", &mut head);
    files.insert("HEAD.json".into(), canonical(&head));
    files
}

#[test]
fn exact_rust_canonical_vectors() {
    let vectors: Value = serde_json::from_str(VECTORS).unwrap();
    for v in vectors["valid"].as_array().unwrap() {
        let input = v["input_utf8"].as_str().unwrap().as_bytes();
        let value = package::parse_json(input, JsonLimits::default()).unwrap();
        let bytes = canonical(&value);
        assert_eq!(
            bytes,
            v["canonical_utf8"].as_str().unwrap().as_bytes(),
            "{}",
            v["id"]
        );
        assert_eq!(hash(&bytes), v["sha256"]);
        assert_eq!(canonical_digest(&value).unwrap().to_string(), v["sha256"]);
        assert_eq!(bytes.len().to_string(), v["byte_length"]);
    }
}
#[test]
fn raw_parser_rejects_duplicates_encoding_and_lossy_integer_coercion() {
    for bytes in [
        br#"{"x":1,"x":2}"#.as_slice(),
        br#"{"outer":{"x":1,"\u0078":2}}"#,
        b"\xef\xbb\xbf{}",
        b"\"\xff\"",
        br#""\ud800""#,
        b"NaN",
        b"1e400",
        b"18446744073709551616",
        b"-9223372036854775809",
        b"{}[]",
        b"{\"x\":1} trailing",
    ] {
        assert!(
            package::parse_json(bytes, JsonLimits::default()).is_err(),
            "{bytes:?}"
        );
    }
    assert_eq!(
        package::parse_canonical_json(b"{ \"x\": 1 }\n", JsonLimits::default()),
        Err(PackageError::NonCanonical)
    );
}
#[test]
fn input_resource_limits_fail_closed() {
    assert_eq!(
        package::parse_json(
            b"{}",
            JsonLimits {
                max_bytes: 1,
                ..JsonLimits::default()
            }
        ),
        Err(PackageError::Limit)
    );
    assert!(
        package::parse_json(
            b"[[[0]]]",
            JsonLimits {
                max_depth: 2,
                ..JsonLimits::default()
            }
        )
        .is_err()
    );
    assert!(
        package::parse_json(
            b"[0,1]",
            JsonLimits {
                max_array_elements: 1,
                ..JsonLimits::default()
            }
        )
        .is_err()
    );
    assert!(
        package::parse_json(
            b"{\"a\":0,\"b\":1}",
            JsonLimits {
                max_members: 1,
                ..JsonLimits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn decimal_counter_and_uuid_spellings_are_strict() {
    for value in [
        json!(1),
        json!(1.0),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!("18446744073709551616"),
        json!(" 1"),
    ] {
        assert!(serde_json::from_value::<DecimalU64>(value).is_err());
    }
    assert_eq!(
        DecimalU64::parse("18446744073709551615").unwrap().get(),
        u64::MAX
    );
    assert!(package::PackageUuid::parse("60000000000040008000000000000001").is_err());
    assert!(package::Sha256Hex::parse(&"F".repeat(64)).is_err());
}
#[test]
fn exact_directory_fixture_validates_without_modification() {
    let files = archive();
    let root = materialize(&files);
    let package = package::validate_directory(root.path(), PackageLimits::default()).unwrap();
    assert_eq!(package.objects.len(), 28);
    assert_eq!(package.commits.len(), 2);
    assert_eq!(package.graphs.len(), 2);
    assert_eq!(package.verified_blobs, 1);
    assert_eq!(package.commits[0].authored, package.commits[1].authored);
    assert_eq!(package.authored.title, "North Coast Fixture");
    assert!(
        package
            .diagnostics
            .contains(&PackageDiagnostic::NodeManifestUnavailable)
    );
    assert!(
        package
            .diagnostics
            .contains(&PackageDiagnostic::ExternalResourceNotResolved)
    );
    for (path, bytes) in files {
        assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
    }
}
#[test]
fn rewritten_valid_fixture_is_a_sound_test_builder() {
    assert!(validate(&rewrite(|_, _| {})).is_ok());
}
#[test]
fn unknown_required_features_allow_bootstrap_inspection_not_validation() {
    let files = rewrite(|s, v| {
        if s == "bootstrap" {
            v["required_features"] = json!(["photara.future.v1"]);
        }
    });
    let root = materialize(&files);
    assert_eq!(
        package::inspect_bootstrap(root.path(), PackageLimits::default())
            .unwrap()
            .required_features,
        vec!["photara.future.v1"]
    );
    assert!(matches!(
        package::validate_directory(root.path(), PackageLimits::default()),
        Err(PackageError::UnsupportedFeature)
    ));
}
#[test]
fn future_schema_minimum_reader_and_codec_fail_closed() {
    for target in [
        "photara.project.asset-inventory",
        "photara.package.commit",
        "bootstrap",
    ] {
        let files = rewrite(|s, v| {
            if s == target {
                match s {
                    "bootstrap" => v["canonical_json"] = json!("future"),
                    "photara.package.commit" => v["minimum_reader"]["minor"] = json!(1),
                    _ => v["schema"]["version"] = json!(2),
                }
            }
        });
        assert!(matches!(
            validate(&files),
            Err(PackageError::UnsupportedVersion)
        ));
    }
}
#[test]
fn unknown_optional_fields_remain_exact() {
    let files = rewrite(|s, v| {
        if s == "photara.project.saved-graph" {
            v["example.future"] = json!({"text":"e\u{301}","ordered":[3,1,2]});
        }
    });
    let package = validate(&files).unwrap();
    assert_eq!(
        package.graphs[0].extra["example.future"]["ordered"],
        json!([3, 1, 2])
    );
    for object in package.objects.values() {
        assert_eq!(canonical(&object.value), object.canonical_bytes);
    }
}
#[test]
fn tampered_head_object_blob_and_missing_managed_bytes_reject() {
    for prefix in ["HEAD.json", "objects/json/", "objects/blobs/"] {
        let mut files = archive();
        let key = files
            .keys()
            .find(|k| k.starts_with(prefix))
            .unwrap()
            .clone();
        files.get_mut(&key).unwrap()[0] ^= 1;
        assert!(validate(&files).is_err());
    }
    let mut files = archive();
    let key = files
        .keys()
        .find(|k| k.starts_with("objects/blobs/"))
        .unwrap()
        .clone();
    files.remove(&key);
    assert!(validate(&files).is_err());
}
#[test]
fn wrong_parent_bootstrap_and_inventory_closure_reject_after_rehash() {
    for change in 0..3 {
        let files = rewrite(|s, v| match change {
            0 if s == "photara.package.commit" && !v["parent"].is_null() => {
                v["parent"]["sha256"] = json!("0".repeat(64));
            }
            1 if s == "photara.package.commit" => v["bootstrap_sha256"] = json!("0".repeat(64)),
            2 if s == "photara.package.inventory" => {
                v["objects"].as_array_mut().unwrap().pop();
            }
            _ => {}
        });
        assert!(matches!(validate(&files), Err(PackageError::Integrity)));
    }
}
#[test]
fn bad_typed_counters_ids_and_timestamps_reject_after_rehash() {
    for change in 0..4 {
        let files = rewrite(|s, v| {
            if s == "photara.project.authored" {
                match change {
                    0 => v["authored_revision"] = json!(1),
                    1 => v["authored_revision"] = json!("01"),
                    2 => v["project_id"] = json!("60000000000040008000000000000011"),
                    _ => v["created_at"] = json!("2026-02-30T18:00:00.000Z"),
                }
            }
        });
        assert!(validate(&files).is_err());
    }
}
#[test]
fn numeric_schema_float_and_missing_record_fields_reject() {
    let files = rewrite(|s, v| {
        if s == "photara.project.party-assignment" {
            v["schema"]["version"] = json!(1.0);
        }
    });
    assert!(validate(&files).is_err());
    let files = rewrite(|s, v| {
        if s == "photara.history.attempt-start" {
            v.as_object_mut().unwrap().remove("started_at");
        }
    });
    assert!(validate(&files).is_err());
}
#[test]
fn external_and_managed_path_traversal_reject_without_following() {
    for path in [
        "../private",
        "/etc/passwd",
        "file:///secret",
        "C:/secret",
        "a\\b",
        "a//b",
        "a/./b",
        "HEAD.json",
        "CoMmItS/file",
    ] {
        assert_eq!(
            package::validate_resource_path(path),
            Err(PackageError::Path)
        );
    }
    assert!(package::validate_resource_path("Sessions/coast/source.jpg").is_ok());
    let files = rewrite(|s, v| {
        if s == "photara.project.representation-content" && v["binding"]["kind"] == "external" {
            v["binding"]["relative_path"] = json!("../not-read");
        }
    });
    assert!(matches!(validate(&files), Err(PackageError::Path)));
    let files = rewrite(|s, v| {
        if s == "photara.project.representation-content" && v["binding"]["kind"] == "managed" {
            v["binding"]["path"] = json!("../../not-read");
        }
    });
    assert!(matches!(validate(&files), Err(PackageError::Integrity)));
}
#[test]
fn graph_pins_location_kind_and_asset_membership_are_checked() {
    for change in 0..3 {
        let files = rewrite(|s, v| match change {
            0 if s == "photara.project.saved-graph"
                && !v["graph"]["nodes"].as_array().unwrap().is_empty() =>
            {
                v["required_packages"][0]["package_version"] = json!("9.0.0");
            }
            1 if s == "photara.project.library-snapshot" && v["source"]["kind"] == "location" => {
                v["payload"]["location_kind_id"] = json!("60000000-0000-4000-8000-000000000099");
            }
            2 if s == "photara.project.asset-inventory" => {
                v["assets"][0]["asset_id"] = json!("60000000-0000-4000-8000-000000000099");
            }
            _ => {}
        });
        assert!(validate(&files).is_err());
    }
}
#[test]
fn run_source_configuration_and_attempt_links_are_checked() {
    for (kind, key) in [
        ("photara.history.run-start", "source_graph_digest"),
        ("photara.history.attempt-start", "configuration_digest"),
    ] {
        let files = rewrite(|s, v| {
            if s == kind {
                v[key] = json!("0".repeat(64));
            }
        });
        assert!(matches!(validate(&files), Err(PackageError::Integrity)));
    }
    let files = rewrite(|s, v| {
        if s == "photara.history.attempt-outcome" {
            v["run_id"] = json!("60000000-0000-4000-8000-000000000099");
        }
    });
    assert!(matches!(validate(&files), Err(PackageError::Integrity)));
}
#[test]
fn package_budgets_are_enforced() {
    let root = materialize(&archive());
    for limits in [
        PackageLimits {
            max_objects: 2,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_commits: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_total_json_bytes: 512,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_blob_bytes: 1,
            ..PackageLimits::default()
        },
    ] {
        assert!(matches!(
            package::validate_directory(root.path(), limits),
            Err(PackageError::Limit)
        ));
    }
}
#[cfg(unix)]
#[test]
fn no_follow_and_hardlink_guards_reject_unsafe_files() {
    use std::os::unix::fs::symlink;
    for mode in 0..3 {
        let root = materialize(&archive());
        let outside = tempfile::tempdir().unwrap();
        match mode {
            0 => {
                fs::rename(root.path().join("HEAD.json"), outside.path().join("head")).unwrap();
                symlink(outside.path().join("head"), root.path().join("HEAD.json")).unwrap();
            }
            1 => {
                fs::rename(root.path().join("objects"), outside.path().join("objects")).unwrap();
                symlink(outside.path().join("objects"), root.path().join("objects")).unwrap();
            }
            _ => {
                fs::hard_link(root.path().join("HEAD.json"), outside.path().join("head")).unwrap();
            }
        }
        assert!(matches!(
            package::validate_directory(root.path(), PackageLimits::default()),
            Err(PackageError::Path)
        ));
    }
}
#[test]
fn exact_internal_spelling_rejects_case_aliases() {
    let root = materialize(&archive());
    fs::rename(root.path().join("HEAD.json"), root.path().join("head.json")).unwrap();
    assert!(package::validate_directory(root.path(), PackageLimits::default()).is_err());
}
#[test]
fn existing_one_json_project_round_trip_and_graph_export_still_work() {
    let project = ProjectDocument::new(
        ProjectId::new(),
        "Retained",
        GraphDocument::new(GraphId::new()),
    )
    .unwrap();
    let json = project.to_pretty_json().unwrap();
    assert_eq!(ProjectDocument::from_json(&json).unwrap(), project);
    let graph = project.export_node_graph("Retained graph").unwrap();
    assert_eq!(
        photara_core::NodeGraphDocument::from_json(&graph.to_pretty_json().unwrap()).unwrap(),
        graph
    );
}
#[test]
fn diagnostics_do_not_echo_untrusted_json_or_host_paths() {
    let secret = b"{\"private-token-DO-NOT-LOG\":1,\"private-token-DO-NOT-LOG\":2}";
    let error = package::parse_json(secret, JsonLimits::default()).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("DO-NOT-LOG"));
}

#[test]
fn graph_name_policy_is_checked_or_reported_read_only() {
    let duplicate = rewrite(|schema, value| {
        if schema == "photara.project.saved-graph" {
            value["name"] = json!("  capture  ");
        }
    });
    assert!(matches!(validate(&duplicate), Err(PackageError::Record)));
    let unicode = rewrite(|schema, value| {
        if schema == "photara.project.saved-graph" && value["name"] == "Review" {
            value["name"] = json!("Café");
        }
    });
    assert!(
        validate(&unicode)
            .unwrap()
            .diagnostics
            .contains(&PackageDiagnostic::GraphNameNormalizationUnsupported)
    );
}

#[test]
fn zero_node_versions_and_invalid_timestamp_digits_reject() {
    for key in ["definition", "configuration"] {
        let files = rewrite(|schema, value| {
            if schema == "photara.project.saved-graph" && value["name"] == "Capture" {
                if key == "definition" {
                    value["graph"]["nodes"][0]["definition"]["definition_version"] = json!(0);
                } else {
                    value["graph"]["nodes"][0]["configuration"]["schema"]["version"] = json!(0);
                }
            }
        });
        assert!(matches!(validate(&files), Err(PackageError::Record)));
    }
    let files = rewrite(|schema, value| {
        if schema == "photara.project.authored" {
            value["created_at"] = json!("2026-09-11T+1:00:00.000Z");
        }
    });
    assert!(matches!(validate(&files), Err(PackageError::Record)));
}

#[test]
fn nested_control_extensions_are_retained() {
    let files = rewrite(|schema, value| {
        if schema == "bootstrap" {
            value["format_version"]["example.future"] = json!({"preserved":true});
        }
        if schema == "photara.package.commit" {
            value["schema"]["example.future"] = json!(7);
        }
    });
    let package = validate(&files).unwrap();
    assert_eq!(
        package.bootstrap.format_version.extra["example.future"]["preserved"],
        true
    );
    assert_eq!(package.commits[0].schema.extra["example.future"], 7);
}

#[test]
fn media_dimensions_and_snapshot_revision_tokens_are_typed() {
    for invalid in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!(4_294_967_296_u64),
        json!("1"),
    ] {
        let files = rewrite(|schema, value| {
            if schema == "photara.project.representation-content" {
                value["media"]["width"] = invalid.clone();
            }
        });
        assert!(matches!(validate(&files), Err(PackageError::Record)));
    }
    for invalid in [
        "0",
        "01",
        "-1",
        "9223372036854775808",
        "18446744073709551616",
    ] {
        let files = rewrite(|schema, value| {
            if schema == "photara.project.library-snapshot" {
                value["source"]["revision"]["value"] = json!(invalid);
            }
        });
        assert!(matches!(validate(&files), Err(PackageError::Record)));
    }
    let files = rewrite(|schema, value| {
        if schema == "photara.project.library-snapshot" {
            value["source"]["revision"] = json!({"authority":"service","value":"sr1:1"});
        }
    });
    validate(&files).unwrap();
}
