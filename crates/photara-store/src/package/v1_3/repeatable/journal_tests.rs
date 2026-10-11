//! Approved framing exercised with the existing integrated repeatable corpus.
use super::super::journal::*;
use super::*;

pub(super) fn limits() -> JournalLimits {
    JournalLimits {
        json: support::limits().json,
        max_file_bytes: 262_144,
        max_records: 16,
        max_work_bytes: 262_144,
    }
}
pub(super) fn record(n: u64) -> JournalRecordIdentity {
    JournalRecordIdentity {
        record_id: package::PackageUuid::parse(&uid(n)).unwrap(),
        session_generation: 1,
    }
}
pub(super) fn header(p: &OriginalPackage) -> Value {
    json!({"format_version":1,"stream_id":uid(90_001),"journal_id":"50000000-0000-4000-8000-000000000090","device_id":uid(90_002),"project_id":p.manifest["project_id"],"library_id":p.commit["root_set"]["library_id"],"incarnation_id":"96000000-0000-4000-8000-000000000001","bootstrap_sha256":hash(&encode(&p.manifest)),"base_head":p.head,"base_package_revision":p.commit["package_revision"]})
}
pub(super) fn context<'a>(
    observed: &'a support::Fixture,
    id: &'a v1_3::SelectionIdentity,
    reader: &'a v1_3::ReaderLimits,
) -> RestartContext<'a, support::Fixture> {
    RestartContext {
        inspection: observed,
        registration: &observed.registration,
        directory: v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        identity: id,
        frames: frames(),
        reader,
        planner: budget(),
        max_originals: 4,
    }
}
fn append_raw(prefix: &[u8], previous: &[u8], value: &Value) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let raw = encode(value);
    let len = u32::try_from(raw.len()).unwrap().to_le_bytes();
    let mut sha = Sha256::new();
    sha.update(previous);
    sha.update(len);
    sha.update(&raw);
    let mut out = prefix.to_vec();
    out.extend(len);
    out.extend(raw);
    out.extend(sha.finalize());
    out
}
// Same original corpus, comparing authenticated full-index reads with the
// unchanged scoped point lookup. No live file or authority is involved.
fn audited_locator_equivalence(p: &OriginalPackage) {
    use v1_3::PhysicalRecords as _;
    struct Counted<'a> {
        packs: std::collections::BTreeMap<package::PackageUuid, v1_3::PackedAllocation<'a>>,
        reads: std::cell::Cell<usize>,
    }
    impl v1_3::PhysicalRecords for Counted<'_> {
        fn resolve(
            &self,
            r: &v1_3::PhysicalRef,
            kind: v1_3::FrameKind,
        ) -> Result<Value, package::PackageError> {
            self.reads.set(self.reads.get() + 1);
            self.packs.resolve(r, kind)
        }
    }
    let packs = p
        .allocations
        .iter()
        .map(|(id, a)| {
            let id = package::PackageUuid::parse(id).unwrap();
            (
                id,
                v1_3::PackedAllocation::open(
                    id,
                    serde_json::from_value(json!(a.arena)).unwrap(),
                    &a.bytes,
                    frames(),
                )
                .unwrap(),
            )
        })
        .collect();
    let provider = Counted {
        packs,
        reads: std::cell::Cell::new(0),
    };
    let id = identity(p);
    let project = id.project;
    let limits = support::limits();
    let envelope = v1_3::SelectedEnvelope::parse(
        &encode(&p.manifest),
        &encode(&p.head),
        &encode(&p.commit),
        id,
        limits.json,
    )
    .unwrap();
    let state = envelope
        .open_role(&provider, v1_3::SelectedRole::Active, limits.tree)
        .unwrap();
    let (indexed, audit) =
        v1_3::Locator::audited(&provider, project, state.locator.clone(), limits.tree).unwrap();
    let scoped = v1_3::Locator::new(&provider, project, state.locator.clone(), limits.tree);
    provider.reads.set(0);
    let mut values = Vec::new();
    for entry in &audit.entries {
        let object: package::ObjectRef = serde_json::from_value(entry["object"].clone()).unwrap();
        let membership = serde_json::from_value(entry["membership"].clone()).unwrap();
        values.push(indexed.json(&object, membership).unwrap());
    }
    let indexed_reads = provider.reads.get();
    assert_eq!(indexed_reads, audit.entries.len());
    provider.reads.set(0);
    for (entry, value) in audit.entries.iter().zip(&values) {
        let object: package::ObjectRef = serde_json::from_value(entry["object"].clone()).unwrap();
        let membership = serde_json::from_value(entry["membership"].clone()).unwrap();
        assert_eq!(scoped.json(&object, membership).unwrap(), *value);
    }
    assert!(provider.reads.get() > indexed_reads);
    eprintln!(
        "locator entries={} indexed_frame_reads={} scoped_frame_reads={}",
        audit.entries.len(),
        indexed_reads,
        provider.reads.get()
    );
    // Malformed but correctly framed/rehashed root must still fail the audit.
    let mut value = provider
        .resolve(&state.locator, v1_3::FrameKind::Physical)
        .unwrap();
    value["count"] = json!("0");
    let frame = v1_3::encode_frame(
        state.locator.arena,
        v1_3::FrameKind::Physical,
        &encode(&value),
        frames(),
    )
    .unwrap();
    let mut bytes = p.allocations[&state.locator.allocation_id.to_string()]
        .bytes
        .clone();
    let mut root = state.locator.clone();
    root.offset = package::DecimalU64::parse(&bytes.len().to_string()).unwrap();
    root.byte_length = package::DecimalU64::parse(&frame.len().to_string()).unwrap();
    root.record_sha256 = package::digest(&frame);
    bytes.extend(frame);
    let bad = std::collections::BTreeMap::from([(
        root.allocation_id,
        v1_3::PackedAllocation::open(root.allocation_id, root.arena, &bytes, frames()).unwrap(),
    )]);
    assert!(v1_3::Locator::audited(&bad, project, root, limits.tree).is_err());
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Exact framing and refusal cases share one existing corpus setup"
)]
fn phpsj001_strict_bytes_torn_tail_and_original_retry() {
    let base = fixture();
    let original = verify(&base);
    let (intent, receipt) = request();
    let p = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    let mut h = header(&original.package);
    h["journal_id"] = receipt["journal_id"].clone();
    let bytes = Journal::create(&h, limits()).unwrap();
    let id = identity(&original.package);
    let reader = support::limits();
    let ctx = context(&base, &id, &reader);
    assert_eq!(&bytes[..8], b"PHPSJ001");
    let header_bytes = encode(&h);
    assert_eq!(
        &bytes[8..12],
        u32::try_from(header_bytes.len()).unwrap().to_le_bytes()
    );
    assert_eq!(&bytes[12..12 + header_bytes.len()], header_bytes);
    assert_eq!(
        hash(&bytes[..bytes.len() - 32]),
        hash_raw(&bytes[bytes.len() - 32..])
    );
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&original.package, &ctx).unwrap();
    let next = checked
        .prepare_intent(&p, record(91_001), limits())
        .unwrap();
    assert!(!next.existing());
    assert_eq!(next.expected_extent(), bytes.len());
    assert_eq!(next.expected_prefix_sha256(), hash(&bytes));
    let raw = append_raw(&bytes, &bytes[bytes.len() - 32..], next.record().value());
    assert_eq!(&raw[bytes.len()..], next.bytes());
    let j = Journal::read(&raw, &h, limits()).unwrap();
    assert_eq!(j.tail(), JournalTail::Complete);
    assert_eq!(j.records()[0].value()["sequence"], "1");
    assert_eq!(
        j.records()[0].value()["body"]["accepted_frame"]["sequence"],
        receipt["journal_sequence"]
    );
    let checked = j.verify_shared(&original.package, &ctx).unwrap();
    let retry = checked
        .prepare_intent(&p, record(91_001), limits())
        .unwrap();
    assert!(retry.existing() && retry.bytes().is_empty());
    assert_eq!(retry.record().checksum(), next.record().checksum());
    assert!(
        checked
            .prepare_intent(&p, record(91_002), limits())
            .is_err()
    );
    let other = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 2000),
        budget(),
    )
    .unwrap();
    assert!(
        checked
            .prepare_intent(&other, record(91_002), limits())
            .is_err()
    );
    for take in [1, 3, 4, 5, next.bytes().len() - 1] {
        let partial = &raw[..bytes.len() + take];
        let j = Journal::read(partial, &h, limits()).unwrap();
        assert_eq!(j.tail(), JournalTail::Incomplete);
        assert_eq!(j.verified_bytes(), bytes.len());
        assert!(
            j.verify_shared(&original.package, &ctx)
                .unwrap()
                .prepare_intent(&p, record(91_001), limits())
                .is_err()
        );
    }
    let mut corrupt = raw.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(Journal::read(&corrupt, &h, limits()).is_err());
    for (field, value) in [
        ("kind", json!("Mutation")),
        ("sequence", json!("2")),
        ("session_generation", json!("0")),
        ("unexpected", json!(true)),
    ] {
        let mut bad = next.record().value().clone();
        bad[field] = value;
        assert!(
            Journal::read(
                &append_raw(&bytes, &bytes[bytes.len() - 32..], &bad),
                &h,
                limits()
            )
            .is_err()
        );
    }
    let mut duplicated = next.record().value().clone();
    duplicated["sequence"] = json!("2");
    assert!(
        Journal::read(
            &append_raw(
                &raw,
                &support::unhex(&next.record().checksum()),
                &duplicated
            ),
            &h,
            limits()
        )
        .is_err()
    );
    let mut unknown = h.clone();
    unknown["unknown"] = json!(1);
    assert!(Journal::create(&unknown, limits()).is_err());
    let mut limited = limits();
    limited.max_records = 0;
    assert!(Journal::read(&raw, &h, limited).is_err());
    limited = limits();
    limited.max_file_bytes = raw.len() - 1;
    assert!(
        checked
            .prepare_receipt(&p, record(91_001).record_id, record(91_002), limited)
            .is_err()
    );
}
fn hash_raw(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(&mut s, "{b:02x}").unwrap();
        s
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Two consecutive existing-corpus Core operations and exact journal retries"
)]
fn phpsj001_two_completed_operations_bind_exact_selectors() {
    let base = fixture();
    let original = verify(&base);
    audited_locator_equivalence(original.shared_package());
    let (intent, receipt) = request();
    let reader = support::limits();
    let p = plan::compile_inner(
        &original,
        &intent,
        &receipt,
        attempt(&original, 1000),
        budget(),
    )
    .unwrap();
    let mut h = header(&original.package);
    h["journal_id"] = receipt["journal_id"].clone();
    let mut bytes = Journal::create(&h, limits()).unwrap();
    let id = identity(&original.package);
    let ctx = context(&base, &id, &reader);
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&original.package, &ctx).unwrap();
    let first = checked
        .prepare_intent(&p, record(92_001), limits())
        .unwrap();
    bytes.extend(first.bytes());
    let after1 = selected(&p, 6);
    let observed1 = snapshot(&base, &after1);
    let id = identity(&after1);
    let ctx1 = context(&observed1, &id, &reader);
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&after1, &ctx1).unwrap();
    let first_receipt = checked
        .prepare_receipt(&p, record(92_001).record_id, record(92_002), limits())
        .unwrap();
    bytes.extend(first_receipt.bytes());
    let mut memo = restart::CompiledPlans::default();
    let first_complete = Journal::read(&bytes, &h, limits()).unwrap();
    first_complete
        .verify_cached(&after1, &ctx1, &mut memo)
        .unwrap();
    assert_eq!(memo.compilations, 1);
    assert_eq!(memo.retained().0, 1);
    let one_cost = memo.retained().1;
    let next = p
        .verify_next_shared_original(
            after1.clone(),
            identity(&after1),
            &observed1,
            &base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &reader,
        )
        .unwrap();
    let inputs = attempt(&next, 2000);
    let coordinate = core::original_coordinate(&next, &inputs.planner, budget()).unwrap();
    let mut intent2 = intent.clone();
    intent2["expected"] = coordinate.clone();
    intent2["operation_id"] = json!(uid(3000));
    intent2["command"]["envelope"]["command_id"] = json!(uid(3000));
    let graph = intent2["command"]["envelope"]["graph_id"]
        .as_str()
        .unwrap()
        .to_owned();
    intent2["command"]["envelope"]["expected_revision"] = json!(
        coordinate["graphs"][&graph]["revision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    );
    let mut receipt2 = receipt.clone();
    receipt2["operation_id"] = intent2["operation_id"].clone();
    receipt2["before"] = receipt["after"].clone();
    receipt2["after"] = receipt["after"].clone();
    receipt2["acceptance_ordinal"] = json!("5");
    receipt2["journal_sequence"] = json!("20");
    receipt2["request_sha256"] = json!(hash(&encode(&intent2)));
    let mut second = plan::compile_inner(&next, &intent2, &receipt2, inputs, budget()).unwrap();
    // A valid next operation cannot enter while its predecessor lacks completion.
    let unresolved =
        Journal::read(&bytes[..first_receipt.expected_extent()], &h, limits()).unwrap();
    assert!(
        unresolved
            .verify_shared(&after1, &ctx1)
            .unwrap()
            .prepare_intent(&second, record(92_003), limits())
            .is_err()
    );
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&after1, &ctx1).unwrap();
    // Dedupe is before latest-head admission: original IDs remain exact retries.
    assert!(
        checked
            .prepare_intent(&p, record(92_001), limits())
            .unwrap()
            .existing()
    );
    // Refuse a branch from the original base, before preparing any bytes.
    let correct_head = second.old.head.clone();
    second.old.head = p.old.head.clone();
    assert!(
        checked
            .prepare_intent(&second, record(92_003), limits())
            .is_err()
    );
    second.old.head = correct_head;
    let second_intent = checked
        .prepare_intent(&second, record(92_003), limits())
        .unwrap();
    bytes.extend(second_intent.bytes());
    // The current physical snapshot is still the first completed operation.
    // Extend its memo with an unresolved successor, preserving original retries.
    let pending = Journal::read(&bytes, &h, limits()).unwrap();
    let pending_cached = pending.verify_cached(&after1, &ctx1, &mut memo).unwrap();
    assert_eq!(memo.journal_replayed, 1);
    assert_eq!(memo.compilations, 2);
    let work = (memo.compilations, memo.reader_work());
    let restored_pending = restart::restore_intent_plan_cached(
        &after1,
        second.original(),
        &intent2,
        &receipt2,
        &ctx1,
        &mut memo,
    )
    .unwrap();
    assert_eq!(restored_pending.files, second.files);
    assert_eq!(
        (memo.compilations, memo.reader_work()),
        work,
        "native persisted-plan path reuses the exact current predecessor"
    );
    assert!(
        pending_cached
            .prepare_intent(&second, record(92_003), limits())
            .unwrap()
            .existing()
    );
    assert_eq!(
        pending_cached.retained_metadata_bytes(),
        pending
            .verify_shared(&after1, &ctx1)
            .unwrap()
            .retained_metadata_bytes()
    );
    let after2 = selected(&second, 6);
    let observed2 = snapshot(&base, &after2);
    let id = identity(&after2);
    let ctx2 = context(&observed2, &id, &reader);
    // The executor revisits prospective clean, published, then selected clean.
    // Reuse the deterministic proof but reread independent observations each time.
    memo.inspect_shared_candidate(&after2, &ctx2, &second, 6)
        .unwrap();
    memo.inspect_shared_candidate(&after2, &ctx2, &second, 6)
        .unwrap();
    assert_eq!(memo.candidate_checks, 1);
    let published = selected(&second, 5);
    memo.inspect_shared_candidate(&published, &ctx2, &second, 5)
        .unwrap();
    memo.inspect_shared_candidate(&after2, &ctx2, &second, 6)
        .unwrap();
    assert_eq!(memo.candidate_checks, 2);
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&after2, &ctx2).unwrap();
    let second_receipt = checked
        .prepare_receipt(&second, record(92_003).record_id, record(92_004), limits())
        .unwrap();
    bytes.extend(second_receipt.bytes());
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify_shared(&after2, &ctx2).unwrap();
    assert_eq!(checked.records().len(), 4);
    // A changed selector without its new completed receipt cannot use the
    // historical prefix shortcut. Nor can altered current loose controls.
    let retained_prefix = memo.journal.clone();
    let mut guards = restart::CompiledPlans::default();
    guards.journal = retained_prefix.clone();
    let prior_hits = guards.journal_hits;
    pending.verify_cached(&after2, &ctx2, &mut guards).unwrap();
    assert_eq!(guards.journal_hits, prior_hits);
    guards.journal = retained_prefix.clone();
    let mut missing_control = after1.clone();
    missing_control.loose.pop_first();
    let _ = pending.verify_cached(&missing_control, &ctx1, &mut guards);
    assert_eq!(guards.journal_hits, prior_hits);
    assert!(
        guards
            .current_original(&missing_control, &ctx1)
            .unwrap()
            .is_none()
    );
    guards.journal = retained_prefix.clone();
    let mut smaller = context(&observed2, &id, &reader);
    smaller.max_originals = 1;
    assert!(j.verify_cached(&after2, &smaller, &mut guards).is_err());
    assert_eq!(guards.journal_hits, prior_hits);
    guards.journal = retained_prefix.clone();
    let mut changed_scope = context(&observed2, &id, &reader);
    changed_scope.directory.inode += 1;
    assert!(
        j.verify_cached(&after2, &changed_scope, &mut guards)
            .is_err()
    );
    assert_eq!(guards.journal_hits, prior_hits);
    guards.journal = retained_prefix;
    let before = std::time::Instant::now();
    let cached = j.verify_cached(&after2, &ctx2, &mut memo).unwrap();
    let growth = before.elapsed();
    assert_eq!(
        memo.journal_replayed, 2,
        "append-only new HEAD proves only new intent/receipt, never skips its closure"
    );
    assert_eq!(
        cached.retained_metadata_bytes(),
        checked.retained_metadata_bytes()
    );
    assert_eq!(
        memo.compilations, 2,
        "only the new original compiles as history grows"
    );
    assert_eq!(memo.retained().0, 2);
    let reader_before = memo.reader_work();
    let before = std::time::Instant::now();
    j.verify_cached(&after2, &ctx2, &mut memo).unwrap();
    assert_eq!(
        memo.reader_work().0,
        reader_before.0,
        "warm replay does not repeat immutable structural proofs"
    );
    assert_eq!(memo.reader_work().1, reader_before.1);
    assert!(memo.journal_hits > 0);
    assert_eq!(
        memo.journal_replayed, 0,
        "same snapshot completed history performs no replay"
    );
    let warm = before.elapsed();
    assert_eq!(memo.journal_prefix_hashed, 0);
    // Independently exercise actual full Journal ordering with warm individual
    // plan/reader memos but no whole-prefix shortcut.
    memo.journal = None;
    j.verify_cached(&after2, &ctx2, &mut memo).unwrap();
    assert_eq!(
        memo.journal_prefix_hashed,
        after2
            .allocations
            .values()
            .map(|a| a.bytes.len())
            .sum::<usize>()
    );
    assert_eq!(
        memo.compilations, 2,
        "unchanged historical compilation is reused"
    );
    assert!(memo.hits >= 3);
    assert!(memo.retained().1 <= usize::try_from(budget().max_total_allocation_bytes).unwrap());
    assert!(cached.retained_metadata_bytes() < memo.retained().1 / 10);
    eprintln!(
        "compiled memo: growing={growth:?} warm={warm:?} compilations={} hits={} plan_payload_bytes={} compact_metadata_bytes={}",
        memo.compilations,
        memo.hits,
        memo.retained().1,
        cached.retained_metadata_bytes()
    );
    let local_cached = cached
        .verify_session(&original, &attempt(&original, 1000).planner, budget())
        .unwrap();
    assert_eq!(local_cached.coordinate(), intent2["expected"]);
    assert!(local_cached.through().is_none());
    let current_original = second
        .verify_next_shared_original(
            after2.clone(),
            identity(&after2),
            &observed2,
            &base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &reader,
        )
        .unwrap();
    let checkpoint_identity = super::super::checkpoint::CheckpointIdentity {
        package: identity(&after2),
        device: package::PackageUuid::parse(h["device_id"].as_str().unwrap()).unwrap(),
        incarnation: base.registration.incarnation,
    };
    let history = super::super::checkpoint::PreparedHistory::prepare(
        &local_cached,
        &current_original,
        &checkpoint_identity,
        package::PackageUuid::parse(&uid(500_100)).unwrap(),
        (2, 2),
        &reader,
    )
    .unwrap();
    assert_eq!(
        history.archive()["segments"][0]["byte_length"],
        bytes.len().to_string()
    );
    let entries = history
        .lookup_objects()
        .values()
        .filter_map(|v| v["entries"].as_array())
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 5);
    assert!(
        entries.iter().all(|entry| entry["local"].is_null()),
        "checkpoint-only history never invents Mutation ownership"
    );
    assert!(
        super::super::checkpoint::PreparedHistory::prepare(
            &local_cached,
            &original,
            &checkpoint_identity,
            package::PackageUuid::parse(&uid(500_100)).unwrap(),
            (2, 2),
            &reader
        )
        .is_err()
    );
    let mut no_pages = reader;
    no_pages.tree.max_pages = 0;
    assert!(
        super::super::checkpoint::PreparedHistory::prepare(
            &local_cached,
            &current_original,
            &checkpoint_identity,
            package::PackageUuid::parse(&uid(500_100)).unwrap(),
            (2, 2),
            &no_pages
        )
        .is_err()
    );

    let mut narrow = budget();
    narrow.semantic.json.max_array_elements = 1;
    assert!(
        cached
            .verify_session(&original, &attempt(&original, 1000).planner, narrow)
            .is_err()
    );

    // A working set beyond the optional cap never churns warm originals or
    // changes acceptance: only the uncached second plan recompiles each pass.
    let mut capped = restart::CompiledPlans::default();
    capped.set_test_ceiling(one_cost);
    for pass in 0..3 {
        // Exercise the compiled-plan cap independently of the completed-journal
        // proof cache, whose warm-hit behavior is checked above.
        capped.journal = None;
        j.verify_cached(&after2, &ctx2, &mut capped).unwrap();
        assert_eq!(capped.compilations, 2 + pass);
        assert_eq!(capped.retained().0, 1);
        assert!(capped.retained().1 <= one_cost);
    }
    eprintln!(
        "over-cap replay: compilations={} hits={} bounded_bytes={}",
        capped.compilations,
        capped.hits,
        capped.retained().1
    );
    let restored = restart::restore_plan_cached(&after2, &ctx2, &mut memo).unwrap();
    assert_eq!(restored.original(), second.original());
    assert_eq!(restored.stages()[6].head, after2.head);
    let restarted = restart::restore_intent_plan_cached(
        &after2,
        second.original(),
        &intent2,
        &receipt2,
        &ctx2,
        &mut memo,
    )
    .unwrap();
    assert_eq!(restarted.files, second.files);
    let mut altered_observation = snapshot(&base, &after2);
    altered_observation
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .inode += 1;
    let changed_context = context(&altered_observation, &id, &reader);
    assert!(
        memo.inspect_shared_candidate(&after2, &changed_context, &second, 6)
            .is_err()
    );
    assert!(
        j.verify_cached(&after2, &changed_context, &mut memo)
            .is_err()
    );
    let mut altered_charge = snapshot(&base, &after2);
    altered_charge
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .charge = 0;
    let changed_context = context(&altered_charge, &id, &reader);
    assert!(
        memo.inspect_shared_candidate(&after2, &changed_context, &second, 6)
            .is_err()
    );
    assert!(
        j.verify_cached(&after2, &changed_context, &mut memo)
            .is_err()
    );
    let mut narrow_reader = reader;
    narrow_reader.tree.max_pages = 0;
    let narrow_context = context(&observed2, &id, &narrow_reader);
    assert!(
        memo.inspect_shared_candidate(&after2, &narrow_context, &second, 6)
            .is_err()
    );
    assert!(
        j.verify_cached(&after2, &narrow_context, &mut memo)
            .is_err()
    );
    let mut wrong_scope = context(&observed2, &id, &reader);
    wrong_scope.directory.inode += 1;
    assert!(j.verify_cached(&after2, &wrong_scope, &mut memo).is_err());
    let mut malformed_control = after2.clone();
    malformed_control
        .loose
        .values_mut()
        .next()
        .unwrap()
        .push(b' ');
    assert!(
        j.verify_cached(&malformed_control, &ctx2, &mut memo)
            .is_err()
    );
    // A rechecksummed same-ID original is decoded but cannot reuse this memo.
    let mut altered = first.record().value().clone();
    altered["body"]["original"]["reserve"] = json!("1");
    let initial = Journal::create(&h, limits()).unwrap();
    let altered_bytes = append_raw(&initial, &initial[initial.len() - 32..], &altered);
    let altered_journal = Journal::read(&altered_bytes, &h, limits()).unwrap();
    assert!(
        altered_journal
            .verify_cached(&after2, &ctx2, &mut memo)
            .is_err()
    );

    // A coherently reframed header must miss the completed-prefix memo; its
    // altered base cannot inherit the earlier verified checkpoint history.
    let mut altered_header = h.clone();
    altered_header["base_package_revision"] = json!("999");
    let mut reframed = Journal::create(&altered_header, limits()).unwrap();
    for record in j.records() {
        let previous = reframed[reframed.len() - 32..].to_vec();
        reframed = append_raw(&reframed, &previous, record.value());
    }
    let reframed = Journal::read(&reframed, &altered_header, limits()).unwrap();
    assert!(reframed.verify_cached(&after2, &ctx2, &mut memo).is_err());

    // Invocation-local predecessor reuse preserves the exact chain depth bound.
    let mut bounded = context(&observed2, &id, &reader);
    bounded.max_originals = 1;
    assert!(matches!(
        j.verify_shared(&after2, &bounded),
        Err(crate::package::PackageError::Limit)
    ));
    bounded.max_originals = 2;
    assert!(j.verify_shared(&after2, &bounded).is_ok());
    // A new verification must read current bytes again, never inherit the cache.
    let mut changed = after2.clone();
    changed.allocations.values_mut().next().unwrap().bytes[16] ^= 1;
    assert!(j.verify_shared(&changed, &ctx2).is_err());
    assert!(
        memo.inspect_shared_candidate(&changed, &ctx2, &second, 6)
            .is_err()
    );
    assert!(j.verify_cached(&changed, &ctx2, &mut memo).is_err());
    let mut changed_witness = after2.clone();
    changed_witness
        .allocations
        .values_mut()
        .next()
        .unwrap()
        .witness["inode"] = json!("0");
    assert!(j.verify_cached(&changed_witness, &ctx2, &mut memo).is_err());
    let mut smaller = context(&observed2, &id, &reader);
    smaller.planner.max_total_allocation_bytes = 1;
    assert!(j.verify_cached(&after2, &smaller, &mut memo).is_err());
    assert_eq!(memo.retained(), (0, 0));

    // Checkpoint-only history advances the inherited PS3 coordinate without
    // inventing local Mutation ownership or resetting to the header base.
    let local = checked
        .verify_session(&original, &attempt(&original, 1000).planner, budget())
        .unwrap();
    assert_eq!(local.coordinate(), intent2["expected"]);
    assert!(local.through().is_none() && local.accepted_frame().is_none());
    let owner = json!({"attachment_id":uid(93000),"attachment_generation":"1","principal":receipt["provenance"]["principal"]});
    let barrier = local
        .prepare_barrier(&owner, "flush", record(93001), limits())
        .unwrap();
    assert_eq!(
        barrier.record().value()["body"]["target"],
        local.coordinate()
    );
    assert!(barrier.record().value()["body"]["through"].is_null());

    assert!(
        checked
            .prepare_receipt(&p, record(92_001).record_id, record(92_002), limits())
            .unwrap()
            .existing()
    );
    assert!(
        checked
            .prepare_receipt(&second, record(92_003).record_id, record(92_004), limits())
            .unwrap()
            .existing()
    );
    // Rechecksummed but wrong completed selector is not accepted evidence.
    let mut forged = second_receipt.record().value().clone();
    forged["body"]["selected_head"] = p.stages[6].head.clone();
    let prefix = &bytes[..second_receipt.expected_extent()];
    let forged = append_raw(
        prefix,
        &support::unhex(&second_intent.record().checksum()),
        &forged,
    );
    let j = Journal::read(&forged, &h, limits()).unwrap();
    assert!(j.verify_shared(&after2, &ctx2).is_err());
    assert!(j.verify_cached(&after2, &ctx2, &mut memo).is_err());
    assert_eq!(
        hash(&bytes),
        "788ad5be74841fdb8e58457d952ad60e72266400eb3813286b6cf4f10bd180b1"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Exact new-segment bindings and old framing compatibility in one existing corpus"
)]
fn phpsj002_exact_base_sequence_and_torn_tail_refusals() {
    use super::super::checkpoint::{CheckpointIdentity, LocalArchive, LocalBase, LocalSelection};
    let fixture = support::Fixture::load("integrated");
    let original = package(&fixture);
    let h1 = header(&original);
    let old = Journal::create(&h1, limits()).unwrap();
    let identity = CheckpointIdentity {
        package: identity(&original),
        device: package::PackageUuid::parse(h1["device_id"].as_str().unwrap()).unwrap(),
        incarnation: fixture.registration.incarnation,
    };
    let common = json!({"project_id":h1["project_id"],"library_id":h1["library_id"],"bootstrap_sha256":h1["bootstrap_sha256"],"extensions":{},"device_id":h1["device_id"],"incarnation_id":h1["incarnation_id"]});
    let record = |name: &str, mut value: Value| {
        for (k, v) in common.as_object().unwrap() {
            value[k] = v.clone();
        }
        value["schema"] = json!({"id":name,"version":1});
        value
    };
    let through = json!({"record_id":uid(500_001),"checksum":"ab".repeat(32),"sequence":"41"});
    let base_value = record(
        "photara.session.checkpoint-base",
        json!({"checkpoint":original.commit["root_set"]["active"],"package_head":original.head,"package_commit":reference(&original.commit),"journal_id":h1["journal_id"],"through":through,"mutation":null,"accepted_frame":null,"coordinate":request().0["expected"],"undo_roots":[],"lookup":reference(&json!({"test":"lookup"})),"archive":reference(&json!({"test":"archive"}))}),
    );
    let base = LocalBase::parse(&encode(&base_value), &identity, limits().json).unwrap();
    let mut h2 = h1.clone();
    h2["format_version"] = json!(2);
    h2["stream_id"] = json!(uid(500_002));
    h2["base"] = reference(base.value());
    h2["sequence_base"] = through["sequence"].clone();
    h2["previous_segment"] =
        json!({"stream_id":h1["stream_id"],"header_sha256":hash(&old),"last":through});
    base.check_header(&h2).unwrap();
    let prefix = Journal::create(&h2, limits()).unwrap();
    assert_eq!(&prefix[..8], b"PHPSJ002");
    let value = json!({"format_version":2,"sequence":"42","record_id":uid(500_003),"session_generation":"1","project_id":h2["project_id"],"incarnation_id":h2["incarnation_id"],"kind":"CheckpointBaseReceipt","body":{"maintenance_id":uid(500_004),"original":reference(&json!({"test":"original"})),"journal_base":h2["base"],"selected_head":original.head,"selected_commit":reference(&original.commit)}});
    let bytes = append_raw(&prefix, &prefix[prefix.len() - 32..], &value);
    let parsed = Journal::read(&bytes, &h2, limits()).unwrap();
    assert_eq!(parsed.records()[0].value(), &value);
    // A structurally valid base/segment is never writable or Accepted authority.
    assert!(
        parsed
            .verify_shared(
                &original,
                &context(&fixture, &identity.package, &support::limits())
            )
            .is_err()
    );
    for cut in [1, 4, bytes.len() - prefix.len() - 1] {
        let partial = Journal::read(&bytes[..prefix.len() + cut], &h2, limits()).unwrap();
        assert_eq!(partial.tail(), JournalTail::Incomplete);
        assert!(partial.records().is_empty());
        assert_eq!(partial.verified_bytes(), prefix.len());
    }
    for field in ["format_version", "sequence"] {
        let mut bad = value.clone();
        bad[field] = if field == "format_version" {
            json!(1)
        } else {
            json!("1")
        };
        assert!(
            Journal::read(
                &append_raw(&prefix, &prefix[prefix.len() - 32..], &bad),
                &h2,
                limits()
            )
            .is_err()
        );
    }
    for commit in [
        original.commit.clone(),
        reference(&json!({"substituted":true})),
    ] {
        let mut changed = value.clone();
        changed["body"]["selected_commit"] = commit;
        assert!(
            Journal::read(
                &append_raw(&prefix, &prefix[prefix.len() - 32..], &changed),
                &h2,
                limits()
            )
            .is_err()
        );
    }
    let mut duplicate = value.clone();
    duplicate["sequence"] = json!("43");
    duplicate["record_id"] = json!(uid(500_005));
    assert!(
        Journal::read(
            &append_raw(&bytes, &bytes[bytes.len() - 32..], &duplicate),
            &h2,
            limits()
        )
        .is_err()
    );
    let mut bad = bytes.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    assert!(Journal::read(&bad, &h2, limits()).is_err());
    assert!(Journal::read(&bytes, &h1, limits()).is_err());
    assert!(Journal::read(&old, &h2, limits()).is_err());
    let mut bad_header = h2.clone();
    bad_header["sequence_base"] = json!("40");
    assert!(Journal::create(&bad_header, limits()).is_err());
    bad_header = h2.clone();
    bad_header["base"] = reference(&json!({"changed":true}));
    assert!(base.check_header(&bad_header).is_err());
    let selected = record(
        "photara.session.checkpoint-selection",
        json!({"generation":"1","previous_sha256":null,"selected":{"format":"PHPSJ001","stream_id":h1["stream_id"],"header_sha256":hash(&old),"base":null},"pending":null}),
    );
    let selection = LocalSelection::parse(&encode(&selected), &identity, limits().json).unwrap();
    let mut next = selected.clone();
    next["generation"] = json!("2");
    next["previous_sha256"] = json!(hash(&encode(&selected)));
    next["selected"] = json!({"format":"PHPSJ002","stream_id":h2["stream_id"],"header_sha256":hash(&prefix),"base":h2["base"]});
    let next = LocalSelection::parse(&encode(&next), &identity, limits().json).unwrap();
    selection
        .successor(&next, &hash(&encode(&selected)))
        .unwrap();
    assert!(selection.successor(&next, &"00".repeat(32)).is_err());
    let archive = record(
        "photara.session.checkpoint-archive",
        json!({"archive_id":uid(500_006),"previous":null,"segments":[{"stream_id":h1["stream_id"],"journal_id":h1["journal_id"],"header_sha256":hash(&old),"byte_length":"10000","sha256":"cd".repeat(32),"first_sequence":"1","last_sequence":"41","last_checksum":through["checksum"]}]}),
    );
    LocalArchive::parse(&encode(&archive), &identity, limits().json).unwrap();
    let mut duplicate = archive.clone();
    duplicate["segments"]
        .as_array_mut()
        .unwrap()
        .push(archive["segments"][0].clone());
    assert!(LocalArchive::parse(&encode(&duplicate), &identity, limits().json).is_err());
    assert_eq!(Journal::create(&h1, limits()).unwrap(), old);
}
