//! Opt-in disposable APFS adapter for the existing repeatable test path.
//! No mount commands, production admission, storage qualification, or Saved claim.
use super::{
    Attempt, BTreeMap, DisposablePermit, EffectFailure, ExecutionError, Observation,
    OriginalPackage, PlannerInputs, RepeatableIo, RepeatablePlan, RestartContext, SelectedStage,
    Value, VerifiedOriginal, aid, attempt, budget, encode, execute, frames, hash, identity,
    journal, json, package, plan, recovery, reference, request, restart, restore_plan, selected,
    support, v1_3,
};
#[cfg(test)]
use rustix::fs::mkdirat;
use rustix::fs::{
    AtFlags, FlockOperation, Mode, OFlags, fcntl_fullfsync, flock, fstatfs, fsync, open, openat,
    renameat, unlinkat,
};
#[cfg(test)]
use std::process::Command;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
type IoResult<T> = std::io::Result<T>;
type Pin = [u64; 2];
const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const PACKAGE: &str = "repeatable-package";
const EVIDENCE: &str = "repeatable-evidence";
const CHECKPOINT: &str = "checkpoint.phpsj";
fn invalid() -> std::io::Error {
    std::io::ErrorKind::InvalidData.into()
}
fn pin(f: &File) -> IoResult<Pin> {
    let m = f.metadata()?;
    Ok([m.dev(), m.ino()])
}
fn canonical(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn number(v: &Value) -> IoResult<u64> {
    v.as_str()
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())
}
fn regular(root: &File, name: &str, write: bool) -> IoResult<File> {
    if name.is_empty() || name.contains('/') || name == "." || name == ".." {
        return Err(invalid());
    }
    let f = File::from(openat(
        root,
        name,
        (if write { OFlags::RDWR } else { OFlags::RDONLY })
            | OFlags::NOFOLLOW
            | OFlags::NONBLOCK
            | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let m = f.metadata()?;
    if !m.is_file()
        || m.nlink() != 1
        || m.dev() != root.metadata()?.dev()
        || m.uid() != root.metadata()?.uid()
    {
        return Err(invalid());
    }
    Ok(f)
}
fn read(root: &File, name: &str) -> IoResult<Vec<u8>> {
    let mut f = regular(root, name, false)?;
    let n = f.metadata()?.len();
    if n > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut b = Vec::new();
    std::io::Read::by_ref(&mut f)
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut b)?;
    if b.len() as u64 != n {
        return Err(invalid());
    }
    Ok(b)
}
fn barrier(f: &File) -> IoResult<()> {
    fsync(f)?;
    fcntl_fullfsync(f)?;
    Ok(())
}
fn create(root: &File, name: &str, bytes: &[u8]) -> IoResult<File> {
    let mut f = File::from(openat(
        root,
        name,
        OFlags::CREATE | OFlags::EXCL | OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?);
    f.write_all(bytes)?;
    barrier(&f)?;
    Ok(f)
}
fn exact_file(root: &File, name: &str, bytes: &[u8]) -> IoResult<()> {
    match regular(root, name, false) {
        Ok(file) => {
            if read(root, name)? != bytes {
                return Err(invalid());
            }
            barrier(&file)?;
            barrier(root)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            create(root, name, bytes)?;
            barrier(root)?;
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
fn append_exact(root: &File, name: &str, expected: &[u8], suffix: &[u8]) -> IoResult<()> {
    let mut file = regular(root, name, true)?;
    let original_pin = pin(&file)?;
    if read(root, name)? != expected
        || pin(&regular(root, name, false)?)? != original_pin
        || file.metadata()?.len() != expected.len() as u64
    {
        return Err(invalid());
    }
    file.seek(SeekFrom::Start(expected.len() as u64))?;
    file.write_all(suffix)?;
    barrier(&file)?;
    barrier(root)
}
fn native_string(a: &[std::ffi::c_char]) -> String {
    String::from_utf8(
        a.iter()
            .take_while(|c| **c != 0)
            .map(|c| u8::try_from(*c).unwrap())
            .collect(),
    )
    .unwrap()
}
struct Scope {
    scratch: File,
    mount: File,
    path: PathBuf,
    nonce: String,
    generation: u64,
    assessment: v1_3::native_profile::DisposableAssessment,
    permit: DisposablePermit,
}
impl Scope {
    #[cfg(test)]
    fn load() -> IoResult<Self> {
        let path =
            PathBuf::from(std::env::var("PHOTARA_PS2_REMOUNT_MANIFEST").map_err(|_| invalid())?);
        let binding = serde_json::from_str(
            &std::env::var("PHOTARA_PS2_REMOUNT_BINDING").map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        Self::load_explicit(&path, binding)
    }
    fn load_explicit(path: &Path, binding: Value) -> IoResult<Self> {
        let parent = path.parent().ok_or_else(invalid)?;
        if parent.parent() != Some(Path::new("/private/tmp"))
            || !parent
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("photara-ps2-remount-"))
            || path.file_name().and_then(|s| s.to_str()) != Some("manifest.json")
        {
            return Err(invalid());
        }
        let scratch = File::from(open(parent, DIRECTORY, Mode::empty())?);
        if scratch.metadata()?.permissions().mode() & 0o077 != 0 {
            return Err(invalid());
        }
        let raw = read(&scratch, "manifest.json")?;
        let manifest: Value = serde_json::from_slice(&raw).map_err(|_| invalid())?;
        let nonce = manifest["nonce"].as_str().ok_or_else(invalid)?.to_owned();
        if manifest["fixture_version"] != 1
            || manifest["trial"] != "disk-image-remount"
            || manifest["scratch"] != parent.to_string_lossy().as_ref()
            || manifest["scratch_pin"] != json!(pin(&scratch)?)
            || manifest["image"]
                != parent
                    .join("fixture.sparseimage")
                    .to_string_lossy()
                    .as_ref()
            || manifest["mount"] != parent.join("mount").to_string_lossy().as_ref()
            || binding["manifest_digest"] != hash(&raw)
            || binding["image_association_asserted_by_controller"] != true
            || read(&scratch, "owner.json")?
                != canonical(&json!({"nonce":nonce,"manifest_digest":hash(&raw)}))
        {
            return Err(invalid());
        }
        let image = regular(&scratch, "fixture.sparseimage", false)?;
        if binding["image_pin"] != json!(pin(&image)?)
            || image.metadata()?.uid() != scratch.metadata()?.uid()
        {
            return Err(invalid());
        }
        let mount = File::from(openat(&scratch, "mount", DIRECTORY, Mode::empty())?);
        let facts = fstatfs(&mount)?;
        if binding["mount_pin"] != json!(pin(&mount)?)
            || binding["mount_source"] != native_string(&facts.f_mntfromname)
            || native_string(&facts.f_fstypename) != "apfs"
            || facts.f_flags & 0x1000 == 0
            || facts.f_flags & 1 != 0
            || pin(&mount)?[0] == pin(&scratch)?[0]
        {
            return Err(invalid());
        }
        let generation = binding["generation"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(invalid)?;
        let volume = package::digest(&canonical(&binding));
        let assessment = v1_3::native_profile::assess_disposable_image(
            path,
            serde_json::from_value(binding).map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        Ok(Self {
            scratch,
            mount,
            path: parent.join("mount").join(PACKAGE),
            nonce,
            generation,
            assessment,
            permit: DisposablePermit {
                volume,
                epoch: package::planning::OwnerEpoch::parse(&uuid::Uuid::new_v4().to_string())
                    .map_err(|_| invalid())?,
            },
        })
    }
}
#[cfg(test)]
fn witness(p: Pin) -> Value {
    json!({"device":p[0].to_string(),"inode":p[1].to_string()})
}
fn pack_name(id: &str) -> String {
    format!("pack-{id}")
}
fn source_name(path: &str) -> String {
    format!("retained-{}", hash(path.as_bytes()))
}
#[expect(
    clippy::too_many_lines,
    reason = "Explicit disposable bootstrap with captured native witnesses"
)]
#[cfg(test)]
fn prepare(scope: &Scope, session: bool) -> IoResult<()> {
    mkdirat(&scope.mount, EVIDENCE, Mode::RWXU)?;
    let evidence = File::from(openat(&scope.mount, EVIDENCE, DIRECTORY, Mode::empty())?);
    barrier(&evidence)?;
    barrier(&scope.mount)?;
    mkdirat(&scope.mount, PACKAGE, Mode::RWXU)?;
    let root = File::from(openat(&scope.mount, PACKAGE, DIRECTORY, Mode::empty())?);
    create(&root, ".owner", scope.nonce.as_bytes())?;
    let lock = create(&root, ".writer-lock", b"")?;
    let base = if let Ok(seed) = std::env::var("PHOTARA_PS4_SEED_ROOT") {
        let bytes = std::fs::read(
            Path::new(&seed).join("docs/architecture/proposals/ps2/integrated/linked.json"),
        )?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(invalid());
        }
        support::Fixture::from_value(serde_json::from_slice(&bytes).map_err(|_| invalid())?)?
    } else {
        support::Fixture::load("integrated")
    };
    let mut captured = json!({"allocations":{},"retained":{}});
    let mut handles = BTreeMap::new();
    for id in base.allocations.keys() {
        let f = create(&root, &pack_name(&id.to_string()), b"")?;
        captured["allocations"][id.to_string()] = witness(pin(&f)?);
        handles.insert(id.to_string(), f);
    }
    for (path, raw) in base.data["source_files"].as_object().ok_or_else(invalid)? {
        let f = create(
            &root,
            &source_name(path),
            &support::unhex(raw.as_str().ok_or_else(invalid)?),
        )?;
        captured["retained"][path] = witness(pin(&f)?);
    }
    barrier(&root)?;
    barrier(&scope.mount)?;
    if session {
        let requested = std::env::var("PHOTARA_PS3_PACK_RESERVE_BYTES")
            .map_err(|_| invalid())?
            .parse::<u64>()
            .map_err(|_| invalid())?;
        if requested == 0 || requested % 4096 != 0 || requested > i64::MAX as u64 {
            return Err(invalid());
        }
        let root_pin = pin(&root)?;
        let mut child = Command::new("python3")
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/ps2_reader_support/native_bootstrap.py"),
            )
            .arg("--preallocate")
            .arg(&scope.path)
            .arg(root_pin[0].to_string())
            .arg(root_pin[1].to_string())
            .arg(requested.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or_else(invalid)?
            .write_all(&canonical(&captured))?;
        let result = child.wait_with_output()?;
        if !result.status.success() {
            return Err(std::io::Error::other(
                String::from_utf8_lossy(&result.stderr).into_owned(),
            ));
        }
        captured["tip_charges"] = json!({});
        for (id, file) in &handles {
            if id == &aid(1) {
                continue;
            }
            barrier(file)?;
            let observed = file.metadata()?;
            let allocated = observed.blocks().checked_mul(512).ok_or_else(invalid)?;
            if observed.len() != 0
                || allocated < requested
                || witness(pin(file)?) != captured["allocations"][id]
            {
                return Err(invalid());
            }
            let charged = allocated
                .checked_add(4095)
                .map(|n| n / 4096 * 4096)
                .ok_or_else(invalid)?;
            captured["tip_charges"][id] = json!(charged.to_string());
        }
    }
    create(
        &scope.scratch,
        "repeatable-native-pins.json",
        &canonical(&captured),
    )?;
    barrier(&scope.scratch)?;
    let standing = if session {
        std::env::var("PHOTARA_PS3_STANDING_BYTES")
            .map_err(|_| invalid())?
            .parse::<u64>()
            .map_err(|_| invalid())?
    } else {
        262_144
    };
    if standing == 0 || standing % 4096 != 0 {
        return Err(invalid());
    }
    let mut command = Command::new("python3");
    command.env("PHOTARA_PS3_STANDING_BYTES", standing.to_string());
    let output = command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/ps2_reader_support/native_bootstrap.py"),
        )
        .arg(
            scope
                .path
                .parent()
                .ok_or_else(invalid)?
                .parent()
                .ok_or_else(invalid)?
                .join("repeatable-native-pins.json"),
        )
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let data: Value = serde_json::from_slice(&output.stdout).map_err(|_| invalid())?;
    for (id, mut f) in handles {
        let raw = support::unhex(
            data["allocations"][&id]["hex"]
                .as_str()
                .ok_or_else(invalid)?,
        );
        if pin(&f)?
            != [
                number(&captured["allocations"][&id]["device"])?,
                number(&captured["allocations"][&id]["inode"])?,
            ]
        {
            return Err(invalid());
        }
        f.write_all(&raw)?;
        barrier(&f)?;
        if session && id != aid(1) {
            let allocated = f
                .metadata()?
                .blocks()
                .checked_mul(512)
                .ok_or_else(invalid)?;
            if allocated > number(&captured["tip_charges"][&id])? {
                return Err(invalid());
            }
        }
    }
    for (sha, raw) in data["loose"].as_object().ok_or_else(invalid)? {
        create(
            &root,
            &format!("{sha}.control"),
            raw.as_str().ok_or_else(invalid)?.as_bytes(),
        )?;
    }
    create(
        &root,
        "manifest.json",
        data["bootstrap"]["manifest"]
            .as_str()
            .ok_or_else(invalid)?
            .as_bytes(),
    )?;
    let commit = data["bootstrap"]["commit"]
        .as_str()
        .ok_or_else(invalid)?
        .as_bytes();
    create(&root, &format!("{}.commit", hash(commit)), commit)?;
    create(
        &root,
        "HEAD.json",
        data["bootstrap"]["head"]
            .as_str()
            .ok_or_else(invalid)?
            .as_bytes(),
    )?;
    barrier(&root)?;
    let initial: OriginalPackage = package(&support::Fixture::from_value(data.clone())?);
    let (_, receipt) = request();
    let device_id =
        std::env::var("PHOTARA_PS4_DEVICE_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    package::PackageUuid::parse(&device_id).map_err(|_| invalid())?;
    let journal_header = json!({"format_version":1,"stream_id":uuid::Uuid::new_v4().to_string(),"journal_id":receipt["journal_id"],"device_id":device_id,"project_id":initial.manifest["project_id"],"library_id":initial.commit["root_set"]["library_id"],"incarnation_id":initial.commit["root_set"]["placement"]["incarnation"],"bootstrap_sha256":hash(&canonical(&initial.manifest)),"base_head":initial.head,"base_package_revision":initial.commit["package_revision"]});
    // Incarnation is selected by accounting, not the portable placement selector.
    let mut journal_header = journal_header;
    journal_header["incarnation_id"] = serde_json::to_value(
        support::Fixture::from_value(data.clone())?
            .registration
            .incarnation,
    )
    .map_err(|_| invalid())?;
    let checkpoint = create(
        &evidence,
        CHECKPOINT,
        &journal::Journal::create(&journal_header, journal_limits(standing))
            .map_err(|_| invalid())?,
    )?;
    barrier(&evidence)?;
    // Trusted experimental registration captured before any repeatable writer effect.
    let registration = json!({"nonce":scope.nonce,"package_pin":pin(&root)?,"lock_pin":pin(&lock)?,"evidence_pin":pin(&evidence)?,"generation":scope.generation,"journal_header":journal_header,"journal_pin":pin(&checkpoint)?,"session":session,"tip_charges":captured.get("tip_charges"),"data":data});
    create(
        &scope.scratch,
        "repeatable-registration.json",
        &canonical(&registration),
    )?;
    barrier(&scope.scratch)
}
struct Native {
    scope: Scope,
    root: File,
    evidence: File,
    evidence_pin: Pin,
    base: support::Fixture,
    root_pin: Pin,
    lock_pin: Pin,
    interrupt: bool,
    allowed: BTreeMap<String, Vec<u8>>,
    commits: BTreeMap<String, Vec<u8>>,
    journal_header: Value,
    journal_pin: Pin,
    journal_capacity: u64,
    journal_high_water: u64,
    completion: Option<journal::JournalAppend>,
    intent_record: Option<Value>,
    published_head: Value,
    completed_head: Value,
    ceiling: Option<support::Fixture>,
    namespace_allowance: u64,
    session_mode: bool,
    local_provenance: Option<Value>,
    local_authority: Option<std::sync::Arc<dyn Fn() -> IoResult<Value> + Send + Sync>>,
    session_project_limit: Option<u64>,
    reserved_journal_extra: usize,
}
// Current observations preserve separately captured original highwaters. The
// shared synthetic helper uses rounded logical lengths; native prepaid storage
// must never lose its measured registration when assembling a current view.
fn snapshot(base: &support::Fixture, p: &OriginalPackage) -> support::Fixture {
    let mut observed = super::snapshot(base, p);
    for (id, allocation) in &mut observed.allocations {
        allocation.charge = allocation.charge.max(base.allocations[id].charge);
    }
    observed
}
fn journal_limits(standing: u64) -> journal::JournalLimits {
    let bytes = usize::try_from(standing).expect("bounded fixture standing pool");
    journal::JournalLimits {
        json: support::limits().json,
        max_file_bytes: bytes,
        max_records: 2, // Existing fixture: one original operation, intent + completion.
        max_work_bytes: bytes,
    }
}
fn record_identity(
    record: Option<&Value>,
    generation: u64,
) -> IoResult<journal::JournalRecordIdentity> {
    Ok(journal::JournalRecordIdentity {
        record_id: package::PackageUuid::parse(&record.map_or_else(
            || uuid::Uuid::new_v4().to_string(),
            |r| r["record_id"].as_str().unwrap_or("").to_owned(),
        ))
        .map_err(|_| invalid())?,
        session_generation: record.map_or(Ok(generation), |r| number(&r["session_generation"]))?,
    })
}
type NativeLease = crate::package::planning::io::RegisteredCooperativeLease<File>;
impl Native {
    #[expect(
        clippy::too_many_lines,
        reason = "Keep independent captured registration and immutable session ceiling checks together"
    )]
    fn load(scope: Scope) -> IoResult<Self> {
        let registration: Value =
            serde_json::from_slice(&read(&scope.scratch, "repeatable-registration.json")?)
                .map_err(|_| invalid())?;
        let root = File::from(openat(&scope.mount, PACKAGE, DIRECTORY, Mode::empty())?);
        if registration["nonce"] != scope.nonce
            || registration["package_pin"] != json!(pin(&root)?)
            || read(&root, ".owner")? != scope.nonce.as_bytes()
            || registration["generation"].as_u64().ok_or_else(invalid)? > scope.generation
        {
            return Err(invalid());
        }
        let lock = regular(&root, ".writer-lock", false)?;
        if registration["lock_pin"] != json!(pin(&lock)?) {
            return Err(invalid());
        }
        let evidence = File::from(openat(&scope.mount, EVIDENCE, DIRECTORY, Mode::empty())?);
        let evidence_pin = pin(&evidence)?;
        if registration["evidence_pin"] != json!(evidence_pin)
            || evidence_pin[0] != pin(&scope.mount)?[0]
            || evidence.metadata()?.permissions().mode() & 0o077 != 0
            || evidence.metadata()?.uid() != root.metadata()?.uid()
        {
            return Err(invalid());
        }
        for entry in std::fs::read_dir(scope.path.parent().ok_or_else(invalid)?.join(EVIDENCE))? {
            if entry?.file_name() != CHECKPOINT {
                return Err(invalid());
            }
        }
        let journal_pin = pin(&regular(&evidence, CHECKPOINT, false)?)?;
        if registration["journal_pin"] != json!(journal_pin) {
            return Err(invalid());
        }
        let journal_header = registration["journal_header"].clone();
        let root_pin = pin(&root)?;
        let lock_pin = pin(&lock)?;
        let mut base = support::Fixture::from_value(registration["data"].clone())?;
        if let Some(charges) = registration["tip_charges"].as_object() {
            if charges.len() != 7 {
                return Err(invalid());
            }
            for (id, amount) in charges {
                let key = package::PackageUuid::parse(id).map_err(|_| invalid())?;
                if id == &aid(1) {
                    return Err(invalid());
                }
                let allocation = base.allocations.get_mut(&key).ok_or_else(invalid)?;
                let charge = number(amount)?;
                if charge < allocation.charge || charge % 4096 != 0 {
                    return Err(invalid());
                }
                allocation.charge = charge;
            }
        }
        let mut native = Self {
            scope,
            root,
            evidence,
            evidence_pin,
            base,
            root_pin,
            lock_pin,
            interrupt: false,
            allowed: BTreeMap::new(),
            commits: BTreeMap::new(),
            journal_header,
            journal_pin,
            journal_capacity: 0,
            journal_high_water: 0,
            completion: None,
            intent_record: None,
            published_head: Value::Null,
            completed_head: Value::Null,
            ceiling: None,
            namespace_allowance: 0,
            local_provenance: None,
            local_authority: None,
            session_mode: registration["session"].as_bool().unwrap_or(false),
            session_project_limit: None,
            reserved_journal_extra: 0,
        };
        if native.session_mode {
            // Configuration derives solely from the separately captured INITIAL
            // ledger, never the later selected ledger. The first original records
            // it before effects; every reopen checks that exact persisted value.
            let initial: Value = native
                .base
                .loose
                .values()
                .filter_map(|b| serde_json::from_slice::<Value>(b).ok())
                .find(|v| v["schema"]["id"] == "photara.storage.ledger")
                .ok_or_else(invalid)?;
            let limit = number(&initial["total_charge"])?
                .checked_add(4096)
                .ok_or_else(invalid)?;
            let journal = native.checkpoint()?;
            if journal
                .records()
                .iter()
                .filter(|r| r.value()["kind"] == "CheckpointIntent")
                .any(|r| r.value()["body"]["original"]["project_limit"] != json!(limit.to_string()))
            {
                return Err(std::io::Error::other(
                    "existing original differs from immutable initial prepaid project ceiling",
                ));
            }
            native.session_project_limit = Some(limit);
        }
        Ok(native)
    }
    fn attempt(&self, old: &VerifiedOriginal, n: u64) -> IoResult<Attempt> {
        let mut candidate = super::attempt(old, n)?;
        if let Some(limit) = self.session_project_limit {
            candidate.project_limit = limit.to_string();
        }
        Ok(candidate)
    }
    fn lease(&self, p: &OriginalPackage) -> IoResult<NativeLease> {
        self.scope.assessment.revalidate().map_err(|_| invalid())?;
        let lock = regular(&self.root, ".writer-lock", false)?;
        flock(&lock, FlockOperation::NonBlockingLockExclusive)?;
        Ok(crate::package::planning::io::disposable_lease(
            lock,
            &self.scope.permit,
            &encode(&p.manifest),
            &encode(&p.head),
            crate::package::planning::IncarnationId::parse(
                &self.base.registration.incarnation.to_string(),
            )
            .unwrap(),
            package::DecimalU64::parse(p.commit["package_revision"].as_str().unwrap()).unwrap(),
        ))
    }
    fn pins(&self) -> IoResult<()> {
        if let Some(authority) = &self.local_authority
            && self.local_provenance.as_ref() != Some(&authority()?)
        {
            return Err(invalid());
        }
        self.scope.assessment.check_pins().map_err(|_| invalid())?;
        let evidence = File::from(openat(
            &self.scope.mount,
            EVIDENCE,
            DIRECTORY,
            Mode::empty(),
        )?);
        if pin(&evidence)? != self.evidence_pin
            || pin(&self.evidence)? != self.evidence_pin
            || evidence.metadata()?.permissions().mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        if pin(&regular(&evidence, CHECKPOINT, false)?)? != self.journal_pin {
            return Err(invalid());
        }
        let root = File::from(open(&self.scope.path, DIRECTORY, Mode::empty())?);
        if pin(&root)? != self.root_pin
            || pin(&regular(&root, ".writer-lock", false)?)? != self.lock_pin
        {
            return Err(invalid());
        }
        for (id, a) in &self.base.allocations {
            if pin(&regular(&root, &pack_name(&id.to_string()), false)?)? != [a.device, a.inode] {
                return Err(invalid());
            }
        }
        for (path, original) in &self.base.registration.retained_files {
            let f = regular(&root, &source_name(&path.join("/")), false)?;
            if pin(&f)? != [original.device, original.inode]
                || f.metadata()?.len() != original.extent
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn current(&self) -> IoResult<OriginalPackage> {
        self.pins()?;
        let mut p = package(&self.base);
        p.head = serde_json::from_slice(&read(&self.root, "HEAD.json")?).map_err(|_| invalid())?;
        p.commit = serde_json::from_slice(&read(
            &self.root,
            &format!(
                "{}.commit",
                p.head["commit_sha256"].as_str().ok_or_else(invalid)?
            ),
        )?)
        .map_err(|_| invalid())?;
        if read(&self.root, "manifest.json")? != canonical(&p.manifest) {
            return Err(invalid());
        }
        p.loose.clear();
        let mut known = std::collections::BTreeSet::from([
            ".owner".to_owned(),
            ".writer-lock".into(),
            "manifest.json".into(),
            "HEAD.json".into(),
            "HEAD.next".into(),
        ]);
        known.extend(
            self.base
                .allocations
                .keys()
                .map(|id| pack_name(&id.to_string())),
        );
        known.extend(
            self.base
                .registration
                .retained_files
                .keys()
                .map(|path| source_name(&path.join("/"))),
        );
        for entry in std::fs::read_dir(&self.scope.path)? {
            let name = entry?.file_name().into_string().map_err(|_| invalid())?;
            if !known.contains(&name) && !name.ends_with(".control") && !name.ends_with(".commit") {
                return Err(invalid());
            }
            if let Some(sha) = name.strip_suffix(".commit") {
                let bytes = read(&self.root, &name)?;
                if hash(&bytes) != sha
                    || (!self.commits.is_empty() && self.commits.get(sha) != Some(&bytes))
                {
                    return Err(invalid());
                }
            }
            if let Some(sha) = name.strip_suffix(".control") {
                let bytes = read(&self.root, &name)?;
                if hash(&bytes) != sha {
                    return Err(invalid());
                }
                package::parse_canonical_json(&bytes, support::limits().json)
                    .map_err(|_| invalid())?;
                p.loose.insert(sha.into(), bytes);
            }
        }
        // Extra controls are accepted only as exact known original-plan coexistence.
        let overlay = p.commit["root_set"]["inventory"]["sha256"]
            .as_str()
            .ok_or_else(invalid)?;
        let raw = p.loose.get(overlay).ok_or_else(invalid)?;
        let value: Value = serde_json::from_slice(raw).map_err(|_| invalid())?;
        let mut selected = std::collections::BTreeSet::from([overlay.to_owned()]);
        for r in value["controls"].as_array().ok_or_else(invalid)? {
            selected.insert(r["sha256"].as_str().ok_or_else(invalid)?.into());
        }
        for (sha, bytes) in &p.loose {
            if !selected.contains(sha) && self.allowed.get(sha) != Some(bytes) {
                return Err(invalid());
            }
        }
        p.loose.retain(|sha, _| selected.contains(sha));
        for (id, a) in &mut p.allocations {
            a.bytes = read(&self.root, &pack_name(id))?;
        }
        package::PackageUuid::parse(
            p.commit["root_set"]["library_id"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(|_| invalid())?;
        package::DecimalU64::parse(p.commit["package_revision"].as_str().ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        Ok(p)
    }
    fn check(
        &self,
        p: &OriginalPackage,
        plan: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), ExecutionError> {
        self.pins()
            .map_err(|_| ExecutionError::InvalidObservation)?;
        let observed = snapshot(&self.base, p);
        self.footprint(&observed)
            .map_err(|_| ExecutionError::InvalidObservation)?;
        v1_3::reader::inspect_operation_package(
            p,
            identity(p),
            &observed,
            &self.base.registration,
            frames(),
            &support::limits(),
            plan,
            stage,
        )
        .map(|_| ())
        .map_err(|_| ExecutionError::InvalidObservation)
    }
    fn allow(&mut self, plan: &RepeatablePlan) {
        self.allowed = plan.old.loose.clone();
        self.commits.insert(
            hash(&canonical(&plan.old.commit)),
            canonical(&plan.old.commit),
        );
        for stage in &plan.stages {
            self.allowed.extend(stage.loose.clone());
            self.commits
                .insert(hash(&canonical(&stage.commit)), canonical(&stage.commit));
        }
    }
    fn footprint(&self, ceiling: &support::Fixture) -> IoResult<Value> {
        use v1_3::native_profile::{check_charge, measure_allocation};
        let unit = self.base.registration.charge_unit;
        let mut allocations = Vec::new();
        let mut standing = vec![
            measure_allocation(&self.root).map_err(|_| invalid())?,
            measure_allocation(&self.evidence).map_err(|_| invalid())?,
        ];
        let mut owned = std::collections::BTreeSet::new();
        for (id, a) in &ceiling.allocations {
            let name = pack_name(&id.to_string());
            let usage =
                measure_allocation(&regular(&self.root, &name, false)?).map_err(|_| invalid())?;
            let charge = check_charge(&[usage], unit, a.charge).map_err(|_| invalid())?;
            allocations.push(json!({"name":name,"charge":charge}));
            owned.insert(name);
        }
        for (path, a) in &self.base.registration.retained_files {
            let name = source_name(&path.join("/"));
            let usage =
                measure_allocation(&regular(&self.root, &name, false)?).map_err(|_| invalid())?;
            let charge =
                check_charge(&[usage], unit, a.registered_charge).map_err(|_| invalid())?;
            allocations.push(json!({"name":name,"charge":charge}));
            owned.insert(name);
        }
        for entry in std::fs::read_dir(&self.scope.path)? {
            let name = entry?.file_name().into_string().map_err(|_| invalid())?;
            if !owned.contains(&name) {
                if !matches!(
                    name.as_str(),
                    ".owner" | ".writer-lock" | "manifest.json" | "HEAD.json" | "HEAD.next"
                ) && !name
                    .strip_suffix(".control")
                    .is_some_and(|sha| self.allowed.contains_key(sha))
                    && !name
                        .strip_suffix(".commit")
                        .is_some_and(|sha| self.commits.contains_key(sha))
                {
                    return Err(invalid());
                }
                standing.push(
                    measure_allocation(&regular(&self.root, &name, false)?)
                        .map_err(|_| invalid())?,
                );
            }
        }
        for entry in
            std::fs::read_dir(self.scope.path.parent().ok_or_else(invalid)?.join(EVIDENCE))?
        {
            if entry?.file_name() != CHECKPOINT {
                return Err(invalid());
            }
        }
        standing.push(
            measure_allocation(&regular(&self.evidence, CHECKPOINT, false)?)
                .map_err(|_| invalid())?,
        );
        let charged = check_charge(&standing, unit, self.base.registration.standing_control)
            .map_err(|_| invalid())?;
        Ok(json!({"allocation_charges":allocations,"standing_observation":charged}))
    }
    fn check_effect_footprint(&self) -> IoResult<()> {
        self.footprint(self.ceiling.as_ref().ok_or_else(invalid)?)
            .map(|_| ())
    }
    fn journal_bounds(&self) -> journal::JournalLimits {
        let mut limits = journal_limits(self.base.registration.standing_control);
        if self.session_mode {
            limits.max_records = budget().max_entries;
        }
        limits
    }
    fn checkpoint(&self) -> IoResult<journal::Journal> {
        let bytes = read(&self.evidence, CHECKPOINT)?;
        let parsed = journal::Journal::read(&bytes, &self.journal_header, self.journal_bounds())
            .map_err(|_| invalid())?;
        if parsed.tail() != journal::JournalTail::Complete {
            return Err(invalid());
        }
        Ok(parsed)
    }
    fn append_checkpoint(&mut self, append: &journal::JournalAppend) -> IoResult<()> {
        self.pins()?;
        let bytes = read(&self.evidence, CHECKPOINT)?;
        if bytes.len() != append.expected_extent()
            || hash(&bytes) != append.expected_prefix_sha256()
        {
            return Err(invalid());
        }
        let file = regular(&self.evidence, CHECKPOINT, false)?;
        self.journal_high_water = self.journal_high_water.max(
            file.metadata()?
                .blocks()
                .checked_mul(512)
                .ok_or_else(invalid)?,
        );
        if self.journal_high_water > self.journal_capacity {
            return Err(invalid());
        }
        append_exact(&self.evidence, CHECKPOINT, &bytes, append.bytes())?;
        self.journal_high_water = self.journal_high_water.max(
            file.metadata()?
                .blocks()
                .checked_mul(512)
                .ok_or_else(invalid)?,
        );
        if self.journal_high_water > self.journal_capacity {
            return Err(invalid());
        }
        self.check_effect_footprint()
    }
    #[cfg(test)]
    fn prepare_checkpoint(&mut self, plan: &RepeatablePlan) -> IoResult<()> {
        self.prepare_checkpoint_with(plan, None)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Keep exact journal preflight and original-bound append ordering together"
    )]
    fn prepare_checkpoint_with(
        &mut self,
        plan: &RepeatablePlan,
        session: Option<(&VerifiedOriginal, &PlannerInputs)>,
    ) -> IoResult<()> {
        if self.session_mode {
            if plan.original()["project_limit"]
                != json!(self.session_project_limit.ok_or_else(invalid)?.to_string())
            {
                return Err(invalid());
            }
            // Only the independently registered prepaid native corridor is
            // available to this bounded lab. No future Accepted can grow it.
            for (id, file) in &selected(plan, 6).allocations {
                let key = package::PackageUuid::parse(id).map_err(|_| invalid())?;
                let captured = self.base.allocations.get(&key).ok_or_else(invalid)?;
                if u64::try_from(file.bytes.len()).map_err(|_| invalid())? > captured.charge {
                    return Err(std::io::Error::other(
                        "native prepaid corridor exhausted before admission",
                    ));
                }
            }
        }
        let current = self.current()?;
        let observed = snapshot(&self.base, &current);
        let identity = identity(&current);
        let reader = support::limits();
        let context = RestartContext {
            inspection: &observed,
            registration: &self.base.registration,
            directory: v1_3::DirectoryObservation {
                device: self.root_pin[0],
                inode: self.root_pin[1],
            },
            identity: &identity,
            frames: frames(),
            reader: &reader,
            planner: budget(),
            max_originals: self.journal_bounds().max_records,
        };
        let limits = self.journal_bounds();
        let parsed = self.checkpoint()?;
        let verified = parsed.verify(&current, &context).map_err(|_| invalid())?;
        let prior_intent = parsed
            .records()
            .iter()
            .find(|r| {
                r.value()["kind"] == "CheckpointIntent"
                    && r.value()["body"]["operation_receipt"]["operation_id"]
                        == plan.prepared.receipt["operation_id"]
            })
            .map(journal::JournalRecord::value);
        let identity = record_identity(prior_intent, self.scope.generation)?;
        let intent = if let Some((base, inputs)) = session {
            verified
                .verify_session(base, inputs, budget())
                .map_err(|_| invalid())?
                .prepare_checkpoint_intent(&verified, plan, identity, limits)
        } else {
            verified.prepare_intent(plan, identity, limits)
        }
        .map_err(|_| invalid())?;
        let mut planned = read(&self.evidence, CHECKPOINT)?;
        planned.extend(intent.bytes());
        let intent_id = package::PackageUuid::parse(
            intent.record().value()["record_id"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(|_| invalid())?;
        let next = journal::Journal::read(&planned, &self.journal_header, limits)
            .map_err(|_| invalid())?;
        let verified_next = next.verify(&current, &context).map_err(|_| invalid())?;
        let prior_receipt = next
            .records()
            .iter()
            .find(|r| {
                r.value()["kind"] == "CheckpointReceipt"
                    && r.value()["body"]["operation_receipt"]["operation_id"]
                        == plan.prepared.receipt["operation_id"]
            })
            .map(journal::JournalRecord::value);
        let receipt_identity = record_identity(prior_receipt, self.scope.generation)?;
        let completion = verified_next
            .prepare_receipt(plan, intent_id, receipt_identity, limits)
            .map_err(|_| invalid())?;
        let header_len = journal::Journal::create(&self.journal_header, limits)
            .map_err(|_| invalid())?
            .len();
        let mut intent_end = header_len;
        for record in next.records() {
            intent_end = intent_end
                .checked_add(36)
                .and_then(|n| n.checked_add(canonical(record.value()).len()))
                .ok_or_else(invalid)?;
            if record.value()["record_id"] == intent.record().value()["record_id"] {
                break;
            }
        }
        let prefix = journal::Journal::read(
            planned.get(..intent_end).ok_or_else(invalid)?,
            &self.journal_header,
            limits,
        )
        .map_err(|_| invalid())?;
        let worst = prefix
            .verify(&current, &context)
            .map_err(|_| invalid())?
            .prepare_receipt(
                plan,
                intent_id,
                journal::JournalRecordIdentity {
                    session_generation: u64::MAX,
                    ..receipt_identity
                },
                limits,
            )
            .map_err(|_| invalid())?;
        let extent = u64::try_from(
            planned
                .len()
                .checked_add(worst.bytes().len())
                .and_then(|n| n.checked_add(self.reserved_journal_extra))
                .ok_or_else(invalid)?,
        )
        .map_err(|_| invalid())?;
        let unit = self.base.registration.charge_unit;
        // Fixed fixture reserve, retained in full: rounded complete journal plus
        // one allocation-slack unit and four fixed namespace/control units.
        // Two directories, owner and lock are conservatively charged in addition
        // to the planner pool. This is not a universal APFS metadata bound.
        let capacity = extent
            .checked_add(unit - 1)
            .and_then(|n| n.checked_div(unit))
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(unit))
            .ok_or_else(invalid)?;
        let namespace = unit.checked_mul(4).ok_or_else(invalid)?;
        let fixed = [&self.root, &self.evidence];
        let mut namespace_usage = fixed
            .into_iter()
            .map(|file| v1_3::native_profile::measure_allocation(file).map_err(|_| invalid()))
            .collect::<IoResult<Vec<_>>>()?;
        for name in [".owner", ".writer-lock"] {
            namespace_usage.push(
                v1_3::native_profile::measure_allocation(&regular(&self.root, name, false)?)
                    .map_err(|_| invalid())?,
            );
        }
        v1_3::native_profile::check_charge(&namespace_usage, unit, namespace)
            .map_err(|_| invalid())?;
        let peak = plan
            .control_peak()
            .checked_add(capacity)
            .and_then(|n| n.checked_add(namespace))
            .ok_or_else(invalid)?;
        if peak > self.base.registration.standing_control
            || peak > number(&plan.original()["control_bounds"]["aggregate_control_bytes"])?
            || plan.control_role_peak.checked_add(5).ok_or_else(invalid)?
                > number(&plan.original()["control_bounds"]["aggregate_control_count"])?
        {
            return Err(std::io::Error::other(format!(
                "checkpoint standing refusal: controls={} journal={} namespace={} total={} limit={}",
                plan.control_peak(),
                capacity,
                unit,
                peak,
                self.base.registration.standing_control
            )));
        }
        // Partition the already registered standing pool before effects. APFS
        // can allocate more blocks than the requested logical append; a single
        // slack block is not a physical bound. The exact logical worst case
        // above must fit, and every actual measurement stays within this fixed
        // remainder. Neither original R nor its aggregate cap changes.
        let allowance = self.base.registration.standing_control.min(number(
            &plan.original()["control_bounds"]["aggregate_control_bytes"],
        )?);
        let capacity = if self.session_mode {
            allowance
                .checked_sub(plan.control_peak())
                .and_then(|n| n.checked_sub(namespace))
                .map(|n| n / unit * unit)
                .ok_or_else(invalid)?
        } else {
            capacity
        };
        if extent > capacity || self.journal_high_water > capacity {
            return Err(invalid());
        }
        self.footprint(&snapshot(&self.base, &selected(plan, 6)))?;
        self.namespace_allowance = namespace;
        self.ceiling = Some(snapshot(&self.base, &selected(plan, 6)));
        self.journal_capacity = capacity;
        self.intent_record = Some(intent.record().value().clone());
        self.published_head.clone_from(&plan.stages[5].head);
        self.completed_head.clone_from(&plan.stages[6].head);
        self.completion = Some(completion);
        self.append_checkpoint(&intent)?;
        self.footprint(&snapshot(&self.base, &selected(plan, 6)))?;
        Ok(())
    }
    fn prune_commits(&self, keep: &Value) -> IoResult<()> {
        let current = hash(&canonical(keep));
        for sha in self.commits.keys() {
            if *sha != current {
                match unlinkat(&self.root, format!("{sha}.commit"), AtFlags::empty()) {
                    Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        barrier(&self.root)
    }
    fn lease_ok(&self, lease: &NativeLease) -> Result<(), ExecutionError> {
        self.pins().map_err(|_| ExecutionError::Authorization)?;
        if pin(lease.platform_lease()).ok() != Some(self.lock_pin) {
            return Err(ExecutionError::Authorization);
        }
        Ok(())
    }
}
fn uncertain<T>(r: IoResult<T>) -> Result<T, EffectFailure> {
    r.map_err(|error| {
        eprintln!("native effect outcome unknown: {error}");
        EffectFailure::OutcomeUnknown
    })
}
impl RepeatableIo for Native {
    type Lease = File;
    fn authorize(&mut self, l: &NativeLease, o: &Value, r: &Value) -> Result<(), ExecutionError> {
        self.lease_ok(l)?;
        let (_, mut expected) = request();
        expected["provenance"]["effective_scope"]["project_id"] =
            self.journal_header["project_id"].clone();
        if r["provenance"]
            != self
                .local_provenance
                .as_ref()
                .unwrap_or(&expected["provenance"])
                .clone()
            || o["request"]["receipt"] != reference(r)
        {
            return Err(ExecutionError::Authorization);
        }
        Ok(())
    }
    fn observe(&mut self, l: &NativeLease) -> Result<Observation, ExecutionError> {
        self.lease_ok(l)?;
        let p = self
            .current()
            .map_err(|_| ExecutionError::InvalidObservation)?;
        Ok(Observation {
            head: p.head,
            allocations: p
                .allocations
                .into_iter()
                .map(|(id, a)| (id, a.bytes))
                .collect(),
        })
    }
    fn verify_prospective(
        &mut self,
        l: &NativeLease,
        p: &RepeatablePlan,
    ) -> Result<(), ExecutionError> {
        self.lease_ok(l)?;
        self.allow(p);
        self.footprint(&snapshot(&self.base, &selected(p, 6)))
            .map_err(|_| ExecutionError::InvalidObservation)?;
        self.check(&selected(p, 6), p, 6)
    }
    fn verify_progress(
        &mut self,
        l: &NativeLease,
        p: &RepeatablePlan,
        stage: Option<usize>,
    ) -> Result<(), ExecutionError> {
        self.lease_ok(l)?;
        let current = self
            .current()
            .map_err(|_| ExecutionError::InvalidObservation)?;
        let (head, commit, loose) = stage.map_or((&p.old.head, &p.old.commit, &p.old.loose), |n| {
            (&p.stages[n].head, &p.stages[n].commit, &p.stages[n].loose)
        });
        if current.head != *head || current.commit != *commit || current.loose != *loose {
            return Err(ExecutionError::InvalidObservation);
        }
        let checkpoint = self
            .checkpoint()
            .map_err(|_| ExecutionError::InvalidObservation)?;
        let intent = self
            .intent_record
            .as_ref()
            .ok_or(ExecutionError::InvalidObservation)?;
        if !checkpoint.records().iter().any(|r| r.value() == intent)
            || intent["body"]["original"] != p.original
            || intent["body"]["operation_receipt"] != p.prepared.receipt
            || checkpoint.records().iter().any(|r| {
                r.value()["kind"] == "CheckpointReceipt"
                    && r.value()["body"]["operation_receipt"]["operation_id"]
                        == p.prepared.receipt["operation_id"]
                    && (stage != Some(6)
                        || self
                            .completion
                            .as_ref()
                            .is_none_or(|c| c.record().value() != r.value()))
            })
        {
            return Err(ExecutionError::InvalidObservation);
        }
        // A clean selected HEAD may precede its wrapper after interruption.
        // execute revalidates closure and barriers before record_receipt repairs it.
        match read(&self.root, "HEAD.next") {
            Ok(bytes) if p.stages.iter().any(|s| canonical(&s.head) == bytes) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(ExecutionError::InvalidObservation),
        }
        Ok(())
    }
    fn stabilize_progress(
        &mut self,
        l: &NativeLease,
        p: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        uncertain((|| {
            // The executor already proved exact original-bound current suffixes.
            // Reopening matching bytes is not a substitute for these new barriers.
            for id in p.files.keys() {
                barrier(&regular(&self.root, &pack_name(id), true)?)?;
            }
            for sha in p.stages[stage].loose.keys() {
                barrier(&regular(&self.root, &format!("{sha}.control"), true)?)?;
            }
            let commit = canonical(&p.stages[stage].commit);
            barrier(&regular(
                &self.root,
                &format!("{}.commit", hash(&commit)),
                true,
            )?)?;
            barrier(&regular(&self.root, "HEAD.json", true)?)?;
            barrier(&regular(&self.evidence, CHECKPOINT, true)?)?;
            barrier(&self.root)?;
            barrier(&self.evidence)?;
            // Remove only previously authenticated operation-owned sidecars.
            match read(&self.root, "HEAD.next") {
                Ok(bytes) if p.stages.iter().any(|s| canonical(&s.head) == bytes) => {
                    unlinkat(&self.root, "HEAD.next", AtFlags::empty())?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(invalid()),
            }
            for sha in self.allowed.keys() {
                if !p.stages[stage].loose.contains_key(sha) {
                    match unlinkat(&self.root, format!("{sha}.control"), AtFlags::empty()) {
                        Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            self.prune_commits(&p.stages[stage].commit)?;
            barrier(&self.root)?;
            self.check_effect_footprint()
        })())
    }
    fn select(
        &mut self,
        l: &NativeLease,
        expected: &Value,
        stage: &SelectedStage,
    ) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        uncertain((|| {
            if read(&self.root, "HEAD.json")? != canonical(expected) {
                return Err(invalid());
            }
            for (sha, b) in &stage.loose {
                exact_file(&self.root, &format!("{sha}.control"), b)?;
            }
            let commit = canonical(&stage.commit);
            exact_file(&self.root, &format!("{}.commit", hash(&commit)), &commit)?;
            exact_file(&self.root, "HEAD.next", &canonical(&stage.head))?;
            if read(&self.root, "HEAD.json")? != canonical(expected) {
                return Err(invalid());
            }
            renameat(&self.root, "HEAD.next", &self.root, "HEAD.json")?;
            barrier(&self.root)?;
            for sha in self.allowed.keys() {
                if !stage.loose.contains_key(sha) {
                    match unlinkat(&self.root, format!("{sha}.control"), AtFlags::empty()) {
                        Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            self.prune_commits(&stage.commit)?;
            barrier(&self.root)?;
            self.check_effect_footprint()
        })())
    }
    fn journal(&mut self, l: &NativeLease, frame: &Value) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        let intent = self
            .intent_record
            .as_ref()
            .ok_or(EffectFailure::NotPerformed)?;
        if intent["body"]["accepted_frame"] != *frame {
            return Err(EffectFailure::NotPerformed);
        }
        uncertain((|| {
            self.checkpoint()?;
            barrier(&regular(&self.evidence, CHECKPOINT, true)?)?;
            barrier(&self.evidence)
        })())
    }

    fn append(
        &mut self,
        l: &NativeLease,
        id: &str,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        let interrupt = self.interrupt;
        self.interrupt = false;
        uncertain((|| {
            let mut f = regular(&self.root, &pack_name(id), true)?;
            if f.metadata()?.len() != offset as u64 {
                return Err(invalid());
            }
            f.seek(SeekFrom::Start(offset as u64))?;
            f.write_all(if interrupt {
                &bytes[..bytes.len() / 2]
            } else {
                bytes
            })?;
            barrier(&f)?;
            self.check_effect_footprint()
        })())?;
        if interrupt {
            Err(EffectFailure::OutcomeUnknown)
        } else {
            Ok(())
        }
    }
    fn verify_selected(
        &mut self,
        l: &NativeLease,
        p: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), ExecutionError> {
        self.lease_ok(l)?;
        self.check(
            &self
                .current()
                .map_err(|_| ExecutionError::InvalidObservation)?,
            p,
            stage,
        )
    }
    fn record_receipt(
        &mut self,
        l: &NativeLease,
        o: &Value,
        r: &Value,
    ) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        if o["request"]["receipt"] != reference(r) {
            return Err(EffectFailure::NotPerformed);
        }
        uncertain((|| {
            let head: Value =
                serde_json::from_slice(&read(&self.root, "HEAD.json")?).map_err(|_| invalid())?;
            if head == self.published_head {
                // Intent already contains the exact original receipt. Stage 5
                // barriers it; only selected clean stage 6 gets the wrapper.
                barrier(&regular(&self.evidence, CHECKPOINT, true)?)?;
                return barrier(&self.evidence);
            }
            if head != self.completed_head {
                return Err(invalid());
            }
            barrier(&regular(&self.root, "HEAD.json", true)?)?;
            barrier(&self.root)?;
            let completion = self.completion.take().ok_or_else(invalid)?;
            let result = (|| {
                let parsed = self.checkpoint()?;
                if parsed
                    .records()
                    .iter()
                    .any(|record| record.value() == completion.record().value())
                {
                    barrier(&regular(&self.evidence, CHECKPOINT, true)?)?;
                    barrier(&self.evidence)
                } else {
                    self.append_checkpoint(&completion)
                }
            })();
            self.completion = Some(completion);
            result
        })())
    }

    fn cleanup_controls(
        &mut self,
        l: &NativeLease,
        _: &RepeatablePlan,
    ) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        uncertain(barrier(&self.root))
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "Keep the existing disposable phase dispatcher and persisted-retry checks together"
)]
#[cfg(test)]
fn run(mut io: Native, phase: &str) -> IoResult<()> {
    if phase == "complete"
        && io.session_mode
        && (io.journal_header["project_id"] != request().0["project_id"]
            || io.journal_header["library_id"] != request().0["library_id"])
        && io.checkpoint()?.records().is_empty()
    {
        return session_host::bootstrap(io);
    }
    let actual = io.current()?;
    let original_record = actual
        .loose
        .values()
        .filter_map(|b| serde_json::from_slice::<Value>(b).ok())
        .find(|v| v["schema"]["id"] == "photara.storage.original-admission");
    let observed = snapshot(&io.base, &actual);
    let id = identity(&actual);
    let reader = support::limits();
    let directory = v1_3::DirectoryObservation {
        device: io.root_pin[0],
        inode: io.root_pin[1],
    };
    let checkpoint = io.checkpoint()?;
    let context = RestartContext {
        inspection: &observed,
        registration: &io.base.registration,
        directory,
        identity: &id,
        frames: frames(),
        reader: &reader,
        planner: budget(),
        max_originals: 2,
    };
    checkpoint
        .verify(&actual, &context)
        .map_err(|_| invalid())?;
    let persisted = checkpoint
        .records()
        .iter()
        .find(|r| r.value()["kind"] == "CheckpointIntent");
    let plan = if let Some(record) = persisted {
        let body = &record.value()["body"];
        restart::restore_intent_plan(
            &actual,
            &body["original"],
            &body["semantic_intent"],
            &body["operation_receipt"],
            &context,
        )
        .map_err(|_| invalid())?
    } else {
        if original_record.is_some() {
            return Err(invalid());
        }
        let original = v1_3::verify_original(
            actual.clone(),
            identity(&actual),
            &observed,
            &io.base.registration,
            directory,
            frames(),
            &reader,
        )
        .map_err(|_| invalid())?;
        let (intent, receipt) = request();
        plan::compile_inner(
            &original,
            &intent,
            &receipt,
            io.attempt(&original, 1000)?,
            budget(),
        )
        .map_err(|_| invalid())?
    };
    let receipt = plan.prepared.receipt.clone();
    if actual.head == plan.old.head {
        if actual.commit != plan.old.commit
            || actual.loose != plan.old.loose
            || actual.allocations.len() != plan.old.allocations.len()
            || actual.allocations.iter().any(|(id, a)| {
                plan.old.allocations.get(id).is_none_or(|old| {
                    old.bytes != a.bytes || old.witness != a.witness || old.arena != a.arena
                })
            })
        {
            return Err(invalid());
        }
    } else if restore_plan(&actual, &context)
        .map_err(|_| invalid())?
        .original()
        != plan.original()
    {
        return Err(invalid());
    }
    io.allow(&plan);
    let lease = io.lease(&plan.old)?;
    io.authorize(&lease, plan.original(), &receipt)
        .map_err(|_| invalid())?;
    io.prepare_checkpoint(&plan)?;
    if phase == "verify" {
        let actual = io.current()?;
        io.verify_progress(&lease, &plan, Some(6))
            .map_err(|_| invalid())?;
        io.check(&actual, &plan, 6).map_err(|_| invalid())?;
        if !io
            .checkpoint()?
            .records()
            .iter()
            .any(|r| r.value()["kind"] == "CheckpointReceipt")
        {
            return Err(invalid());
        }
        let report = json!({"binding_generation":io.scope.generation,"head_sha256":hash(&canonical(&actual.head)),"closure_valid":true,"original_sha256":hash(&canonical(plan.original())),"receipt_sha256":hash(&canonical(&receipt)),"standing_control":io.base.registration.standing_control,"checkpoint_capacity_charge":io.journal_capacity,"checkpoint_observed_high_water":io.journal_high_water,"checkpoint_namespace_allowance":io.namespace_allowance,"native_profile":io.scope.assessment.report(),"native_footprint":io.footprint(&snapshot(&io.base,&actual))?,"native_charge_profile_qualified":false,"qualified":false,"saved_claim":false,"power_loss_tested":false,"remount_tested":false});
        exact_file(
            &io.scope.scratch,
            &format!("repeatable-verified-{}.json", io.scope.generation),
            &canonical(&report),
        )?;
        println!("{report}");
        return Ok(());
    }
    if phase == "interrupt" {
        io.interrupt = true;
        let result = execute(&mut io, &lease, &plan);
        if !matches!(
            result,
            Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
        ) {
            return Err(invalid());
        }
        return Ok(());
    }
    if phase != "complete" && phase != "retry" {
        return Err(invalid());
    }
    let result = execute(&mut io, &lease, &plan).map_err(|_| invalid())?;
    if result.receipt() != &receipt {
        return Err(invalid());
    }
    Ok(())
}
#[cfg(test)]
pub(super) fn native_repeatable_phase() {
    let scope = Scope::load().unwrap();
    let phase = std::env::var("PHOTARA_PS2_REPEATABLE_PHASE").unwrap();
    if phase == "prepare" {
        prepare(&scope, false).unwrap();
    } else if phase == "prepare-session" {
        prepare(&scope, true).unwrap();
    } else if phase == "session" {
        session_host::run(Native::load(scope).unwrap()).unwrap();
    } else {
        run(Native::load(scope).unwrap(), &phase).unwrap();
    }
}

#[path = "native_activation.rs"]
pub(super) mod activation_host;
#[path = "native_selection.rs"]
pub(super) mod selection_host;
#[path = "native_session.rs"]
pub(super) mod session_host;

pub(super) fn open_controlled(path: &Path, binding: Value) -> IoResult<session_host::OpenSession> {
    let scope = Scope::load_explicit(path, binding)
        .map_err(|e| std::io::Error::new(e.kind(), format!("controller scope: {e}")))?;
    let native = Native::load(scope)
        .map_err(|e| std::io::Error::new(e.kind(), format!("captured registration: {e}")))?;
    session_host::OpenSession::open(native)
}

pub(super) fn open_local_controlled(
    path: &Path,
    binding: Value,
    authority: std::sync::Arc<dyn Fn() -> IoResult<Value> + Send + Sync>,
) -> IoResult<session_host::OpenSession> {
    let mut native = Native::load(Scope::load_explicit(path, binding)?)?;
    native.local_provenance = Some(authority()?);
    native.local_authority = Some(authority);
    session_host::OpenSession::open(native)
}
pub(super) fn local_database_config(
    manifest: &Path,
    binding: Value,
    config: &Path,
) -> IoResult<Value> {
    let scope = Scope::load_explicit(manifest, binding)?;
    let parent = manifest.parent().ok_or_else(invalid)?;
    if config != parent.join("ll2a-database-registration.json") {
        return Err(invalid());
    }
    let raw = read(&scope.scratch, "ll2a-database-registration.json")?;
    if raw.len() > 65536 {
        return Err(invalid());
    }
    let v: Value = serde_json::from_slice(&raw).map_err(|_| invalid())?;
    if v.as_object().is_none_or(|o| o.len() != 3)
        || v["path"]
            != parent
                .join("mount/library/library.sqlite")
                .to_string_lossy()
                .as_ref()
    {
        return Err(invalid());
    }
    scope
        .assessment
        .check_pins()
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    Ok(v)
}
