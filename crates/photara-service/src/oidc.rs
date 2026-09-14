//! Pinned RS256 verification. Tokens and key responses never implement Debug.
use crate::{
    Result, ServiceError,
    auth::{Clock, IdentityVerifier, VerifiedClaims},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use photara_core::contracts::schema::decode_strict;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};

#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub native_client_id: String,
    pub allow_userinfo_audience: bool,
}
impl OidcConfig {
    /// # Errors
    /// Requires literal HTTPS issuer origin with trailing slash and explicit identifiers.
    pub fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.issuer).map_err(|_| ServiceError::Invalid)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || url.path() != "/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.as_str() != self.issuer
            || self.audience.is_empty()
            || self.audience.len() > 2048
            || self.native_client_id.is_empty()
            || self.native_client_id.len() > 255
        {
            return Err(ServiceError::Invalid);
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    kid: String,
    typ: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}
impl Audience {
    fn values(&self) -> Vec<&str> {
        match self {
            Self::One(s) => vec![s],
            Self::Many(a) => a.iter().map(String::as_str).collect(),
        }
    }
}
#[derive(Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: Audience,
    exp: i64,
    iat: i64,
    nbf: Option<i64>,
    azp: Option<String>,
    scope: Option<String>,
    nonce: Option<String>,
    auth_time: Option<i64>,
    gty: Option<String>,
}
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
}
#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}
#[derive(Deserialize)]
struct Jwk {
    kty: String,
    kid: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    n: String,
    e: String,
    key_ops: Option<Vec<String>>,
}
struct Key {
    n: Vec<u8>,
    e: Vec<u8>,
}
#[derive(Default)]
struct Cache {
    keys: BTreeMap<String, Key>,
    expires_ms: i64,
    last_attempt_ms: Option<i64>,
}

pub struct OidcVerifier {
    config: OidcConfig,
    clock: Arc<dyn Clock>,
    client: reqwest::Client,
    cache: RwLock<Cache>,
    refresh: tokio::sync::Mutex<()>,
}
impl OidcVerifier {
    /// # Errors
    /// Refuses unpinned or incomplete configuration; performs no network request.
    pub fn new(config: OidcConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        config.validate()?;
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| ServiceError::Storage)?;
        Ok(Self {
            config,
            clock,
            client,
            cache: RwLock::default(),
            refresh: tokio::sync::Mutex::new(()),
        })
    }
    async fn document<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| ServiceError::Storage)?;
        if !response.status().is_success() || response.content_length().is_some_and(|n| n > 65536) {
            return Err(ServiceError::Storage);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| ServiceError::Storage)? {
            if body.len() + chunk.len() > 65536 {
                return Err(ServiceError::Storage);
            }
            body.extend_from_slice(&chunk);
        }
        decode_strict(&body, 65536).map_err(|_| ServiceError::Storage)
    }
    fn header(token: &str) -> Result<Header> {
        if token.len() > 16384 {
            return Err(ServiceError::Forbidden);
        }
        let pieces: Vec<_> = token.split('.').collect();
        if pieces.len() != 3 || pieces.iter().any(|s| s.is_empty()) {
            return Err(ServiceError::Forbidden);
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(pieces[0])
            .map_err(|_| ServiceError::Forbidden)?;
        let header: Header = decode_strict(&bytes, 2048).map_err(|_| ServiceError::Forbidden)?;
        if header.alg != "RS256"
            || header.kid.is_empty()
            || header.kid.len() > 128
            || header
                .typ
                .as_deref()
                .is_some_and(|s| s != "JWT" && s != "at+jwt")
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(header)
    }
    /// Single-flight, at most one refresh per 30 seconds, including failed attempts.
    /// # Errors
    /// Missing or expired trust keys fail closed; no token-selected URL is fetched.
    pub async fn prepare(&self, token: &str) -> Result<()> {
        let header = Self::header(token)?;
        self.ensure_keys(Some(&header.kid)).await
    }
    /// # Errors
    /// Loads pinned public trust keys before readiness can become healthy.
    pub async fn warm_up(&self) -> Result<()> {
        self.ensure_keys(None).await
    }
    async fn ensure_keys(&self, kid: Option<&str>) -> Result<()> {
        let _guard = self.refresh.lock().await;
        let now = self.clock.now_ms();
        {
            let mut cache = self.cache.write().map_err(|_| ServiceError::Storage)?;
            if cache.expires_ms > now
                && !cache.keys.is_empty()
                && kid.is_none_or(|kid| cache.keys.contains_key(kid))
            {
                return Ok(());
            }
            if cache
                .last_attempt_ms
                .is_some_and(|last| now.saturating_sub(last) < 30000)
            {
                return Err(ServiceError::Forbidden);
            }
            cache.last_attempt_ms = Some(now);
        }
        let discovery: Discovery = self
            .document(&format!(
                "{}.well-known/openid-configuration",
                self.config.issuer
            ))
            .await?;
        let jwks_url = format!("{}.well-known/jwks.json", self.config.issuer);
        if discovery.issuer != self.config.issuer || discovery.jwks_uri != jwks_url {
            return Err(ServiceError::Forbidden);
        }
        let jwks: Jwks = self.document(&jwks_url).await?;
        self.install(jwks, now)?;
        let cache = self.cache.read().map_err(|_| ServiceError::Storage)?;
        if kid.is_some_and(|kid| !cache.keys.contains_key(kid)) {
            return Err(ServiceError::Forbidden);
        }
        Ok(())
    }
    fn install(&self, jwks: Jwks, now: i64) -> Result<()> {
        if jwks.keys.is_empty() || jwks.keys.len() > 16 {
            return Err(ServiceError::Forbidden);
        }
        let mut keys = BTreeMap::new();
        for key in jwks.keys {
            if key.kty != "RSA"
                || key.alg.as_deref().is_some_and(|a| a != "RS256")
                || key.usage.as_deref().is_some_and(|u| u != "sig")
                || key.key_ops.as_ref().is_some_and(|ops| ops != &["verify"])
                || key.kid.is_empty()
                || key.kid.len() > 128
            {
                return Err(ServiceError::Forbidden);
            }
            let n = URL_SAFE_NO_PAD
                .decode(key.n)
                .map_err(|_| ServiceError::Forbidden)?;
            let e = URL_SAFE_NO_PAD
                .decode(key.e)
                .map_err(|_| ServiceError::Forbidden)?;
            if !(256..=512).contains(&n.len())
                || n[0] < 128
                || e.is_empty()
                || e.len() > 4
                || keys.insert(key.kid, Key { n, e }).is_some()
            {
                return Err(ServiceError::Forbidden);
            }
        }
        let mut cache = self.cache.write().map_err(|_| ServiceError::Storage)?;
        cache.keys = keys;
        cache.expires_ms = now.saturating_add(3_600_000);
        Ok(())
    }
    fn signed(&self, token: &str) -> Result<Claims> {
        let header = Self::header(token)?;
        let pieces: Vec<_> = token.split('.').collect();
        let signature = URL_SAFE_NO_PAD
            .decode(pieces[2])
            .map_err(|_| ServiceError::Forbidden)?;
        let cache = self.cache.read().map_err(|_| ServiceError::Storage)?;
        if cache.expires_ms <= self.clock.now_ms() {
            return Err(ServiceError::Forbidden);
        }
        let key = cache.keys.get(&header.kid).ok_or(ServiceError::Forbidden)?;
        let message = format!("{}.{}", pieces[0], pieces[1]);
        ring::signature::RsaPublicKeyComponents {
            n: &key.n,
            e: &key.e,
        }
        .verify(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            message.as_bytes(),
            &signature,
        )
        .map_err(|_| ServiceError::Forbidden)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(pieces[1])
            .map_err(|_| ServiceError::Forbidden)?;
        let c: Claims = decode_strict(&bytes, 12288).map_err(|_| ServiceError::Forbidden)?;
        let now = self.clock.now_ms().div_euclid(1000);
        if c.iss != self.config.issuer
            || c.sub.is_empty()
            || c.sub.len() > 255
            || c.exp <= now
            || c.iat > now.saturating_add(60)
            || c.iat > c.exp
            || c.exp <= 0
            || c.iat < 0
            || c.nbf
                .is_some_and(|n| n > now.saturating_add(60) || n > c.exp)
            || c.gty.as_deref() == Some("client-credentials")
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(c)
    }
    /// # Errors
    /// Verifies a fresh ID token for this principal and challenge nonce.
    pub fn enrollment_proof(
        &self,
        token: &str,
        access: &VerifiedClaims,
        nonce_hash: &[u8],
        issued_ms: i64,
    ) -> Result<()> {
        let c = self.signed(token)?;
        let aud = c.aud.values();
        let now = self.clock.now_ms().div_euclid(1000);
        if c.sub != access.subject
            || c.iss != access.issuer
            || aud != [self.config.native_client_id.as_str()]
            || c.azp
                .as_deref()
                .is_some_and(|a| a != self.config.native_client_id)
            || c.auth_time.is_none_or(|t| {
                t < issued_ms.div_euclid(1000) - 60 || t < now - 300 || t > now + 60
            })
            || c.nonce
                .as_ref()
                .is_none_or(|n| !crate::secret_eq(&crate::hash(n.as_bytes()), nonce_hash))
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(())
    }
    #[must_use]
    pub fn ready(&self) -> bool {
        self.cache
            .read()
            .is_ok_and(|cache| !cache.keys.is_empty() && cache.expires_ms > self.clock.now_ms())
    }
}
impl IdentityVerifier for OidcVerifier {
    fn verify(&self, token: &str) -> Result<VerifiedClaims> {
        let c = self.signed(token)?;
        let aud = c.aud.values();
        let userinfo = format!("{}userinfo", self.config.issuer);
        if c.exp.saturating_sub(c.iat) > 600
            || !aud.contains(&self.config.audience.as_str())
            || aud.is_empty()
            || aud.len() > 2
            || (aud.len() == 2 && aud[0] == aud[1])
            || aud.iter().any(|a| {
                *a != self.config.audience
                    && !(self.config.allow_userinfo_audience && *a == userinfo)
            })
            || c.azp.as_deref() != Some(self.config.native_client_id.as_str())
            || !c
                .scope
                .as_deref()
                .is_some_and(|s| s.split_ascii_whitespace().any(|s| s == "photara:onboard"))
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(VerifiedClaims {
            issuer: c.iss,
            subject: c.sub,
            audience: self.config.audience.clone(),
            expires_ms: c.exp.checked_mul(1000).ok_or(ServiceError::Forbidden)?,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unreadable_literal)] // JWT numeric-date fixtures read like their wire representation.
    use super::*;
    use ring::signature::KeyPair as _;
    // Every test key is generated in process memory and discarded. It is never an
    // application credential, trusted deployment key or committed private fixture.
    pub(crate) fn keypair() -> ring::signature::RsaKeyPair {
        use std::io::Write as _;
        use std::process::{Command, Stdio};
        let key = Command::new("openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
            ])
            .output()
            .unwrap();
        assert!(key.status.success());
        let mut child = Command::new("openssl")
            .args(["pkcs8", "-topk8", "-nocrypt", "-outform", "DER"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&key.stdout).unwrap();
        let der = child.wait_with_output().unwrap();
        assert!(der.status.success());
        ring::signature::RsaKeyPair::from_pkcs8(&der.stdout).unwrap()
    }
    fn der_value<'a>(bytes: &mut &'a [u8]) -> &'a [u8] {
        let mut length = usize::from(bytes[1]);
        let mut offset = 2;
        if length & 128 != 0 {
            let count = length & 127;
            length = 0;
            for b in &bytes[2..2 + count] {
                length = length * 256 + usize::from(*b);
            }
            offset += count;
        }
        let value = &bytes[offset..offset + length];
        *bytes = &bytes[offset + length..];
        value
    }
    fn jwks(key: &ring::signature::RsaKeyPair, kid: &str) -> Jwks {
        let mut public = key.public_key().as_ref();
        let mut sequence = der_value(&mut public);
        let n = der_value(&mut sequence);
        let n = if n[0] == 0 { &n[1..] } else { n };
        let e = der_value(&mut sequence);
        Jwks {
            keys: vec![Jwk {
                kty: "RSA".into(),
                kid: kid.into(),
                alg: Some("RS256".into()),
                usage: Some("sig".into()),
                n: URL_SAFE_NO_PAD.encode(n),
                e: URL_SAFE_NO_PAD.encode(e),
                key_ops: None,
            }],
        }
    }
    pub(crate) fn sign(key: &ring::signature::RsaKeyPair, kid: &str, claims: &str) -> String {
        let message = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(format!(r#"{{"alg":"RS256","kid":"{kid}"}}"#)),
            URL_SAFE_NO_PAD.encode(claims)
        );
        let mut signature = vec![0; key.public().modulus_len()];
        key.sign(
            &ring::signature::RSA_PKCS1_SHA256,
            &ring::rand::SystemRandom::new(),
            message.as_bytes(),
            &mut signature,
        )
        .unwrap();
        format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature))
    }
    pub(crate) fn install(verifier: &OidcVerifier, key: &ring::signature::RsaKeyPair, kid: &str) {
        verifier
            .install(jwks(key, kid), verifier.clock.now_ms())
            .unwrap();
    }
    #[test]
    #[allow(clippy::too_many_lines)] // One signed-claims admission matrix.
    fn signed_rs256_claims_proof_rotation_and_expiry() {
        let clock = Arc::new(crate::auth::FakeClock::new(1_800_000_000_000));
        let verifier = OidcVerifier::new(
            OidcConfig {
                issuer: "https://issuer.invalid/".into(),
                audience: "photara-test".into(),
                native_client_id: "native-test".into(),
                allow_userinfo_audience: true,
            },
            clock.clone(),
        )
        .unwrap();
        let key = keypair();
        verifier
            .install(jwks(&key, "first"), clock.now_ms())
            .unwrap();
        let claims = serde_json::json!({"iss":"https://issuer.invalid/","sub":"exact-subject","aud":"photara-test","azp":"native-test","scope":"openid photara:onboard","iat":1800000000,"exp":1800000600});
        let token = sign(&key, "first", &claims.to_string());
        assert!(verifier.verify(&token).is_ok());
        let mut paired = claims.clone();
        paired["aud"] = serde_json::json!(["photara-test", "https://issuer.invalid/userinfo"]);
        assert!(
            verifier
                .verify(&sign(&key, "first", &paired.to_string()))
                .is_ok()
        );
        for (field, value) in [
            ("iss", serde_json::json!("https://issuer.invalid")),
            ("aud", serde_json::json!("photara-test-suffix")),
            ("aud", serde_json::json!(["photara-test", "evil"])),
            ("aud", serde_json::json!(["photara-test", "photara-test"])),
            (
                "aud",
                serde_json::json!(["https://issuer.invalid/userinfo"]),
            ),
            (
                "aud",
                serde_json::json!(["photara-test", "https://other.invalid/userinfo"]),
            ),
            (
                "aud",
                serde_json::json!(["photara-test", "https://issuer.invalid/userinfo", "extra"]),
            ),
            ("sub", serde_json::json!("")),
            ("sub", serde_json::json!("x".repeat(256))),
            ("azp", serde_json::json!("other")),
            ("azp", serde_json::Value::Null),
            ("scope", serde_json::json!("photara:onboard-other")),
            ("exp", serde_json::json!(1800000601)),
            ("exp", serde_json::json!(1800000000)),
            ("iat", serde_json::json!(1800000061)),
            ("nbf", serde_json::json!(1800000061)),
            ("gty", serde_json::json!("client-credentials")),
        ] {
            let mut altered = claims.clone();
            altered[field] = value;
            assert!(
                verifier
                    .verify(&sign(&key, "first", &altered.to_string()))
                    .is_err(),
                "{field}"
            );
        }
        let duplicate = claims.to_string().replacen('{', r#"{"sub":"injected","#, 1);
        assert!(verifier.verify(&sign(&key, "first", &duplicate)).is_err());
        let mut id = claims.clone();
        id["aud"] = serde_json::json!("native-test");
        id["nonce"] = serde_json::json!("fresh-nonce");
        id["auth_time"] = serde_json::json!(1800000000);
        let id_token = sign(&key, "first", &id.to_string());
        let access = verifier.verify(&token).unwrap();
        assert!(verifier.verify(&id_token).is_err());
        assert!(
            verifier
                .enrollment_proof(
                    &id_token,
                    &access,
                    &crate::hash(b"fresh-nonce"),
                    clock.now_ms()
                )
                .is_ok()
        );
        assert!(
            verifier
                .enrollment_proof(&id_token, &access, &crate::hash(b"wrong"), clock.now_ms())
                .is_err()
        );
        let other = keypair();
        assert!(
            verifier
                .verify(&sign(&other, "first", &claims.to_string()))
                .is_err()
        );
        verifier
            .install(jwks(&other, "rotated"), clock.now_ms())
            .unwrap();
        assert!(verifier.verify(&token).is_err());
        assert!(
            verifier
                .verify(&sign(&other, "rotated", &claims.to_string()))
                .is_ok()
        );
        clock.set(1_800_003_600_001);
        assert!(!verifier.ready());
        assert!(verifier.verify(&token).is_err());
    }
    #[test]
    fn token_header_admission() {
        for header in [
            r#"{"alg":"none","kid":"a"}"#,
            r#"{"alg":"HS256","kid":"a"}"#,
            r#"{"alg":"RS256","kid":"a","jku":"https://evil.invalid"}"#,
            r#"{"alg":"RS256","kid":"a","kid":"b"}"#,
            r#"{"alg":"RS256","kid":"a","crit":[]}"#,
        ] {
            assert!(
                OidcVerifier::header(&format!("{}.eA.eA", URL_SAFE_NO_PAD.encode(header))).is_err()
            );
        }
        assert!(OidcVerifier::header(&"x".repeat(16385)).is_err());
    }
}
