//! Local PS4 codec uses existing integrated selectors, without granting live proof.
use super::*;
use v1_3::activation::{self, Snapshot, Transition, record_ref, validate_transition};
fn limits() -> activation::Limits {
    activation::Limits {
        json: support::limits().json,
        max_records: 128,
        max_snapshot_bytes: 1 << 20,
    }
}
fn row(n: u64, kind: &str, body: Value) -> Value {
    let mut result = json!({"id":uid(200_000+n),"kind":kind,"version":1,"body":null});
    result["body"] = body;
    result
}
fn add(body: &mut Value, record: Value) -> Value {
    let r = record_ref(&record).unwrap();
    let rows = body["records"].as_array_mut().unwrap();
    rows.push(record);
    rows.sort_by_key(|v| v["id"].as_str().unwrap().to_owned());
    r
}
#[track_caller]
fn snapshot(body: &Value) -> Snapshot {
    Snapshot::from_body(body.clone(), limits()).unwrap()
}
fn step(old: &Snapshot, body: &Value, expected: Transition) -> Snapshot {
    let next = snapshot(body);
    assert_eq!(
        validate_transition(old, &next, old.sha256(), &"a".repeat(64)).unwrap(),
        expected
    );
    next
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One existing selector corpus covers complete activation and related refusal cases"
)]
fn activation_exact_snapshot_generation_and_original_identity() {
    let fixture = fixture();
    let package = package(&fixture);
    let (request, _) = request();
    let coord = request["expected"].clone();
    let graph = coord["graphs"].as_object().unwrap().keys().next().unwrap();
    let target = json!({"library_id":package.commit["root_set"]["library_id"],"project_id":package.manifest["project_id"],"incarnation_id":uid(201_000),"graph_id":graph,"registration_sha256":"b".repeat(64)});
    let mut body = json!({"device_id":uid(202_000),"workspace_slot_id":uid(202_001),"revision":"0","request_generation":"0","committed_generation":"0","authority_scope_sha256":"a".repeat(64),"active":null,"pending":null,"records":[]});
    let empty = snapshot(&body);
    let intent = row(
        1,
        "ActivationIntent",
        json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":"1","expected_committed_generation":"0","authority_scope_sha256":body["authority_scope_sha256"],"source_active":null,"target":target,"source_attachment":null,"confirmation_basis":null}),
    );
    let ir = add(&mut body, intent);
    let preparing = row(
        2,
        "ActivationProgress",
        json!({"intent":ir,"stage":"Preparing","source_saved":null,"capsule":null,"target_open":null}),
    );
    body["pending"] = add(&mut body, preparing);
    body["revision"] = json!("1");
    body["request_generation"] = json!("1");
    let prepared = step(&empty, &body, Transition::Prepared);
    assert_eq!(
        validate_transition(&empty, &prepared, &"0".repeat(64), &"a".repeat(64)).unwrap_err(),
        package::PackageError::Integrity
    );
    let mut changed = body.clone();
    changed["records"][0]["body"]["target"]["registration_sha256"] = json!("c".repeat(64));
    assert!(Snapshot::from_body(changed, limits()).is_err());
    let vr = add(
        &mut body,
        row(
            3,
            "SessionView",
            activation::default_view(&json!(uid(202_000)), &target),
        ),
    );
    let binding = json!({"project_id":target["project_id"],"incarnation_id":target["incarnation_id"],"owner_epoch":uid(202_002),"owner":{"attachment_id":uid(202_003),"attachment_generation":"1","principal":{"kind":"local-controller","principal_id":uid(202_004)}}});
    let op = add(
        &mut body,
        row(
            4,
            "TargetOpenProof",
            json!({"binding":binding,"registration_sha256":target["registration_sha256"],"selected_head":package.head,"selected_commit":package.commit,"accepted":{"coordinate":coord,"mutation":null,"accepted_frame":null},"graph_id":graph,"view":vr}),
        ),
    );
    body["pending"] = add(
        &mut body,
        row(
            5,
            "ActivationProgress",
            json!({"intent":ir,"stage":"TargetReady","source_saved":null,"capsule":null,"target_open":op}),
        ),
    );
    body["revision"] = json!("2");
    let ready = step(&prepared, &body, Transition::Progressed);
    let ar = add(
        &mut body,
        row(
            6,
            "ActiveSession",
            json!({"device_id":uid(202_000),"workspace_slot_id":uid(202_001),"committed_generation":"1","authority_scope_sha256":"a".repeat(64),"library_id":target["library_id"],"project_id":target["project_id"],"incarnation_id":target["incarnation_id"],"graph_id":graph,"activation_id":ir["id"],"view":vr,"target_open":op}),
        ),
    );
    add(
        &mut body,
        row(
            7,
            "ActivationReceipt",
            json!({"intent":ir,"outcome":"Activated","old_committed_generation":"0","new_committed_generation":"1","source_saved":null,"capsule":null,"target_open":op,"active":ar}),
        ),
    );
    body["active"] = ar;
    body["pending"] = Value::Null;
    body["revision"] = json!("3");
    body["committed_generation"] = json!("1");
    let active = step(&ready, &body, Transition::Activated);
    assert_eq!(
        active.sha256(),
        "77ae2a1b78ad0f0e037dcc57d3384175f3f9788388b61db2ac902d29b19a6596"
    );
    assert!(validate_transition(&prepared, &active, prepared.sha256(), &"a".repeat(64)).is_err());
    assert_eq!(
        validate_transition(&active, &active, active.sha256(), &"a".repeat(64)).unwrap(),
        Transition::Retry
    );
    assert_eq!(
        active.receipt(ir["id"].as_str().unwrap()).unwrap()["body"]["active"],
        body["active"]
    );
    let mut bytes = active.bytes().to_vec();
    bytes.push(b'\n');
    assert!(Snapshot::parse(&bytes, limits()).is_err());
    let mut raw: Value = serde_json::from_slice(active.bytes()).unwrap();
    raw["body_sha256"] = json!("0".repeat(64));
    assert!(Snapshot::parse(&encode(&raw), limits()).is_err());
    let mut wrong = body.clone();
    wrong["committed_generation"] = json!("2");
    assert!(Snapshot::from_body(wrong, limits()).is_err());
    let mut wrong = body.clone();
    wrong["records"][0]["unknown"] = json!(true);
    assert!(Snapshot::from_body(wrong, limits()).is_err());
    let mut wrong = body.clone();
    wrong["records"][0]["version"] = json!(2);
    assert!(Snapshot::from_body(wrong, limits()).is_err());
    // An optional future view remains inert, and must not pass the use validator.
    let optional = row(10, "SessionView", json!({"future":"view"}));
    let mut optional_body = body.clone();
    add(&mut optional_body, optional.clone());
    assert!(Snapshot::from_body(optional_body, limits()).is_ok());
    assert!(!activation::validate_view(
        &optional,
        &body["device_id"],
        &target,
        &json!({"id":graph,"nodes":[]})
    ));
    // Cancellation persists Refused then its original receipt without activation.
    let mut refusal = prepared.body().clone();
    refusal["pending"] = add(
        &mut refusal,
        row(
            20,
            "ActivationProgress",
            json!({"intent":ir,"stage":"Refused","source_saved":null,"capsule":null,"target_open":null}),
        ),
    );
    refusal["revision"] = json!("2");
    let refused = step(&prepared, &refusal, Transition::Progressed);
    add(
        &mut refusal,
        row(
            21,
            "ActivationReceipt",
            json!({"intent":ir,"outcome":"RetainedCurrent","old_committed_generation":"0","new_committed_generation":"0","source_saved":null,"capsule":null,"target_open":null,"active":null}),
        ),
    );
    refusal["pending"] = Value::Null;
    refusal["revision"] = json!("3");
    step(&refused, &refusal, Transition::Retained);
    let mut retained = body.clone();
    let view = row(
        30,
        "SessionView",
        activation::default_view(&body["device_id"], &target),
    );
    add(&mut retained, view);
    retained["revision"] = json!("4");
    let original_record = step(&active, &retained, Transition::Progressed);
    retained["revision"] = json!("5");
    retained["records"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["body"]["pan_x"] = json!(1);
    let replaced = snapshot(&retained);
    assert!(
        validate_transition(
            &original_record,
            &replaced,
            original_record.sha256(),
            &"a".repeat(64)
        )
        .is_err()
    );
    let mut no_receipt = body.clone();
    no_receipt["records"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["kind"] != "ActivationReceipt");
    assert!(Snapshot::from_body(no_receipt, limits()).is_err());
    let mut confirmation = body.clone();
    add(
        &mut confirmation,
        row(
            31,
            "ConfirmationEvidence",
            json!({"activation_id":uid(220_000),"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":"2","expected_committed_generation":"1","authority_scope_sha256":body["authority_scope_sha256"],"source_active":body["active"],"source_binding":binding,"accepted":{"coordinate":coord,"mutation":null,"accepted_frame":null},"prior_saved":null,"target":target}),
        ),
    );
    assert!(Snapshot::from_body(confirmation, limits()).is_err());
    let mut small = limits();
    small.max_records = 1;
    assert!(Snapshot::parse(active.bytes(), small).is_err());
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One authenticated existing checkpoint supplies Saved/confirmation/capsule refusal cases"
)]
fn activation_saved_confirmation_and_capsule_exact_evidence() {
    use super::super::journal::Journal;
    use super::journal_tests as jt;
    let base = fixture();
    let old = verify(&base);
    let (intent, receipt) = request();
    let reader = support::limits();
    let plan = plan::compile_inner(&old, &intent, &receipt, attempt(&old, 1000), budget()).unwrap();
    let mut header = jt::header(&old.package);
    header["journal_id"] = receipt["journal_id"].clone();
    let mut bytes = Journal::create(&header, jt::limits()).unwrap();
    let identity0 = identity(&old.package);
    let context0 = jt::context(&base, &identity0, &reader);
    let journal = Journal::read(&bytes, &header, jt::limits()).unwrap();
    let verified = journal.verify(&old.package, &context0).unwrap();
    let original = verified
        .prepare_intent(&plan, jt::record(92_001), jt::limits())
        .unwrap();
    bytes.extend(original.bytes());
    let selected = selected(&plan, 6);
    let observed = super::snapshot(&base, &selected);
    let identity1 = identity(&selected);
    let context1 = jt::context(&observed, &identity1, &reader);
    let journal = Journal::read(&bytes, &header, jt::limits()).unwrap();
    let verified = journal.verify(&selected, &context1).unwrap();
    let completed = verified
        .prepare_receipt(
            &plan,
            jt::record(92_001).record_id,
            jt::record(92_002),
            jt::limits(),
        )
        .unwrap();
    let next = plan
        .verify_next_original(
            selected.clone(),
            identity(&selected),
            &observed,
            &base.registration,
            v1_3::DirectoryObservation {
                device: 7,
                inode: 900,
            },
            frames(),
            &reader,
        )
        .unwrap();
    let coord = core::original_coordinate(&next, &attempt(&next, 2000).planner, budget()).unwrap();
    let graph = plan
        .prepared
        .authored
        .values()
        .find(|v| v["graph"].is_object())
        .unwrap()["graph"]
        .clone();
    let graph_id = graph["id"].clone();
    let binding = json!({"project_id":selected.manifest["project_id"],"incarnation_id":header["incarnation_id"],"owner_epoch":uid(202_002),"owner":{"attachment_id":uid(202_003),"attachment_generation":"1","principal":{"kind":"local-controller","principal_id":uid(202_004)}}});
    let accepted = json!({"coordinate":coord,"mutation":null,"accepted_frame":null});
    let target = json!({"library_id":selected.commit["root_set"]["library_id"],"project_id":selected.manifest["project_id"],"incarnation_id":binding["incarnation_id"],"graph_id":graph_id,"registration_sha256":"b".repeat(64)});
    let mut body = json!({"device_id":uid(202_000),"workspace_slot_id":uid(202_001),"revision":"3","request_generation":"1","committed_generation":"1","authority_scope_sha256":"a".repeat(64),"active":null,"pending":null,"records":[]});
    let ir = add(
        &mut body,
        row(
            1,
            "ActivationIntent",
            json!({"device_id":uid(202_000),"workspace_slot_id":uid(202_001),"request_generation":"1","expected_committed_generation":"0","authority_scope_sha256":"a".repeat(64),"source_active":null,"target":target,"source_attachment":null,"confirmation_basis":null}),
        ),
    );
    let vr = add(
        &mut body,
        row(
            2,
            "SessionView",
            activation::default_view(&json!(uid(202_000)), &target),
        ),
    );
    let tr = add(
        &mut body,
        row(
            3,
            "TargetOpenProof",
            json!({"binding":binding,"registration_sha256":target["registration_sha256"],"selected_head":selected.head,"selected_commit":selected.commit,"accepted":accepted,"graph_id":graph_id,"view":vr}),
        ),
    );
    let ar = add(
        &mut body,
        row(
            4,
            "ActiveSession",
            json!({"device_id":uid(202_000),"workspace_slot_id":uid(202_001),"committed_generation":"1","authority_scope_sha256":"a".repeat(64),"library_id":target["library_id"],"project_id":target["project_id"],"incarnation_id":target["incarnation_id"],"graph_id":graph_id,"activation_id":ir["id"],"view":vr,"target_open":tr}),
        ),
    );
    add(
        &mut body,
        row(
            5,
            "ActivationReceipt",
            json!({"intent":ir,"outcome":"Activated","old_committed_generation":"0","new_committed_generation":"1","source_saved":null,"capsule":null,"target_open":tr,"active":ar}),
        ),
    );
    body["active"] = ar.clone();
    let saved = row(
        6,
        "SavedProof",
        json!({"binding":binding,"accepted":accepted,"checkpoint_intent":{"value":original.record().value(),"record_checksum":original.record().checksum()},"checkpoint_receipt":{"value":completed.record().value(),"record_checksum":completed.record().checksum()},"registration_sha256":target["registration_sha256"]}),
    );
    let base_body = body.clone();
    let sr = add(&mut body, saved.clone());
    snapshot(&body);
    let capsule = row(
        7,
        "RollbackCapsule",
        json!({"source_active":ar,"source_saved":sr,"view":vr,"graph_snapshot":graph,"registration_sha256":target["registration_sha256"]}),
    );
    add(&mut body, capsule.clone());
    snapshot(&body);
    let mut target2 = target.clone();
    target2["project_id"] = json!(uid(250_000));
    target2["incarnation_id"] = json!(uid(250_001));
    let confirmation = row(
        8,
        "ConfirmationEvidence",
        json!({"activation_id":uid(250_002),"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":"2","expected_committed_generation":"1","authority_scope_sha256":body["authority_scope_sha256"],"source_active":ar,"source_binding":binding,"accepted":accepted,"prior_saved":sr,"target":target2}),
    );
    add(&mut body, confirmation.clone());
    snapshot(&body);
    for path in ["checksum", "binding", "receipt", "coordinate"] {
        let mut bad = saved.clone();
        match path {
            "checksum" => {
                bad["body"]["checkpoint_intent"]["record_checksum"] = json!("0".repeat(64));
            }
            "binding" => bad["body"]["binding"]["project_id"] = json!(uid(250_009)),
            "receipt" => {
                bad["body"]["checkpoint_receipt"]["value"]["body"]["selected_head"]["commit_id"] =
                    json!(uid(250_010));
            }
            _ => bad["body"]["accepted"]["coordinate"]["revision"] = json!("999"),
        }
        let mut candidate = base_body.clone();
        add(&mut candidate, bad);
        assert!(Snapshot::from_body(candidate, limits()).is_err(), "{path}");
    }
    let mut saved_only = base_body.clone();
    add(&mut saved_only, saved);
    let mut bad = capsule.clone();
    bad["body"]["graph_snapshot"]["name"] = json!("changed bytes");
    let mut candidate = saved_only.clone();
    add(&mut candidate, bad);
    assert!(Snapshot::from_body(candidate, limits()).is_err());
    let mut bad = capsule;
    bad["body"]["source_saved"] = vr;
    let mut candidate = saved_only.clone();
    add(&mut candidate, bad);
    assert!(Snapshot::from_body(candidate, limits()).is_err());
    for field in ["accepted", "prior_saved", "source_binding"] {
        let mut bad = confirmation.clone();
        match field {
            "accepted" => bad["body"][field]["coordinate"]["revision"] = json!("0"),
            "prior_saved" => bad["body"][field] = Value::Null,
            _ => bad["body"][field]["owner_epoch"] = json!(uid(250_011)),
        }
        let mut candidate = saved_only.clone();
        add(&mut candidate, bad);
        assert!(Snapshot::from_body(candidate, limits()).is_err(), "{field}");
    }
}
