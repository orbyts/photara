#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Bounded deterministic codec assertions"
)]
use super::*;
fn uid(n: u64) -> Value {
    json!(format!("10000000-0000-4000-8000-{n:012}"))
}
fn limits() -> Limits {
    Limits {
        json: JsonLimits {
            max_bytes: 1 << 20,
            max_depth: 64,
            max_members: 512,
            max_array_elements: 512,
        },
        max_records: 128,
        max_snapshot_bytes: 1 << 20,
    }
}
fn record(n: u64, kind: &str, body: Value) -> Value {
    let mut record = json!({"id":uid(n),"kind":kind,"version":2});
    record["body"] = body;
    record
}
fn add(body: &mut Value, record: Value) -> Value {
    let r = record_ref(&record).unwrap();
    body["records"].as_array_mut().unwrap().push(record);
    body["records"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    r
}
fn empty() -> Snapshot {
    Snapshot::from_body(json!({"device_id":uid(1),"workspace_slot_id":uid(2),"slot_scope_sha256":"a".repeat(64),"revision":"0","request_generation":"0","committed_generation":"0","active":null,"pending":null,"records":[]}),limits()).unwrap()
}
fn target(library: u64) -> Value {
    json!({"kind":"library","context":{"authority":{"kind":"local","database_id":uid(3),"epoch":uid(4)},"principal":{"kind":"local","id":uid(5)},"library_id":uid(library)}})
}
fn selected(old: &Snapshot, library: u64, base: u64) -> Vec<Snapshot> {
    let mut body = old.body().clone();
    let target = target(library);
    let request = number(&body["request_generation"]).unwrap() + 1;
    let generation = number(&body["committed_generation"]).unwrap();
    let intent_body = json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"slot_scope_sha256":body["slot_scope_sha256"],"request_generation":request.to_string(),"expected_committed_generation":generation.to_string(),"source_active":body["active"],"target":target,"source_attachment":null,"confirmation_basis":null});
    let intent = add(&mut body, record(base, "ActivationIntent", intent_body));
    body["request_generation"] = json!(request.to_string());
    body["revision"] = json!((number(&body["revision"]).unwrap() + 1).to_string());
    body["pending"] = add(
        &mut body,
        record(
            base + 1,
            "ActivationProgress",
            json!({"intent":intent,"stage":"Preparing","source_saved":null,"capsule":null,"target_ready":null}),
        ),
    );
    let preparing = Snapshot::from_body(body.clone(), limits()).unwrap();
    let context = add(
        &mut body,
        record(
            base + 2,
            "LocalContextProof",
            json!({"context":target["context"],"library_revision":"1","contract_revision":"1","authorization_generation":"1"}),
        ),
    );
    let ready = add(
        &mut body,
        record(
            base + 3,
            "SelectionReady",
            json!({"intent":intent,"context_proof":context,"project_open":null}),
        ),
    );
    body["pending"] = add(
        &mut body,
        record(
            base + 4,
            "ActivationProgress",
            json!({"intent":intent,"stage":"TargetReady","source_saved":null,"capsule":null,"target_ready":ready}),
        ),
    );
    body["revision"] = json!((number(&body["revision"]).unwrap() + 1).to_string());
    let prepared = Snapshot::from_body(body.clone(), limits()).unwrap();
    let active_body = json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"slot_scope_sha256":body["slot_scope_sha256"],"committed_generation":(generation+1).to_string(),"activation_id":uid(base),"target":target,"ready":ready});
    let active = add(&mut body, record(base + 5, "ActiveSelection", active_body));
    add(
        &mut body,
        record(
            base + 6,
            "ActivationReceipt",
            json!({"intent":intent,"outcome":"Activated","old_committed_generation":generation.to_string(),"new_committed_generation":(generation+1).to_string(),"source_saved":null,"capsule":null,"target_ready":ready,"active":active}),
        ),
    );
    body["active"] = active;
    body["pending"] = Value::Null;
    body["committed_generation"] = json!((generation + 1).to_string());
    body["revision"] = json!((number(&body["revision"]).unwrap() + 1).to_string());
    vec![
        preparing,
        prepared,
        Snapshot::from_body(body, limits()).unwrap(),
    ]
}
#[test]
fn v2_library_only_cross_library_exact_original_and_scopes() {
    let initial = empty();
    let mut current = initial.clone();
    for library in [10, 11] {
        for next in selected(&current, library, library * 100) {
            validate_transition(&current, &next, current.sha256(), &"a".repeat(64)).unwrap();
            current = next;
        }
    }
    assert!(
        current.body()["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| !matches!(
                r["kind"].as_str(),
                Some("SavedProof" | "TargetOpenProof" | "RollbackCapsule")
            ))
    );
    assert_eq!(current.body()["committed_generation"], "2");
    assert!(validate_transition(&initial, &current, initial.sha256(), &"a".repeat(64)).is_err());
    assert!(validate_transition(&current, &current, &"0".repeat(64), &"a".repeat(64)).is_err());
    let mut bad = current.body().clone();
    bad["committed_generation"] = json!("3");
    assert!(Snapshot::from_body(bad, limits()).is_err());
    let mut bad = initial.body().clone();
    bad["request_generation"] = json!("9223372036854775808");
    assert!(Snapshot::from_body(bad, limits()).is_err());
    let mut bad = selected(&initial, 10, 100)[0].body().clone();
    bad["records"][0]["body"]["target"]["context"]["authority"]["kind"] = json!("cloud");
    assert!(Snapshot::from_body(bad, limits()).is_err());
    let mut bad = current.body().clone();
    bad["records"][2]["body"]["extra"] = json!(true);
    assert!(Snapshot::from_body(bad, limits()).is_err());
    let mut v1 = initial.body().clone();
    v1.as_object_mut().unwrap().remove("slot_scope_sha256");
    v1["authority_scope_sha256"] = json!("a".repeat(64));
    let v1 = super::super::Snapshot::from_body(
        v1,
        super::super::Limits {
            json: limits().json,
            max_records: 128,
            max_snapshot_bytes: 1 << 20,
        },
    )
    .unwrap();
    assert!(Snapshot::parse(v1.bytes(), limits()).is_err());
    assert!(
        super::super::Snapshot::parse(
            initial.bytes(),
            super::super::Limits {
                json: limits().json,
                max_records: 128,
                max_snapshot_bytes: 1 << 20
            }
        )
        .is_err()
    );
}
#[test]
fn frozen_receipt_union_accepts_local_controller_and_refuses_shape_substitution() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../../../docs/architecture/proposals/ps2/route/operation-four.json"
    ))
    .unwrap();
    let mut receipt = corpus["records"]["receipt-4"]["input"].clone();
    let identity = crate::package::v1_3::SelectionIdentity {
        project: crate::package::PackageUuid::parse(receipt["project_id"].as_str().unwrap())
            .unwrap(),
        library: crate::package::PackageUuid::parse(receipt["library_id"].as_str().unwrap())
            .unwrap(),
        bootstrap_sha256: crate::package::Sha256Hex::parse(
            receipt["bootstrap_sha256"].as_str().unwrap(),
        )
        .unwrap(),
    };
    super::super::super::operations::validate_receipt(&receipt, &identity).unwrap();
    for key in ["principal", "grantor"] {
        receipt["provenance"][key] = json!({"kind":"local-controller","principal_id":uid(5)});
    }
    super::super::super::operations::validate_receipt(&receipt, &identity).unwrap();
    receipt["provenance"]["principal"]["account_id"] = uid(6);
    assert!(super::super::super::operations::validate_receipt(&receipt, &identity).is_err());
}

// Rehash a changed immutable record and all downstream references so a refusal
// exercises semantic binding, rather than merely a stale digest.
fn replace_reference(value: &mut Value, old: &Value, new: &Value) {
    if value == old {
        *value = new.clone();
    } else {
        match value {
            Value::Array(values) => {
                for value in values {
                    replace_reference(value, old, new);
                }
            }
            Value::Object(values) => {
                for value in values.values_mut() {
                    replace_reference(value, old, new);
                }
            }
            _ => {}
        }
    }
}
fn change_record(body: &mut Value, index: usize, change: impl FnOnce(&mut Value)) {
    let original = body["records"].as_array().unwrap().clone();
    change(&mut body["records"][index]["body"]);
    for (index, record) in original.iter().enumerate() {
        let old = record_ref(record).unwrap();
        let new = record_ref(&body["records"][index]).unwrap();
        if old != new {
            replace_reference(body, &old, &new);
        }
    }
}
#[test]
fn v2_context_binding_stale_transition_and_retained_original() {
    let initial = empty();
    let stages = selected(&initial, 10, 100);
    let preparing = &stages[0];
    let ready = &stages[1];
    let scope = "a".repeat(64);
    for field in ["library_id", "principal", "authority"] {
        let mut body = ready.body().clone();
        change_record(&mut body, 2, |proof| {
            proof["context"][field] = match field {
                "principal" => json!({"kind":"local","id":uid(99)}),
                "authority" => json!({"kind":"local","database_id":uid(99),"epoch":uid(4)}),
                _ => uid(99),
            };
        });
        assert!(Snapshot::from_body(body, limits()).is_err(), "{field}");
    }
    let mut stale = ready.body().clone();
    stale["revision"] = json!("9");
    let stale = Snapshot::from_body(stale, limits()).unwrap();
    assert!(validate_transition(preparing, &stale, preparing.sha256(), &scope).is_err());
    assert!(validate_transition(preparing, ready, preparing.sha256(), &"b".repeat(64)).is_err());

    let intent = preparing.body()["records"][0].clone();
    let intent_ref = record_ref(&intent).unwrap();
    let mut refused = preparing.body().clone();
    refused["revision"] = json!("2");
    refused["pending"] = add(
        &mut refused,
        record(
            107,
            "ActivationProgress",
            json!({"intent":intent_ref,"stage":"Refused","source_saved":null,"capsule":null,"target_ready":null}),
        ),
    );
    let refused = Snapshot::from_body(refused, limits()).unwrap();
    assert_eq!(
        validate_transition(preparing, &refused, preparing.sha256(), &scope).unwrap(),
        Transition::Progressed
    );
    let mut terminal = refused.body().clone();
    terminal["revision"] = json!("3");
    terminal["pending"] = Value::Null;
    add(
        &mut terminal,
        record(
            108,
            "ActivationReceipt",
            json!({"intent":intent_ref,"outcome":"RetainedCurrent","old_committed_generation":"0","new_committed_generation":"0","source_saved":null,"capsule":null,"target_ready":null,"active":null}),
        ),
    );
    let terminal = Snapshot::from_body(terminal, limits()).unwrap();
    assert_eq!(
        validate_transition(&refused, &terminal, refused.sha256(), &scope).unwrap(),
        Transition::Retained
    );
    assert_eq!(
        validate_transition(&terminal, &terminal, terminal.sha256(), &scope).unwrap(),
        Transition::Retry
    );
    assert_eq!(terminal.record(&intent_ref).unwrap(), &intent);
    assert_eq!(terminal.body()["committed_generation"], "0");
    // A structurally valid terminal receipt cannot skip the durable Refused stage.
    let mut skipped = terminal.body().clone();
    skipped["revision"] = json!("2");
    let skipped = Snapshot::from_body(skipped, limits()).unwrap();
    assert!(validate_transition(preparing, &skipped, preparing.sha256(), &scope).is_err());
}
