//! Private stdio adapter for the explicitly controlled disposable-image lab.
//! No production lease constructor, public filesystem path, or network service.
use super::{
    BTreeMap, EffectFailure, IoResult, Native, NativeLease, PlannerInputs, RepeatablePlan,
    RestartContext, Timing, Value, VerifiedOriginal, attempt, barrier, budget, canonical, execute,
    frames, hash, identity, invalid, journal, json, number, package, read, record_identity,
    reference, regular, request, restart, snapshot, support, uncertain, v1_3,
};
use crate::package::v1_3::repeatable::checkpoint;
use crate::package::v1_3::repeatable::session::{
    AcceptedCoordinate, Presentation, Session, SessionBinding, SessionError, SessionFailure,
    SessionIo, SessionSnapshot,
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
fn lifecycle_controls(current: &super::OriginalPackage) -> IoResult<BTreeMap<String, Vec<u8>>> {
    let overlay = &current.commit["root_set"]["inventory"];
    let bytes = current
        .loose
        .get(overlay["sha256"].as_str().ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    let value =
        package::parse_canonical_json(bytes, support::limits().json).map_err(|_| invalid())?;
    let mut controls = BTreeMap::new();
    for reference in
        std::iter::once(overlay).chain(value["controls"].as_array().ok_or_else(invalid)?)
    {
        let sha = reference["sha256"].as_str().ok_or_else(invalid)?;
        let bytes = current.loose.get(sha).ok_or_else(invalid)?;
        if reference
            != &json!({"kind":"json","sha256":hash(bytes),"byte_length":bytes.len().to_string()})
        {
            return Err(invalid());
        }
        controls.insert(sha.to_owned(), bytes.clone());
    }
    Ok(controls)
}

#[test]
fn session_tests_native_cold_selection_controls_are_exact() {
    let fixture = support::Fixture::load("integrated");
    let mut current = super::package(&fixture);
    let selected = lifecycle_controls(&current).unwrap();
    assert_eq!(selected, current.loose);
    let unrelated = canonical(&json!({"unselected":true}));
    current.loose.insert(hash(&unrelated), unrelated);
    assert_eq!(lifecycle_controls(&current).unwrap(), selected);
    let sha = current.commit["root_set"]["inventory"]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    current.loose.get_mut(&sha).unwrap().push(b' ');
    assert!(lifecycle_controls(&current).is_err());
}
// One optional live checkpoint capability. Its immutable inputs are compared
// against fresh bytes and observations; authority and durability are never cached.
struct SnapshotView {
    key: String,
    selector: Vec<u8>,
    journal: Vec<u8>,
    checkpoint: std::sync::Arc<checkpoint::ActivatedCheckpoint>,
    records: std::sync::Arc<ObservedRecords>,
    reads: Vec<(RecordRead, String)>,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RecordRead {
    Record(String, usize),
    Frame(package::PackageUuid, u64, usize),
}
struct ObservedRecords {
    source: std::sync::RwLock<std::sync::Arc<dyn checkpoint::CheckpointRecords>>,
    reads: std::sync::Mutex<Option<BTreeMap<RecordRead, String>>>,
    overflow: std::sync::atomic::AtomicBool,
}
impl ObservedRecords {
    fn capture(&self, key: RecordRead, bytes: &[u8]) -> Result<(), package::PackageError> {
        let mut reads = self
            .reads
            .lock()
            .map_err(|_| package::PackageError::Integrity)?;
        if let Some(map) = reads.as_mut() {
            if map.len() >= budget().max_objects && !map.contains_key(&key) {
                *reads = None;
                self.overflow
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            } else if map
                .insert(key, hash(bytes))
                .is_some_and(|prior| prior != hash(bytes))
            {
                return Err(package::PackageError::Integrity);
            }
        }
        Ok(())
    }
    fn recheck(
        &self,
        reads: &[(RecordRead, String)],
        source: std::sync::Arc<dyn checkpoint::CheckpointRecords>,
    ) -> Result<(), package::PackageError> {
        for (key, expected) in reads {
            let bytes = match key {
                RecordRead::Record(reference, maximum) => source.record(
                    &serde_json::from_str(reference).map_err(|_| package::PackageError::Record)?,
                    *maximum,
                )?,
                RecordRead::Frame(stream, offset, length) => {
                    source.frame(*stream, *offset, *length)?
                }
            };
            if hash(&bytes) != *expected {
                return Err(package::PackageError::Integrity);
            }
        }
        *self
            .source
            .write()
            .map_err(|_| package::PackageError::Integrity)? = source;
        Ok(())
    }
}
impl checkpoint::CheckpointRecords for ObservedRecords {
    fn record(&self, reference: &Value, maximum: usize) -> Result<Vec<u8>, package::PackageError> {
        let bytes = self
            .source
            .read()
            .map_err(|_| package::PackageError::Integrity)?
            .record(reference, maximum)?;
        self.capture(
            RecordRead::Record(
                String::from_utf8(canonical(reference))
                    .map_err(|_| package::PackageError::Record)?,
                maximum,
            ),
            &bytes,
        )?;
        Ok(bytes)
    }
    fn frame(
        &self,
        stream: package::PackageUuid,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, package::PackageError> {
        let bytes = self
            .source
            .read()
            .map_err(|_| package::PackageError::Integrity)?
            .frame(stream, offset, length)?;
        self.capture(RecordRead::Frame(stream, offset, length), &bytes)?;
        Ok(bytes)
    }
}
#[test]
fn session_tests_snapshot_observation_refresh_is_exact() {
    use checkpoint::CheckpointRecords;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Source(Vec<u8>, Arc<AtomicUsize>);
    impl CheckpointRecords for Source {
        fn record(&self, _: &Value, maximum: usize) -> Result<Vec<u8>, package::PackageError> {
            self.1.fetch_add(1, Ordering::Relaxed);
            if self.0.len() > maximum {
                return Err(package::PackageError::Limit);
            }
            Ok(self.0.clone())
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
    let count = Arc::new(AtomicUsize::new(0));
    let source = || Arc::new(Source(vec![1, 2, 3], count.clone())) as Arc<dyn CheckpointRecords>;
    let observed = ObservedRecords {
        source: std::sync::RwLock::new(source()),
        reads: std::sync::Mutex::new(Some(BTreeMap::new())),
        overflow: std::sync::atomic::AtomicBool::new(false),
    };
    let r = json!({"test":"record"});
    observed.record(&r, 3).unwrap();
    let reads = observed
        .reads
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    for _ in 0..2 {
        observed.recheck(&reads, source()).unwrap();
    }
    assert_eq!(count.load(Ordering::Relaxed), 3);
    assert!(
        observed
            .recheck(&reads, Arc::new(Source(vec![1, 2, 4], count.clone())))
            .is_err()
    );
    assert_eq!(observed.record(&r, 3).unwrap(), vec![1, 2, 3]);
    assert!(observed.record(&r, 2).is_err());
}
struct Host {
    io: Native,
    lease: NativeLease,
    binding: SessionBinding,
    seed: Option<PlannerInputs>,
    snapshot: std::cell::RefCell<Option<SnapshotView>>,
}
impl Host {
    fn snapshot_mode(&self) -> bool {
        self.io.journal_header["format_version"] == 2
    }
    fn snapshot_capability(
        &self,
        journal: &journal::Journal,
    ) -> Result<std::sync::Arc<checkpoint::ActivatedCheckpoint>, SessionError> {
        let reader = support::limits();
        let current = self.io.current().map_err(session_error)?;
        let observed = snapshot(&self.io.base, &current);
        let identity = identity(&current);
        let context = self.context(&observed, &identity, &reader);
        let key = restart::CompiledPlans::snapshot_key(&current, &context);
        let selector = self
            .io
            .checkpoint_selector(&self.lease)
            .map_err(session_error)?;
        let bytes = self.read_journal()?;
        if journal.exact_bytes() != bytes {
            return Err(SessionError::InvalidRequest);
        }
        let mut cache = self.snapshot.borrow_mut();
        if let Some(saved) = cache.as_ref()
            && saved.key == key
            && saved.selector == selector
            && bytes.starts_with(&saved.journal)
        {
            saved.records.recheck(
                &saved.reads,
                std::sync::Arc::new(
                    self.io
                        .activation_records(&self.lease)
                        .map_err(session_error)?,
                ),
            )?;
            if self
                .io
                .checkpoint_selector(&self.lease)
                .map_err(session_error)?
                != selector
            {
                return Err(SessionError::InvalidRequest);
            }
            return Ok(saved.checkpoint.clone());
        }
        let limits = checkpoint::ActivationLimits {
            journal: self.io.journal_bounds(),
            frames: frames(),
            reader: &reader,
            planner: budget(),
        };
        let mut state = self.io.read_checkpoint_state(&self.lease, limits)?;
        if state.selector != selector
            || state.segment != bytes
            || restart::CompiledPlans::snapshot_key(state.current.shared_package(), &context) != key
        {
            return Err(SessionError::InvalidRequest);
        }
        let records = std::sync::Arc::new(ObservedRecords {
            source: std::sync::RwLock::new(state.records),
            reads: std::sync::Mutex::new(Some(BTreeMap::new())),
            overflow: std::sync::atomic::AtomicBool::new(false),
        });
        state.records = records.clone();
        let checkpoint = std::sync::Arc::new(checkpoint::reopen_state(
            state,
            &self.io.checkpoint_identity()?,
            limits,
        )?);
        let reads = records
            .reads
            .lock()
            .map_err(|_| SessionError::InvalidRequest)?
            .take()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        // Optional metadata is bounded; overflow changes speed only.
        if records.overflow.load(std::sync::atomic::Ordering::Relaxed) {
            *cache = None;
        } else {
            *cache = Some(SnapshotView {
                key,
                selector,
                journal: bytes,
                checkpoint: checkpoint.clone(),
                records,
                reads,
            });
        }
        Ok(checkpoint)
    }
    fn snapshot_view<'a>(
        &self,
        journal: &'a journal::Journal,
    ) -> Result<journal::VerifiedSession<'a>, SessionError> {
        Ok(self.snapshot_capability(journal)?.verify_session(
            journal,
            frames(),
            &support::limits(),
            budget(),
        )?)
    }
    fn context<'a>(
        &'a self,
        observed: &'a super::Snapshot<'_>,
        id: &'a v1_3::SelectionIdentity,
        reader: &'a v1_3::ReaderLimits,
    ) -> RestartContext<'a, super::Snapshot<'a>> {
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
        let _timing = Timing::new("header-original");
        if self.snapshot_mode() {
            let journal = self.io.checkpoint()?;
            return self
                .snapshot_capability(&journal)
                .map(|p| p.package().shared_clone())
                .map_err(|e| std::io::Error::other(format!("snapshot header: {e:?}")));
        }
        let current = self.io.current()?;
        let j = self.io.checkpoint()?;
        let raw = if let Some(first) = j
            .records()
            .iter()
            .find(|r| r.value()["kind"] == "CheckpointIntent")
        {
            self.io
                .compiled
                .borrow()
                .reconstruct(&current, &first.value()["body"]["original"], budget())
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
        restart::verify_original_prefix_cached(
            &raw,
            &self.context(&observed, &id, &reader),
            &mut self.io.compiled.borrow_mut(),
        )
        .map_err(|_| invalid())
    }
    fn original(&self) -> IoResult<VerifiedOriginal> {
        if self.snapshot_mode() {
            return self.header_original();
        }
        let current = self.io.current()?;
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        let ctx = self.context(&observed, &id, &reader);
        let directory = ctx.directory;
        if let Some(original) = self
            .io
            .compiled
            .borrow_mut()
            .current_original(&current, &ctx)
            .map_err(|_| invalid())?
        {
            return Ok(original);
        }
        if current.head == self.io.journal_header["base_head"] {
            v1_3::reader::verify_shared_original(
                current.clone(),
                id,
                &observed,
                &self.io.base.registration,
                directory,
                frames(),
                &reader,
            )
            .map_err(|_| invalid())
        } else {
            let plan =
                restart::restore_plan_cached(&current, &ctx, &mut self.io.compiled.borrow_mut())
                    .map_err(|_| invalid())?;
            v1_3::reader::verify_operation_original(
                current.clone(),
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
    fn snapshot_plan_key(&self, current: &super::OriginalPackage) -> IoResult<String> {
        let observed = snapshot(&self.io.base, current);
        let id = identity(current);
        let reader = support::limits();
        let context = self.context(&observed, &id, &reader);
        Ok(hash(&canonical(&json!([
            restart::CompiledPlans::snapshot_key(current, &context),
            self.io.checkpoint_selector(&self.lease)?
        ]))))
    }
    fn snapshot_origin(
        &self,
        current: &super::OriginalPackage,
        original: &Value,
    ) -> IoResult<(VerifiedOriginal, checkpoint::ActivatedCheckpoint)> {
        let raw = self
            .io
            .compiled
            .borrow()
            .reconstruct(current, original, budget())
            .map_err(|_| invalid())?;
        let reader = support::limits();
        let limits = checkpoint::ActivationLimits {
            journal: self.io.journal_bounds(),
            frames: frames(),
            reader: &reader,
            planner: budget(),
        };
        let source = self
            .io
            .checkpoint_source(&self.lease, limits)
            .map_err(|_| invalid())?;
        let old = self
            .io
            .verify_checkpoint_prefix(&source, &raw, current, limits)
            .map_err(|_| invalid())?;
        let mut state = self
            .io
            .read_checkpoint_state_at(&self.lease, limits, Some(old.shared_clone()))
            .map_err(|_| invalid())?;
        let journal =
            journal::Journal::read(&state.segment, &state.header, self.io.journal_bounds())
                .map_err(|_| invalid())?;
        if let Some(index) = journal.records().iter().position(|r| {
            r.value()["kind"] == "CheckpointIntent" && r.value()["body"]["original"] == *original
        }) {
            let end = if index == 0 {
                journal::Journal::create(&state.header, self.io.journal_bounds())
                    .map_err(|_| invalid())?
                    .len()
            } else {
                journal
                    .record_end(&journal.records()[index - 1].value()["record_id"])
                    .ok_or_else(invalid)?
            };
            state.segment.truncate(end);
        }
        let base = checkpoint::reopen_state(
            state,
            &self.io.checkpoint_identity().map_err(|_| invalid())?,
            limits,
        )
        .map_err(|_| invalid())?;
        Ok((old, base))
    }
    fn persisted(&self, operation: &Value) -> IoResult<RepeatablePlan> {
        let _timing = Timing::new("persisted-original");
        let current = if self.snapshot_mode() {
            self.io.current_candidate()?
        } else {
            self.io.current()?
        };
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
        if self.snapshot_mode() {
            let key = self.snapshot_plan_key(&current)?;
            if let Some(plan) = self
                .io
                .compiled
                .borrow_mut()
                .snapshot_plan(
                    &key,
                    &current,
                    &b["original"],
                    budget(),
                    self.io.journal_bounds().max_records,
                )
                .map_err(|_| invalid())?
            {
                if plan.prepared.receipt != b["operation_receipt"]
                    || plan.original()["request"]["intent"] != reference(&b["semantic_intent"])
                {
                    return Err(invalid());
                }
                return Ok(plan);
            }
            let (old, base) = self.snapshot_origin(&current, &b["original"])?;
            return base
                .restore_operation(
                    &old,
                    &b["semantic_intent"],
                    &b["operation_receipt"],
                    &canonical(&b["original"]),
                    budget(),
                )
                .map_err(|_| invalid());
        }
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        restart::restore_intent_plan_cached(
            &current,
            &b["original"],
            &b["semantic_intent"],
            &b["operation_receipt"],
            &self.context(&observed, &id, &reader),
            &mut self.io.compiled.borrow_mut(),
        )
        .map_err(|_| invalid())
    }
    fn prepare_plan(&mut self, plan: &RepeatablePlan) -> IoResult<()> {
        if !self.snapshot_mode() {
            let base = self.header_original()?;
            return self.io.prepare_checkpoint_with(
                plan,
                Some((&base, self.seed.as_ref().expect("initialized header seed"))),
            );
        }
        let journal = self.io.checkpoint()?;
        let current = self.io.current_candidate()?;
        let checkpoint = if current.head == plan.old.head && current.commit == plan.old.commit {
            self.snapshot_capability(&journal).map_err(|_| invalid())?
        } else {
            std::sync::Arc::new(self.snapshot_origin(&current, plan.original())?.1)
        };
        let view = checkpoint
            .verify_session(&journal, frames(), &support::limits(), budget())
            .map_err(|_| invalid())?;
        let prior = journal.records().iter().find(|r| {
            r.value()["kind"] == "CheckpointIntent"
                && r.value()["body"]["operation_receipt"]["operation_id"]
                    == plan.prepared.receipt["operation_id"]
        });
        let intent = view
            .prepare_snapshot_intent(
                plan,
                record_identity(
                    prior.map(journal::JournalRecord::value),
                    self.io.scope.generation,
                )?,
                self.io.journal_bounds(),
            )
            .map_err(|_| invalid())?;
        let mut planned = journal.exact_bytes().to_vec();
        planned.extend_from_slice(intent.bytes());
        let next = journal::Journal::read_reusing(
            &planned,
            &self.io.journal_header,
            self.io.journal_bounds(),
            &journal,
        )
        .map_err(|_| invalid())?;
        let intent_id = package::PackageUuid::parse(
            intent.record().value()["record_id"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(|_| invalid())?;
        let prior = next.records().iter().find(|r| {
            r.value()["kind"] == "CheckpointReceipt"
                && r.value()["body"]["operation_receipt"]["operation_id"]
                    == plan.prepared.receipt["operation_id"]
        });
        let receipt_id = record_identity(
            prior.map(journal::JournalRecord::value),
            self.io.scope.generation,
        )?;
        let completion = next
            .snapshot_receipt_plan(plan, intent_id, receipt_id, self.io.journal_bounds())
            .map_err(|_| invalid())?;
        let prefix = journal::Journal::read_reusing(
            &planned[..next
                .record_end(&intent.record().value()["record_id"])
                .ok_or_else(invalid)?],
            &self.io.journal_header,
            self.io.journal_bounds(),
            &next,
        )
        .map_err(|_| invalid())?;
        let worst = prefix
            .snapshot_receipt_plan(
                plan,
                intent_id,
                journal::JournalRecordIdentity {
                    session_generation: u64::MAX,
                    ..receipt_id
                },
                self.io.journal_bounds(),
            )
            .map_err(|_| invalid())?;
        self.io.install_checkpoint_plan(
            plan,
            &intent,
            completion,
            planned.len(),
            worst.bytes().len(),
        )
    }
    fn execute_original(&mut self, operation: &Value) -> IoResult<()> {
        let _timing = Timing::new("execute-original-total");
        let plan = self
            .persisted(operation)
            .map_err(|e| std::io::Error::other(format!("restore original: {e}")))?;
        self.io.allow(&plan);
        self.io.reserved_journal_extra = 0;
        self.prepare_plan(&plan)?;
        let _execute_timing = Timing::new("executor-effects-and-checks");
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
    fn prepare_snapshot_lifecycle(&mut self, current: &VerifiedOriginal) -> IoResult<()> {
        // A cold Native has no prior executor allow-set. Admit only the exact
        // controls of this independently verified selection before accounting
        // its namespace; candidate-only coexisting controls remain refused.
        self.io.allowed = lifecycle_controls(&current.package)?;
        self.io.commits = BTreeMap::from([(
            hash(&canonical(&current.package.commit)),
            canonical(&current.package.commit),
        )]);
        let ceiling = super::charges(&self.io.base, &current.package);
        let footprint = self.io.footprint(&ceiling)?;
        let used = super::checkpoint_native::standing_charge(&footprint).map_err(|_| invalid())?;
        let file = regular(&self.io.evidence, &self.io.journal_name, false)?;
        let usage = v1_3::native_profile::measure_allocation(&file).map_err(|_| invalid())?;
        let charge = v1_3::native_profile::check_charge(
            &[usage],
            self.io.base.registration.charge_unit,
            self.io.base.registration.standing_control,
        )
        .map_err(|_| invalid())?;
        self.io.journal_capacity = self
            .io
            .base
            .registration
            .standing_control
            .checked_sub(used)
            .and_then(|n| n.checked_add(charge.rounded_required_charge))
            .ok_or_else(invalid)?;
        self.io.journal_high_water = charge.allocated_bytes;
        self.io.ceiling = Some(ceiling);
        self.io.check_effect_footprint()
    }
    fn current_target(&self) -> Result<AcceptedCoordinate, SessionError> {
        let j = self.io.checkpoint().map_err(session_error)?;
        let base = self.header_original().map_err(session_error)?;
        let s = self.verify_view(
            &j,
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
    fn archived_retry(&self, req: &Value) -> Result<Option<(Value, String)>, SessionError> {
        if !self.snapshot_mode() {
            return Ok(None);
        }
        let operation = req["id"].as_str().ok_or(SessionError::InvalidRequest)?;
        let journal = self.io.checkpoint().map_err(session_error)?;
        if journal.records().iter().any(|r| {
            r.value()["kind"] == "Mutation"
                && r.value()["body"]["operation_receipt"]["operation_id"] == operation
        }) {
            return Ok(None);
        }
        self.authorize(&self.binding, None)?;
        let Some((record, checksum)) = self.snapshot_capability(&journal)?.retained_mutation(
            operation,
            frames(),
            &support::limits(),
        )?
        else {
            return Ok(None);
        };
        check_archived_request(req, &record, &self.binding.owner)?;
        self.authorize(&self.binding, Some(&record["body"]["semantic_intent"]))?;
        Ok(Some((record, checksum)))
    }
}
impl SessionIo for Host {
    fn maintain_checkpoint(
        &mut self,
        scheduled: bool,
        prior: &Value,
    ) -> Result<Option<(VerifiedOriginal, Value)>, SessionError> {
        if self.io.checkpoint_namespace.is_none() {
            return Ok(None);
        }
        self.authorize(&self.binding, None)?;
        let pending = self.io.pending_turnover().map_err(session_error)?;
        if pending {
            self.io.activate_checkpoint(&self.lease)?;
        }
        self.io.load_selected_journal().map_err(session_error)?;
        let journal = self.io.checkpoint().map_err(session_error)?;
        let count = journal
            .records()
            .iter()
            .filter(|r| r.value()["kind"] == "Mutation")
            .count();
        // Disposable scheduling batch only: neither acceptance nor retention
        // limits change. An initial Saved legacy segment is adopted once.
        let due = !pending
            && scheduled
            && count
                >= if journal.header()["format_version"] == 1 {
                    1
                } else {
                    8
                };
        if due {
            self.io.activate_checkpoint(&self.lease)?;
            self.io.load_selected_journal().map_err(session_error)?;
        }
        if *prior == self.io.journal_header {
            return Ok(None);
        }
        *self.snapshot.borrow_mut() = None;
        let journal = self.io.checkpoint().map_err(session_error)?;
        let checkpoint = self.snapshot_capability(&journal)?;
        Ok(Some((
            checkpoint.package().shared_clone(),
            journal.header().clone(),
        )))
    }
    fn activation_checkpoint(
        &self,
        journal: &journal::Journal,
        receipt: &Value,
    ) -> Result<Value, SessionError> {
        self.authorize(&self.binding, None)?;
        let checkpoint = self.snapshot_capability(journal)?;
        let records = self.io.activation_records(&self.lease)?;
        let evidence =
            checkpoint.activation_checkpoint(journal, receipt, &records, &support::limits())?;
        self.authorize(&self.binding, None)?;
        Ok(evidence)
    }
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
        read(&self.io.evidence, &self.io.journal_name).map_err(session_error)
    }
    fn verify<'a>(
        &self,
        j: &'a journal::Journal,
    ) -> Result<journal::VerifiedJournal<'a>, SessionError> {
        let _timing = Timing::new("journal-verify");
        let current = self.io.current().map_err(session_error)?;
        let observed = snapshot(&self.io.base, &current);
        let id = identity(&current);
        let reader = support::limits();
        Ok(j.verify_cached(
            &current,
            &self.context(&observed, &id, &reader),
            &mut self.io.compiled.borrow_mut(),
        )?)
    }
    fn verify_session<'a>(
        &self,
        verified: &journal::VerifiedJournal<'a>,
        base: &VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: super::super::Limits,
    ) -> Result<journal::VerifiedSession<'a>, SessionError> {
        let _timing = Timing::new("session-replay");
        Ok(verified.verify_session_cached(
            base,
            inputs,
            limits,
            &mut self.io.compiled.borrow_mut(),
        )?)
    }
    fn verify_view<'a>(
        &self,
        journal: &'a journal::Journal,
        base: &VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: super::super::Limits,
    ) -> Result<journal::VerifiedSession<'a>, SessionError> {
        if self.snapshot_mode() {
            self.snapshot_view(journal)
        } else {
            let verified = self.verify(journal)?;
            self.verify_session(&verified, base, inputs, limits)
        }
    }
    fn selected_head(&self) -> Result<Value, SessionError> {
        Ok(self.io.current().map_err(session_error)?.head)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One serialized admission keeps private sizing before all reservation and effects"
    )]
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
                self.prepare_plan(&p).map_err(session_error)?;
            }
            // Exact original retry has no new capacity or physical admission.
            barrier(
                &regular(&self.io.evidence, &self.io.journal_name, true).map_err(session_error)?,
            )
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
        let p = if self.snapshot_mode() {
            let journal = self.io.checkpoint().map_err(session_error)?;
            let checkpoint = self.snapshot_capability(&journal)?;
            let mut raw =
                serde_json::to_value(self.io.attempt(&original, n).map_err(session_error)?)
                    .map_err(session_error)?;
            raw["planner"]["codec"] = json!("photara.codec.ps2-repeatable-layout-v4");
            for corridor in raw["planner"]["corridors"]
                .as_array_mut()
                .ok_or(SessionError::InvalidRequest)?
            {
                corridor
                    .as_object_mut()
                    .ok_or(SessionError::InvalidRequest)?
                    .remove("original_sha256");
                corridor["base_head_sha256"] = json!(hash(&canonical(&original.package.head)));
            }
            crate::package::v1_3::repeatable::native_sizing::compile(
                raw,
                |raw| checkpoint.compile_operation(&original, intent, receipt, raw, budget()),
                |plan| &plan.files,
            )?
            .0
        } else {
            let raw = serde_json::to_value(self.io.attempt(&original, n).map_err(session_error)?)
                .map_err(session_error)?;
            crate::package::v1_3::repeatable::native_sizing::compile(
                raw,
                |raw| {
                    self.io.compiled.borrow_mut().compile(
                        &original,
                        intent,
                        receipt,
                        serde_json::from_value(raw.clone()).map_err(|_| "sizing attempt")?,
                        budget(),
                        self.io.journal_bounds().max_records,
                    )
                },
                |plan| &plan.files,
            )
            .map_err(session_error)?
            .0
        };
        if self.snapshot_mode() {
            let key = self
                .snapshot_plan_key(original.shared_package())
                .map_err(session_error)?;
            self.io.compiled.borrow_mut().remember_snapshot_plan(
                &key,
                &p,
                budget(),
                self.io.journal_bounds().max_records,
            )?;
        }
        self.reserve_mutation(mutation)?;
        self.io.allow(&p);
        self.prepare_plan(&p).map_err(session_error)
    }
    fn append(&mut self, a: &journal::JournalAppend) -> Result<(), EffectFailure> {
        if a.existing() {
            return uncertain(
                barrier(
                    &regular(&self.io.evidence, &self.io.journal_name, true)
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
        if self.snapshot_mode() {
            let view = self
                .snapshot_view(&j)
                .map_err(|_| EffectFailure::OutcomeUnknown)?;
            if view.checkpoint_target()["accepted_frame"] == *frame {
                return uncertain(
                    barrier(
                        &regular(&self.io.evidence, &self.io.journal_name, true)
                            .map_err(|_| EffectFailure::OutcomeUnknown)?,
                    )
                    .and_then(|()| barrier(&self.io.evidence)),
                );
            }
        }
        if j.records().iter().any(|r| {
            r.value()["kind"] == "CheckpointReceipt"
                && r.value()["body"]["operation_receipt"]["operation_id"] == *operation
        }) {
            if self.snapshot_mode() {
                self.snapshot_view(&j)
                    .map_err(|_| EffectFailure::OutcomeUnknown)?;
            } else {
                self.verify(&j).map_err(|_| EffectFailure::OutcomeUnknown)?;
            }
            return uncertain(
                barrier(
                    &regular(&self.io.evidence, &self.io.journal_name, true)
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
        if self.snapshot_mode() {
            self.snapshot_view(&j)?;
        } else {
            self.verify(&j)?;
        }
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
        barrier(&regular(&self.io.evidence, &self.io.journal_name, true).map_err(session_error)?)
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
    result_from(s.presentation()?)
}
fn result_from(view: Presentation) -> Result<Value, SessionError> {
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
fn check_archived_request(req: &Value, record: &Value, owner: &Value) -> Result<(), SessionError> {
    let body = &record["body"];
    if record["kind"] != "Mutation"
        || body["operation_receipt"]["operation_id"] != req["id"]
        || body["owner"] != *owner
        || !matches_request(req, &body["semantic_intent"], &body["action"])
    {
        return Err(SessionError::InvalidRequest);
    }
    Ok(())
}
#[test]
fn session_tests_native_archived_retry_matches_exact_request_and_owner() {
    let (intent, receipt) = request();
    let command = &intent["command"]["envelope"]["command"];
    let owner = json!({"attachment_id":"registered-original-stream","attachment_generation":"1","principal":receipt["provenance"]["principal"]});
    let record = json!({"kind":"Mutation","body":{"semantic_intent":intent,"operation_receipt":receipt,"owner":owner,"action":{"kind":"edit"}}});
    let req = json!({"id":receipt["operation_id"],"command":"submit","node_id":command["node_id"],"x":command["x"],"y":command["y"]});
    assert!(check_archived_request(&req, &record, &owner).is_ok());
    for field in ["x", "y", "node_id", "id", "command"] {
        let mut changed = req.clone();
        changed[field] = json!("different");
        assert!(check_archived_request(&changed, &record, &owner).is_err());
    }
    let mut changed_owner = owner.clone();
    changed_owner["attachment_id"] = json!("new-segment-is-not-the-attachment");
    assert!(check_archived_request(&req, &record, &changed_owner).is_err());
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
    pub(super) fn open(mut io: Native) -> IoResult<Self> {
        if !io.session_mode {
            return Err(invalid());
        }
        if io.pending_turnover()? {
            let candidate = io.current_candidate()?;
            let lease = io.lease(&candidate)?;
            io.activate_checkpoint(&lease)
                .map_err(|e| std::io::Error::other(format!("maintenance recovery: {e:?}")))?;
        }
        io.load_selected_journal()?;
        let current = if io.journal_header["format_version"] == 2 {
            io.current_candidate()?
        } else {
            io.current()?
        };
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
            owner: json!({"attachment_id":io.registered_journal_header["stream_id"],"attachment_generation":registration["generation"].as_u64().ok_or_else(invalid)?.to_string(),"principal":io.local_provenance.as_ref().unwrap_or(&request().1["provenance"])["principal"]}),
        };
        let mut host = Host {
            io,
            lease,
            binding: binding.clone(),
            seed: None,
            snapshot: std::cell::RefCell::new(None),
        };
        if host.snapshot_mode() {
            // Reconcile exact accepted originals before cold selection needs
            // their missing receipt; execute revalidates stages and barriers.
            let journal = host.io.checkpoint()?;
            for record in journal.records() {
                if record.value()["kind"] == "Mutation" {
                    let operation = &record.value()["body"]["operation_receipt"]["operation_id"];
                    if !journal.records().iter().any(|r| {
                        r.value()["kind"] == "CheckpointReceipt"
                            && r.value()["body"]["operation_receipt"]["operation_id"] == *operation
                    }) {
                        host.execute_original(operation)?;
                    }
                }
            }
        }
        let base = host.header_original()?;
        if host.snapshot_mode() {
            host.prepare_snapshot_lifecycle(&base)?;
        }
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
    #[cfg(test)]
    pub(super) fn checkpoint_turnover(mut self) -> IoResult<Self> {
        let flush = self
            .session
            .begin_flush("flush", identity_new(self.generation))
            .map_err(|e| std::io::Error::other(format!("maintenance flush: {e:?}")))?;
        self.session
            .finish_flush(&flush)
            .map_err(|e| std::io::Error::other(format!("maintenance flush: {e:?}")))?;
        let mut host = self.session.into_io();
        host.io
            .activate_checkpoint(&host.lease)
            .map_err(|e| std::io::Error::other(format!("maintenance activation: {e:?}")))?;
        let Host { io, lease, .. } = host;
        drop(lease);
        Self::open(io)
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
        s.begin_response();
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
                    if let Some((original, checksum)) = s.io_mut().archived_retry(req)? {
                        let body = &original["body"];
                        // Exact retry acknowledges historical acceptance. It
                        // neither submits again nor changes current Saved.
                        ack = json!({"binding":binding_dto(&s.snapshot().binding),"operation_id":id,
                            "accepted":projected(&AcceptedCoordinate{coordinate:body["result"].clone(),mutation:Some(json!({"record_id":original["record_id"],"record_checksum":checksum})),accepted_frame:Some(body["accepted_frame"].clone())}),
                            "operation_receipt_sha256":hash(&canonical(&body["operation_receipt"]))});
                        return Ok(());
                    }
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
                        let mut provenance = s
                            .io_mut()
                            .io
                            .local_provenance
                            .clone()
                            .unwrap_or_else(|| request().1["provenance"].clone());
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
                "maintain" => s.maintain()?,
                _ => return Err(SessionError::InvalidRequest),
            }
            Ok(())
        })();
        let displayed = if s.snapshot().closed && response.is_ok() {
            // Close already has a fully verified final snapshot. No extra graph
            // inspection after the coordinator commits its closed state.
            Value::Null
        } else if let Ok(value) = s.finish_response().and_then(result_from) {
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
