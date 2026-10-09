#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
fn id(n: u64) -> Uuid {
    Uuid::parse_str(&format!("10000000-0000-4000-8000-{n:012}")).unwrap()
}
fn setup() -> (tempfile::TempDir, LocalDatabase) {
    let dir = tempfile::tempdir_in("/private/tmp").unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let database = id(20);
    let reg = Registration {
        database_id: database,
        epoch: id(21),
        device_id: id(10),
        workspace_slot_id: id(22),
        principal_id: bootstrap_ids(database).1,
        slot_scope_sha256: "a".repeat(64),
        max_snapshot_bytes: 1 << 20,
        max_records: 512,
        max_database_bytes: 16 << 20,
        coexistence_bytes: (32 << 20) + 65536,
        namespace_bytes: 65536,
        allow_faults: true,
    };
    let db = LocalDatabase::create_fresh(&dir.path().join("library.sqlite"), reg, 100).unwrap();
    (dir, db)
}
fn request(db: &LocalDatabase, operation: u64, library: Uuid, kind: &str) -> Value {
    let mut value = json!({"schema":"photara.library-lifecycle.v1","kind":kind,"authority":db.registration.authority(),"principal":db.registration.principal(),"operation_id":id(operation),"initiating_device_id":db.registration.device_id,"library_id":library,"name":"Second Library"});
    if kind == "rename" {
        value["expected_revision"] = json!("1");
    }
    value
}
#[test]
fn fresh_sql_lifecycle_atomic_original_retry_and_unknown_commit() {
    let (_dir, mut db) = setup();
    assert_eq!(db.settings().unwrap()["journal_mode"], "delete");
    let library = id(30);
    let create = request(&db, 40, library, "create");
    db.inject_fault("before-commit").unwrap();
    assert!(db.create(&create).is_err());
    assert_eq!(db.libraries().unwrap().len(), 1);
    db.inject_fault("after-commit").unwrap();
    assert!(db.create(&create).is_err());
    assert_eq!(db.libraries().unwrap().len(), 2);
    let receipt = db.create(&create).unwrap();
    assert_eq!(receipt["result"]["kind"], "created");
    let rename = request(&db, 41, library, "rename");
    let renamed = db.rename(&rename).unwrap();
    assert_eq!(renamed["result"]["revision"], "2");
    assert_eq!(db.create(&create).unwrap(), receipt);
    assert_eq!(db.rename(&rename).unwrap(), renamed);
    let mut changed = create.clone();
    changed["name"] = json!("changed");
    assert!(db.create(&changed).is_err());
    let stale = request(&db, 42, library, "rename");
    assert_eq!(
        db.rename(&stale).unwrap()["result"],
        json!({"kind":"rejected","reason":"revision-conflict"})
    );
    let mut invalid = request(&db, 43, library, "create");
    invalid["principal"]["id"] = json!(id(900));
    assert!(db.create(&invalid).is_err());
    let reg = db.registration.clone();
    let pins = db.pins();
    let path = db.path.clone();
    let stored = db.load().unwrap();
    drop(db);
    let mut reopened = LocalDatabase::reopen(&path, reg, pins).unwrap();
    assert_eq!(reopened.load().unwrap(), stored);
    assert_eq!(reopened.create(&create).unwrap(), receipt);
    assert!(
        reopened
            .connection
            .execute("DELETE FROM local_activation_slots", [])
            .is_err()
    );
    assert!(
        reopened
            .connection
            .execute("UPDATE lifecycle_receipts SET result='rejected'", [])
            .is_err()
    );
}
#[test]
fn current_transferred_bootstrap_controller_not_historical_principal() {
    let (_dir, mut db) = setup();
    let default = bootstrap_ids(db.registration.database_id).0;
    let initial = db.registration.principal_id;
    let next = id(99);
    db.connection.execute("UPDATE library_contract_state SET local_principal_id=?,authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=101 WHERE library_id=?",params![next.as_bytes(),default.as_bytes()]).unwrap();
    assert!(db.create(&request(&db, 50, id(31), "create")).is_err());
    db.registration.principal_id = next;
    let create = request(&db, 51, id(31), "create");
    assert_eq!(db.create(&create).unwrap()["result"]["kind"], "created");
    let owner: Vec<u8> = db
        .connection
        .query_row(
            "SELECT local_principal_id FROM library_contract_state WHERE library_id=?",
            [id(31).as_bytes()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owner, next.as_bytes());
    assert_ne!(owner, initial.as_bytes());
}
fn record(n: u64, kind: &str, body: Value) -> Value {
    let mut value = json!({"id":id(n),"kind":kind,"version":2,"body":null});
    value["body"] = body;
    value
}
fn add(body: &mut Value, record: Value) -> Value {
    let reference = v2::record_ref(&record).unwrap();
    let rows = body["records"].as_array_mut().unwrap();
    rows.push(record);
    rows.sort_by_key(|v| v["id"].as_str().unwrap().to_owned());
    reference
}
fn encode_snapshot(db: &LocalDatabase, body: &Value) -> Snapshot {
    Snapshot::from_body(body.clone(), db.registration.limits()).unwrap()
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One SQL publication sequence covers atomic rollback, unknown and access-change outcomes"
)]
fn sql_slot_exact_cas_projection_atomic_unknown_and_authority_recheck() {
    let (_dir, mut db) = setup();
    let library = bootstrap_ids(db.registration.database_id).0;
    let target = json!({"kind":"library","context":{"authority":db.registration.authority(),"principal":db.registration.principal(),"library_id":library}});
    let bytes = db.load().unwrap();
    let empty = Snapshot::parse(&bytes, db.registration.limits()).unwrap();
    let mut body = empty.body().clone();
    let intent = add(
        &mut body,
        record(
            200,
            "ActivationIntent",
            json!({"device_id":db.registration.device_id,"workspace_slot_id":db.registration.workspace_slot_id,"request_generation":"1","expected_committed_generation":"0","slot_scope_sha256":db.registration.slot_scope_sha256,"source_active":null,"target":target,"source_attachment":null,"confirmation_basis":null}),
        ),
    );
    let pending = add(
        &mut body,
        record(
            201,
            "ActivationProgress",
            json!({"intent":intent,"stage":"Preparing","source_saved":null,"capsule":null,"target_ready":null}),
        ),
    );
    body["pending"] = pending;
    body["revision"] = json!("1");
    body["request_generation"] = json!("1");
    let preparing = encode_snapshot(&db, &body);
    assert!(db.publish(&"0".repeat(64), preparing.bytes()).is_err());
    assert_eq!(db.load().unwrap(), bytes);
    db.inject_fault("before-commit").unwrap();
    assert!(db.publish(empty.sha256(), preparing.bytes()).is_err());
    assert_eq!(db.load().unwrap(), bytes);
    db.inject_fault("before-activation-commit").unwrap();
    db.publish(empty.sha256(), preparing.bytes()).unwrap();
    let proof = db.context(&target).unwrap();
    let context = add(&mut body, record(202, "LocalContextProof", proof));
    let ready = add(
        &mut body,
        record(
            203,
            "SelectionReady",
            json!({"intent":intent,"context_proof":context,"project_open":null}),
        ),
    );
    body["pending"] = add(
        &mut body,
        record(
            204,
            "ActivationProgress",
            json!({"intent":intent,"stage":"TargetReady","source_saved":null,"capsule":null,"target_ready":ready}),
        ),
    );
    body["revision"] = json!("2");
    let ready_snapshot = encode_snapshot(&db, &body);
    db.publish(preparing.sha256(), ready_snapshot.bytes())
        .unwrap();
    let active = add(
        &mut body,
        record(
            205,
            "ActiveSelection",
            json!({"device_id":db.registration.device_id,"workspace_slot_id":db.registration.workspace_slot_id,"committed_generation":"1","slot_scope_sha256":db.registration.slot_scope_sha256,"activation_id":id(200),"target":target,"ready":ready}),
        ),
    );
    add(
        &mut body,
        record(
            206,
            "ActivationReceipt",
            json!({"intent":intent,"outcome":"Activated","old_committed_generation":"0","new_committed_generation":"1","source_saved":null,"capsule":null,"target_ready":ready,"active":active}),
        ),
    );
    body["active"] = active;
    body["pending"] = Value::Null;
    body["revision"] = json!("3");
    body["committed_generation"] = json!("1");
    let activated = encode_snapshot(&db, &body);
    assert!(
        db.publish(ready_snapshot.sha256(), activated.bytes())
            .is_err()
    );
    assert_eq!(db.load().unwrap(), ready_snapshot.bytes());
    db.inject_fault("after-activation-commit").unwrap();
    assert!(
        db.publish(ready_snapshot.sha256(), activated.bytes())
            .is_err()
    );
    assert_eq!(db.load().unwrap(), activated.bytes());
    db.publish(ready_snapshot.sha256(), activated.bytes())
        .unwrap();
    let selected: Vec<u8> = db
        .connection
        .query_row(
            "SELECT active_library_id FROM local_activation_slots",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(selected, library.as_bytes());
    let reg = db.registration.clone();
    let pins = db.pins();
    let path = db.path.clone();
    drop(db);
    let mut db = LocalDatabase::reopen(&path, reg, pins).unwrap();
    assert_eq!(db.load().unwrap(), activated.bytes());
    // Stored context proof cannot authorize use after controller transfer.
    db.connection.execute("UPDATE library_contract_state SET local_principal_id=?,authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=101 WHERE library_id=?",params![id(99).as_bytes(),library.as_bytes()]).unwrap();
    assert!(db.context(&target).is_err());
    let mut next = body.clone();
    next["revision"] = json!("4");
    let next = encode_snapshot(&db, &next);
    assert!(db.publish(activated.sha256(), next.bytes()).is_err());
    assert_eq!(db.load().unwrap(), activated.bytes());
}
#[test]
fn project_access_provenance_is_actual_registered_grant_and_pins_refuse_replacement() {
    let (_dir, mut db) = setup();
    let library = bootstrap_ids(db.registration.database_id).0;
    let registration = ProjectRegistration {
        library_id: library,
        project_id: id(300),
        association_commit_id: id(301),
        association_sha256: "b".repeat(64),
        grant_id: id(302),
    };
    db.register_project(&registration, 101).unwrap();
    let target = json!({"kind":"project","context":{"authority":db.registration.authority(),"principal":db.registration.principal(),"library_id":library},"project":{"library_id":library,"project_id":id(300),"incarnation_id":id(303),"graph_id":id(304),"registration_sha256":"c".repeat(64)}});
    let provenance = db.provenance(&target).unwrap();
    assert_eq!(provenance["grant_ref"]["reference_id"], id(302).to_string());
    assert_eq!(
        provenance["principal"]["principal_id"],
        db.registration.principal_id.to_string()
    );
    db.connection.execute("UPDATE project_access_grants SET action_mask=0,local_revision=local_revision+1,updated_at=102 WHERE grant_id=?",[id(302).as_bytes()]).unwrap();
    assert!(db.context(&target).is_err());
    assert!(db.provenance(&target).is_err());
    let moved = db.path.with_extension("retained");
    std::fs::rename(&db.path, &moved).unwrap();
    File::create(&db.path).unwrap();
    assert!(db.load().is_err());
}
