//! Opt-in disposable APFS adapter for the existing repeatable test path.
//! No mount commands, production admission, storage qualification, or Saved claim.
use super::*;
use rustix::fs::{
    AtFlags, FlockOperation, Mode, OFlags, fcntl_fullfsync, flock, fstatfs, fsync, mkdirat, open,
    openat, renameat, unlinkat,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};
type IoResult<T> = std::io::Result<T>;
type Pin = [u64; 2];
const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const PACKAGE: &str = "repeatable-package";
const EVIDENCE: &str = "repeatable-evidence";
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
        Ok(_) => {
            if read(root, name)? != bytes {
                return Err(invalid());
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            create(root, name, bytes)?;
            barrier(root)?;
        }
        Err(e) => return Err(e),
    }
    Ok(())
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
}
impl Scope {
    fn load() -> IoResult<Self> {
        let path =
            PathBuf::from(std::env::var("PHOTARA_PS2_REMOUNT_MANIFEST").map_err(|_| invalid())?);
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
        let binding: Value = serde_json::from_str(
            &std::env::var("PHOTARA_PS2_REMOUNT_BINDING").map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
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
        Ok(Self {
            scratch,
            mount,
            path: parent.join("mount").join(PACKAGE),
            nonce,
            generation,
        })
    }
}
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
fn prepare(scope: &Scope) -> IoResult<()> {
    mkdirat(&scope.mount, EVIDENCE, Mode::RWXU)?;
    let evidence = File::from(openat(&scope.mount, EVIDENCE, DIRECTORY, Mode::empty())?);
    barrier(&evidence)?;
    barrier(&scope.mount)?;
    mkdirat(&scope.mount, PACKAGE, Mode::RWXU)?;
    let root = File::from(openat(&scope.mount, PACKAGE, DIRECTORY, Mode::empty())?);
    create(&root, ".owner", scope.nonce.as_bytes())?;
    let lock = create(&root, ".writer-lock", b"")?;
    let base = support::Fixture::load("integrated");
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
    create(
        &scope.scratch,
        "repeatable-native-pins.json",
        &canonical(&captured),
    )?;
    barrier(&scope.scratch)?;
    let output = Command::new("python3")
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
    // Trusted experimental registration captured before any repeatable writer effect.
    let registration = json!({"nonce":scope.nonce,"package_pin":pin(&root)?,"lock_pin":pin(&lock)?,"evidence_pin":pin(&evidence)?,"generation":scope.generation,"data":data});
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
}
type NativeLease = crate::package::planning::io::RegisteredCooperativeLease<File>;
impl Native {
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
        let root_pin = pin(&root)?;
        let lock_pin = pin(&lock)?;
        Ok(Self {
            scope,
            root,
            evidence,
            evidence_pin,
            base: support::Fixture::from_value(registration["data"].clone()),
            root_pin,
            lock_pin,
            interrupt: false,
            allowed: BTreeMap::new(),
            commits: BTreeMap::new(),
        })
    }
    fn lease(&self, p: &OriginalPackage) -> IoResult<NativeLease> {
        let lock = regular(&self.root, ".writer-lock", false)?;
        flock(&lock, FlockOperation::NonBlockingLockExclusive)?;
        Ok(crate::package::planning::io::test_lease(
            lock,
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
    r.map_err(|_| EffectFailure::OutcomeUnknown)
}
impl RepeatableIo for Native {
    type Lease = File;
    fn authorize(&mut self, l: &NativeLease, o: &Value, r: &Value) -> Result<(), ExecutionError> {
        self.lease_ok(l)?;
        let (_, expected) = request();
        if r["provenance"] != expected["provenance"] || o["request"]["receipt"] != reference(r) {
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
        let id = p.prepared.receipt["operation_id"]
            .as_str()
            .ok_or(ExecutionError::InvalidObservation)?;
        for (required, name, bytes) in [
            (
                stage.is_some_and(|n| n >= 1),
                format!("repeatable-journal-{id}.json"),
                canonical(&plan::journal_frame(&p.prepared.receipt)),
            ),
            (
                stage == Some(6),
                format!("repeatable-receipt-{id}.json"),
                canonical(&p.prepared.receipt),
            ),
        ] {
            match read(&self.evidence, &name) {
                Ok(actual) if actual == bytes => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {}
                _ => return Err(ExecutionError::InvalidObservation),
            }
        }
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
            let id = p.prepared.receipt["operation_id"]
                .as_str()
                .ok_or_else(invalid)?;
            if stage >= 1 {
                barrier(&regular(
                    &self.evidence,
                    &format!("repeatable-journal-{id}.json"),
                    true,
                )?)?;
            }
            if stage == 6 {
                barrier(&regular(
                    &self.evidence,
                    &format!("repeatable-receipt-{id}.json"),
                    true,
                )?)?;
            }
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
            barrier(&self.root)
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
            barrier(&self.root)
        })())
    }
    fn journal(&mut self, l: &NativeLease, frame: &Value) -> Result<(), EffectFailure> {
        self.lease_ok(l).map_err(|_| EffectFailure::NotPerformed)?;
        uncertain(exact_file(
            &self.evidence,
            &format!(
                "repeatable-journal-{}.json",
                frame["operation_id"]
                    .as_str()
                    .ok_or(EffectFailure::NotPerformed)?
            ),
            &canonical(frame),
        ))
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
            barrier(&f)
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
        uncertain(exact_file(
            &self.evidence,
            &format!(
                "repeatable-receipt-{}.json",
                r["operation_id"]
                    .as_str()
                    .ok_or(EffectFailure::NotPerformed)?
            ),
            &canonical(r),
        ))
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
fn run(mut io: Native, phase: &str) -> IoResult<()> {
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
    let plan = if original_record.is_some() {
        // Retry authority comes entirely from persisted original controls and
        // observed native bytes, never regenerated fixture planner inputs.
        restore_plan(
            &actual,
            &RestartContext {
                inspection: &observed,
                registration: &io.base.registration,
                directory,
                identity: &id,
                frames: frames(),
                reader: &reader,
                planner: budget(),
                max_originals: 2,
            },
        )
        .map_err(|_| invalid())?
    } else {
        let original = v1_3::verify_original(
            actual.clone(),
            id,
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
            attempt(&original, 1000),
            budget(),
        )
        .map_err(|_| invalid())?
    };
    let receipt = plan.prepared.receipt.clone();
    io.allow(&plan);
    let lease = io.lease(&plan.old)?;
    if phase == "verify" {
        let actual = io.current()?;
        io.verify_progress(&lease, &plan, Some(6))
            .map_err(|_| invalid())?;
        io.check(&actual, &plan, 6).map_err(|_| invalid())?;
        let report = json!({"binding_generation":io.scope.generation,"head_sha256":hash(&canonical(&actual.head)),"closure_valid":true,"original_sha256":hash(&canonical(plan.original())),"receipt_sha256":hash(&canonical(&receipt)),"standing_control":io.base.registration.standing_control,"native_charge_profile_qualified":false,"qualified":false,"saved_claim":false,"power_loss_tested":false,"remount_tested":false});
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
#[test]
#[ignore = "explicit owned APFS manifest/binding only; no image or mount commands"]
fn native_repeatable_phase() {
    let scope = Scope::load().unwrap();
    let phase = std::env::var("PHOTARA_PS2_REPEATABLE_PHASE").unwrap();
    if phase == "prepare" {
        prepare(&scope).unwrap();
    } else {
        run(Native::load(scope).unwrap(), &phase).unwrap();
    }
}
