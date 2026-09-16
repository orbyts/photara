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
    let intent = json!({
        "write_id": plan.ids().write_id.uuid(),
        "commit_id": plan.ids().commit_id,
        "expected_head_sha256": plan.expected_head().head_digest(),
        "candidate_head_sha256": package::Sha256Hex::parse(&hash(plan.candidate().files().files()["HEAD.json"].as_ref())).unwrap(),
        "immutable": plan.immutable_files().iter().map(|f| json!({"name":f.name(),"sha256":f.sha256()})).collect::<Vec<_>>()
    });
    let mut intent_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&intent_path)
        .map_err(|_| "intent collision")?;
    intent_file.write_all(&canon(&intent)).unwrap();
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
        assert!(journal.path().join("checkpoint-intent.json").is_file());
    }
}
