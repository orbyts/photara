//! Minimal production HTTP adapter. The deployment operator supplies TLS ingress.
#![allow(clippy::manual_let_else)] // Handler matches keep status mapping explicit.
use crate::{
    Result, Service, ServiceError,
    auth::{Clock, IdentityVerifier},
    oidc::{OidcConfig, OidcVerifier},
    onboarding::{self, ChallengeRequest, Proof, SCHEMA},
};
use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse as _, Response},
    routing::{get, post},
};
use base64::Engine as _;
use photara_core::contracts::{DeviceId, OperationId};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use storexa::DatabaseConfig;
use uuid::Uuid;

#[derive(Clone)]
pub struct HttpConfig {
    pub release_channel: crate::release::ReleaseChannel,
    pub environment_id: String,
    pub service_origin: String,
    pub oidc: OidcConfig,
}
struct Limits {
    window: Instant,
    buckets: BTreeMap<[u8; 32], u32>,
}
pub struct HttpService {
    service: Service,
    verifier: Arc<OidcVerifier>,
    config: HttpConfig,
    limits: Mutex<Limits>,
    concurrency: tokio::sync::Semaphore,
}
impl HttpService {
    /// # Errors
    /// Requires checked release coordinates and safe pools. Only the explicit
    /// development channel permits its checked loopback HTTP origin.
    /// This never migrates a database or creates credentials.
    pub async fn connect(
        config: HttpConfig,
        database_urls: [String; 3],
        clock: Arc<dyn Clock>,
        cursor_key: [u8; 32],
    ) -> Result<Arc<Self>> {
        crate::release::validate_http(&config)?;
        let verifier = Arc::new(OidcVerifier::new(config.oidc.clone(), clock.clone())?);
        let mut checked = Vec::new();
        for url in database_urls {
            let parsed = reqwest::Url::parse(&url).map_err(|_| ServiceError::Invalid)?;
            if !["postgres", "postgresql"].contains(&parsed.scheme())
                || parsed
                    .query_pairs()
                    .filter(|(key, _)| key == "sslmode")
                    .map(|(_, value)| value.into_owned())
                    .collect::<Vec<_>>()
                    != ["verify-full"]
            {
                return Err(ServiceError::Invalid);
            }
            checked.push(
                DatabaseConfig::from_url(url)?
                    .with_max_connections(4)
                    .with_acquire_timeout(Duration::from_secs(3)),
            );
        }
        let databases: [DatabaseConfig; 3] =
            checked.try_into().map_err(|_| ServiceError::Invalid)?;
        verifier.warm_up().await?;
        let service = Service::connect(
            databases,
            verifier.clone(),
            clock,
            config.oidc.issuer.clone(),
            config.oidc.audience.clone(),
            cursor_key,
        )
        .await?;
        Ok(Arc::new(Self {
            service,
            verifier,
            config,
            limits: Mutex::new(Limits {
                window: Instant::now(),
                buckets: BTreeMap::new(),
            }),
            concurrency: tokio::sync::Semaphore::new(64),
        }))
    }
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/health/live", get(live))
            .route("/health/ready", get(ready))
            .route("/v1/onboarding/capabilities", get(capabilities))
            .route("/v1/onboarding/challenges", post(challenge))
            .route("/v1/onboarding/bootstrap", post(bootstrap))
            .route("/v1/onboarding/operations/{operation_id}", get(operation))
            .route("/v1/session", get(session))
            .route("/v1/session/resume", post(resume))
            .route("/v1/session/logout", post(logout))
            .fallback(|| async { failure(StatusCode::NOT_FOUND, "not-found", false, true) })
            .layer(DefaultBodyLimit::max(65536))
            .layer(middleware::from_fn_with_state(self.clone(), bounds))
            .with_state(self)
    }
    fn limit(&self, key: [u8; 32], maximum: u32) -> bool {
        let Ok(mut limits) = self.limits.lock() else {
            return false;
        };
        if limits.window.elapsed() >= Duration::from_mins(1) {
            limits.window = Instant::now();
            limits.buckets.clear();
        }
        if limits.buckets.len() >= 4096 && !limits.buckets.contains_key(&key) {
            return false;
        }
        let count = limits.buckets.entry(key).or_default();
        *count += 1;
        *count <= maximum
    }
    async fn credentials(
        &self,
        headers: &HeaderMap,
    ) -> Result<(crate::auth::VerifiedClaims, Vec<u8>)> {
        let token = single(headers, "authorization", 16391)?
            .strip_prefix("Bearer ")
            .ok_or(ServiceError::Forbidden)?;
        self.verifier.prepare(token).await?;
        let claims = self.verifier.verify(token)?;
        let secret = single(headers, "photara-device-credential", 43)?;
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(secret)
            .map_err(|_| ServiceError::Forbidden)?;
        if credential.len() != 32 {
            return Err(ServiceError::Forbidden);
        }
        let key = crate::hash(&crate::canonical(&(
            "principal",
            &claims.issuer,
            &claims.subject,
            crate::hash(&credential),
        ))?);
        if !self.limit(key, 60) {
            return Err(ServiceError::Throttled);
        }
        Ok((claims, credential))
    }
    async fn proof(
        &self,
        headers: &HeaderMap,
        claims: &crate::auth::VerifiedClaims,
    ) -> Result<Option<Proof>> {
        if !headers.contains_key("photara-enrollment-token")
            && !headers.contains_key("photara-challenge-id")
        {
            return Ok(None);
        }
        let id = Uuid::parse_str(single(headers, "photara-challenge-id", 36)?)
            .map_err(|_| ServiceError::Forbidden)?;
        if id.is_nil() {
            return Err(ServiceError::Forbidden);
        }
        let token = single(headers, "photara-enrollment-token", 16384)?;
        self.verifier.prepare(token).await?;
        let facts = self.service.proof_facts(id).await?;
        self.verifier
            .enrollment_proof(token, claims, &facts.nonce_sha256, facts.issued_ms)?;
        Ok(Some(facts))
    }
}
fn single<'a>(headers: &'a HeaderMap, name: &str, max: usize) -> Result<&'a str> {
    let values = headers.get_all(name);
    let mut it = values.iter();
    let value = it.next().ok_or(ServiceError::Forbidden)?;
    if it.next().is_some() || value.len() > max {
        return Err(ServiceError::Forbidden);
    }
    value.to_str().map_err(|_| ServiceError::Forbidden)
}
async fn bounds(
    State(state): State<Arc<HttpService>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Ok(_permit) = state.concurrency.try_acquire() else {
        return failure(StatusCode::TOO_MANY_REQUESTS, "throttled", true, true);
    };
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| crate::hash(format!("ip:{}", peer.0.ip()).as_bytes()));
    if request
        .headers()
        .iter()
        .map(|(k, v)| k.as_str().len() + v.len())
        .sum::<usize>()
        > 49152
    {
        return failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid-command",
            false,
            true,
        );
    }
    if !request.uri().path().starts_with("/health/")
        && !key.is_some_and(|key| state.limit(key, 120))
    {
        return failure(StatusCode::TOO_MANY_REQUESTS, "throttled", true, true);
    }
    let mut response = match tokio::time::timeout(Duration::from_secs(10), next.run(request)).await
    {
        Ok(r) => r,
        Err(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "timeout", true, false),
    };
    if response.status().is_client_error()
        && response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_none_or(|value| value != "application/json")
    {
        response = failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid-command",
            false,
            true,
        );
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
fn failure(status: StatusCode, code: &str, retryable: bool, outcome_known: bool) -> Response {
    let mut response=(status,axum::Json(serde_json::json!({"schema":SCHEMA,"code":code,"retryable":retryable,"outcome_known":outcome_known,"request_id":Uuid::new_v4()}))).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    if status == StatusCode::TOO_MANY_REQUESTS {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
    }
    response
}
fn authentication_failure(error: ServiceError) -> Response {
    match error {
        ServiceError::Forbidden | ServiceError::Invalid => failure(
            StatusCode::UNAUTHORIZED,
            "authentication-required",
            false,
            true,
        ),
        other => result(Err(other)),
    }
}
fn result(result: Result<Vec<u8>>) -> Response {
    match result {
        Ok(bytes) if bytes.len() <= 65536 => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        Ok(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "integrity", false, false),
        Err(error) => match error {
            ServiceError::Forbidden => failure(StatusCode::FORBIDDEN, "access-denied", false, true),
            ServiceError::Invalid => failure(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid-command",
                false,
                true,
            ),
            ServiceError::Conflict => failure(StatusCode::CONFLICT, "conflict", false, true),
            ServiceError::Unsupported => failure(
                StatusCode::UPGRADE_REQUIRED,
                "protocol-incompatible",
                false,
                true,
            ),
            ServiceError::Throttled => {
                failure(StatusCode::TOO_MANY_REQUESTS, "throttled", true, true)
            }
            _ => failure(StatusCode::SERVICE_UNAVAILABLE, "unavailable", true, false),
        },
    }
}
async fn live() -> Response {
    result(Ok(b"{\"healthy\":true}".to_vec()))
}
async fn ready(State(state): State<Arc<HttpService>>) -> Response {
    if state.verifier.warm_up().await.is_err() {
        return failure(StatusCode::SERVICE_UNAVAILABLE, "unavailable", true, true);
    }
    result(
        state
            .service
            .readiness()
            .await
            .map(|()| b"{\"ready\":true}".to_vec()),
    )
}
async fn capabilities(State(state): State<Arc<HttpService>>) -> Response {
    result(crate::canonical(
        &serde_json::json!({"schema":SCHEMA,"environment_id":state.config.environment_id,"service_origin":state.config.service_origin,"minimum_api":"3","canonical_codec":"photara.canonical-json.v1","max_body_bytes":"65536","max_token_bytes":"16384","challenge_ttl_seconds":"300","authentication_profile":"auth0-rs256-api-userinfo-v1","providers":["google"]}),
    ))
}
async fn challenge(State(state): State<Arc<HttpService>>, body: Bytes) -> Response {
    result(
        async {
            let c: ChallengeRequest = onboarding::command(&body)?;
            crate::canonical(&state.service.challenge(&c).await?)
        }
        .await,
    )
}
async fn bootstrap(
    State(state): State<Arc<HttpService>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (claims, credential) = match state.credentials(&headers).await {
        Ok(c) => c,
        Err(ServiceError::Throttled) => return result(Err(ServiceError::Throttled)),
        Err(error) => return authentication_failure(error),
    };
    // Exact replay must succeed without repeating a consumed/expired proof.
    let first = state
        .service
        .bootstrap(
            &claims,
            &state.config.environment_id,
            &body,
            &credential,
            None,
        )
        .await;
    if !matches!(first, Err(ServiceError::Forbidden)) {
        return result(first);
    }
    result(
        async {
            let proof = state.proof(&headers, &claims).await?;
            state
                .service
                .bootstrap(
                    &claims,
                    &state.config.environment_id,
                    &body,
                    &credential,
                    proof.as_ref(),
                )
                .await
        }
        .await,
    )
}
async fn operation(
    State(state): State<Arc<HttpService>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (claims, credential) = match state.credentials(&headers).await {
        Ok(c) => c,
        Err(error) => return authentication_failure(error),
    };
    let id = match Uuid::parse_str(&id)
        .ok()
        .and_then(|u| OperationId::from_uuid(u).ok())
    {
        Some(id) => id,
        None => return result(Err(ServiceError::Invalid)),
    };
    match state.service.operation(&claims, id, &credential).await {
        Ok(Some(bytes)) => result(Ok(bytes)),
        Ok(None) => failure(
            StatusCode::NOT_FOUND,
            "operation-not-yet-found",
            true,
            false,
        ),
        Err(e) => result(Err(e)),
    }
}
async fn session(State(state): State<Arc<HttpService>>, headers: HeaderMap) -> Response {
    let (claims, credential) = match state.credentials(&headers).await {
        Ok(c) => c,
        Err(error) => return authentication_failure(error),
    };
    result(
        async {
            let id = DeviceId::from_uuid(
                Uuid::parse_str(single(&headers, "photara-device-id", 36)?)
                    .map_err(|_| ServiceError::Invalid)?,
            )
            .map_err(|_| ServiceError::Invalid)?;
            state.service.session(&claims, id, &credential).await
        }
        .await,
    )
}
async fn change(
    state: Arc<HttpService>,
    headers: HeaderMap,
    body: Bytes,
    action: &str,
) -> Response {
    let (claims, credential) = match state.credentials(&headers).await {
        Ok(c) => c,
        Err(error) => return authentication_failure(error),
    };
    let first = state
        .service
        .session_change(&claims, &body, &credential, action, None)
        .await;
    if action != "resume" || !matches!(first, Err(ServiceError::Forbidden)) {
        return result(first);
    }
    result(
        async {
            let proof = state.proof(&headers, &claims).await?;
            state
                .service
                .session_change(&claims, &body, &credential, action, proof.as_ref())
                .await
        }
        .await,
    )
}
async fn resume(
    State(state): State<Arc<HttpService>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    change(state, headers, body, "resume").await
}
async fn logout(
    State(state): State<Arc<HttpService>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    change(state, headers, body, "logout").await
}

#[cfg(test)]
mod native_furnace;

#[cfg(test)]
mod tests {
    #![allow(clippy::too_many_lines)] // End-to-end wire scenario retains one readable lifecycle.
    use super::*;
    use tower::ServiceExt as _;
    async fn call(
        router: &Router,
        path: &str,
        method: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> (StatusCode, Vec<u8>) {
        let mut request = axum::http::Request::builder().uri(path).method(method);
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        let mut request = request.body(axum::body::Body::from(body)).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:4567".parse::<SocketAddr>().unwrap()));
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec();
        (status, bytes)
    }
    #[tokio::test]
    #[ignore = "requires explicit disposable PostgreSQL runner"]
    async fn postgres_http_signed_bootstrap_and_bounds() {
        let fixture = crate::pgtests::Fixture::new().await;
        let service = fixture.into_service().await;
        let oidc = OidcConfig {
            issuer: "https://issuer.invalid/".into(),
            audience: "synthetic-api".into(),
            native_client_id: "synthetic-native".into(),
            allow_userinfo_audience: true,
        };
        let verifier = Arc::new(OidcVerifier::new(oidc.clone(), service.clock.clone()).unwrap());
        let key = crate::oidc::tests::keypair();
        crate::oidc::tests::install(&verifier, &key, "synthetic");
        let state = Arc::new(HttpService {
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
        let router = state.clone().router();
        assert_eq!(
            call(&router, "/health/live", "GET", &[], vec![]).await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&router, "/health/ready", "GET", &[], vec![]).await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&router, "/v1/onboarding/capabilities", "GET", &[], vec![])
                .await
                .0,
            StatusCode::OK
        );
        let secret = [101; 32];
        let device = DeviceId::from_uuid(Uuid::new_v4()).unwrap();
        let operation = OperationId::from_uuid(Uuid::new_v4()).unwrap();
        let command = onboarding::BootstrapRequest {
            schema: SCHEMA.into(),
            operation_id: operation,
            environment_id: "synthetic".into(),
            device_id: device,
            device_credential_sha256: onboarding::hex(&crate::hash(&secret)),
            device_display_name: "HTTP fixture".into(),
            requested_library_id: photara_core::contracts::LibraryId::from_uuid(Uuid::new_v4())
                .unwrap(),
            library_display_name: "My Library".into(),
            intent: "enroll-default".into(),
        };
        let body = crate::canonical(&command).unwrap();
        let challenge = ChallengeRequest {
            schema: SCHEMA.into(),
            action: "bootstrap".into(),
            operation_id: operation,
            request_sha256: onboarding::hex(&crate::hash(&body)),
            device_id: device,
            device_credential_sha256: command.device_credential_sha256.clone(),
        };
        let (status, challenge) = call(
            &router,
            "/v1/onboarding/challenges",
            "POST",
            &[],
            crate::canonical(&challenge).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let challenge: onboarding::ChallengeResponse = serde_json::from_slice(&challenge).unwrap();
        let access = serde_json::json!({"iss":"https://issuer.invalid/","sub":Uuid::new_v4().to_string(),"aud":["synthetic-api","https://issuer.invalid/userinfo"],"azp":"synthetic-native","scope":"photara:onboard","iat":1_800_000_000,"exp":1_800_000_600});
        let bearer = format!(
            "Bearer {}",
            crate::oidc::tests::sign(&key, "synthetic", &access.to_string())
        );
        let mut proof = access.clone();
        proof["aud"] = serde_json::json!("synthetic-native");
        proof["nonce"] = serde_json::json!(challenge.nonce);
        proof["auth_time"] = serde_json::json!(1_800_000_000);
        let proof = crate::oidc::tests::sign(&key, "synthetic", &proof.to_string());
        let credential = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret);
        let challenge_id = challenge.challenge_id.to_string();
        assert_eq!(
            call(
                &router,
                "/v1/onboarding/bootstrap",
                "POST",
                &[],
                body.clone()
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        let headers = [
            ("authorization", bearer.as_str()),
            ("photara-device-credential", credential.as_str()),
        ];
        assert_eq!(
            call(
                &router,
                "/v1/onboarding/bootstrap",
                "POST",
                &headers,
                body.clone()
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
        let full = [
            headers[0],
            headers[1],
            ("photara-challenge-id", challenge_id.as_str()),
            ("photara-enrollment-token", proof.as_str()),
        ];
        let (status, response) = call(
            &router,
            "/v1/onboarding/bootstrap",
            "POST",
            &full,
            body.clone(),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&response)
        );
        assert_eq!(
            call(
                &router,
                "/v1/onboarding/bootstrap",
                "POST",
                &headers,
                body.clone()
            )
            .await,
            (StatusCode::OK, response.clone())
        );
        assert_eq!(
            call(
                &router,
                &format!("/v1/onboarding/operations/{operation}"),
                "GET",
                &headers,
                vec![]
            )
            .await,
            (StatusCode::OK, response)
        );
        assert_eq!(
            call(
                &router,
                "/v1/onboarding/bootstrap",
                "POST",
                &headers,
                vec![b'x'; 65537]
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let mut duplicate = body.clone();
        duplicate.insert(1, b' ');
        assert_eq!(
            call(
                &router,
                "/v1/onboarding/bootstrap",
                "POST",
                &headers,
                duplicate
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let invalid = [
            ("authorization", "Bearer not-a-jwt"),
            ("photara-device-credential", credential.as_str()),
        ];
        let (status, error) =
            call(&router, "/v1/onboarding/bootstrap", "POST", &invalid, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(!String::from_utf8_lossy(&error).contains("not-a-jwt"));
        for _ in 0..130 {
            let _ = call(&router, "/v1/onboarding/capabilities", "GET", &[], vec![]).await;
        }
        assert_eq!(
            call(&router, "/v1/onboarding/capabilities", "GET", &[], vec![])
                .await
                .0,
            StatusCode::TOO_MANY_REQUESTS
        );
        state.service.close().await;
    }
}
