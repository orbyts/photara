//! Fixture-only bridge between framing and the single-RenameGraph evidence.
//! This body deliberately has no production schema/compatibility promise: undo,
//! patches, sessions, batching and qualified durability remain separate gates.
use super::super::journal::{self, AppendFault, AppendOutcome, Header, Record, Tail};
use super::*;
use package::{PackageUuid, Sha256Hex};
use std::{
    fs,
    fs::OpenOptions,
    io::{Read as _, Seek as _},
};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationBody {
    operation_id: PackageUuid,
    request_digest: Sha256Hex,
    evidence: Evidence,
}

fn fixture() -> (VerifiedClosure, Box<CheckpointPlan>, Header, Record) {
    let base = verified(build(add_history));
    let mutation = rename(&base, "Journal replay");
    let plan = checkpoint(run(&base, &mutation, 34000).unwrap());
    let read = package::v1_1::validate_memory(base.files()).unwrap();
    let header = Header {
        format_version: 1,
        journal_id: Uuid::parse_str(&id(35000)).unwrap(),
        device_id: Uuid::parse_str(&id(35001)).unwrap(),
        project_id: decode(json!(read.bootstrap.project_id)),
        library_id: decode(read.authored.value["owning_library_id"].clone()),
        incarnation_id: incarnation().uuid().as_uuid(),
        manifest_sha256: base.token().manifest_digest().as_str().into(),
        base_head_sha256: base.token().head_digest().as_str().into(),
    };
    let body = MutationBody {
        operation_id: decode(json!(mutation.operation_id)),
        request_digest: plan.receipt().request_digest().clone(),
        evidence: serde_json::from_slice(&prepare(&base, &mutation, 34000)).unwrap(),
    };
    let record = Record {
        format_version: 1,
        sequence: 1,
        id: Uuid::parse_str(&id(35002)).unwrap(),
        session_generation: Uuid::parse_str(&id(35003)).unwrap(),
        project_id: header.project_id,
        incarnation_id: header.incarnation_id,
        kind: "Mutation".into(),
        body: serde_json::to_value(body).unwrap(),
    };
    (base, plan, header, record)
}

fn encoded(header: &Header, record: &Record) -> Vec<u8> {
    let mut bytes = journal::header_bytes(header).unwrap();
    let (_, _, previous) = journal::read_header(&bytes).unwrap();
    journal::append_frame(&mut bytes, previous, record).unwrap();
    bytes
}

fn recover(
    bytes: &[u8],
    base: &VerifiedClosure,
    plan: &CheckpointPlan,
    header: &Header,
    expected: &Record,
) -> Result<Box<CheckpointPlan>, &'static str> {
    let scan = journal::scan_bound(bytes, header).map_err(|_| "journal binding")?;
    if scan.tail != Tail::Complete || scan.records.len() != 1 {
        return Err("single complete mutation required");
    }
    let record = &scan.records[0];
    if record.id != expected.id || record.session_generation != expected.session_generation {
        return Err("record binding");
    }
    let bytes = canon(&record.body);
    let value = package::parse_canonical_json(&bytes, package::JsonLimits::default())
        .map_err(|_| "body admission")?;
    let body: MutationBody = serde_json::from_value(value).map_err(|_| "typed body")?;
    if canonical_json(&body).map_err(|_| "body encoding")? != bytes
        || body.operation_id.as_uuid().is_nil()
        || body.operation_id.to_string() != plan.receipt().operation_id().to_string()
        || body.evidence.mutation.operation_id != plan.receipt().operation_id()
        || &body.request_digest != plan.receipt().request_digest()
        || body.evidence.request_digest != body.request_digest.as_str()
    {
        return Err("operation binding");
    }
    let replayed = replay(
        base,
        &canonical_json(&body.evidence).map_err(|_| "evidence encoding")?,
    )
    .map_err(|_| "semantic evidence")?;
    if replayed.ids().write_id != plan.ids().write_id
        || replayed.ids().commit_id != plan.ids().commit_id
        || replayed.receipt().before() != plan.receipt().before()
        || replayed.receipt().after() != plan.receipt().after()
        || owned(replayed.candidate().files()) != owned(plan.candidate().files())
    {
        return Err("checkpoint binding");
    }
    Ok(replayed)
}

// Even this fixture validates typed command/result agreement before append; the
// framing adapter alone understands only minimal operation/digest evidence.
fn append_typed(
    file: &mut fs::File,
    base: &VerifiedClosure,
    plan: &CheckpointPlan,
    header: &Header,
    record: &Record,
    expected: &Record,
    fault: AppendFault,
) -> Result<AppendOutcome, &'static str> {
    recover(&encoded(header, record), base, plan, header, expected)?;
    file.rewind().map_err(|_| "journal seek")?;
    let mut prior = Vec::new();
    file.read_to_end(&mut prior).map_err(|_| "journal read")?;
    if prior != journal::header_bytes(header).map_err(|_| "header encoding")? {
        // A matching minimal operation/digest pair cannot bless an existing
        // unknown or inconsistent typed body during the framing dedupe path.
        recover(&prior, base, plan, header, expected)?;
    }
    Ok(journal::append_disposable(file, header, record, fault))
}

#[test]
fn typed_mutation_refuses_unknown_fields_tampered_ids_and_wrong_base() {
    let (base, plan, header, record) = fixture();
    let original = owned(base.files());
    let complete = encoded(&header, &record);
    recover(&complete, &base, &plan, &header, &record).unwrap();
    for (pointer, value) in [
        ("/operation_id", json!(id(35999))),
        ("/operation_id", json!(Uuid::nil())),
        ("/request_digest", json!("e".repeat(64))),
        ("/evidence/mutation/operation_id", json!(id(35999))),
        ("/evidence/write_id", json!(id(35999))),
        ("/evidence/commit_id", json!(id(35999))),
        ("/evidence/mutation/command/name", json!("tampered")),
        ("/evidence/version", json!(2)),
    ] {
        let mut changed = record.clone();
        *changed.body.pointer_mut(pointer).unwrap() = value;
        assert!(
            recover(&encoded(&header, &changed), &base, &plan, &header, &record).is_err(),
            "{pointer}"
        );
    }
    for pointer in [
        "",
        "/evidence",
        "/evidence/mutation",
        "/evidence/mutation/command",
    ] {
        let mut changed = record.clone();
        changed.body.pointer_mut(pointer).unwrap()["future_field"] = json!(true);
        assert!(
            recover(&encoded(&header, &changed), &base, &plan, &header, &record).is_err(),
            "{pointer}"
        );
    }
    for change_session in [false, true] {
        let mut changed = record.clone();
        if change_session {
            changed.session_generation = Uuid::new_v4();
        } else {
            changed.id = Uuid::new_v4();
        }
        assert!(recover(&encoded(&header, &changed), &base, &plan, &header, &record).is_err());
    }
    assert!(recover(&complete, plan.candidate(), &plan, &header, &record).is_err());
    let mut wrong_header = header.clone();
    wrong_header.incarnation_id = Uuid::new_v4();
    assert!(recover(&complete, &base, &plan, &wrong_header, &record).is_err());
    assert_eq!(owned(base.files()), original);
}

#[test]
fn subprocess_typed_mutation_helper() {
    let Ok(mode) = std::env::var("PHOTARA_PS2_TYPED_MODE") else {
        return;
    };
    let journal_path = std::env::var("PHOTARA_PS2_TYPED_JOURNAL").unwrap();
    let (base, plan, header, record) = fixture();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&journal_path)
        .unwrap();
    if mode == "append" {
        assert_eq!(
            append_typed(
                &mut file,
                &base,
                &plan,
                &header,
                &record,
                &record,
                AppendFault::AfterWrite
            ),
            Ok(AppendOutcome::OutcomeUnknown)
        );
        // No acknowledgment, teardown or unwinding after the completed write.
        std::process::exit(88);
    }
    assert_eq!(mode, "recover");
    let bytes = fs::read(&journal_path).unwrap();
    let recovered = recover(&bytes, &base, &plan, &header, &record).unwrap();
    assert_eq!(
        append_typed(
            &mut file,
            &base,
            &plan,
            &header,
            &record,
            &record,
            AppendFault::None
        ),
        Ok(AppendOutcome::Existing)
    );
    assert_eq!(fs::read(&journal_path).unwrap(), bytes);
    // The observation is the exact virtual candidate, not an on-disk package
    // publication or durable receipt. It includes original write/commit IDs.
    let output = std::env::var("PHOTARA_PS2_TYPED_OUTPUT").unwrap();
    fs::write(
        output,
        canonical_json(&owned(recovered.candidate().files())).unwrap(),
    )
    .unwrap();
    std::process::exit(89);
}

#[test]
fn fresh_process_typed_recovery_reuses_one_frame_and_exact_checkpoint_ids() {
    let (base, plan, header, record) = fixture();
    let root = tempfile::tempdir().unwrap();
    let journal_path = root.path().join("typed-journal.fixture");
    let header_bytes = journal::header_bytes(&header).unwrap();
    fs::write(&journal_path, &header_bytes).unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&journal_path)
        .unwrap();
    let mut bad = record.clone();
    bad.body["operation_id"] = json!(id(35999));
    assert!(
        append_typed(
            &mut file,
            &base,
            &plan,
            &header,
            &bad,
            &record,
            AppendFault::None
        )
        .is_err()
    );
    assert_eq!(fs::read(&journal_path).unwrap(), header_bytes);
    let mut poisoned = record.clone();
    poisoned.body["future_field"] = json!(true);
    let prior = encoded(&header, &poisoned);
    fs::write(&journal_path, &prior).unwrap();
    assert!(
        append_typed(
            &mut file,
            &base,
            &plan,
            &header,
            &record,
            &record,
            AppendFault::None
        )
        .is_err()
    );
    assert_eq!(fs::read(&journal_path).unwrap(), prior);
    fs::write(&journal_path, &header_bytes).unwrap();
    drop(file);
    let executable = std::env::current_exe().unwrap();
    let child = |mode: &str, output: &std::path::Path| {
        std::process::Command::new(&executable)
            .arg("--exact")
            .arg("planning::replay::framed::subprocess_typed_mutation_helper")
            .env("PHOTARA_PS2_TYPED_MODE", mode)
            .env("PHOTARA_PS2_TYPED_JOURNAL", &journal_path)
            .env("PHOTARA_PS2_TYPED_OUTPUT", output)
            .output()
            .unwrap()
    };
    let first_output = root.path().join("first-observation.json");
    let appended = child("append", &first_output);
    assert_eq!(appended.status.code(), Some(88), "{appended:?}");
    assert!(!first_output.exists());
    let original = fs::read(&journal_path).unwrap();
    assert_eq!(original, encoded(&header, &record));
    for output in [first_output, root.path().join("second-observation.json")] {
        let recovered = child("recover", &output);
        assert_eq!(recovered.status.code(), Some(89), "{recovered:?}");
        assert_eq!(
            fs::read(output).unwrap(),
            canonical_json(&owned(plan.candidate().files())).unwrap()
        );
        assert_eq!(fs::read(&journal_path).unwrap(), original);
        let scan = journal::scan_bound(&original, &header).unwrap();
        assert_eq!(scan.tail, Tail::Complete);
        assert_eq!(scan.records.as_slice(), std::slice::from_ref(&record));
    }
}
