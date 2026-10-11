//! Existing corpus, real Core/journal/repeatable executor; synthetic IO faults.
use super::super::{
    journal::{Journal, JournalAppend, JournalLimits, JournalRecordIdentity, VerifiedJournal},
    session::*,
};
use super::*;
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
    verifications: std::rc::Rc<std::cell::Cell<usize>>,
    memo: std::cell::RefCell<super::super::restart::CompiledPlans>,
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
            return v1_3::verify_original(
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
        let lease = lease(&p.old);
        execute(&mut self.mock, &lease, &p).map_err(|_| EffectFailure::OutcomeUnknown)?;
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
    let mut header = super::journal_tests::header(original.package());
    header["journal_id"] = receipt["journal_id"].clone();
    let bytes = Journal::create(&header, limits()).unwrap();
    let id = identity(original.package());
    let reader = support::limits();
    let ctx = super::journal_tests::context(&base, &id, &reader);
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let prepared = journal
        .verify(original.package(), &ctx)
        .unwrap()
        .prepare_intent(&plan, rid(120_001), limits())
        .unwrap();
    let mut preflight = bytes.clone();
    preflight.extend(prepared.bytes());
    let parsed = Journal::read(&preflight, &header, limits()).unwrap();
    // No package effect or journal append has occurred: this is the exact next
    // immutable journal candidate inspected before its first durable write.
    parsed
        .verify_cached(original.package(), &ctx, &mut memo)
        .unwrap();
    assert_eq!(
        memo.compilations, 1,
        "preflight must reuse the just-compiled original"
    );
    assert!(memo.hits > 0);
    let retry = super::super::restart::restore_intent_plan_cached(
        original.package(),
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
    let mut changed = original.package().clone();
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
        original.package(),
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
    let source = original.package();
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
    let expected = j.verify(current, &context).unwrap();
    let mut memo = super::super::restart::CompiledPlans::default();
    j.verify_cached(current, &context, &mut memo).unwrap();
    let cold_records = memo.journal_replayed;
    let before = std::time::Instant::now();
    let cached = j.verify_cached(current, &context, &mut memo).unwrap();
    assert_eq!(memo.journal_replayed, 0);
    // Exercise the exact shortcut used by native Host::original after journal
    // verification, not merely another Journal scan. No historical recursion.
    let work = (memo.compilations, memo.reader_work());
    let native_original = memo.current_original(current, &context).unwrap().unwrap();
    assert_eq!((memo.compilations, memo.reader_work()), work);
    let full_original = backend.original();
    assert_eq!(native_original.package().head, full_original.package().head);
    assert_eq!(native_original.proof.global, full_original.proof.global);
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
    let mut duplicate = original.package().clone();
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
