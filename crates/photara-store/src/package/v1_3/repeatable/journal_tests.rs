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
    let checked = j.verify(&original.package, &ctx).unwrap();
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
    let checked = j.verify(&original.package, &ctx).unwrap();
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
            j.verify(&original.package, &ctx)
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
    let checked = j.verify(&original.package, &ctx).unwrap();
    let first = checked
        .prepare_intent(&p, record(92_001), limits())
        .unwrap();
    bytes.extend(first.bytes());
    let after1 = selected(&p, 6);
    let observed1 = snapshot(&base, &after1);
    let id = identity(&after1);
    let ctx1 = context(&observed1, &id, &reader);
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify(&after1, &ctx1).unwrap();
    let first_receipt = checked
        .prepare_receipt(&p, record(92_001).record_id, record(92_002), limits())
        .unwrap();
    bytes.extend(first_receipt.bytes());
    let next = p
        .verify_next_original(
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
            .verify(&after1, &ctx1)
            .unwrap()
            .prepare_intent(&second, record(92_003), limits())
            .is_err()
    );
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify(&after1, &ctx1).unwrap();
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
    let after2 = selected(&second, 6);
    let observed2 = snapshot(&base, &after2);
    let id = identity(&after2);
    let ctx2 = context(&observed2, &id, &reader);
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify(&after2, &ctx2).unwrap();
    let second_receipt = checked
        .prepare_receipt(&second, record(92_003).record_id, record(92_004), limits())
        .unwrap();
    bytes.extend(second_receipt.bytes());
    let j = Journal::read(&bytes, &h, limits()).unwrap();
    let checked = j.verify(&after2, &ctx2).unwrap();
    assert_eq!(checked.records().len(), 4);
    // Invocation-local predecessor reuse preserves the exact chain depth bound.
    let mut bounded = context(&observed2, &id, &reader);
    bounded.max_originals = 1;
    assert!(matches!(
        j.verify(&after2, &bounded),
        Err(crate::package::PackageError::Limit)
    ));
    bounded.max_originals = 2;
    assert!(j.verify(&after2, &bounded).is_ok());
    // A new verification must read current bytes again, never inherit the cache.
    let mut changed = after2.clone();
    changed.allocations.values_mut().next().unwrap().bytes[16] ^= 1;
    assert!(j.verify(&changed, &ctx2).is_err());

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
    assert!(j.verify(&after2, &ctx2).is_err());
    assert_eq!(
        hash(&bytes),
        "788ad5be74841fdb8e58457d952ad60e72266400eb3813286b6cf4f10bd180b1"
    );
}
