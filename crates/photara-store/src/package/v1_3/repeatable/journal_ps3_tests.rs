//! Additive local records reuse the existing integrated package and commands.
use super::super::journal::{Journal, JournalLimits, JournalRecordIdentity, VerifiedSession};
use super::*;
fn limits() -> JournalLimits {
    JournalLimits {
        json: support::limits().json,
        max_file_bytes: 1 << 20,
        max_records: 32,
        max_work_bytes: 1 << 20,
    }
}
fn record(n: u64) -> JournalRecordIdentity {
    JournalRecordIdentity {
        record_id: package::PackageUuid::parse(&uid(n)).unwrap(),
        session_generation: 1,
    }
}
fn header(base: &OriginalPackage, receipt: &Value) -> Value {
    json!({"format_version":1,"stream_id":uid(90001),"journal_id":receipt["journal_id"],"device_id":uid(90002),"project_id":base.manifest["project_id"],"library_id":base.commit["root_set"]["library_id"],"incarnation_id":"96000000-0000-4000-8000-000000000001","bootstrap_sha256":hash(&encode(&base.manifest)),"base_head":base.head,"base_package_revision":base.commit["package_revision"]})
}
fn next(
    session: &VerifiedSession<'_>,
    prior: &Value,
    owner: &Value,
    action: &Value,
    n: u64,
) -> (Value, Value) {
    let (mut intent, mut receipt) = request();
    intent["operation_id"] = json!(uid(n));
    intent["expected"] = session.coordinate();
    if action["kind"] != "edit" {
        intent["command"] = session.inverse_command(owner, action).unwrap();
    }
    let graph = intent["command"]["envelope"]["graph_id"]
        .as_str()
        .unwrap()
        .to_owned();
    intent["command"]["envelope"]["command_id"] = intent["operation_id"].clone();
    intent["command"]["envelope"]["expected_revision"] = json!(
        intent["expected"]["graphs"][&graph]["revision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    );
    let result = session.preview(&intent).unwrap();
    receipt["operation_id"] = intent["operation_id"].clone();
    receipt["request_sha256"] = json!(hash(&encode(&intent)));
    receipt["before"] = json!({"revision":intent["expected"]["revision"],"digest":intent["expected"]["authored_digest"]});
    receipt["after"] = json!({"revision":result["revision"],"digest":result["authored_digest"]});
    for key in ["acceptance_ordinal", "journal_sequence"] {
        receipt[key] =
            json!((prior[key].as_str().unwrap().parse::<u64>().unwrap() + 1).to_string());
    }
    (intent, receipt)
}
fn append_raw(prefix: &[u8], record: &Value) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut bytes = prefix.to_vec();
    let payload = encode(record);
    let length = u32::try_from(payload.len()).unwrap().to_le_bytes();
    let mut sha = Sha256::new();
    sha.update(&bytes[bytes.len() - 32..]);
    sha.update(length);
    sha.update(&payload);
    bytes.extend(length);
    bytes.extend(payload);
    bytes.extend(sha.finalize());
    bytes
}
fn replace_first(header: &Value, record: &Value) -> Vec<u8> {
    append_raw(&Journal::create(header, limits()).unwrap(), record)
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One existing-corpus history exercises shared replay, undo and coherent negative edits"
)]
fn ps3_core_patch_restart_undo_redo_and_finite_barrier() {
    let fixture = fixture();
    let original = verify(&fixture);
    let inputs = attempt(&original, 1000);
    let (intent, mut receipt) = request();
    receipt["journal_sequence"] = json!("12");
    let header = header(&original.package, &receipt);
    let mut bytes = Journal::create(&header, limits()).unwrap();
    let owner = json!({"attachment_id":uid(91000),"attachment_generation":"1","principal":receipt["provenance"]["principal"]});
    let id = identity(&original.package);
    let reader = support::limits();
    let context = RestartContext {
        inspection: &fixture,
        registration: &fixture.registration,
        directory: v1_3::DirectoryObservation {
            device: 7,
            inode: 900,
        },
        identity: &id,
        frames: frames(),
        reader: &reader,
        planner: budget(),
        max_originals: 4,
    };
    macro_rules! session {
        ($journal:ident) => {
            $journal
                .verify(&original.package, &context)
                .unwrap()
                .verify_session(&original, &inputs.planner, budget())
                .unwrap()
        };
    }
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    let edit = session
        .prepare_mutation(
            &intent,
            &receipt,
            &owner,
            &json!({"kind":"edit"}),
            record(92001),
            limits(),
        )
        .unwrap();
    assert!(
        !edit.record().value()["body"]["patch"]["added"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Rechecksummed but semantically false complete patches are refused.
    let mut forged = edit.record().value().clone();
    forged["body"]["patch"]["added"][0]["value"]["future_optional"] =
        json!({"retained":"must not be invented"});
    forged["body"]["patch"]["added"][0]["reference"] =
        reference(&forged["body"]["patch"]["added"][0]["value"]);
    forged["body"]["patch"]["added"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|row| super::super::wire::key(&row["reference"]).unwrap());
    let forged_bytes = replace_first(&header, &forged);
    let forged_journal = Journal::read(&forged_bytes, &header, limits()).unwrap();
    assert!(
        forged_journal
            .verify(&original.package, &context)
            .unwrap()
            .verify_session(&original, &inputs.planner, budget())
            .is_err()
    );
    let mut unknown = intent.clone();
    unknown["command"]["envelope"]["future_optional"] = json!(true);
    assert!(session.preview(&unknown).is_err());
    let mut wrong_result = edit.record().value().clone();
    wrong_result["body"]["result"]["authored_digest"] = json!("0".repeat(64));
    let forged_bytes = replace_first(&header, &wrong_result);
    let forged_journal = Journal::read(&forged_bytes, &header, limits()).unwrap();
    assert!(
        forged_journal
            .verify(&original.package, &context)
            .unwrap()
            .verify_session(&original, &inputs.planner, budget())
            .is_err()
    );
    bytes.extend(edit.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    assert_eq!(
        session.coordinate(),
        edit.record().value()["body"]["result"]
    );
    assert!(
        session
            .prepare_mutation(
                &intent,
                &receipt,
                &owner,
                &json!({"kind":"edit"}),
                record(92001),
                limits()
            )
            .unwrap()
            .existing()
    );
    assert!(
        session
            .prepare_mutation(
                &intent,
                &receipt,
                &owner,
                &json!({"kind":"undo","target":session.undo_targets(&owner)[0]}),
                record(92001),
                limits()
            )
            .is_err()
    );
    let target = session.undo_targets(&owner)[0].clone();
    let barrier = session
        .prepare_barrier(&owner, "save-now", record(92002), limits())
        .unwrap();
    bytes.extend(barrier.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    let boundary = session
        .prepare_boundary(&owner, &target, record(92003), limits())
        .unwrap();
    bytes.extend(boundary.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    let action = json!({"kind":"undo","target":target});
    let (undo_intent, undo_receipt) = next(&session, &receipt, &owner, &action, 93001);
    let mut foreign = owner.clone();
    foreign["attachment_id"] = json!(uid(91001));
    assert!(
        session
            .prepare_mutation(
                &undo_intent,
                &undo_receipt,
                &foreign,
                &action,
                record(92004),
                limits()
            )
            .is_err()
    );
    let undo = session
        .prepare_mutation(
            &undo_intent,
            &undo_receipt,
            &owner,
            &action,
            record(92004),
            limits(),
        )
        .unwrap();
    bytes.extend(undo.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    assert_eq!(session.redo_targets(&owner).len(), 1);
    assert_ne!(
        session.coordinate(),
        barrier.record().value()["body"]["target"]
    );
    // A new edit durably invalidates this owner's redo set in the same record.
    let (branch_intent, branch_receipt) = next(
        &session,
        &undo_receipt,
        &owner,
        &json!({"kind":"edit"}),
        93010,
    );
    let branch = session
        .prepare_mutation(
            &branch_intent,
            &branch_receipt,
            &owner,
            &json!({"kind":"edit"}),
            record(92010),
            limits(),
        )
        .unwrap();
    assert_eq!(
        branch.record().value()["body"]["redo_invalidated"],
        json!([receipt["operation_id"]])
    );
    let mut branch_bytes = bytes.clone();
    branch_bytes.extend(branch.bytes());
    let branch_journal = Journal::read(&branch_bytes, &header, limits()).unwrap();
    let branch_view = session!(branch_journal);
    assert!(branch_view.redo_targets(&owner).is_empty());
    assert!(
        branch_view
            .prepare_mutation(
                &undo_intent,
                &undo_receipt,
                &owner,
                &action,
                record(92004),
                limits()
            )
            .unwrap()
            .existing()
    );
    let redo_target = session.redo_targets(&owner)[0].clone();
    let action = json!({"kind":"redo","target":redo_target["target"],"undo":redo_target["undo"]});
    let (redo_intent, redo_receipt) = next(&session, &undo_receipt, &owner, &action, 93002);
    let redo = session
        .prepare_mutation(
            &redo_intent,
            &redo_receipt,
            &owner,
            &action,
            record(92005),
            limits(),
        )
        .unwrap();
    bytes.extend(redo.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    assert!(session.redo_targets(&owner).is_empty());
    let decision = session
        .prepare_replay_decision(&owner, record(92006), limits())
        .unwrap();
    let mut forged_decision = decision.record().value().clone();
    forged_decision["body"]["outcome"]["through_mutation"]["record_checksum"] =
        json!("0".repeat(64));
    let forged_bytes = append_raw(&bytes, &forged_decision);
    let forged_journal = Journal::read(&forged_bytes, &header, limits()).unwrap();
    assert!(
        forged_journal
            .verify(&original.package, &context)
            .unwrap()
            .verify_session(&original, &inputs.planner, budget())
            .is_err()
    );
    bytes.extend(decision.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    assert_eq!(
        session.coordinate(),
        redo.record().value()["body"]["result"]
    );
    assert_eq!(
        session.accepted_frame().unwrap()["sequence"],
        redo_receipt["journal_sequence"]
    );
    assert_eq!(journal.records().len(), 6);
    assert_eq!(
        hash(&bytes),
        "75b9ccd52e7adcc4599c1abdd9879fae46e094ce4a71ed404ee4d490285443ef"
    );
    let mut stale_barrier = barrier.record().value().clone();
    stale_barrier["record_id"] = json!(uid(92999));
    stale_barrier["sequence"] = json!("7");
    let forged_bytes = append_raw(&bytes, &stale_barrier);
    let forged_journal = Journal::read(&forged_bytes, &header, limits()).unwrap();
    assert!(
        forged_journal
            .verify(&original.package, &context)
            .unwrap()
            .verify_session(&original, &inputs.planner, budget())
            .is_err()
    );
    let (noop_intent, noop_receipt) = next(
        &session,
        &redo_receipt,
        &owner,
        &json!({"kind":"edit"}),
        93012,
    );
    let noop = session
        .prepare_mutation(
            &noop_intent,
            &noop_receipt,
            &owner,
            &json!({"kind":"edit"}),
            record(92012),
            limits(),
        )
        .unwrap();
    let patch = &noop.record().value()["body"]["patch"];
    assert_eq!(patch["before"], patch["after"]);
    assert_eq!(patch["removed"], json!([]));
    assert_eq!(patch["added"], json!([]));
    // Stale inverse: another authored edit changed this operation's affected value.
    let (mut stale_intent, mut stale_receipt) = next(
        &session,
        &redo_receipt,
        &owner,
        &json!({"kind":"edit"}),
        93011,
    );
    stale_intent["command"]["envelope"]["command"]["x"] = json!(999);
    let result = session.preview(&stale_intent).unwrap();
    stale_receipt["request_sha256"] = json!(hash(&encode(&stale_intent)));
    stale_receipt["after"] =
        json!({"revision":result["revision"],"digest":result["authored_digest"]});
    let stale = session
        .prepare_mutation(
            &stale_intent,
            &stale_receipt,
            &owner,
            &json!({"kind":"edit"}),
            record(92011),
            limits(),
        )
        .unwrap();
    bytes.extend(stale.bytes());
    let journal = Journal::read(&bytes, &header, limits()).unwrap();
    let session = session!(journal);
    assert!(
        session
            .inverse_command(&owner, &json!({"kind":"undo","target":target}))
            .is_err()
    );
}
