//! Disposable credential seam only: no lifecycle route, SQL grant or deletion.
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use super::*;
use photara_core::contracts::AccountId;
use serde::{Deserialize, Serialize};

const NOW: i64 = 1_800_000_000_000;
const PATH: &str = "/fixture/ll1-credential-seam";
const ISSUER: &str = "https://issuer.invalid/";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    device: DeviceId,
    credential_revision: i64,
    operation: Uuid,
    digest: String,
    execute: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Event {
    account: AccountId,
    identity: Uuid,
    device: DeviceId,
    revision: i64,
    execution: bool,
}
struct Observer {
    account: AccountId,
    original_device: DeviceId,
    terminal: Uuid,
    pending: Uuid,
    digest: String,
    // Seeded immutable bytes stand in for a previously committed receipt.
    receipt: Vec<u8>,
    review_expires: i64,
    events: Vec<Event>,
}
struct Candidate {
    http: Arc<HttpService>,
    observer: Mutex<Observer>,
    waiting: Mutex<Option<tokio::sync::oneshot::Sender<i32>>>,
}
async fn candidate(
    State(state): State<Arc<Candidate>>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Response {
    let (claims, credential) = match state.http.credentials(&headers).await {
        Ok(value) => value,
        Err(e) => return authentication_failure(e),
    };
    result(admit(&state, &claims, &credential, &bytes).await)
}
async fn admit(
    state: &Candidate,
    claims: &crate::auth::VerifiedClaims,
    credential: &[u8],
    bytes: &[u8],
) -> Result<Vec<u8>> {
    let command: Command = serde_json::from_slice(bytes).map_err(|_| ServiceError::Invalid)?;
    if crate::canonical(&command)? != bytes || command.credential_revision < 1 {
        return Err(ServiceError::Invalid);
    }
    let service = &state.http.service;
    let mut tx = service.principal_transaction(claims).await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await?;
    if let Some(sender) = state.waiting.lock().unwrap().take() {
        let _ = sender.send(pid);
    }
    let actor = onboarding::identity(&mut tx, claims)
        .await?
        .ok_or(ServiceError::Forbidden)?;
    let (_, revision) = onboarding::device(
        &mut tx,
        &actor,
        command.device,
        &crate::hash(credential),
        false,
    )
    .await?
    .ok_or(ServiceError::Forbidden)?;
    if revision != command.credential_revision || claims.expires_ms <= service.clock.now_ms() {
        return Err(ServiceError::Forbidden);
    }
    // This observer has no authority capability. All database work above is
    // credential validation in one transaction; no membership/default lookup.
    let (response, execution) = {
        let observer = state.observer.lock().unwrap();
        if actor.account != observer.account {
            (br#"{"fixture":"not-found"}"#.to_vec(), false)
        } else if command.operation == observer.terminal {
            if command.digest != observer.digest {
                return Err(ServiceError::Conflict);
            }
            (observer.receipt.clone(), false)
        } else if command.execute {
            if command.operation != observer.pending
                || command.device != observer.original_device
                || command.digest != observer.digest
                || service.clock.now_ms() >= observer.review_expires
            {
                return Err(ServiceError::Forbidden);
            }
            (br#"{"fixture":"execution-admission-only"}"#.to_vec(), true)
        } else {
            (br#"{"fixture":"not-found"}"#.to_vec(), false)
        }
    };
    tx.commit().await?;
    state.observer.lock().unwrap().events.push(Event {
        account: actor.account,
        identity: actor.identity,
        device: command.device,
        revision,
        execution,
    });
    Ok(response)
}

struct Harness {
    owner: storexa::Database,
    clock: Arc<crate::auth::FakeClock>,
    state: Arc<Candidate>,
    router: Router,
    key: ring::signature::RsaKeyPair,
    account: AccountId,
    subject: String,
    alternate: String,
    revoked: String,
    other: String,
    devices: [DeviceId; 5],
    secrets: [[u8; 32]; 5],
}
impl Harness {
    async fn new() -> Self {
        let fixture = crate::pgtests::Fixture::new().await;
        let clock = fixture.clock.clone();
        let service = fixture.into_service().await;
        // Fixture::new has already checked this generated private socket URL.
        let base = std::env::var("PHOTARA_TEST_MIGRATOR_URL").unwrap();
        let owner = storexa::Database::connect(DatabaseConfig::from_url(base).unwrap())
            .await
            .unwrap();
        let oidc = OidcConfig {
            issuer: ISSUER.into(),
            audience: "synthetic-api".into(),
            native_client_id: "synthetic-native".into(),
            allow_userinfo_audience: false,
        };
        let verifier = Arc::new(OidcVerifier::new(oidc.clone(), clock.clone()).unwrap());
        let key = crate::oidc::tests::keypair();
        crate::oidc::tests::install(&verifier, &key, "synthetic");
        let account = AccountId::from_uuid(Uuid::new_v4()).unwrap();
        let other_account = Uuid::new_v4();
        let subjects: [String; 4] = std::array::from_fn(|_| Uuid::new_v4().to_string());
        let devices = std::array::from_fn(|_| DeviceId::from_uuid(Uuid::new_v4()).unwrap());
        let secrets = std::array::from_fn(|n| [u8::try_from(201 + n).unwrap(); 32]);
        let mut tx = owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        for id in [account.uuid(), other_account] {
            sqlx::query("INSERT INTO photara_identity.accounts VALUES($1,'LL1 credential fixture','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)")
                .bind(id).execute(&mut *tx).await.unwrap();
        }
        for (n, subject) in subjects.iter().enumerate() {
            sqlx::query("INSERT INTO photara_identity.account_identities VALUES($1,$2,$3,$4,$5,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CASE WHEN $5='revoked' THEN CURRENT_TIMESTAMP ELSE NULL END)")
                .bind(Uuid::new_v4()).bind(if n == 3 {other_account} else {account.uuid()})
                .bind(ISSUER).bind(subject).bind(if n == 2 {"revoked"} else {"active"})
                .execute(&mut *tx).await.unwrap();
        }
        for (n, device) in devices.iter().enumerate() {
            let actor = if n == 4 {
                other_account
            } else {
                account.uuid()
            };
            sqlx::query("INSERT INTO photara_identity.devices VALUES($1,$2,'LL1 fixture',$3,CURRENT_TIMESTAMP,NULL)")
                .bind(actor).bind(device.uuid()).bind(if n == 3 {"revoked"} else {"active"})
                .execute(&mut *tx).await.unwrap();
            sqlx::query("INSERT INTO photara_identity.device_credentials VALUES($1,$2,$3,$4,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)")
                .bind(actor).bind(device.uuid()).bind(crate::hash(&secrets[n]).to_vec())
                .bind(if n == 2 {"suspended"} else {"active"}).execute(&mut *tx).await.unwrap();
        }
        tx.commit().await.unwrap();
        let http = Arc::new(HttpService {
            service,
            verifier,
            config: HttpConfig {
                release_channel: crate::release::ReleaseChannel::RemoteAcceptance,
                environment_id: "synthetic".into(),
                service_origin: "https://service.invalid/".into(),
                oidc,
            },
            limits: Mutex::new(Limits {
                window: Instant::now(),
                buckets: BTreeMap::new(),
            }),
            concurrency: tokio::sync::Semaphore::new(64),
        });
        let state = Arc::new(Candidate {
            http: http.clone(),
            observer: Mutex::new(Observer {
                account,
                original_device: devices[0],
                terminal: Uuid::new_v4(),
                pending: Uuid::new_v4(),
                digest: onboarding::hex(&crate::hash(b"original test request")),
                receipt: br#"{"fixture":"original-terminal-bytes","device":"A"}"#.to_vec(),
                review_expires: NOW + 300_000,
                events: vec![],
            }),
            waiting: Mutex::new(None),
        });
        let router = Router::new()
            .route(PATH, post(candidate))
            .layer(DefaultBodyLimit::max(65536))
            .layer(middleware::from_fn_with_state(http, bounds))
            .with_state(state.clone());
        let [subject, alternate, revoked, other] = subjects;
        Self {
            owner,
            clock,
            state,
            router,
            key,
            account,
            subject,
            alternate,
            revoked,
            other,
            devices,
            secrets,
        }
    }
    fn claims(subject: &str) -> serde_json::Value {
        serde_json::json!({"iss":ISSUER,"sub":subject,"aud":"synthetic-api","azp":"synthetic-native","scope":"photara:onboard","iat":NOW/1000,"exp":NOW/1000+600})
    }
    fn bearer(&self, claims: &serde_json::Value) -> String {
        format!(
            "Bearer {}",
            crate::oidc::tests::sign(&self.key, "synthetic", &claims.to_string())
        )
    }
    fn command(&self, device: usize, terminal: bool, execute: bool) -> Command {
        let o = self.state.observer.lock().unwrap();
        Command {
            device: self.devices[device],
            credential_revision: 1,
            operation: if terminal { o.terminal } else { o.pending },
            digest: o.digest.clone(),
            execute,
        }
    }
    async fn call(&self, bearer: &str, secret: &[u8], body: Vec<u8>) -> (StatusCode, Vec<u8>) {
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret);
        super::tests::call(
            &self.router,
            PATH,
            "POST",
            &[
                ("authorization", bearer),
                ("photara-device-credential", &credential),
            ],
            body,
        )
        .await
    }
    async fn snapshot(&self) -> Vec<serde_json::Value> {
        let mut tx = self.owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        let mut rows = vec![];
        // All four credential relations, including unrelated fixture principals.
        for sql in [
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM photara_identity.accounts t",
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM photara_identity.account_identities t",
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM photara_identity.devices t",
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM photara_identity.device_credentials t",
        ] {
            rows.push(sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap());
        }
        tx.rollback().await.unwrap();
        rows
    }
    async fn denied(&self, bearer: &str, secret: &[u8], body: Vec<u8>, status: StatusCode) {
        let before = self.snapshot().await;
        let events = self.state.observer.lock().unwrap().events.clone();
        let response = self.call(bearer, secret, body).await;
        assert_eq!(
            response.0,
            status,
            "{}",
            String::from_utf8_lossy(&response.1)
        );
        assert_eq!(self.snapshot().await, before);
        assert_eq!(self.state.observer.lock().unwrap().events, events);
    }
    async fn race(&self, expire: bool) {
        let mut blocker = self.owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        sqlx::query("SELECT revision FROM photara_identity.device_credentials WHERE account_id=$1 AND device_id=$2 FOR UPDATE")
            .bind(self.account.uuid()).bind(self.devices[1].uuid()).fetch_one(&mut *blocker).await.unwrap();
        let before = self.snapshot().await;
        let events = self.state.observer.lock().unwrap().events.clone();
        let (send, recv) = tokio::sync::oneshot::channel();
        *self.state.waiting.lock().unwrap() = Some(send);
        let router = self.router.clone();
        let bearer = self.bearer(&Self::claims(&self.subject));
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.secrets[1]);
        let body = crate::canonical(&self.command(1, true, false)).unwrap();
        let request = tokio::spawn(async move {
            super::tests::call(
                &router,
                PATH,
                "POST",
                &[
                    ("authorization", &bearer),
                    ("photara-device-credential", &credential),
                ],
                body,
            )
            .await
        });
        let pid = tokio::time::timeout(Duration::from_secs(2), recv)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let blockers: Vec<i32> = sqlx::query_scalar("SELECT pg_blocking_pids($1)")
                    .bind(pid)
                    .fetch_one(self.owner.pool())
                    .await
                    .unwrap();
                if blockers.contains(&blocker_pid) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("request must really wait on the credential lock");
        if expire {
            self.clock.set(NOW + 600_000);
        } else {
            sqlx::query("UPDATE photara_identity.device_credentials SET state='suspended',revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1 AND device_id=$2")
                .bind(self.account.uuid()).bind(self.devices[1].uuid()).execute(&mut *blocker).await.unwrap();
        }
        blocker.commit().await.unwrap();
        assert_eq!(request.await.unwrap().0, StatusCode::FORBIDDEN);
        assert_eq!(self.state.observer.lock().unwrap().events, events);
        if expire {
            assert_eq!(self.snapshot().await, before);
        } else {
            let mut after = self.snapshot().await;
            let mut expected = before;
            let is_target = |row: &&mut serde_json::Value| {
                row["account_id"] == self.account.to_string()
                    && row["device_id"] == self.devices[1].to_string()
            };
            let observed = after[3]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(is_target)
                .unwrap();
            let original = expected[3]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(is_target)
                .unwrap();
            assert!(
                observed["updated_at"].as_str().unwrap()
                    >= original["updated_at"].as_str().unwrap()
            );
            original["state"] = serde_json::json!("suspended");
            original["revision"] = serde_json::json!(2);
            original["updated_at"] = observed["updated_at"].clone();
            for relations in [&mut expected, &mut after] {
                for rows in relations {
                    rows.as_array_mut()
                        .unwrap()
                        .sort_by_cached_key(serde_json::Value::to_string);
                }
            }
            assert_eq!(
                after, expected,
                "only harness credential revocation may change rows"
            );
        }
        self.clock.set(NOW);
    }
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_ll1_signed_credential_seam() {
    let h = Harness::new().await;
    let bearer = h.bearer(&Harness::claims(&h.subject));
    let body = crate::canonical(&h.command(0, true, false)).unwrap();
    let receipt = h.state.observer.lock().unwrap().receipt.clone();
    assert_eq!(
        h.call(&bearer, &h.secrets[0], body.clone()).await,
        (StatusCode::OK, receipt.clone())
    );
    // Valid original A admission is only an observer event, never SQL execution.
    assert_eq!(
        h.call(
            &bearer,
            &h.secrets[0],
            crate::canonical(&h.command(0, false, true)).unwrap()
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut wrong = Harness::claims(&h.subject);
    wrong["iss"] = serde_json::json!("https://wrong.invalid/");
    let mut audience = Harness::claims(&h.subject);
    audience["aud"] = serde_json::json!("other-api");
    let mut expired = Harness::claims(&h.subject);
    expired["exp"] = serde_json::json!(NOW / 1000);
    for claims in [wrong, audience, expired] {
        h.denied(
            &h.bearer(&claims),
            &h.secrets[0],
            body.clone(),
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }
    let wrong_key = crate::oidc::tests::keypair();
    let wrong_signature = format!(
        "Bearer {}",
        crate::oidc::tests::sign(
            &wrong_key,
            "synthetic",
            &Harness::claims(&h.subject).to_string()
        )
    );
    h.denied(
        &wrong_signature,
        &h.secrets[0],
        body.clone(),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    for subject in [Uuid::new_v4().to_string(), h.revoked.clone()] {
        h.denied(
            &h.bearer(&Harness::claims(&subject)),
            &h.secrets[0],
            body.clone(),
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    for secret in [&h.secrets[1], &[99; 32]] {
        h.denied(&bearer, secret, body.clone(), StatusCode::FORBIDDEN)
            .await;
    }
    for n in [2, 3] {
        h.denied(
            &bearer,
            &h.secrets[n],
            crate::canonical(&h.command(n, true, false)).unwrap(),
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    let mut stale = h.command(0, true, false);
    stale.credential_revision = 2;
    h.denied(
        &bearer,
        &h.secrets[0],
        crate::canonical(&stale).unwrap(),
        StatusCode::FORBIDDEN,
    )
    .await;
    let mut substituted = serde_json::to_value(h.command(0, true, false)).unwrap();
    substituted["account_id"] = serde_json::json!(h.account.to_string());
    h.denied(
        &bearer,
        &h.secrets[0],
        crate::canonical(&substituted).unwrap(),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    let mut changed = h.command(1, true, false);
    changed.digest = onboarding::hex(&crate::hash(b"changed"));
    h.denied(
        &bearer,
        &h.secrets[1],
        crate::canonical(&changed).unwrap(),
        StatusCode::CONFLICT,
    )
    .await;
    h.denied(
        &bearer,
        &h.secrets[1],
        crate::canonical(&h.command(1, false, true)).unwrap(),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_eq!(
        h.call(
            &bearer,
            &h.secrets[1],
            crate::canonical(&h.command(1, false, false)).unwrap()
        )
        .await
        .1,
        br#"{"fixture":"not-found"}"#
    );
    assert_eq!(
        h.call(
            &h.bearer(&Harness::claims(&h.other)),
            &h.secrets[4],
            crate::canonical(&h.command(4, true, false)).unwrap()
        )
        .await
        .1,
        br#"{"fixture":"not-found"}"#
    );
    // No Library/default/membership was seeded for this account. Revoke A and
    // recover its exact original bytes through B, even after review expiry.
    let mut tx = h.owner.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE photara_identity.devices SET state='revoked' WHERE account_id=$1 AND device_id=$2",
    )
    .bind(h.account.uuid())
    .bind(h.devices[0].uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    h.denied(&bearer, &h.secrets[0], body, StatusCode::FORBIDDEN)
        .await;
    h.clock.set(NOW + 300_000);
    let before = h.snapshot().await;
    for subject in [&h.subject, &h.alternate] {
        for execute in [false, true] {
            assert_eq!(
                h.call(
                    &h.bearer(&Harness::claims(subject)),
                    &h.secrets[1],
                    crate::canonical(&h.command(1, true, execute)).unwrap()
                )
                .await,
                (StatusCode::OK, receipt.clone())
            );
        }
    }
    assert_eq!(h.snapshot().await, before);
    assert_eq!(
        h.state
            .observer
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.execution)
            .count(),
        1
    );
    h.clock.set(NOW);
    h.race(true).await;
    h.race(false).await;
    println!(
        "LL1 credential seam: signed HTTP claims, exact credential/revision, fresh-device receipt observer, original-device execution observer, and two proven lock waits passed; SQL executor and production lifecycle unchanged"
    );
    h.state.http.service.close().await;
    h.owner.close().await;
}
