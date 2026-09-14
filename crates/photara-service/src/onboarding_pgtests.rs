//! Disposable onboarding proof; no real token, credential or user database.
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use super::*;
use crate::onboarding::*;
use photara_core::contracts::*;
use uuid::Uuid;

fn id() -> OperationId {
    OperationId::from_uuid(Uuid::new_v4()).unwrap()
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_onboarding_upgrade_preserves_deployed_ledger() {
    let base = std::env::var("PHOTARA_TEST_MIGRATOR_URL").unwrap();
    assert!(base.contains("/private/tmp/") && base.contains("/photara_cxt3c?"));
    let url = base.replace("/photara_cxt3c?", "/photara_cxt3c_upgrade?");
    let db = storexa::Database::connect(storexa::DatabaseConfig::from_url(&url).unwrap())
        .await
        .unwrap();
    let mut tx = db.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    let baseline = sqlx::migrate::Migrator::with_migrations(
        MIGRATOR
            .iter()
            .filter(|m| m.version <= 7)
            .cloned()
            .collect::<Vec<_>>(),
    );
    baseline.run(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO photara.schema_metadata VALUES(true,'photara.service.g2',1,1,'photara.canonical-json.v1',CURRENT_TIMESTAMP)").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO photara.normalization_policies VALUES(1,'16.0.0',$1,$2)")
        .bind(hash(POLICY.as_bytes()).to_vec())
        .bind(POLICY)
        .execute(&mut *tx)
        .await
        .unwrap();
    let deployed = sqlx::migrate::Migrator::with_migrations(
        MIGRATOR
            .iter()
            .filter(|m| m.version <= 13)
            .cloned()
            .collect::<Vec<_>>(),
    );
    deployed.run(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let before: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version,checksum FROM public._sqlx_migrations ORDER BY version")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(before.len(), 13);
    migrate(storexa::DatabaseConfig::from_url(&url).unwrap())
        .await
        .unwrap();
    migrate(storexa::DatabaseConfig::from_url(&url).unwrap())
        .await
        .unwrap();
    let after: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version,checksum FROM public._sqlx_migrations ORDER BY version")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(after.len(), 14);
    assert_eq!(&after[..13], before);
    let floor: i32 = sqlx::query_scalar("SELECT minimum_api FROM photara.schema_metadata")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(floor, 3);
    db.close().await;
}
fn request(secret: &[u8]) -> BootstrapRequest {
    BootstrapRequest {
        schema: SCHEMA.into(),
        operation_id: id(),
        environment_id: "synthetic".into(),
        device_id: DeviceId::from_uuid(Uuid::new_v4()).unwrap(),
        device_credential_sha256: hex(&hash(secret)),
        device_display_name: "Test installation".into(),
        requested_library_id: LibraryId::from_uuid(Uuid::new_v4()).unwrap(),
        library_display_name: "My Library".into(),
        intent: "enroll-default".into(),
    }
}
async fn proof(
    service: &Service,
    bytes: &[u8],
    operation: OperationId,
    device: DeviceId,
    secret: &[u8],
    action: &str,
) -> Proof {
    let c = service
        .challenge(&ChallengeRequest {
            schema: SCHEMA.into(),
            action: action.into(),
            operation_id: operation,
            request_sha256: hex(&hash(bytes)),
            device_id: device,
            device_credential_sha256: hex(&hash(secret)),
        })
        .await
        .unwrap();
    service.proof_facts(c.challenge_id).await.unwrap()
}
#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_onboarding_atomic_replay_and_sessions() {
    let f = crate::pgtests::Fixture::new().await;
    let claims = auth::VerifiedClaims {
        issuer: "https://synthetic.invalid/".into(),
        subject: Uuid::new_v4().to_string(),
        audience: "synthetic".into(),
        expires_ms: 1_900_000_000_000,
    };
    let secret = [83; 32];
    let c = request(&secret);
    let bytes = canonical(&c).unwrap();
    assert!(
        f.service
            .operation(&claims, c.operation_id, &secret)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        f.service
            .bootstrap(&claims, "synthetic", &bytes, &secret, None)
            .await
            .is_err()
    );
    let p = proof(
        &f.service,
        &bytes,
        c.operation_id,
        c.device_id,
        &secret,
        "bootstrap",
    )
    .await;
    let response = f
        .service
        .bootstrap(&claims, "synthetic", &bytes, &secret, Some(&p))
        .await
        .unwrap();
    let receipt: ReceiptEnvelope = serde_json::from_slice(&response).unwrap();
    assert_eq!(
        receipt.receipt["default_library_id"],
        serde_json::to_value(c.requested_library_id).unwrap()
    );
    assert_eq!(receipt.receipt["outcome"], "claimed-default");
    assert_eq!(receipt.receipt["initial_high_water"], "0");
    // Lost response: immutable bytes are recovered without the consumed proof.
    assert_eq!(
        f.service
            .bootstrap(&claims, "synthetic", &bytes, &secret, None)
            .await
            .unwrap(),
        response
    );
    assert_eq!(
        f.service
            .operation(&claims, c.operation_id, &secret)
            .await
            .unwrap()
            .unwrap(),
        response
    );
    assert!(
        f.service
            .operation(&claims, c.operation_id, &[84; 32])
            .await
            .is_err()
    );
    let mut changed = c.clone();
    changed.library_display_name = "Changed".into();
    assert!(matches!(
        f.service
            .bootstrap(
                &claims,
                "synthetic",
                &canonical(&changed).unwrap(),
                &secret,
                None
            )
            .await,
        Err(ServiceError::Conflict)
    ));
    assert!(f.service.proof_facts(p.challenge_id).await.is_err());
    // Second installation converges to the durable default without a second Library.
    let second_secret = [85; 32];
    let second = request(&second_secret);
    let second_bytes = canonical(&second).unwrap();
    let p2 = proof(
        &f.service,
        &second_bytes,
        second.operation_id,
        second.device_id,
        &second_secret,
        "bootstrap",
    )
    .await;
    let second_response = f
        .service
        .bootstrap(
            &claims,
            "synthetic",
            &second_bytes,
            &second_secret,
            Some(&p2),
        )
        .await
        .unwrap();
    let second_receipt: ReceiptEnvelope = serde_json::from_slice(&second_response).unwrap();
    assert_eq!(
        second_receipt.receipt["outcome"],
        "existing-default-requires-local-choice"
    );
    assert_eq!(
        second_receipt.receipt["account_id"],
        receipt.receipt["account_id"]
    );
    assert_eq!(
        second_receipt.receipt["default_library_id"],
        receipt.receipt["default_library_id"]
    );
    // Global collision rolls back the new account/identity/device and leaves challenge reusable.
    let collision_claims = auth::VerifiedClaims {
        subject: Uuid::new_v4().to_string(),
        ..claims.clone()
    };
    let collision_secret = [86; 32];
    let mut collision = request(&collision_secret);
    collision.requested_library_id = c.requested_library_id;
    let collision_bytes = canonical(&collision).unwrap();
    let pc = proof(
        &f.service,
        &collision_bytes,
        collision.operation_id,
        collision.device_id,
        &collision_secret,
        "bootstrap",
    )
    .await;
    assert!(matches!(
        f.service
            .bootstrap(
                &collision_claims,
                "synthetic",
                &collision_bytes,
                &collision_secret,
                Some(&pc)
            )
            .await,
        Err(ServiceError::Conflict)
    ));
    assert!(
        f.service
            .operation(&collision_claims, collision.operation_id, &collision_secret)
            .await
            .unwrap()
            .is_none()
    );
    assert!(f.service.proof_facts(pc.challenge_id).await.is_ok());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM photara_identity.account_identities WHERE issuer=$1 AND subject=$2",
    )
    .bind(&collision_claims.issuer)
    .bind(&collision_claims.subject)
    .fetch_one(f.service.control.pool())
    .await
    .unwrap();
    assert_eq!(count, 0);
    let logout = SessionCommand {
        schema: SCHEMA.into(),
        operation_id: id(),
        device_id: c.device_id,
        expected_credential_revision: "1".into(),
    };
    let logout_bytes = canonical(&logout).unwrap();
    let logged_out = f
        .service
        .session_change(&claims, &logout_bytes, &secret, "logout", None)
        .await
        .unwrap();
    assert_eq!(
        f.service
            .operation(&claims, logout.operation_id, &secret)
            .await
            .unwrap()
            .unwrap(),
        logged_out
    );
    assert_eq!(
        f.service
            .session_change(&claims, &logout_bytes, &secret, "logout", None)
            .await
            .unwrap(),
        logged_out
    );
    assert!(
        f.service
            .session(&claims, c.device_id, &secret)
            .await
            .is_err()
    );
    assert!(
        f.service
            .bootstrap(&claims, "synthetic", &bytes, &secret, None)
            .await
            .is_err()
    );
    assert!(
        f.service
            .operation(&claims, c.operation_id, &secret)
            .await
            .is_err()
    );
    let resume = SessionCommand {
        operation_id: id(),
        expected_credential_revision: "2".into(),
        ..logout
    };
    let resume_bytes = canonical(&resume).unwrap();
    assert!(
        f.service
            .session_change(&claims, &resume_bytes, &secret, "resume", None)
            .await
            .is_err()
    );
    let pr = proof(
        &f.service,
        &resume_bytes,
        resume.operation_id,
        c.device_id,
        &secret,
        "resume",
    )
    .await;
    let resumed = f
        .service
        .session_change(&claims, &resume_bytes, &secret, "resume", Some(&pr))
        .await
        .unwrap();
    assert_eq!(
        f.service
            .session_change(&claims, &resume_bytes, &secret, "resume", None)
            .await
            .unwrap(),
        resumed
    );
    assert!(
        f.service
            .session(&claims, c.device_id, &secret)
            .await
            .is_ok()
    );
    // Private-table privilege matrix actually uses each independent unprivileged pool.
    for db in [&f.service.api, &f.service.auth] {
        for table in [
            "photara_identity.account_defaults",
            "photara_identity.device_credentials",
            "photara_private.onboarding_receipts",
            "photara_private.onboarding_challenges",
        ] {
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(format!("SELECT * FROM {table}")))
                    .fetch_all(db.pool())
                    .await
                    .is_err()
            );
            for privilege in ["INSERT", "UPDATE", "DELETE"] {
                let allowed: bool =
                    sqlx::query_scalar("SELECT has_table_privilege(current_user,(SELECT c.oid FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname||'.'||c.relname=$1),$2)")
                        .bind(table)
                        .bind(privilege)
                        .fetch_one(db.pool())
                        .await
                        .unwrap();
                assert!(!allowed);
            }
        }
    }
    assert!(
        sqlx::query("UPDATE photara_private.onboarding_receipts SET action='logout'")
            .execute(f.service.control.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM photara_private.onboarding_challenges WHERE challenge_id=$1")
            .bind(p.challenge_id)
            .execute(f.service.control.pool())
            .await
            .is_err()
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_onboarding_concurrent_installations() {
    let f = crate::pgtests::Fixture::new().await;
    let claims = auth::VerifiedClaims {
        issuer: "https://synthetic.invalid/".into(),
        subject: Uuid::new_v4().to_string(),
        audience: "synthetic".into(),
        expires_ms: 1_900_000_000_000,
    };
    let sa = [91; 32];
    let sb = [92; 32];
    let a = request(&sa);
    let b = request(&sb);
    let ba = canonical(&a).unwrap();
    let bb = canonical(&b).unwrap();
    let pa = proof(
        &f.service,
        &ba,
        a.operation_id,
        a.device_id,
        &sa,
        "bootstrap",
    )
    .await;
    let pb = proof(
        &f.service,
        &bb,
        b.operation_id,
        b.device_id,
        &sb,
        "bootstrap",
    )
    .await;
    let (ra, rb) = tokio::join!(
        f.service
            .bootstrap(&claims, "synthetic", &ba, &sa, Some(&pa)),
        f.service
            .bootstrap(&claims, "synthetic", &bb, &sb, Some(&pb))
    );
    let ra: ReceiptEnvelope = serde_json::from_slice(&ra.unwrap()).unwrap();
    let rb: ReceiptEnvelope = serde_json::from_slice(&rb.unwrap()).unwrap();
    assert_eq!(ra.receipt["account_id"], rb.receipt["account_id"]);
    assert_eq!(
        ra.receipt["default_library_id"],
        rb.receipt["default_library_id"]
    );
    assert_ne!(ra.receipt["outcome"], rb.receipt["outcome"]);
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_onboarding_write_faults_and_unknown_commit() {
    use std::sync::atomic::Ordering;
    let f = crate::pgtests::Fixture::new().await;
    for phase in [1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16] {
        let secret = [phase; 32];
        let c = request(&secret);
        let bytes = canonical(&c).unwrap();
        let claims = auth::VerifiedClaims {
            issuer: "https://fault.invalid/".into(),
            subject: Uuid::new_v4().to_string(),
            audience: "synthetic".into(),
            expires_ms: 1_900_000_000_000,
        };
        let proof = proof(
            &f.service,
            &bytes,
            c.operation_id,
            c.device_id,
            &secret,
            "bootstrap",
        )
        .await;
        f.service.fault_phase.store(phase, Ordering::SeqCst);
        assert!(
            matches!(
                f.service
                    .bootstrap(&claims, "synthetic", &bytes, &secret, Some(&proof))
                    .await,
                Err(ServiceError::Storage)
            ),
            "phase {phase}"
        );
        f.service.fault_phase.store(0, Ordering::SeqCst);
        let recovered = f
            .service
            .operation(&claims, c.operation_id, &secret)
            .await
            .unwrap();
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM photara_identity.account_identities WHERE issuer=$1 AND subject=$2").bind(&claims.issuer).bind(&claims.subject).fetch_one(f.service.control.pool()).await.unwrap();
        if phase == 8 {
            assert_eq!(count, 1);
            let recovered = recovered.unwrap();
            assert_eq!(
                f.service
                    .bootstrap(&claims, "synthetic", &bytes, &secret, None)
                    .await
                    .unwrap(),
                recovered
            );
        } else {
            assert_eq!(count, 0);
            assert!(recovered.is_none());
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM photara.libraries WHERE library_id=$1)",
            )
            .bind(c.requested_library_id.uuid())
            .fetch_one(f.service.control.pool())
            .await
            .unwrap();
            assert!(!exists);
            assert!(f.service.proof_facts(proof.challenge_id).await.is_ok());
        }
    }
    f.finish().await;
}
