//! Serialized one-project disposable session. The private adapter is the native
//! authority boundary; journal metadata and these DTOs never grant real-path IO.
#![cfg_attr(
    not(any(test, feature = "controlled-disposable")),
    allow(
        dead_code,
        reason = "Only explicitly enabled controlled-disposable adapters construct sessions; real-library admission remains unavailable"
    )
)]
use super::{
    EffectFailure, Limits, PlannerInputs, VerifiedOriginal,
    journal::{
        Journal, JournalAppend, JournalLimits, JournalRecordIdentity, VerifiedJournal,
        VerifiedSession,
    },
};
use crate::package::{PackageError, PackageUuid};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionBinding {
    pub project_id: PackageUuid,
    pub incarnation_id: PackageUuid,
    pub owner_epoch: PackageUuid,
    pub owner: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AcceptedCoordinate {
    pub coordinate: Value,
    pub mutation: Option<Value>,
    pub accepted_frame: Option<Value>,
}
impl AcceptedCoordinate {
    fn current(view: &VerifiedSession<'_>) -> Self {
        Self {
            coordinate: view.coordinate(),
            mutation: view.through(),
            accepted_frame: view.accepted_frame(),
        }
    }
    fn checkpoint(view: &VerifiedSession<'_>) -> Self {
        let value = view.checkpoint_target();
        Self {
            coordinate: value["coordinate"].clone(),
            mutation: (!value["through"].is_null()).then(|| value["through"].clone()),
            accepted_frame: (!value["accepted_frame"].is_null())
                .then(|| value["accepted_frame"].clone()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedEvidence {
    pub target: AcceptedCoordinate,
    pub head: Value,
    pub commit_sha256: String,
    pub checkpoint_receipt: Value,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionFailure {
    Authorization,
    Storage,
    Journal,
    Checkpoint,
    Conflict,
    Capacity,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionSnapshot {
    pub binding: SessionBinding,
    pub event_sequence: u64,
    pub accepted: AcceptedCoordinate,
    pub saved: Option<SavedEvidence>,
    pub frozen: bool,
    pub closed: bool,
    pub failure: Option<SessionFailure>,
    pub dirty: bool,
}
#[derive(Debug)]
pub enum SessionError {
    Frozen,
    Closed,
    Busy,
    InvalidRequest,
    Failure(SessionFailure),
}
impl From<PackageError> for SessionError {
    fn from(_: PackageError) -> Self {
        Self::Failure(SessionFailure::Journal)
    }
}
/// Returned only after the exact Mutation bytes and native barriers succeed.
#[derive(Clone, Debug, Serialize)]
pub struct Accepted {
    receipt: Value,
    mutation: Value,
    result: Value,
    frame: Value,
}
impl Accepted {
    #[must_use]
    pub fn receipt(&self) -> &Value {
        &self.receipt
    }
    #[must_use]
    pub fn mutation(&self) -> &Value {
        &self.mutation
    }
    #[must_use]
    pub fn result(&self) -> &Value {
        &self.result
    }
    #[must_use]
    pub fn accepted_frame(&self) -> &Value {
        &self.frame
    }
}
/// Finite barrier identity, not a Saved acknowledgement.
#[derive(Clone, Debug)]
pub struct FlushTarget {
    link: Option<Value>,
    target: AcceptedCoordinate,
    closing: bool,
    baseline: Option<Value>,
}
impl FlushTarget {
    #[must_use]
    pub fn coordinate(&self) -> &AcceptedCoordinate {
        &self.target
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct FlushReceipt {
    target: AcceptedCoordinate,
    saved: SavedEvidence,
}
impl FlushReceipt {
    #[must_use]
    pub fn saved(&self) -> &SavedEvidence {
        &self.saved
    }
}
/// Only compiled disposable adapters implement this. No caller booleans or
/// paths construct it. `verify` must use actual current package observations;
/// successful append/checkpoint/reconcile include all approved native barriers.
pub(crate) struct Presentation {
    pub graphs: Vec<Value>,
    pub undo: Vec<Value>,
    pub redo: Vec<Value>,
}
pub(crate) trait SessionIo {
    fn authorize(
        &self,
        binding: &SessionBinding,
        intent: Option<&Value>,
    ) -> Result<(), SessionError>;
    fn read_journal(&self) -> Result<Vec<u8>, SessionError>;
    fn verify<'a>(&self, journal: &'a Journal) -> Result<VerifiedJournal<'a>, SessionError>;
    fn selected_head(&self) -> Result<Value, SessionError>;
    /// After pure request/action validation, durably retain its exact original
    /// plan and full liability before Mutation admission. Complete earlier
    /// physical admission first; never construct an original from future bytes.
    fn admit_mutation(
        &mut self,
        intent: &Value,
        receipt: &Value,
        mutation: &JournalAppend,
    ) -> Result<(), SessionError>;
    fn append(&mut self, append: &JournalAppend) -> Result<(), EffectFailure>;
    fn checkpoint(&mut self, target: &AcceptedCoordinate) -> Result<(), EffectFailure>;
    /// Revalidate original scope, reconcile only original-bound outcomes, verify
    /// selected closure, then rebarrier complete journal/package before return.
    fn reconcile(&mut self) -> Result<(), SessionError>;
}
/// No public constructor: current admission is the approved private-image lab.
pub(crate) struct Session<I: SessionIo> {
    io: I,
    base: VerifiedOriginal,
    inputs: PlannerInputs,
    limits: Limits,
    journal_limits: JournalLimits,
    header: Value,
    snapshot: SessionSnapshot,
    flight: Option<FlushTarget>,
    first_dirty: Option<u64>,
    last_edit: Option<u64>,
}
impl<I: SessionIo> Session<I> {
    pub(crate) fn open(
        mut io: I,
        base: VerifiedOriginal,
        inputs: PlannerInputs,
        limits: Limits,
        journal_limits: JournalLimits,
        header: Value,
        binding: SessionBinding,
    ) -> Result<Self, SessionError> {
        if binding.project_id.to_string() != header["project_id"]
            || binding.incarnation_id.to_string() != header["incarnation_id"]
            || binding.owner_epoch.as_uuid().is_nil()
        {
            return Err(SessionError::InvalidRequest);
        }
        io.authorize(&binding, None)?;
        io.reconcile()?;
        let placeholder = AcceptedCoordinate {
            coordinate: Value::Null,
            mutation: None,
            accepted_frame: None,
        };
        let mut session = Self {
            io,
            base,
            inputs,
            limits,
            journal_limits,
            header,
            snapshot: SessionSnapshot {
                binding,
                event_sequence: 0,
                accepted: placeholder,
                saved: None,
                frozen: false,
                closed: false,
                failure: None,
                dirty: false,
            },
            flight: None,
            first_dirty: None,
            last_edit: None,
        };
        session.refresh()?;
        Ok(session)
    }
    fn inspect<T>(
        &self,
        f: impl FnOnce(&Journal, &VerifiedSession<'_>) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let bytes = self.io.read_journal()?;
        let journal = Journal::read(&bytes, &self.header, self.journal_limits)?;
        if journal.tail() != super::journal::JournalTail::Complete {
            return Err(SessionError::Failure(SessionFailure::Journal));
        }
        let verified = self.io.verify(&journal)?;
        let view = verified.verify_session(&self.base, &self.inputs, self.limits)?;
        f(&journal, &view)
    }
    fn refresh(&mut self) -> Result<(), SessionError> {
        let head = self.io.selected_head()?;
        let (accepted, checkpoint, saved) = self.inspect(|_, view| {
            if &head != view.base_head() {
                return Err(SessionError::Failure(SessionFailure::Conflict));
            }
            let checkpoint = AcceptedCoordinate::checkpoint(view);
            let saved = view.completed_checkpoint().map(|receipt| SavedEvidence {
                target: checkpoint.clone(),
                commit_sha256: head["commit_sha256"].as_str().unwrap_or("").to_owned(),
                head: head.clone(),
                checkpoint_receipt: receipt.clone(),
            });
            Ok((AcceptedCoordinate::current(view), checkpoint, saved))
        })?;
        self.snapshot.accepted = accepted;
        self.snapshot.dirty = self.snapshot.accepted != checkpoint;
        self.snapshot.saved = saved;
        if !self.snapshot.dirty {
            self.first_dirty = None;
            self.last_edit = None;
        }
        Ok(())
    }
    fn next_event(&self) -> Result<u64, SessionError> {
        self.snapshot
            .event_sequence
            .checked_add(1)
            .ok_or(SessionError::Failure(SessionFailure::Capacity))
    }
    fn live(&self) -> Result<(), SessionError> {
        if self.snapshot.closed {
            return Err(SessionError::Closed);
        }
        if self.snapshot.frozen {
            return Err(SessionError::Frozen);
        }
        self.io.authorize(&self.snapshot.binding, None)
    }
    pub(crate) fn freeze(&mut self, failure: SessionFailure) -> SessionError {
        self.snapshot.frozen = true;
        self.snapshot.failure = Some(failure);
        self.snapshot.event_sequence = self.snapshot.event_sequence.saturating_add(1);
        SessionError::Failure(failure)
    }
    #[must_use]
    pub(crate) fn snapshot(&self) -> &SessionSnapshot {
        &self.snapshot
    }
    pub(crate) fn submit(
        &mut self,
        intent: &Value,
        receipt: &Value,
        action: &Value,
        identity: JournalRecordIdentity,
        now_ms: u64,
    ) -> Result<Accepted, SessionError> {
        self.live()?;
        if self.flight.as_ref().is_some_and(|f| f.closing) {
            return Err(SessionError::Busy);
        }
        let event = self.next_event()?;
        self.io.authorize(&self.snapshot.binding, Some(intent))?;
        // Validate before any admission effects. Admission can append its exact
        // CheckpointIntent, so recompute the Mutation against the new prefix.
        let preview = self.inspect(|_, view| {
            view.prepare_mutation(
                intent,
                receipt,
                &self.snapshot.binding.owner,
                action,
                identity,
                self.journal_limits,
            )
            .map_err(|_| SessionError::InvalidRequest)
        })?;
        if self.io.admit_mutation(intent, receipt, &preview).is_err() {
            return Err(self.freeze(SessionFailure::Checkpoint));
        }
        let append = self.inspect(|_, view| {
            Ok(view.prepare_mutation(
                intent,
                receipt,
                &self.snapshot.binding.owner,
                action,
                identity,
                self.journal_limits,
            )?)
        })?;
        let body = &append.record().value()["body"];
        let accepted = Accepted {
            receipt: body["operation_receipt"].clone(),
            mutation: json!({"record_id":append.record().value()["record_id"],"record_checksum":append.record().checksum()}),
            result: body["result"].clone(),
            frame: body["accepted_frame"].clone(),
        };
        if self.io.append(&append).is_err() {
            return Err(self.freeze(SessionFailure::Storage));
        }
        if self.refresh().is_err() {
            return Err(self.freeze(SessionFailure::Journal));
        }
        self.snapshot.event_sequence = event;
        if self.snapshot.dirty {
            self.first_dirty.get_or_insert(now_ms);
            self.last_edit = Some(now_ms);
        }
        Ok(accepted)
    }
    pub(crate) fn begin_flush(
        &mut self,
        reason: &str,
        identity: JournalRecordIdentity,
    ) -> Result<FlushTarget, SessionError> {
        self.live()?;
        if self.flight.is_some() {
            return Err(SessionError::Busy);
        }
        if !matches!(
            reason,
            "flush" | "save-now" | "close" | "switch" | "sleep" | "terminate"
        ) {
            return Err(SessionError::InvalidRequest);
        }
        let event = self.next_event()?;
        let closing = matches!(reason, "close" | "switch" | "terminate");
        let link = if self.snapshot.dirty {
            let append = self.inspect(|_, view| {
                Ok(view.prepare_barrier(
                    &self.snapshot.binding.owner,
                    reason,
                    identity,
                    self.journal_limits,
                )?)
            })?;
            let link = json!({"record_id":append.record().value()["record_id"],"record_checksum":append.record().checksum()});
            if self.io.append(&append).is_err() {
                return Err(self.freeze(SessionFailure::Storage));
            }
            Some(link)
        } else {
            None
        };
        let target = FlushTarget {
            link,
            target: self.snapshot.accepted.clone(),
            closing,
            baseline: self
                .snapshot
                .saved
                .as_ref()
                .map(|s| s.checkpoint_receipt.clone()),
        };
        self.flight = Some(target.clone());
        self.snapshot.event_sequence = event;
        Ok(target)
    }
    pub(crate) fn finish_flush(
        &mut self,
        target: &FlushTarget,
    ) -> Result<FlushReceipt, SessionError> {
        self.live()?;
        let flight = self.flight.as_ref().ok_or(SessionError::InvalidRequest)?;
        if flight.link != target.link
            || flight.target != target.target
            || flight.closing != target.closing
            || flight.baseline != target.baseline
        {
            return Err(SessionError::InvalidRequest);
        }
        let event = self.next_event()?;
        if self.io.checkpoint(&target.target).is_err() {
            return Err(self.freeze(SessionFailure::Checkpoint));
        }
        if self.refresh().is_err() {
            return Err(self.freeze(SessionFailure::Journal));
        }
        let Some(saved) = self.snapshot.saved.clone() else {
            return Err(self.freeze(SessionFailure::Checkpoint));
        };
        let Ok(covered) = self.inspect(|journal, _| Ok(covers(journal, target, &saved))) else {
            return Err(self.freeze(SessionFailure::Journal));
        };
        if !covered {
            return Err(self.freeze(SessionFailure::Checkpoint));
        }
        self.flight = None;
        self.snapshot.closed = target.closing;
        self.snapshot.event_sequence = event;
        Ok(FlushReceipt {
            target: target.target.clone(),
            saved,
        })
    }
    pub(crate) fn retry(&mut self) -> Result<(), SessionError> {
        if self.snapshot.closed {
            return Err(SessionError::Closed);
        }
        self.io.authorize(&self.snapshot.binding, None)?;
        if self.io.reconcile().is_err() {
            return Err(self.freeze(SessionFailure::Storage));
        }
        if self.refresh().is_err() {
            return Err(self.freeze(SessionFailure::Journal));
        }
        self.snapshot.frozen = false;
        self.snapshot.failure = None;
        self.flight = None;
        self.snapshot.event_sequence = self.next_event()?;
        Ok(())
    }
    /// Existing PS0 tuning only; no persisted deadline or durability guarantee.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn autosave_due(&self, now_ms: u64) -> bool {
        !self.snapshot.frozen
            && !self.snapshot.closed
            && self.flight.is_none()
            && self.snapshot.dirty
            && (self
                .last_edit
                .is_some_and(|at| now_ms.saturating_sub(at) >= 1000)
                || self
                    .first_dirty
                    .is_some_and(|at| now_ms.saturating_sub(at) >= 5000))
    }
    pub(crate) fn undo_targets(&self) -> Result<Vec<Value>, SessionError> {
        self.live()?;
        self.inspect(|_, v| Ok(v.undo_targets(&self.snapshot.binding.owner)))
    }
    pub(crate) fn redo_targets(&self) -> Result<Vec<Value>, SessionError> {
        self.live()?;
        self.inspect(|_, v| Ok(v.redo_targets(&self.snapshot.binding.owner)))
    }
    /// One fresh proof for a single response; never retained across effects.
    pub(crate) fn presentation(&self) -> Result<Presentation, SessionError> {
        self.io.authorize(&self.snapshot.binding, None)?;
        self.inspect(|_, v| {
            let enabled = !self.snapshot.frozen && !self.snapshot.closed;
            Ok(Presentation {
                graphs: v.graph_objects()?,
                undo: if enabled {
                    v.undo_targets(&self.snapshot.binding.owner)
                } else {
                    vec![]
                },
                redo: if enabled {
                    v.redo_targets(&self.snapshot.binding.owner)
                } else {
                    vec![]
                },
            })
        })
    }
    /// Fresh full journal evidence for the controlled activation coordinator.
    /// The returned bytes are historical evidence, never an IO capability.
    #[cfg(all(target_os = "macos", any(test, feature = "controlled-disposable")))]
    pub(crate) fn activation_evidence(
        &self,
        registration: &str,
        graph_id: &str,
    ) -> Result<Value, SessionError> {
        self.live()?;
        self.io.authorize(&self.snapshot.binding, None)?;
        let head = self.io.selected_head()?;
        let saved = self
            .snapshot
            .saved
            .as_ref()
            .ok_or(SessionError::Failure(SessionFailure::Checkpoint))?;
        if self.snapshot.dirty || saved.target != self.snapshot.accepted || saved.head != head {
            return Err(SessionError::Failure(SessionFailure::Checkpoint));
        }
        self.inspect(|journal, view| {
            if AcceptedCoordinate::current(view) != self.snapshot.accepted || view.base_head() != &head || view.completed_checkpoint() != Some(&saved.checkpoint_receipt) {
                return Err(SessionError::Failure(SessionFailure::Checkpoint));
            }
            let receipt = journal.records().iter().find(|r| r.value()["record_id"] == saved.checkpoint_receipt["record_id"] && r.checksum() == saved.checkpoint_receipt["record_checksum"]).ok_or(SessionError::InvalidRequest)?;
            let intent = journal.records().iter().find(|r| r.value()["record_id"] == receipt.value()["body"]["intent_record_id"] && r.checksum() == receipt.value()["body"]["intent_record_checksum"]).ok_or(SessionError::InvalidRequest)?;
            let graph = view.graph_objects()?.into_iter().find(|v| v["graph"]["id"] == graph_id).ok_or(SessionError::InvalidRequest)?;
            Ok(json!({"saved":{"binding":self.snapshot.binding,"accepted":self.snapshot.accepted,"checkpoint_intent":{"value":intent.value(),"record_checksum":intent.checksum()},"checkpoint_receipt":{"value":receipt.value(),"record_checksum":receipt.checksum()},"registration_sha256":registration},"graph":graph["graph"],"head":head,"commit":receipt.value()["body"]["selected_commit"]}))
        })
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn io_mut(&mut self) -> &mut I {
        &mut self.io
    }
    #[cfg(test)]
    pub(crate) fn into_io(self) -> I {
        self.io
    }
    pub(crate) fn prepare_graph(
        &self,
        mut command: Value,
        operation: PackageUuid,
        updated_at: &str,
        provenance: &Value,
        action: &Value,
    ) -> Result<(Value, Value), SessionError> {
        self.live()?;
        self.inspect(|_, view| {
            if action["kind"] != "edit" { command = view.inverse_command(&self.snapshot.binding.owner,action)?; }
            let before=view.coordinate();
            let graph=command["envelope"]["graph_id"].as_str().ok_or(SessionError::InvalidRequest)?.to_owned();
            let revision=super::wire::number(&before["graphs"][&graph]["revision"]).map_err(|_|SessionError::InvalidRequest)?;
            command["envelope"]["expected_revision"]=json!(revision);
            command["envelope"]["command_id"]=json!(operation);
            let intent=json!({"domain":"photara.package.operation-intent.v1","version":1,"project_id":self.header["project_id"],"library_id":self.header["library_id"],"bootstrap_sha256":self.header["bootstrap_sha256"],"operation_id":operation,"expected":before,"command":command,"updated_at":updated_at,"boundary":"single","undo_group_id":null});
            let after=view.preview(&intent)?;
            let (ordinal,sequence)=view.next_receipt_coordinates()?;
            let receipt=json!({"schema":{"id":"photara.package.operation-receipt","version":1},"project_id":self.header["project_id"],"library_id":self.header["library_id"],"bootstrap_sha256":self.header["bootstrap_sha256"],"operation_id":operation,"request_sha256":super::wire::hash(&super::wire::encode(&intent)),"before":{"revision":before["revision"],"digest":before["authored_digest"]},"after":{"revision":after["revision"],"digest":after["authored_digest"]},"outcome":"accepted","journal_id":self.header["journal_id"],"journal_sequence":sequence.to_string(),"acceptance_ordinal":ordinal.to_string(),"undo_group_id":null,"provenance":provenance,"extensions":{}});
            Ok((intent,receipt))
        })
    }
}

fn covers(journal: &Journal, target: &FlushTarget, saved: &SavedEvidence) -> bool {
    if saved.target == target.target {
        return true;
    }
    let index = |link: &Value| {
        journal.records().iter().position(|r| {
            r.value()["record_id"] == link["record_id"] && r.checksum() == link["record_checksum"]
        })
    };
    if let (Some(wanted), Some(through)) = (&target.target.mutation, &saved.target.mutation) {
        let (Some(a), Some(b)) = (index(wanted), index(through)) else {
            return false;
        };
        let body = &journal.records()[a].value()["body"];
        return a <= b
            && journal.records()[a].value()["kind"] == "Mutation"
            && body["result"] == target.target.coordinate
            && target.target.accepted_frame.as_ref() == Some(&body["accepted_frame"]);
    }
    match (&target.baseline, index(&saved.checkpoint_receipt)) {
        (Some(base), Some(end)) if target.target.mutation.is_none() => {
            index(base).is_some_and(|start| start <= end)
        }
        _ => false,
    }
}
