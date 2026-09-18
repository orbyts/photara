//! Split-phase disposable harness. It never creates images, mounts or detaches.
//! Native mount identity is checked; image-to-volume association remains an
//! explicitly supplied experiment-controller assertion, never writer admission.
use super::*;
use rustix::fs::{
    FlockOperation, Mode, OFlags, RenameFlags, fcntl_fullfsync, flock, fstatfs, fsync, mkdirat,
    open, openat, renameat, renameat_with,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read as _, Write as _},
    os::unix::fs::{MetadataExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const PACKAGE: &str = "synthetic-package";
type Pin = [u64; 2];
type Result<T> = std::io::Result<T>;

fn invalid() -> std::io::Error {
    std::io::ErrorKind::InvalidData.into()
}
fn pin(file: &File) -> Result<Pin> {
    let m = file.metadata()?;
    Ok([m.dev(), m.ino()])
}
fn native(chars: &[std::ffi::c_char]) -> String {
    String::from_utf8(
        chars
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| u8::try_from(*c).unwrap())
            .collect(),
    )
    .unwrap()
}
fn read_regular(parent: &File, name: &str) -> Result<Vec<u8>> {
    let mut file = File::from(openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let meta = file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 {
        return Err(invalid());
    }
    let mut result = vec![];
    file.read_to_end(&mut result)?;
    Ok(result)
}

fn optional_regular(parent: &File, name: &str) -> Result<Option<Vec<u8>>> {
    match read_regular(parent, name) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Trial {
    LocalLogicOnly,
    DiskImageRemount,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    fixture_version: u32,
    trial: Trial,
    scratch: PathBuf,
    scratch_pin: Pin,
    image: PathBuf,
    mount: PathBuf,
    nonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    manifest_digest: String,
    mount_pin: Pin,
    image_pin: Pin,
    mount_source: String,
    generation: u64,
    image_association_asserted_by_controller: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepared {
    nonce: String,
    old_head: String,
    package_inode: u64,
    lock_inode: u64,
    binding_generation: u64,
}

fn base_and_plan() -> (VerifiedClosure, Box<CheckpointPlan>) {
    let base = verified(build(add_history));
    let plan =
        checkpoint(run(&base, &rename(&base, "Split phase remount fixture"), 36400).unwrap());
    (base, plan)
}

fn intent_bytes(scope: &Scope, base: &VerifiedClosure, plan: &CheckpointPlan) -> Vec<u8> {
    canon(
        &json!({"nonce":scope.manifest.nonce,"operation_id":id(30001),"write_id":id(36400),"commit_id":id(36401),"old_head":hash(base.token().head_bytes()),"candidate_head":hash(plan.candidate().token().head_bytes()),"request_digest":plan.receipt().request_digest()}),
    )
}

struct Scope {
    manifest: Manifest,
    binding: Binding,
    scratch: File,
    mount: File,
}
impl Scope {
    fn load(manifest_path: &Path, binding: Binding) -> Result<Self> {
        let parent = manifest_path.parent().ok_or_else(invalid)?;
        // Only an explicitly allocated private /private/tmp experiment is eligible.
        if parent.parent() != Some(Path::new("/private/tmp"))
            || !parent
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("photara-ps2-remount-"))
            || manifest_path.file_name().and_then(|n| n.to_str()) != Some("manifest.json")
        {
            return Err(invalid());
        }
        let scratch = File::from(open(parent, DIRECTORY, Mode::empty())?);
        let meta = scratch.metadata()?;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(invalid());
        }
        let raw = read_regular(&scratch, "manifest.json")?;
        let manifest: Manifest = serde_json::from_slice(&raw).map_err(|_| invalid())?;
        if manifest.fixture_version != 1
            || manifest.scratch != parent
            || manifest.scratch_pin != pin(&scratch)?
            || manifest.image != parent.join("fixture.sparseimage")
            || manifest.mount != parent.join("mount")
            || binding.manifest_digest != hash(&raw)
            || binding.generation == 0
            || !binding.image_association_asserted_by_controller
        {
            return Err(invalid());
        }
        let owner = read_regular(&scratch, "owner.json")?;
        if owner != canon(&json!({"nonce":manifest.nonce,"manifest_digest":hash(&raw)})) {
            return Err(invalid());
        }
        let image = File::from(openat(
            &scratch,
            "fixture.sparseimage",
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        let image_meta = image.metadata()?;
        if !image_meta.is_file()
            || image_meta.nlink() != 1
            || image_meta.uid() != meta.uid()
            || pin(&image)? != binding.image_pin
        {
            return Err(invalid());
        }
        let mount = File::from(openat(&scratch, "mount", DIRECTORY, Mode::empty())?);
        let facts = fstatfs(&mount)?;
        if pin(&mount)? != binding.mount_pin
            || native(&facts.f_mntfromname) != binding.mount_source
            || native(&facts.f_fstypename) != "apfs"
            || facts.f_flags & 0x1000 == 0
            || facts.f_flags & 1 != 0
            || (manifest.trial == Trial::DiskImageRemount && pin(&mount)?[0] == pin(&scratch)?[0])
        {
            return Err(invalid());
        }
        Ok(Self {
            manifest,
            binding,
            scratch,
            mount,
        })
    }

    fn recheck(&self) -> Result<()> {
        if pin(&File::from(open(
            &self.manifest.scratch,
            DIRECTORY,
            Mode::empty(),
        )?))?
            != self.manifest.scratch_pin
            || pin(&File::from(openat(
                &self.scratch,
                "mount",
                DIRECTORY,
                Mode::empty(),
            )?))?
                != self.binding.mount_pin
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn package(&self) -> Result<File> {
        self.recheck()?;
        let package = File::from(openat(&self.mount, PACKAGE, DIRECTORY, Mode::empty())?);
        if pin(&package)?[0] != self.binding.mount_pin[0]
            || read_regular(&package, ".owner")? != self.manifest.nonce.as_bytes()
        {
            return Err(invalid());
        }
        let prepared: Prepared =
            serde_json::from_slice(&read_regular(&self.scratch, "prepared.json")?)
                .map_err(|_| invalid())?;
        let lock = File::from(openat(
            &package,
            ".writer-lock",
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        let lock_metadata = lock.metadata()?;
        let (base, _) = base_and_plan();
        if prepared.nonce != self.manifest.nonce
            || prepared.old_head != hash(base.token().head_bytes())
            || prepared.package_inode != pin(&package)?[1]
            || prepared.lock_inode != pin(&lock)?[1]
            || prepared.binding_generation > self.binding.generation
            || !lock_metadata.is_file()
            || lock_metadata.nlink() != 1
        {
            return Err(invalid());
        }
        Ok(package)
    }
}

struct Trace {
    file: File,
    cut: Option<(String, bool)>,
}
impl Trace {
    fn new(scope: &Scope, phase: &str) -> Result<Self> {
        let file = File::from(openat(
            &scope.scratch,
            format!("{phase}.trace.jsonl"),
            OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )?);
        let cut = std::env::var("PHOTARA_PS2_REMOUNT_CUT").ok().map(|name| {
            (
                name,
                std::env::var("PHOTARA_PS2_REMOUNT_CUT_AFTER").as_deref() == Ok("1"),
            )
        });
        Ok(Self { file, cut })
    }
    fn record(&mut self, value: &Value) -> Result<()> {
        writeln!(&mut self.file, "{value}")?;
        self.file.flush()
    }
    fn step(&mut self, name: &str, action: impl FnOnce() -> Result<()>) -> Result<()> {
        self.record(&json!({"step":name,"event":"before"}))?;
        if self
            .cut
            .as_ref()
            .is_some_and(|(cut, after)| cut == name && !after)
        {
            std::process::exit(81);
        }
        if let Err(error) = action() {
            self.record(&json!({"step":name,"event":"failed","outcome":"OutcomeUnknown","errno":error.raw_os_error()}))?;
            return Err(error);
        }
        self.record(&json!({"step":name,"event":"after","errno":null}))?;
        if self
            .cut
            .as_ref()
            .is_some_and(|(cut, after)| cut == name && *after)
        {
            std::process::exit(82);
        }
        Ok(())
    }
    fn barriers(&mut self, name: &str, file: &File) -> Result<()> {
        self.step(&format!("fsync {name}"), || fsync(file).map_err(Into::into))?;
        self.step(&format!("F_FULLFSYNC {name}"), || {
            fcntl_fullfsync(file).map_err(Into::into)
        })
    }
}

fn write_new(parent: &File, name: &str, bytes: &[u8]) -> Result<File> {
    let mut file = File::from(openat(
        parent,
        name,
        OFlags::CREATE | OFlags::EXCL | OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?);
    file.write_all(bytes)?;
    Ok(file)
}

fn directories(root: &File, relative: &str, trace: &mut Trace) -> Result<File> {
    let mut parent = root.try_clone()?;
    if relative == "." {
        return Ok(parent);
    }
    let mut prefix = String::new();
    for component in relative.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(invalid());
        }
        prefix.push_str(component);
        let created = match mkdirat(&parent, component, Mode::RWXU) {
            Ok(()) => true,
            Err(rustix::io::Errno::EXIST) => false,
            Err(error) => return Err(error.into()),
        };
        let child = File::from(openat(&parent, component, DIRECTORY, Mode::empty())?);
        if pin(&child)?[0] != pin(root)?[0] {
            return Err(invalid());
        }
        if created {
            trace.barriers(&format!("new-directory {prefix}"), &child)?;
            trace.barriers(&format!("ancestor-of {prefix}"), &parent)?;
        }
        parent = child;
        prefix.push('/');
    }
    Ok(parent)
}

fn publish_file(
    root: &File,
    name: &str,
    bytes: &[u8],
    replace: bool,
    trace: &mut Trace,
) -> Result<()> {
    let (parent_name, target) = name.rsplit_once('/').unwrap_or((".", name));
    let parent = directories(root, parent_name, trace)?;
    let temporary = format!(".stage-{target}");
    let mut handle = None;
    trace.step(&format!("write {name}"), || {
        handle = Some(write_new(&parent, &temporary, bytes)?);
        Ok(())
    })?;
    trace.barriers(&format!("file {name}"), &handle.unwrap())?;
    trace.step(&format!("publish {name}"), || {
        if replace {
            renameat(&parent, &temporary, &parent, target)
        } else {
            renameat_with(&parent, &temporary, &parent, target, RenameFlags::NOREPLACE)
        }
        .map_err(Into::into)
    })?;
    trace.barriers(&format!("directory-after {name}"), &parent)
}

fn prepare(scope: &Scope) -> Result<()> {
    scope.recheck()?;
    let mut trace = Trace::new(scope, "prepare")?;
    mkdirat(&scope.mount, PACKAGE, Mode::RWXU)?; // Existing package is never adopted/overwritten.
    let root = File::from(openat(&scope.mount, PACKAGE, DIRECTORY, Mode::empty())?);
    let owner = write_new(&root, ".owner", scope.manifest.nonce.as_bytes())?;
    trace.barriers("owner file", &owner)?;
    let lock = write_new(&root, ".writer-lock", b"")?;
    trace.barriers("lock file", &lock)?;
    trace.barriers("new package directory", &root)?;
    trace.barriers("mount containing package", &scope.mount)?;
    let (base, _) = base_and_plan();
    for (name, bytes) in base.files().files() {
        if name != "HEAD.json" {
            publish_file(&root, name, bytes, false, &mut trace)?;
        }
    }
    publish_file(
        &root,
        "HEAD.json",
        base.token().head_bytes(),
        false,
        &mut trace,
    )?;
    package::v1_1::validate_directory(scope.manifest.mount.join(PACKAGE), PackageLimits::default())
        .map_err(|_| invalid())?;
    let prepared = Prepared {
        nonce: scope.manifest.nonce.clone(),
        old_head: hash(base.token().head_bytes()),
        package_inode: pin(&root)?[1],
        lock_inode: pin(&lock)?[1],
        binding_generation: scope.binding.generation,
    };
    let receipt = write_new(
        &scope.scratch,
        "prepared.json",
        &canon(&serde_json::to_value(&prepared).unwrap()),
    )?;
    trace.barriers("prepared receipt", &receipt)?;
    trace.barriers("control directory", &scope.scratch)
}

fn publish(scope: &Scope) -> Result<()> {
    let root = scope.package()?;
    let lock = File::from(openat(
        &root,
        ".writer-lock",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    if !lock.metadata()?.is_file() || lock.metadata()?.nlink() != 1 {
        return Err(invalid());
    }
    flock(&lock, FlockOperation::NonBlockingLockExclusive)?;
    let (base, plan) = base_and_plan();
    if read_regular(&root, "HEAD.json")? != base.token().head_bytes()
        || read_regular(&root, "manifest.json")? != base.files().files()["manifest.json"].as_ref()
    {
        return Err(invalid());
    }
    let mut trace = Trace::new(scope, "publish")?;
    let intent = intent_bytes(scope, &base, &plan);
    publish_file(&root, ".intent.json", &intent, false, &mut trace)?;
    for file in plan.immutable_files() {
        publish_file(&root, file.name(), file.bytes(), false, &mut trace)?;
    }
    scope.recheck()?;
    if read_regular(&root, "HEAD.json")? != base.token().head_bytes() {
        return Err(invalid());
    }
    publish_file(
        &root,
        "HEAD.json",
        plan.candidate().token().head_bytes(),
        true,
        &mut trace,
    )?;
    trace.step("independent candidate validation", || {
        package::v1_1::validate_directory(
            scope.manifest.mount.join(PACKAGE),
            PackageLimits::default(),
        )
        .map_err(|_| invalid())?;
        if read_regular(&root, "HEAD.json")? != plan.candidate().token().head_bytes() {
            return Err(invalid());
        }
        Ok(())
    })?;
    publish_file(&root, ".receipt.json", &intent, false, &mut trace)?;
    trace.record(&json!({"event":"synthetic-complete","qualified":false,"saved_claim":false}))
}

fn verify(scope: &Scope) -> Result<Value> {
    let root = scope.package()?;
    let (base, plan) = base_and_plan();
    let head = read_regular(&root, "HEAD.json")?;
    let selected = if head == base.token().head_bytes() {
        "old"
    } else if head == plan.candidate().token().head_bytes() {
        "candidate"
    } else {
        return Err(invalid());
    };
    let read = package::v1_1::validate_directory(
        scope.manifest.mount.join(PACKAGE),
        PackageLimits::default(),
    )
    .map_err(|_| invalid())?;
    if read.commits.len() != if selected == "old" { 1 } else { 2 } {
        return Err(invalid());
    }
    let intent = optional_regular(&root, ".intent.json")?;
    if selected == "candidate" && intent.is_none() {
        return Err(invalid());
    }
    if intent
        .as_ref()
        .is_some_and(|intent| intent != &intent_bytes(scope, &base, &plan))
    {
        return Err(invalid());
    }
    let receipt = optional_regular(&root, ".receipt.json")?;
    if receipt.is_some() && receipt != intent {
        return Err(invalid());
    }
    Ok(
        json!({"selected":selected,"head_digest":hash(&head),"closure_valid":true,"intent_present":intent.is_some(),"receipt_present":receipt.is_some(),"binding_generation":scope.binding.generation,"qualified":false,"saved_claim":false,"remount_tested":false,"power_loss_tested":false,"note":"mount experiment evidence must be recorded separately; reopening alone proves no remount"}),
    )
}

#[test]
#[ignore = "external phase driver requires explicit owned manifest and exact mount binding; never invokes mount tools"]
fn remount_phase() {
    let path = PathBuf::from(
        std::env::var("PHOTARA_PS2_REMOUNT_MANIFEST").expect("explicit owned manifest required"),
    );
    let binding: Binding = serde_json::from_str(
        &std::env::var("PHOTARA_PS2_REMOUNT_BINDING")
            .expect("exact current mount binding required"),
    )
    .unwrap();
    let scope = Scope::load(&path, binding).unwrap();
    match std::env::var("PHOTARA_PS2_REMOUNT_PHASE").unwrap().as_str() {
        "prepare" => prepare(&scope).unwrap(),
        "publish" => publish(&scope).unwrap(),
        "verify" => {
            let report = verify(&scope).unwrap();
            write_new(
                &scope.scratch,
                &format!("verification-{}.json", scope.binding.generation),
                &canon(&report),
            )
            .unwrap();
            println!("{report}");
        }
        _ => panic!("unknown phase"),
    }
}

fn create_manifest(scratch: &Path, image: &Path, mount: &Path, trial: Trial) -> Result<Manifest> {
    if scratch.parent() != Some(Path::new("/private/tmp"))
        || !scratch
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("photara-ps2-remount-"))
        || image != scratch.join("fixture.sparseimage")
        || mount != scratch.join("mount")
    {
        return Err(invalid());
    }
    let directory = File::from(open(scratch, DIRECTORY, Mode::empty())?);
    if directory.metadata()?.permissions().mode() & 0o077 != 0 {
        return Err(invalid());
    }
    let manifest = Manifest {
        fixture_version: 1,
        trial,
        scratch: scratch.into(),
        scratch_pin: pin(&directory)?,
        image: image.into(),
        mount: mount.into(),
        nonce: uuid::Uuid::new_v4().to_string(),
    };
    let raw = canon(&serde_json::to_value(&manifest).unwrap());
    write_new(&directory, "manifest.json", &raw)?;
    write_new(
        &directory,
        "owner.json",
        &canon(&json!({"nonce":manifest.nonce,"manifest_digest":hash(&raw)})),
    )?;
    Ok(manifest)
}

#[test]
#[ignore = "dry-run manifest generator requires explicit private scratch/image/mount; no image or mount commands are executed"]
fn remount_dry_run_manifest() {
    let scratch = PathBuf::from(
        std::env::var("PHOTARA_PS2_REMOUNT_SCRATCH").expect("explicit unique scratch required"),
    );
    let image =
        PathBuf::from(std::env::var("PHOTARA_PS2_REMOUNT_IMAGE").expect("explicit image required"));
    let mount =
        PathBuf::from(std::env::var("PHOTARA_PS2_REMOUNT_MOUNT").expect("explicit mount required"));
    let manifest = create_manifest(&scratch, &image, &mount, Trial::DiskImageRemount).unwrap();
    println!(
        "{}",
        json!({"manifest":scratch.join("manifest.json"),"image":manifest.image,"mount":manifest.mount,"phase_test":"planning::macos_remount_harness::remount_phase","required_environment":["PHOTARA_PS2_REMOUNT_MANIFEST","PHOTARA_PS2_REMOUNT_BINDING","PHOTARA_PS2_REMOUNT_PHASE"],"phases":["prepare","publish","verify"],"controller_must":"create and attach only the owned image; supply exact current mount/image pins, source and a fresh generation; after detach/remount supply a fresh binding; no mount is performed by this harness","qualified":false,"remount_executed":false,"power_loss_executed":false})
    );
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn child(manifest: &Manifest, binding: &Binding, phase: &str, cut: Option<(&str, bool)>) -> i32 {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "planning::macos_remount_harness::remount_phase",
        ])
        .env(
            "PHOTARA_PS2_REMOUNT_MANIFEST",
            manifest.scratch.join("manifest.json"),
        )
        .env(
            "PHOTARA_PS2_REMOUNT_BINDING",
            serde_json::to_string(binding).unwrap(),
        )
        .env("PHOTARA_PS2_REMOUNT_PHASE", phase)
        .env_remove("PHOTARA_PS2_REMOUNT_CUT")
        .env_remove("PHOTARA_PS2_REMOUNT_CUT_AFTER")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some((label, after)) = cut {
        command.env("PHOTARA_PS2_REMOUNT_CUT", label).env(
            "PHOTARA_PS2_REMOUNT_CUT_AFTER",
            if after { "1" } else { "0" },
        );
    }
    let mut child = ChildGuard(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status.code().unwrap();
        }
        assert!(Instant::now() < deadline, "phase {phase} timed out");
        thread::sleep(Duration::from_millis(5));
    }
}

fn local_fixture() -> (tempfile::TempDir, Manifest, Binding) {
    let temporary = tempfile::Builder::new()
        .prefix("photara-ps2-remount-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in("/private/tmp")
        .unwrap();
    let path = temporary.path();
    let manifest = create_manifest(
        path,
        &path.join("fixture.sparseimage"),
        &path.join("mount"),
        Trial::LocalLogicOnly,
    )
    .unwrap();
    // This marker is deliberately NOT an actual disk image.
    fs::write(
        &manifest.image,
        b"local protocol logic only; no image created or attached",
    )
    .unwrap();
    fs::create_dir(&manifest.mount).unwrap();
    let mount = File::from(open(&manifest.mount, DIRECTORY, Mode::empty()).unwrap());
    let binding = Binding {
        manifest_digest: hash(&fs::read(path.join("manifest.json")).unwrap()),
        mount_pin: pin(&mount).unwrap(),
        image_pin: pin(&File::open(&manifest.image).unwrap()).unwrap(),
        mount_source: native(&fstatfs(&mount).unwrap().f_mntfromname),
        generation: 1,
        image_association_asserted_by_controller: true,
    };
    (temporary, manifest, binding)
}

#[test]
#[ignore = "explicit split-phase APFS protocol tests; actual mount/remount/power-loss trials unexecuted"]
fn split_phases_reopen_owned_fixture_after_controlled_process_cuts() {
    let cuts = [
        None,
        Some(("publish HEAD.json", false)),
        Some(("publish HEAD.json", true)),
        Some(("F_FULLFSYNC directory-after HEAD.json", true)),
        Some(("independent candidate validation", true)),
        Some(("publish .receipt.json", false)),
        Some(("F_FULLFSYNC directory-after .receipt.json", true)),
    ];
    for cut in cuts {
        let (_temporary, manifest, binding) = local_fixture();
        assert_eq!(child(&manifest, &binding, "prepare", None), 0);
        assert!(manifest.scratch.join("prepared.json").is_file());
        let expected = cut.map_or(0, |(_, after)| if after { 82 } else { 81 });
        assert_eq!(child(&manifest, &binding, "publish", cut), expected);
        let before = fs::read(manifest.mount.join(PACKAGE).join("HEAD.json")).unwrap();
        assert_eq!(child(&manifest, &binding, "verify", None), 0);
        assert_eq!(
            fs::read(manifest.mount.join(PACKAGE).join("HEAD.json")).unwrap(),
            before
        );
        let report: Value = serde_json::from_slice(
            &fs::read(manifest.scratch.join("verification-1.json")).unwrap(),
        )
        .unwrap();
        let phase_trace = |name: &str| -> Vec<Value> {
            fs::read_to_string(manifest.scratch.join(format!("{name}.trace.jsonl")))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        };
        assert_eq!(report["closure_valid"], true);
        assert_eq!(report["remount_tested"], false);
        assert_eq!(report["power_loss_tested"], false);
        assert_eq!(
            report["selected"],
            if cut == Some(("publish HEAD.json", false)) {
                "old"
            } else {
                "candidate"
            }
        );
        println!(
            "{}",
            json!({"cut":cut,"report":report,"publication_exit":expected,"prepare_barriers":true,"independent_processes":3,"prepare_trace":phase_trace("prepare"),"publish_trace":phase_trace("publish")})
        );
    }
}

#[test]
fn wrong_scope_identity_or_image_never_reaches_prepare() {
    let (_temporary, manifest, binding) = local_fixture();
    let path = manifest.scratch.join("manifest.json");
    assert!(Scope::load(&path, binding.clone()).is_ok());
    for wrong in [
        "mount",
        "image",
        "source",
        "generation",
        "assertion",
        "manifest",
    ] {
        let mut changed = binding.clone();
        match wrong {
            "mount" => changed.mount_pin[1] += 1,
            "image" => changed.image_pin[1] += 1,
            "source" => changed.mount_source.push('x'),
            "generation" => changed.generation = 0,
            "assertion" => changed.image_association_asserted_by_controller = false,
            "manifest" => changed.manifest_digest.push('x'),
            _ => unreachable!(),
        }
        assert!(Scope::load(&path, changed).is_err());
        assert!(!manifest.mount.join(PACKAGE).exists());
    }
    assert!(
        create_manifest(
            &manifest.scratch,
            Path::new("/private/tmp/unowned-image"),
            &manifest.mount,
            Trial::DiskImageRemount
        )
        .is_err()
    );
    fs::rename(&manifest.mount, manifest.scratch.join("old-mount")).unwrap();
    fs::create_dir(&manifest.mount).unwrap();
    assert!(Scope::load(&path, binding).is_err());
}

fn snapshot(path: &Path, prefix: &str, files: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{prefix}{}", entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            snapshot(&entry.path(), &format!("{name}/"), files);
        } else {
            files.insert(name, hash(&fs::read(entry.path()).unwrap()));
        }
    }
}

#[test]
#[ignore = "explicit native preparation and publication barriers in disposable fixtures"]
fn split_verifier_refuses_changed_owner_lock_intent_receipt_or_head_without_writes() {
    for changed in ["owner", "lock", "intent", "receipt", "head"] {
        let (_temporary, manifest, binding) = local_fixture();
        assert_eq!(child(&manifest, &binding, "prepare", None), 0);
        assert_eq!(child(&manifest, &binding, "publish", None), 0);
        let root = manifest.mount.join(PACKAGE);
        match changed {
            "owner" => fs::write(root.join(".owner"), b"wrong owner").unwrap(),
            "lock" => {
                fs::rename(root.join(".writer-lock"), root.join(".old-lock")).unwrap();
                File::create_new(root.join(".writer-lock")).unwrap();
            }
            "intent" => fs::write(root.join(".intent.json"), b"wrong intent").unwrap(),
            "receipt" => fs::write(root.join(".receipt.json"), b"wrong receipt").unwrap(),
            "head" => fs::write(root.join("HEAD.json"), b"wrong HEAD").unwrap(),
            _ => unreachable!(),
        }
        let mut before = BTreeMap::new();
        snapshot(&manifest.scratch, "", &mut before);
        let scope = Scope::load(&manifest.scratch.join("manifest.json"), binding).unwrap();
        assert!(verify(&scope).is_err(), "accepted {changed}");
        let mut after = BTreeMap::new();
        snapshot(&manifest.scratch, "", &mut after);
        assert_eq!(before, after);
    }
}

#[test]
#[ignore = "synthetic mountpoint replacement only; no mount or remount operation"]
fn split_fresh_binding_reopens_the_same_owned_package_after_path_replacement() {
    let (_temporary, manifest, binding) = local_fixture();
    assert_eq!(child(&manifest, &binding, "prepare", None), 0);
    let old = manifest.scratch.join("old-mount");
    fs::rename(&manifest.mount, &old).unwrap();
    fs::create_dir(&manifest.mount).unwrap();
    // Preserve the owned package inode while replacing only the containing path.
    fs::rename(old.join(PACKAGE), manifest.mount.join(PACKAGE)).unwrap();
    let path = manifest.scratch.join("manifest.json");
    assert!(Scope::load(&path, binding.clone()).is_err());
    let mut refreshed = binding;
    refreshed.mount_pin = pin(&File::from(
        open(&manifest.mount, DIRECTORY, Mode::empty()).unwrap(),
    ))
    .unwrap();
    refreshed.generation += 1;
    assert_eq!(child(&manifest, &refreshed, "verify", None), 0);
    let report: Value =
        serde_json::from_slice(&fs::read(manifest.scratch.join("verification-2.json")).unwrap())
            .unwrap();
    assert_eq!(report["selected"], "old");
    assert_eq!(report["binding_generation"], 2);
    assert_eq!(report["remount_tested"], false);
}
