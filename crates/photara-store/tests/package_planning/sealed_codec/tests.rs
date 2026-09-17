use super::*;

fn installed() -> (Disk, Ref, Ref) {
    let disk = Disk::new();
    let recovery = codec::state(&disk, 1, None, vec![]).unwrap();
    let old = codec::root(&disk, &recovery).unwrap();
    let Object::Operations { accepted } = disk.get(&old.operations).unwrap() else {
        panic!()
    };
    let active = codec::state(&disk, 2, Some(recovery.digest.clone()), accepted).unwrap();
    codec::snapshot(&disk).unwrap();
    let head = codec::candidate(&disk, active.clone(), recovery.clone()).unwrap();
    disk.replace_head(&head).unwrap();
    disk.barrier().unwrap();
    (disk, active, recovery)
}

fn edit_commit(disk: &Disk, edit: impl FnOnce(&mut Value)) {
    let mut head: Value = parse(&disk.read("HEAD.json").unwrap()).unwrap();
    let path = format!("commits/{}.json", head["commit_id"].as_str().unwrap());
    let mut commit: Value = parse(&disk.read(&path).unwrap()).unwrap();
    edit(&mut commit);
    let bytes = canon(&commit);
    fs::write(disk.path().join(path), &bytes).unwrap();
    head["commit_sha256"] = json!(hash(&bytes));
    fs::write(disk.path().join("HEAD.json"), canon(&head)).unwrap();
}

#[test]
fn serialized_roots_roundtrip_and_real_legacy_reader_refuses() {
    let (disk, active, recovery) = installed();
    let (set, root) = codec::open(&disk).unwrap();
    assert_eq!(set.active, active);
    assert_eq!(set.recovery, recovery);
    assert_eq!(root.revision, 2);
    assert_eq!(
        package::v1_1::validate_directory(disk.path(), PackageLimits::default()).unwrap_err(),
        PackageError::UnsupportedFeature
    );
    let original = package::v1_1::validate_directory(
        disk.path().join("fixture-original"),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(original.commits.len(), 1);
    assert_eq!(
        disk.read("manifest.json").unwrap(),
        disk.original["manifest.json"]
    );
}

#[test]
fn reopening_reader_requires_no_fixture_oracle_memory() {
    let (mut disk, _, _) = installed();
    disk.original.clear();
    assert_eq!(codec::open(&disk).unwrap().1.revision, 2);
}

#[test]
fn independent_recovery_needs_neither_active_root_nor_predecessor() {
    let (disk, active, recovery) = installed();
    fs::write(disk.path().join(active.path().unwrap()), b"damaged").unwrap();
    assert_eq!(codec::root(&disk, &recovery).unwrap().revision, 1);
    assert!(codec::open(&disk).is_err());
    assert!(disk.read("HEAD.json").is_ok()); // Never silently rolls HEAD back.
}

#[test]
fn recovery_damage_refuses_even_with_valid_active() {
    let (disk, active, recovery) = installed();
    fs::write(disk.path().join(recovery.path().unwrap()), b"damaged").unwrap();
    assert!(codec::root(&disk, &active).is_ok());
    assert!(codec::open(&disk).is_err());
}

#[test]
fn required_resource_records_and_embedded_snapshot_are_complete_closure() {
    for kind in [
        "snapshot",
        "resource",
        "version",
        "backing",
        "provenance",
        "retention",
        "operations",
    ] {
        let (disk, active, _) = installed();
        let root = codec::root(&disk, &active).unwrap();
        let victim = root
            .inventory
            .iter()
            .find(|r| {
                matches!(
                    (kind, disk.get::<Object>(r).unwrap()),
                    ("snapshot", Object::Snapshot { .. })
                        | ("resource", Object::Resource { .. })
                        | ("version", Object::Version { .. })
                        | ("backing", Object::Backing { .. })
                        | ("provenance", Object::Provenance { .. })
                        | ("retention", Object::Retention { .. })
                        | ("operations", Object::Operations { .. })
                )
            })
            .unwrap();
        fs::write(disk.path().join(victim.path().unwrap()), b"damaged").unwrap();
        assert!(codec::open(&disk).is_err(), "{kind}");
    }
}

#[test]
fn digest_recomputed_semantic_mismatch_is_still_rejected() {
    for field in [
        "authored",
        "history",
        "inventory",
        "fixture_operations",
        "project_id",
        "package_revision",
        "bootstrap_sha256",
    ] {
        let (disk, _, _) = installed();
        edit_commit(&disk, |c| c[field] = Value::Null);
        assert!(codec::open(&disk).is_err(), "{field}");
    }
}

#[test]
fn feature_discriminator_and_experimental_reader_are_all_required() {
    for case in 0..4 {
        let (disk, _, _) = installed();
        edit_commit(&disk, |c| match case {
            0 => c["required_features"]
                .as_array_mut()
                .unwrap()
                .retain(|v| v != FEATURE),
            1 => c["required_features"]
                .as_array_mut()
                .unwrap()
                .push(json!("example.unknown")),
            2 => c["fixture_root_set"]["discriminator"] = json!("optional-marker"),
            _ => c["fixture_root_set"]["required_reader"] = json!("unknown"),
        });
        assert!(matches!(codec::open(&disk), Err(Error::Unsupported)));
    }
}

#[test]
fn exact_root_and_root_set_inventories_reject_surplus_and_omission() {
    for surplus in [true, false] {
        let (disk, active, _) = installed();
        let mut root = codec::root(&disk, &active).unwrap();
        if surplus {
            root.inventory.insert(
                disk.put(&Object::Snapshot {
                    authored: "unused".into(),
                })
                .unwrap(),
            );
        } else {
            root.inventory.pop_first();
        }
        let damaged = disk.put(&root).unwrap();
        assert_eq!(codec::root(&disk, &damaged).unwrap_err(), Error::Integrity);
        edit_commit(&disk, |c| {
            c["fixture_root_set"]["inventory"]
                .as_array_mut()
                .unwrap()
                .pop();
        });
        assert!(matches!(codec::open(&disk), Err(Error::Integrity)));
    }
}

#[test]
fn resource_identity_is_not_a_backing_locator_and_cross_links_are_checked() {
    let (disk, active, _) = installed();
    let root = codec::root(&disk, &active).unwrap();
    let version = root
        .inventory
        .iter()
        .find(|r| matches!(disk.get::<Object>(r).unwrap(), Object::Version { .. }))
        .unwrap();
    let bad = disk
        .put(&Object::Resource {
            identity: "different-resource".into(),
            version: version.clone(),
        })
        .unwrap();
    let resources = disk.put(&Object::Resources { records: vec![bad] }).unwrap();
    let mut changed = root;
    changed.resources = resources;
    assert!(codec::root(&disk, &disk.put(&changed).unwrap()).is_err());
}

#[test]
fn offline_external_eight_gb_backing_never_opened_by_validation_or_turnover() {
    let (disk, mut active, _) = installed();
    // The instrumented provider proves reads are observable and accounted for.
    assert_eq!(disk.external(false), Err(Error::Unsupported));
    assert_eq!(disk.external_opens.get(), 1);
    disk.external_opens.set(0);
    for revision in 3..=34 {
        let previous = codec::root(&disk, &active).unwrap();
        let Object::Operations { accepted } = disk.get(&previous.operations).unwrap() else {
            panic!()
        };
        let next = codec::state(&disk, revision, Some(active.digest.clone()), accepted).unwrap();
        let head = codec::candidate(&disk, next.clone(), active).unwrap();
        disk.replace_head(&head).unwrap();
        disk.barrier().unwrap();
        let (_, root) = codec::open(&disk).unwrap();
        assert_eq!(root.revision, revision);
        active = next;
    }
    assert_eq!(disk.external_opens.get(), 0);
    assert_eq!(disk.external_bytes.get(), 0);
    disk.external(true).unwrap();
    assert_eq!(disk.external_opens.get(), 1);
    assert_eq!(disk.external_bytes.get(), LARGE);
    println!(
        "sealed codec: 32 disk turnovers; external reads=0; explicit verification logical bytes={LARGE}; no media allocation"
    );
}

#[test]
fn operation_evidence_survives_undo_and_ancestry_without_pinning_old_graph_bytes() {
    let (disk, active, recovery) = installed();
    let original = codec::root(&disk, &recovery).unwrap();
    let current = codec::root(&disk, &active).unwrap();
    let Object::Operations { accepted } = disk.get(&current.operations).unwrap() else {
        panic!()
    };
    let next = codec::state(&disk, 3, Some(active.digest.clone()), accepted).unwrap();
    let intent = recovery::prepare(&disk, next, active).unwrap();
    recovery::publish(&disk, &intent).unwrap();
    let (_, root) = codec::open(&disk).unwrap();
    let Object::Operations { accepted } = disk.get(&root.operations).unwrap() else {
        panic!()
    };
    assert_eq!(accepted.len(), 3);
    assert_eq!(accepted[0].resulting_graph, original.authored);
    assert!(!root.inventory.contains(&original.authored));
    assert!(!root.inventory.contains(&recovery));
    assert!(
        accepted
            .iter()
            .all(|a| a.provenance == "original-credential-free-authority")
    );
}

#[test]
fn every_conversion_snapshot_cut_preserves_complete_original_representation() {
    let count = Disk::new().original.len();
    for cut in 0..=count {
        let disk = Disk::new();
        for (path, bytes) in disk.original.iter().take(cut) {
            disk.immutable(&format!("fixture-original/{path}"), bytes)
                .unwrap();
        }
        assert!(package::v1_1::validate_directory(disk.path(), PackageLimits::default()).is_ok());
        for (path, bytes) in &disk.original {
            assert_eq!(disk.read(path).unwrap(), *bytes);
        }
        // Resume same no-replace snapshot; immutable originals never rewritten.
        codec::snapshot(&disk).unwrap();
        for (path, bytes) in &disk.original {
            assert_eq!(
                disk.read(&format!("fixture-original/{path}")).unwrap(),
                *bytes
            );
        }
    }
    println!(
        "sealed codec: {} interrupted original-snapshot positions",
        count + 1
    );
}

#[test]
fn dispatch_barrier_reopen_and_receipt_cuts_reconcile_original_operation() {
    for cut in 0..6 {
        let disk = Disk::new();
        let recovery = codec::state(&disk, 1, None, vec![]).unwrap();
        let prior = codec::root(&disk, &recovery).unwrap();
        let Object::Operations { accepted } = disk.get(&prior.operations).unwrap() else {
            panic!()
        };
        let active = codec::state(&disk, 2, Some(recovery.digest.clone()), accepted).unwrap();
        let intent = recovery::prepare(&disk, active, recovery).unwrap();
        if cut >= 1 {
            recovery::publish(&disk, &intent).unwrap();
        }
        if cut >= 2 {
            disk.barrier().unwrap();
        }
        if cut >= 3 {
            codec::open(&disk).unwrap();
        }
        if cut >= 4 {
            recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap();
        }
        if cut == 1 {
            assert_eq!(
                recovery::reconcile(&disk, &intent.operation, &intent.request, false),
                Err(Error::Unknown)
            );
        }
        let result = recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap();
        if cut == 0 {
            assert_eq!(result, recovery::Outcome::Pending);
            assert!(
                !disk
                    .path()
                    .join(recovery::path(&intent.operation, "receipt").unwrap())
                    .exists()
            );
            recovery::publish(&disk, &intent).unwrap();
        }
        assert_eq!(
            recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap(),
            recovery::Outcome::Acknowledged
        );
        let receipt_path = recovery::path(&intent.operation, "receipt").unwrap();
        let receipt = disk.read(&receipt_path).unwrap();
        assert_eq!(
            recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap(),
            recovery::Outcome::Acknowledged
        );
        assert_eq!(disk.read(&receipt_path).unwrap(), receipt);
        for (path, bytes) in &disk.original {
            assert_eq!(
                disk.read(&format!("fixture-original/{path}")).unwrap(),
                *bytes
            );
        }
    }
}

#[test]
fn changed_operation_request_or_unrelated_head_never_acknowledges() {
    let disk = Disk::new();
    let root = codec::state(&disk, 1, None, vec![]).unwrap();
    let intent = recovery::prepare(&disk, root.clone(), root).unwrap();
    recovery::publish(&disk, &intent).unwrap();
    assert_eq!(
        recovery::reconcile(&disk, "fresh-op", &intent.request, true),
        Err(Error::Conflict)
    );
    assert_eq!(
        recovery::reconcile(&disk, &intent.operation, "changed-request", true),
        Err(Error::Conflict)
    );
    let intent_path = recovery::path(&intent.operation, "intent").unwrap();
    let before = disk.read(&intent_path).unwrap();
    fs::write(disk.path().join("HEAD.json"), b"unrelated").unwrap();
    assert_eq!(
        recovery::reconcile(&disk, &intent.operation, &intent.request, true),
        Err(Error::Conflict)
    );
    assert!(
        !disk
            .path()
            .join(recovery::path(&intent.operation, "receipt").unwrap())
            .exists()
    );
    assert_eq!(disk.read(&intent_path).unwrap(), before);
}

#[test]
fn tampered_receipt_fails_closed() {
    let disk = Disk::new();
    let root = codec::state(&disk, 1, None, vec![]).unwrap();
    let intent = recovery::prepare(&disk, root.clone(), root).unwrap();
    recovery::publish(&disk, &intent).unwrap();
    let receipt_path = recovery::path(&intent.operation, "receipt").unwrap();
    disk.write(&receipt_path, b"unrelated-receipt").unwrap();
    assert_eq!(
        recovery::reconcile(&disk, &intent.operation, &intent.request, true),
        Err(Error::Conflict)
    );
    assert_eq!(disk.read(&receipt_path).unwrap(), b"unrelated-receipt");
}

#[test]
fn canonical_hash_and_path_validation_fail_closed() {
    let (disk, active, _) = installed();
    assert_eq!(
        Ref {
            digest: "../HEAD".into(),
            length: 1
        }
        .path(),
        Err(Error::Integrity)
    );
    let bytes = disk.read(&active.path().unwrap()).unwrap();
    let mut altered = bytes.clone();
    altered.push(b' ');
    fs::write(disk.path().join(active.path().unwrap()), altered).unwrap();
    assert!(codec::open(&disk).is_err());
    assert!(parse::<Value>(b"{ \"a\":1}").is_err());
}

#[test]
fn minimum_reader_and_conversion_inventory_cannot_be_weakened() {
    for case in 0..3 {
        let (disk, _, _) = installed();
        edit_commit(&disk, |c| match case {
            0 => c["minimum_reader"] = json!({"major":1,"minor":1}),
            1 => {
                c["fixture_root_set"]["conversion"]
                    .as_object_mut()
                    .unwrap()
                    .remove("unknown-extension/opaque.bin");
            }
            _ => c["fixture_root_set"]["conversion"]["HEAD.json"]["digest"] = json!(hash(b"bad")),
        });
        assert!(codec::open(&disk).is_err());
    }
}

#[test]
fn original_snapshot_survives_second_journaled_publication() {
    let (disk, active, recovery) = installed();
    let mut selected = active;
    let original_root = codec::root(&disk, &recovery).unwrap();
    for revision in 3..=4 {
        let root = codec::root(&disk, &selected).unwrap();
        let Object::Operations { accepted } = disk.get(&root.operations).unwrap() else {
            panic!()
        };
        let next = codec::state(&disk, revision, Some(selected.digest.clone()), accepted).unwrap();
        let intent = recovery::prepare(&disk, next.clone(), selected).unwrap();
        recovery::publish(&disk, &intent).unwrap();
        assert_eq!(
            recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap(),
            recovery::Outcome::Acknowledged
        );
        let (set, root) = codec::open(&disk).unwrap();
        assert!(!set.inventory.contains(&original_root.authored));
        let Object::Operations { accepted } = disk.get(&root.operations).unwrap() else {
            panic!()
        };
        assert_eq!(accepted[0].operation, "op-1");
        assert_eq!(accepted.len(), usize::try_from(revision).unwrap());
        for (path, bytes) in &disk.original {
            assert_eq!(
                disk.read(&format!("fixture-original/{path}")).unwrap(),
                *bytes
            );
        }
        selected = next;
    }
    assert!(
        disk.path()
            .join(recovery::path("op-3", "receipt").unwrap())
            .exists()
    );
    assert!(
        disk.path()
            .join(recovery::path("op-4", "receipt").unwrap())
            .exists()
    );
}

#[test]
fn tampered_intent_or_rehashed_inclusion_never_acknowledges() {
    for rehash in [false, true] {
        let disk = Disk::new();
        let root = codec::state(&disk, 1, None, vec![]).unwrap();
        let intent = recovery::prepare(&disk, root.clone(), root).unwrap();
        recovery::publish(&disk, &intent).unwrap();
        let path = recovery::path(&intent.operation, "intent").unwrap();
        let mut envelope: Value = parse(&disk.read(&path).unwrap()).unwrap();
        envelope["intent"]["inclusion"]["digest"] = json!(hash(b"unrelated"));
        if rehash {
            envelope["digest"] = json!(hash(&canon(&envelope["intent"])));
        }
        fs::write(disk.path().join(path), canon(&envelope)).unwrap();
        assert_eq!(
            recovery::reconcile(&disk, &intent.operation, &intent.request, true),
            Err(Error::Integrity)
        );
        assert!(
            !disk
                .path()
                .join(recovery::path(&intent.operation, "receipt").unwrap())
                .exists()
        );
    }
}

#[test]
fn rehashed_active_root_cannot_drop_recovery_operation_evidence() {
    let (disk, active, recovery) = installed();
    let mut root = codec::root(&disk, &active).unwrap();
    let Object::Operations { mut accepted } = disk.get(&root.operations).unwrap() else {
        panic!()
    };
    accepted.remove(0);
    root.inventory.remove(&root.operations);
    root.operations = disk.put(&Object::Operations { accepted }).unwrap();
    root.inventory.insert(root.operations.clone());
    let changed = disk.put(&root).unwrap();
    let head = codec::candidate(&disk, changed, recovery).unwrap();
    disk.replace_head(&head).unwrap();
    assert!(matches!(codec::open(&disk), Err(Error::Integrity)));
}

#[test]
fn acknowledged_head_rollback_is_conflict_without_automatic_replay() {
    let disk = Disk::new();
    let root = codec::state(&disk, 1, None, vec![]).unwrap();
    let intent = recovery::prepare(&disk, root.clone(), root).unwrap();
    recovery::publish(&disk, &intent).unwrap();
    recovery::reconcile(&disk, &intent.operation, &intent.request, true).unwrap();
    disk.replace_head(&intent.old_head).unwrap();
    assert_eq!(
        recovery::reconcile(&disk, &intent.operation, &intent.request, true),
        Err(Error::Conflict)
    );
    assert_eq!(disk.read("HEAD.json").unwrap(), intent.old_head);
}

#[test]
fn same_operation_prepare_reuses_exact_commit_write_ids_and_bytes() {
    let disk = Disk::new();
    let root = codec::state(&disk, 1, None, vec![]).unwrap();
    let first = recovery::prepare(&disk, root.clone(), root.clone()).unwrap();
    let before = file_paths(disk.path()).unwrap();
    let second = recovery::prepare(&disk, root.clone(), root).unwrap();
    assert_eq!(first.next_head, second.next_head);
    assert_eq!(first.commit_bytes, second.commit_bytes);
    assert_eq!(file_paths(disk.path()).unwrap(), before);
}

#[test]
fn changed_head_rejects_publication_and_compaction_preserves_authored_revision() {
    let (disk, active, recovery) = installed();
    let before = codec::root(&disk, &active).unwrap();
    let intent = recovery::prepare(&disk, active.clone(), recovery.clone()).unwrap();
    let alternative = codec::candidate(&disk, active.clone(), recovery).unwrap();
    disk.replace_head(&alternative).unwrap();
    assert_eq!(recovery::publish(&disk, &intent), Err(Error::Conflict));
    assert_eq!(disk.read("HEAD.json").unwrap(), alternative);
    let (_, after) = codec::open(&disk).unwrap();
    assert_eq!(before.authored, after.authored);
    assert_eq!(before.revision, after.revision);
    let head: Value = parse(&disk.read("HEAD.json").unwrap()).unwrap();
    let commit: Value = parse(
        &disk
            .read(&format!(
                "commits/{}.json",
                head["commit_id"].as_str().unwrap()
            ))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(commit["package_revision"], "3");
}

#[test]
fn invalid_candidate_closure_is_rejected_before_dispatch() {
    let disk = Disk::new();
    let root = codec::state(&disk, 1, None, vec![]).unwrap();
    let intent = recovery::prepare(&disk, root.clone(), root.clone()).unwrap();
    fs::write(
        disk.path().join(root.path().unwrap()),
        b"corrupted-before-publication",
    )
    .unwrap();
    assert!(recovery::publish(&disk, &intent).is_err());
    assert_eq!(disk.read("HEAD.json").unwrap(), intent.old_head);
    assert!(package::v1_1::validate_directory(disk.path(), PackageLimits::default()).is_ok());
}
