//! Existing repeatable corpus, exact adapter syscalls; no general storage admission.
use super::super::{DIRECTORY, Scope, create, mkdirat};
use super::*;
use crate::package::v1_3::repeatable::disposable as host;
use serde_json::{Value, json};
use std::process::Command;

fn fixture() -> (Value, SelectedStage) {
    let f = host::support::Fixture::load("integrated");
    let p = host::package(&f);
    let old = host::v1_3::reader::verify_shared_original(
        p.clone(),
        host::identity(&p),
        &f,
        &f.registration,
        host::v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        host::frames(),
        &host::support::limits(),
    )
    .unwrap();
    let (intent, receipt) = host::request();
    let mut attempt = host::attempt(&old, 1000).unwrap();
    attempt.planner.codec = "photara.codec.ps2-repeatable-layout-v1".into();
    let plan = crate::package::v1_3::repeatable::plan::compile_inner(
        &old,
        &intent,
        &receipt,
        attempt,
        host::budget(),
    )
    .unwrap();
    (p.head, plan.stages[0].clone())
}
fn directory(parent: &File, name: &str) -> File {
    mkdirat(parent, name, Mode::RWXU).unwrap();
    File::from(openat(parent, name, DIRECTORY, Mode::empty()).unwrap())
}
fn trace(stage: &SelectedStage) -> Vec<(Step, bool)> {
    let steps = (0..stage.loose.len() + 2).map(Step::FileSync).chain([
        Step::DirectorySync,
        Step::DeviceSync,
        Step::Rename,
        Step::PublishedBarrier,
    ]);
    steps
        .flat_map(|step| [(step, false), (step, true)])
        .collect()
}
fn verify_files(root: &File, stage: &SelectedStage) {
    for (sha, bytes) in &stage.loose {
        assert_eq!(read(root, &format!("{sha}.control")).unwrap(), *bytes);
    }
    let commit = canonical(&stage.commit);
    assert_eq!(
        read(root, &format!("{}.commit", hash(&commit))).unwrap(),
        commit
    );
}
fn reconcile(root: &File, old: &Value, stage: &SelectedStage) {
    let head = read(root, "HEAD.json").unwrap();
    if head == canonical(old) {
        let mut observed = Vec::new();
        publish(root, old, stage, |step, after| {
            observed.push((step, after));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            observed,
            trace(stage),
            "exact existing files must be fsynced again"
        );
    } else {
        assert_eq!(head, canonical(&stage.head));
        // Native::stabilize_progress does not rename a second time. Reissue the
        // existing file + directory barriers before treating visible as durable.
        for sha in stage.loose.keys() {
            barrier(&regular(root, &format!("{sha}.control"), true).unwrap()).unwrap();
        }
        barrier(
            &regular(
                root,
                &format!("{}.commit", hash(&canonical(&stage.commit))),
                true,
            )
            .unwrap(),
        )
        .unwrap();
        barrier(&regular(root, "HEAD.json", true).unwrap()).unwrap();
        barrier(root).unwrap();
    }
    verify_files(root, stage);
    assert_eq!(read(root, "HEAD.json").unwrap(), canonical(&stage.head));
}
fn matrix(parent: &File, old: &Value, stage: &SelectedStage) -> usize {
    let events = trace(stage);
    for (cut, target) in events.iter().enumerate() {
        for errno in [
            rustix::io::Errno::IO,
            rustix::io::Errno::INTR,
            rustix::io::Errno::NOTSUP,
        ] {
            let root = directory(parent, &format!("cut-{cut}-{}", errno.raw_os_error()));
            create(&root, "HEAD.json", &canonical(old)).unwrap();
            barrier(&root).unwrap();
            let mut observed = Vec::new();
            let result = publish(&root, old, stage, |step, after| {
                observed.push((step, after));
                if (step, after) == *target {
                    Err(errno.into())
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
            assert_eq!(observed, events[..=cut]);
            let renamed = events[..=cut].contains(&(Step::Rename, true));
            assert_eq!(
                read(&root, "HEAD.json").unwrap(),
                canonical(if renamed { &stage.head } else { old })
            );
            reconcile(&root, old, stage);
        }
    }
    events.len() * 3
}
#[test]
fn session_tests_grouped_select_order_retry_and_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let parent = File::from(rustix::fs::open(temp.path(), DIRECTORY, Mode::empty()).unwrap());
    let (old, stage) = fixture();
    assert!(matrix(&parent, &old, &stage) > 0);
    let root = directory(&parent, "changed-file");
    create(&root, "HEAD.json", &canonical(&old)).unwrap();
    let mut held = None;
    assert!(
        publish(&root, &old, &stage, |step, after| {
            if (step, after) == (Step::FileSync(0), true) {
                let name = format!("{}.control", stage.loose.keys().next().unwrap());
                held = Some(regular(&root, &name, false)?);
                renameat(&root, &name, &root, "retained-original")?;
                create(&root, &name, b"changed")?;
            }
            Ok(())
        })
        .is_err()
    );
    assert!(held.is_some());
    assert_eq!(read(&root, "HEAD.json").unwrap(), canonical(&old));
    let root = directory(&parent, "changed-head");
    create(&root, "HEAD.json", &canonical(&old)).unwrap();
    assert!(
        publish(&root, &old, &stage, |step, after| {
            if (step, after) == (Step::DeviceSync, true) {
                create(&root, "foreign-head", b"unrelated")?;
                renameat(&root, "foreign-head", &root, "HEAD.json")?;
            }
            Ok(())
        })
        .is_err()
    );
    assert_eq!(read(&root, "HEAD.json").unwrap(), b"unrelated");
}

fn comparison(parent: &File, old: &Value, stage: &SelectedStage) -> Value {
    let mut observations = Vec::new();
    for grouped in [false, true] {
        for run in 0..3 {
            let root = directory(parent, &format!("timing-{grouped}-{run}"));
            create(&root, "HEAD.json", &canonical(old)).unwrap();
            barrier(&root).unwrap();
            let start = std::time::Instant::now();
            if grouped {
                publish(&root, old, stage, |_, _| Ok(())).unwrap();
            } else {
                for (sha, bytes) in &stage.loose {
                    super::super::exact_file(&root, &format!("{sha}.control"), bytes).unwrap();
                }
                let commit = canonical(&stage.commit);
                super::super::exact_file(&root, &format!("{}.commit", hash(&commit)), &commit)
                    .unwrap();
                super::super::exact_file(&root, "HEAD.next", &canonical(&stage.head)).unwrap();
                assert_eq!(read(&root, "HEAD.json").unwrap(), canonical(old));
                renameat(&root, "HEAD.next", &root, "HEAD.json").unwrap();
                barrier(&root).unwrap();
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            verify_files(&root, stage);
            assert_eq!(read(&root, "HEAD.json").unwrap(), canonical(&stage.head));
            observations.push(json!({"grouped":grouped,"milliseconds":elapsed,
                "fullsync_calls_including_post_rename":if grouped {2} else {2*(stage.loose.len()+2)+1}}));
        }
    }
    json!(observations)
}

fn spec(root: &File) -> (Value, SelectedStage) {
    let value: Value = serde_json::from_slice(&read(root, "spec.json").unwrap()).unwrap();
    let stage = SelectedStage {
        head: value["head"].clone(),
        commit: value["commit"].clone(),
        loose: serde_json::from_value(value["loose"].clone()).unwrap(),
    };
    (value["old"].clone(), stage)
}
#[test]
fn grouped_select_child() {
    let Ok(cut) = std::env::var("PHOTARA_GROUP_CUT") else {
        return;
    };
    let scope = Scope::load().unwrap();
    let group = std::env::var("PHOTARA_GROUP_NAME").unwrap();
    assert!(group.starts_with("grouped-select-") && !group.contains('/'));
    let parent =
        File::from(openat(&scope.mount, group.as_str(), DIRECTORY, Mode::empty()).unwrap());
    let (old, stage) = spec(&parent);
    let cut: usize = cut.parse().unwrap();
    let root =
        File::from(openat(&parent, format!("process-{cut}"), DIRECTORY, Mode::empty()).unwrap());
    let expected: Pin = serde_json::from_str(&std::env::var("PHOTARA_GROUP_PIN").unwrap()).unwrap();
    assert_eq!(pin(&root).unwrap(), expected);
    let mut index = 0;
    publish(&root, &old, &stage, |_, _| {
        if index == cut {
            std::process::exit(83)
        }
        index += 1;
        Ok(())
    })
    .unwrap();
    panic!("cut must be reached");
}
#[test]
#[ignore = "owned controller APFS qualification only; retains all artifacts, no review package access"]
fn grouped_select_qualification() {
    let scope = Scope::load().unwrap();
    let mode = std::env::var("PHOTARA_GROUP_MODE").unwrap_or_else(|_| "matrix".into());
    if mode == "verify" {
        let group = std::env::var("PHOTARA_GROUP_NAME").unwrap();
        assert!(group.starts_with("grouped-select-") && !group.contains('/'));
        let parent =
            File::from(openat(&scope.mount, group.as_str(), DIRECTORY, Mode::empty()).unwrap());
        let (old, stage) = spec(&parent);
        for entry in std::fs::read_dir(scope.path.parent().unwrap().join(&group)).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            let root =
                File::from(openat(&parent, entry.file_name(), DIRECTORY, Mode::empty()).unwrap());
            verify_files(&root, &stage);
            assert_eq!(read(&root, "HEAD.json").unwrap(), canonical(&stage.head));
        }
        assert_ne!(canonical(&old), canonical(&stage.head));
        println!(
            "{}",
            json!({"group":group,"remount_exact_bytes":true,"generation":scope.generation})
        );
        return;
    }
    assert_eq!(mode, "matrix");
    let name = format!("grouped-select-{}", uuid::Uuid::new_v4());
    let parent = directory(&scope.mount, &name);
    let (old, stage) = fixture();
    create(
        &parent,
        "spec.json",
        &canonical(&json!({"old":old,"head":stage.head,"commit":stage.commit,"loose":stage.loose})),
    )
    .unwrap();
    barrier(&parent).unwrap();
    let timings = comparison(&parent, &old, &stage);
    let failures = matrix(&parent, &old, &stage);
    for cut in 0..trace(&stage).len() {
        let root = directory(&parent, &format!("process-{cut}"));
        create(&root, "HEAD.json", &canonical(&old)).unwrap();
        barrier(&root).unwrap();
        let status=Command::new(std::env::current_exe().unwrap())
            .args(["--exact","package::v1_3::repeatable::disposable::native::selection_group::tests::grouped_select_child"])
            .env("PHOTARA_GROUP_NAME",&name).env("PHOTARA_GROUP_CUT",cut.to_string())
            .env("PHOTARA_GROUP_PIN",serde_json::to_string(&pin(&root).unwrap()).unwrap())
            .status().unwrap();
        assert_eq!(status.code(), Some(83));
        reconcile(&root, &old, &stage);
    }
    // A descriptor from the controller's host filesystem cannot enter this group.
    let foreign = Installed {
        name: "spec.json".into(),
        file: scope.scratch.try_clone().unwrap(),
        identity: pin(&scope.scratch).unwrap(),
    };
    assert!(foreign.check(&parent).is_err());
    barrier(&parent).unwrap();
    barrier(&scope.mount).unwrap();
    println!(
        "{}",
        json!({"profile":scope.assessment.report(),"timings":timings,"group":name,"file_count":stage.loose.len()+2,"failure_cases":failures,"process_cuts":trace(&stage).len(),"different_device_refused":true,"trace":trace(&stage).iter().map(|e|format!("{e:?}")).collect::<Vec<_>>(),"selected_head_sha256":hash(&canonical(&stage.head)),"retained":true,"qualified":false})
    );
}
