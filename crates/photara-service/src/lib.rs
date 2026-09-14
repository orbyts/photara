//! Photara-owned `PostgreSQL` controllers above Storexa. No native UI, package
//! publisher, production identity provider or network server is installed here.
mod access;
pub mod auth;
mod fake_sync;
pub mod http;
mod management;
pub mod oidc;
pub mod onboarding;
pub mod release;
pub use fake_sync::{FakeSync, IntentState};
mod media;
mod runtime;
mod sync;
pub use access::{
    AcceptInvitation, Disclosure, GrantChange, Invite, ProjectDiscovery, RegisterProject, SetGrant,
    SetMembership, SetPolicy,
};
pub use management::{ClaimLibrary, TransferManager};
pub use media::{CreateUpload, FakeMedia, MediaAdapter, MediaPurpose};
pub use runtime::{Actor, ControlReceipt, Request, Scope, Service};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use sqlx::Row as _;
use storexa::{Database, DatabaseConfig};
pub use sync::{
    Batch, CatalogProjection, ContentReceipt, ContentRoot, Cursor, FeedPage, PublishContent,
    Snapshot, StorageProjection,
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ServiceError>;
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("transaction-retry-required")]
    RetryTransaction,
    #[error("throttled")]
    Throttled,
    #[error("not-found-or-forbidden")]
    Forbidden,
    #[error("invalid-request")]
    Invalid,
    #[error("revision-or-idempotency-conflict")]
    Conflict,
    #[error("unsupported-schema-or-contract")]
    Unsupported,
    #[error("stored-integrity-failure")]
    Integrity,
    #[error("database-unavailable")]
    Storage,
    #[error("migration-rejected")]
    Migration(#[source] sqlx::migrate::MigrateError),
}
impl From<sqlx::Error> for ServiceError {
    fn from(error: sqlx::Error) -> Self {
        if let Some(db) = error.as_database_error() {
            match db.code().as_deref() {
                Some("42501") => Self::Forbidden,
                Some("40001" | "40P01" | "55P03") => Self::RetryTransaction,
                Some("23505" | "23514") => Self::Conflict,
                Some("23503" | "22P02" | "22003") => Self::Invalid,
                _ => Self::Storage,
            }
        } else {
            Self::Storage
        }
    }
}
impl From<storexa::StorexaError> for ServiceError {
    fn from(error: storexa::StorexaError) -> Self {
        match error {
            storexa::StorexaError::TransactionCommit(source)
            | storexa::StorexaError::Query(source) => Self::from(source),
            _ => Self::Storage,
        }
    }
}
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/postgres");
const POLICY: &str = "photara.term-policy.v1;unicode=16.0.0;NFC;full-default-casefold;NFC;UCD-White_Space-collapse;groups=beach,beaches|studio,studios";

/// Applies only this clean service family, through an explicitly supplied
/// migration-owner connection. Runtime service logins never receive this handle.
/// # Errors
/// Refuses an unknown/populated database, checksums, floors or invalid role setup.
pub async fn migrate(config: DatabaseConfig) -> Result<()> {
    let db = Database::connect(config.with_max_connections(1)).await?;
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await?;
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('photara.schema_metadata') IS NOT NULL")
            .fetch_one(&mut *tx)
            .await?;
    if present {
        let row=sqlx::query("SELECT schema_family,schema_epoch,minimum_api,canonical_codec FROM photara.schema_metadata WHERE singleton").fetch_one(&mut *tx).await?;
        if row.try_get::<String, _>("schema_family")? != "photara.service.g2"
            || row.try_get::<i32, _>("schema_epoch")? != 1
            || !(2..=3).contains(&row.try_get::<i32, _>("minimum_api")?)
            || row.try_get::<String, _>("canonical_codec")? != "photara.canonical-json.v1"
        {
            return Err(ServiceError::Unsupported);
        }
    } else {
        let tables:i64=sqlx::query_scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname NOT IN ('pg_catalog','information_schema') AND n.nspname NOT LIKE 'pg_toast%' AND c.relkind IN ('r','p')").fetch_one(&mut *tx).await?;
        if tables != 0 {
            return Err(ServiceError::Unsupported);
        }
        let baseline = sqlx::migrate::Migrator::with_migrations(
            MIGRATOR
                .iter()
                .filter(|m| m.version <= 7)
                .cloned()
                .collect::<Vec<_>>(),
        );
        baseline
            .run(&mut *tx)
            .await
            .map_err(ServiceError::Migration)?;
        sqlx::query("INSERT INTO photara.schema_metadata VALUES(true,'photara.service.g2',1,1,'photara.canonical-json.v1',CURRENT_TIMESTAMP)").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara.normalization_policies VALUES(1,'16.0.0',$1,$2)")
            .bind(hash(POLICY.as_bytes()).to_vec())
            .bind(POLICY)
            .execute(&mut *tx)
            .await?;
    }
    MIGRATOR
        .run(&mut *tx)
        .await
        .map_err(ServiceError::Migration)?;
    let floor: i32 =
        sqlx::query_scalar("SELECT minimum_api FROM photara.schema_metadata WHERE singleton")
            .fetch_one(&mut *tx)
            .await?;
    if floor != 3 {
        return Err(ServiceError::Unsupported);
    }
    tx.commit().await?;
    db.close().await;
    Ok(())
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = photara_core::canonical_json(value).map_err(|_| ServiceError::Invalid)?;
    if bytes.len() > 1_048_576 {
        return Err(ServiceError::Invalid);
    }
    Ok(bytes)
}

#[cfg(test)]
mod onboarding_pgtests;
#[cfg(test)]
mod pgtests;

fn secret_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq as _;
    bool::from(a.ct_eq(b))
}
