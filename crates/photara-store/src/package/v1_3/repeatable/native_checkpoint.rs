//! Registered local checkpoint namespace. Names locate bytes, never authority.
use super::{
    BTreeMap, CHECKPOINT, DIRECTORY, EVIDENCE, File, IoResult, Mode, OFlags, Pin, Read, Seek,
    SeekFrom, Value, Write, barrier, hash, invalid, openat, pin, read, regular, renameat,
};
use crate::package::v1_3::repeatable::checkpoint::CheckpointRecords;
use crate::package::{PackageError, PackageUuid};
use std::os::unix::fs::MetadataExt;

pub(super) struct Namespace {
    parent: File,
    root: File,
    root_pin: Pin,
    selector: String,
    initial_stream: PackageUuid,
    initial_pin: Pin,
}

pub(super) fn safe_selector(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        && name != CHECKPOINT
}
fn record_name(bytes: &[u8]) -> String {
    format!("{}.checkpoint-record", hash(bytes))
}
fn segment_name(stream: PackageUuid) -> String {
    format!("{stream}.checkpoint-segment")
}

// Conservative logical coexistence preflight; actual native allocated charge
// is remeasured after every effect and can still refuse without changing R.
fn additional(root: &File, name: &str, bytes: &[u8], unit: u64) -> IoResult<u64> {
    let current = match regular(root, name, false) {
        Ok(file) => {
            if !bytes.starts_with(&read(root, name)?) {
                return Err(invalid());
            }
            file.metadata()?
                .blocks()
                .checked_mul(512)
                .ok_or_else(invalid)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(e),
    };
    if unit == 0 {
        return Err(invalid());
    }
    let target = (bytes.len() as u64)
        .max(1)
        .checked_add(unit - 1)
        .and_then(|n| (n / unit).checked_add(1))
        .and_then(|n| n.checked_mul(unit))
        .ok_or_else(invalid)?;
    Ok(target.saturating_sub(current))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Step {
    Write,
    FileBarrier,
    DirectoryBarrier,
    Rename,
    PublishedBarrier,
}

impl Namespace {
    pub(super) fn open(
        parent: File,
        root: File,
        selector: &str,
        initial_stream: PackageUuid,
        initial_pin: Pin,
    ) -> IoResult<Self> {
        if !safe_selector(selector) {
            return Err(invalid());
        }
        let this = Self {
            parent,
            root_pin: pin(&root)?,
            root,
            selector: selector.into(),
            initial_stream,
            initial_pin,
        };
        // Registration cannot be removed or converted to implicit initialization.
        this.selector()?;
        this.check()?;
        Ok(this)
    }
    fn check(&self) -> IoResult<()> {
        if pin(&self.root)? != self.root_pin
            || pin(&File::from(openat(
                &self.parent,
                EVIDENCE,
                DIRECTORY,
                Mode::empty(),
            )?))?
                != self.root_pin
            || pin(&regular(&self.root, CHECKPOINT, false)?)? != self.initial_pin
        {
            return Err(invalid());
        }
        Ok(())
    }
    /// All retained names, including pre-selector orphans, count toward standing
    /// storage. No record discovered by this scan grants a selected capability.
    pub(super) fn retained(&self) -> IoResult<Vec<File>> {
        self.check()?;
        let mut files = Vec::new();
        for entry in rustix::fs::Dir::read_from(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().to_str().map_err(|_| invalid())?;
            if matches!(name, "." | "..") {
                continue;
            }
            if files.len() >= super::budget().max_objects {
                return Err(invalid());
            }
            let known = name == CHECKPOINT
                || name == self.selector
                || name == format!("{}-next", self.selector)
                || name
                    .strip_suffix(".checkpoint-record")
                    .is_some_and(|s| crate::package::Sha256Hex::parse(s).is_ok())
                || name
                    .strip_suffix(".checkpoint-segment")
                    .is_some_and(|s| PackageUuid::parse(s).is_ok_and(|id| !id.as_uuid().is_nil()));
            if !known {
                return Err(invalid());
            }
            // Orphans may contain a torn write. Account them without parsing;
            // an exact-name retry must prove its existing bytes before reuse.
            files.push(regular(&self.root, name, false)?);
        }
        Ok(files)
    }
    pub(super) fn selector(&self) -> IoResult<Vec<u8>> {
        self.check()?;
        read(&self.root, &self.selector)
    }
    fn same(&self, name: &str, file: &File, identity: Pin) -> IoResult<()> {
        self.check()?;
        if pin(file)? != identity || pin(&regular(&self.root, name, false)?)? != identity {
            return Err(invalid());
        }
        Ok(())
    }
    fn install(
        &self,
        name: &str,
        bytes: &[u8],
        mut event: impl FnMut(Step, bool) -> IoResult<()>,
    ) -> IoResult<()> {
        self.check()?;
        let (mut file, fresh) = match regular(&self.root, name, true) {
            Ok(file) => (file, false),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
                File::from(openat(
                    &self.root,
                    name,
                    OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::RDWR
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )?),
                true,
            ),
            Err(e) => return Err(e),
        };
        let identity = pin(&file)?;
        let mut actual = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(bytes.len() as u64 + 1)
            .read_to_end(&mut actual)?;
        // A partial create is original-bound only by its exact candidate prefix.
        // Never truncate or repair a divergent existing file.
        if !bytes.starts_with(&actual) || actual.len() > bytes.len() {
            return Err(invalid());
        }
        if fresh || actual.len() < bytes.len() {
            event(Step::Write, false)?;
            self.same(name, &file, identity)?;
            file.seek(SeekFrom::Start(actual.len() as u64))?;
            file.write_all(&bytes[actual.len()..])?;
            event(Step::Write, true)?;
        }
        event(Step::FileBarrier, false)?;
        self.same(name, &file, identity)?;
        barrier(&file)?;
        event(Step::FileBarrier, true)?;
        event(Step::DirectoryBarrier, false)?;
        self.same(name, &file, identity)?;
        barrier(&self.root)?;
        event(Step::DirectoryBarrier, true)?;
        self.same(name, &file, identity)?;
        if read(&self.root, name)? != bytes {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn retain(
        &self,
        bytes: &[u8],
        event: impl FnMut(Step, bool) -> IoResult<()>,
    ) -> IoResult<()> {
        self.install(&record_name(bytes), bytes, event)
    }
    pub(super) fn install_segment(
        &self,
        stream: PackageUuid,
        bytes: &[u8],
        event: impl FnMut(Step, bool) -> IoResult<()>,
    ) -> IoResult<()> {
        if stream == self.initial_stream || stream.as_uuid().is_nil() {
            return Err(invalid());
        }
        self.install(&segment_name(stream), bytes, event)
    }
    pub(super) fn select(
        &self,
        old: &[u8],
        new: &[u8],
        mut event: impl FnMut(Step, bool) -> IoResult<()>,
    ) -> IoResult<()> {
        let actual = self.selector()?;
        if actual != old && actual != new {
            return Err(invalid());
        }
        if actual == old {
            let next = format!("{}-next", self.selector);
            self.install(&next, new, &mut event)?;
            event(Step::Rename, false)?;
            if self.selector()? != old {
                return Err(invalid());
            }
            renameat(&self.root, &next, &self.root, &self.selector)?;
            event(Step::Rename, true)?;
        }
        // Even exact-new retry must rebarrier the selected inode and namespace.
        event(Step::PublishedBarrier, false)?;
        let selected = regular(&self.root, &self.selector, true)?;
        let identity = pin(&selected)?;
        barrier(&selected)?;
        barrier(&self.root)?;
        self.same(&self.selector, &selected, identity)?;
        if self.selector()? != new {
            return Err(invalid());
        }
        event(Step::PublishedBarrier, true)
    }
    pub(super) fn records(&self) -> IoResult<Records> {
        self.retained()?;
        let mut names = BTreeMap::new();
        for entry in rustix::fs::Dir::read_from(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().to_str().map_err(|_| invalid())?;
            if name == CHECKPOINT
                || name.ends_with(".checkpoint-record")
                || name.ends_with(".checkpoint-segment")
            {
                if names.len() >= super::budget().max_objects {
                    return Err(invalid());
                }
                let file = regular(&self.root, name, false)?;
                names.insert(name.to_owned(), (pin(&file)?, file.metadata()?.len()));
            }
        }
        self.retained()?;
        Ok(Records {
            parent: self.parent.try_clone()?,
            root: self.root.try_clone()?,
            root_pin: self.root_pin,
            initial_stream: self.initial_stream,
            names,
            authority: None,
        })
    }
}

type Authority = (
    std::sync::Arc<dyn Fn() -> IoResult<Value> + Send + Sync>,
    Value,
);
pub(super) struct Records {
    parent: File,
    root: File,
    root_pin: Pin,
    initial_stream: PackageUuid,
    names: BTreeMap<String, (Pin, u64)>,
    authority: Option<Authority>,
}
impl Records {
    fn range(&self, name: &str, offset: u64, length: usize) -> Result<Vec<u8>, PackageError> {
        let fail = |_| PackageError::Integrity;
        if let Some((authority, expected)) = &self.authority
            && authority().map_err(fail)? != *expected
        {
            return Err(PackageError::Integrity);
        }
        let (identity, extent) = self.names.get(name).ok_or(PackageError::Integrity)?;
        let end = offset
            .checked_add(length as u64)
            .ok_or(PackageError::Limit)?;
        if end > *extent || pin(&self.root).map_err(fail)? != self.root_pin {
            return Err(PackageError::Integrity);
        }
        let named = File::from(
            openat(&self.parent, EVIDENCE, DIRECTORY, Mode::empty())
                .map_err(|_| PackageError::Integrity)?,
        );
        if pin(&named).map_err(fail)? != self.root_pin {
            return Err(PackageError::Integrity);
        }
        let mut file = regular(&self.root, name, false).map_err(fail)?;
        if pin(&file).map_err(fail)? != *identity || file.metadata().map_err(fail)?.len() != *extent
        {
            return Err(PackageError::Integrity);
        }
        let mut bytes = vec![0; length];
        file.seek(SeekFrom::Start(offset)).map_err(fail)?;
        file.read_exact(&mut bytes).map_err(fail)?;
        if pin(&regular(&self.root, name, false).map_err(fail)?).map_err(fail)? != *identity
            || file.metadata().map_err(fail)?.len() != *extent
        {
            return Err(PackageError::Integrity);
        }
        if let Some((authority, expected)) = &self.authority
            && authority().map_err(fail)? != *expected
        {
            return Err(PackageError::Integrity);
        }
        let named = File::from(
            openat(&self.parent, EVIDENCE, DIRECTORY, Mode::empty())
                .map_err(|_| PackageError::Integrity)?,
        );
        if pin(&self.root).map_err(fail)? != self.root_pin
            || pin(&named).map_err(fail)? != self.root_pin
        {
            return Err(PackageError::Integrity);
        }
        Ok(bytes)
    }
}
impl CheckpointRecords for Records {
    fn record(&self, reference: &Value, max_bytes: usize) -> Result<Vec<u8>, PackageError> {
        crate::package::v1_3::selected::json_ref(reference)?;
        let length = usize::try_from(crate::package::v1_3::tree::number(
            &reference["byte_length"],
        )?)
        .map_err(|_| PackageError::Limit)?;
        if length > max_bytes {
            return Err(PackageError::Limit);
        }
        let sha = reference["sha256"]
            .as_str()
            .ok_or(PackageError::Integrity)?;
        let name = format!("{sha}.checkpoint-record");
        if self.names.get(&name).map(|(_, n)| *n) != Some(length as u64) {
            return Err(PackageError::Integrity);
        }
        let bytes = self.range(&name, 0, length)?;
        if hash(&bytes) != sha {
            return Err(PackageError::Integrity);
        }
        Ok(bytes)
    }
    fn frame(
        &self,
        stream: PackageUuid,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, PackageError> {
        let name = if stream == self.initial_stream {
            CHECKPOINT.into()
        } else {
            segment_name(stream)
        };
        self.range(&name, offset, length)
    }
}

use super::{
    Native, NativeLease, OriginalPackage, VerifiedOriginal, canonical, frames, identity, pack_name,
    snapshot, support, v1_3,
};
use crate::package::v1_3::repeatable::checkpoint::{
    ActivationLimits, CheckpointIdentity, CheckpointIo, CheckpointRecoveryIo, LocalSelection,
    PreparedHistory, ReopenState, TurnoverPlan,
};

fn refused(_: impl std::fmt::Debug) -> PackageError {
    PackageError::Integrity
}

pub(super) fn standing_charge(footprint: &Value) -> Result<u64, PackageError> {
    footprint["standing_observation"]["rounded_required_charge"]
        .as_u64()
        .ok_or(PackageError::Integrity)
}
fn preflight_charge(footprint: &Value, extra: u64, limit: u64) -> Result<(), PackageError> {
    if standing_charge(footprint)?
        .checked_add(extra)
        .ok_or(PackageError::Limit)?
        > limit
    {
        return Err(PackageError::Limit);
    }
    Ok(())
}
impl Native {
    pub(super) fn verify_active_custody(&self) -> IoResult<()> {
        if let Some(namespace) = &self.checkpoint_namespace {
            let selected = LocalSelection::parse(
                &namespace.selector()?,
                &self.checkpoint_identity().map_err(|_| invalid())?,
                support::limits().json,
            )
            .map_err(|_| invalid())?;
            let selected = selected.value();
            if !selected["pending"].is_null()
                || selected["selected"]["format"]
                    != if self.journal_header["format_version"] == 1 {
                        "PHPSJ001"
                    } else {
                        "PHPSJ002"
                    }
                || selected["selected"]["stream_id"] != self.journal_header["stream_id"]
                || selected["selected"]["header_sha256"]
                    != hash(
                        &super::journal::Journal::create(
                            &self.journal_header,
                            self.journal_bounds(),
                        )
                        .map_err(|_| invalid())?,
                    )
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    pub(super) fn load_selected_journal(&mut self) -> IoResult<bool> {
        let Some(namespace) = &self.checkpoint_namespace else {
            return Ok(false);
        };
        let selected = LocalSelection::parse(
            &namespace.selector()?,
            &self.checkpoint_identity().map_err(|_| invalid())?,
            support::limits().json,
        )
        .map_err(|_| invalid())?;
        let selected = selected.value();
        if !selected["pending"].is_null() {
            return Err(invalid());
        }
        if selected["selected"]["format"] == "PHPSJ001" {
            self.verify_active_custody()?;
            return Ok(false);
        }
        let stream = PackageUuid::parse(
            selected["selected"]["stream_id"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(|_| invalid())?;
        let name = segment_name(stream);
        let bytes = read(&self.evidence, &name)?;
        let length = u32::from_le_bytes(
            bytes
                .get(8..12)
                .ok_or_else(invalid)?
                .try_into()
                .map_err(|_| invalid())?,
        ) as usize;
        let header = super::package::parse_canonical_json(
            bytes
                .get(12..12usize.checked_add(length).ok_or_else(invalid)?)
                .ok_or_else(invalid)?,
            self.journal_bounds().json,
        )
        .map_err(|_| invalid())?;
        if header["format_version"] != 2 || header["base"] != selected["selected"]["base"] {
            return Err(invalid());
        }
        for key in [
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "device_id",
            "incarnation_id",
            "journal_id",
        ] {
            if header[key] != self.registered_journal_header[key] {
                return Err(invalid());
            }
        }
        super::journal::Journal::read(&bytes, &header, self.journal_bounds())
            .map_err(|_| invalid())?;
        self.active_journal_pin = pin(&regular(&self.evidence, &name, false)?)?;
        self.journal_header = header;
        self.journal_name = name;
        self.verify_active_custody()?;
        Ok(true)
    }
    pub(super) fn checkpoint_identity(&self) -> Result<CheckpointIdentity, PackageError> {
        let uuid = |key: &str| {
            PackageUuid::parse(
                self.registered_journal_header[key]
                    .as_str()
                    .ok_or(PackageError::Integrity)?,
            )
        };
        Ok(CheckpointIdentity {
            package: v1_3::SelectionIdentity {
                project: uuid("project_id")?,
                library: uuid("library_id")?,
                bootstrap_sha256: crate::package::Sha256Hex::parse(
                    self.registered_journal_header["bootstrap_sha256"]
                        .as_str()
                        .ok_or(PackageError::Integrity)?,
                )?,
            },
            device: uuid("device_id")?,
            incarnation: uuid("incarnation_id")?,
        })
    }
    fn custody_namespace(&self) -> Result<&Namespace, PackageError> {
        self.checkpoint_namespace
            .as_ref()
            .ok_or(PackageError::Integrity)
    }
    pub(super) fn checkpoint_selector(&self, lease: &NativeLease) -> IoResult<Vec<u8>> {
        self.checkpoint_guard(lease)?;
        self.verify_active_custody()?;
        self.custody_namespace().map_err(|_| invalid())?.selector()
    }
    fn checkpoint_guard(&self, lease: &NativeLease) -> IoResult<()> {
        self.lease_ok(lease).map_err(|_| invalid())?;
        self.custody_namespace()
            .map_err(|_| invalid())?
            .retained()?;
        if let Some(ceiling) = &self.ceiling {
            let actual = self.footprint(ceiling)?;
            if let Some(limit) = self.checkpoint_control_limit
                && standing_charge(&actual).map_err(|_| invalid())? > limit
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn checkpoint_preflight(&self, extra: u64) -> Result<(), PackageError> {
        let footprint = self
            .footprint(self.ceiling.as_ref().ok_or(PackageError::Integrity)?)
            .map_err(refused)?;
        preflight_charge(
            &footprint,
            extra,
            self.checkpoint_control_limit
                .ok_or(PackageError::Integrity)?,
        )
    }
    fn allow_turnover(&mut self, plan: &TurnoverPlan) -> Result<(), PackageError> {
        if plan.original()["scope"]
            != serde_json::json!({"profile":self.base.registration.profile,
            "incarnation":self.base.registration.incarnation.to_string(),
            "directory":{"device":self.root_pin[0].to_string(),"inode":self.root_pin[1].to_string()}})
            || plan.old.manifest != super::package(&self.base).manifest
            || self.session_project_limit.is_some_and(|n| {
                super::number(&plan.original()["attempt"]["project_limit"]).ok() != Some(n)
            })
        {
            return Err(PackageError::Integrity);
        }
        let mut ceiling = BTreeMap::new();
        for (id, bytes) in &plan.files {
            let id = PackageUuid::parse(id)?;
            let allocation = self
                .base
                .allocations
                .get(&id)
                .ok_or(PackageError::Integrity)?;
            // Never expand the initial prepaid corridor or its physical charge.
            if bytes.len() as u64 > allocation.charge {
                return Err(PackageError::Limit);
            }
            ceiling.insert(id, allocation.charge);
        }
        self.allowed.extend(plan.old.loose.clone());
        self.commits.insert(
            hash(&canonical(&plan.old.commit)),
            canonical(&plan.old.commit),
        );
        for stage in plan.stages() {
            self.allowed.extend(stage.loose.clone());
            self.commits
                .insert(hash(&canonical(&stage.commit)), canonical(&stage.commit));
        }
        self.ceiling = Some(ceiling);
        let limit = super::number(&plan.original()["attempt"]["aggregate_control_bytes"])
            .map_err(refused)?;
        if limit > self.base.registration.standing_control {
            return Err(PackageError::Limit);
        }
        self.checkpoint_control_limit = Some(limit);
        self.check_effect_footprint().map_err(refused)
    }
    fn turnover_proof(
        &self,
        plan: &TurnoverPlan,
        stage: usize,
    ) -> Result<VerifiedOriginal, PackageError> {
        let current = self.current().map_err(refused)?;
        let observed = snapshot(&self.base, &current);
        self.check_effect_footprint().map_err(refused)?;
        v1_3::reader::verify_checkpoint_original(
            current.clone(),
            identity(&current),
            &observed,
            &self.base.registration,
            v1_3::DirectoryObservation {
                device: self.root_pin[0],
                inode: self.root_pin[1],
            },
            frames(),
            &support::limits(),
            plan,
            stage,
        )
    }
    fn turnover_stage(
        &self,
        plan: &TurnoverPlan,
    ) -> Result<(OriginalPackage, Option<usize>), PackageError> {
        let current = self.current().map_err(refused)?;
        let stage = if current.head == plan.old.head {
            None
        } else {
            Some(
                plan.stages()
                    .iter()
                    .position(|s| s.head == current.head)
                    .ok_or(PackageError::Integrity)?,
            )
        };
        let (commit, loose) = stage.map_or((&plan.old.commit, &plan.old.loose), |n| {
            (&plan.stages()[n].commit, &plan.stages()[n].loose)
        });
        if current.commit != *commit
            || current.loose != *loose
            || current.allocations.len() != plan.files.len()
        {
            return Err(PackageError::Integrity);
        }
        for (id, target) in &plan.files {
            let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
            let old = plan
                .old
                .allocations
                .get(id)
                .ok_or(PackageError::Integrity)?;
            if actual.arena != old.arena
                || actual.witness != old.witness
                || !actual.bytes.starts_with(&old.bytes)
                || !target.starts_with(&actual.bytes)
                || (stage.is_none() && actual.bytes != old.bytes)
                || (stage.is_some_and(|n| n >= 1) && actual.bytes != *target)
            {
                return Err(PackageError::Integrity);
            }
        }
        Ok((current, stage))
    }
    fn turnover_barrier(
        &self,
        lease: &NativeLease,
        plan: &TurnoverPlan,
        stage: usize,
    ) -> Result<(), PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        for id in plan.files.keys() {
            barrier(&regular(&self.root, &pack_name(id), true).map_err(refused)?)
                .map_err(refused)?;
        }
        for sha in plan.stages()[stage].loose.keys() {
            barrier(&regular(&self.root, &format!("{sha}.control"), true).map_err(refused)?)
                .map_err(refused)?;
        }
        barrier(
            &regular(
                &self.root,
                &format!("{}.commit", hash(&canonical(&plan.stages()[stage].commit))),
                true,
            )
            .map_err(refused)?,
        )
        .map_err(refused)?;
        barrier(&regular(&self.root, "HEAD.json", true).map_err(refused)?).map_err(refused)?;
        barrier(&self.root).map_err(refused)?;
        self.checkpoint_guard(lease).map_err(refused)
    }
    fn turnover_cleanup(
        &self,
        lease: &NativeLease,
        plan: &TurnoverPlan,
        stage: usize,
    ) -> Result<(), PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        self.turnover_stage(plan)?;
        for sha in self.allowed.keys() {
            if !plan.stages()[stage].loose.contains_key(sha) {
                match super::unlinkat(
                    &self.root,
                    format!("{sha}.control"),
                    super::AtFlags::empty(),
                ) {
                    Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                    Err(e) => return Err(refused(e)),
                }
            }
        }
        self.prune_commits(&plan.stages()[stage].commit)
            .map_err(refused)?;
        barrier(&self.root).map_err(refused)?;
        self.checkpoint_guard(lease).map_err(refused)
    }
}

impl CheckpointIo for Native {
    type Lease = File;
    fn observe_selector(
        &mut self,
        lease: &NativeLease,
        original: &Value,
    ) -> Result<Vec<u8>, PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        let id = self.checkpoint_identity()?;
        if original["project_id"] != id.package.project.to_string()
            || original["library_id"] != id.package.library.to_string()
            || original["bootstrap_sha256"] != id.package.bootstrap_sha256.as_str()
        {
            return Err(PackageError::Integrity);
        }
        let bytes = self.custody_namespace()?.selector().map_err(refused)?;
        LocalSelection::parse(&bytes, &id, support::limits().json)?;
        barrier(
            &regular(&self.evidence, &self.custody_namespace()?.selector, true).map_err(refused)?,
        )
        .map_err(refused)?;
        barrier(&self.evidence).map_err(refused)?;
        self.checkpoint_guard(lease).map_err(refused)?;
        if self.custody_namespace()?.selector().map_err(refused)? != bytes {
            return Err(PackageError::Integrity);
        }
        Ok(bytes)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One closed two-domain coexistence preflight before retaining the original"
    )]
    fn retain_original(
        &mut self,
        lease: &NativeLease,
        plan: &TurnoverPlan,
        history: &PreparedHistory<'_>,
    ) -> Result<(), PackageError> {
        self.allow_turnover(plan)?;
        self.checkpoint_guard(lease).map_err(refused)?;
        let namespace = self.custody_namespace()?;
        let old_ref = &plan.original()["old"]["selector"];
        let observed = namespace.selector().map_err(refused)?;
        let old = if hash(&observed) == old_ref["sha256"]
            && observed.len() as u64 == super::number(&old_ref["byte_length"]).map_err(refused)?
        {
            observed
        } else {
            namespace
                .records()
                .map_err(refused)?
                .record(old_ref, support::limits().json.max_bytes)?
        };
        let id = self.checkpoint_identity()?;
        let reader = support::limits();
        let old_selection = LocalSelection::parse(&old, &id, reader.json)?;
        // Freeze only the exact drained source segment proved by PreparedHistory.
        // A later append cannot be skipped merely because the selector is equal.
        for segment in history.archive()["segments"]
            .as_array()
            .ok_or(PackageError::Integrity)?
        {
            let stream = PackageUuid::parse(
                segment["stream_id"]
                    .as_str()
                    .ok_or(PackageError::Integrity)?,
            )?;
            let name = if stream == namespace.initial_stream {
                CHECKPOINT.into()
            } else {
                segment_name(stream)
            };
            let bytes = read(&self.evidence, &name).map_err(refused)?;
            if bytes.len() as u64 != super::number(&segment["byte_length"]).map_err(refused)?
                || hash(&bytes) != segment["sha256"]
            {
                return Err(PackageError::Integrity);
            }
            barrier(&regular(&self.evidence, &name, true).map_err(refused)?).map_err(refused)?;
        }
        barrier(&self.evidence).map_err(refused)?;
        let mut candidate = plan.old.clone();
        candidate.head = plan.stages()[3].head.clone();
        candidate.commit = plan.stages()[3].commit.clone();
        candidate.loose = plan.stages()[3].loose.clone();
        for (id, bytes) in &plan.files {
            candidate
                .allocations
                .get_mut(id)
                .ok_or(PackageError::Integrity)?
                .bytes = bytes.clone();
        }
        let observed = snapshot(&self.base, &candidate);
        // Prospective byte verification only. activate independently reads and
        // verifies the actual clean package after all native effect barriers.
        let prospective = v1_3::reader::verify_checkpoint_original(
            candidate.clone(),
            identity(&candidate),
            &observed,
            &self.base.registration,
            v1_3::DirectoryObservation {
                device: self.root_pin[0],
                inode: self.root_pin[1],
            },
            frames(),
            &reader,
            plan,
            3,
        )?;
        let installation = crate::package::v1_3::repeatable::checkpoint::prepare_installation(
            plan,
            history,
            &old_selection,
            &prospective,
            &id,
            ActivationLimits {
                journal: self.journal_bounds(),
                frames: frames(),
                reader: &reader,
                planner: super::budget(),
            },
        )?;
        let mut records = BTreeMap::from([(record_name(&old), old)]);
        for value in [
            plan.original(),
            plan.acceptance_base(),
            history.archive(),
            history.lookup(),
        ]
        .into_iter()
        .chain(history.lookup_objects().values())
        {
            let bytes = canonical(value);
            records.insert(record_name(&bytes), bytes);
        }
        let unit = self.base.registration.charge_unit;
        let mut extra = 0u64;
        let base = canonical(installation.base.value());
        extra = extra
            .checked_add(
                additional(&self.evidence, &record_name(&base), &base, unit).map_err(refused)?,
            )
            .ok_or(PackageError::Limit)?;
        let segment_id = PackageUuid::parse(
            installation.header["stream_id"]
                .as_str()
                .ok_or(PackageError::Integrity)?,
        )?;
        extra = extra
            .checked_add(
                additional(
                    &self.evidence,
                    &segment_name(segment_id),
                    &installation.segment,
                    unit,
                )
                .map_err(refused)?,
            )
            .ok_or(PackageError::Limit)?;
        // Reserve both selected and candidate namespace inodes at the larger
        // final-selector length. Pending has the same exact bounded field set.
        let final_selection = canonical(installation.selector.value());
        let pending_selection = canonical(installation.pending.value());
        let selection_len = final_selection.len().max(pending_selection.len());
        let selector_charge = (selection_len as u64)
            .checked_add(unit - 1)
            .and_then(|n| (n / unit).checked_add(1))
            .and_then(|n| n.checked_mul(unit))
            .ok_or(PackageError::Limit)?;
        extra = extra
            .checked_add(selector_charge.checked_mul(2).ok_or(PackageError::Limit)?)
            .ok_or(PackageError::Limit)?;
        extra = extra
            .checked_add(unit.checked_mul(2).ok_or(PackageError::Limit)?)
            .ok_or(PackageError::Limit)?; // directory/name coexistence slack
        for (name, bytes) in &records {
            extra = extra
                .checked_add(additional(&self.evidence, name, bytes, unit).map_err(refused)?)
                .ok_or(PackageError::Limit)?;
        }
        // All finite original-owned controls may coexist after an interrupted
        // publication. Account them before retaining the original or selecting P.
        for (sha, bytes) in &self.allowed {
            extra = extra
                .checked_add(
                    additional(&self.root, &format!("{sha}.control"), bytes, unit)
                        .map_err(refused)?,
                )
                .ok_or(PackageError::Limit)?;
        }
        for (sha, bytes) in &self.commits {
            extra = extra
                .checked_add(
                    additional(&self.root, &format!("{sha}.commit"), bytes, unit)
                        .map_err(refused)?,
                )
                .ok_or(PackageError::Limit)?;
        }
        let count = self
            .allowed
            .len()
            .checked_add(self.commits.len())
            .and_then(|n| n.checked_add(records.len()))
            .and_then(|n| n.checked_add(namespace.retained().ok()?.len()))
            .and_then(|n| n.checked_add(4))
            .ok_or(PackageError::Limit)?;
        if count as u64
            > super::number(&plan.original()["attempt"]["aggregate_control_count"])
                .map_err(refused)?
        {
            return Err(PackageError::Limit);
        }
        self.checkpoint_preflight(extra)?;
        // Each exact retained object is an independent immutable original input.
        // Nothing in this installation chooses an active base or grants Saved.
        for bytes in records.values() {
            namespace
                .retain(bytes, |_, _| self.checkpoint_guard(lease))
                .map_err(refused)?;
        }
        Ok(())
    }
    fn select(&mut self, lease: &NativeLease, old: &[u8], new: &[u8]) -> Result<(), PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        let identity = self.checkpoint_identity()?;
        let before = LocalSelection::parse(old, &identity, support::limits().json)?;
        let after = LocalSelection::parse(new, &identity, support::limits().json)?;
        before.successor(&after, &hash(old))?;
        if self.custody_namespace()?.selector().map_err(refused)? != new {
            self.checkpoint_preflight(
                additional(
                    &self.evidence,
                    &format!("{}-next", self.custody_namespace()?.selector),
                    new,
                    self.base.registration.charge_unit,
                )
                .map_err(refused)?,
            )?;
        }
        self.custody_namespace()?
            .select(old, new, |_, _| self.checkpoint_guard(lease))
            .map_err(refused)
    }
    fn finish_package(
        &mut self,
        lease: &NativeLease,
        plan: &TurnoverPlan,
    ) -> Result<VerifiedOriginal, PackageError> {
        self.allow_turnover(plan)?;
        self.checkpoint_guard(lease).map_err(refused)?;
        let (_, mut stage) = self.turnover_stage(plan)?;
        if let Some(n) = stage {
            self.turnover_barrier(lease, plan, n)?;
            self.turnover_cleanup(lease, plan, n)?;
        }
        while stage != Some(3) {
            let next = stage.map_or(0, |n| n + 1);
            if next == 1 {
                let (current, _) = self.turnover_stage(plan)?;
                for (id, target) in &plan.files {
                    self.checkpoint_guard(lease).map_err(refused)?;
                    let actual = &current.allocations[id].bytes;
                    super::append_exact(
                        &self.root,
                        &pack_name(id),
                        actual,
                        &target[actual.len()..],
                    )
                    .map_err(refused)?;
                    self.checkpoint_guard(lease).map_err(refused)?;
                }
            }
            let expected = stage.map_or(&plan.old.head, |n| &plan.stages()[n].head);
            super::selection_group::publish(&self.root, expected, &plan.stages()[next], |_, _| {
                self.checkpoint_guard(lease)
            })
            .map_err(refused)?;
            // Same original-owned sidecar cleanup as the existing executor.
            self.turnover_cleanup(lease, plan, next)?;
            stage = Some(next);
            self.turnover_stage(plan)?;
            if next >= 2 {
                self.turnover_proof(plan, next)?;
            }
        }
        self.turnover_proof(plan, 3)
    }
    fn verify_clean(
        &mut self,
        lease: &NativeLease,
        plan: &TurnoverPlan,
    ) -> Result<VerifiedOriginal, PackageError> {
        self.allow_turnover(plan)?;
        self.checkpoint_guard(lease).map_err(refused)?;
        self.turnover_proof(plan, 3)
    }
    fn install_segment(
        &mut self,
        lease: &NativeLease,
        base: &[u8],
        header: &Value,
        segment: &[u8],
    ) -> Result<(), PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        let base_value = super::package::parse_canonical_json(base, support::limits().json)?;
        if header["base"] != super::reference(&base_value) {
            return Err(PackageError::Integrity);
        }
        super::journal::Journal::read(segment, header, self.journal_bounds())?;
        let namespace = self.custody_namespace()?;
        let stream = PackageUuid::parse(
            header["stream_id"]
                .as_str()
                .ok_or(PackageError::Integrity)?,
        )?;
        let unit = self.base.registration.charge_unit;
        self.checkpoint_preflight(
            additional(&self.evidence, &record_name(base), base, unit)
                .map_err(refused)?
                .checked_add(
                    additional(&self.evidence, &segment_name(stream), segment, unit)
                        .map_err(refused)?,
                )
                .ok_or(PackageError::Limit)?,
        )?;
        namespace
            .retain(base, |_, _| self.checkpoint_guard(lease))
            .map_err(refused)?;
        namespace
            .install_segment(stream, segment, |_, _| self.checkpoint_guard(lease))
            .map_err(refused)
    }
}

impl Native {
    pub(super) fn read_checkpoint_state(
        &self,
        lease: &NativeLease,
        limits: ActivationLimits<'_>,
    ) -> Result<ReopenState, PackageError> {
        self.read_checkpoint_state_at(lease, limits, None)
    }
    pub(super) fn read_checkpoint_state_at(
        &self,
        lease: &NativeLease,
        limits: ActivationLimits<'_>,
        original: Option<VerifiedOriginal>,
    ) -> Result<ReopenState, PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        let namespace = self.custody_namespace()?;
        let selector = namespace.selector().map_err(refused)?;
        let selected =
            LocalSelection::parse(&selector, &self.checkpoint_identity()?, limits.reader.json)?;
        if !selected.value()["pending"].is_null()
            || selected.value()["selected"]["format"] != "PHPSJ002"
        {
            return Err(PackageError::Integrity);
        }
        let mut records = namespace.records().map_err(refused)?;
        records.authority = self
            .local_authority
            .clone()
            .zip(self.local_provenance.clone());
        let records = std::sync::Arc::new(records);
        let base = records.record(
            &selected.value()["selected"]["base"],
            limits.reader.json.max_bytes,
        )?;
        let stream = PackageUuid::parse(
            selected.value()["selected"]["stream_id"]
                .as_str()
                .ok_or(PackageError::Integrity)?,
        )?;
        let segment = read(&self.evidence, &segment_name(stream)).map_err(refused)?;
        let length = u32::from_le_bytes(
            segment
                .get(8..12)
                .ok_or(PackageError::Integrity)?
                .try_into()
                .map_err(refused)?,
        ) as usize;
        let header = super::package::parse_canonical_json(
            segment
                .get(12..12usize.checked_add(length).ok_or(PackageError::Limit)?)
                .ok_or(PackageError::Integrity)?,
            limits.journal.json,
        )?;
        super::journal::Journal::read(&segment, &header, limits.journal)?;
        let current = if let Some(original) = original {
            original
        } else {
            let current = self.current().map_err(refused)?;
            let observed = snapshot(&self.base, &current);
            v1_3::reader::verify_shared_original(
                current.clone(),
                identity(&current),
                &observed,
                &self.base.registration,
                v1_3::DirectoryObservation {
                    device: self.root_pin[0],
                    inode: self.root_pin[1],
                },
                limits.frames,
                limits.reader,
            )?
        };
        self.checkpoint_guard(lease).map_err(refused)?;
        Ok(ReopenState {
            selector,
            base,
            header,
            segment,
            current,
            records,
        })
    }
}

impl CheckpointRecoveryIo for Native {
    type Lease = File;
    fn read_checkpoint(
        &mut self,
        lease: &NativeLease,
        limits: ActivationLimits<'_>,
    ) -> Result<ReopenState, PackageError> {
        self.read_checkpoint_state(lease, limits)
    }
}

#[cfg(test)]
#[path = "native_checkpoint_tests.rs"]
mod tests;
