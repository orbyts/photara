//! Private stdio adapter for the explicitly controlled disposable-image lab.
//! No production lease constructor, public filesystem path, or network service.
use super::{
    BTreeMap, CHECKPOINT, EffectFailure, IoResult, Native, NativeLease, PlannerInputs,
    RepeatablePlan, RestartContext, Value, VerifiedOriginal, attempt, barrier, budget, canonical,
    execute, frames, hash, identity, invalid, journal, json, number, package, plan, read,
    record_identity, recovery, reference, regular, request, restart, restore_plan, snapshot,
    support, uncertain, v1_3,
};
use crate::package::v1_3::repeatable::session::{
    AcceptedCoordinate, Session, SessionBinding, SessionError, SessionFailure, SessionIo,
    SessionSnapshot,
};
#[cfg(test)]
use std::io::{BufRead, Read, Write};

#[track_caller]
fn session_error(error: impl std::fmt::Debug) -> SessionError {
    eprintln!(
        "private session IO refusal at {}: {error:?}",
        std::panic::Location::caller()
    );
    SessionError::Failure(SessionFailure::Storage)
}
struct Host {
    io: Native,
    lease: NativeLease,
    binding: SessionBinding,
    seed: Option<PlannerInputs>,
}
impl Host {
    fn context<'a>(
        &'a self,
        observed: &'a support::Fixture,
        id: &'a v1_3::SelectionIdentity,
        reader: &'a v1_3::ReaderLimits,
    ) -> RestartContext<'a, support::Fixture> {
        RestartContext {
            inspection: observed,
            registration: &self.io.base.registration,
            directory: v1_3::DirectoryObservation {
                device: self.io.root_pin[0],
                inode: self.io.root_pin[1],
            },
            identity: id,
            frames: frames(),
            reader,
            planner: budget(),
            max_originals: self.io.journal_bounds().max_records,
        }
    }
    fn header_original(&self) -> IoResult<VerifiedOriginal> {
        let current = self.io.current()?;
        let j = self.io.checkpoint()?;
        let raw = if let Some(first) = j
            .records()
            .iter()
            .find(|r| r.value()["kind"] == "CheckpointIntent")
        {
            recovery::reconstruct_original(&current, &first.value()["body"]["original"], budget())
                .map_err(|_| invalid())?
        } else {
            current.clone()
        };
        if raw.head != self.io.journal_header["base_head"] {
            return Err(invalid());
        }
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        restart::verify_original_prefix(raw, &self.context(&observed, &id, &reader))
            .map_err(|_| invalid())
    }
    fn original(&self) -> IoResult<VerifiedOriginal> {
        let current = self.io.current()?;
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        let ctx = self.context(&observed, &id, &reader);
        let directory = ctx.directory;
        if current.head == self.io.journal_header["base_head"] {
            v1_3::verify_original(
                current,
                id,
                &observed,
                &self.io.base.registration,
                directory,
                frames(),
                &reader,
            )
            .map_err(|_| invalid())
        } else {
            let plan = restore_plan(&current, &ctx).map_err(|_| invalid())?;
            v1_3::reader::verify_operation_original(
                current,
                id,
                &observed,
                &self.io.base.registration,
                directory,
                frames(),
                &reader,
                &plan,
                6,
            )
            .map_err(|_| invalid())
        }
    }
    fn persisted(&self, operation: &Value) -> IoResult<RepeatablePlan> {
        let current = self.io.current()?;
        let j = self.io.checkpoint()?;
        let b = &j
            .records()
            .iter()
            .find(|r| {
                r.value()["kind"] == "CheckpointIntent"
                    && r.value()["body"]["operation_receipt"]["operation_id"] == *operation
            })
            .ok_or_else(invalid)?
            .value()["body"];
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        restart::restore_intent_plan(
            &current,
            &b["original"],
            &b["semantic_intent"],
            &b["operation_receipt"],
            &self.context(&observed, &id, &reader),
        )
        .map_err(|_| invalid())
    }
    fn execute_original(&mut self, operation: &Value) -> IoResult<()> {
        let plan = self
            .persisted(operation)
            .map_err(|e| std::io::Error::other(format!("restore original: {e}")))?;
        self.io.allow(&plan);
        let base = self.header_original()?;
        // The actual Mutation/barrier are now retained in the parsed journal;
        // count them once, not again as pre-Mutation future capacity.
        self.io.reserved_journal_extra = 0;
        self.io.prepare_checkpoint_with(
            &plan,
            Some((&base, self.seed.as_ref().expect("initialized header seed"))),
        )?;
        execute(&mut self.io, &self.lease, &plan)
            .map_err(|e| std::io::Error::other(format!("native checkpoint execute: {e:?}")))?;
        Ok(())
    }
    fn reserve_mutation(&mut self, mutation: &journal::JournalAppend) -> Result<(), SessionError> {
        // Retain exact Mutation plus the maximum-width one finite barrier in the
        // original standing pool before persisting its intent. This is a local
        // fixture reservation, never a claim about all APFS allocation overhead.
        let mut barrier = mutation.record().value().clone();
        barrier["kind"] = json!("SessionBarrier");
        barrier["record_id"] = json!(uuid::Uuid::nil().to_string());
        barrier["sequence"] = json!(u64::MAX.to_string());
        barrier["session_generation"] = json!(u64::MAX.to_string());
        barrier["body"] = json!({"owner":self.binding.owner,"reason":"terminate","through":{"record_id":uuid::Uuid::nil().to_string(),"record_checksum":"f".repeat(64)},"target":mutation.record().value()["body"]["result"],"accepted_frame":mutation.record().value()["body"]["accepted_frame"]});
        // Mutation is re-prepared after the intent appends. Bound its sequence
        // and generation widths too, rather than using the dry prefix's width.
        let mut maximum_mutation = mutation.record().value().clone();
        maximum_mutation["sequence"] = json!(u64::MAX.to_string());
        maximum_mutation["session_generation"] = json!(u64::MAX.to_string());
        self.io.reserved_journal_extra = canonical(&maximum_mutation)
            .len()
            .checked_add(canonical(&barrier).len())
            .and_then(|n| n.checked_add(72))
            .ok_or(SessionError::Failure(SessionFailure::Capacity))?;
        Ok(())
    }
    fn current_target(&self) -> Result<AcceptedCoordinate, SessionError> {
        let j = self.io.checkpoint().map_err(session_error)?;
        let v = self.verify(&j)?;
        let base = self.header_original().map_err(session_error)?;
        let s = v.verify_session(
            &base,
            self.seed.as_ref().expect("initialized header seed"),
            budget(),
        )?;
        Ok(AcceptedCoordinate {
            coordinate: s.coordinate(),
            mutation: s.through(),
            accepted_frame: s.accepted_frame(),
        })
    }
    fn prior(
        &self,
        operation: &str,
    ) -> IoResult<Option<(Value, Value, Value, journal::JournalRecordIdentity)>> {
        let j = self.io.checkpoint()?;
        if let Some(r) = j.records().iter().find(|r| {
            r.value()["kind"] == "Mutation"
                && r.value()["body"]["operation_receipt"]["operation_id"] == operation
        }) {
            let b = &r.value()["body"];
            if b["owner"] != self.binding.owner {
                return Err(invalid());
            }
            return Ok(Some((
                b["semantic_intent"].clone(),
                b["operation_receipt"].clone(),
                b["action"].clone(),
                record_identity(Some(r.value()), self.io.scope.generation)?,
            )));
        }
        Ok(None)
    }
}
impl SessionIo for Host {
    fn authorize(&self, b: &SessionBinding, intent: Option<&Value>) -> Result<(), SessionError> {
        if *b != self.binding
            || intent.is_some_and(|v| {
                v["project_id"] != self.io.journal_header["project_id"]
                    || v["library_id"] != self.io.journal_header["library_id"]
            })
        {
            return Err(SessionError::Failure(SessionFailure::Authorization));
        }
        self.io.lease_ok(&self.lease).map_err(session_error)?;
        self.io.pins().map_err(session_error)
    }
    fn read_journal(&self) -> Result<Vec<u8>, SessionError> {
        read(&self.io.evidence, CHECKPOINT).map_err(session_error)
    }
    fn verify<'a>(
        &self,
        j: &'a journal::Journal,
    ) -> Result<journal::VerifiedJournal<'a>, SessionError> {
        let current = self.io.current().map_err(session_error)?;
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        Ok(j.verify(&current, &self.context(&observed, &id, &reader))?)
    }
    fn selected_head(&self) -> Result<Value, SessionError> {
        Ok(self.io.current().map_err(session_error)?.head)
    }
    fn admit_mutation(
        &mut self,
        intent: &Value,
        receipt: &Value,
        mutation: &journal::JournalAppend,
    ) -> Result<(), SessionError> {
        let j = self.io.checkpoint().map_err(session_error)?;
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointIntent"
                && r.value()["body"]["operation_receipt"]["operation_id"] == receipt["operation_id"]
        }) {
            let p = self
                .persisted(&receipt["operation_id"])
                .map_err(session_error)?;
            if p.prepared.receipt != *receipt
                || p.original()["request"]["intent"] != reference(intent)
            {
                return Err(SessionError::InvalidRequest);
            }
            let has_mutation = j.records().iter().any(|r| {
                r.value()["kind"] == "Mutation"
                    && r.value()["body"]["operation_receipt"]["operation_id"]
                        == receipt["operation_id"]
            });
            if !has_mutation {
                self.reserve_mutation(mutation)?;
                self.io.allow(&p);
                let base = self.header_original().map_err(session_error)?;
                self.io
                    .prepare_checkpoint_with(
                        &p,
                        Some((&base, self.seed.as_ref().expect("initialized header seed"))),
                    )
                    .map_err(session_error)?;
            }
            // Exact original retry has no new capacity or physical admission.
            barrier(&regular(&self.io.evidence, CHECKPOINT, true).map_err(session_error)?)
                .map_err(session_error)?;
            barrier(&self.io.evidence).map_err(session_error)?;
            return Ok(());
        }
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointIntent"
                && !j.records().iter().any(|m| {
                    matches!(
                        m.value()["kind"].as_str(),
                        Some("Mutation" | "CheckpointReceipt")
                    ) && m.value()["body"]["operation_receipt"]["operation_id"]
                        == r.value()["body"]["operation_receipt"]["operation_id"]
                })
        }) {
            return Err(SessionError::Busy);
        }
        // At most one physical admission in flight: complete older Accepted work.
        let target = self.current_target()?;
        if target.mutation.is_some() {
            self.checkpoint(&target).map_err(session_error)?;
        }
        let original = self.original().map_err(session_error)?;
        let n = number(&receipt["acceptance_ordinal"])
            .map_err(session_error)?
            .checked_mul(1000)
            .ok_or(SessionError::Failure(SessionFailure::Capacity))?;
        let p = plan::compile_inner(
            &original,
            intent,
            receipt,
            self.io.attempt(&original, n).map_err(session_error)?,
            budget(),
        )
        .map_err(session_error)?;
        self.reserve_mutation(mutation)?;
        self.io.allow(&p);
        let base = self.header_original().map_err(session_error)?;
        self.io
            .prepare_checkpoint_with(
                &p,
                Some((&base, self.seed.as_ref().expect("initialized header seed"))),
            )
            .map_err(session_error)
    }
    fn append(&mut self, a: &journal::JournalAppend) -> Result<(), EffectFailure> {
        if a.existing() {
            return uncertain(
                barrier(
                    &regular(&self.io.evidence, CHECKPOINT, true)
                        .map_err(|_| EffectFailure::OutcomeUnknown)?,
                )
                .and_then(|()| barrier(&self.io.evidence)),
            );
        }
        if a.expected_extent()
            .checked_add(a.bytes().len())
            .is_none_or(|n| u64::try_from(n).unwrap_or(u64::MAX) > self.io.journal_capacity)
        {
            return Err(EffectFailure::OutcomeUnknown);
        }
        uncertain(self.io.append_checkpoint(a))
    }
    fn checkpoint(&mut self, target: &AcceptedCoordinate) -> Result<(), EffectFailure> {
        let Some(frame) = &target.accepted_frame else {
            return Ok(());
        };
        let operation = &frame["operation_id"];
        let j = self
            .io
            .checkpoint()
            .map_err(|_| EffectFailure::OutcomeUnknown)?;
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointReceipt"
                && r.value()["body"]["operation_receipt"]["operation_id"] == *operation
        }) {
            self.verify(&j).map_err(|_| EffectFailure::OutcomeUnknown)?;
            return uncertain(
                barrier(
                    &regular(&self.io.evidence, CHECKPOINT, true)
                        .map_err(|_| EffectFailure::OutcomeUnknown)?,
                )
                .and_then(|()| barrier(&self.io.evidence)),
            );
        }
        uncertain(self.execute_original(operation))
    }
    fn reconcile(&mut self) -> Result<(), SessionError> {
        self.io
            .scope
            .assessment
            .revalidate()
            .map_err(session_error)?;
        self.io.pins().map_err(session_error)?;
        let j = self.io.checkpoint().map_err(session_error)?;
        self.verify(&j)?;
        // A pre-Mutation intent alone is not accepted and is never checkpointed
        // by reopen. Exact request resubmission is required to retain its action.
        for r in j.records() {
            if r.value()["kind"] != "Mutation" {
                continue;
            }
            let op = &r.value()["body"]["operation_receipt"]["operation_id"];
            if !j.records().iter().any(|r| {
                r.value()["kind"] == "CheckpointReceipt"
                    && r.value()["body"]["operation_receipt"]["operation_id"] == *op
            }) {
                self.execute_original(op).map_err(session_error)?;
            }
        }
        barrier(&regular(&self.io.evidence, CHECKPOINT, true).map_err(session_error)?)
            .map_err(session_error)?;
        barrier(&self.io.evidence).map_err(session_error)
    }
}
fn link(v: &Value) -> Value {
    json!({"record_id":v["record_id"],"checksum":v["record_checksum"]})
}
fn projected(a: &AcceptedCoordinate) -> Value {
    json!({"revision":number(&a.coordinate["revision"]).unwrap_or(0),"authored_digest":a.coordinate["authored_digest"],"coordinate_sha256":hash(&canonical(&a.coordinate)),"mutation":a.mutation.as_ref().map(link),"accepted_frame_sha256":a.accepted_frame.as_ref().map(|f|hash(&canonical(f))),"operation_id":a.accepted_frame.as_ref().map(|f|f["operation_id"].clone())})
}
fn binding_dto(b: &SessionBinding) -> Value {
    json!({"project_id":b.project_id,"incarnation_id":b.incarnation_id,"owner_epoch":b.owner_epoch,"attachment_id":b.owner["attachment_id"],"attachment_generation":number(&b.owner["attachment_generation"]).unwrap_or(0),"principal_sha256":hash(&canonical(&b.owner["principal"]))})
}
fn projection(s: &SessionSnapshot) -> Value {
    json!({"binding":binding_dto(&s.binding),"event_sequence":s.event_sequence,"accepted":projected(&s.accepted),"saved":s.saved.as_ref().map(|saved|json!({"target":projected(&saved.target),"head_sha256":hash(&canonical(&saved.head)),"commit_sha256":saved.commit_sha256,"checkpoint_receipt":link(&saved.checkpoint_receipt)})),"frozen":s.frozen,"closed":s.closed,"failure":s.failure})
}
fn identity_new(generation: u64) -> journal::JournalRecordIdentity {
    record_identity(None, generation).expect("fresh UUID")
}
fn result(s: &Session<Host>) -> Result<Value, SessionError> {
    let view = s.presentation()?;
    let mut nodes = Vec::new();
    for g in view.graphs {
        for n in g["graph"]["nodes"]
            .as_array()
            .ok_or(SessionError::InvalidRequest)?
        {
            let id = n["id"].as_str().ok_or(SessionError::InvalidRequest)?;
            let title = n["definition"]["definition_id"]
                .as_str()
                .ok_or(SessionError::InvalidRequest)?;
            let (Some(x), Some(y)) = (
                n["photara.graph-position"]["x"].as_i64(),
                n["photara.graph-position"]["y"].as_i64(),
            ) else {
                continue;
            };
            nodes.push(json!({"id":id,"title":title,"x":x,"y":y}));
        }
    }
    Ok(
        json!({"nodes":nodes,"undo_available":!view.undo.is_empty(),"redo_available":!view.redo.is_empty()}),
    )
}
fn matches_request(req: &Value, intent: &Value, action: &Value) -> bool {
    match req["command"].as_str() {
        Some("submit") => {
            action["kind"] == "edit"
                && intent["command"]["envelope"]["command"]["kind"] == "set-node-position"
                && ["node_id", "x", "y"]
                    .iter()
                    .all(|k| req[*k] == intent["command"]["envelope"]["command"][*k])
        }
        Some("undo") => action["kind"] == "undo",
        Some("redo") => action["kind"] == "redo",
        _ => false,
    }
}
/// The app, CLI and historical test host share this exact coordinator/adapter.
pub(in crate::package::v1_3::repeatable::disposable) struct OpenSession {
    session: Session<Host>,
    requests: BTreeMap<String, (Value, Value, Value, journal::JournalRecordIdentity)>,
    generation: u64,
    start: std::time::Instant,
}
impl OpenSession {
    pub(super) fn activation_device(&mut self) -> Value {
        self.session.io_mut().io.journal_header["device_id"].clone()
    }
    pub(super) fn activation_target(&mut self) -> IoResult<Value> {
        let io = &self.session.io_mut().io;
        let current = io.current()?;
        let authored = &current.commit["root_set"];
        let graph = request().0["command"]["envelope"]["graph_id"].clone();
        Ok(
            json!({"library_id":authored["library_id"],"project_id":io.journal_header["project_id"],"incarnation_id":io.journal_header["incarnation_id"],"graph_id":graph,"registration_sha256":hash(&read(&io.scope.scratch,"repeatable-registration.json")?)}),
        )
    }
    pub(super) fn activation_binding(&self) -> Value {
        serde_json::to_value(&self.session.snapshot().binding).expect("typed binding")
    }
    pub(super) fn activation_display(&self) -> IoResult<Value> {
        Ok(
            json!({"snapshot":projection(self.session.snapshot()),"result":result(&self.session).map_err(|_|invalid())?,"error":null,"acknowledgement":null}),
        )
    }
    pub(super) fn activation_flush(&mut self, target: &Value) -> IoResult<Value> {
        if self.activation_target()? != *target {
            return Err(invalid());
        }
        self.session.retry().map_err(|_| invalid())?;
        let flush = self
            .session
            .begin_flush("flush", identity_new(self.generation))
            .map_err(|_| invalid())?;
        self.session.finish_flush(&flush).map_err(|_| invalid())?;
        self.activation_observe(target)
    }
    pub(super) fn activation_observe(&mut self, target: &Value) -> IoResult<Value> {
        if self.activation_target()? != *target {
            return Err(invalid());
        }
        let evidence = self
            .session
            .activation_evidence(
                target["registration_sha256"].as_str().ok_or_else(invalid)?,
                target["graph_id"].as_str().ok_or_else(invalid)?,
            )
            .map_err(|_| invalid())?;
        let actual = self.session.io_mut().io.current()?;
        if evidence["head"] != actual.head || evidence["commit"] != actual.commit {
            return Err(invalid());
        }
        Ok(evidence)
    }
    pub(super) fn open(io: Native) -> IoResult<Self> {
        if !io.session_mode {
            return Err(invalid());
        }
        let current = io.current()?;
        let lease = io.lease(&current)?;
        let registration: Value =
            serde_json::from_slice(&read(&io.scope.scratch, "repeatable-registration.json")?)
                .map_err(|_| invalid())?;
        let binding = SessionBinding {
            project_id: package::PackageUuid::parse(
                io.journal_header["project_id"]
                    .as_str()
                    .ok_or_else(invalid)?,
            )
            .map_err(|_| invalid())?,
            incarnation_id: package::PackageUuid::parse(
                io.journal_header["incarnation_id"]
                    .as_str()
                    .ok_or_else(invalid)?,
            )
            .map_err(|_| invalid())?,
            owner_epoch: package::PackageUuid::parse(&uuid::Uuid::new_v4().to_string())
                .map_err(|_| invalid())?,
            owner: json!({"attachment_id":io.journal_header["stream_id"],"attachment_generation":registration["generation"].as_u64().ok_or_else(invalid)?.to_string(),"principal":request().1["provenance"]["principal"]}),
        };
        let mut host = Host {
            io,
            lease,
            binding: binding.clone(),
            seed: None,
        };
        let base = host.header_original()?;
        host.seed = Some(attempt(&base, 1000)?.planner);
        let seed = host.seed.clone().expect("initialized header seed");
        let header = host.io.journal_header.clone();
        let limits = host.io.journal_bounds();
        let generation = host.io.scope.generation;
        let s = Session::open(host, base, seed, budget(), limits, header, binding)
            .map_err(|e| std::io::Error::other(format!("session open {e:?}")))?;
        Ok(Self {
            session: s,
            requests: BTreeMap::new(),
            generation,
            start: std::time::Instant::now(),
        })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One serialized request path shared by controlled app and CLI"
    )]
    pub(in crate::package::v1_3::repeatable::disposable) fn execute(
        &mut self,
        req: &Value,
    ) -> Value {
        let s = &mut self.session;
        let requests = &mut self.requests;
        let generation = self.generation;
        let start = self.start;
        let mut ack = Value::Null;
        let mut response = (|| -> Result<(), SessionError> {
            let id = req["id"].as_str().ok_or(SessionError::InvalidRequest)?;
            let operation =
                package::PackageUuid::parse(id).map_err(|_| SessionError::InvalidRequest)?;
            let command = req["command"]
                .as_str()
                .ok_or(SessionError::InvalidRequest)?;
            if command != "snapshot"
                && (req["expected_owner_epoch"] != s.snapshot().binding.owner_epoch.to_string()
                    || req["expected_attachment_generation"]
                        .as_u64()
                        .map(|n| n.to_string())
                        .as_deref()
                        != s.snapshot().binding.owner["attachment_generation"].as_str())
            {
                return Err(SessionError::InvalidRequest);
            }
            let prior_request = s
                .io_mut()
                .prior(id)
                .map_err(session_error)?
                .or_else(|| requests.get(id).cloned());
            if prior_request
                .as_ref()
                .is_some_and(|(intent, _, action, _)| !matches_request(req, intent, action))
            {
                return Err(SessionError::InvalidRequest);
            }
            if s.snapshot().closed {
                return Err(SessionError::InvalidRequest);
            }
            if s.snapshot().frozen {
                s.retry()?;
            }
            match command {
                "snapshot" | "revalidate" => {
                    s.retry()?;
                }
                "submit" | "undo" | "redo" => {
                    let prior = s.io_mut().prior(id).map_err(session_error)?;
                    let (intent, receipt, action, record) = if let Some(v) =
                        prior.or_else(|| requests.get(id).cloned())
                    {
                        v
                    } else {
                        let action = match command {
                            "undo" => {
                                json!({"kind":"undo","target":s.undo_targets()?.first().ok_or(SessionError::InvalidRequest)?})
                            }
                            "redo" => {
                                let r = s
                                    .redo_targets()?
                                    .last()
                                    .cloned()
                                    .ok_or(SessionError::InvalidRequest)?;
                                json!({"kind":"redo","target":r["target"],"undo":r["undo"]})
                            }
                            _ => json!({"kind":"edit"}),
                        };
                        let mut core = request().0["command"].clone();
                        core["envelope"]["command"]["node_id"] = req["node_id"].clone();
                        core["envelope"]["command"]["x"] = req["x"].clone();
                        core["envelope"]["command"]["y"] = req["y"].clone();
                        let mut provenance = request().1["provenance"].clone();
                        provenance["effective_scope"]["project_id"] =
                            json!(s.snapshot().binding.project_id);
                        let (intent, receipt) = s.prepare_graph(
                            core,
                            operation,
                            "2026-10-05T00:00:00.000Z",
                            &provenance,
                            &action,
                        )?;
                        (intent, receipt, action, identity_new(generation))
                    };
                    if !matches_request(req, &intent, &action) {
                        return Err(SessionError::InvalidRequest);
                    }
                    requests.insert(
                        id.to_owned(),
                        (intent.clone(), receipt.clone(), action.clone(), record),
                    );
                    let a = s.submit(
                        &intent,
                        &receipt,
                        &action,
                        record,
                        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    )?;
                    ack = json!({"binding":binding_dto(&s.snapshot().binding),"operation_id":id,"accepted":projected(&AcceptedCoordinate{coordinate:a.result().clone(),mutation:Some(a.mutation().clone()),accepted_frame:Some(a.accepted_frame().clone())}),"operation_receipt_sha256":hash(&canonical(a.receipt()))});
                }
                "complete" | "barrier" | "close" => {
                    let t = s.begin_flush(
                        if command == "close" { "close" } else { "flush" },
                        identity_new(generation),
                    )?;
                    s.finish_flush(&t)?;
                }
                "retry" => s.retry()?,
                _ => return Err(SessionError::InvalidRequest),
            }
            Ok(())
        })();
        let displayed = if s.snapshot().closed && response.is_ok() {
            // Close already has a fully verified final snapshot. No extra graph
            // inspection after the coordinator commits its closed state.
            Value::Null
        } else if let Ok(value) = result(s) {
            value
        } else {
            response = Err(s.freeze(SessionFailure::Journal));
            Value::Null
        };
        let output = json!({"id":req["id"],"snapshot":projection(s.snapshot()),"result":displayed,"error":response.err().map(|e|format!("{e:?}")),"acknowledgement":ack});
        output
    }
}
#[cfg(test)]
pub(super) fn bootstrap(io: Native) -> IoResult<()> {
    let mut host = OpenSession::open(io)?;
    let b = host.activation_binding();
    for command in ["submit", "complete"] {
        let response=host.execute(&json!({"id":uuid::Uuid::new_v4().to_string(),"command":command,"node_id":request().0["command"]["envelope"]["command"]["node_id"],"x":13,"y":14,"expected_owner_epoch":b["owner_epoch"],"expected_attachment_generation":number(&b["owner"]["attachment_generation"])?}));
        if !response["error"].is_null() {
            return Err(std::io::Error::other(response.to_string()));
        }
        println!("{response}");
    }
    Ok(())
}
#[cfg(test)]
pub(super) fn run(io: Native) -> IoResult<()> {
    let mut session = OpenSession::open(io)?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut bytes = Vec::new();
        let count = input
            .by_ref()
            .take(1_048_577)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            break;
        }
        if bytes.len() > 1_048_576 || !bytes.ends_with(b"\n") {
            return Err(invalid());
        }
        let mut req: Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        req["expected_owner_epoch"] =
            json!(session.session.snapshot().binding.owner_epoch.to_string());
        req["expected_attachment_generation"] = json!(
            session.session.snapshot().binding.owner["attachment_generation"]
                .as_str()
                .ok_or_else(invalid)?
                .parse::<u64>()
                .map_err(|_| invalid())?
        );
        let output = session.execute(&req);
        println!(
            "\nPHOTARA_PS3 {}",
            serde_json::to_string(&output).map_err(|_| invalid())?
        );
        std::io::stdout().flush()?;
        if output["snapshot"]["closed"] == true {
            break;
        }
    }
    Ok(())
}
