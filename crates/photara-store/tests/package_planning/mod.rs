use super::*;
use package::{MemoryPackage, planning::io::*, planning::*};
use photara_core::{NodeDefinitionRegistry, ValueTypeRegistry, creation::PackageExtension};

#[path = "furnace.rs"]
mod furnace;
#[path = "replay.rs"]
mod replay;

fn incarnation() -> IncarnationId {
    IncarnationId::parse(&id(30000)).unwrap()
}
fn verified(files: BTreeMap<String, Vec<u8>>) -> VerifiedClosure {
    VerifiedClosure::verify(
        MemoryPackage::new(files, PackageLimits::default()).unwrap(),
        incarnation(),
    )
    .unwrap()
}
fn request(base: &VerifiedClosure, command: AuthoredCommand) -> MutationRequest {
    MutationRequest {
        version: 1,
        operation_id: decode(json!(id(30001))),
        expected: base.coordinate().clone(),
        command,
        updated_at: TIME.into(),
    }
}
fn naming() -> PackageNamingPolicy {
    PackageNamingPolicy {
        write_extension: PackageExtension::try_from("jprtest".to_owned()).unwrap(),
        legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
    }
}
fn ids(n: u32) -> CheckpointIds {
    CheckpointIds {
        write_id: WriteId::parse(&id(n)).unwrap(),
        commit_id: decode(json!(id(n + 1))),
    }
}
fn run(
    base: &VerifiedClosure,
    mutation: &MutationRequest,
    n: u32,
) -> Result<PlanOutcome, PlanError> {
    let policy = naming();
    plan(
        base,
        PlanRequest {
            mutation,
            expected_head: base.token(),
            ids: ids(n),
            naming: &policy,
            observed_extension: &policy.write_extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
}
fn checkpoint(outcome: PlanOutcome) -> Box<CheckpointPlan> {
    let PlanOutcome::Checkpoint(plan) = outcome else {
        panic!("expected checkpoint")
    };
    plan
}
fn rename(base: &VerifiedClosure, name: &str) -> MutationRequest {
    request(
        base,
        AuthoredCommand::RenameGraph {
            graph_id: decode(json!(id(20018))),
            name: name.into(),
        },
    )
}
fn owned(files: &MemoryPackage) -> BTreeMap<String, Vec<u8>> {
    files
        .files()
        .iter()
        .map(|(p, b)| (p.clone(), b.to_vec()))
        .collect()
}
#[test]
fn deterministic_candidate_retains_ancestry_and_passes_current_reader() {
    let base = verified(build(add_history));
    let before = owned(base.files());
    let command = rename(&base, "Renamed");
    let first = checkpoint(run(&base, &command, 30100).unwrap());
    let second = checkpoint(run(&base, &command, 30100).unwrap());
    assert_eq!(
        owned(first.candidate().files()),
        owned(second.candidate().files())
    );
    assert_eq!(
        first.receipt().request_digest(),
        second.receipt().request_digest()
    );
    assert_eq!(first.candidate().token().package_revision().get(), 2);
    assert_eq!(first.receipt().after().revision.get(), 2);
    let graph = &first.receipt().after().graphs[&decode(json!(id(20018)))];
    assert_eq!(graph.revision.get(), 0); // metadata is not graph semantic revision
    assert_ne!(
        graph.envelope_digest,
        first.receipt().before().graphs[&decode(json!(id(20018)))].envelope_digest
    );
    for (p, b) in &before {
        if p != "HEAD.json" {
            assert_eq!(first.candidate().files().files()[p].as_ref(), b);
        }
    }
    assert_eq!(owned(base.files()), before);
    let candidate = owned(first.candidate().files());
    let read = validate(&candidate).unwrap(); // existing filesystem reader; disposable only
    assert_eq!(read.commits.len(), 2);
    assert_eq!(read.commits[0].history, read.commits[1].history);
    assert_eq!(read.graphs[0].value["name"], "Renamed");
    assert_eq!(read.verified_blobs, 1);
    let third = checkpoint(
        run(
            first.candidate(),
            &rename(first.candidate(), "Again"),
            30200,
        )
        .unwrap(),
    );
    assert_eq!(
        validate(&owned(third.candidate().files()))
            .unwrap()
            .commits
            .len(),
        3
    );
}
#[test]
fn opaque_fields_and_untouched_objects_survive_without_lossy_projection() {
    let base = verified(build(|o| {
        o.get_mut("authored").unwrap()["future_optional"] =
            json!({"x":[1,"e\u{301}",null],"negative_zero":-0.0});
        o.get_mut("graph").unwrap()["future_optional"] =
            json!({"schema":{"id":"example.opaque","version":999},"payload":[false,17]});
        o.get_mut("authored").unwrap()["graphs"][0]["future_link"] = json!(["keep"]);
    }));
    let planned = checkpoint(run(&base, &rename(&base, "Changed"), 30100).unwrap());
    let a = package::v1_1::validate_memory(base.files()).unwrap();
    let b = package::v1_1::validate_memory(planned.candidate().files()).unwrap();
    assert_eq!(
        a.authored.value["future_optional"],
        b.authored.value["future_optional"]
    );
    assert_eq!(
        a.graphs[0].value["future_optional"],
        b.graphs[0].value["future_optional"]
    );
    assert_eq!(
        a.authored.value["graphs"][0]["future_link"],
        b.authored.value["graphs"][0]["future_link"]
    );
    for (p, bytes) in base.files().files() {
        if p != "HEAD.json" {
            assert_eq!(planned.candidate().files().files()[p], *bytes);
        }
    }
}
#[test]
fn exact_head_manifest_incarnation_and_authored_conflicts() {
    let base = verified(build(|_| {}));
    let head = base.token().head_bytes();
    let manifest = &base.files().files()["manifest.json"];
    base.token().compare(incarnation(), manifest, head).unwrap();
    let mut altered: Value = serde_json::from_slice(head).unwrap();
    altered["opaque"] = json!(true); // same commit/revision, different exact HEAD
    assert_eq!(
        base.token()
            .compare(incarnation(), manifest, &canon(&altered)),
        Err(PlanError::ExternalChange)
    );
    assert_eq!(
        base.token().compare(incarnation(), b"different", head),
        Err(PlanError::ExternalChange)
    );
    assert_eq!(
        base.token()
            .compare(IncarnationId::parse(&id(30009)).unwrap(), manifest, head),
        Err(PlanError::ExternalChange)
    );
    let mut mutation = rename(&base, "Changed");
    mutation.expected.revision = package::DecimalU64::parse("2").unwrap();
    assert!(matches!(
        run(&base, &mutation, 30100),
        Err(PlanError::RevisionConflict)
    ));
    let first = checkpoint(run(&base, &rename(&base, "Changed"), 30100).unwrap());
    let policy = naming();
    assert!(matches!(
        plan(
            first.candidate(),
            PlanRequest {
                mutation: &rename(first.candidate(), "Again"),
                expected_head: base.token(),
                ids: ids(30200),
                naming: &policy,
                observed_extension: &policy.write_extension
            },
            &NodeDefinitionRegistry::default(),
            &ValueTypeRegistry::default()
        ),
        Err(PlanError::ExternalChange)
    ));
}
#[test]
fn aliases_never_grant_write_and_public_names_do_not_enter_bytes() {
    let base = verified(build(|_| {}));
    let mutation = rename(&base, "Changed");
    let mut policy = naming();
    let legacy = PackageExtension::legacy_creation_alias();
    assert!(policy.can_read(&legacy));
    assert!(matches!(
        plan(
            &base,
            PlanRequest {
                mutation: &mutation,
                expected_head: base.token(),
                ids: ids(30100),
                naming: &policy,
                observed_extension: &legacy
            },
            &NodeDefinitionRegistry::default(),
            &ValueTypeRegistry::default()
        ),
        Err(PlanError::FilenameCutoverRequired)
    ));
    let first = checkpoint(run(&base, &mutation, 30100).unwrap());
    policy.write_extension = PackageExtension::try_from("different".to_owned()).unwrap();
    let other = checkpoint(
        plan(
            &base,
            PlanRequest {
                mutation: &mutation,
                expected_head: base.token(),
                ids: ids(30100),
                naming: &policy,
                observed_extension: &policy.write_extension,
            },
            &NodeDefinitionRegistry::default(),
            &ValueTypeRegistry::default(),
        )
        .unwrap(),
    );
    assert_eq!(
        owned(first.candidate().files()),
        owned(other.candidate().files())
    );
}
#[test]
fn no_op_does_not_allocate_commit_or_advance_revisions() {
    let base = verified(build(|_| {}));
    assert!(matches!(
        run(&base, &rename(&base, "Main"), 30100).unwrap(),
        PlanOutcome::Unchanged(_)
    ));
    let mutation = request(
        &base,
        AuthoredCommand::ProjectMetadata {
            title: "D19 package specimen".into(),
            description: "Synthetic".into(),
        },
    );
    let PlanOutcome::Unchanged(receipt) = run(&base, &mutation, 30100).unwrap() else {
        panic!()
    };
    assert_eq!(receipt.before(), receipt.after());
}
#[test]
fn immutable_names_and_write_ids_cannot_be_rebound() {
    let base = verified(build(|_| {}));
    let mutation = rename(&base, "Changed");
    let first = checkpoint(run(&base, &mutation, 30100).unwrap());
    for f in first.immutable_files() {
        f.verify_existing(f.bytes()).unwrap();
        assert_eq!(
            f.verify_existing(b"collision"),
            Err(PlanError::ImmutableConflict)
        );
        assert_eq!(f.byte_length(), f.bytes().len());
        assert_eq!(f.sha256().as_str(), hash(f.bytes()));
    }
    assert!(matches!(
        run(
            first.candidate(),
            &rename(first.candidate(), "Again"),
            30100
        ),
        Err(PlanError::ImmutableConflict)
    ));
    let mut files = owned(base.files());
    files.insert(format!("commits/{}.json", id(30101)), b"{}".to_vec());
    let base = verified(files);
    assert!(matches!(
        run(&base, &rename(&base, "Changed"), 30100),
        Err(PlanError::ImmutableConflict)
    ));
}
#[test]
fn unknown_versions_bad_bytes_and_resource_limits_fail_closed() {
    let files = build(|_| {});
    let mut bad = files.clone();
    bad.insert("../escape".into(), vec![]);
    assert!(matches!(
        MemoryPackage::new(bad, PackageLimits::default()),
        Err(PackageError::Path)
    ));
    let mut bad = files.clone();
    let p = bad
        .keys()
        .find(|p| p.starts_with("objects/json/"))
        .unwrap()
        .clone();
    bad.get_mut(&p).unwrap().push(b' ');
    assert!(matches!(
        MemoryPackage::new(bad, PackageLimits::default()),
        Err(PackageError::Integrity)
    ));
    let base = verified(files.clone());
    let mut mutation = rename(&base, "Changed");
    mutation.version = 2;
    assert!(matches!(
        run(&base, &mutation, 30100),
        Err(PlanError::UnsupportedCommand)
    ));
    let unsupported = build(|o| o.get_mut("graph").unwrap()["schema"]["version"] = json!(99));
    assert!(
        VerifiedClosure::verify(
            MemoryPackage::new(unsupported, PackageLimits::default()).unwrap(),
            incarnation()
        )
        .is_err()
    );
    for limits in [
        PackageLimits {
            max_objects: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_total_json_bytes: 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_blob_bytes: 4,
            ..PackageLimits::default()
        },
        PackageLimits {
            max_total_blob_bytes: 4,
            ..PackageLimits::default()
        },
        PackageLimits {
            json: package::JsonLimits {
                max_bytes: 1,
                ..package::JsonLimits::default()
            },
            ..PackageLimits::default()
        },
    ] {
        assert!(matches!(
            MemoryPackage::new(files.clone(), limits),
            Err(PackageError::Limit)
        ));
    }
    let base = VerifiedClosure::verify(
        MemoryPackage::new(
            files,
            PackageLimits {
                max_commits: 1,
                ..PackageLimits::default()
            },
        )
        .unwrap(),
        incarnation(),
    )
    .unwrap();
    assert!(matches!(
        run(&base, &rename(&base, "Changed"), 30100),
        Err(PlanError::Package(PackageError::Limit))
    ));
}
#[test]
fn checked_authored_and_metadata_overflow_refuse() {
    for key in ["authored_revision", "metadata_revision"] {
        let base = verified(build(|o| {
            o.get_mut(if key == "authored_revision" {
                "authored"
            } else {
                "graph"
            })
            .unwrap()[key] = json!(u64::MAX.to_string());
        }));
        assert!(matches!(
            run(&base, &rename(&base, "Changed"), 30100),
            Err(PlanError::RevisionExhausted)
        ));
    }
}
#[test]
fn capability_policy_refuses_network_providers_and_missing_barriers() {
    let qualified = CapabilityProfile {
        version: 1,
        kind: StorageKind::LocalApfs,
        volume_identity: package::Sha256Hex::parse(&hash(b"volume")).unwrap(),
        safe_handles: true,
        exclusive_lifetime_lock: true,
        immutable_no_replace: true,
        atomic_same_volume_head: true,
        full_file_flush: true,
        directory_flush: true,
        excludes_uncooperative_writers: true,
    };
    qualified.check_policy().unwrap();
    for kind in [
        StorageKind::Network,
        StorageKind::ProviderManaged,
        StorageKind::Unqualified,
    ] {
        let mut p = qualified.clone();
        p.kind = kind;
        assert_eq!(p.check_policy(), Err(PlanError::UnsupportedStorage));
    }
    let mut p = qualified.clone();
    p.full_file_flush = false;
    assert_eq!(p.check_policy(), Err(PlanError::UnsupportedStorage));
    let mut p = qualified.clone();
    p.excludes_uncooperative_writers = false;
    assert_eq!(p.check_policy(), Err(PlanError::UnsupportedStorage));
    let mut p = qualified;
    p.version = 2;
    assert_eq!(p.check_policy(), Err(PlanError::UnsupportedStorage));
}
#[test]
fn core_graph_move_preserves_history_and_separate_digests() {
    let base = verified(build(add_history));
    let envelope = photara_core::GraphCommandEnvelope {
        command_id: decode(json!(id(30001))),
        graph_id: decode(json!(id(20018))),
        expected_revision: photara_core::GraphRevision::initial(),
        command: photara_core::GraphCommand::SetNodePosition {
            node_id: decode(json!(id(20103))),
            x: 15,
            y: -9,
        },
    };
    let planned = checkpoint(
        run(
            &base,
            &request(
                &base,
                AuthoredCommand::Graph {
                    envelope: Box::new(envelope),
                },
            ),
            30100,
        )
        .unwrap(),
    );
    let result = validate(&owned(planned.candidate().files())).unwrap();
    assert_eq!(
        result.graphs[0].value["graph"]["nodes"][0]["photara.graph-position"],
        json!({"x":15,"y":-9})
    );
    let coordinate = &planned.receipt().after().graphs[&decode(json!(id(20018)))];
    assert_eq!(coordinate.revision.get(), 1);
    assert_ne!(coordinate.semantic_digest, coordinate.envelope_digest);
    assert_eq!(planned.receipt().after().revision.get(), 2);
}

#[test]
fn actual_1024_commit_ceiling_never_truncates_history() {
    let mut files = build(|_| {});
    let mut previous: Value =
        serde_json::from_slice(&files[&format!("commits/{}.json", id(20019))]).unwrap();
    let mut prior_digest = hash(&canon(&previous));
    for n in 2..=1024 {
        let old_id = previous["commit_id"].clone();
        previous["parent"] = json!({"commit_id":old_id,"sha256":prior_digest});
        previous["commit_id"] = json!(id(40000 + n));
        previous["write_id"] = json!(id(50000 + n));
        previous["package_revision"] = json!(n.to_string());
        let bytes = canon(&previous);
        prior_digest = hash(&bytes);
        files.insert(format!("commits/{}.json", id(40000 + n)), bytes);
    }
    let mut head: Value = serde_json::from_slice(&files["HEAD.json"]).unwrap();
    head["commit_id"] = previous["commit_id"].clone();
    head["commit_sha256"] = json!(prior_digest);
    files.insert("HEAD.json".into(), canon(&head));
    let base = verified(files.clone());
    assert_eq!(base.token().package_revision().get(), 1024);
    assert!(matches!(
        run(&base, &rename(&base, "Changed"), 60000),
        Err(PlanError::Package(PackageError::Limit))
    ));
    assert_eq!(owned(base.files()), files);
    // Even a caller-supplied larger budget cannot lift current reader ceilings.
    files.insert(format!("commits/{}.json", id(60001)), b"{}".to_vec());
    assert!(matches!(
        MemoryPackage::new(
            files,
            PackageLimits {
                max_commits: usize::MAX,
                ..PackageLimits::default()
            }
        ),
        Err(PackageError::Limit)
    ));
}

#[test]
#[ignore = "explicit disposable capacity measurement; run with --ignored --nocapture"]
fn measure_repeated_graph_rename_checkpoint_growth() {
    let started = std::time::Instant::now();
    let mut base = verified(build(add_history));
    let initial_bytes: usize = base.files().files().values().map(|bytes| bytes.len()).sum();
    for step in 0..128_u32 {
        let mut mutation = rename(&base, &format!("Graph rename {step}"));
        mutation.operation_id = decode(json!(id(70000 + step)));
        let plan = checkpoint(run(&base, &mutation, 80000 + step * 2).unwrap());
        let files = plan.candidate().files().files();
        let total_bytes: usize = files.values().map(|bytes| bytes.len()).sum();
        if matches!(step, 0 | 31 | 63 | 127) {
            println!(
                "checkpoint={} revision={} files={} total_bytes={} growth_bytes={} elapsed_ms={}",
                step + 1,
                plan.candidate().token().package_revision().get(),
                files.len(),
                total_bytes,
                total_bytes - initial_bytes,
                started.elapsed().as_millis()
            );
        }
        base = verified(owned(plan.candidate().files()));
    }
    assert_eq!(base.token().package_revision().get(), 129);
}
#[test]
fn unknown_nested_graph_fields_refuse_lossy_core_edits() {
    let base = verified(build(|o| {
        add_history(o);
        o.get_mut("graph").unwrap()["graph"]["nodes"][0]["definition"]["future_semantics"] =
            json!(true);
        let graph_digest = hash(&canon(&o["graph"]["graph"]));
        let config_digest = hash(&canon(&o["graph"]["graph"]["nodes"][0]["configuration"]));
        o.get_mut("run-start").unwrap()["source_graph_digest"] = json!(graph_digest);
        o.get_mut("attempt-start").unwrap()["configuration_digest"] = json!(config_digest);
    }));
    let envelope = photara_core::GraphCommandEnvelope {
        command_id: decode(json!(id(30001))),
        graph_id: decode(json!(id(20018))),
        expected_revision: photara_core::GraphRevision::initial(),
        command: photara_core::GraphCommand::SetNodePosition {
            node_id: decode(json!(id(20103))),
            x: 1,
            y: 2,
        },
    };
    assert!(matches!(
        run(
            &base,
            &request(
                &base,
                AuthoredCommand::Graph {
                    envelope: Box::new(envelope)
                }
            ),
            30100
        ),
        Err(PlanError::UnsupportedCommand)
    ));
}
#[test]
fn command_depth_size_and_graph_revision_overflow_are_bounded() {
    let base = verified(build(add_history));
    let mut command = photara_core::GraphCommand::SetNodePosition {
        node_id: decode(json!(id(20103))),
        x: 1,
        y: 2,
    };
    for _ in 0..65 {
        command = photara_core::GraphCommand::Batch {
            commands: vec![command],
        };
    }
    let envelope = photara_core::GraphCommandEnvelope {
        command_id: decode(json!(id(30001))),
        graph_id: decode(json!(id(20018))),
        expected_revision: photara_core::GraphRevision::initial(),
        command,
    };
    assert!(matches!(
        run(
            &base,
            &request(
                &base,
                AuthoredCommand::Graph {
                    envelope: Box::new(envelope)
                }
            ),
            30100
        ),
        Err(PlanError::Package(PackageError::Limit))
    ));
    let huge = request(
        &base,
        AuthoredCommand::ProjectMetadata {
            title: "Okay".into(),
            description: "x".repeat(16 * 1024 * 1024),
        },
    );
    assert!(matches!(
        run(&base, &huge, 30100),
        Err(PlanError::Package(PackageError::Limit))
    ));
    let files = build(|o| {
        add_history(o);
        o.get_mut("graph").unwrap()["graph"]["revision"] = json!(u64::MAX);
        o.get_mut("run-start").unwrap()["source_graph_revision"] = json!(u64::MAX.to_string());
        let digest = hash(&canon(&o["graph"]["graph"]));
        o.get_mut("run-start").unwrap()["source_graph_digest"] = json!(digest);
    });
    let base = verified(files);
    let envelope = photara_core::GraphCommandEnvelope {
        command_id: decode(json!(id(30001))),
        graph_id: decode(json!(id(20018))),
        expected_revision: decode(json!(u64::MAX)),
        command: photara_core::GraphCommand::SetNodePosition {
            node_id: decode(json!(id(20103))),
            x: 1,
            y: 2,
        },
    };
    assert!(matches!(
        run(
            &base,
            &request(
                &base,
                AuthoredCommand::Graph {
                    envelope: Box::new(envelope)
                }
            ),
            30100
        ),
        Err(PlanError::RevisionExhausted)
    ));
}

#[test]
fn unknown_node_value_schema_and_unmapped_add_node_are_refused() {
    let files = build(|o| {
        add_history(o);
        o.get_mut("graph").unwrap()["graph"]["nodes"][0]["configuration"]["schema"]["version"] =
            json!(99);
        let graph_digest = hash(&canon(&o["graph"]["graph"]));
        let config_digest = hash(&canon(&o["graph"]["graph"]["nodes"][0]["configuration"]));
        o.get_mut("run-start").unwrap()["source_graph_digest"] = json!(graph_digest);
        o.get_mut("attempt-start").unwrap()["configuration_digest"] = json!(config_digest);
    });
    let memory = MemoryPackage::new(files, PackageLimits::default()).unwrap();
    package::v1_1::validate_memory(&memory).unwrap(); // reader can preserve it read-only
    assert!(matches!(
        VerifiedClosure::verify(memory, incarnation()),
        Err(PlanError::UnsupportedCommand)
    ));
    let base = verified(build(add_history));
    let read = package::v1_1::validate_memory(base.files()).unwrap();
    let instance = decode(read.graphs[0].value["graph"]["nodes"][0].clone());
    let envelope = photara_core::GraphCommandEnvelope {
        command_id: decode(json!(id(30001))),
        graph_id: decode(json!(id(20018))),
        expected_revision: photara_core::GraphRevision::initial(),
        command: photara_core::GraphCommand::AddNode { instance },
    };
    assert!(matches!(
        run(
            &base,
            &request(
                &base,
                AuthoredCommand::Graph {
                    envelope: Box::new(envelope)
                }
            ),
            30100
        ),
        Err(PlanError::UnsupportedCommand)
    ));
}
#[test]
fn operation_digest_distinguishes_altered_retry_and_invalid_candidates_refuse() {
    let base = verified(build(|_| {}));
    let a = checkpoint(run(&base, &rename(&base, "First"), 30100).unwrap());
    let b = checkpoint(run(&base, &rename(&base, "Second"), 30100).unwrap());
    assert_eq!(a.receipt().operation_id(), b.receipt().operation_id());
    assert_ne!(a.receipt().request_digest(), b.receipt().request_digest());
    // Stateless PS1 exposes the conflict coordinates; PS2 must persist dedupe.
    assert!(run(&base, &rename(&base, ""), 30100).is_err());
    let mut request = rename(&base, "Changed");
    request.updated_at = "not-a-timestamp".into();
    assert!(run(&base, &request, 30100).is_err());
}

#[test]
fn envelope_and_inventory_opaque_fields_are_preserved() {
    let mut files = build(|_| {});
    let commit_path = format!("commits/{}.json", id(20019));
    let mut commit: Value = serde_json::from_slice(&files[&commit_path]).unwrap();
    let old_ref: package::ObjectRef = decode(commit["inventory"].clone());
    let mut inventory: Value = serde_json::from_slice(&files[&old_ref.path()]).unwrap();
    inventory["opaque_inventory"] = json!({"untouched":[1,true]});
    let bytes = canon(&inventory);
    let reference: package::ObjectRef =
        decode(json!({"kind":"json","sha256":hash(&bytes),"byte_length":bytes.len().to_string()}));
    files.insert(reference.path(), bytes);
    commit["inventory"] = json!(reference);
    files.insert(commit_path, canon(&commit));
    rewrite_envelopes(&mut files, |manifest, commit| {
        manifest["opaque_bootstrap"] = json!(["keep"]);
        commit["opaque_commit"] = json!({"payload":"keep"});
        commit["schema"]["opaque_schema"] = json!(7);
    });
    let mut head: Value = serde_json::from_slice(&files["HEAD.json"]).unwrap();
    head["opaque_head"] = json!({"keep":true});
    files.insert("HEAD.json".into(), canon(&head));
    let base = verified(files);
    let planned = checkpoint(run(&base, &rename(&base, "Changed"), 30100).unwrap());
    let reader = package::v1_1::validate_memory(planned.candidate().files()).unwrap();
    assert_eq!(reader.bootstrap.extra["opaque_bootstrap"], json!(["keep"]));
    assert_eq!(reader.head.extra["opaque_head"], json!({"keep":true}));
    assert_eq!(
        reader.commits[0].extra["opaque_commit"],
        json!({"payload":"keep"})
    );
    assert_eq!(reader.commits[0].schema.extra["opaque_schema"], json!(7));
    assert_eq!(
        reader.objects[&reader.commits[0].inventory.sha256].value["opaque_inventory"],
        inventory["opaque_inventory"]
    );
}
