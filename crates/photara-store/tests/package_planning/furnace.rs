//! Disposable PS2 publication furnace. This is intentionally NOT the qualified
//! production adapter: paths are confined to `TempDir` fixtures, and std sync is
//! insufficient evidence of macOS full power-loss durability.
use super::*;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stop {
    None,
    AfterIntent,
    AfterImmutable(usize),
    AfterHead,
}

fn sync_dir(path: &Path) {
    File::open(path).unwrap().sync_all().unwrap();
}

fn intent_bytes(plan: &CheckpointPlan) -> Vec<u8> {
    canon(&json!({
        "write_id": plan.ids().write_id.uuid(),
        "commit_id": plan.ids().commit_id,
        "expected_head_sha256": plan.expected_head().head_digest(),
        "candidate_head_sha256": package::Sha256Hex::parse(&hash(plan.candidate().files().files()["HEAD.json"].as_ref())).unwrap(),
        "immutable": plan.immutable_files().iter().map(|f| json!({"name":f.name(),"sha256":f.sha256()})).collect::<Vec<_>>()
    }))
}

fn exact_head(root: &Path, plan: &CheckpointPlan) -> bool {
    let manifest = fs::read(root.join("manifest.json")).unwrap();
    let head = fs::read(root.join("HEAD.json")).unwrap();
    plan.expected_head()
        .compare(incarnation(), &manifest, &head)
        .is_ok()
}

fn candidate_head(root: &Path, plan: &CheckpointPlan) -> bool {
    fs::read(root.join("HEAD.json")).unwrap()
        == plan.candidate().files().files()["HEAD.json"].as_ref()
}

// One deterministic, deliberately disposable attempt. The separately synced
// intent precedes *all* package writes; stop points simulate unknown outcomes.
fn publish_until(
    root: &Path,
    journal: &Path,
    plan: &CheckpointPlan,
    stop: Stop,
) -> Result<(), &'static str> {
    if !exact_head(root, plan) {
        return Err("stale HEAD");
    }
    let intent_path = journal.join("checkpoint-intent.json");
    let mut intent_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&intent_path)
        .map_err(|_| "intent collision")?;
    intent_file.write_all(&intent_bytes(plan)).unwrap();
    intent_file.sync_all().unwrap();
    sync_dir(journal);
    if stop == Stop::AfterIntent {
        return Ok(());
    }
    for (index, planned) in plan.immutable_files().iter().enumerate() {
        let destination = root.join(planned.name());
        let parent = destination.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        let temporary = parent.join(format!(
            ".ps2-fixture-{}-{index}.tmp",
            plan.ids().write_id.uuid()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "temporary collision")?;
        file.write_all(planned.bytes()).unwrap();
        file.sync_all().unwrap();
        // hard_link is create-no-replace in this isolated furnace. Production
        // needs descriptor-relative publication and identity checks instead.
        fs::hard_link(&temporary, &destination).map_err(|_| "immutable collision")?;
        fs::remove_file(&temporary).unwrap();
        sync_dir(parent);
        if stop == Stop::AfterImmutable(index) {
            return Ok(());
        }
    }
    if !exact_head(root, plan) {
        return Err("stale HEAD");
    }
    let temporary = root.join(format!(".ps2-head-{}.tmp", plan.ids().write_id.uuid()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "HEAD temporary collision")?;
    file.write_all(plan.candidate().files().files()["HEAD.json"].as_ref())
        .unwrap();
    file.sync_all().unwrap();
    fs::rename(&temporary, root.join("HEAD.json")).unwrap();
    sync_dir(root);
    if stop == Stop::AfterHead {
        return Ok(());
    }
    if !candidate_head(root, plan) {
        return Err("HEAD publication mismatch");
    }
    let verified = package::v1_1::validate_directory(root, PackageLimits::default())
        .map_err(|_| "candidate reader rejection")?;
    if verified.commits[0].commit_id.as_uuid() != plan.ids().commit_id.uuid() {
        return Err("candidate commit mismatch");
    }
    Ok(())
}

fn classify(root: &Path, plan: &CheckpointPlan) -> &'static str {
    let verified = package::v1_1::validate_directory(root, PackageLimits::default()).unwrap();
    if exact_head(root, plan) {
        "old"
    } else if candidate_head(root, plan)
        && verified.commits[0].commit_id.as_uuid() == plan.ids().commit_id.uuid()
    {
        "new"
    } else {
        "unrelated"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Recovery {
    ResumePending,
    AlreadyPublished,
    Conflict,
}

// Deliberately disposable recovery oracle. It uses the persisted intent,
// immutable-byte checks and the independent package reader; HEAD alone is not
// a receipt. No live package is ever opened by this fixture.
fn reconcile_disposable(root: &Path, journal: &Path, plan: &CheckpointPlan) -> Recovery {
    if fs::read(journal.join("checkpoint-intent.json"))
        .ok()
        .as_deref()
        != Some(intent_bytes(plan).as_slice())
    {
        return Recovery::Conflict;
    }
    for planned in plan.immutable_files() {
        let path = root.join(planned.name());
        if path.exists()
            && fs::read(path).map_or(true, |bytes| planned.verify_existing(&bytes).is_err())
        {
            return Recovery::Conflict;
        }
    }
    if exact_head(root, plan) {
        return Recovery::ResumePending;
    }
    if !candidate_head(root, plan) {
        return Recovery::Conflict;
    }
    let Ok(manifest) = fs::read(root.join("manifest.json")) else {
        return Recovery::Conflict;
    };
    if hash(&manifest) != plan.expected_head().manifest_digest().as_str() {
        return Recovery::Conflict;
    }
    let Ok(verified) = package::v1_1::validate_directory(root, PackageLimits::default()) else {
        return Recovery::Conflict;
    };
    if verified.commits[0].commit_id.as_uuid() == plan.ids().commit_id.uuid() {
        Recovery::AlreadyPublished
    } else {
        Recovery::Conflict
    }
}

// Idempotent *fixture* completion after a clean process stop. It deliberately
// cannot resume unknown temporary-file or concurrent-writer states and offers
// no production durability receipt.
fn resume_disposable(
    root: &Path,
    journal: &Path,
    plan: &CheckpointPlan,
) -> Result<Recovery, &'static str> {
    resume_disposable_with_hook(root, journal, plan, |_| {})
}

fn resume_disposable_with_hook(
    root: &Path,
    journal: &Path,
    plan: &CheckpointPlan,
    mut after: impl FnMut(Stop),
) -> Result<Recovery, &'static str> {
    match reconcile_disposable(root, journal, plan) {
        Recovery::AlreadyPublished => return Ok(Recovery::AlreadyPublished),
        Recovery::Conflict => return Err("recovery conflict"),
        Recovery::ResumePending => {}
    }
    for (index, planned) in plan.immutable_files().iter().enumerate() {
        let destination = root.join(planned.name());
        if destination.exists() {
            continue; // reconcile already verified the exact bytes.
        }
        let parent = destination.parent().ok_or("missing parent")?;
        fs::create_dir_all(parent).map_err(|_| "parent create failed")?;
        let temporary = parent.join(format!(
            ".ps2-resume-{}-{index}.tmp",
            plan.ids().write_id.uuid()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "resume temporary collision")?;
        file.write_all(planned.bytes())
            .map_err(|_| "write failed")?;
        file.sync_all().map_err(|_| "file sync failed")?;
        fs::hard_link(&temporary, &destination).map_err(|_| "immutable collision")?;
        fs::remove_file(&temporary).map_err(|_| "temporary cleanup failed")?;
        sync_dir(parent);
        after(Stop::AfterImmutable(index));
    }
    if !exact_head(root, plan) {
        return Err("HEAD changed during recovery");
    }
    let temporary = root.join(format!(
        ".ps2-resume-head-{}.tmp",
        plan.ids().write_id.uuid()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "HEAD temporary collision")?;
    file.write_all(plan.candidate().files().files()["HEAD.json"].as_ref())
        .map_err(|_| "HEAD write failed")?;
    file.sync_all().map_err(|_| "HEAD sync failed")?;
    fs::rename(&temporary, root.join("HEAD.json")).map_err(|_| "HEAD rename failed")?;
    sync_dir(root);
    after(Stop::AfterHead);
    if reconcile_disposable(root, journal, plan) != Recovery::AlreadyPublished {
        return Err("candidate verification failed");
    }
    Ok(Recovery::AlreadyPublished)
}

#[test]
fn every_disposable_publish_stop_has_a_valid_old_or_new_head() {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30300).unwrap());
    assert!(!plan.immutable_files().is_empty());
    let stops = std::iter::once(Stop::AfterIntent)
        .chain((0..plan.immutable_files().len()).map(Stop::AfterImmutable))
        .chain([Stop::AfterHead, Stop::None]);
    for stop in stops {
        let package_root = materialize(&owned(base.files()));
        let journal = tempfile::tempdir().unwrap();
        publish_until(package_root.path(), journal.path(), &plan, stop).unwrap();
        let state = classify(package_root.path(), &plan);
        assert_eq!(
            state,
            if matches!(stop, Stop::AfterHead | Stop::None) {
                "new"
            } else {
                "old"
            },
            "stop={stop:?}"
        );
        assert_eq!(
            reconcile_disposable(package_root.path(), journal.path(), &plan),
            if matches!(stop, Stop::AfterHead | Stop::None) {
                Recovery::AlreadyPublished
            } else {
                Recovery::ResumePending
            },
            "stop={stop:?}"
        );
        assert!(journal.path().join("checkpoint-intent.json").is_file());
        // A stop never rewrites any prior immutable byte.
        for (name, bytes) in base.files().files() {
            if name != "HEAD.json" {
                assert_eq!(
                    fs::read(package_root.path().join(name)).unwrap(),
                    bytes.as_ref()
                );
            }
        }
        assert_eq!(
            resume_disposable(package_root.path(), journal.path(), &plan),
            Ok(Recovery::AlreadyPublished),
            "stop={stop:?}"
        );
        assert_eq!(classify(package_root.path(), &plan), "new");
        let completed_head = fs::read(package_root.path().join("HEAD.json")).unwrap();
        assert_eq!(
            resume_disposable(package_root.path(), journal.path(), &plan),
            Ok(Recovery::AlreadyPublished)
        );
        assert_eq!(
            fs::read(package_root.path().join("HEAD.json")).unwrap(),
            completed_head
        );
    }
}

#[test]
fn exact_head_conflict_refuses_all_package_writes() {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30400).unwrap());
    let package_root = materialize(&owned(base.files()));
    let journal = tempfile::tempdir().unwrap();
    let head_path = package_root.path().join("HEAD.json");
    let original = fs::read(&head_path).unwrap();
    fs::write(&head_path, b"different HEAD").unwrap();
    assert_eq!(
        publish_until(package_root.path(), journal.path(), &plan, Stop::None),
        Err("stale HEAD")
    );
    assert_eq!(fs::read(&head_path).unwrap(), b"different HEAD");
    assert!(!journal.path().join("checkpoint-intent.json").exists());
    assert_ne!(original, b"different HEAD");
}

#[test]
fn reconciliation_refuses_tampered_intent_and_immutable_collision() {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30450).unwrap());
    let package_root = materialize(&owned(base.files()));
    let journal = tempfile::tempdir().unwrap();
    publish_until(
        package_root.path(),
        journal.path(),
        &plan,
        Stop::AfterImmutable(0),
    )
    .unwrap();
    assert_eq!(
        reconcile_disposable(package_root.path(), journal.path(), &plan),
        Recovery::ResumePending
    );
    let intent_path = journal.path().join("checkpoint-intent.json");
    let intent = fs::read(&intent_path).unwrap();
    fs::write(&intent_path, b"unrelated intent").unwrap();
    assert_eq!(
        reconcile_disposable(package_root.path(), journal.path(), &plan),
        Recovery::Conflict
    );
    assert_eq!(
        resume_disposable(package_root.path(), journal.path(), &plan),
        Err("recovery conflict")
    );
    fs::write(&intent_path, intent).unwrap();
    fs::write(
        package_root.path().join(plan.immutable_files()[0].name()),
        b"collision",
    )
    .unwrap();
    assert_eq!(
        reconcile_disposable(package_root.path(), journal.path(), &plan),
        Recovery::Conflict
    );
    assert_eq!(
        resume_disposable(package_root.path(), journal.path(), &plan),
        Err("recovery conflict")
    );
    fs::write(
        package_root.path().join(plan.immutable_files()[0].name()),
        plan.immutable_files()[0].bytes(),
    )
    .unwrap();
    fs::write(package_root.path().join("HEAD.json"), b"unrelated HEAD").unwrap();
    assert_eq!(
        reconcile_disposable(package_root.path(), journal.path(), &plan),
        Recovery::Conflict
    );
    assert_eq!(
        resume_disposable(package_root.path(), journal.path(), &plan),
        Err("recovery conflict")
    );
}

#[test]
fn subprocess_stop_helper() {
    let Ok(stage) = std::env::var("PHOTARA_PS2_STOP_STAGE") else {
        return;
    };
    let root = std::env::var("PHOTARA_PS2_PACKAGE_ROOT").unwrap();
    let journal = std::env::var("PHOTARA_PS2_JOURNAL_ROOT").unwrap();
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30500).unwrap());
    let stop = match stage.as_str() {
        "intent" => Stop::AfterIntent,
        "head" => Stop::AfterHead,
        _ => {
            let index: usize = stage
                .strip_prefix("immutable-")
                .expect("known disposable stop stage")
                .parse()
                .expect("immutable index");
            assert!(index < plan.immutable_files().len());
            Stop::AfterImmutable(index)
        }
    };
    if std::env::var("PHOTARA_PS2_RESUME").as_deref() == Ok("1") {
        resume_disposable_with_hook(Path::new(&root), Path::new(&journal), &plan, |phase| {
            if phase == stop {
                // Terminate inside resume, before its final reconciliation.
                std::process::exit(87);
            }
        })
        .unwrap();
        panic!("resume did not reach requested stop: {stop:?}");
    }
    publish_until(Path::new(&root), Path::new(&journal), &plan, stop).unwrap();
    // No test teardown or Rust unwinding: the parent must recover from disk.
    std::process::exit(86);
}

#[test]
fn child_process_exit_after_each_publish_phase_reopens_old_or_new() {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30500).unwrap());
    let executable = std::env::current_exe().unwrap();
    let stages = std::iter::once(("intent".to_owned(), "old"))
        .chain((0..plan.immutable_files().len()).map(|index| (format!("immutable-{index}"), "old")))
        .chain(std::iter::once(("head".to_owned(), "new")));
    for (stage, expected) in stages {
        let package_root = materialize(&owned(base.files()));
        let journal = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(&executable)
            .arg("--exact")
            .arg("planning::furnace::subprocess_stop_helper")
            .env("PHOTARA_PS2_STOP_STAGE", &stage)
            .env("PHOTARA_PS2_PACKAGE_ROOT", package_root.path())
            .env("PHOTARA_PS2_JOURNAL_ROOT", journal.path())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(86), "stage={stage}: {result:?}");
        assert_eq!(
            classify(package_root.path(), &plan),
            expected,
            "stage={stage}"
        );
        assert_eq!(
            reconcile_disposable(package_root.path(), journal.path(), &plan),
            if expected == "new" {
                Recovery::AlreadyPublished
            } else {
                Recovery::ResumePending
            },
            "stage={stage}"
        );
        assert_eq!(
            resume_disposable(package_root.path(), journal.path(), &plan),
            Ok(Recovery::AlreadyPublished),
            "stage={stage}"
        );
        assert_eq!(classify(package_root.path(), &plan), "new");
        assert!(journal.path().join("checkpoint-intent.json").is_file());
    }
}

// Read the entire synthetic fixture, including any unexpected extra files, so
// byte equality also catches duplicate commits, objects or leaked temporaries.
fn fixture_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                pending.push(entry.path());
            } else {
                assert!(kind.is_file(), "fixture must not contain special files");
                let path = entry.path();
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                files.insert(name, fs::read(path).unwrap());
            }
        }
    }
    files
}

#[test]
fn child_exit_during_each_resume_phase_reuses_intent_and_completes_once() {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Checkpointed");
    let plan = checkpoint(run(&base, &mutation, 30500).unwrap());
    let executable = std::env::current_exe().unwrap();
    // Every initial publication prefix, then every still-missing immutable and
    // HEAD boundary during resume. Existing immutable files must be reused.
    for present in 0..=plan.immutable_files().len() {
        let stages = (present..plan.immutable_files().len())
            .map(|index| (format!("immutable-{index}"), "old"))
            .chain(std::iter::once(("head".to_owned(), "new")));
        for (stage, expected) in stages {
            let package_root = materialize(&owned(base.files()));
            let journal = tempfile::tempdir().unwrap();
            let initial_stop = present
                .checked_sub(1)
                .map_or(Stop::AfterIntent, Stop::AfterImmutable);
            publish_until(package_root.path(), journal.path(), &plan, initial_stop).unwrap();
            let original_intent = fixture_files(journal.path());
            let result = std::process::Command::new(&executable)
                .arg("--exact")
                .arg("planning::furnace::subprocess_stop_helper")
                .env("PHOTARA_PS2_STOP_STAGE", &stage)
                .env("PHOTARA_PS2_RESUME", "1")
                .env("PHOTARA_PS2_PACKAGE_ROOT", package_root.path())
                .env("PHOTARA_PS2_JOURNAL_ROOT", journal.path())
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(87),
                "present={present}, stage={stage}: {result:?}"
            );
            // Construct a fresh recovery oracle; no child receipt or in-memory
            // progress is available. Identity remains the original persisted intent.
            let reopened = verified(build(add_history));
            let recovered_plan =
                checkpoint(run(&reopened, &rename(&reopened, "Checkpointed"), 30500).unwrap());
            assert_eq!(classify(package_root.path(), &recovered_plan), expected);
            assert_eq!(
                reconcile_disposable(package_root.path(), journal.path(), &recovered_plan),
                if expected == "old" {
                    Recovery::ResumePending
                } else {
                    Recovery::AlreadyPublished
                }
            );
            assert_eq!(fixture_files(journal.path()), original_intent);
            assert_eq!(
                resume_disposable(package_root.path(), journal.path(), &recovered_plan),
                Ok(Recovery::AlreadyPublished)
            );
            assert_eq!(classify(package_root.path(), &recovered_plan), "new");
            let complete = fixture_files(package_root.path());
            assert_eq!(complete, owned(plan.candidate().files()));
            assert_eq!(
                resume_disposable(package_root.path(), journal.path(), &recovered_plan),
                Ok(Recovery::AlreadyPublished)
            );
            assert_eq!(fixture_files(package_root.path()), complete);
            assert_eq!(fixture_files(journal.path()), original_intent);
        }
    }
}
