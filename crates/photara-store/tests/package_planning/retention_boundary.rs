//! Pure reader-boundary fixtures, not a new root format or migration.
use super::*;

fn read(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<package::v1_1::ValidatedPackageV1_1, PackageError> {
    let memory = MemoryPackage::new(files.clone(), PackageLimits::default()).unwrap();
    let result = package::v1_1::validate_memory(&memory);
    assert_eq!(owned(&memory), *files); // Validation never rewrites the input.
    result
}

fn rewrite_current(files: &mut BTreeMap<String, Vec<u8>>, edit: impl FnOnce(&mut Value)) {
    let mut head: Value = serde_json::from_slice(&files["HEAD.json"]).unwrap();
    let path = format!("commits/{}.json", head["commit_id"].as_str().unwrap());
    let mut commit: Value = serde_json::from_slice(&files[&path]).unwrap();
    edit(&mut commit);
    let bytes = canon(&commit);
    head["commit_sha256"] = json!(hash(&bytes));
    files.insert(path, bytes);
    files.insert("HEAD.json".into(), canon(&head));
}

#[test]
fn ordinary_1_1_remains_readable_without_conversion() {
    let base = verified(build(add_history));
    let original = owned(base.files());
    assert_eq!(read(&original).unwrap().commits.len(), 1);
    let plan = checkpoint(run(&base, &rename(&base, "Ordinary checkpoint"), 62000).unwrap());
    let ordinary = owned(plan.candidate().files());
    for _ in 0..2 {
        let parsed = read(&ordinary).unwrap();
        assert_eq!(parsed.commits.len(), 2);
        assert_eq!(parsed.commits[0].package_revision.get(), 2);
        assert_eq!(parsed.bootstrap.format_version.minor, 1);
    }
    assert_eq!(ordinary["manifest.json"], original["manifest.json"]);
    assert_eq!(owned(base.files()), original);
}

#[test]
fn optional_root_marker_cannot_launder_truncated_1_1_ancestry() {
    let base = verified(build(add_history));
    let original = owned(base.files());
    let plan = checkpoint(run(&base, &rename(&base, "Boundary fixture"), 62000).unwrap());
    let ordinary = owned(plan.candidate().files());
    let mut marked = ordinary.clone();
    rewrite_current(&mut marked, |commit| {
        commit["fixture_optional_root"] =
            json!({"sealed":true,"predecessor_digest":base.token().head_digest()});
    });
    assert_eq!(read(&marked).unwrap().commits.len(), 2);

    let mut truncated = marked.clone();
    truncated
        .remove(&format!("commits/{}.json", id(20019)))
        .unwrap();
    rewrite_current(&mut truncated, |commit| {
        assert_eq!(commit["package_revision"], "2");
        commit["parent"] = Value::Null;
    });
    // Commit and HEAD hashes were recomputed: this is a semantic rejection.
    assert_eq!(read(&truncated).unwrap_err(), PackageError::Integrity);

    let mut gated = truncated;
    rewrite_current(&mut gated, |commit| {
        let features = commit["required_features"].as_array_mut().unwrap();
        // Deliberately unsupported test marker; not a reserved production feature.
        features.push(json!("example.fixture.reader-boundary-only"));
        features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    });
    assert_eq!(read(&gated).unwrap_err(), PackageError::UnsupportedFeature);
    assert_eq!(read(&ordinary).unwrap().commits.len(), 2);
    assert_eq!(owned(base.files()), original);
}
