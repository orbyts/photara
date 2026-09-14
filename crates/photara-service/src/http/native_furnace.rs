//! Test-only synthetic provider and fault controls around the real production
//! router/service, least-privileged pools and disposable `PostgreSQL` database.
#![allow(clippy::too_many_lines, clippy::unwrap_used)]
use super::*;
use ring::signature::KeyPair as _;
use std::{
    process::{Command, Stdio},
    sync::atomic::Ordering,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires explicit disposable PostgreSQL and compiled native furnace"]
async fn postgres_native_recovery_furnace() {
    let executable = std::env::var("PHOTARA_NATIVE_FURNACE_BINARY")
        .expect("run scripts/verify_service_postgres.py");
    assert!(executable.starts_with("/private/tmp/photara-native-service-"));
    let fixture = crate::pgtests::Fixture::new().await;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    fixture.clock.set(now);
    let service = fixture.into_service().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let oidc = OidcConfig {
        issuer: "https://issuer.invalid/".into(),
        audience: "synthetic-api".into(),
        native_client_id: "synthetic-native".into(),
        allow_userinfo_audience: true,
    };
    let verifier = Arc::new(OidcVerifier::new(oidc.clone(), service.clock.clone()).unwrap());
    let key = Arc::new(crate::oidc::tests::keypair());
    crate::oidc::tests::install(&verifier, &key, "furnace");
    let public_key = base64::engine::general_purpose::STANDARD.encode(key.public_key().as_ref());
    let state = Arc::new(HttpService {
        service,
        verifier,
        config: HttpConfig {
            release_channel: crate::release::ReleaseChannel::RemoteAcceptance,
            environment_id: "synthetic".into(),
            service_origin: origin.clone(),
            oidc,
        },
        limits: Mutex::new(Limits {
            window: Instant::now(),
            buckets: BTreeMap::new(),
        }),
        concurrency: tokio::sync::Semaphore::new(64),
    });
    let faults = state.clone();
    let delayed = state.clone();
    let router = state.clone().router()
        .route("/__furnace/fault", post(move |axum::Json(phase): axum::Json<u8>| {
            let state = faults.clone();
            async move {
                assert!([0, 7, 8, 9, 10].contains(&phase));
                state.service.fault_phase.store(phase, Ordering::SeqCst);
                axum::Json(serde_json::json!({"configured":true}))
            }
        }))
        .route("/__furnace/token", post(move |axum::Json(input): axum::Json<serde_json::Value>| {
            let key = key.clone();
            async move {
                let subject = input["subject"].as_str().unwrap();
                assert!(subject.starts_with("synthetic-"));
                let seconds = now / 1000;
                let access = serde_json::json!({"iss":"https://issuer.invalid/","sub":subject,
                    "aud":["synthetic-api","https://issuer.invalid/userinfo"],"azp":"synthetic-native",
                    "scope":"openid photara:onboard","iat":seconds,"exp":seconds+600});
                let mut proof = access.clone();
                proof["aud"] = serde_json::json!("synthetic-native");
                proof["nonce"] = input["nonce"].clone();
                proof["auth_time"] = serde_json::json!(seconds);
                axum::Json(serde_json::json!({
                    "access":crate::oidc::tests::sign(&key,"furnace",&access.to_string()),
                    "proof":crate::oidc::tests::sign(&key,"furnace",&proof.to_string())}))
            }
        }))
        .layer(middleware::from_fn(move |request: axum::extract::Request, next: Next| {
            let state = delayed.clone();
            async move {
                let bootstrap = request.uri().path() == "/v1/onboarding/bootstrap";
                let response = next.run(request).await;
                if bootstrap && state.service.fault_phase.compare_exchange(10, 0, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    // The database transaction has committed. Delay actual TCP
                    // response delivery beyond the native request timeout.
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                response
            }
        }));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    for scenario in [
        "normal",
        "accounts",
        "lost-reply",
        "commit-timeout",
        "commit-503",
        "rollback-503",
        "not-found-replay",
        "lookup-503",
        "unauthorized",
        "stale-capabilities",
        "crash-commit",
        "expired-fresh",
        "expired-fresh-inline",
        "expired-fresh-late-before",
        "expired-fresh-late-after",
        "expired-fresh-crash-commit",
        "expired-fresh-crash-receipt",
    ] {
        let directory = std::path::PathBuf::from("/private/tmp")
            .join(format!("photara-native-service-case-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let subject = format!("synthetic-{}", Uuid::new_v4());
        let config = serde_json::json!({"origin":origin,"public_key":public_key,"subject":subject,"now_ms":now});
        let config_path = directory.join("configuration.json");
        std::fs::write(&config_path, config.to_string()).unwrap();
        state.service.fault_phase.store(0, Ordering::SeqCst);
        state.limits.lock().unwrap().buckets.clear();
        let run = |mode: &str| {
            let executable = executable.clone();
            let directory = directory.clone();
            let mode = mode.to_owned();
            tokio::task::spawn_blocking(move || {
                let mut child = Command::new(executable)
                    .arg(&directory)
                    .arg(mode)
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(50);
                loop {
                    if let Some(status) = child.try_wait().unwrap() {
                        break status.code();
                    }
                    if Instant::now() >= deadline {
                        child.kill().unwrap();
                        child.wait().unwrap();
                        panic!("native furnace exceeded deadline");
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
        };
        if scenario.starts_with("expired-") {
            assert_eq!(run("expired-start").await.unwrap(), Some(0));
            state.service.fault_phase.store(0, Ordering::SeqCst);
            if scenario != "expired-fresh-inline" {
                assert_eq!(run("expired-retire").await.unwrap(), Some(0));
                assert_eq!(run("expired-cancel").await.unwrap(), Some(0));
                assert_eq!(run("expired-prepared").await.unwrap(), Some(74));
            }
        }
        let exit = run(scenario).await.unwrap();
        if scenario == "accounts" {
            assert_eq!(exit, Some(0));
            assert_eq!(run("accounts-rejoin").await.unwrap(), Some(0));
            assert_eq!(run("accounts-relaunch").await.unwrap(), Some(0));
        }
        if scenario == "expired-fresh-crash-receipt" {
            assert_eq!(exit, Some(75));
            assert_eq!(run("recover").await.unwrap(), Some(0));
        } else if scenario == "crash-commit" || scenario == "expired-fresh-crash-commit" {
            assert_eq!(exit, Some(73));
            assert_eq!(run("recover").await.unwrap(), Some(0));
        } else {
            assert_eq!(exit, Some(0), "{scenario}");
        }
        // Measure committed identity/device/receipt/library counts through a
        // read-only owner transaction; never mutate around a failing boundary.
        let owner_url = std::env::var("PHOTARA_TEST_MIGRATOR_URL").unwrap();
        let owner = storexa::Database::connect(DatabaseConfig::from_url(owner_url).unwrap())
            .await
            .unwrap();
        let mut tx = owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        let counts: (i64,i64,i64,i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM photara_identity.account_identities WHERE subject=$1), (SELECT count(*) FROM photara_identity.devices WHERE account_id IN (SELECT account_id FROM photara_identity.account_identities WHERE subject=$1)), (SELECT count(*) FROM photara_private.onboarding_receipts WHERE subject=$1), (SELECT count(*) FROM photara_identity.account_defaults WHERE account_id IN (SELECT account_id FROM photara_identity.account_identities WHERE subject=$1)), (SELECT count(*) FROM photara_identity.accounts WHERE account_id IN (SELECT account_id FROM photara_identity.account_identities WHERE subject=$1)), (SELECT count(*) FROM photara.libraries WHERE display_name='My Library' AND library_id IN (SELECT library_id FROM photara_identity.memberships WHERE account_id IN (SELECT account_id FROM photara_identity.account_identities WHERE subject=$1))), (SELECT count(*) FROM photara_identity.memberships WHERE account_id IN (SELECT account_id FROM photara_identity.account_identities WHERE subject=$1))")
            .bind(subject).fetch_one(&mut *tx).await.unwrap();
        tx.rollback().await.unwrap();
        owner.close().await;
        let expected = if ["rollback-503", "unauthorized", "stale-capabilities"].contains(&scenario)
        {
            (0, 0, 0, 0, 0, 0, 0)
        } else if scenario.contains("late-") {
            (1, 1, 2, 1, 1, 1, 1)
        } else {
            (1, 1, 1, 1, 1, 1, 1)
        };
        assert_eq!(
            counts, expected,
            "no duplicate or partial onboarding: {scenario}"
        );
        println!("native-service-furnace: {scenario}: committed counts verified; no duplicates");
        std::fs::remove_dir_all(directory).unwrap();
    }
    server.abort();
    let _ = server.await;
    state.service.close().await;
}
