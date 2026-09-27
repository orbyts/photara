//! Disposable combined signed HTTP/SQL authority proof; never a production route.
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const NOW: i64 = 1_800_000_000_000;
const PATH: &str = "/fixture/ll1-sql";
const ISSUER: &str = "https://fake.invalid/";
const SQL: &str = "SELECT ll1_probe.http_command($1,$2,$3,$4,$5,$6,$7,decode($8,'hex'),$9)";
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    device: Uuid,
    revision: i64,
    operation: Uuid,
    digest: String,
    execute: bool,
}
struct Candidate {
    http: Arc<HttpService>,
    authority: storexa::Database,
    waiting: Mutex<Option<tokio::sync::oneshot::Sender<i32>>>,
    errors: Mutex<Vec<String>>,
}
async fn candidate(
    State(state): State<Arc<Candidate>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (claims, secret) = match state.http.credentials(&headers).await {
        Ok(value) => value,
        Err(e) => return authentication_failure(e),
    };
    result(admit(&state, &claims, &secret, &body).await)
}
async fn admit(
    state: &Candidate,
    claims: &crate::auth::VerifiedClaims,
    secret: &[u8],
    body: &[u8],
) -> Result<Vec<u8>> {
    let command: Command = serde_json::from_slice(body).map_err(|_| ServiceError::Invalid)?;
    if crate::canonical(&command)? != body
        || command.revision < 1
        || command.digest.len() != 64
        || !command.digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ServiceError::Invalid);
    }
    let mut tx = state.authority.begin().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await?;
    if let Some(sender) = state.waiting.lock().unwrap().take() {
        let _ = sender.send(pid);
    }
    let response: std::result::Result<Option<Vec<u8>>, sqlx::Error> = sqlx::query_scalar(SQL)
        .bind(&claims.issuer)
        .bind(&claims.subject)
        .bind(claims.expires_ms)
        .bind(command.device)
        .bind(secret)
        .bind(command.revision)
        .bind(command.operation)
        .bind(&command.digest)
        .bind(command.execute)
        .fetch_one(&mut *tx)
        .await;
    let response = match response {
        Ok(value) => value,
        Err(error) => {
            state.errors.lock().unwrap().push(
                error
                    .as_database_error()
                    .map_or_else(|| error.to_string(), |e| e.message().to_owned()),
            );
            tx.rollback().await?;
            return Err(
                if error
                    .as_database_error()
                    .is_some_and(|e| e.message().contains("operation_conflict"))
                {
                    ServiceError::Conflict
                } else {
                    ServiceError::Forbidden
                },
            );
        }
    };
    if claims.expires_ms <= state.http.service.clock.now_ms() {
        tx.rollback().await?;
        return Err(ServiceError::Forbidden);
    }
    // The SQL destructive-boundary trigger already checked deadlines after waits.
    // Returning any original bytes still requires this transaction's commit.
    tx.commit().await?;
    Ok(response.unwrap_or_else(|| br#"{"fixture":"not-found"}"#.to_vec()))
}
struct Harness {
    admin: storexa::Database,
    state: Arc<Candidate>,
    router: Router,
    key: ring::signature::RsaKeyPair,
    socket: String,
    clock: Arc<crate::auth::FakeClock>,
}
impl Harness {
    fn config(socket: &str, role: &str) -> DatabaseConfig {
        DatabaseConfig::from_url(format!(
            "postgresql://{role}@localhost/postgres?host={socket}&port=55439"
        ))
        .unwrap()
        .with_max_connections(4)
    }
    async fn new() -> Self {
        let socket =
            std::env::var("PHOTARA_LL1_ADAPTER_SOCKET").expect("use private LL1 adapter runner");
        let path = std::path::Path::new(&socket);
        assert!(
            socket.starts_with("/private/tmp/photara-ll1-constraints-")
                && path.file_name().unwrap() == "socket"
        );
        assert_eq!(path.canonicalize().unwrap(), path);
        assert!(path.join(".s.PGSQL.55439").exists());
        let admin = storexa::Database::connect(Self::config(&socket, "ll1_fixture"))
            .await
            .unwrap();
        let listen: String = sqlx::query_scalar("SHOW listen_addresses")
            .fetch_one(admin.pool())
            .await
            .unwrap();
        let actual: String = sqlx::query_scalar("SHOW unix_socket_directories")
            .fetch_one(admin.pool())
            .await
            .unwrap();
        assert!(listen.is_empty());
        assert_eq!(actual, socket);
        let clock = Arc::new(crate::auth::FakeClock::new(NOW));
        let oidc = OidcConfig {
            issuer: ISSUER.into(),
            audience: "synthetic-api".into(),
            native_client_id: "synthetic-native".into(),
            allow_userinfo_audience: false,
        };
        let verifier = Arc::new(OidcVerifier::new(oidc.clone(), clock.clone()).unwrap());
        let key = crate::oidc::tests::keypair();
        crate::oidc::tests::install(&verifier, &key, "synthetic");
        let service = Service::connect(
            ["ll1_api", "ll1_control", "ll1_auth"].map(|role| Self::config(&socket, role)),
            verifier.clone(),
            clock.clone(),
            ISSUER.into(),
            "synthetic-api".into(),
            [51; 32],
        )
        .await
        .unwrap();
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
            authority: storexa::Database::connect(Self::config(&socket, "ll1_service"))
                .await
                .unwrap(),
            waiting: Mutex::new(None),
            errors: Mutex::new(vec![]),
        });
        let router = Router::new()
            .route(PATH, post(candidate))
            .layer(DefaultBodyLimit::max(65536))
            .layer(middleware::from_fn_with_state(http, bounds))
            .with_state(state.clone());
        Self {
            admin,
            state,
            router,
            key,
            socket,
            clock,
        }
    }
    fn bearer(&self, subject: &str) -> String {
        let claims = json!({"iss":ISSUER,"sub":subject,"aud":"synthetic-api","azp":"synthetic-native","scope":"photara:onboard","iat":NOW/1000,"exp":NOW/1000+600});
        format!(
            "Bearer {}",
            crate::oidc::tests::sign(&self.key, "synthetic", &claims.to_string())
        )
    }
    fn command(device: u128, execute: bool) -> Command {
        Command {
            device: id(device),
            revision: 1,
            operation: id(9200),
            digest: onboarding::hex(&crate::hash(b"request-one")),
            execute,
        }
    }
    async fn call(&self, subject: &str, secret: u8, command: &Command) -> (StatusCode, Vec<u8>) {
        let bearer = self.bearer(subject);
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([secret; 32]);
        super::tests::call(
            &self.router,
            PATH,
            "POST",
            &[
                ("authorization", &bearer),
                ("photara-device-credential", &credential),
            ],
            crate::canonical(command).unwrap(),
        )
        .await
    }
    async fn sql(&self, sql: &str) {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(self.admin.pool())
            .await
            .unwrap();
    }
    async fn snapshot(&self) -> BTreeMap<String, Value> {
        let tables:Vec<String>=sqlx::query_scalar("SELECT schemaname||'.'||tablename FROM pg_tables WHERE schemaname IN ('photara','photara_identity','photara_private','ll1_probe') ORDER BY 1")
            .fetch_all(self.admin.pool()).await.unwrap();
        let mut rows = BTreeMap::new();
        for table in tables {
            assert!(
                table
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            );
            let mut value: Value=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM {table} t")))
                .fetch_one(self.admin.pool()).await.unwrap();
            value
                .as_array_mut()
                .unwrap()
                .sort_by_cached_key(Value::to_string);
            rows.insert(table, value);
        }
        rows
    }
    async fn denied(&self, subject: &str, secret: u8, command: &Command, status: StatusCode) {
        let before = self.snapshot().await;
        let response = self.call(subject, secret, command).await;
        assert_eq!(
            response.0,
            status,
            "{}",
            String::from_utf8_lossy(&response.1)
        );
        assert_eq!(self.snapshot().await, before);
    }
    async fn race(&self, mode: u8) {
        let mut blocker = self.admin.begin().await.unwrap();
        let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        if mode == 2 {
            sqlx::query("SELECT pg_advisory_xact_lock(741852963)")
                .execute(&mut *blocker)
                .await
                .unwrap();
        } else if mode == 3 {
            sqlx::query("SELECT revision FROM photara.people WHERE library_id=$1 FOR UPDATE")
                .bind(id(1))
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
        } else {
            sqlx::query("SELECT revision FROM photara_identity.device_credentials WHERE account_id=$1 AND device_id=$2 FOR UPDATE")
                .bind(id(10)).bind(id(210)).fetch_one(&mut *blocker).await.unwrap();
        }
        let mut expected = self.snapshot().await;
        let (send, recv) = tokio::sync::oneshot::channel();
        *self.state.waiting.lock().unwrap() = Some(send);
        let router = self.router.clone();
        let bearer = self.bearer("owner");
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0xaa; 32]);
        let body = crate::canonical(&Self::command(210, true)).unwrap();
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
        let pid = tokio::time::timeout(Duration::from_secs(3), recv)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let blockers: Vec<i32> = sqlx::query_scalar("SELECT pg_blocking_pids($1)")
                    .bind(pid)
                    .fetch_one(self.admin.pool())
                    .await
                    .unwrap();
                if blockers.contains(&blocker_pid) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("actual authority request must wait on selected lock");
        if mode == 1 {
            let row:Value=sqlx::query_scalar("UPDATE photara_identity.device_credentials SET state='suspended',revision=revision+1,updated_at=clock_timestamp() WHERE account_id=$1 AND device_id=$2 RETURNING to_jsonb(device_credentials)")
                .bind(id(10)).bind(id(210)).fetch_one(&mut *blocker).await.unwrap();
            let rows = expected
                .get_mut("photara_identity.device_credentials")
                .unwrap()
                .as_array_mut()
                .unwrap();
            *rows
                .iter_mut()
                .find(|r| r["device_id"] == id(210).to_string())
                .unwrap() = row;
            rows.sort_by_cached_key(Value::to_string);
        } else {
            let ms = NOW + if mode >= 2 { 300_000 } else { 600_000 };
            self.clock.set(ms);
            self.sql(&format!(
                "UPDATE ll1_probe.review_clock SET instant=to_timestamp({ms}/1000.0)"
            ))
            .await;
            let row: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM ll1_probe.review_clock t")
                .fetch_one(self.admin.pool())
                .await
                .unwrap();
            expected.get_mut("ll1_probe.review_clock").unwrap()[0] = row;
        }
        blocker.commit().await.unwrap();
        assert_eq!(request.await.unwrap().0, StatusCode::FORBIDDEN);
        assert_eq!(
            self.state.errors.lock().unwrap().last().unwrap(),
            if mode == 1 {
                "credential_denied"
            } else {
                "http_deadline"
            }
        );
        let actual = self.snapshot().await;
        for (table, rows) in expected {
            assert_eq!(
                actual[&table], rows,
                "unexpected race mutation: {table}, mode {mode}"
            );
        }
        self.clock.set(NOW);
        self.sql(&format!(
            "UPDATE ll1_probe.review_clock SET instant=to_timestamp({NOW}/1000.0)"
        ))
        .await;
        if mode == 1 {
            self.sql(&format!("UPDATE photara_identity.device_credentials SET state='active',revision=revision+1,updated_at=clock_timestamp() WHERE device_id='{}'",id(210))).await;
        }
    }
}

#[tokio::test]
#[ignore = "requires explicit private HTTP/SQL adapter runner"]
async fn postgres_ll1_signed_sql_adapter() {
    let h = Harness::new().await;
    let mut a = Harness::command(210, true);
    let mut b = Harness::command(211, true);
    h.denied("owner", 0xbb, &b, StatusCode::FORBIDDEN).await;
    h.denied("owner", 0xbb, &a, StatusCode::FORBIDDEN).await;
    let mut wrong = a.clone();
    wrong.revision = 2;
    h.denied("owner", 0xaa, &wrong, StatusCode::FORBIDDEN).await;
    h.denied("unknown", 0xaa, &a, StatusCode::FORBIDDEN).await;
    for role in ["ll1_api", "ll1_control", "ll1_auth"] {
        let db = storexa::Database::connect(Harness::config(&h.socket, role))
            .await
            .unwrap();
        let before = h.snapshot().await;
        let mut tx = db.begin().await.unwrap();
        sqlx::query("SELECT set_config('photara.account_id',$1,true)")
            .bind(id(10).to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let denied = sqlx::query_scalar::<_, Option<Vec<u8>>>(SQL)
            .bind(ISSUER)
            .bind("owner")
            .bind(NOW + 600_000)
            .bind(id(210))
            .bind(vec![0xaa_u8; 32])
            .bind(1_i64)
            .bind(id(9200))
            .bind(&a.digest)
            .bind(true)
            .fetch_one(&mut *tx)
            .await
            .unwrap_err();
        assert!(
            denied
                .as_database_error()
                .unwrap()
                .message()
                .contains("permission denied"),
            "{denied}"
        );
        tx.rollback().await.unwrap();
        db.close().await;
        assert_eq!(h.snapshot().await, before);
    }
    for mode in [0, 2, 3, 1] {
        h.race(mode).await;
    }
    a.revision = 3;
    // A real error after receipt capture must roll the entire candidate unit back.
    h.sql("CREATE FUNCTION ll1_probe.http_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NOT EXISTS(SELECT 1 FROM ll1_probe.original_receipt) THEN RAISE EXCEPTION 'boundary_not_reached'; END IF; RAISE EXCEPTION 'after_http_terminal'; END $$; CREATE TRIGGER zz_http_failure AFTER INSERT ON ll1_probe.receipts FOR EACH ROW EXECUTE FUNCTION ll1_probe.http_fail();").await;
    h.denied("owner", 0xaa, &a, StatusCode::FORBIDDEN).await;
    assert_eq!(
        h.state.errors.lock().unwrap().last().unwrap(),
        "after_http_terminal"
    );
    h.sql(
        "DROP TRIGGER zz_http_failure ON ll1_probe.receipts; DROP FUNCTION ll1_probe.http_fail();",
    )
    .await;
    let before_execution = h.snapshot().await;
    let response = h.call("owner", 0xaa, &a).await;
    assert_eq!(
        response.0,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&response.1)
    );
    let original: Vec<u8> = sqlx::query_scalar("SELECT body FROM ll1_probe.original_receipt")
        .fetch_one(h.admin.pool())
        .await
        .unwrap();
    assert_eq!(response.1, original);
    let after_execution = h.snapshot().await;
    for (table, rows) in &before_execution {
        if table.starts_with("ll1_probe.") {
            continue;
        }
        let expected = Value::Array(
            rows.as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["library_id"] != id(1).to_string()
                        && row["claimed_library_id"] != id(1).to_string()
                })
                .cloned()
                .collect(),
        );
        assert_eq!(
            after_execution[table], expected,
            "unexpected baseline change: {table}"
        );
    }
    let captured: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(captured["result"], "removed");
    h.sql(&format!("UPDATE photara_identity.devices SET state='revoked' WHERE device_id='{}'; UPDATE photara_identity.device_credentials SET state='suspended',revision=revision+1,updated_at=clock_timestamp() WHERE device_id='{}'; CREATE FUNCTION ll1_probe.http_no_grant() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'receipt_must_not_mint_grant'; END $$; CREATE TRIGGER no_grant BEFORE INSERT ON ll1_probe.authorization FOR EACH ROW EXECUTE FUNCTION ll1_probe.http_no_grant();",id(210),id(210))).await;
    h.denied("owner", 0xaa, &a, StatusCode::FORBIDDEN).await;
    let before = h.snapshot().await;
    assert_eq!(
        h.call("owner", 0xbb, &b).await,
        (StatusCode::OK, original.clone())
    );
    b.execute = false;
    assert_eq!(h.call("owner", 0xbb, &b).await, (StatusCode::OK, original));
    assert_eq!(h.snapshot().await, before);
    b.operation = id(9201);
    b.execute = true;
    h.denied("owner", 0xbb, &b, StatusCode::FORBIDDEN).await;
    for table in ["authorization", "permit"] {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM ll1_probe.{table}"
        )))
        .fetch_one(h.admin.pool())
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
    println!(
        "LL1 combined adapter: signed execution/receipt, exact rollback, three role denials, credential revocation and three post-lock deadline gates passed"
    );
    h.state.authority.close().await;
    h.state.http.service.close().await;
    h.admin.close().await;
}
