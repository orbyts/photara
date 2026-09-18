//! Explicit synthetic process-fault observations; no production admission or profile.
use super::*;
use rustix::fs::{
    Mode, OFlags, RenameFlags, fcntl_fullfsync, fstatfs, fsync, mkdirat, open, openat, renameat,
    renameat_with,
};
use std::{
    fs::File,
    io::Write as _,
    os::unix::fs::MetadataExt as _,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const RECOVERY: &str = ".ps2-fault-recovery";

#[derive(Debug)]
enum Action {
    Directory,
    Write(String, Vec<u8>),
    Barrier(String, bool, bool),   // path, directory, full-sync
    Publish(String, String, bool), // source, destination, replace HEAD
    Validate,
}

fn barrier_steps(steps: &mut Vec<Action>, path: &str, directory: bool) {
    for full in [false, true] {
        steps.push(Action::Barrier(path.into(), directory, full));
    }
}

fn fixture() -> (VerifiedClosure, Box<CheckpointPlan>) {
    let base = verified(build(add_history));
    let plan = checkpoint(run(&base, &rename(&base, "Barrier fault matrix"), 36100).unwrap());
    (base, plan)
}

fn sequence(plan: &CheckpointPlan) -> Vec<Action> {
    let mut steps = vec![Action::Directory];
    barrier_steps(&mut steps, ".", true);
    let intent = canon(&json!({
        "fixture_only":true,
        "operation_id":id(30001),
        "write_id":id(36100),
        "commit_id":id(36101),
        "old_head_sha256":hash(plan.expected_head().head_bytes()),
        "candidate_head_sha256":hash(plan.candidate().token().head_bytes()),
        "request_digest":plan.receipt().request_digest()
    }));
    for (name, bytes) in [
        ("intent", intent.as_slice()),
        (
            "journal",
            b"accepted synthetic RenameGraph; pending checkpoint".as_slice(),
        ),
    ] {
        let path = format!("{RECOVERY}/{name}");
        steps.push(Action::Write(path.clone(), bytes.to_vec()));
        barrier_steps(&mut steps, &path, false);
        barrier_steps(&mut steps, RECOVERY, true);
    }
    for (index, planned) in plan.immutable_files().iter().enumerate() {
        let (parent, _) = planned.name().rsplit_once('/').unwrap();
        let temporary = format!("{parent}/.ps2-fault-{index}.tmp");
        steps.push(Action::Write(temporary.clone(), planned.bytes().to_vec()));
        barrier_steps(&mut steps, &temporary, false);
        steps.push(Action::Publish(temporary, planned.name().into(), false));
        barrier_steps(&mut steps, parent, true);
    }
    let temporary = ".ps2-fault-head.tmp";
    steps.push(Action::Write(
        temporary.into(),
        plan.candidate().token().head_bytes().to_vec(),
    ));
    barrier_steps(&mut steps, temporary, false);
    steps.push(Action::Publish(temporary.into(), "HEAD.json".into(), true));
    barrier_steps(&mut steps, ".", true);
    steps.push(Action::Validate);
    let receipt = format!("{RECOVERY}/receipt");
    steps.push(Action::Write(
        receipt.clone(),
        b"synthetic publication receipt; not Saved".to_vec(),
    ));
    barrier_steps(&mut steps, &receipt, false);
    barrier_steps(&mut steps, RECOVERY, true);
    steps
}

fn parent(root: &File, path: &str) -> (File, String) {
    let (directory, name) = path.rsplit_once('/').unwrap_or((".", path));
    (
        File::from(openat(root, directory, DIRECTORY, Mode::empty()).unwrap()),
        name.into(),
    )
}

impl Action {
    fn label(&self) -> String {
        match self {
            Self::Directory => format!("mkdir {RECOVERY}"),
            Self::Write(path, _) => format!("create-exclusive/write-all {path}"),
            Self::Barrier(path, directory, full) => format!(
                "{}({}:{path})",
                if *full { "F_FULLFSYNC" } else { "fsync" },
                if *directory { "directory" } else { "file" }
            ),
            Self::Publish(_, destination, replace) => format!(
                "{} {destination}",
                if *replace { "replace" } else { "RENAME_EXCL" }
            ),
            Self::Validate => "independent candidate closure validation".into(),
        }
    }

    fn execute(&self, root: &File, path: &Path, plan: &CheckpointPlan) -> std::io::Result<()> {
        match self {
            Self::Directory => mkdirat(root, RECOVERY, Mode::RWXU).map_err(Into::into),
            Self::Write(name, bytes) => {
                let (parent, name) = parent(root, name);
                let mut file = File::from(openat(
                    &parent,
                    name,
                    OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::RDWR
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )?);
                file.write_all(bytes)
            }
            Self::Barrier(name, directory, full) => {
                let flags = if *directory {
                    DIRECTORY
                } else {
                    OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC
                };
                let file = File::from(openat(root, name.as_str(), flags, Mode::empty())?);
                assert_eq!(file.metadata()?.dev(), root.metadata()?.dev());
                if *full {
                    fcntl_fullfsync(&file)
                } else {
                    fsync(&file)
                }
                .map_err(Into::into)
            }
            Self::Publish(source, target, replace) => {
                let (source_parent, source) = parent(root, source);
                let (target_parent, target) = parent(root, target);
                if *replace {
                    renameat(&source_parent, source, &target_parent, target)
                } else {
                    renameat_with(
                        &source_parent,
                        source,
                        &target_parent,
                        target,
                        RenameFlags::NOREPLACE,
                    )
                }
                .map_err(Into::into)
            }
            Self::Validate => {
                let read =
                    package::v1_1::validate_directory(path, PackageLimits::default()).unwrap();
                assert_eq!(
                    read.commits[0].commit_id.as_uuid(),
                    plan.ids().commit_id.uuid()
                );
                assert_eq!(
                    fs::read(path.join("HEAD.json"))?,
                    plan.candidate().token().head_bytes()
                );
                Ok(())
            }
        }
    }
}

fn event(trace: &mut File, value: &Value) {
    writeln!(trace, "{value}").unwrap();
    trace.flush().unwrap(); // Process-exit observation only, not a persistence claim.
}

#[test]
fn barrier_fault_child() {
    let Ok(path) = std::env::var("PHOTARA_PS2_BARRIER_ROOT") else {
        return;
    };
    let path = Path::new(&path);
    let control = std::env::var("PHOTARA_PS2_BARRIER_CONTROL").unwrap();
    let mode = std::env::var("PHOTARA_PS2_BARRIER_MODE").unwrap();
    let target: usize = std::env::var("PHOTARA_PS2_BARRIER_STEP")
        .unwrap()
        .parse()
        .unwrap();
    // Parent supplies a private synthetic fixture and identity pins; no public adapter.
    let expected: [u64; 2] =
        serde_json::from_str(&std::env::var("PHOTARA_PS2_BARRIER_PIN").unwrap()).unwrap();
    let root = File::from(open(path, DIRECTORY, Mode::empty()).unwrap());
    let pin = root.metadata().unwrap();
    assert_eq!([pin.dev(), pin.ino()], expected);
    let facts = fstatfs(&root).unwrap();
    let filesystem: Vec<u8> = facts
        .f_fstypename
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| u8::try_from(*b).unwrap())
        .collect();
    assert_eq!(filesystem, b"apfs");
    assert_ne!(facts.f_flags & 0x1000, 0, "refuse nonlocal fixture");
    let (base, plan) = fixture();
    assert_eq!(
        fs::read(path.join("HEAD.json")).unwrap(),
        base.token().head_bytes()
    );
    let mut trace = File::create_new(Path::new(&control).join("trace.jsonl")).unwrap();
    event(
        &mut trace,
        &json!({"event":"started", "mode":mode,"step":target,"filesystem":"apfs","mount_flags":facts.f_flags,"device":pin.dev(),"inode":pin.ino()}),
    );
    for (index, action) in sequence(&plan).iter().enumerate() {
        event(
            &mut trace,
            &json!({"event":"before", "step":index,"operation":action.label()}),
        );
        if index == target {
            if mode == "exit-before" {
                std::process::exit(81);
            }
            if mode.starts_with("before-") {
                let errno = match mode.as_str() {
                    "before-unsupported" => rustix::io::Errno::NOTSUP,
                    "before-interrupted" => rustix::io::Errno::INTR,
                    "before-io" => rustix::io::Errno::IO,
                    _ => panic!("unknown injection"),
                };
                event(
                    &mut trace,
                    &json!({"event":"injected", "step":index,"outcome":"NotPerformed","errno":errno.raw_os_error()}),
                );
                return;
            }
        }
        if let Err(error) = action.execute(&root, path, &plan) {
            event(
                &mut trace,
                &json!({"event":"actual-error", "step":index,"errno":error.raw_os_error(),"error":error.to_string(),"outcome":"OutcomeUnknown"}),
            );
            return; // No fallback primitive and no further mutation.
        }
        event(
            &mut trace,
            &json!({"event":"after", "step":index,"operation":action.label(),"errno":null}),
        );
        if index == target {
            if mode == "exit-after" {
                std::process::exit(82);
            }
            if mode == "after-io" || mode == "unknown" {
                event(
                    &mut trace,
                    &json!({"event":"injected", "step":index,"outcome":"OutcomeUnknown","errno":if mode == "after-io" {Some(rustix::io::Errno::IO.raw_os_error())} else {None}}),
                );
                return;
            }
        }
    }
    event(
        &mut trace,
        &json!({"event":"synthetic-acknowledgement", "qualified":false,"saved_claim":false}),
    );
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn recovery_classification(path: &Path, plan: &CheckpointPlan) -> &'static str {
    let Ok(head) = fs::read(path.join("HEAD.json")) else {
        return "frozen";
    };
    if package::v1_1::validate_directory(path, PackageLimits::default()).is_err() {
        return "frozen";
    }
    if head == plan.expected_head().head_bytes() {
        "old-selected"
    } else if head == plan.candidate().token().head_bytes() {
        "candidate-visible-awaiting-barrier-and-receipt-reconciliation"
    } else {
        "frozen"
    }
}

#[test]
fn independent_opener_freezes_unrelated_or_invalid_head_without_mutation() {
    let (base, plan) = fixture();
    let unrelated =
        checkpoint(run(&base, &rename(&base, "Unrelated valid checkpoint"), 36200).unwrap());
    let temporary = materialize(&owned(unrelated.candidate().files()));
    let mut before = BTreeMap::new();
    inventory(temporary.path(), "", &mut before);
    assert_eq!(recovery_classification(temporary.path(), &plan), "frozen");
    let mut after = BTreeMap::new();
    inventory(temporary.path(), "", &mut after);
    assert_eq!(after, before);
    fs::write(temporary.path().join("HEAD.json"), b"truncated").unwrap();
    before.clear();
    inventory(temporary.path(), "", &mut before);
    assert_eq!(recovery_classification(temporary.path(), &plan), "frozen");
    after.clear();
    inventory(temporary.path(), "", &mut after);
    assert_eq!(after, before);
}

fn inventory(path: &Path, prefix: &str, output: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{prefix}{}", entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            inventory(&entry.path(), &format!("{name}/"), output);
        } else {
            output.insert(name, hash(&fs::read(entry.path()).unwrap()));
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Explicit synthetic fault matrix and recovery assertions"
)]
fn scenario(
    step: usize,
    mode: &str,
    plan: &CheckpointPlan,
    original: &BTreeMap<String, Vec<u8>>,
) -> Value {
    let temporary = materialize(original);
    let control = tempfile::tempdir().unwrap();
    let sentinel = control.path().join("sentinel");
    fs::write(&sentinel, b"outside-package sentinel").unwrap();
    let pin = temporary.path().metadata().unwrap();
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "planning::macos_barrier_faults::barrier_fault_child",
            ])
            .env("PHOTARA_PS2_BARRIER_ROOT", temporary.path())
            .env("PHOTARA_PS2_BARRIER_CONTROL", control.path())
            .env("PHOTARA_PS2_BARRIER_MODE", mode)
            .env("PHOTARA_PS2_BARRIER_STEP", step.to_string())
            .env(
                "PHOTARA_PS2_BARRIER_PIN",
                serde_json::to_string(&[pin.dev(), pin.ino()]).unwrap(),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "child timeout at {step}/{mode}");
        thread::sleep(Duration::from_millis(5));
    };
    let trace: Vec<Value> = fs::read_to_string(control.path().join("trace.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        trace[0]["event"], "started",
        "child test must actually execute"
    );
    assert!(
        !trace.iter().any(|event| event["event"] == "actual-error"),
        "real syscall refused; no qualification or fallback: {trace:?}"
    );
    assert_eq!(
        status.code(),
        Some(match mode {
            "exit-before" => 81,
            "exit-after" => 82,
            _ => 0,
        })
    );
    let steps = sequence(plan);
    let head_step = steps
        .iter()
        .position(|step| matches!(step, Action::Publish(_, _, true)))
        .unwrap();
    let acted = !mode.starts_with("before-") && mode != "exit-before";
    let candidate = step > head_step || (step == head_step && acted);
    let observed_head = fs::read(temporary.path().join("HEAD.json")).unwrap();
    assert_eq!(
        observed_head,
        if candidate {
            plan.candidate().token().head_bytes()
        } else {
            &original["HEAD.json"]
        }
    );
    // Independent opener, after writer process death/return: full current reader closure.
    let read =
        package::v1_1::validate_directory(temporary.path(), PackageLimits::default()).unwrap();
    assert_eq!(read.commits.len(), if candidate { 2 } else { 1 });
    let recovery = recovery_classification(temporary.path(), plan);
    assert_eq!(
        recovery,
        if candidate {
            "candidate-visible-awaiting-barrier-and-receipt-reconciliation"
        } else {
            "old-selected"
        }
    );
    for (name, bytes) in original {
        if name != "HEAD.json" {
            assert_eq!(&fs::read(temporary.path().join(name)).unwrap(), bytes);
        }
    }
    assert_eq!(fs::read(&sentinel).unwrap(), b"outside-package sentinel");
    let current_pin = temporary.path().metadata().unwrap();
    assert_eq!(
        [current_pin.dev(), current_pin.ino()],
        [pin.dev(), pin.ino()]
    );
    let mut retained = BTreeMap::new();
    inventory(temporary.path(), "", &mut retained);
    // Every completed publication/write remains; no retry, cleanup or retirement.
    let completed_steps = if mode == "none" {
        steps.len()
    } else {
        step + usize::from(acted)
    };
    for action in steps.iter().take(completed_steps) {
        if let Action::Write(name, bytes) = action {
            let renamed = steps
                .iter()
                .take(completed_steps)
                .find_map(|action| match action {
                    Action::Publish(source, target, _) if source == name => Some(target),
                    _ => None,
                });
            assert_eq!(retained.get(renamed.unwrap_or(name)), Some(&hash(bytes)));
        }
    }
    let acknowledged = trace
        .iter()
        .any(|event| event["event"] == "synthetic-acknowledgement");
    assert_eq!(acknowledged, mode == "none");
    if mode != "none" {
        let last = trace.last().unwrap();
        assert_eq!(last["step"], step);
        assert!(!trace.iter().any(|event| {
            event["step"]
                .as_u64()
                .is_some_and(|index| index > step as u64)
        }));
    }
    json!({"step":step,"operation":steps.get(step).map(Action::label),"injection":mode,"exit_code":status.code(),"observed_head":if candidate {"candidate"} else {"old"},"head_digest":hash(&observed_head),"independent_closure_valid":true,"intent_retained":retained.contains_key(&format!("{RECOVERY}/intent")),"journal_retained":retained.contains_key(&format!("{RECOVERY}/journal")),"receipt_present":retained.contains_key(&format!("{RECOVERY}/receipt")),"acknowledged":acknowledged,"recovery":if mode == "none" {"synthetic-completed"} else {recovery},"cleanup_performed":false,"retained_inventory":retained,"trace":trace})
}

#[test]
#[ignore = "explicit macOS local-APFS subprocess/barrier matrix; not storage qualification"]
fn observe_barrier_fault_matrix() {
    let (base, plan) = fixture();
    let original = owned(base.files());
    let steps = sequence(&plan);
    let modes = [
        "before-unsupported",
        "before-interrupted",
        "before-io",
        "after-io",
        "unknown",
        "exit-before",
        "exit-after",
    ];
    println!(
        "{}",
        json!({"fixture":"PS2 disposable barrier fault matrix","qualified":false,"power_loss_tested":false,"provider_exclusion":"unassessed","directory_persistence_ordering":"not demonstrated","base_setup":"ordinary synthetic materialization; not qualified publication","trace_flush":"process-visible only","steps":steps.len(),"scenarios":steps.len()*modes.len()+1})
    );
    println!("{}", scenario(steps.len(), "none", &plan, &original));
    for step in 0..steps.len() {
        for mode in modes {
            println!("{}", scenario(step, mode, &plan, &original));
        }
    }
}
