//! Exact local selector custody after original-bound maintenance. No native
//! implementation is exported; successful protocol completion is not Saved.
use super::{CheckpointIdentity, LocalBase, LocalSelection, PreparedHistory, TurnoverPlan, check};
use crate::package::PackageError;
use crate::package::planning::io::RegisteredCooperativeLease;
use crate::package::v1_3::repeatable::{
    Limits, VerifiedOriginal,
    journal::{Journal, JournalLimits, JournalRecordIdentity},
    wire,
};
use crate::package::v1_3::{FrameLimits, ReaderLimits, snapshot::AcceptanceBase, tree};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub struct ActivationLimits<'a> {
    pub journal: JournalLimits,
    pub frames: FrameLimits,
    pub reader: &'a ReaderLimits,
    pub planner: Limits,
}

/// Trusted registered adapter boundary. Every method retains the lease and
/// rechecks safe namespace/handle pins and current authority. Errors, including
/// uncertainty after a completed write, stop activation without rollback.
pub trait CheckpointIo {
    type Lease;
    /// Reauthorize the exact original and standing liability; observe/rebarrier
    /// the registered selector. Never substitute a well-formed third value.
    /// # Errors
    /// Refuses stale authority, unsafe namespace, pins or unknown selector bytes.
    fn observe_selector(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        original: &Value,
    ) -> Result<Vec<u8>, PackageError>;
    /// Persist exact original, acceptance-base and history lookup/archive bytes
    /// under registered immutable names, including barriers on exact retries.
    /// This precedes the pending selector and all package effects.
    /// # Errors
    /// Refuses changed existing bytes, insufficient original space or uncertainty.
    fn retain_original(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &TurnoverPlan,
        history: &PreparedHistory<'_>,
    ) -> Result<(), PackageError>;
    /// Exact-old-or-exact-new CAS, file/directory/full barriers, and fresh read.
    /// Even an already-selected exact candidate must be rebarriered on retry.
    /// # Errors
    /// Refuses any third value or failed publication/barrier.
    fn select(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        old: &[u8],
        new: &[u8],
    ) -> Result<(), PackageError>;
    /// Execute/reconcile only the four original-bound package stages. Preserve
    /// payload, selection, cleanup barriers and exact expected-HEAD checks;
    /// never regress a later stage. Reissue barriers after uncertain completion.
    /// Return only the independently reverified clean package, never published.
    /// # Errors
    /// Refuses mismatched bytes, incomplete stages, authority or barrier failure.
    fn finish_package(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &TurnoverPlan,
    ) -> Result<VerifiedOriginal, PackageError>;
    /// Read-only fresh clean-package verification for an already-selected retry.
    /// No package or local effects are permitted before exact final CAS matching.
    /// # Errors
    /// Refuses any incomplete or changed original-bound clean package.
    fn verify_clean(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &TurnoverPlan,
    ) -> Result<VerifiedOriginal, PackageError>;
    /// Durably install exact local base and complete new segment (header plus
    /// `BaseReceipt`); all immutable-name retries compare bytes and rebarrier.
    /// No selector is changed here. Old streams and lookup objects are retained.
    /// # Errors
    /// Refuses changed bytes/pins, incomplete persistence or failed barriers.
    fn install_segment(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        base: &[u8],
        header: &Value,
        segment: &[u8],
    ) -> Result<(), PackageError>;
}

/// Independently bound artifacts plus completed registered-selector protocol.
/// Not a graph-operation acceptance or production Saved acknowledgement.
pub struct ActivatedCheckpoint {
    base: LocalBase,
    selector: LocalSelection,
    header: Value,
    segment: Vec<u8>,
    pub(super) clean: VerifiedOriginal,
    pub(super) seed: crate::package::v1_3::repeatable::journal::session::CheckpointSeed,
}
impl ActivatedCheckpoint {
    /// Replay a local suffix from this independently verified live turnover.
    /// Durable reopen requires separately recovered selector/base custody; plain
    /// parsed records cannot create this capability.
    /// # Errors
    /// Refuses substituted base/segment bytes, tighter bounds or invalid suffix.
    pub fn verify_session<'a>(
        &self,
        journal: &'a Journal,
        frames: FrameLimits,
        reader: &ReaderLimits,
        limits: Limits,
    ) -> Result<crate::package::v1_3::repeatable::journal::VerifiedSession<'a>, PackageError> {
        self.seed.session(journal, self, frames, reader, limits)
    }

    #[must_use]
    pub fn base(&self) -> &LocalBase {
        &self.base
    }
    #[must_use]
    pub fn selector(&self) -> &LocalSelection {
        &self.selector
    }
    #[must_use]
    pub fn header(&self) -> &Value {
        &self.header
    }
    #[must_use]
    pub fn segment(&self) -> &[u8] {
        &self.segment
    }
    #[must_use]
    pub fn package(&self) -> &VerifiedOriginal {
        &self.clean
    }
}

fn successor(
    old: &LocalSelection,
    pending: Value,
    selected: Value,
    identity: &CheckpointIdentity,
    limits: &ReaderLimits,
) -> Result<LocalSelection, PackageError> {
    let mut value = old.value().clone();
    value["generation"] = json!(
        tree::number(&value["generation"])?
            .checked_add(1)
            .ok_or(PackageError::Limit)?
            .to_string()
    );
    let hash = wire::hash(&wire::encode(old.value()));
    value["previous_sha256"] = json!(hash);
    value["pending"] = pending;
    value["selected"] = selected;
    let next = LocalSelection::parse(&wire::encode(&value), identity, limits.json)?;
    old.successor(&next, &hash)?;
    Ok(next)
}

/// Original-first maintenance followed by exact local segment activation.
/// Caller-supplied IDs are immutable retry inputs, never generated here.
/// # Errors
/// Refuses changed original/old selector/base, unrelated selector outcomes,
/// incomplete clean proof, changed identities/bounds, or any failed IO barrier.
pub fn activate<I: CheckpointIo>(
    io: &mut I,
    lease: &RegisteredCooperativeLease<I::Lease>,
    plan: &TurnoverPlan,
    history: &PreparedHistory<'_>,
    old: &LocalSelection,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<ActivatedCheckpoint, PackageError> {
    check(plan.original()["old"]["selector"] == wire::reference(old.value()))?;
    let pending = successor(
        old,
        wire::reference(plan.original()),
        old.value()["selected"].clone(),
        identity,
        limits.reader,
    )?;
    let observed = io.observe_selector(lease, plan.original())?;
    // Validate shape now, but never treat arbitrary successors as our candidate.
    LocalSelection::parse(&observed, identity, limits.reader.json)?;
    let old_bytes = wire::encode(old.value());
    let pending_bytes = wire::encode(pending.value());
    let receipt = JournalRecordIdentity {
        record_id: crate::package::v1_3::selected::nonnil(&plan.original()["maintenance_id"])?,
        session_generation: tree::number(&pending.value()["generation"])?,
    };
    if observed == old_bytes || observed == pending_bytes {
        io.retain_original(lease, plan, history)?;
        io.select(lease, &old_bytes, &pending_bytes)?;
    }
    let active_retry = observed != old_bytes && observed != pending_bytes;
    let clean = if active_retry {
        io.verify_clean(lease, plan)?
    } else {
        io.finish_package(lease, plan)?
    };
    check(
        clean.package.head == plan.stages()[3].head
            && clean.package.commit == plan.stages()[3].commit,
    )?;
    let base = AcceptanceBase::parse(
        &wire::encode(plan.acceptance_base()),
        &identity.package,
        limits.reader.json,
    )?;
    let base = history.bind_package(
        &base,
        &clean,
        identity,
        limits.frames,
        limits.reader,
        limits.planner,
    )?;
    let previous = &history.archive()["segments"][0];
    let mut header = json!({"format_version":2,"stream_id":plan.original()["journal"]["next_stream_id"],
        "journal_id":base.value()["journal_id"],"base_head":base.value()["package_head"],
        "base_package_revision":clean.package.commit["package_revision"],"base":wire::reference(base.value()),
        "sequence_base":base.value()["through"]["sequence"],"previous_segment":{
            "stream_id":previous["stream_id"],"header_sha256":previous["header_sha256"],"last":base.value()["through"]}});
    for key in [
        "project_id",
        "library_id",
        "bootstrap_sha256",
        "device_id",
        "incarnation_id",
    ] {
        header[key] = base.value()[key].clone();
    }
    base.check_header(&header)?;
    let mut segment = Journal::create(&header, limits.journal)?;
    let header_hash = wire::hash(&segment);
    let journal = Journal::read(&segment, &header, limits.journal)?;
    let body = json!({"maintenance_id":plan.original()["maintenance_id"],"original":wire::reference(plan.original()),
        "journal_base":wire::reference(base.value()),"selected_head":clean.package.head,
        "selected_commit":wire::reference(&clean.package.commit)});
    segment.extend_from_slice(
        journal
            .base_receipt(&body, receipt, limits.journal)?
            .bytes(),
    );
    let selected = json!({"format":"PHPSJ002","stream_id":header["stream_id"],
        "header_sha256":header_hash,"base":header["base"]});
    let selector = successor(&pending, Value::Null, selected, identity, limits.reader)?;
    let selected_bytes = wire::encode(selector.value());
    check(observed == old_bytes || observed == pending_bytes || observed == selected_bytes)?;
    if active_retry {
        let stabilized = io.finish_package(lease, plan)?;
        check(
            stabilized.package.head == clean.package.head
                && stabilized.package.commit == clean.package.commit,
        )?;
    }
    io.install_segment(lease, &wire::encode(base.value()), &header, &segment)?;
    io.select(lease, &pending_bytes, &selected_bytes)?;
    Ok(ActivatedCheckpoint {
        base,
        selector,
        header,
        segment,
        clean,
        seed: history.seed.clone(),
    })
}
