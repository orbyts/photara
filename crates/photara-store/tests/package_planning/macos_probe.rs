//! Explicit disposable syscall observations, never a qualified storage profile.
use super::*;
use rustix::fs::{
    Mode, OFlags, RenameFlags, fcntl_fullfsync, fstatfs, fsync, open, openat, renameat,
    renameat_with,
};
use std::{fs::File, io::Write as _, os::unix::fs::MetadataExt as _, process::Command};

fn observe(trace: &mut Vec<Value>, operation: impl Into<String>, result: rustix::io::Result<()>) {
    trace.push(json!({"operation": operation.into(), "errno": result.err().map(rustix::io::Errno::raw_os_error)}));
}

fn barriers(trace: &mut Vec<Value>, target: &str, file: &File) {
    observe(trace, format!("fsync({target})"), fsync(file));
    observe(
        trace,
        format!("F_FULLFSYNC({target})"),
        fcntl_fullfsync(file),
    );
}

fn version(argument: &str) -> String {
    let result = Command::new("/usr/bin/sw_vers")
        .arg(argument)
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

fn create(parent: &File, name: &str, bytes: &[u8]) -> File {
    let mut file = File::from(
        openat(
            parent,
            name,
            OFlags::CREATE | OFlags::EXCL | OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .unwrap(),
    );
    file.write_all(bytes).unwrap();
    file
}

fn filesystem_name(facts: &rustix::fs::StatFs) -> String {
    String::from_utf8(
        facts
            .f_fstypename
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| u8::try_from(*c).unwrap())
            .collect(),
    )
    .unwrap()
}

#[test]
#[ignore = "explicit macOS disposable syscall probe; observations are not durability qualification"]
fn observe_local_barriers_and_exclusive_rename() {
    let base = verified(build(add_history));
    let plan = checkpoint(run(&base, &rename(&base, "Disposable syscall probe"), 36000).unwrap());
    let temporary = materialize(&owned(base.files()));
    // The sole root is allocated by this test. No arbitrary input path is accepted.
    let root = File::from(open(temporary.path(), DIRECTORY, Mode::empty()).unwrap());
    let facts = fstatfs(&root).unwrap();
    let filesystem = filesystem_name(&facts);
    let local = facts.f_flags & 0x0000_1000 != 0; // MNT_LOCAL, Apple SDK sys/mount.h.
    assert_eq!(filesystem, "apfs", "this explicit probe targets APFS only");
    assert!(local, "this explicit probe refuses nonlocal storage");
    let initial_pin = root.metadata().unwrap();
    let mut trace = vec![
        json!({"operation":"materialize synthetic base; open pinned root; fstatfs", "errno":null}),
    ];
    for (index, planned) in plan.immutable_files().iter().enumerate() {
        let (parent_name, name) = planned.name().rsplit_once('/').unwrap();
        let parent = File::from(openat(&root, parent_name, DIRECTORY, Mode::empty()).unwrap());
        assert_eq!(parent.metadata().unwrap().dev(), initial_pin.dev());
        let source = format!(".ps2-probe-{index}.tmp");
        let file = create(&parent, &source, planned.bytes());
        trace.push(json!({"operation":format!("create-exclusive/write-all immutable[{index}]"), "errno":null}));
        barriers(&mut trace, &format!("immutable[{index}]"), &file);
        let published = renameat_with(&parent, &source, &parent, name, RenameFlags::NOREPLACE);
        observe(
            &mut trace,
            format!("RENAME_EXCL immutable[{index}]"),
            published,
        );
        assert!(published.is_ok());
        barriers(&mut trace, &format!("parent[{index}]"), &parent);
        if index == 0 {
            let contender = create(&parent, ".ps2-probe-collision.tmp", b"must not replace");
            trace.push(
                json!({"operation":"create-exclusive/write-all collision source", "errno":null}),
            );
            barriers(&mut trace, "collision-source", &contender);
            let collision = renameat_with(
                &parent,
                ".ps2-probe-collision.tmp",
                &parent,
                name,
                RenameFlags::NOREPLACE,
            );
            observe(&mut trace, "RENAME_EXCL occupied immutable[0]", collision);
            assert_eq!(collision, Err(rustix::io::Errno::EXIST));
            assert_eq!(
                fs::read(temporary.path().join(planned.name())).unwrap(),
                planned.bytes()
            );
            assert_eq!(
                fs::read(
                    temporary
                        .path()
                        .join(parent_name)
                        .join(".ps2-probe-collision.tmp")
                )
                .unwrap(),
                b"must not replace"
            );
        }
    }
    let head = create(
        &root,
        ".ps2-probe-head.tmp",
        plan.candidate().token().head_bytes(),
    );
    trace.push(json!({"operation":"create-exclusive/write-all HEAD temporary", "errno":null}));
    barriers(&mut trace, "HEAD temporary", &head);
    let replaced = renameat(&root, ".ps2-probe-head.tmp", &root, "HEAD.json");
    observe(&mut trace, "renameat HEAD replacement", replaced);
    assert!(replaced.is_ok());
    barriers(&mut trace, "package root after HEAD", &root);
    let read =
        package::v1_1::validate_directory(temporary.path(), PackageLimits::default()).unwrap();
    assert_eq!(
        read.commits[0].commit_id.as_uuid(),
        plan.ids().commit_id.uuid()
    );
    assert_eq!(
        fs::read(temporary.path().join("HEAD.json")).unwrap(),
        plan.candidate().token().head_bytes()
    );
    trace.push(json!({"operation":"independent reader verifies exact candidate HEAD/closure", "errno":null}));
    let final_pin = root.metadata().unwrap();
    assert_eq!(
        (initial_pin.dev(), initial_pin.ino()),
        (final_pin.dev(), final_pin.ino())
    );
    println!("{}", serde_json::to_string_pretty(&json!({
        "macos":version("-productVersion"), "build":version("-buildVersion"),
        "architecture":std::env::consts::ARCH, "filesystem":filesystem,
        "mount_flags":facts.f_flags, "mnt_local":local, "immutable_files":plan.immutable_files().len(),
        "qualified":false, "provider_exclusion":"unassessed", "directory_persistence_ordering":"not demonstrated",
        "power_loss_tested":false, "trace":trace
    })).unwrap());
}
