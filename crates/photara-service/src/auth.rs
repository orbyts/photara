//! Injected identity verification. The fake issuer is deliberately not a JWT implementation.
use crate::{Result, ServiceError};
use std::{
    collections::BTreeMap,
    sync::{
        RwLock,
        atomic::{AtomicI64, Ordering},
    },
};
use uuid::Uuid;

#[derive(Clone)]
pub struct VerifiedClaims {
    pub issuer: String,
    pub subject: String,
    pub audience: String,
    pub expires_ms: i64,
}
/// Implementations must verify signatures before returning claims; no email linking.
pub trait IdentityVerifier: Send + Sync {
    /// # Errors
    /// Rejects invalid signatures or tokens.
    fn verify(&self, token: &str) -> Result<VerifiedClaims>;
}
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}
#[derive(Default)]
pub struct FakeClock(AtomicI64);
impl FakeClock {
    #[must_use]
    pub const fn new(ms: i64) -> Self {
        Self(AtomicI64::new(ms))
    }
    pub fn set(&self, ms: i64) {
        self.0.store(ms, Ordering::SeqCst);
    }
}
impl Clock for FakeClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}
/// Opaque random bearer registry for disposable tests only. No external calls.
#[derive(Default)]
pub struct FakeAuth0 {
    tokens: RwLock<BTreeMap<String, VerifiedClaims>>,
}
impl FakeAuth0 {
    /// # Errors
    /// Returns storage failure if the fake registry is poisoned.
    pub fn issue(&self, claims: VerifiedClaims) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        self.tokens
            .write()
            .map_err(|_| ServiceError::Storage)?
            .insert(token.clone(), claims);
        Ok(token)
    }
}
impl IdentityVerifier for FakeAuth0 {
    fn verify(&self, token: &str) -> Result<VerifiedClaims> {
        self.tokens
            .read()
            .map_err(|_| ServiceError::Storage)?
            .get(token)
            .cloned()
            .ok_or(ServiceError::Forbidden)
    }
}
