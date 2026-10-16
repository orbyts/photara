//! Existing corpus, real Core/journal/repeatable executor; synthetic IO faults.
use super::super::checkpoint::{ActivationLimits, activate};
use super::super::{
    journal::{Journal, JournalAppend, JournalLimits, JournalRecordIdentity, VerifiedJournal},
    session::*,
};
use super::*;
use crate::package::PackageError;
struct Archived {
    objects: BTreeMap<super::super::wire::Key, Value>,
    stream: Value,
    bytes: Vec<u8>,
    reads: std::sync::atomic::AtomicUsize,
}
impl super::super::checkpoint::CheckpointRecords for Archived {
    fn record(&self, reference: &Value, max: usize) -> Result<Vec<u8>, PackageError> {
        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let value = self
            .objects
            .get(&super::super::wire::key(reference).map_err(|_| PackageError::Record)?)
            .ok_or(PackageError::Integrity)?;
        let bytes = encode(value);
        if bytes.len() > max {
            return Err(PackageError::Limit);
        }
        Ok(bytes)
    }
    fn frame(
        &self,
        stream: package::PackageUuid,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, PackageError> {
        if self.stream != json!(stream.to_string()) {
            return Err(PackageError::Integrity);
        }
        let start = usize::try_from(offset).map_err(|_| PackageError::Limit)?;
        let end = start.checked_add(length).ok_or(PackageError::Limit)?;
        self.bytes
            .get(start..end)
            .map(<[u8]>::to_vec)
            .ok_or(PackageError::Integrity)
    }
}
struct LocalRecords(BTreeMap<super::super::wire::Key, Value>);
impl super::super::checkpoint::CheckpointRecords for LocalRecords {
    fn record(&self, reference: &Value, max: usize) -> Result<Vec<u8>, package::PackageError> {
        let value = self
            .0
            .get(&super::super::wire::key(reference).map_err(|_| package::PackageError::Record)?)
            .ok_or(package::PackageError::Integrity)?;
        let bytes = encode(value);
        if bytes.len() > max {
            return Err(package::PackageError::Limit);
        }
        Ok(bytes)
    }
    fn frame(
        &self,
        _: package::PackageUuid,
        _: u64,
        _: usize,
    ) -> Result<Vec<u8>, package::PackageError> {
        Err(package::PackageError::Integrity)
    }
}
struct Cold(Option<super::super::checkpoint::ReopenState>);
fn saved2_proof(mut evidence: Value, binding: Value, accepted: Value, golden: bool) {
    use crate::package::v1_3::activation;
    evidence["binding"] = binding;
    evidence["accepted"] = accepted;
    evidence["registration_sha256"] = json!("b".repeat(64));
    let record = json!({"id":uid(799_000),"kind":"SavedProof","version":2,"body":evidence});
    let limits = activation::Limits {
        json: support::limits().json,
        max_records: 16,
        max_snapshot_bytes: support::limits().json.max_bytes,
    };
    let parse = |record: &Value| {
        let mut body = json!({"device_id":record["body"]["journal_header"]["device_id"],"workspace_slot_id":uid(799_001),"revision":"0","request_generation":"0","committed_generation":"0","authority_scope_sha256":"a".repeat(64),"active":null,"pending":null,"records":[record]});
        let first = activation::Snapshot::from_body(body.clone(), limits);
        let object = body.as_object_mut().unwrap();
        object.remove("authority_scope_sha256");
        object.insert("slot_scope_sha256".into(), json!("a".repeat(64)));
        let second = activation::v2::Snapshot::from_body(
            body,
            activation::v2::Limits {
                json: limits.json,
                max_records: limits.max_records,
                max_snapshot_bytes: limits.max_snapshot_bytes,
            },
        );
        (first, second)
    };
    let (first, second) = parse(&record);
    first.expect("SavedProof2 PS4 consumer");
    second.expect("SavedProof2 LL2a consumer");
    let expected = match record["body"]["accepted"]["coordinate"]["revision"]
        .as_str()
        .unwrap()
    {
        "3" => (
            20659,
            "727dcfbf4359f82a167da48da561f2cda5da894baad634d32e63117c179df218",
        ),
        "4" => (
            39502,
            "259424e72a8ba6963d2ebe97126d7034a1f996fa0a5dd381b0ebe6cecc65a71f",
        ),
        "5" => (
            40518,
            "53218f5e197b0526d64ada743bc34d72091132737dbff1e8f297853ce06e05a9",
        ),
        _ => unreachable!(),
    };
    if golden {
        assert_eq!(
            (encode(&record).len(), hash(&encode(&record))),
            (expected.0, expected.1.to_owned())
        );
    }
    for path in [
        "/version",
        "/body/journal_header/format_version",
        "/body/checkpoint/receipt/value/sequence",
        "/body/accepted/coordinate/revision",
        "/body/local_base/checkpoint/sha256",
        "/body/checkpoint/kind",
    ] {
        let mut changed = record.clone();
        *changed.pointer_mut(path).unwrap() = json!(0);
        let (first, second) = parse(&changed);
        assert!(first.is_err() && second.is_err(), "{path}");
    }
    let mut changed = record.clone();
    changed["body"]["checkpoint"]["extra"] = json!(true);
    assert!(parse(&changed).0.is_err());
    changed = record;
    changed["version"] = json!(1);
    assert!(parse(&changed).0.is_err());
}
impl super::super::checkpoint::CheckpointRecoveryIo for Cold {
    type Lease = ();
    fn read_checkpoint(
        &mut self,
        _: &crate::package::planning::io::RegisteredCooperativeLease<()>,
        _: ActivationLimits<'_>,
    ) -> Result<super::super::checkpoint::ReopenState, package::PackageError> {
        self.0.take().ok_or(package::PackageError::Integrity)
    }
}

fn limits() -> JournalLimits {
    JournalLimits {
        json: support::limits().json,
        max_file_bytes: 262_144,
        max_records: 64,
        max_work_bytes: 262_144,
    }
}
fn rid(n: u64) -> JournalRecordIdentity {
    JournalRecordIdentity {
        record_id: package::PackageUuid::parse(&uid(n)).unwrap(),
        session_generation: 1,
    }
}
fn binding() -> SessionBinding {
    SessionBinding {
        project_id: package::PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap(),
        incarnation_id: package::PackageUuid::parse("96000000-0000-4000-8000-000000000001")
            .unwrap(),
        owner_epoch: package::PackageUuid::parse(&uid(91001)).unwrap(),
        owner: json!({"attachment_id":uid(91002),"attachment_generation":"1","principal":request().1["provenance"]["principal"]}),
    }
}
struct Backend {
    mock: Mock,
    bytes: Vec<u8>,
    header: Value,
    fail_after_append: bool,
    checkpoint_fail: bool,
    omit_checkpoint: bool,
    maintenance_calls: usize,
    maintenance_failure: Option<SessionFailure>,
    verifications: std::rc::Rc<std::cell::Cell<usize>>,
    memo: std::cell::RefCell<super::super::restart::CompiledPlans>,
}
/// Real independently verified checkpoint views; only the IO swap is synthetic.
struct RotatingSession<'a> {
    current: &'a super::super::checkpoint::ActivatedCheckpoint,
    next: Option<&'a super::super::checkpoint::ActivatedCheckpoint>,
    bytes: Vec<u8>,
    binding: SessionBinding,
}
impl SessionIo for RotatingSession<'_> {
    fn authorize(&self, binding: &SessionBinding, _: Option<&Value>) -> Result<(), SessionError> {
        if binding == &self.binding {
            Ok(())
        } else {
            Err(SessionError::InvalidRequest)
        }
    }
    fn read_journal(&self) -> Result<Vec<u8>, SessionError> {
        Ok(self.bytes.clone())
    }
    fn verify<'a>(&self, _: &'a Journal) -> Result<VerifiedJournal<'a>, SessionError> {
        Err(SessionError::InvalidRequest)
    }
    fn verify_view<'a>(
        &self,
        journal: &'a Journal,
        _: &VerifiedOriginal,
        _: &PlannerInputs,
        limits: Limits,
    ) -> Result<super::super::journal::VerifiedSession<'a>, SessionError> {
        Ok(self.current
            .verify_session(journal, frames(), &support::limits(), limits)?)
    }
    fn selected_head(&self) -> Result<Value, SessionError> {
        Ok(self.current.package().shared_package().head.clone())
    }
    fn maintain_checkpoint(
        &mut self,
        scheduled: bool,
        _: &Value,
    ) -> Result<Option<(VerifiedOriginal, Value)>, SessionError> {
        if scheduled && let Some(next) = self.next.take() {
            self.current = next;
            self.bytes = next.segment().to_vec();
            return Ok(Some((next.package().shared_clone(), next.header().clone())));
        }
        Ok(None)
    }
    fn admit_mutation(
        &mut self,
        _: &Value,
        _: &Value,
        _: &JournalAppend,
    ) -> Result<(), SessionError> {
        Err(SessionError::InvalidRequest)
    }
    fn append(&mut self, _: &JournalAppend) -> Result<(), EffectFailure> {
        Err(EffectFailure::OutcomeUnknown)
    }
    fn checkpoint(&mut self, _: &AcceptedCoordinate) -> Result<(), EffectFailure> {
        Err(EffectFailure::OutcomeUnknown)
    }
    fn reconcile(&mut self) -> Result<(), SessionError> {
        Ok(())
    }
}
impl Backend {
    fn context<'a>(
        &'a self,
        observed: &'a support::Fixture,
        id: &'a v1_3::SelectionIdentity,
        reader: &'a v1_3::ReaderLimits,
    ) -> RestartContext<'a, support::Fixture> {
        RestartContext {
            inspection: observed,
            registration: &self.mock.base.registration,
            directory: v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            identity: id,
            frames: frames(),
            reader,
            planner: budget(),
            max_originals: 16,
        }
    }
    fn parsed(&self) -> Journal {
        Journal::read(&self.bytes, &self.header, limits()).unwrap()
    }
    fn original(&self) -> VerifiedOriginal {
        let observed = snapshot(&self.mock.base, &self.mock.current);
        let id = identity(&self.mock.current);
        let reader = support::limits();
        if self.mock.current.head == package(&self.mock.base).head {
            return v1_3::reader::verify_shared_original(
                self.mock.current.clone(),
                id,
                &observed,
                &self.mock.base.registration,
                v1_3::DirectoryObservation {
                    device: 7,
                    inode: 900,
                },
                frames(),
                &reader,
            )
            .unwrap();
        }
        let p = restore_plan(&self.mock.current, &self.context(&observed, &id, &reader)).unwrap();
        v1_3::reader::verify_operation_original(
            self.mock.current.clone(),
            id,
            &observed,
            &self.mock.base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &reader,
            &p,
            6,
        )
        .unwrap()
    }
    fn append_raw(&mut self, a: &JournalAppend) {
        assert_eq!(self.bytes.len(), a.expected_extent());
        assert_eq!(hash(&self.bytes), a.expected_prefix_sha256());
        self.bytes.extend(a.bytes());
    }
    fn persisted_plan(&self, operation: &str) -> RepeatablePlan {
        let j = self.parsed();
        let b = &j
            .records()
            .iter()
            .find(|r| {
                r.value()["kind"] == "CheckpointIntent"
                    && r.value()["body"]["operation_receipt"]["operation_id"] == operation
            })
            .unwrap()
            .value()["body"];
        let observed = snapshot(&self.mock.base, &self.mock.current);
        let id = identity(&self.mock.current);
        let reader = support::limits();
        super::super::restart::restore_intent_plan(
            &self.mock.current,
            &b["original"],
            &b["semantic_intent"],
            &b["operation_receipt"],
            &self.context(&observed, &id, &reader),
        )
        .unwrap()
    }
}
impl SessionIo for Backend {
    fn maintain_checkpoint(
        &mut self,
        scheduled: bool,
        _: &Value,
    ) -> Result<Option<(VerifiedOriginal, Value)>, SessionError> {
        if scheduled {
            self.maintenance_calls += 1;
            if let Some(failure) = self.maintenance_failure {
                return Err(SessionError::Failure(failure));
            }
        }
        Ok(None)
    }
    fn authorize(&self, b: &SessionBinding, _: Option<&Value>) -> Result<(), SessionError> {
        if *b != binding() {
            return Err(SessionError::Failure(SessionFailure::Authorization));
        }
        Ok(())
    }
    fn read_journal(&self) -> Result<Vec<u8>, SessionError> {
        Ok(self.bytes.clone())
    }
    fn verify<'a>(&self, j: &'a Journal) -> Result<VerifiedJournal<'a>, SessionError> {
        self.verifications.set(self.verifications.get() + 1);
        let observed = snapshot(&self.mock.base, &self.mock.current);
        let id = identity(&self.mock.current);
        let reader = support::limits();
        Ok(j.verify_cached(
            &self.mock.current,
            &self.context(&observed, &id, &reader),
            &mut self.memo.borrow_mut(),
        )?)
    }
    fn verify_session<'a>(
        &self,
        verified: &VerifiedJournal<'a>,
        base: &VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: Limits,
    ) -> Result<super::super::journal::VerifiedSession<'a>, SessionError> {
        Ok(verified.verify_session_cached(base, inputs, limits, &mut self.memo.borrow_mut())?)
    }
    fn selected_head(&self) -> Result<Value, SessionError> {
        Ok(self.mock.current.head.clone())
    }
    fn append(&mut self, a: &JournalAppend) -> Result<(), EffectFailure> {
        if self.fail_after_append {
            self.fail_after_append = false;
            self.append_raw(a);
            return Err(EffectFailure::OutcomeUnknown);
        }
        self.append_raw(a);
        Ok(())
    }
    fn reconcile(&mut self) -> Result<(), SessionError> {
        let j = self.parsed();
        self.verify(&j)?;
        Ok(())
    }
    fn admit_mutation(
        &mut self,
        intent: &Value,
        receipt: &Value,
        _: &JournalAppend,
    ) -> Result<(), SessionError> {
        let j = self.parsed();
        // Original operation lookup precedes any newer automatic checkpoint.
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointIntent"
                && r.value()["body"]["operation_receipt"]["operation_id"] == receipt["operation_id"]
        }) {
            let plan = self.persisted_plan(receipt["operation_id"].as_str().unwrap());
            if plan.prepared.receipt != *receipt
                || plan.original["request"]["intent"] != reference(intent)
            {
                return Err(SessionError::InvalidRequest);
            }
            return Ok(());
        }
        let original = verify(&self.mock.base);
        let seed = attempt(&original, 1000).planner;
        let verified = self.verify(&j)?;
        let view = self.verify_session(&verified, &original, &seed, budget())?;
        if let Some(frame) = view.accepted_frame() {
            let target = AcceptedCoordinate {
                coordinate: view.coordinate(),
                mutation: view.through(),
                accepted_frame: Some(frame),
            };
            self.checkpoint(&target)
                .map_err(|_| SessionError::Failure(SessionFailure::Checkpoint))?;
        }
        let j = self.parsed();
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointIntent"
                && r.value()["body"]["operation_receipt"]["operation_id"] == receipt["operation_id"]
        }) {
            let p = self.persisted_plan(receipt["operation_id"].as_str().unwrap());
            if p.prepared.receipt != *receipt {
                return Err(SessionError::InvalidRequest);
            }
            return Ok(());
        }
        let current = self.original();
        let n = wire::number(&receipt["acceptance_ordinal"]).unwrap() * 1000;
        let p =
            plan::compile_inner(&current, intent, receipt, attempt(&current, n), budget()).unwrap();
        let original = verify(&self.mock.base);
        let seed = attempt(&original, 1000).planner;
        let v = self.verify(&j)?;
        let session = v.verify_session(&original, &seed, budget())?;
        let append = session.prepare_checkpoint_intent(&v, &p, rid(92000 + n), limits())?;
        self.append_raw(&append);
        Ok(())
    }
    fn checkpoint(&mut self, target: &AcceptedCoordinate) -> Result<(), EffectFailure> {
        if self.checkpoint_fail {
            return Err(EffectFailure::OutcomeUnknown);
        }
        if self.omit_checkpoint {
            return Ok(());
        }
        let Some(frame) = &target.accepted_frame else {
            return Ok(());
        };
        let operation = frame["operation_id"].as_str().unwrap();
        let p = self.persisted_plan(operation);
        let j = self.parsed();
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointReceipt"
                && r.value()["body"]["operation_receipt"]["operation_id"] == operation
        }) {
            return Ok(());
        }
        // Native prepares the exact wrapper before effects, then appends it only
        // after the executor has independently checked and barriered clean HEAD.
        let j = self.parsed();
        let intent = j
            .records()
            .iter()
            .find(|r| {
                r.value()["kind"] == "CheckpointIntent"
                    && r.value()["body"]["operation_receipt"]["operation_id"] == operation
            })
            .unwrap();
        let id =
            package::PackageUuid::parse(intent.value()["record_id"].as_str().unwrap()).unwrap();
        let v = self.verify(&j).unwrap();
        let append = v
            .prepare_receipt(
                &p,
                id,
                rid(97000 + wire::number(&p.prepared.receipt["acceptance_ordinal"]).unwrap()),
                limits(),
            )
            .unwrap();
        let lease = lease(&p.old);
        execute(&mut self.mock, &lease, &p).map_err(|_| EffectFailure::OutcomeUnknown)?;
        self.append_raw(&append);
        Ok(())
    }
}
fn open() -> Session<Backend> {
    let base = fixture();
    let original = verify(&base);
    let seed = attempt(&original, 1000).planner;
    let p = &original.package;
    let header = json!({"format_version":1,"stream_id":uid(98001),"journal_id":request().1["journal_id"],"device_id":uid(98002),"project_id":p.manifest["project_id"],"library_id":p.commit["root_set"]["library_id"],"incarnation_id":binding().incarnation_id,"bootstrap_sha256":hash(&encode(&p.manifest)),"base_head":p.head,"base_package_revision":p.commit["package_revision"]});
    let bytes = Journal::create(&header, limits()).unwrap();
    let backend = Backend {
        mock: Mock {
            current: p.clone(),
            base,
            receipts: BTreeMap::new(),
            journals: BTreeMap::new(),
            interrupt_append: false,
            interrupt_select: None,
            fail_stabilize: false,
            effects: 0,
        },
        bytes,
        header: header.clone(),
        fail_after_append: false,
        checkpoint_fail: false,
        omit_checkpoint: false,
        maintenance_calls: 0,
        maintenance_failure: None,
        verifications: std::rc::Rc::new(std::cell::Cell::new(0)),
        memo: std::cell::RefCell::default(),
    };
    Session::open(
        backend,
        original,
        seed,
        budget(),
        limits(),
        header,
        binding(),
    )
    .unwrap()
}
fn edit(s: &Session<Backend>, n: u64, x: i64) -> (Value, Value) {
    let mut command = request().0["command"].clone();
    command["envelope"]["command"]["x"] = json!(x);
    s.prepare_graph(
        command,
        package::PackageUuid::parse(&uid(n)).unwrap(),
        "2026-09-27T00:00:00.000Z",
        &request().1["provenance"],
        &json!({"kind":"edit"}),
    )
    .unwrap()
}
#[test]
fn session_accepted_finite_flush_and_original_retry() {
    let mut s = open();
    assert!(s.snapshot().saved.is_none());
    let (a, ar) = edit(&s, 101_001, 30);
    let ack = s
        .submit(&a, &ar, &json!({"kind":"edit"}), rid(102_001), 0)
        .unwrap();
    assert_eq!(ack.receipt(), &ar);
    assert_eq!(ack.accepted_frame(), &plan::journal_frame(&ar));
    let Presentation { graphs, undo, redo } = s.presentation().unwrap();
    assert!(!graphs.is_empty());
    assert_eq!(undo, s.undo_targets().unwrap());
    assert_eq!(redo, s.redo_targets().unwrap());
    assert!(s.snapshot().dirty);
    assert!(!s.autosave_due(999));
    assert!(s.autosave_due(1000));
    let barrier = s.begin_flush("save-now", rid(102_002)).unwrap();
    let (b, br) = edit(&s, 101_002, 40);
    s.submit(&b, &br, &json!({"kind":"edit"}), rid(102_003), 100)
        .unwrap();
    let current = s.snapshot().accepted.clone();
    let flushed = s.finish_flush(&barrier).unwrap();
    assert_eq!(flushed.saved().target, barrier.coordinate().clone());
    assert!(s.snapshot().dirty);
    assert_eq!(s.snapshot().accepted, current);
    let retry = s
        .submit(&a, &ar, &json!({"kind":"edit"}), rid(102_001), 101)
        .unwrap();
    assert_eq!(retry.receipt(), &ar);
    assert_eq!(s.snapshot().accepted, current);
    let end = s.begin_flush("close", rid(102_004)).unwrap();
    s.finish_flush(&end).unwrap();
    assert!(s.snapshot().closed && !s.snapshot().dirty);
}

#[test]
fn session_idle_maintenance_requires_saved_and_preserves_failed_target() {
    let mut session = open();
    session.maintain().unwrap();
    assert_eq!(session.io_mut().maintenance_calls, 0);
    let (intent, receipt) = edit(&session, 109_001, 31);
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(109_002), 0)
        .unwrap();
    session.maintain().unwrap();
    assert_eq!(session.io_mut().maintenance_calls, 0);
    let target = session.begin_flush("flush", rid(109_003)).unwrap();
    session.maintain().unwrap();
    assert_eq!(session.io_mut().maintenance_calls, 0);
    session.finish_flush(&target).unwrap();
    let accepted = session.snapshot().accepted.clone();
    let head = session.snapshot().saved.as_ref().unwrap().head.clone();
    let bytes = session.io_mut().bytes.clone();
    session.maintain().unwrap();
    assert_eq!(session.io_mut().maintenance_calls, 1);
    session.io_mut().maintenance_failure = Some(SessionFailure::Storage);
    assert!(session.maintain().is_err());
    assert!(session.snapshot().frozen);
    assert_eq!(session.snapshot().accepted, accepted);
    assert_eq!(session.snapshot().saved.as_ref().unwrap().head, head);
    assert_eq!(session.io_mut().bytes, bytes);
    session.retry().unwrap();
    assert!(!session.snapshot().frozen);
    assert_eq!(session.snapshot().accepted, accepted);
}

#[test]
fn session_newer_checkpoint_covers_only_proven_finite_target() {
    let mut session = open();
    let (first, receipt) = edit(&session, 103_001, 31);
    session
        .submit(&first, &receipt, &json!({"kind":"edit"}), rid(104_001), 0)
        .unwrap();
    let captured = session.begin_flush("save-now", rid(104_002)).unwrap();
    let (second, receipt2) = edit(&session, 103_002, 41);
    session
        .submit(
            &second,
            &receipt2,
            &json!({"kind":"edit"}),
            rid(104_003),
            20,
        )
        .unwrap();
    let later = session.snapshot().accepted.clone();
    session.io_mut().checkpoint(&later).unwrap();
    let result = session.finish_flush(&captured).unwrap();
    assert_eq!(result.saved().target, later);
    assert!(!session.snapshot().dirty);
    assert_ne!(&result.saved().target, captured.coordinate());
}
#[test]
fn session_unknown_append_freezes_reopens_and_requires_saved_evidence() {
    let mut session = open();
    let (intent, receipt) = edit(&session, 105_001, 32);
    session.io_mut().fail_after_append = true;
    assert!(
        session
            .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(106_001), 0)
            .is_err()
    );
    assert!(session.snapshot().frozen && session.snapshot().saved.is_none());
    assert!(
        session
            .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(106_001), 1)
            .is_err()
    );
    session.retry().unwrap();
    let original = session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(106_001), 2)
        .unwrap();
    assert_eq!(original.receipt(), &receipt);
    let backend = session.into_io();
    let base = verify(&backend.mock.base);
    let seed = attempt(&base, 1000).planner;
    let header = backend.header.clone();
    let mut session =
        Session::open(backend, base, seed, budget(), limits(), header, binding()).unwrap();
    assert!(session.snapshot().dirty && !session.snapshot().frozen);
    session.io_mut().omit_checkpoint = true;
    let target = session.begin_flush("save-now", rid(106_002)).unwrap();
    assert!(session.finish_flush(&target).is_err());
    assert!(session.snapshot().frozen && session.snapshot().saved.is_none());
    session.io_mut().omit_checkpoint = false;
    session.retry().unwrap();
    session.io_mut().checkpoint_fail = true;
    let target = session.begin_flush("save-now", rid(106_002)).unwrap();
    assert!(session.finish_flush(&target).is_err());
    assert!(session.snapshot().frozen);
    session.io_mut().checkpoint_fail = false;
    session.retry().unwrap();
    let target = session.begin_flush("save-now", rid(106_002)).unwrap();
    session.finish_flush(&target).unwrap();
    assert!(!session.snapshot().dirty && session.snapshot().saved.is_some());
}

#[test]
fn response_projection_consumes_same_request_proof_and_failure_rechecks() {
    let mut session = open();
    let calls = std::rc::Rc::clone(&session.io_mut().verifications);
    let (intent, receipt) = edit(&session, 107_001, 53);
    session.begin_response();
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(108_001), 0)
        .unwrap();
    let before = calls.get();
    let projected = session.finish_response().unwrap();
    assert_eq!(
        calls.get(),
        before,
        "reply must not replay the history again"
    );
    let fresh = session.presentation().unwrap();
    assert_eq!(calls.get(), before + 1);
    assert_eq!(
        (projected.graphs, projected.undo, projected.redo),
        (fresh.graphs, fresh.undo, fresh.redo)
    );

    session.begin_response();
    let target = session.begin_flush("save-now", rid(108_002)).unwrap();
    session.finish_flush(&target).unwrap();
    let before = calls.get();
    session.finish_response().unwrap();
    assert_eq!(
        calls.get(),
        before,
        "Saved and reply share the just-verified completed prefix"
    );
    session.begin_response();
    session.finish_response().unwrap();
    assert_eq!(
        calls.get(),
        before + 1,
        "a new request cannot reuse presentation"
    );

    let (intent, receipt) = edit(&session, 107_002, 63);
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(108_003), 1)
        .unwrap();
    session.io_mut().checkpoint_fail = true;
    session.begin_response();
    let target = session.begin_flush("save-now", rid(108_004)).unwrap();
    assert!(session.finish_flush(&target).is_err());
    assert!(session.snapshot().frozen);
    let before = calls.get();
    let failure = session.finish_response().unwrap();
    assert_eq!(
        calls.get(),
        before + 1,
        "failure must freshly verify presentation"
    );
    assert!(failure.undo.is_empty() && failure.redo.is_empty());
    session.io_mut().checkpoint_fail = false;
    session.begin_response();
    session.retry().unwrap();
    let before = calls.get();
    session.finish_response().unwrap();
    assert_eq!(calls.get(), before);
}

#[test]
fn generic_graph_batch_reopens_exactly_and_unavailable_edits_have_no_effects() {
    // Real Core, journal, planner and replay; this Backend models IO and is not a native durability proof.
    let mut session = open();
    let node = request().0["command"]["envelope"]["command"]["node_id"].clone();
    let mut command = request().0["command"].clone();
    command["envelope"]["command"] = json!({"kind":"batch","commands":[
        {"kind":"set-node-position","node_id":node,"x":17,"y":18},
        {"kind":"set-node-position","node_id":node,"x":71,"y":81}
    ]});
    let (intent, receipt) = session
        .prepare_graph(
            command.clone(),
            package::PackageUuid::parse(&uid(109_001)).unwrap(),
            "2026-10-09T00:00:00.000Z",
            &request().1["provenance"],
            &json!({"kind":"edit"}),
        )
        .unwrap();
    let ack = session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(110_001), 0)
        .unwrap();
    let target = session.begin_flush("save-now", rid(110_002)).unwrap();
    session.finish_flush(&target).unwrap();
    let expected = session.presentation().unwrap().graphs;
    assert_eq!(
        expected[0]["graph"]["nodes"][0]["photara.graph-position"],
        json!({"x":71,"y":81})
    );
    let backend = session.into_io();
    let base = verify(&backend.mock.base);
    let seed = attempt(&base, 1000).planner;
    let header = backend.header.clone();
    let mut reopened =
        Session::open(backend, base, seed, budget(), limits(), header, binding()).unwrap();
    assert_eq!(reopened.presentation().unwrap().graphs, expected);
    let retry = reopened
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(110_001), 1)
        .unwrap();
    assert_eq!(retry.receipt(), ack.receipt());
    assert_eq!(reopened.presentation().unwrap().graphs, expected);
    let before_journal = reopened.io_mut().bytes.clone();
    let before_head = reopened.io_mut().mock.current.head.clone();
    let before_effects = reopened.io_mut().mock.effects;
    let mut instance = expected[0]["graph"]["nodes"][0].clone();
    instance["id"] = json!(uid(109_099));
    for unsupported in [
        json!({"kind":"add-node","instance":instance}),
        json!({"kind":"delete-node","node_id":node}),
        json!({"kind":"connect","connection":{"id":uid(109_098),"output":{"node_id":node,"port_id":"out"},"input":{"node_id":node,"port_id":"in"}}}),
        json!({"kind":"disconnect","connection_ids":[uid(109_098)]}),
    ] {
        command["envelope"]["command"] = unsupported;
        assert!(
            reopened
                .prepare_graph(
                    command.clone(),
                    package::PackageUuid::parse(&uid(109_002)).unwrap(),
                    "2026-10-09T00:00:00.000Z",
                    &request().1["provenance"],
                    &json!({"kind":"edit"})
                )
                .is_err()
        );
    }
    assert_eq!(reopened.io_mut().bytes, before_journal);
    assert_eq!(reopened.io_mut().mock.current.head, before_head);
    assert_eq!(reopened.io_mut().mock.effects, before_effects);
}

#[test]
fn session_checkpoint_turnover_actual_append_and_four_selections() {
    checkpoint_turnover_actual_append_and_four_selections(false);
}
#[test]
fn session_sized_corridors_turnover_two_moves_and_retry() {
    checkpoint_turnover_actual_append_and_four_selections(true);
}

fn sized_compile<T, E>(
    sizing: bool,
    raw: Value,
    mut build: impl FnMut(&Value) -> Result<T, E>,
    files: impl Fn(&T) -> &BTreeMap<String, super::super::shared::Bytes>,
) -> Result<(T, Value), E> {
    if sizing {
        super::super::native_sizing::compile(raw, build, files)
    } else {
        Ok((build(&raw)?, raw))
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One original preflight and byte-pressure retry sequence"
)]
fn fresh_admission_reuses_compilation_before_package_effects() {
    // Real existing-corpus planner and journal preflight, synthetic immutable IO.
    let base = fixture();
    let original = verify(&base);
    let (intent, receipt) = request();
    let mut memo = super::super::restart::CompiledPlans::default();
    let initial_attempt = attempt(&original, 1000);
    super::super::layout::take_load_work();
    let plan = memo
        .compile(&original, &intent, &receipt, initial_attempt, budget(), 16)
        .unwrap();
    assert_eq!(
        super::super::layout::take_load_work().0,
        1,
        "Core/layout/cache share one original decoding pass"
    );
    assert_eq!(memo.compilations, 1);
    let mut header = super::journal_tests::header(original.shared_package());
    header["journal_id"] = receipt["journal_id"].clone();
    let bytes = Journal::create(&header, limits()).unwrap();
    let id = identity(original.shared_package());
    let reader = support::limits();
    let ctx = super::journal_tests::context(&base, &id, &reader);
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let prepared = journal
        .verify_shared(original.shared_package(), &ctx)
        .unwrap()
        .prepare_intent(&plan, rid(120_001), limits())
        .unwrap();
    let mut preflight = bytes.clone();
    preflight.extend(prepared.bytes());
    let parsed = Journal::read(&preflight, &header, limits()).unwrap();
    // No package effect or journal append has occurred: this is the exact next
    // immutable journal candidate inspected before its first durable write.
    parsed
        .verify_cached(original.shared_package(), &ctx, &mut memo)
        .unwrap();
    assert_eq!(
        memo.compilations, 1,
        "preflight must reuse the just-compiled original"
    );
    assert!(memo.hits > 0);
    let retry = super::super::restart::restore_intent_plan_cached(
        original.shared_package(),
        plan.original(),
        &intent,
        &receipt,
        &ctx,
        &mut memo,
    )
    .unwrap();
    assert_eq!(retry.original_bytes(), plan.original_bytes());
    assert_eq!(retry.files, plan.files);
    assert_eq!(
        memo.compilations, 1,
        "original retry must not compile again"
    );
    assert!(memo.retained().1 <= usize::try_from(budget().max_total_allocation_bytes).unwrap());
    let mut changed = original.shared_package().clone();
    changed.allocations.values_mut().next().unwrap().bytes[16] ^= 1;
    assert!(parsed.verify_cached(&changed, &ctx, &mut memo).is_err());
    assert_eq!(bytes, Journal::create(&header, limits()).unwrap());
    // A fresh admission must replace an older optional entry at the byte cap.
    // Distinct operation IDs use the same immutable original for this pure test.
    let mut intent2 = intent.clone();
    intent2["operation_id"] = json!(uid(120_010));
    intent2["command"]["envelope"]["command_id"] = intent2["operation_id"].clone();
    let mut receipt2 = receipt.clone();
    receipt2["operation_id"] = intent2["operation_id"].clone();
    receipt2["request_sha256"] = json!(hash(&encode(&intent2)));
    let cache_ceiling = memo.retained().1 + 4096;
    memo.set_test_ceiling(cache_ceiling);
    let fresh = memo
        .compile(
            &original,
            &intent2,
            &receipt2,
            attempt(&original, 2000),
            budget(),
            16,
        )
        .unwrap();
    assert_eq!(memo.retained().0, 1);
    assert!(memo.retained().1 <= cache_ceiling);
    let before_work = memo.compilations;
    let retry = super::super::restart::restore_intent_plan_cached(
        original.shared_package(),
        fresh.original(),
        &intent2,
        &receipt2,
        &ctx,
        &mut memo,
    )
    .unwrap();
    assert_eq!(retry.files, fresh.files);
    assert_eq!(
        memo.compilations, before_work,
        "fresh entry survives old-history cache saturation"
    );
    assert!(memo.retained().1 <= usize::try_from(budget().max_total_allocation_bytes).unwrap());
    let mut tight = budget();
    tight.max_frames = 1;
    assert!(
        plan::compile_inner(
            &original,
            &intent2,
            &receipt2,
            attempt(&original, 2000),
            tight
        )
        .is_err()
    );
}

#[test]
fn journal_snapshot_prefix_work_is_linear_and_changed_bytes_never_reuse_hash() {
    use super::super::restart::prefix_index::Prefixes;
    let base = fixture();
    let original = verify(&base);
    let source = original.shared_package();
    let maximum = usize::try_from(budget().max_total_allocation_bytes).unwrap();
    let indexed = Prefixes::new(source, maximum);
    let disabled = Prefixes::new(source, 0);
    let mut ordinary_work = 0usize;
    let total = source
        .allocations
        .values()
        .map(|a| a.bytes.len())
        .sum::<usize>();
    // Thirty-two chronological historical boundaries over actual fixture bytes,
    // with the three consumers (reconstruct/materialize/fingerprint) per step.
    for step in 1..=32 {
        for (id, a) in &source.allocations {
            let end = a.bytes.len() * step / 32;
            let bytes = &a.bytes[..end];
            let expected = hash(bytes);
            for _ in 0..3 {
                assert_eq!(indexed.hash(id, bytes), expected);
                assert_eq!(disabled.hash(id, bytes), expected);
                ordinary_work += bytes.len();
            }
        }
    }
    assert_eq!(indexed.hashed.get(), total);
    assert_eq!(disabled.hashed.get(), ordinary_work);
    assert!(ordinary_work > total * 40);
    let (id, a) = source.allocations.iter().next().unwrap();
    let mut changed = a.bytes.clone();
    changed[16] ^= 1;
    assert_eq!(indexed.hash(id, &changed), hash(&changed));
    assert_ne!(indexed.hash(id, &changed), hash(&a.bytes));
    assert_eq!(indexed.hash(id, &a.bytes[..17]), hash(&a.bytes[..17]));
    // Another actual snapshot always creates a fresh index; no stored hash or
    // checked observation can cross requests, even under the same allocation ID.
    let mut changed_source = source.clone();
    changed_source.allocations.get_mut(id).unwrap().bytes = changed.clone();
    let next = Prefixes::new(&changed_source, maximum);
    assert_eq!(next.hash(id, &changed), hash(&changed));
    assert_eq!(next.hashed.get(), changed.len());
    eprintln!(
        "32-step prefix work indexed={total} ordinary={ordinary_work} ratio_at_least={}",
        ordinary_work / total
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One four-operation workload compares cold, warm, native-admission and altered-input paths"
)]
fn completed_journal_prefix_extends_without_historical_replay() {
    let mut session = open();
    for step in 0..4u64 {
        let (intent, receipt) = edit(&session, 130_001 + step, 50 + i64::try_from(step).unwrap());
        session
            .submit(
                &intent,
                &receipt,
                &json!({"kind":"edit"}),
                rid(131_001 + step * 2),
                step,
            )
            .unwrap();
        let flush = session
            .begin_flush("save-now", rid(131_002 + step * 2))
            .unwrap();
        session.finish_flush(&flush).unwrap();
        let memo = session.io_mut().memo.borrow();
        assert_eq!(
            memo.compilations,
            usize::try_from(step + 1).unwrap(),
            "new HEAD/extent must not recompile completed history"
        );
        assert!(
            memo.journal_replayed <= 4,
            "new HEAD/extent replays only its intent, Mutation, barrier and receipt"
        );
    }
    let backend = session.io_mut();
    let j = backend.parsed();
    let current = &backend.mock.current;
    let observed = snapshot(&backend.mock.base, current);
    let identity = identity(current);
    let reader = support::limits();
    let context = backend.context(&observed, &identity, &reader);
    let base = verify(&backend.mock.base);
    let planner = attempt(&base, 1000).planner;
    let expected = j.verify_shared(current, &context).unwrap();
    let mut memo = super::super::restart::CompiledPlans::default();
    j.verify_cached(current, &context, &mut memo).unwrap();
    let cold_records = memo.journal_replayed;
    // Reproduce the measured native cache cliff at a smaller optional budget:
    // the old full-Original metadata cannot fit, while commitments do. No
    // acceptance limit changes; an even smaller memo falls back to full replay.
    let compact_cost = memo.journal.as_ref().unwrap().cost;
    let old_cost = memo.journal.as_ref().unwrap().uncompressed_cost(&j);
    assert!(old_cost > compact_cost);
    memo.set_test_ceiling(compact_cost * 8);
    j.verify_cached(current, &context, &mut memo).unwrap();
    assert_eq!(memo.journal_replayed, 0);
    memo.set_test_ceiling((compact_cost - 1) * 8);
    j.verify_cached(current, &context, &mut memo).unwrap();
    assert_eq!(memo.journal_replayed, j.records().len());
    assert!(memo.journal.is_none());
    memo.set_test_ceiling(usize::try_from(budget().max_total_allocation_bytes).unwrap());
    j.verify_cached(current, &context, &mut memo).unwrap();
    let before = std::time::Instant::now();
    let cached = j.verify_cached(current, &context, &mut memo).unwrap();
    assert_eq!(memo.journal_replayed, 0);
    let intent_record = j
        .records()
        .iter()
        .rev()
        .find(|r| r.value()["kind"] == "CheckpointIntent")
        .unwrap()
        .value();
    let receipt_record = j
        .records()
        .iter()
        .rev()
        .find(|r| r.value()["kind"] == "CheckpointReceipt")
        .unwrap()
        .value();
    let mut retry = backend.persisted_plan(
        intent_record["body"]["operation_receipt"]["operation_id"]
            .as_str()
            .unwrap(),
    );
    let original_id =
        package::PackageUuid::parse(intent_record["record_id"].as_str().unwrap()).unwrap();
    let receipt_id = JournalRecordIdentity {
        record_id: package::PackageUuid::parse(receipt_record["record_id"].as_str().unwrap())
            .unwrap(),
        session_generation: receipt_record["session_generation"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    };
    assert!(
        cached
            .prepare_receipt(&retry, original_id, receipt_id, limits())
            .unwrap()
            .existing()
    );
    retry.original["extensions"] = json!({"changed":true});
    assert!(
        cached
            .prepare_receipt(&retry, original_id, receipt_id, limits())
            .is_err()
    );
    // Exercise the exact shortcut used by native Host::original after journal
    // verification, not merely another Journal scan. No historical recursion.
    let work = (memo.compilations, memo.reader_work());
    let native_original = memo.current_original(current, &context).unwrap().unwrap();
    assert_eq!((memo.compilations, memo.reader_work()), work);
    let full_original = backend.original();
    assert_eq!(
        native_original.shared_package().head,
        full_original.shared_package().head
    );
    assert_eq!(native_original.proof.global, full_original.proof.global);
    assert_eq!(
        native_original.completed_original,
        full_original.completed_original
    );
    assert!(native_original.completed_original.is_some());
    assert_eq!(
        native_original.proof.roles["active"].state,
        full_original.proof.roles["active"].state
    );
    let pool = super::super::layout::original_objects(current, &planner, budget()).unwrap();
    let resource_ref = &native_original.proof.roles["active"].state["resource_state"];
    let resource =
        super::super::restart::original_json(&native_original, resource_ref, frames(), &reader)
            .unwrap();
    assert_eq!(
        resource,
        pool[&super::super::wire::key(resource_ref).unwrap()]
    );
    let sources = super::super::restart::original_json(
        &native_original,
        &resource["retention_sources"],
        frames(),
        &reader,
    )
    .unwrap();
    assert_eq!(
        sources,
        pool[&super::super::wire::key(&resource["retention_sources"]).unwrap()]
    );
    let mut different_observation = snapshot(&backend.mock.base, current);
    different_observation
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .charge += 4096;
    assert!(
        memo.current_original(
            current,
            &backend.context(&different_observation, &identity, &reader)
        )
        .unwrap()
        .is_none()
    );
    let mut narrower = context;
    narrower.max_originals = 1;
    assert!(memo.current_original(current, &narrower).unwrap().is_none());
    let context = backend.context(&observed, &identity, &reader);
    let expected_view = expected.verify_session(&base, &planner, budget()).unwrap();
    cached
        .verify_session_cached(&base, &planner, budget(), &mut memo)
        .unwrap();
    assert_eq!(memo.session_replayed, j.records().len());
    let actual = cached
        .verify_session_cached(&base, &planner, budget(), &mut memo)
        .unwrap();
    assert_eq!(memo.session_replayed, 0);
    assert!(
        backend.memo.borrow().session_record_work <= j.records().len() + 4,
        "whole submit/flush path extends the local prefix instead of replaying its history"
    );
    assert_eq!(actual.coordinate(), expected_view.coordinate());
    assert_eq!(actual.accepted_frame(), expected_view.accepted_frame());
    assert_eq!(
        actual.undo_targets(&binding().owner),
        expected_view.undo_targets(&binding().owner)
    );
    assert_eq!(
        actual.graph_objects().unwrap(),
        expected_view.graph_objects().unwrap()
    );
    let barrier = actual
        .prepare_barrier(&binding().owner, "save-now", rid(132_001), limits())
        .unwrap();
    let mut extended = backend.bytes.clone();
    extended.extend(barrier.bytes());
    let appended = Journal::read(&extended, &backend.header, limits()).unwrap();
    let appended_proof = appended
        .verify_cached(current, &context, &mut memo)
        .unwrap();
    let appended_view = appended_proof
        .verify_session_cached(&base, &planner, budget(), &mut memo)
        .unwrap();
    assert_eq!(memo.session_replayed, 1);
    assert_eq!(appended_view.coordinate(), actual.coordinate());
    let mut narrow = budget();
    narrow.semantic.json.max_array_elements = 1;
    assert!(
        appended_proof
            .verify_session_cached(&base, &planner, narrow, &mut memo)
            .is_err()
    );
    assert_eq!(
        memo.journal_replayed, 1,
        "only the newly appended record replays"
    );
    assert!(memo.retained().1 <= usize::try_from(budget().max_total_allocation_bytes).unwrap());
    let mut changed = current.clone();
    changed.allocations.values_mut().next().unwrap().bytes[16] ^= 1;
    assert!(memo.current_original(&changed, &context).unwrap().is_none());
    let mut extra = current.clone();
    extra.allocations.values_mut().next().unwrap().bytes.push(0);
    assert!(memo.current_original(&extra, &context).unwrap().is_none());
    assert!(
        appended
            .verify_cached(&changed, &context, &mut memo)
            .is_err()
    );
    eprintln!(
        "four-operation journal: cold_records={cold_records} warm_records=0 appended_records=1 elapsed={:?} retained_bytes={}",
        before.elapsed(),
        memo.retained().1
    );
}

#[test]
fn original_pool_dedup_preserves_physical_budgets_and_malformed_refusal() {
    let original = verify(&fixture());
    let inputs = attempt(&original, 1000).planner;
    let mut duplicate = original.shared_package().clone();
    let ids = duplicate.allocations.keys().cloned().collect::<Vec<_>>();
    let copied = duplicate.allocations[&ids[0]].bytes.clone();
    duplicate.allocations.get_mut(&ids[1]).unwrap().bytes = copied;
    // This intentionally unselected layout exercises only the internal decoder;
    // no reader authority or package acceptance is inferred from its result.
    super::super::layout::Builder::load(&duplicate, &inputs, budget()).unwrap();
    let mut physical_json = duplicate.loose.values().map(Vec::len).sum::<usize>();
    for a in duplicate.allocations.values() {
        let mut at = 0;
        while at < a.bytes.len() {
            let length = u32::from_le_bytes(a.bytes[at + 12..at + 16].try_into().unwrap()) as usize;
            if a.bytes[at + 8] != 0 {
                physical_json += length;
            }
            at += 16 + length;
        }
    }
    let mut narrow = budget();
    narrow.semantic.max_total_json_bytes = physical_json - 1;
    assert!(super::super::layout::Builder::load(&duplicate, &inputs, narrow).is_err());
    let raw = &mut duplicate.allocations.get_mut(&ids[0]).unwrap().bytes;
    let mut at = 0;
    while raw[at + 8] == 0 {
        at += 16 + u32::from_le_bytes(raw[at + 12..at + 16].try_into().unwrap()) as usize;
    }
    raw[at + 16] = b' '; // Both repeated bodies are now invalid canonical JSON.
    let malformed = raw.clone();
    duplicate.allocations.get_mut(&ids[1]).unwrap().bytes = malformed;
    assert!(super::super::layout::Builder::load(&duplicate, &inputs, budget()).is_err());
}

#[path = "incremental_tests.rs"]
mod incremental_tests;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One exact clean candidate/cache proof with changed observation refusals"
)]
fn clean_executor_proof_reused_by_receipt_with_fresh_observations() {
    let base = fixture();
    let original = verify(&base);
    let (intent, receipt) = request();
    let plan = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    super::super::shared::take_copied();
    let current = selected(&plan, 6);
    assert_eq!(super::super::shared::take_copied(), 0);
    super::super::restart::check_compact_ownership(&plan, budget());
    let public: &BTreeMap<String, Vec<u8>> = plan.target_allocations();
    assert!(std::ptr::eq(public, plan.target_allocations()));
    let mut external = public.clone();
    external.values_mut().next().unwrap()[0] ^= 1;
    assert_ne!(
        external.values().next().unwrap().as_slice(),
        plan.files.values().next().unwrap().as_slice()
    );

    let observed = snapshot(&base, &current);
    let identity = identity(&current);
    let reader = support::limits();
    let context = super::journal_tests::context(&observed, &identity, &reader);
    v1_3::tree::take_encoded_logical_bytes();
    let mut memo = super::super::restart::CompiledPlans::default();
    crate::package::v1_3::locator::take_full_audits();
    memo.inspect_shared_candidate(&current, &context, &plan, 6)
        .unwrap();
    assert_eq!(
        crate::package::v1_3::locator::take_full_audits(),
        original.proof.roles.len(),
        "one full locator audit per role, shared by semantic/retention/physical proofs"
    );
    assert_eq!(
        v1_3::tree::take_encoded_logical_bytes(),
        0,
        "native candidate uses authenticated canonical frame bodies"
    );
    let completed = memo.completed_current(&current, &context).unwrap().unwrap();
    assert_eq!(
        completed.completed_original,
        Some(serde_json::from_value(reference(plan.original())).unwrap())
    );
    let mut changed = current.clone();
    changed.head["commit_sha256"] = json!("00".repeat(32));
    assert!(
        memo.completed_current(&changed, &context)
            .unwrap()
            .is_none()
    );
    let mut changed = current.clone();
    changed.loose.values_mut().next().unwrap().push(b' ');
    assert!(
        memo.completed_current(&changed, &context)
            .unwrap()
            .is_none()
    );
    let before = memo.reader_work();
    let proved = super::super::restart::verify_completed_plan(
        &current,
        &plan,
        &context,
        Some(&mut memo),
        None,
    )
    .unwrap();
    assert_eq!(memo.reader_work(), (before.0, before.1 + 1));
    assert_eq!(
        proved.completed_original,
        Some(serde_json::from_value(reference(plan.original())).unwrap())
    );
    assert_eq!(proved.shared_package().head, current.head);
    let mut changed = current.clone();
    changed.allocations.values_mut().next().unwrap().bytes[16] ^= 1;
    assert!(
        super::super::restart::verify_completed_plan(
            &changed,
            &plan,
            &context,
            Some(&mut memo),
            None
        )
        .is_err()
    );
    let mut changed_observation = snapshot(&base, &current);
    changed_observation
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .inode += 1;
    let altered = super::journal_tests::context(&changed_observation, &identity, &reader);
    assert!(
        memo.completed_current(&current, &altered)
            .unwrap()
            .is_none()
    );
    assert!(
        super::super::restart::verify_completed_plan(
            &current,
            &plan,
            &altered,
            Some(&mut memo),
            None
        )
        .is_err()
    );
    let mut narrowed = reader;
    narrowed.max_roles = 0;
    let limited = super::journal_tests::context(&observed, &identity, &narrowed);
    assert!(
        memo.completed_current(&current, &limited)
            .unwrap()
            .is_none()
    );
    assert!(
        super::super::restart::verify_completed_plan(
            &current,
            &plan,
            &limited,
            Some(&mut memo),
            None
        )
        .is_err()
    );
    #[cfg(target_os = "macos")]
    super::super::disposable::check_borrowed_snapshot(base.data.clone(), &current);
}

#[test]
fn shared_allocation_views_copy_only_on_mutation_and_preserve_public_dtos() {
    use super::super::shared::{Bytes, take_copied};
    let original: Bytes = (0_u8..64).collect::<Vec<_>>().into();
    take_copied();
    let mut prefix = original.prefix(8).unwrap();
    let retained = prefix.clone();
    assert_eq!(take_copied(), 0);
    assert!(prefix.prefix(9).is_none());
    prefix[0] = 99;
    assert_eq!(take_copied(), 8);
    assert_eq!(&retained[..], &(0_u8..8).collect::<Vec<_>>());
    assert_eq!(original[0], 0);
    prefix.extend_from_slice(&[100, 101]);
    prefix.resize(16, 200);
    assert_eq!(&prefix[8..10], &[100, 101]);
    assert_eq!(&prefix[10..], &[200; 6], "hidden suffix never reappears");
    assert_eq!(take_copied(), 0);
    let base = fixture();
    let source = package(&base);
    let total = source
        .allocations
        .values()
        .map(|a| a.bytes.len())
        .sum::<usize>();
    take_copied();
    let shared_clone = source.clone();
    assert_eq!(
        take_copied(),
        0,
        "whole-package clone shares actual allocation bytes"
    );
    let verified = v1_3::reader::verify_shared_original(
        shared_clone,
        identity(&source),
        &base,
        &base.registration,
        v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        frames(),
        &support::limits(),
    )
    .unwrap();
    assert_eq!(
        take_copied(),
        0,
        "reader does not materialize compatibility DTO"
    );
    let public: &super::super::OriginalPackage = verified.package();
    assert_eq!(take_copied(), total);
    assert!(std::ptr::eq(public, verified.package()));
    assert_eq!(take_copied(), 0);
    let mut mutable_public = public.clone();
    mutable_public
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .bytes[0] ^= 1;
    assert_ne!(
        mutable_public.allocations.values().next().unwrap().bytes,
        verified
            .shared_package()
            .allocations
            .values()
            .next()
            .unwrap()
            .bytes
    );
    assert!(
        v1_3::verify_original(
            mutable_public,
            identity(&source),
            &base,
            &base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900
            },
            frames(),
            &support::limits()
        )
        .is_err()
    );
    assert_eq!(
        source.allocations.values().next().unwrap().bytes,
        verified
            .shared_package()
            .allocations
            .values()
            .next()
            .unwrap()
            .bytes
    );
}

#[test]
fn hypothetical_journal_reuses_only_exact_canonical_prefix_and_limits() {
    let mut session = open();
    let (intent, receipt) = edit(&session, 160_001, 90);
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(161_001), 0)
        .unwrap();
    let flush = session.begin_flush("save-now", rid(161_002)).unwrap();
    session.finish_flush(&flush).unwrap();
    let backend = session.io_mut();
    let parsed = backend.parsed();
    let checkpoint = parsed
        .records()
        .iter()
        .find(|r| r.value()["kind"] == "CheckpointIntent")
        .unwrap();
    let end = parsed.record_end(&checkpoint.value()["record_id"]).unwrap();
    let prefix = &backend.bytes[..end];
    let same = |a: &Journal, b: &Journal| {
        assert_eq!(a.header(), b.header());
        assert_eq!(a.tail(), b.tail());
        assert_eq!(a.verified_bytes(), b.verified_bytes());
        assert_eq!(
            a.records()
                .iter()
                .map(|r| (r.value(), r.checksum()))
                .collect::<Vec<_>>(),
            b.records()
                .iter()
                .map(|r| (r.value(), r.checksum()))
                .collect::<Vec<_>>()
        );
    };
    let ordinary = Journal::read(prefix, &backend.header, limits()).unwrap();
    journal::take_decoded_bytes();
    let reused = Journal::read_reusing(prefix, &backend.header, limits(), &parsed).unwrap();
    assert_eq!(journal::take_decoded_bytes(), encode(&backend.header).len());
    same(&ordinary, &reused);
    let extended =
        Journal::read_reusing(&backend.bytes, &backend.header, limits(), &reused).unwrap();
    same(&parsed, &extended);
    let mut bad = backend.bytes.clone();
    bad[end - 1] ^= 1;
    assert!(Journal::read_reusing(&bad, &backend.header, limits(), &parsed).is_err());
    let mut header = backend.header.clone();
    header["project_id"] = json!(uid(160_002));
    assert!(Journal::read_reusing(&backend.bytes, &header, limits(), &parsed).is_err());
    for narrow in [
        journal::JournalLimits {
            max_work_bytes: 1,
            ..limits()
        },
        journal::JournalLimits {
            max_records: 0,
            ..limits()
        },
        journal::JournalLimits {
            json: package::JsonLimits {
                max_depth: 1,
                ..limits().json
            },
            ..limits()
        },
    ] {
        assert!(Journal::read_reusing(&backend.bytes, &backend.header, narrow, &parsed).is_err());
    }
    for cut in [end + 1, backend.bytes.len() - 1] {
        let a = Journal::read(&backend.bytes[..cut], &backend.header, limits()).unwrap();
        let b = Journal::read_reusing(&backend.bytes[..cut], &backend.header, limits(), &parsed)
            .unwrap();
        same(&a, &b);
        assert_eq!(b.tail(), journal::JournalTail::Incomplete);
        let retried = Journal::read_reusing(&backend.bytes, &backend.header, limits(), &b).unwrap();
        same(&parsed, &retried);
        // Torn bytes are not charged as canonical payload; every complete
        // header/record is charged exactly as in ordinary decoding.
        let work = encode(b.header()).len()
            + b.records()
                .iter()
                .map(|r| encode(r.value()).len())
                .sum::<usize>();
        let exact = journal::JournalLimits {
            max_work_bytes: work,
            ..limits()
        };
        assert!(Journal::read_reusing(&backend.bytes[..cut], &backend.header, exact, &b).is_ok());
        let short = journal::JournalLimits {
            max_work_bytes: work - 1,
            ..limits()
        };
        assert!(Journal::read_reusing(&backend.bytes[..cut], &backend.header, short, &b).is_err());
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Existing history corpus covers requested lookup and corruption without a second fixture"
)]
fn session_checkpoint_history_requires_drained_exact_mutation_evidence() {
    use super::super::checkpoint::{CheckpointIdentity, PreparedHistory};
    let reader = support::limits();
    let mut session = open();
    let (intent, receipt) = edit(&session, 510_001, 35);
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(510_002), 0)
        .unwrap();
    let inspect = |io: &Backend, drained: bool, retired: bool| {
        let current = io.original();
        let base = verify(&io.mock.base);
        let journal = io.parsed();
        let verified = io.verify(&journal).unwrap();
        let view = verified
            .verify_session(&base, &attempt(&base, 1000).planner, budget())
            .unwrap();
        let identity = CheckpointIdentity {
            package: identity(&current.package),
            device: package::PackageUuid::parse(io.header["device_id"].as_str().unwrap()).unwrap(),
            incarnation: binding().incarnation_id,
        };
        let result = PreparedHistory::prepare(
            &view,
            &current,
            &identity,
            package::PackageUuid::parse(&uid(510_003)).unwrap(),
            (2, 2),
            &support::limits(),
        );
        if !drained {
            assert!(result.is_err());
            return;
        }
        let history = result.unwrap();
        let (_, undo) = view.checkpoint_source(&current).unwrap();
        assert_eq!(undo.is_empty(), retired);
        assert_eq!(
            view.through(),
            Some(view.checkpoint_target()["through"].clone())
        );
        assert_eq!(
            view.accepted_frame(),
            Some(view.checkpoint_target()["accepted_frame"].clone())
        );
        let entry = history
            .lookup_objects()
            .values()
            .filter_map(|v| v["entries"].as_array())
            .flatten()
            .find(|v| v["operation_id"] == intent["operation_id"])
            .unwrap();
        assert_eq!(entry["receipt"], reference(&receipt));
        assert_eq!(
            entry["local"]["owner_sha256"],
            hash(&encode(&binding().owner))
        );
        assert_eq!(
            entry["local"]["action_sha256"],
            hash(&encode(&json!({"kind":"edit"})))
        );
        let archived = &entry["local"]["mutation"];
        let offset = archived["offset"]
            .as_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let length = archived["byte_length"]
            .as_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let payload = u32::from_le_bytes(io.bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(length, payload + 36);
        assert_eq!(
            io.bytes[offset + 4 + payload..offset + length].iter().fold(
                String::new(),
                |mut text, byte| {
                    use std::fmt::Write;
                    write!(&mut text, "{byte:02x}").unwrap();
                    text
                }
            ),
            archived["checksum"].as_str().unwrap()
        );
        let record: Value =
            serde_json::from_slice(&io.bytes[offset + 4..offset + 4 + payload]).unwrap();
        assert_eq!(archived["record"], reference(&record));
        assert_eq!(record["body"]["semantic_intent"], intent);
        let mut objects = history.lookup_objects().clone();
        objects.insert(
            super::super::wire::key(&reference(history.archive())).unwrap(),
            history.archive().clone(),
        );
        let mut local = history.archive().clone();
        for field in ["archive_id", "previous", "segments"] {
            local.as_object_mut().unwrap().remove(field);
        }
        local["schema"]["id"] = json!("photara.session.checkpoint-base");
        local["checkpoint"] = reference(&receipt);
        local["package_head"] = current.package.head.clone();
        local["package_commit"] = reference(&current.package.commit);
        local["journal_id"] = journal.header()["journal_id"].clone();
        local["through"] = json!({"record_id":record["record_id"],"checksum":archived["checksum"],"sequence":record["sequence"]});
        local["mutation"] = Value::Null;
        local["accepted_frame"] = Value::Null;
        local["coordinate"] = view.coordinate();
        local["undo_roots"] = json!([]);
        local["lookup"] = history.lookup().clone();
        local["archive"] = reference(history.archive());
        let local =
            super::super::checkpoint::LocalBase::parse(&encode(&local), &identity, reader.json)
                .unwrap();
        let source = Archived {
            objects,
            stream: journal.header()["stream_id"].clone(),
            bytes: io.bytes.clone(),
            reads: std::sync::atomic::AtomicUsize::new(0),
        };
        let operation =
            package::PackageUuid::parse(intent["operation_id"].as_str().unwrap()).unwrap();
        assert_eq!(
            local
                .lookup_mutation(&source, &identity, operation, &reader)
                .unwrap(),
            Some(record.clone())
        );
        assert!(
            source.reads.load(std::sync::atomic::Ordering::Relaxed) <= reader.tree.max_depth + 2
        );
        let missing = package::PackageUuid::parse(&uid(599_999)).unwrap();
        assert!(
            local
                .lookup_operation(&source, &identity, missing, &reader)
                .unwrap()
                .is_none()
        );
        let mut damaged = io.bytes.clone();
        damaged[offset + 4] ^= 1;
        let damaged = Archived {
            objects: source.objects.clone(),
            stream: source.stream.clone(),
            bytes: damaged,
            reads: std::sync::atomic::AtomicUsize::new(0),
        };
        assert!(
            local
                .lookup_mutation(&damaged, &identity, operation, &reader)
                .is_err()
        );
        let mut tight = reader;
        tight.tree.max_pages = 0;
        assert!(
            local
                .lookup_mutation(&source, &identity, operation, &tight)
                .is_err()
        );
    };
    inspect(session.io_mut(), false, true);
    let target = session.begin_flush("save-now", rid(510_004)).unwrap();
    session.finish_flush(&target).unwrap();
    inspect(session.io_mut(), true, true);
    let mut command = request().0["command"].clone();
    let node = command["envelope"]["command"]["node_id"].clone();
    // A batch is not a trivial standalone position original. Conservatively
    // retain its entire dependency-linked chain, including the preceding move.
    command["envelope"]["command"] = json!({"kind":"batch","commands":[
        {"kind":"set-node-position","node_id":node,"x":40,"y":41},
        {"kind":"set-node-position","node_id":node,"x":42,"y":43}
    ]});
    let (intent, receipt) = session
        .prepare_graph(
            command,
            package::PackageUuid::parse(&uid(510_005)).unwrap(),
            "2026-09-27T00:00:00.000Z",
            &request().1["provenance"],
            &json!({"kind":"edit"}),
        )
        .unwrap();
    session
        .submit(&intent, &receipt, &json!({"kind":"edit"}), rid(510_006), 0)
        .unwrap();
    let target = session.begin_flush("save-now", rid(510_007)).unwrap();
    session.finish_flush(&target).unwrap();
    inspect(session.io_mut(), true, false);
}

#[expect(
    clippy::too_many_lines,
    reason = "One real turnover verifies append, stages, bounds and exact retry"
)]
fn checkpoint_turnover_actual_append_and_four_selections(sizing: bool) {
    use super::super::checkpoint::{
        CheckpointIdentity, LocalSelection, PreparedHistory, TurnoverIds, TurnoverPlan,
    };
    use super::super::journal::Journal;
    let fixture = fixture();
    let old = verify(&fixture);
    let mut header = super::journal_tests::header(&old.package);
    header["journal_id"] = request().1["journal_id"].clone();
    let limits = super::journal_tests::limits();
    let reader = support::limits();
    let id = CheckpointIdentity {
        package: identity(&old.package),
        device: package::PackageUuid::parse(header["device_id"].as_str().unwrap()).unwrap(),
        incarnation: fixture.registration.incarnation,
    };
    let context = super::journal_tests::context(&fixture, &id.package, &reader);
    let inputs = attempt(&old, 610_000).planner;
    let mut bytes = Journal::create(&header, limits).unwrap();
    let journal = Journal::read(&bytes, &header, limits).unwrap();
    let verified = journal.verify_shared(&old.package, &context).unwrap();
    let view = verified.verify_session(&old, &inputs, budget()).unwrap();
    let owner = json!({"attachment_id":uid(610_001),"attachment_generation":"1","principal":request().1["provenance"]["principal"]});
    bytes.extend(
        view.prepare_barrier(
            &owner,
            "close",
            super::journal_tests::record(610_002),
            limits,
        )
        .unwrap()
        .bytes(),
    );
    let journal = Journal::read(&bytes, &header, limits).unwrap();
    let verified = journal.verify_shared(&old.package, &context).unwrap();
    let view = verified.verify_session(&old, &inputs, budget()).unwrap();
    let history = PreparedHistory::prepare(
        &view,
        &old,
        &id,
        package::PackageUuid::parse(&uid(610_003)).unwrap(),
        (2, 2),
        &reader,
    )
    .unwrap();
    let mut selector = history.archive().clone();
    selector.as_object_mut().unwrap().remove("archive_id");
    selector.as_object_mut().unwrap().remove("previous");
    let segment = selector
        .as_object_mut()
        .unwrap()
        .remove("segments")
        .unwrap()[0]
        .clone();
    selector["schema"]["id"] = json!("photara.session.checkpoint-selection");
    selector["generation"] = json!("1");
    selector["previous_sha256"] = Value::Null;
    selector["selected"] = json!({"format":"PHPSJ001","stream_id":segment["stream_id"],"header_sha256":segment["header_sha256"],"base":null});
    selector["pending"] = Value::Null;
    let selector = LocalSelection::parse(&encode(&selector), &id, reader.json).unwrap();
    let mut attempt = attempt(&old, 620_000);
    attempt.selectors.truncate(4);
    let mut raw = serde_json::to_value(attempt).unwrap();
    raw["planner"]["codec"] = json!("photara.codec.ps2-checkpoint-turnover-layout-v1");
    for corridor in raw["planner"]["corridors"].as_array_mut().unwrap() {
        corridor.as_object_mut().unwrap().remove("original_sha256");
        corridor["base_head_sha256"] = json!(hash(&encode(&old.package.head)));
    }
    let ids = || TurnoverIds {
        maintenance_id: uid(630_000),
        checkpoint_id: uid(630_001),
        next_stream_id: uid(630_002),
    };
    let mut bounded = budget();
    bounded.max_allocation_growth = 262_143;
    assert!(TurnoverPlan::compile(&history, &selector, ids(), raw.clone(), bounded).is_err());
    for bound in ["project_limit", "aggregate_control_bytes"] {
        let mut too_small = raw.clone();
        too_small[bound] = json!("1");
        assert!(TurnoverPlan::compile(&history, &selector, ids(), too_small, budget()).is_err());
    }
    let (plan, mut raw) = sized_compile(
        sizing,
        raw,
        |raw| TurnoverPlan::compile_inner(&history, &selector, ids(), raw.clone(), budget()),
        |plan| &plan.files,
    )
    .unwrap();
    let appended = |files: &BTreeMap<String, super::super::shared::Bytes>,
                    old: &OriginalPackage| {
        files
            .iter()
            .map(|(id, bytes)| bytes.len() - old.allocations[id].bytes.len())
            .sum::<usize>()
    };
    let turnover_bytes = appended(&plan.files, &plan.old);
    assert!(turnover_bytes <= 7 * 262_144);
    if sizing {
        assert!(turnover_bytes < 7 * 262_144);
    }
    eprintln!(
        "sized turnover append={turnover_bytes} versus fixed={}",
        7 * 262_144
    );
    assert_eq!(plan.stages().len(), 4);
    assert_eq!(plan.original()["attempt"], raw);
    let again =
        TurnoverPlan::compile_inner(&history, &selector, ids(), raw.clone(), budget()).unwrap();
    assert_eq!(encode(plan.original()), encode(again.original()));
    assert_eq!(plan.files, again.files);
    let restored = TurnoverPlan::restore(
        &history,
        &selector,
        &encode(plan.original()),
        &encode(plan.acceptance_base()),
        budget(),
    )
    .unwrap();
    assert_eq!(restored.files, plan.files);
    let mut changed = plan.original().clone();
    changed["writes"][0]["suffix_sha256"] = json!("00".repeat(32));
    assert!(
        TurnoverPlan::restore(
            &history,
            &selector,
            &encode(&changed),
            &encode(plan.acceptance_base()),
            budget()
        )
        .is_err()
    );
    for (stage, selected) in plan.stages().iter().enumerate() {
        assert_eq!(selected.head, again.stages()[stage].head);
        assert_eq!(selected.commit, again.stages()[stage].commit);
        assert_eq!(selected.loose, again.stages()[stage].loose);
        if stage < 2 {
            continue;
        }
        let mut package = old.package.clone();
        package.head = selected.head.clone();
        package.commit = selected.commit.clone();
        package.loose = selected.loose.clone();
        for (allocation, bytes) in &plan.files {
            assert!(bytes.starts_with(&old.package.allocations[allocation].bytes));
            package.allocations.get_mut(allocation).unwrap().bytes = bytes.clone();
        }
        let observed = snapshot(&fixture, &package);
        let mut changed = package.clone();
        changed.allocations.get_mut(&aid(2)).unwrap().bytes[0] ^= 1;
        assert!(
            plan.verify_candidate(
                changed.public(),
                stage,
                identity(&package),
                &observed,
                &fixture.registration,
                v1_3::DirectoryObservation {
                    device: 7,
                    inode: 900
                },
                frames(),
                &reader
            )
            .is_err()
        );
        let clean = plan
            .verify_candidate(
                package.public(),
                stage,
                identity(&package),
                &observed,
                &fixture.registration,
                v1_3::DirectoryObservation {
                    device: 7,
                    inode: 900,
                },
                frames(),
                &reader,
            )
            .unwrap();
        if stage == 3 {
            let clean = clean.unwrap();
            let base = v1_3::snapshot::AcceptanceBase::parse(
                &encode(plan.acceptance_base()),
                &id.package,
                reader.json,
            )
            .unwrap();
            let local = history
                .bind_package(&base, &clean, &id, frames(), &reader, budget())
                .unwrap();
            assert_eq!(local.value()["coordinate"], view.coordinate());
            let activation_limits = || ActivationLimits {
                journal: limits,
                frames: frames(),
                reader: &reader,
                planner: budget(),
            };
            let held = lease(&old.package);
            let mut expected = None;
            for cut in 1..=5 {
                let mut io = CheckpointMemory {
                    fixture: &fixture,
                    package: clean.package.clone(),
                    selector: encode(selector.value()),
                    original: None,
                    installed: None,
                    fail_after: Some(cut),
                    effects: 0,
                };
                assert!(
                    activate(
                        &mut io,
                        &held,
                        &plan,
                        &history,
                        &selector,
                        &id,
                        activation_limits()
                    )
                    .is_err()
                );
                let active = activate(
                    &mut io,
                    &held,
                    &plan,
                    &history,
                    &selector,
                    &id,
                    activation_limits(),
                )
                .unwrap();
                assert_eq!(active.base().value(), local.value());
                let journal = Journal::read(active.segment(), active.header(), limits).unwrap();
                assert_eq!(journal.records().len(), 1);
                assert_eq!(
                    journal.records()[0].value()["kind"],
                    "CheckpointBaseReceipt"
                );
                assert_eq!(journal.records()[0].value()["sequence"], "2");
                assert_eq!(active.header()["base"], reference(local.value()));
                if cut == 5 {
                    let seeded = active
                        .verify_session(&journal, frames(), &reader, budget())
                        .unwrap();
                    let source = LocalRecords(
                        [plan.acceptance_base(), plan.original(), selector.value()]
                            .into_iter()
                            .map(|value| (wire::key(&reference(value)).unwrap(), value.clone()))
                            .collect(),
                    );
                    saved2_proof(active.activation_checkpoint(&journal, &json!({"record_id":journal.records()[0].value()["record_id"],"record_checksum":journal.records()[0].checksum()}), &source, &reader).unwrap(),
                        json!({"project_id":id.package.project,"incarnation_id":id.incarnation,"owner_epoch":uid(799_002),"owner":owner}),
                        json!({"coordinate":seeded.coordinate(),"mutation":seeded.through(),"accepted_frame":seeded.accepted_frame()}), !sizing);
                    assert_eq!(seeded.coordinate(), view.coordinate());
                    assert_eq!(
                        seeded.next_receipt_coordinates().unwrap(),
                        view.next_receipt_coordinates().unwrap()
                    );
                    let (mut intent, mut receipt) = request();
                    receipt["journal_sequence"] =
                        json!(seeded.next_receipt_coordinates().unwrap().1.to_string());
                    let make_raw = |original: &VerifiedOriginal, n| {
                        let mut raw = serde_json::to_value(super::attempt(original, n)).unwrap();
                        raw["planner"]["codec"] = json!("photara.codec.ps2-repeatable-layout-v4");
                        for corridor in raw["planner"]["corridors"].as_array_mut().unwrap() {
                            corridor.as_object_mut().unwrap().remove("original_sha256");
                            corridor["base_head_sha256"] =
                                json!(hash(&encode(&original.package.head)));
                        }
                        raw
                    };
                    let (first, first_raw) = sized_compile(
                        sizing,
                        make_raw(active.package(), 650_000),
                        |raw| {
                            active.compile_operation_inner(
                                active.package(),
                                &intent,
                                &receipt,
                                raw,
                                budget(),
                            )
                        },
                        |plan| &plan.files,
                    )
                    .unwrap();
                    let first_bytes = appended(&first.files, &first.old);
                    if sizing {
                        assert!(first_bytes < 7 * 262_144 / 2);
                    }
                    eprintln!("sized first move append={first_bytes}");
                    assert_eq!(
                        first.original()["original_codec"],
                        "photara.codec.ps2-repeatable-admission-v4"
                    );
                    assert_eq!(
                        first.original()["payload_recipe"]["checkpoint_base"],
                        reference(local.value())
                    );
                    assert_eq!(
                        active
                            .restore_operation(
                                active.package(),
                                &intent,
                                &receipt,
                                &first.original_bytes(),
                                budget()
                            )
                            .unwrap()
                            .original_bytes(),
                        first.original_bytes()
                    );
                    for range in first.original()["payload_recipe"]["allocations"]
                        .as_array()
                        .unwrap()
                    {
                        assert!(range.get("original_sha256").is_none());
                        assert_eq!(
                            range["base_head_sha256"],
                            hash(&encode(&active.package().package.head))
                        );
                    }
                    for keep_new in [false, true] {
                        let mut wrong = first.original().clone();
                        let range = wrong["payload_recipe"]["allocations"][0]
                            .as_object_mut()
                            .unwrap();
                        range.insert("original_sha256".into(), json!("00".repeat(32)));
                        if !keep_new {
                            range.remove("base_head_sha256");
                        }
                        assert!(
                            active
                                .restore_operation(
                                    active.package(),
                                    &intent,
                                    &receipt,
                                    &encode(&wrong),
                                    budget()
                                )
                                .is_err()
                        );
                    }
                    // The native v4 hot path keeps only compact deterministic
                    // suffix output in the existing bounded plan memo. Fresh
                    // package/custody context is still required on every reuse.
                    let mut memo = restart::CompiledPlans::default();
                    memo.remember_snapshot_plan("exact-current-and-selector", &first, budget(), 8)
                        .unwrap();
                    let hits = memo.hits;
                    for _ in 0..2 {
                        let reused = memo
                            .snapshot_plan(
                                "exact-current-and-selector",
                                active.package().shared_package(),
                                first.original(),
                                budget(),
                                8,
                            )
                            .unwrap()
                            .unwrap();
                        assert_eq!(reused.original_bytes(), first.original_bytes());
                        assert_eq!(reused.files, first.files);
                    }
                    assert_eq!(memo.hits, hits + 2);
                    assert_eq!(memo.compilations, 0);
                    assert!(
                        memo.snapshot_plan(
                            "changed-selector",
                            active.package().shared_package(),
                            first.original(),
                            budget(),
                            8
                        )
                        .unwrap()
                        .is_none()
                    );
                    let mut altered = active.package().shared_package().clone();
                    altered.head["commit_sha256"] = json!("00".repeat(32));
                    assert!(
                        memo.snapshot_plan(
                            "exact-current-and-selector",
                            &altered,
                            first.original(),
                            budget(),
                            8
                        )
                        .is_err()
                    );
                    let mut altered = active.package().shared_package().clone();
                    altered.loose.values_mut().next().unwrap().push(b' ');
                    assert!(
                        memo.snapshot_plan(
                            "exact-current-and-selector",
                            &altered,
                            first.original(),
                            budget(),
                            8
                        )
                        .is_err()
                    );
                    let mut changed = first.original().clone();
                    changed["nonce"] = json!("different");
                    assert!(
                        memo.snapshot_plan(
                            "exact-current-and-selector",
                            active.package().shared_package(),
                            &changed,
                            budget(),
                            8
                        )
                        .unwrap()
                        .is_none()
                    );
                    let mut smaller = budget();
                    smaller.max_objects -= 1;
                    assert!(
                        memo.snapshot_plan(
                            "exact-current-and-selector",
                            active.package().shared_package(),
                            first.original(),
                            smaller,
                            8
                        )
                        .unwrap()
                        .is_none()
                    );
                    let finalizer = first.stages()[6]
                        .loose
                        .values()
                        .map(|v| wire::parse(v).unwrap())
                        .find(|v| v["schema"]["id"] == "photara.storage.finalization-control")
                        .unwrap();
                    for write in finalizer["writes"].as_array().unwrap() {
                        assert!(write.get("original_sha256").is_none());
                        assert_eq!(
                            write["base_head_sha256"],
                            hash(&encode(&active.package().package.head))
                        );
                        let mut recipe = write.clone();
                        recipe.as_object_mut().unwrap().remove("recipe_sha256");
                        assert_eq!(write["recipe_sha256"], hash(&encode(&recipe)));
                    }
                    let prove = |plan: &RepeatablePlan| {
                        let package = super::selected(plan, 6);
                        let observed = snapshot(&fixture, &package);
                        v1_3::reader::verify_operation_original(
                            package.clone(),
                            identity(&package),
                            &observed,
                            &fixture.registration,
                            v1_3::DirectoryObservation {
                                device: 7,
                                inode: 900,
                            },
                            frames(),
                            &reader,
                            plan,
                            6,
                        )
                        .unwrap()
                    };
                    for after_publication in [false, true] {
                        let mut writer = Mock {
                            base: super::fixture(),
                            current: first.old.clone(),
                            receipts: BTreeMap::new(),
                            journals: BTreeMap::new(),
                            interrupt_append: !after_publication,
                            interrupt_select: after_publication.then(|| {
                                (
                                    first.stages()[5].commit["commit_id"]
                                        .as_str()
                                        .unwrap()
                                        .to_owned(),
                                    true,
                                )
                            }),
                            fail_stabilize: false,
                            effects: 0,
                        };
                        let interrupted = execute(&mut writer, &held, &first);
                        assert!(
                            matches!(
                                interrupted,
                                Err(ExecutionError::Effect(EffectFailure::OutcomeUnknown))
                            ),
                            "v4 cut {after_publication}: {:?}",
                            interrupted.err()
                        );
                        let retry = active
                            .restore_operation(
                                active.package(),
                                &intent,
                                &receipt,
                                &first.original_bytes(),
                                budget(),
                            )
                            .unwrap();
                        let completed = execute(&mut writer, &held, &retry).unwrap();
                        assert_eq!(completed.receipt(), &receipt);
                        assert_eq!(writer.current.head, first.stages()[6].head);
                        assert_eq!(
                            execute(&mut writer, &held, &retry).unwrap().receipt(),
                            &receipt
                        );
                        assert_eq!(writer.receipts.len(), 1);
                    }
                    let mut bad_head = first_raw.clone();
                    bad_head["planner"]["corridors"][0]["base_head_sha256"] =
                        json!("00".repeat(32));
                    assert!(
                        active
                            .compile_operation(
                                active.package(),
                                &intent,
                                &receipt,
                                &bad_head,
                                budget()
                            )
                            .is_err()
                    );
                    for field in ["witness", "extent", "suffix"] {
                        let mut candidate = super::selected(&first, 6);
                        let (allocation, bytes) = first
                            .files
                            .iter()
                            .find(|(id, bytes)| {
                                bytes.len() > first.old.allocations[*id].bytes.len()
                            })
                            .unwrap();
                        let current = candidate.allocations.get_mut(allocation).unwrap();
                        match field {
                            "witness" => current.witness["inode"] = json!("9999"),
                            "extent" => current.bytes.resize(bytes.len() - 1, 0),
                            _ => {
                                current.bytes
                                    [first.old.allocations[allocation].bytes.len() + 16] ^= 1;
                            }
                        }
                        let observed = snapshot(&fixture, &candidate);
                        assert!(
                            v1_3::reader::verify_operation_original(
                                candidate.clone(),
                                identity(&candidate),
                                &observed,
                                &fixture.registration,
                                v1_3::DirectoryObservation {
                                    device: 7,
                                    inode: 900
                                },
                                frames(),
                                &reader,
                                &first,
                                6
                            )
                            .is_err()
                        );
                    }
                    let first_current = prove(&first);
                    let append = seeded
                        .prepare_mutation(
                            &intent,
                            &receipt,
                            &owner,
                            &json!({"kind":"edit"}),
                            rid(640_001),
                            limits,
                        )
                        .unwrap();
                    let mut continued = active.segment().to_vec();
                    continued.extend_from_slice(append.bytes());
                    let suffix = Journal::read(&continued, active.header(), limits).unwrap();
                    let current = active
                        .verify_session(&suffix, frames(), &reader, budget())
                        .unwrap();
                    assert_ne!(current.coordinate(), seeded.coordinate());
                    assert!(
                        current
                            .prepare_mutation(
                                &intent,
                                &receipt,
                                &owner,
                                &json!({"kind":"edit"}),
                                rid(640_001),
                                limits
                            )
                            .unwrap()
                            .existing()
                    );
                    intent["command"]["envelope"]["command"]["x"] = json!(333);
                    assert!(
                        current
                            .prepare_mutation(
                                &intent,
                                &receipt,
                                &owner,
                                &json!({"kind":"edit"}),
                                rid(640_001),
                                limits
                            )
                            .is_err()
                    );
                    let action = json!({"kind":"undo","target":current.undo_targets(&owner)[0]});
                    let (intent, receipt) = super::journal_ps3_tests::next(
                        &current, &receipt, &owner, &action, 640_002,
                    );
                    let (second, second_raw) = sized_compile(
                        sizing,
                        make_raw(&first_current, 660_000),
                        |raw| {
                            active.compile_operation_inner(
                                &first_current,
                                &intent,
                                &receipt,
                                raw,
                                budget(),
                            )
                        },
                        |plan| &plan.files,
                    )
                    .unwrap();
                    let second_bytes = appended(&second.files, &second.old);
                    if sizing {
                        assert!(second_bytes < 7 * 262_144 / 2);
                    }
                    eprintln!("sized second move append={second_bytes}");
                    for compiled in [&first, &second] {
                        let selected = super::selected(compiled, 6);
                        let restored = super::super::recovery::reconstruct_shared_original(
                            &selected,
                            compiled.original(),
                            budget(),
                        )
                        .unwrap();
                        assert_eq!(restored.head, compiled.old.head);
                        assert_eq!(restored.commit, compiled.old.commit);
                        assert_eq!(restored.loose, compiled.old.loose);
                        for (id, allocation) in &restored.allocations {
                            assert_eq!(
                                &allocation.bytes[..],
                                &compiled.old.allocations[id].bytes[..]
                            );
                        }
                        let mut changed = compiled.original().clone();
                        changed["payload_recipe"]["allocations"][0]["base_head_sha256"] =
                            json!("0".repeat(64));
                        assert!(
                            super::super::recovery::reconstruct_shared_original(
                                &selected,
                                &changed,
                                budget()
                            )
                            .is_err()
                        );
                        changed = compiled.original().clone();
                        changed["payload_recipe"]["allocations"][0]["original_sha256"] =
                            json!("0".repeat(64));
                        assert!(
                            super::super::recovery::reconstruct_shared_original(
                                &selected,
                                &changed,
                                budget()
                            )
                            .is_err()
                        );
                    }
                    // Retry must ignore an unrequested archived receipt body,
                    // while regenerating exactly the same original and new suffix.
                    let archived = encode(old.proof.roles["active"].receipts.last().unwrap());
                    let (archive_allocation, archive_at) = old
                        .package
                        .allocations
                        .iter()
                        .find_map(|(id, a)| {
                            a.bytes
                                .windows(archived.len())
                                .enumerate()
                                .find_map(|(at, body)| {
                                    (body == archived
                                        && at >= 16
                                        && &a.bytes[at - 16..at - 8] == b"PS2PKD01"
                                        && u32::from_le_bytes(
                                            a.bytes[at - 4..at].try_into().unwrap(),
                                        ) as usize
                                            == archived.len())
                                    .then_some((id.clone(), at))
                                })
                        })
                        .unwrap();
                    let mut dirty = first_current.package.clone();
                    dirty
                        .allocations
                        .get_mut(&archive_allocation)
                        .unwrap()
                        .bytes[archive_at] ^= 1;
                    let observed = snapshot(&fixture, &dirty);
                    let dirty = v1_3::reader::verify_operation_original(
                        dirty.clone(),
                        identity(&dirty),
                        &observed,
                        &fixture.registration,
                        v1_3::DirectoryObservation {
                            device: 7,
                            inode: 900,
                        },
                        frames(),
                        &reader,
                        &first,
                        6,
                    )
                    .unwrap();
                    let retry = active
                        .restore_operation(
                            &dirty,
                            &intent,
                            &receipt,
                            &second.original_bytes(),
                            budget(),
                        )
                        .unwrap();
                    assert_eq!(retry.original_bytes(), second.original_bytes());
                    for (allocation, bytes) in &retry.files {
                        let start = dirty.package.allocations[allocation].bytes.len();
                        assert_eq!(&bytes[start..], &second.files[allocation][start..]);
                    }
                    let clean_retry = prove(&retry);
                    assert_eq!(
                        clean_retry.proof.roles["active"].state,
                        prove(&second).proof.roles["active"].state
                    );
                    let mut narrow = budget();
                    narrow.max_frames = 0;
                    assert!(
                        active
                            .restore_operation(
                                &dirty,
                                &intent,
                                &receipt,
                                &second.original_bytes(),
                                narrow
                            )
                            .is_err()
                    );
                    let second_current = prove(&second);
                    assert_eq!(
                        second_current.proof.roles["active"].state["accepted"]["through_ordinal"],
                        receipt["acceptance_ordinal"]
                    );
                    continued.extend_from_slice(
                        current
                            .prepare_mutation(
                                &intent,
                                &receipt,
                                &owner,
                                &action,
                                rid(640_003),
                                limits,
                            )
                            .unwrap()
                            .bytes(),
                    );
                    let suffix = Journal::read(&continued, active.header(), limits).unwrap();
                    let undone = active
                        .verify_session(&suffix, frames(), &reader, budget())
                        .unwrap();
                    assert_eq!(
                        undone.graph_objects().unwrap()[0]["graph"]["nodes"],
                        seeded.graph_objects().unwrap()[0]["graph"]["nodes"]
                    );
                    assert_eq!(
                        undone.graph_objects().unwrap()[0]["graph"]["connections"],
                        seeded.graph_objects().unwrap()[0]["graph"]["connections"]
                    );
                    assert_eq!(
                        undone.next_receipt_coordinates().unwrap().0,
                        seeded.next_receipt_coordinates().unwrap().0 + 2
                    );
                    assert_eq!(suffix.records()[2].value()["sequence"], "4");
                    let mut tighter = budget();
                    tighter.semantic.json.max_bytes = 1;
                    assert!(
                        active
                            .verify_session(&suffix, frames(), &reader, tighter)
                            .is_err()
                    );
                    // Cold recovery starts at each current included package,
                    // not by replaying the turnover graph through saved history.
                    let mut records = history.lookup_objects().clone();
                    for value in [
                        history.archive(),
                        plan.original(),
                        plan.acceptance_base(),
                        selector.value(),
                    ] {
                        records.insert(
                            super::super::wire::key(&reference(value)).unwrap(),
                            value.clone(),
                        );
                    }
                    let records = std::sync::Arc::new(LocalRecords(records));
                    let mut settled = active.segment().to_vec();
                    let mut recovered = None;
                    let mut first_body = None;
                    for (n, compiled) in [&first, &second].into_iter().enumerate() {
                        let checkpoint = recovered.as_ref().unwrap_or(&active);
                        let parsed = Journal::read(&settled, active.header(), limits).unwrap();
                        let session = checkpoint
                            .verify_session(&parsed, frames(), &reader, budget())
                            .unwrap();
                        let intent_id = rid(690_001 + n as u64 * 3);
                        settled.extend_from_slice(
                            session
                                .prepare_snapshot_intent(compiled, intent_id, limits)
                                .unwrap()
                                .bytes(),
                        );
                        let parsed = Journal::read(&settled, active.header(), limits).unwrap();
                        let session = checkpoint
                            .verify_session(&parsed, frames(), &reader, budget())
                            .unwrap();
                        let body = &suffix.records()[n + 1].value()["body"];
                        let action = if n == 0 {
                            body["action"].clone()
                        } else {
                            json!({"kind":"undo","target":session.undo_targets(&body["owner"])[0]})
                        };
                        let mutation = session
                            .prepare_mutation(
                                &body["semantic_intent"],
                                &body["operation_receipt"],
                                &body["owner"],
                                &action,
                                rid(640_002 + n as u64),
                                limits,
                            )
                            .unwrap();
                        if n == 0 {
                            first_body = Some(mutation.record().value()["body"].clone());
                        }
                        settled.extend_from_slice(mutation.bytes());
                        let parsed = Journal::read(&settled, active.header(), limits).unwrap();
                        let selected = prove(compiled);
                        let source = super::super::checkpoint::proof::Source {
                            selector: encode(active.selector().value()),
                            base: encode(active.base().value()),
                            header: active.header().clone(),
                            segment: settled.clone(),
                            records: records.clone(),
                        };
                        let observed = snapshot(&fixture, &selected.package);
                        let context =
                            super::journal_tests::context(&observed, &id.package, &reader);
                        let mut memo = super::super::restart::CompiledPlans::default();
                        memo.inspect_shared_candidate(&selected.package, &context, compiled, 6)
                            .unwrap();
                        let work = memo.reader_work();
                        let checked = source
                            .verify(
                                &selected.package,
                                &context,
                                &mut memo,
                                &id,
                                activation_limits(),
                            )
                            .unwrap();
                        assert_eq!(checked.package.head, selected.package.head);
                        assert!(checked.completed_original.is_some());
                        assert_eq!(memo.reader_work(), (work.0, work.1 + 1));
                        assert_eq!(memo.compilations, 0);
                        assert_eq!(checked.selected_frames, selected.selected_frames);
                        assert_eq!(checked.scope, selected.scope);
                        assert_eq!(checked.charge_unit, selected.charge_unit);
                        assert_eq!(checked.empty_hold, selected.empty_hold);
                        assert_eq!(checked.retained_controls, selected.retained_controls);
                        assert_eq!(checked.proof.global, selected.proof.global);
                        for (role, expected) in &selected.proof.roles {
                            let actual = &checked.proof.roles[role];
                            assert_eq!(actual.state, expected.state);
                            assert_eq!(actual.semantic, expected.semantic);
                            assert_eq!(actual.receipts, expected.receipts);
                        }
                        super::super::restart::check_compact_verified(&checked);
                        if n == 0 {
                            let next = active
                                .compile_operation_inner(
                                    &checked,
                                    &intent,
                                    &receipt,
                                    &second_raw,
                                    budget(),
                                )
                                .unwrap();
                            assert_eq!(next.original_bytes(), second.original_bytes());
                            assert_eq!(next.files, second.files);
                        }
                        // Old prefixes are proven against the actually grown files too.
                        let old = source
                            .verify(&compiled.old, &context, &mut memo, &id, activation_limits())
                            .unwrap();
                        assert_eq!(old.package.head, compiled.old.head);
                        let mut bounded =
                            super::journal_tests::context(&observed, &id.package, &reader);
                        bounded.frames.max_frames = 1;
                        assert!(
                            source
                                .verify(
                                    &selected.package,
                                    &bounded,
                                    &mut memo,
                                    &id,
                                    activation_limits()
                                )
                                .is_err()
                        );
                        let mut changed = compiled.old.clone();
                        changed.allocations.values_mut().next().unwrap().witness["inode"] =
                            json!("1");
                        assert!(
                            source
                                .verify(&changed, &context, &mut memo, &id, activation_limits())
                                .is_err()
                        );
                        let mut missing = super::super::checkpoint::proof::Source {
                            selector: source.selector.clone(),
                            base: source.base.clone(),
                            header: source.header.clone(),
                            segment: active.segment().to_vec(),
                            records: records.clone(),
                        };
                        assert!(
                            missing
                                .verify(
                                    &selected.package,
                                    &context,
                                    &mut memo,
                                    &id,
                                    activation_limits()
                                )
                                .is_err()
                        );
                        missing.segment = source.segment.clone();
                        missing.header["base"]["sha256"] = json!("0".repeat(64));
                        assert!(
                            missing
                                .verify(
                                    &selected.package,
                                    &context,
                                    &mut memo,
                                    &id,
                                    activation_limits()
                                )
                                .is_err()
                        );
                        let before = if n == 0 {
                            let observed = snapshot(&fixture, &clean.package);
                            plan.verify_candidate(
                                clean.package.public(),
                                3,
                                identity(&clean.package),
                                &observed,
                                &fixture.registration,
                                v1_3::DirectoryObservation {
                                    device: 7,
                                    inode: 900,
                                },
                                frames(),
                                &reader,
                            )
                            .unwrap()
                            .unwrap()
                        } else {
                            prove(&first)
                        };
                        let mut pending = Cold(Some(super::super::checkpoint::ReopenState {
                            selector: encode(active.selector().value()),
                            base: encode(active.base().value()),
                            header: active.header().clone(),
                            segment: settled.clone(),
                            current: before,
                            records: records.clone(),
                        }));
                        let pending = super::super::checkpoint::reopen(
                            &mut pending,
                            &held,
                            &id,
                            activation_limits(),
                        )
                        .unwrap();
                        let pending_view = pending
                            .verify_session(&parsed, frames(), &reader, budget())
                            .unwrap();
                        assert_eq!(pending_view.coordinate(), body["result"]);
                        assert_ne!(
                            pending_view.checkpoint_target()["coordinate"],
                            pending_view.coordinate()
                        );
                        assert!(
                            pending_view
                                .prepare_mutation(
                                    &body["semantic_intent"],
                                    &body["operation_receipt"],
                                    &body["owner"],
                                    &action,
                                    rid(640_002 + n as u64),
                                    limits
                                )
                                .unwrap()
                                .existing()
                        );
                        let receipt_identity = rid(690_003 + n as u64 * 3);
                        let expected_receipt = pending
                            .prepare_receipt(
                                &parsed,
                                &selected,
                                compiled,
                                intent_id.record_id,
                                receipt_identity,
                                limits,
                            )
                            .unwrap();
                        let advanced = |segment: &[u8]| {
                            Cold(Some(super::super::checkpoint::ReopenState {
                                selector: encode(active.selector().value()),
                                base: encode(active.base().value()),
                                header: active.header().clone(),
                                segment: segment.to_vec(),
                                current: prove(compiled),
                                records: records.clone(),
                            }))
                        };
                        if n == 1 {
                            let mut prior = advanced(&settled);
                            prior.0.as_mut().unwrap().current = prove(&first);
                            assert!(
                                super::super::checkpoint::prepare_recovery_receipt(
                                    &mut prior,
                                    &held,
                                    &id,
                                    rid(690_003),
                                    activation_limits(),
                                )
                                .unwrap()
                                .existing()
                            );
                        }
                        // Crash after clean package publication, before local receipt:
                        // ordinary open still refuses, and plain selected closure
                        // proof cannot be substituted for exact operation completion.
                        assert!(
                            super::super::checkpoint::reopen(
                                &mut advanced(&settled),
                                &held,
                                &id,
                                activation_limits(),
                            )
                            .is_err()
                        );
                        let mut generic = advanced(&settled);
                        generic.0.as_mut().unwrap().current.completed_original = None;
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut generic,
                                &held,
                                &id,
                                receipt_identity,
                                activation_limits(),
                            )
                            .is_err()
                        );
                        let mut wrong = advanced(&settled);
                        wrong.0.as_mut().unwrap().current.completed_original =
                            Some(serde_json::from_value(reference(plan.original())).unwrap());
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut wrong,
                                &held,
                                &id,
                                receipt_identity,
                                activation_limits(),
                            )
                            .is_err()
                        );
                        let intent_end = parsed.record_end(&json!(intent_id.record_id)).unwrap();
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut advanced(&settled[..intent_end]),
                                &held,
                                &id,
                                receipt_identity,
                                activation_limits(),
                            )
                            .is_err()
                        );
                        let mut narrow = activation_limits();
                        narrow.journal.max_work_bytes = 1;
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut advanced(&settled),
                                &held,
                                &id,
                                receipt_identity,
                                narrow,
                            )
                            .is_err()
                        );
                        if n == 0 {
                            let mut newer = suffix.records()[2].value().clone();
                            newer["sequence"] = json!(
                                (wire::number(
                                    &parsed.records().last().unwrap().value()["sequence"]
                                )
                                .unwrap()
                                    + 1)
                                .to_string()
                            );
                            let overshadowed = super::journal_tests::append_raw(
                                &settled,
                                &support::unhex(&parsed.records().last().unwrap().checksum()),
                                &newer,
                            );
                            assert!(Journal::read(&overshadowed, active.header(), limits).is_ok());
                            assert!(
                                super::super::checkpoint::prepare_recovery_receipt(
                                    &mut advanced(&overshadowed),
                                    &held,
                                    &id,
                                    receipt_identity,
                                    activation_limits(),
                                )
                                .is_err()
                            );
                        }
                        let repaired = super::super::checkpoint::prepare_recovery_receipt(
                            &mut advanced(&settled),
                            &held,
                            &id,
                            receipt_identity,
                            activation_limits(),
                        )
                        .unwrap();
                        assert_eq!(repaired.bytes(), expected_receipt.bytes());
                        assert_eq!(repaired.expected_extent(), settled.len());
                        settled.extend_from_slice(repaired.bytes());
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut advanced(&settled),
                                &held,
                                &id,
                                receipt_identity,
                                activation_limits(),
                            )
                            .unwrap()
                            .existing()
                        );
                        assert!(
                            super::super::checkpoint::prepare_recovery_receipt(
                                &mut advanced(&settled),
                                &held,
                                &id,
                                rid(695_000 + n as u64),
                                activation_limits(),
                            )
                            .is_err()
                        );
                        let mut cold = Cold(Some(super::super::checkpoint::ReopenState {
                            selector: encode(active.selector().value()),
                            base: encode(active.base().value()),
                            header: active.header().clone(),
                            segment: settled.clone(),
                            current: selected,
                            records: records.clone(),
                        }));
                        let opened = super::super::checkpoint::reopen(
                            &mut cold,
                            &held,
                            &id,
                            activation_limits(),
                        )
                        .unwrap();
                        let parsed = Journal::read(&settled, active.header(), limits).unwrap();
                        let opened_session = opened
                            .verify_session(&parsed, frames(), &reader, budget())
                            .unwrap();
                        saved2_proof(
                            opened
                                .activation_checkpoint(
                                    &parsed,
                                    opened_session.completed_checkpoint().unwrap(),
                                    records.as_ref(),
                                    &reader,
                                )
                                .unwrap(),
                            json!({"project_id":id.package.project,"incarnation_id":id.incarnation,"owner_epoch":uid(799_002),"owner":owner}),
                            json!({"coordinate":opened_session.coordinate(),"mutation":opened_session.through(),"accepted_frame":opened_session.accepted_frame()}),
                            !sizing,
                        );
                        assert_eq!(opened_session.coordinate(), body["result"]);
                        assert_eq!(
                            opened_session.through(),
                            Some(
                                json!({"record_id":uid(640_002+n as u64),"record_checksum":parsed.records().iter().find(|r|r.value()["record_id"]==uid(640_002+n as u64)).unwrap().checksum()})
                            )
                        );
                        assert_eq!(
                            opened_session.accepted_frame(),
                            Some(body["accepted_frame"].clone())
                        );
                        let old = first_body.as_ref().unwrap();
                        let retried = opened
                            .original_mutation(
                                &old["semantic_intent"],
                                &old["owner"],
                                &old["action"],
                                frames(),
                                &reader,
                            )
                            .unwrap()
                            .unwrap();
                        assert_eq!(retried["body"], *old);
                        let mut wrong = old["owner"].clone();
                        wrong["attachment_generation"] = json!("999");
                        assert!(
                            opened
                                .original_mutation(
                                    &old["semantic_intent"],
                                    &wrong,
                                    &old["action"],
                                    frames(),
                                    &reader
                                )
                                .is_err()
                        );
                        recovered = Some(opened);
                    }
                    let recovered = recovered.unwrap();
                    assert_eq!(
                        recovered.package().package.head,
                        second_current.package.head
                    );
                    let parsed = Journal::read(&settled, recovered.header(), limits).unwrap();
                    let session = recovered
                        .verify_session(&parsed, frames(), &reader, budget())
                        .unwrap();
                    let next_history = PreparedHistory::prepare_next(
                        &session,
                        recovered.package(),
                        &id,
                        package::PackageUuid::parse(&uid(700_003)).unwrap(),
                        (2, 2),
                        &reader,
                        recovered.base(),
                        records.as_ref(),
                    )
                    .unwrap();
                    assert_eq!(
                        next_history.archive()["previous"],
                        recovered.base().value()["archive"]
                    );
                    let mut next_attempt = super::attempt(recovered.package(), 710_000);
                    next_attempt.selectors.truncate(4);
                    let mut next_raw = serde_json::to_value(next_attempt).unwrap();
                    next_raw["planner"]["codec"] =
                        json!("photara.codec.ps2-checkpoint-turnover-layout-v1");
                    for corridor in next_raw["planner"]["corridors"].as_array_mut().unwrap() {
                        corridor.as_object_mut().unwrap().remove("original_sha256");
                        corridor["base_head_sha256"] =
                            json!(hash(&encode(&recovered.package().package.head)));
                    }
                    let next_plan = TurnoverPlan::compile_inner(
                        &next_history,
                        recovered.selector(),
                        TurnoverIds {
                            maintenance_id: uid(720_000),
                            checkpoint_id: uid(720_001),
                            next_stream_id: uid(720_002),
                        },
                        next_raw,
                        budget(),
                    )
                    .unwrap();
                    let next_restored = TurnoverPlan::restore(
                        &next_history,
                        recovered.selector(),
                        &encode(next_plan.original()),
                        &encode(next_plan.acceptance_base()),
                        budget(),
                    )
                    .unwrap();
                    assert_eq!(next_plan.files, next_restored.files);
                    let mut next_package = recovered.package().package.clone();
                    let selected = &next_plan.stages()[3];
                    next_package.head = selected.head.clone();
                    next_package.commit = selected.commit.clone();
                    next_package.loose = selected.loose.clone();
                    for (allocation, bytes) in &next_plan.files {
                        next_package.allocations.get_mut(allocation).unwrap().bytes = bytes.clone();
                    }
                    let mut next_io = CheckpointMemory {
                        fixture: &fixture,
                        package: next_package,
                        selector: encode(recovered.selector().value()),
                        original: None,
                        installed: None,
                        fail_after: None,
                        effects: 0,
                    };
                    let next_clean = next_io
                        .verify(&next_plan)
                        .expect("second turnover clean reader");
                    let next_base = v1_3::snapshot::AcceptanceBase::parse(
                        &encode(next_plan.acceptance_base()),
                        &id.package,
                        reader.json,
                    )
                    .unwrap();
                    next_history
                        .bind_package(&next_base, &next_clean, &id, frames(), &reader, budget())
                        .expect("second turnover base binding");
                    let rotated = activate(
                        &mut next_io,
                        &held,
                        &next_plan,
                        &next_history,
                        recovered.selector(),
                        &id,
                        activation_limits(),
                    )
                    .unwrap();
                    assert_eq!(rotated.base().value()["coordinate"], session.coordinate());
                    assert_eq!(rotated.base().value()["undo_roots"], json!([]));
                    // Exercise the real Session snapshot contract across this
                    // actual verified turnover, not an unchanged mock response.
                    let binding = SessionBinding {
                        project_id: id.package.project,
                        incarnation_id: id.incarnation,
                        owner_epoch: package::PackageUuid::parse(&uid(799_002)).unwrap(),
                        owner: owner.clone(),
                    };
                    let mut live = Session::open(
                        RotatingSession {
                            current: &recovered,
                            next: Some(&rotated),
                            bytes: settled.clone(),
                            binding: binding.clone(),
                        },
                        recovered.package().shared_clone(),
                        inputs.clone(),
                        budget(),
                        limits,
                        recovered.header().clone(),
                        binding,
                    )
                    .unwrap();
                    let before = live.snapshot().clone();
                    live.begin_response();
                    live.maintain().unwrap();
                    let after = live.snapshot().clone();
                    assert_eq!(after.accepted, before.accepted);
                    assert_ne!(after.saved, before.saved);
                    assert_eq!(after.event_sequence, before.event_sequence + 1);
                    assert!(!after.dirty && !after.frozen);
                    let presentation = live.finish_response().unwrap();
                    assert_eq!(presentation.graphs, session.graph_objects().unwrap());
                    live.maintain().unwrap();
                    assert_eq!(live.snapshot(), &after, "no-op maintenance is unchanged");
                    assert_eq!(
                        rotated.base().value()["archive"],
                        reference(next_history.archive())
                    );
                    let mut next_records = records.0.clone();
                    next_records.extend(next_history.lookup_objects().clone());
                    for value in [
                        next_history.archive(),
                        next_plan.original(),
                        recovered.selector().value(),
                    ] {
                        next_records.insert(wire::key(&reference(value)).unwrap(), value.clone());
                    }
                    let mut cold = Cold(Some(super::super::checkpoint::ReopenState {
                        selector: encode(rotated.selector().value()),
                        base: encode(rotated.base().value()),
                        header: rotated.header().clone(),
                        segment: rotated.segment().to_vec(),
                        current: next_io.verify(&next_plan).unwrap(),
                        records: std::sync::Arc::new(Archived {
                            objects: next_records.clone(),
                            stream: recovered.header()["stream_id"].clone(),
                            bytes: settled.clone(),
                            reads: std::sync::atomic::AtomicUsize::new(0),
                        }),
                    }));
                    let reopened = super::super::checkpoint::reopen(
                        &mut cold,
                        &held,
                        &id,
                        activation_limits(),
                    )
                    .unwrap();
                    let parsed =
                        Journal::read(reopened.segment(), reopened.header(), limits).unwrap();
                    let view = reopened
                        .verify_session(&parsed, frames(), &reader, budget())
                        .unwrap();
                    assert_eq!(view.coordinate(), session.coordinate());
                    assert_eq!(view.through(), session.through());
                    assert_eq!(view.accepted_frame(), session.accepted_frame());
                    assert!(view.undo_targets(&owner).is_empty());
                    let original = first_body.as_ref().unwrap();
                    let (retry_record, retry_checksum) = reopened
                        .retained_mutation(
                            original["operation_receipt"]["operation_id"]
                                .as_str()
                                .unwrap(),
                            frames(),
                            &reader,
                        )
                        .unwrap()
                        .unwrap();
                    assert_eq!(retry_record["body"], *original);
                    let prior_journal =
                        Journal::read(&settled, recovered.header(), limits).unwrap();
                    assert_eq!(
                        retry_checksum,
                        prior_journal
                            .records()
                            .iter()
                            .find(|r| r.value()["record_id"] == retry_record["record_id"])
                            .unwrap()
                            .checksum()
                    );
                    let mut changed_intent = original["semantic_intent"].clone();
                    changed_intent["command"]["envelope"]["command"]["x"] = json!(999);
                    assert!(
                        reopened
                            .original_mutation(
                                &changed_intent,
                                &original["owner"],
                                &original["action"],
                                frames(),
                                &reader
                            )
                            .is_err()
                    );
                    // No new Mutation exists in this segment. Its accepted
                    // link still names the authenticated previous-base record.
                    assert!(
                        parsed
                            .records()
                            .iter()
                            .all(|r| r.value()["kind"] != "Mutation")
                    );
                    assert!(view.through().is_some());
                    let empty_history = PreparedHistory::prepare_next(
                        &view,
                        reopened.package(),
                        &id,
                        package::PackageUuid::parse(&uid(730_003)).unwrap(),
                        (2, 2),
                        &reader,
                        reopened.base(),
                        &LocalRecords(next_records),
                    )
                    .unwrap();
                    let mut empty_attempt = super::attempt(reopened.package(), 740_000);
                    empty_attempt.selectors.truncate(4);
                    let mut empty_raw = serde_json::to_value(empty_attempt).unwrap();
                    empty_raw["planner"]["codec"] =
                        json!("photara.codec.ps2-checkpoint-turnover-layout-v1");
                    for corridor in empty_raw["planner"]["corridors"].as_array_mut().unwrap() {
                        corridor.as_object_mut().unwrap().remove("original_sha256");
                        corridor["base_head_sha256"] =
                            json!(hash(&encode(&reopened.package().package.head)));
                    }
                    let empty_plan = TurnoverPlan::compile_inner(
                        &empty_history,
                        reopened.selector(),
                        TurnoverIds {
                            maintenance_id: uid(750_000),
                            checkpoint_id: uid(750_001),
                            next_stream_id: uid(750_002),
                        },
                        empty_raw,
                        budget(),
                    )
                    .unwrap();
                    let empty_retry = TurnoverPlan::restore(
                        &empty_history,
                        reopened.selector(),
                        &encode(empty_plan.original()),
                        &encode(empty_plan.acceptance_base()),
                        budget(),
                    )
                    .unwrap();
                    assert_eq!(empty_plan.files, empty_retry.files);
                    continued.pop();
                    let torn = Journal::read(&continued, active.header(), limits).unwrap();
                    assert!(
                        active
                            .verify_session(&torn, frames(), &reader, budget())
                            .is_err()
                    );
                }

                let exact = (encode(active.selector().value()), active.segment().to_vec());
                if let Some(prior) = &expected {
                    assert_eq!(prior, &exact);
                }
                expected = Some(exact);
                let retry = activate(
                    &mut io,
                    &held,
                    &plan,
                    &history,
                    &selector,
                    &id,
                    activation_limits(),
                )
                .unwrap();
                assert_eq!(retry.segment(), active.segment());
                let installed = io.installed.clone();
                *io.installed.as_mut().unwrap().2.last_mut().unwrap() ^= 1;
                assert!(
                    activate(
                        &mut io,
                        &held,
                        &plan,
                        &history,
                        &selector,
                        &id,
                        activation_limits()
                    )
                    .is_err()
                );
                io.installed = installed;
                let mut third = active.selector().value().clone();
                third["selected"]["header_sha256"] = json!("aa".repeat(32));
                io.selector = encode(&third);
                let effects = io.effects;
                assert!(
                    activate(
                        &mut io,
                        &held,
                        &plan,
                        &history,
                        &selector,
                        &id,
                        activation_limits()
                    )
                    .is_err()
                );
                assert_eq!(effects, io.effects);
            }
        }
    }
    raw["planner"]["corridors"][0]["base_head_sha256"] = json!("00".repeat(32));
    assert!(TurnoverPlan::compile(&history, &selector, ids(), raw, budget()).is_err());
}

/// Synthetic custody IO with real independent clean reader verification. Cuts
/// occur after effects, so retry must reconcile/rebarrier exact existing bytes.
struct CheckpointMemory<'a> {
    fixture: &'a support::Fixture,
    package: super::super::shared::OriginalPackage,
    selector: Vec<u8>,
    original: Option<Vec<u8>>,
    installed: Option<(Vec<u8>, Value, Vec<u8>)>,
    fail_after: Option<usize>,
    effects: usize,
}
impl CheckpointMemory<'_> {
    fn effect(&mut self) -> Result<(), package::PackageError> {
        self.effects += 1;
        if self.fail_after == Some(self.effects) {
            self.fail_after = None;
            Err(package::PackageError::Integrity)
        } else {
            Ok(())
        }
    }
    fn verify(
        &self,
        plan: &super::super::checkpoint::TurnoverPlan,
    ) -> Result<super::super::VerifiedOriginal, package::PackageError> {
        let observed = snapshot(self.fixture, &self.package);
        plan.verify_candidate(
            self.package.public(),
            3,
            identity(&self.package),
            &observed,
            &self.fixture.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &support::limits(),
        )?
        .ok_or(package::PackageError::Integrity)
    }
}
impl super::super::checkpoint::CheckpointIo for CheckpointMemory<'_> {
    type Lease = ();
    fn observe_selector(&mut self, _: &Lease, _: &Value) -> Result<Vec<u8>, package::PackageError> {
        Ok(self.selector.clone())
    }
    fn retain_original(
        &mut self,
        _: &Lease,
        plan: &super::super::checkpoint::TurnoverPlan,
        _: &super::super::checkpoint::PreparedHistory<'_>,
    ) -> Result<(), package::PackageError> {
        let bytes = encode(plan.original());
        if self.original.as_ref().is_some_and(|old| old != &bytes) {
            return Err(package::PackageError::Integrity);
        }
        self.original = Some(bytes);
        self.effect()
    }
    fn select(&mut self, _: &Lease, old: &[u8], new: &[u8]) -> Result<(), package::PackageError> {
        if self.selector != old && self.selector != new {
            return Err(package::PackageError::Integrity);
        }
        self.selector = new.to_vec();
        self.effect()
    }
    fn finish_package(
        &mut self,
        _: &Lease,
        plan: &super::super::checkpoint::TurnoverPlan,
    ) -> Result<super::super::VerifiedOriginal, package::PackageError> {
        if self.original.as_deref() != Some(encode(plan.original()).as_slice()) {
            return Err(package::PackageError::Integrity);
        }
        self.effect()?;
        self.verify(plan)
    }
    fn verify_clean(
        &mut self,
        _: &Lease,
        plan: &super::super::checkpoint::TurnoverPlan,
    ) -> Result<super::super::VerifiedOriginal, package::PackageError> {
        self.verify(plan)
    }
    fn install_segment(
        &mut self,
        _: &Lease,
        base: &[u8],
        header: &Value,
        segment: &[u8],
    ) -> Result<(), package::PackageError> {
        let candidate = (base.to_vec(), header.clone(), segment.to_vec());
        if self.installed.as_ref().is_some_and(|old| old != &candidate) {
            return Err(package::PackageError::Integrity);
        }
        self.installed = Some(candidate);
        self.effect()
    }
}

#[test]
fn session_overlay_import_preserves_verified_legacy_fanout() {
    let fixture = fixture();
    let old = verify(&fixture);
    let (intent, receipt) = request();
    let mut inputs = attempt(&old, 980_000);
    inputs.planner.codec = "photara.codec.ps2-repeatable-layout-v3".into();
    assert_eq!(inputs.planner.geometry.physical_leaf, 4);
    let plan = plan::compile_inner(&old, &intent, &receipt, inputs.clone(), budget()).unwrap();
    let retry = plan::compile_inner(&old, &intent, &receipt, inputs.clone(), budget()).unwrap();
    assert_eq!(vector(&plan), vector(&retry));
    let selected = super::selected(&plan, 6);
    let observed = snapshot(&fixture, &selected);
    v1_3::reader::verify_operation_original(
        selected.clone(),
        identity(&selected),
        &observed,
        &fixture.registration,
        v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        frames(),
        &support::limits(),
        &plan,
        6,
    )
    .unwrap();
    let mut restricted = budget();
    restricted.max_entries = 4;
    assert!(plan::compile_inner(&old, &intent, &receipt, inputs, restricted).is_err());
    // Trust remains with the original reader: the legacy locator has 64-entry
    // pages, and a genuinely smaller reader bound still refuses that package.
    let mut reader = support::limits();
    reader.tree.max_leaf_entries = 4;
    assert!(
        v1_3::reader::verify_shared_original(
            old.package.clone(),
            identity(&old.package),
            &fixture,
            &fixture.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900
            },
            frames(),
            &reader
        )
        .is_err()
    );
}
