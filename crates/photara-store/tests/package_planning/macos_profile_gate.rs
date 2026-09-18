//! Candidate primitive engineering only. Never constructs production admission.
//! The companion 344-case subprocess matrix supplies process-cut observations.
use super::*;
use rustix::fs::{
    FlockOperation, Mode, OFlags, RenameFlags, fcntl_fullfsync, flock, fstatfs, fsync, open,
    openat, renameat, renameat_with,
};
use std::{
    fs::File,
    io::{Read as _, Write as _},
    os::unix::fs::{MetadataExt as _, symlink},
    path::{Path, PathBuf},
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const READ: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC)
    .union(OFlags::NONBLOCK);
type Identity = [u64; 2];

fn identity(file: &File) -> Identity {
    let metadata = file.metadata().unwrap();
    [metadata.dev(), metadata.ino()]
}

fn walk(root: &File, path: &str) -> std::io::Result<File> {
    let mut directory = root.try_clone()?;
    if path == "." {
        return Ok(directory);
    }
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        directory = File::from(openat(&directory, component, DIRECTORY, Mode::empty())?);
    }
    Ok(directory)
}

fn regular(root: &File, name: &str) -> std::io::Result<File> {
    let file = File::from(openat(root, name, READ, Mode::empty())?);
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(file)
}

fn bytes(mut file: File) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

struct Fixture {
    temporary: tempfile::TempDir,
    path: PathBuf,
    plan: Box<CheckpointPlan>,
}
impl Fixture {
    fn new() -> Self {
        let base = verified(build(add_history));
        let plan =
            checkpoint(run(&base, &rename(&base, "Namespace and barrier gate"), 36300).unwrap());
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("parent/package");
        for (name, data) in owned(base.files()) {
            let target = path.join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, data).unwrap();
        }
        File::create_new(path.join(".writer-lock")).unwrap();
        fs::create_dir(temporary.path().join("outside")).unwrap();
        fs::write(temporary.path().join("outside/sentinel"), b"never changed").unwrap();
        Self {
            temporary,
            path,
            plan,
        }
    }
}

struct Pins {
    grand: File,
    parent: File,
    root: File,
    lock: File,
    directories: Vec<(String, Identity)>,
    head: Vec<u8>,
    manifest: Vec<u8>,
}
impl Pins {
    fn acquire(fixture: &Fixture) -> Self {
        let grand = File::from(open(fixture.temporary.path(), DIRECTORY, Mode::empty()).unwrap());
        let parent = walk(&grand, "parent").unwrap();
        let root = walk(&parent, "package").unwrap();
        let lock = regular(&root, ".writer-lock").unwrap();
        flock(&lock, FlockOperation::NonBlockingLockExclusive).unwrap();
        let mut paths = BTreeSet::new();
        for file in fixture.plan.immutable_files() {
            let mut prefix = String::new();
            let (parent, _) = file.name().rsplit_once('/').unwrap();
            for component in parent.split('/') {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(component);
                paths.insert(prefix.clone());
            }
        }
        let directories = paths
            .into_iter()
            .map(|path| {
                let pin = identity(&walk(&root, &path).unwrap());
                (path, pin)
            })
            .collect();
        let head = bytes(regular(&root, "HEAD.json").unwrap()).unwrap();
        let manifest = bytes(regular(&root, "manifest.json").unwrap()).unwrap();
        Self {
            grand,
            parent,
            root,
            lock,
            directories,
            head,
            manifest,
        }
    }

    fn recheck(&self) -> std::io::Result<()> {
        let changed = || std::io::Error::from(std::io::ErrorKind::InvalidData);
        let parent = walk(&self.grand, "parent")?;
        let root = walk(&parent, "package")?;
        let lock = regular(&root, ".writer-lock")?;
        if identity(&parent) != identity(&self.parent)
            || identity(&root) != identity(&self.root)
            || identity(&lock) != identity(&self.lock)
            || self.lock.metadata()?.nlink() != 1
            || identity(&root)[0] != identity(&parent)[0]
        {
            return Err(changed());
        }
        for (path, expected) in &self.directories {
            let current = identity(&walk(&root, path)?);
            if &current != expected || current[0] != identity(&root)[0] {
                return Err(changed());
            }
        }
        if bytes(regular(&root, "HEAD.json")?)? != self.head
            || bytes(regular(&root, "manifest.json")?)? != self.manifest
        {
            return Err(changed());
        }
        Ok(())
    }
}

fn inventory(path: &Path, prefix: &str, found: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{prefix}{}", entry.file_name().to_str().unwrap());
        let kind = entry.file_type().unwrap();
        if kind.is_symlink() {
            found.insert(
                name,
                format!("symlink:{}", fs::read_link(entry.path()).unwrap().display()),
            );
        } else if kind.is_dir() {
            inventory(&entry.path(), &format!("{name}/"), found);
        } else {
            found.insert(name, hash(&fs::read(entry.path()).unwrap()));
        }
    }
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Explicit namespace substitution matrix"
)]
fn namespace_and_lock_substitution_refuse_before_output_and_preserve_evidence() {
    for attack in [
        "parent",
        "root-copy",
        "root-symlink",
        "child-copy",
        "child-symlink",
        "lock-copy",
        "lock-symlink",
        "lock-hardlink",
        "head",
        "head-hardlink",
        "manifest",
    ] {
        let fixture = Fixture::new();
        let pins = Pins::acquire(&fixture);
        pins.recheck().unwrap();
        let mut recovery = fixture.path.clone();
        match attack {
            "parent" => {
                fs::rename(
                    fixture.temporary.path().join("parent"),
                    fixture.temporary.path().join("old-parent"),
                )
                .unwrap();
                fs::create_dir(fixture.temporary.path().join("parent")).unwrap();
                recovery = fixture.temporary.path().join("old-parent/package");
            }
            "root-copy" | "root-symlink" => {
                recovery = fixture.path.with_file_name("old-package");
                fs::rename(&fixture.path, &recovery).unwrap();
                if attack == "root-symlink" {
                    symlink(fixture.temporary.path().join("outside"), &fixture.path).unwrap();
                } else {
                    copy_fixture_tree(&recovery, &fixture.path);
                }
            }
            "child-copy" | "child-symlink" => {
                fs::rename(
                    fixture.path.join("objects/json"),
                    fixture.path.join("objects/old-json"),
                )
                .unwrap();
                if attack == "child-symlink" {
                    symlink(
                        fixture.temporary.path().join("outside"),
                        fixture.path.join("objects/json"),
                    )
                    .unwrap();
                } else {
                    copy_fixture_tree(
                        &fixture.path.join("objects/old-json"),
                        &fixture.path.join("objects/json"),
                    );
                }
            }
            "lock-copy" | "lock-symlink" => {
                fs::rename(
                    fixture.path.join(".writer-lock"),
                    fixture.path.join(".old-lock"),
                )
                .unwrap();
                if attack == "lock-symlink" {
                    symlink(
                        fixture.temporary.path().join("outside/sentinel"),
                        fixture.path.join(".writer-lock"),
                    )
                    .unwrap();
                } else {
                    File::create_new(fixture.path.join(".writer-lock")).unwrap();
                }
            }
            "lock-hardlink" => fs::hard_link(
                fixture.path.join(".writer-lock"),
                fixture.path.join(".alias-lock"),
            )
            .unwrap(),
            "head-hardlink" => fs::hard_link(
                fixture.path.join("HEAD.json"),
                fixture.path.join(".alias-head"),
            )
            .unwrap(),
            "head" => fs::write(fixture.path.join("HEAD.json"), b"different HEAD").unwrap(),
            "manifest" => {
                fs::write(fixture.path.join("manifest.json"), b"different manifest").unwrap();
            }
            _ => unreachable!(),
        }
        let mut before = BTreeMap::new();
        inventory(fixture.temporary.path(), "", &mut before);
        assert!(pins.recheck().is_err(), "accepted {attack}");
        let mut after = BTreeMap::new();
        inventory(fixture.temporary.path(), "", &mut after);
        assert_eq!(before, after, "refusal mutated {attack}");
        assert_eq!(
            fs::read(fixture.temporary.path().join("outside/sentinel")).unwrap(),
            b"never changed"
        );
        // Independently validate old closure whenever the attack left that closure intact.
        if !matches!(
            attack,
            "child-symlink" | "head" | "head-hardlink" | "manifest"
        ) {
            package::v1_1::validate_directory(&recovery, PackageLimits::default()).unwrap();
        }
        println!(
            "{}",
            json!({"namespace_attack":attack,"refused":true,"evidence_unchanged":true,"qualified":false})
        );
    }
}

#[test]
fn held_lease_blocks_a_separately_opened_description_until_release() {
    let fixture = Fixture::new();
    let pins = Pins::acquire(&fixture);
    let contender = regular(&pins.root, ".writer-lock").unwrap();
    assert_eq!(identity(&contender), identity(&pins.lock));
    let result = flock(&contender, FlockOperation::NonBlockingLockExclusive);
    assert!(
        result == Err(rustix::io::Errno::AGAIN) || result == Err(rustix::io::Errno::WOULDBLOCK)
    );
    drop(pins);
    flock(&contender, FlockOperation::NonBlockingLockExclusive).unwrap();
    package::v1_1::validate_directory(&fixture.path, PackageLimits::default()).unwrap();
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    NoSpaceBefore,
    IoAfter,
    UnknownAfter,
}

#[test]
fn exclusive_publication_refuses_symlink_hardlink_and_directory_collisions() {
    for collision in ["symlink", "hardlink", "directory"] {
        let fixture = Fixture::new();
        let pins = Pins::acquire(&fixture);
        let source = fixture.path.join(".collision-source");
        let target = fixture.path.join(".collision-target");
        fs::write(&source, b"candidate bytes").unwrap();
        let sentinel = fixture.temporary.path().join("outside/sentinel");
        match collision {
            "symlink" => symlink(&sentinel, &target).unwrap(),
            "hardlink" => fs::hard_link(&sentinel, &target).unwrap(),
            "directory" => fs::create_dir(&target).unwrap(),
            _ => unreachable!(),
        }
        let mut before = BTreeMap::new();
        inventory(fixture.temporary.path(), "", &mut before);
        assert_eq!(
            renameat_with(
                &pins.root,
                ".collision-source",
                &pins.root,
                ".collision-target",
                RenameFlags::NOREPLACE
            ),
            Err(rustix::io::Errno::EXIST)
        );
        let mut after = BTreeMap::new();
        inventory(fixture.temporary.path(), "", &mut after);
        assert_eq!(before, after);
        assert_eq!(fs::read(sentinel).unwrap(), b"never changed");
    }
}

#[test]
fn advisory_lease_does_not_prevent_noncooperating_writes() {
    let fixture = Fixture::new();
    let pins = Pins::acquire(&fixture);
    pins.recheck().unwrap();
    // Deliberately ignores the held lock: documents its limitation, not exclusion.
    fs::write(fixture.path.join("HEAD.json"), b"noncooperating writer").unwrap();
    assert_eq!(
        fs::read(fixture.path.join("HEAD.json")).unwrap(),
        b"noncooperating writer"
    );
    assert!(pins.recheck().is_err());
    assert_eq!(
        fs::read(fixture.path.join("HEAD.json")).unwrap(),
        b"noncooperating writer"
    );
}

struct Attempt {
    target: Option<(usize, Fault)>,
    trace: Vec<Value>,
}
impl Attempt {
    fn step(
        &mut self,
        label: &str,
        action: impl FnOnce() -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let index = self.trace.len();
        if matches!(self.target, Some((target, Fault::NoSpaceBefore)) if target == index) {
            self.trace.push(json!({"step":index,"operation":label,"injection":"ENOSPC-before","outcome":"NotPerformed"}));
            return Err(rustix::io::Errno::NOSPC.into());
        }
        action()?;
        if let Some((target, fault @ (Fault::IoAfter | Fault::UnknownAfter))) = self.target
            && target == index
        {
            self.trace.push(json!({"step":index,"operation":label,"injection":format!("{fault:?}"),"outcome":"OutcomeUnknown"}));
            return Err(rustix::io::Errno::IO.into());
        }
        self.trace
            .push(json!({"step":index,"operation":label,"errno":null}));
        Ok(())
    }

    fn publish(&mut self, fixture: &Fixture, pins: &Pins) -> std::io::Result<()> {
        pins.recheck()?;
        for (index, planned) in fixture.plan.immutable_files().iter().enumerate() {
            let (parent, name) = planned.name().rsplit_once('/').unwrap();
            self.file(
                pins,
                parent,
                &format!(".gate-{index}.tmp"),
                name,
                planned.bytes(),
                false,
            )?;
        }
        pins.recheck()?;
        self.file(
            pins,
            ".",
            ".gate-head.tmp",
            "HEAD.json",
            fixture.plan.candidate().token().head_bytes(),
            true,
        )?;
        self.step("independent candidate verification", || {
            package::v1_1::validate_directory(&fixture.path, PackageLimits::default()).unwrap();
            assert_eq!(
                fs::read(fixture.path.join("HEAD.json"))?,
                fixture.plan.candidate().token().head_bytes()
            );
            Ok(())
        })
    }

    fn file(
        &mut self,
        pins: &Pins,
        path: &str,
        temporary: &str,
        target: &str,
        data: &[u8],
        replace: bool,
    ) -> std::io::Result<()> {
        let parent = walk(&pins.root, path)?;
        if identity(&parent)[0] != identity(&pins.root)[0] {
            return Err(rustix::io::Errno::XDEV.into());
        }
        let mut handle = None;
        self.step(&format!("create/write {target}"), || {
            let mut file = File::from(openat(
                &parent,
                temporary,
                OFlags::CREATE | OFlags::EXCL | OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )?);
            file.write_all(data)?;
            handle = Some(file);
            Ok(())
        })?;
        let file = handle.unwrap();
        self.step(&format!("fsync(file:{target})"), || {
            fsync(&file).map_err(Into::into)
        })?;
        self.step(&format!("F_FULLFSYNC(file:{target})"), || {
            fcntl_fullfsync(&file).map_err(Into::into)
        })?;
        self.step(
            &format!(
                "{} {target}",
                if replace { "replace" } else { "RENAME_EXCL" }
            ),
            || {
                if replace {
                    renameat(&parent, temporary, &parent, target)
                } else {
                    renameat_with(&parent, temporary, &parent, target, RenameFlags::NOREPLACE)
                }
                .map_err(Into::into)
            },
        )?;
        self.step(&format!("fsync(directory:{path})"), || {
            fsync(&parent).map_err(Into::into)
        })?;
        self.step(&format!("F_FULLFSYNC(directory:{path})"), || {
            fcntl_fullfsync(&parent).map_err(Into::into)
        })
    }
}

#[test]
#[ignore = "explicit local-APFS ENOSPC and post-effect barrier observations; profile remains unqualified"]
fn observe_candidate_profile_faults_with_pinned_namespace() {
    let fixture = Fixture::new();
    let pins = Pins::acquire(&fixture);
    let facts = fstatfs(&pins.root).unwrap();
    let filesystem: Vec<u8> = facts
        .f_fstypename
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| u8::try_from(*b).unwrap())
        .collect();
    assert_eq!(filesystem, b"apfs");
    assert_ne!(facts.f_flags & 0x1000, 0);
    let mut control = Attempt {
        target: None,
        trace: vec![],
    };
    control.publish(&fixture, &pins).unwrap();
    println!(
        "{}",
        json!({"candidate_profile":"test-only-local-APFS-per-file-and-directory-fullsync","device":identity(&pins.root)[0],"mount_flags":facts.f_flags,"filesystem":"apfs","qualified":false,"provider_exclusion":"unassessed","directory_persistence_ordering":"not established by syscall success","power_loss_tested":false,"control_trace":control.trace})
    );
    let boundaries = control.trace.len();
    let head_index = control
        .trace
        .iter()
        .position(|event| event["operation"] == "replace HEAD.json")
        .unwrap();
    for index in 0..boundaries {
        for fault in [Fault::NoSpaceBefore, Fault::IoAfter, Fault::UnknownAfter] {
            let fixture = Fixture::new();
            let pins = Pins::acquire(&fixture);
            let mut attempt = Attempt {
                target: Some((index, fault)),
                trace: vec![],
            };
            assert!(attempt.publish(&fixture, &pins).is_err());
            assert_eq!(attempt.trace.len(), index + 1);
            let candidate = index > head_index
                || (index == head_index && !matches!(fault, Fault::NoSpaceBefore));
            assert_eq!(
                fs::read(fixture.path.join("HEAD.json")).unwrap(),
                if candidate {
                    fixture.plan.candidate().token().head_bytes()
                } else {
                    fixture.plan.expected_head().head_bytes()
                }
            );
            let read =
                package::v1_1::validate_directory(&fixture.path, PackageLimits::default()).unwrap();
            assert_eq!(read.commits.len(), if candidate { 2 } else { 1 });
            let mut retained = BTreeMap::new();
            inventory(fixture.temporary.path(), "", &mut retained);
            println!(
                "{}",
                json!({"step":index,"injection":format!("{fault:?}"),"observed_head":if candidate {"candidate"} else {"old"},"independent_closure_valid":true,"acknowledged":false,"cleanup_performed":false,"retained_inventory":retained,"trace":attempt.trace})
            );
        }
    }
}
