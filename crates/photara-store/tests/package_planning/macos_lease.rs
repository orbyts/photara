//! Disposable independent-process advisory locking, not a qualified adapter.
use super::*;
use rustix::fs::{FlockOperation, flock};
use std::{
    fs::File,
    os::unix::fs::MetadataExt as _,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Holder(Child);
impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn identity(root: &Path, lock: &File) -> [u64; 4] {
    let directory = root.metadata().unwrap();
    let file = lock.metadata().unwrap();
    assert!(file.is_file());
    assert_eq!(file.nlink(), 1);
    [directory.dev(), directory.ino(), file.dev(), file.ino()]
}

fn try_lock(file: &File) -> Result<(), LockFailure> {
    flock(file, FlockOperation::NonBlockingLockExclusive).map_err(|error| {
        if error == rustix::io::Errno::WOULDBLOCK || error == rustix::io::Errno::AGAIN {
            LockFailure::WriterBusy
        } else {
            LockFailure::Io(IoFailure::NotPerformed(WritePhase::Lock))
        }
    })
}

#[test]
fn lease_child() {
    let Ok(mode) = std::env::var("PHOTARA_PS2_LEASE_MODE") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var("PHOTARA_PS2_LEASE_ROOT").unwrap());
    let expected: [u64; 4] =
        serde_json::from_str(&std::env::var("PHOTARA_PS2_LEASE_IDENTITY").unwrap()).unwrap();
    let lock = File::open(root.join(".writer-lock")).unwrap();
    assert_eq!(identity(&root, &lock), expected);
    let result = try_lock(&lock);
    if mode == "busy" {
        assert_eq!(result, Err(LockFailure::WriterBusy));
        fs::write(
            std::env::var("PHOTARA_PS2_LEASE_READY").unwrap(),
            mode.as_bytes(),
        )
        .unwrap();
        return;
    }
    result.unwrap();
    let original = build(add_history);
    for name in ["manifest.json", "HEAD.json"] {
        assert_eq!(fs::read(root.join(name)).unwrap(), original[name]);
    }
    package::v1_1::validate_directory(&root, PackageLimits::default()).unwrap();
    assert_eq!(identity(&root, &lock), expected);
    if mode == "hold" {
        fs::write(std::env::var("PHOTARA_PS2_LEASE_READY").unwrap(), b"").unwrap();
        thread::sleep(Duration::from_secs(30));
        panic!("holder outlived parent protocol");
    }
    assert_eq!(mode, "acquire");
    fs::write(
        std::env::var("PHOTARA_PS2_LEASE_READY").unwrap(),
        mode.as_bytes(),
    )
    .unwrap();
}

fn child(root: &Path, ready: &Path, expected: &[u64; 4], mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "planning::macos_lease::lease_child"])
        .env("PHOTARA_PS2_LEASE_MODE", mode)
        .env("PHOTARA_PS2_LEASE_ROOT", root)
        .env("PHOTARA_PS2_LEASE_READY", ready)
        .env(
            "PHOTARA_PS2_LEASE_IDENTITY",
            serde_json::to_string(expected).unwrap(),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    command
}

fn run_client(root: &Path, ready: &Path, expected: &[u64; 4], mode: &str) {
    let completed = ready.with_extension(mode);
    let mut process = Holder(child(root, &completed, expected, mode).spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process.0.try_wait().unwrap() {
            assert!(status.success(), "{mode} helper failed: {status}");
            assert_eq!(fs::read(&completed).unwrap(), mode.as_bytes());
            return;
        }
        assert!(Instant::now() < deadline, "{mode} helper timed out");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn independent_holder_busy_then_exit_reacquires_unchanged_package() {
    let original = build(add_history);
    let root = materialize(&original);
    let control = tempfile::tempdir().unwrap();
    let ready = control.path().join("ready");
    let lock = File::create_new(root.path().join(".writer-lock")).unwrap();
    let expected = identity(root.path(), &lock);
    drop(lock);
    let mut holder = Holder(
        child(root.path(), &ready, &expected, "hold")
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        assert!(
            holder.0.try_wait().unwrap().is_none(),
            "holder exited early"
        );
        assert!(Instant::now() < deadline, "holder readiness timed out");
        thread::sleep(Duration::from_millis(5));
    }
    run_client(root.path(), &ready, &expected, "busy");
    holder.0.kill().unwrap();
    assert!(!holder.0.wait().unwrap().success());
    run_client(root.path(), &ready, &expected, "acquire");
    for (name, bytes) in original {
        assert_eq!(fs::read(root.path().join(name)).unwrap(), bytes);
    }
    assert_eq!(fs::read(root.path().join(".writer-lock")).unwrap(), b"");
}
