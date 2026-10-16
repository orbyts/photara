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
    let Installation {
        base,
        selector,
        header,
        segment,
        pending: prepared_pending,
    } = prepare_installation(plan, history, old, &clean, identity, limits)?;
    check(prepared_pending.value() == pending.value())?;
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

/// Exact deterministic local outputs, shared by native coexistence preflight
/// and post-barrier activation. Preparing bytes is never a custody/Saved grant.
pub(crate) struct Installation {
    pub(crate) pending: LocalSelection,
    pub(crate) base: LocalBase,
    pub(crate) selector: LocalSelection,
    pub(crate) header: Value,
    pub(crate) segment: Vec<u8>,
}
pub(crate) fn prepare_installation(
    plan: &TurnoverPlan,
    history: &PreparedHistory<'_>,
    old: &LocalSelection,
    clean: &VerifiedOriginal,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<Installation, PackageError> {
    let pending = successor(
        old,
        wire::reference(plan.original()),
        old.value()["selected"].clone(),
        identity,
        limits.reader,
    )?;
    let receipt = JournalRecordIdentity {
        record_id: crate::package::v1_3::selected::nonnil(&plan.original()["maintenance_id"])?,
        session_generation: tree::number(&pending.value()["generation"])?,
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
        clean,
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
    Ok(Installation {
        pending,
        base,
        selector,
        header,
        segment,
    })
}

/// Fresh registered observations for cold recovery. These are untrusted inputs
/// until `reopen` authenticates the selected base and included journal evidence.
pub struct ReopenState {
    pub selector: Vec<u8>,
    pub base: Vec<u8>,
    pub header: Value,
    pub segment: Vec<u8>,
    pub current: VerifiedOriginal,
    pub records: std::sync::Arc<dyn super::CheckpointRecords>,
}
/// Registered cold-open boundary. The implementation checks the held lease,
/// current authority, exact namespace/handle pins and fresh independent package
/// observation. It never chooses an arbitrary well-formed replacement selector.
pub trait CheckpointRecoveryIo {
    type Lease;
    /// Read the actually selected immutable base/segment and current package.
    /// # Errors
    /// Refuses missing registration, changed pins, pending/unknown custody,
    /// missing files, or failed independent current package verification.
    fn read_checkpoint(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        limits: ActivationLimits<'_>,
    ) -> Result<ReopenState, PackageError>;
}
fn retained(
    source: &dyn super::CheckpointRecords,
    reference: &Value,
    limits: &ReaderLimits,
) -> Result<Value, PackageError> {
    crate::package::v1_3::selected::json_ref(reference)?;
    let length = usize::try_from(tree::number(&reference["byte_length"])?)
        .map_err(|_| PackageError::Limit)?;
    if length > limits.json.max_bytes {
        return Err(PackageError::Limit);
    }
    let bytes = source.record(reference, length)?;
    check(bytes.len() == length && wire::hash(&bytes) == reference["sha256"])?;
    crate::package::parse_canonical_json(&bytes, limits.json)
}
/// Recover only from registered selected custody plus independently verified
/// current inclusion. No old authored transition is replayed to invent a base.
/// Pending maintenance remains refused here and must use its original retry.
/// # Errors
/// Refuses substituted selectors/base/receipt, current-base mismatch, missing
/// included checkpoint or historical lookup evidence, or any changed bounds.
pub fn reopen<I: CheckpointRecoveryIo>(
    io: &mut I,
    lease: &RegisteredCooperativeLease<I::Lease>,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<ActivatedCheckpoint, PackageError> {
    let state = io.read_checkpoint(lease, limits)?;
    reopen_state(state, identity, limits)
}

pub(crate) fn reopen_state(
    state: ReopenState,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<ActivatedCheckpoint, PackageError> {
    let (selector, base, journal) = custody(&state, identity, limits)?;
    let seed = recover_seed(&state, &base, &journal, identity, limits)?;
    let segment = journal.base_prefix()?.to_vec();
    Ok(ActivatedCheckpoint {
        base,
        selector,
        header: state.header,
        segment,
        clean: state.current,
        seed,
    })
}

fn custody(
    state: &ReopenState,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<(LocalSelection, LocalBase, Journal), PackageError> {
    let (selector, base, journal, _) = custody_records(
        &state.selector,
        &state.base,
        &state.header,
        &state.segment,
        state.records.as_ref(),
        &state.current.scope,
        identity,
        limits,
    )?;
    crate::package::v1_3::reader::snapshot_base_binding(
        &state.current,
        &crate::package::v1_3::selected::json_ref(&base.value()["checkpoint"])?,
        limits.frames,
        limits.reader,
    )?;
    Ok((selector, base, journal))
}
#[expect(
    clippy::too_many_arguments,
    reason = "Exact registered local custody inputs precede package proof"
)]
pub(super) fn custody_records(
    selector_bytes: &[u8],
    base_bytes: &[u8],
    header: &Value,
    segment: &[u8],
    records: &dyn super::CheckpointRecords,
    scope: &Value,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<(LocalSelection, LocalBase, Journal, Value), PackageError> {
    let selector = LocalSelection::parse(selector_bytes, identity, limits.reader.json)?;
    check(
        selector.value()["pending"].is_null()
            && selector.value()["selected"]["format"] == "PHPSJ002",
    )?;
    let base = LocalBase::parse(base_bytes, identity, limits.reader.json)?;
    base.check_header(header)?;
    let header_bytes = Journal::create(header, limits.journal)?;
    check(
        selector.value()["selected"]
            == json!({"format":"PHPSJ002","stream_id":header["stream_id"],"header_sha256":wire::hash(&header_bytes),"base":wire::reference(base.value())}),
    )?;
    let journal = Journal::read(segment, header, limits.journal)?;
    let first = journal
        .records()
        .first()
        .ok_or(PackageError::Integrity)?
        .value();
    check(
        first["kind"] == "CheckpointBaseReceipt"
            && first["body"]["journal_base"] == wire::reference(base.value())
            && first["body"]["selected_head"] == base.value()["package_head"]
            && first["body"]["selected_commit"] == base.value()["package_commit"],
    )?;
    let original = retained(records, &first["body"]["original"], limits.reader)?;
    tree::fields(
        &original,
        &[
            "schema",
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "extensions",
            "maintenance_id",
            "scope",
            "old",
            "target",
            "journal",
            "attempt",
            "writes",
        ],
    )?;
    check(
        original["schema"]
            == json!({"id":"photara.storage.checkpoint-turnover-original","version":1})
            && original["extensions"] == json!({})
            && original["scope"] == *scope
            && original["maintenance_id"] == first["record_id"]
            && first["record_id"] == first["body"]["maintenance_id"]
            && original["target"]["base"] == base.value()["checkpoint"]
            && original["journal"]["next_stream_id"] == header["stream_id"]
            && original["journal"]["through"] == base.value()["through"]
            && original["journal"]["lookup"] == base.value()["lookup"]
            && original["journal"]["archive"] == base.value()["archive"],
    )?;
    for key in ["project_id", "library_id", "bootstrap_sha256"] {
        check(original[key] == base.value()[key])?;
    }
    let old = retained(records, &original["old"]["selector"], limits.reader)?;
    let old = LocalSelection::parse(&wire::encode(&old), identity, limits.reader.json)?;
    let pending = successor(
        &old,
        wire::reference(&original),
        old.value()["selected"].clone(),
        identity,
        limits.reader,
    )?;
    let expected = successor(
        &pending,
        Value::Null,
        selector.value()["selected"].clone(),
        identity,
        limits.reader,
    )?;
    check(
        expected.value() == selector.value()
            && first["session_generation"] == pending.value()["generation"],
    )?;
    Ok((selector, base, journal, original))
}

fn recover_seed(
    state: &ReopenState,
    base: &LocalBase,
    journal: &Journal,
    identity: &CheckpointIdentity,
    limits: ActivationLimits<'_>,
) -> Result<crate::package::v1_3::repeatable::journal::session::CheckpointSeed, PackageError> {
    let history = std::sync::Arc::new(super::lookup::History {
        base: base.clone(),
        source: state.records.clone(),
        identity: identity.clone(),
        limits: *limits.reader,
    });
    crate::package::v1_3::repeatable::journal::session::CheckpointSeed::recover(
        base,
        history,
        &state.current,
        journal,
        limits.frames,
        limits.reader,
        limits.planner,
    )
}
/// Prepare a missing local receipt when publication completed before the crash.
/// The registered adapter must rebarrier the exact current package before this
/// call and append/rebarrier the returned bytes before ordinary reopen. Neither
/// this result nor matching package JSON is a Saved acknowledgment.
/// # Errors
/// Refuses absent exact clean-plan reader evidence, substituted custody/base,
/// changed original/Mutation, incomplete suffix or any conflicting receipt.
pub fn prepare_recovery_receipt<I: CheckpointRecoveryIo>(
    io: &mut I,
    lease: &RegisteredCooperativeLease<I::Lease>,
    identity: &CheckpointIdentity,
    receipt: JournalRecordIdentity,
    limits: ActivationLimits<'_>,
) -> Result<crate::package::v1_3::repeatable::journal::JournalAppend, PackageError> {
    let state = io.read_checkpoint(lease, limits)?;
    let (_, base, journal) = custody(&state, identity, limits)?;
    let append = journal.reconcile_snapshot_receipt(&state.current, receipt, limits.journal)?;
    let mut bytes = state.segment.clone();
    bytes.extend_from_slice(append.bytes());
    let repaired = Journal::read(&bytes, &state.header, limits.journal)?;
    recover_seed(&state, &base, &repaired, identity, limits)?;
    Ok(append)
}
