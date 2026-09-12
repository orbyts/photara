//! Separate generation-two local authority. Never opens or converts the v1 store.
//! SQL/migrations and normalization stay here; Storexa owns database mechanics.
mod catalog;
mod library;
mod types;
mod validation;
pub use types::*;
pub use validation::normalize_term;

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use sqlx::{Row as _, Sqlite, SqliteConnection, Transaction};
use std::{
    fs::OpenOptions,
    io::Read,
    path::{Component, Path},
    time::Duration,
};
use storexa::{SqliteDatabase, SqliteDatabaseConfig, SqliteJournalMode, SqliteSynchronous};
use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error("invalid local record or path")]
    Invalid,
    #[error("local record limit exceeded")]
    Limit,
    #[error("local revision or uniqueness conflict")]
    Conflict,
    #[error("local reference or lifecycle constraint failed")]
    Constraint,
    #[error("record is retired")]
    Retired,
    #[error("unsupported local schema or record")]
    Unsupported,
    #[error("database is not the generation-two family")]
    ForeignDatabase,
    #[error("migration history is incompatible or damaged")]
    Migration,
    #[error("local state is corrupt")]
    Corrupt,
    #[error("local database is busy")]
    Busy,
    #[error("local database is closed")]
    Closed,
    #[error("local database operation failed")]
    Storage,
    #[error("local filesystem operation failed")]
    Io,
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::PoolClosed => Self::Closed,
            sqlx::Error::PoolTimedOut => Self::Busy,
            sqlx::Error::Database(ref e) if e.is_unique_violation() => Self::Conflict,
            sqlx::Error::Database(ref e)
                if matches!(e.code().as_deref(), Some("5" | "6" | "261" | "517")) =>
            {
                Self::Busy
            }
            sqlx::Error::Database(ref e)
                if e.is_foreign_key_violation()
                    || e.is_check_violation()
                    || e.code().as_deref() == Some("1811") =>
            {
                Self::Constraint
            }
            sqlx::Error::ColumnDecode { .. }
            | sqlx::Error::Decode(_)
            | sqlx::Error::ColumnNotFound(_) => Self::Corrupt,
            _ => Self::Storage,
        }
    }
}
impl From<storexa::SqliteError> for Error {
    fn from(value: storexa::SqliteError) -> Self {
        match value {
            storexa::SqliteError::Connection(e)
            | storexa::SqliteError::HealthCheck(e)
            | storexa::SqliteError::TransactionBegin(e)
            | storexa::SqliteError::TransactionCommit(e)
            | storexa::SqliteError::TransactionRollback(e)
            | storexa::SqliteError::Query(e) => e.into(),
            storexa::SqliteError::Migration(_) => Self::Migration,
            _ => Self::Storage,
        }
    }
}

const APPLICATION_ID: i64 = 0x5048_5432;
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/generation_two");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenMode {
    CreateNew,
    OpenExisting,
}

/// Private concrete pool; no raw SQL escapes to domain callers or nodes.
#[derive(Clone)]
pub struct LocalLibraryStore {
    db: SqliteDatabase,
    info: StoreInfo,
}
impl std::fmt::Debug for LocalLibraryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalLibraryStore")
            .field("stats", &self.stats())
            .finish_non_exhaustive()
    }
}
impl LocalLibraryStore {
    /// Opens only an explicit local path; never chooses a default or converts v1.
    /// The host must verify local (not SMB) storage and protect the parent directory.
    /// `CreateNew` requires an absent file; `OpenExisting` verifies its family first.
    /// # Errors
    /// Returns redacted path/family/version/migration/storage errors.
    pub async fn open(
        path: impl AsRef<Path>,
        mode: OpenMode,
        device: DeviceId,
        at: Timestamp,
    ) -> Result<Self> {
        let path = path.as_ref();
        check_path(path)?;
        match mode {
            OpenMode::CreateNew => {
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt as _;
                    options.mode(0o600);
                }
                options.open(path).map_err(|_| Error::Io)?;
            }
            OpenMode::OpenExisting => preflight(path)?,
        }
        let config = SqliteDatabaseConfig::from_path(path)
            .map_err(|_| Error::Invalid)?
            .with_max_connections(1)
            .with_min_connections(1)
            .with_idle_timeout(None)
            .with_max_lifetime(None)
            .with_busy_timeout(Duration::from_secs(2))
            .with_acquire_timeout(Duration::from_secs(3))
            .with_foreign_keys(true)
            .with_synchronous(SqliteSynchronous::Full)
            .with_journal_mode((mode == OpenMode::CreateNew).then_some(SqliteJournalMode::Wal));
        let db = SqliteDatabase::connect(config).await?;
        let result = initialize(&db, mode, device, at).await;
        match result {
            Ok(info) => Ok(Self { db, info }),
            Err(error) => {
                db.close().await;
                Err(error)
            }
        }
    }
    #[must_use]
    pub fn info(&self) -> &StoreInfo {
        &self.info
    }
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        let s = self.db.stats();
        StoreStats {
            closed: s.closed,
            connections: s.size,
            idle: s.idle,
        }
    }
    /// Closes every clone and waits for outstanding operations.
    pub async fn close(&self) {
        self.db.close().await;
    }
    /// # Errors
    /// Returns a redacted lifecycle/storage error.
    pub async fn health(&self) -> Result<()> {
        self.db.health_check().await.map_err(Error::from)
    }
    /// Checks engine integrity and every foreign key without changing records.
    /// # Errors
    /// Returns corruption or storage failure.
    pub async fn verify_integrity(&self) -> Result<()> {
        let mut tx = self.db.begin().await?;
        let checks: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_all(&mut *tx)
            .await?;
        if checks != ["ok"]
            || !sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&mut *tx)
                .await?
                .is_empty()
        {
            return Err(Error::Corrupt);
        }
        tx.commit().await?;
        Ok(())
    }
    async fn write(&self) -> Result<Transaction<'static, Sqlite>> {
        Ok(self.db.pool().begin_with("BEGIN IMMEDIATE").await?)
    }
    async fn read(&self) -> Result<storexa::SqliteTransaction> {
        Ok(self.db.begin().await?)
    }
    /// Exact immutable local history, not the S5 network feed. Bounds are 1..500.
    /// # Errors
    /// Rejects invalid bounds or damaged stored bytes/digests.
    pub async fn changes(
        &self,
        workspace: WorkspaceId,
        after: i64,
        limit: u32,
    ) -> Result<Vec<LocalChange>> {
        page(after, limit)?;
        let rows=sqlx::query("SELECT sequence,mutation_id,entity_kind,entity_id,local_revision,change_kind,post_state_json,post_state_sha256 FROM local_changes WHERE workspace_id=? AND sequence>? ORDER BY sequence LIMIT ?")
            .bind(workspace.bytes()).bind(after).bind(limit).fetch_all(self.db.pool()).await?;
        rows.into_iter()
            .map(|row| {
                let bytes: String = row.try_get("post_state_json")?;
                let digest: Vec<u8> = row.try_get("post_state_sha256")?;
                let sha256: [u8; 32] = digest.try_into().map_err(|_| Error::Corrupt)?;
                if sha(bytes.as_bytes()) != sha256 {
                    return Err(Error::Corrupt);
                }
                let post_state = photara_store::package::parse_canonical_json(
                    bytes.as_bytes(),
                    photara_store::package::JsonLimits::default(),
                )
                .map_err(|_| Error::Corrupt)?;
                Ok(LocalChange {
                    sequence: row.try_get("sequence")?,
                    mutation_id: MutationId::from_bytes(
                        &row.try_get::<Vec<u8>, _>("mutation_id")?,
                    )?,
                    entity_kind: row.try_get("entity_kind")?,
                    entity_id: Uuid::from_slice(&row.try_get::<Vec<u8>, _>("entity_id")?)
                        .map_err(|_| Error::Corrupt)?,
                    revision: Revision::try_from(row.try_get::<i64, _>("local_revision")?)?,
                    change_kind: row.try_get("change_kind")?,
                    post_state,
                    sha256,
                })
            })
            .collect()
    }
}

fn check_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.extension().and_then(|x| x.to_str()) != Some("sqlite") {
        return Err(Error::Invalid);
    }
    for part in path.components() {
        if !matches!(part, Component::RootDir | Component::Normal(_)) {
            return Err(Error::Invalid);
        }
    }
    for parent in path.ancestors() {
        if parent.extension().and_then(|x| x.to_str()) == Some("photara") {
            return Err(Error::Invalid);
        }
        if let Ok(meta) = std::fs::symlink_metadata(parent) {
            if meta.file_type().is_symlink() {
                return Err(Error::Invalid);
            }
            if parent == path && !meta.is_file() {
                return Err(Error::Invalid);
            }
        }
    }
    Ok(())
}
fn preflight(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path).map_err(|_| Error::Io)?;
    let mut header = [0; 100];
    file.read_exact(&mut header)
        .map_err(|_| Error::ForeignDatabase)?;
    if &header[..16] != b"SQLite format 3\0"
        || i64::from(u32::from_be_bytes(
            header[68..72].try_into().map_err(|_| Error::Corrupt)?,
        )) != APPLICATION_ID
    {
        return Err(Error::ForeignDatabase);
    }
    if u32::from_be_bytes(header[60..64].try_into().map_err(|_| Error::Corrupt)?) != 2 {
        return Err(Error::Unsupported);
    }
    Ok(())
}
async fn initialize(
    db: &SqliteDatabase,
    mode: OpenMode,
    device: DeviceId,
    at: Timestamp,
) -> Result<StoreInfo> {
    // One retained connection is configured now; replacements get the same options.
    let options = (*db.pool().connect_options())
        .clone()
        .pragma("recursive_triggers", "ON");
    db.pool().set_connect_options(options);
    sqlx::query("PRAGMA recursive_triggers=ON")
        .execute(db.pool())
        .await?;
    let sqlite_version = db.health().await?.sqlite_version;
    let parts: Vec<u32> = sqlite_version
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| Error::Unsupported)?;
    if parts.as_slice() < [3, 38, 0].as_slice() {
        return Err(Error::Unsupported);
    }
    for pragma in ["PRAGMA foreign_keys", "PRAGMA recursive_triggers"] {
        if sqlx::query_scalar::<_, i64>(pragma)
            .fetch_one(db.pool())
            .await?
            != 1
        {
            return Err(Error::Unsupported);
        }
    }
    if sqlx::query_scalar::<_, i64>("PRAGMA synchronous")
        .fetch_one(db.pool())
        .await?
        != 2
        || sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
            .fetch_one(db.pool())
            .await?
            != "wal"
    {
        return Err(Error::Unsupported);
    }
    sqlx::query_scalar::<_, i64>("SELECT json_valid('{}')")
        .fetch_one(db.pool())
        .await?;
    // SQLite's writer reservation serializes independent startup processes too.
    // Migrator applies nested savepoints: family, ledger and bootstrap commit together.
    let mut tx = db.pool().begin_with("BEGIN IMMEDIATE").await?;
    if mode == OpenMode::OpenExisting {
        check_metadata(&mut tx, device).await?;
    }
    MIGRATOR.run(&mut *tx).await.map_err(|_| Error::Migration)?;
    if mode == OpenMode::CreateNew {
        sqlx::query("INSERT INTO schema_metadata VALUES(1,'photara.local.g2',?,1,1,1,'photara.canonical-json.v1',?)")
            .bind(DatabaseId::new().bytes()).bind(at.get()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO local_device VALUES(1,?,'Local device',?)")
            .bind(device.bytes())
            .bind(at.get())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO normalization_policies VALUES(1,'16.0.0',?,?)")
            .bind(sha(validation::POLICY.as_bytes()).to_vec())
            .bind(validation::POLICY)
            .execute(&mut *tx)
            .await?;
    }
    let database_id = check_metadata(&mut tx, device).await?;
    tx.commit().await?;
    // Publish the family header into the main file before returning a new store;
    // later opens can reject foreign databases without interpreting their WAL.
    if mode == OpenMode::CreateNew {
        let row = sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .fetch_one(db.pool())
            .await?;
        if row.try_get::<i64, _>(0)? != 0 {
            return Err(Error::Busy);
        }
    }
    Ok(StoreInfo {
        database_id,
        device_id: device,
        sqlite_version,
        migration_count: MIGRATOR.iter().count(),
    })
}
async fn check_metadata(conn: &mut SqliteConnection, device: DeviceId) -> Result<DatabaseId> {
    if sqlx::query_scalar::<_, i64>("PRAGMA application_id")
        .fetch_one(&mut *conn)
        .await?
        != APPLICATION_ID
    {
        return Err(Error::ForeignDatabase);
    }
    if sqlx::query_scalar::<_, i64>("PRAGMA user_version")
        .fetch_one(&mut *conn)
        .await?
        != 2
    {
        return Err(Error::Unsupported);
    }
    let row=sqlx::query("SELECT schema_family,database_id,schema_epoch,minimum_reader,minimum_writer,canonical_codec FROM schema_metadata WHERE singleton=1").fetch_optional(&mut *conn).await?.ok_or(Error::Corrupt)?;
    if row.try_get::<String, _>("schema_family")? != "photara.local.g2" {
        return Err(Error::ForeignDatabase);
    }
    if row.try_get::<i64, _>("schema_epoch")? != 1
        || row.try_get::<i64, _>("minimum_reader")? != 1
        || row.try_get::<i64, _>("minimum_writer")? != 1
        || row.try_get::<String, _>("canonical_codec")? != "photara.canonical-json.v1"
    {
        return Err(Error::Unsupported);
    }
    let stored: Vec<u8> =
        // Cloud-associated stores require a later adapter with durable outbox semantics.
        // Never silently treat such a store as local-only.
        {
            let unsupported:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sync_targets) OR EXISTS(SELECT 1 FROM workspaces WHERE term_policy_version<>1)").fetch_one(&mut *conn).await?;
            if unsupported {return Err(Error::Unsupported);}
        sqlx::query_scalar("SELECT device_id FROM local_device WHERE singleton=1")
            .fetch_one(&mut *conn)
            .await?
        };
    if stored != device.bytes() {
        return Err(Error::Unsupported);
    }
    let policy = sqlx::query(
        "SELECT unicode_version,rules_sha256 FROM normalization_policies WHERE policy_version=1",
    )
    .fetch_one(&mut *conn)
    .await?;
    if policy.try_get::<String, _>("unicode_version")? != "16.0.0"
        || policy.try_get::<Vec<u8>, _>("rules_sha256")? != sha(validation::POLICY.as_bytes())
    {
        return Err(Error::Unsupported);
    }
    DatabaseId::from_bytes(&row.try_get::<Vec<u8>, _>("database_id")?)
}

#[derive(Clone, Copy)]
enum Table {
    Workspace,
    Person,
    Organization,
    Relationship,
    Kind,
    Location,
    Social,
    Root,
    Catalog,
    Locator,
}
impl Table {
    const fn names(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Workspace => ("workspaces", "workspace_id", "workspace"),
            Self::Person => ("people", "person_id", "person"),
            Self::Organization => ("organizations", "organization_id", "organization"),
            Self::Relationship => (
                "person_organization_relationships",
                "relationship_id",
                "person-organization-relationship",
            ),
            Self::Kind => ("location_kinds", "location_kind_id", "location-kind"),
            Self::Location => ("locations", "location_id", "location"),
            Self::Social => ("social_profiles", "social_profile_id", "social-profile"),
            Self::Root => ("storage_roots", "storage_root_id", "storage-root"),
            Self::Catalog => ("project_catalog", "project_id", "project-catalog"),
            Self::Locator => ("project_locators", "locator_id", "project-locator"),
        }
    }
}
async fn check_meta<I: Copy + Into<Uuid>>(
    conn: &mut SqliteConnection,
    table: Table,
    meta: &Metadata<I>,
    expected: Option<Revision>,
) -> Result<()>
where
    Uuid: From<I>,
{
    let (name, id, _) = table.names();
    let state = if matches!(table, Table::Catalog) {
        "'active'"
    } else {
        "state"
    };
    // Identifiers come exclusively from the closed Table enum, never caller input.
    let scoped = matches!(table, Table::Catalog);
    let sql = sqlx::AssertSqlSafe(format!(
        "SELECT workspace_id,local_revision,created_at_ms,{state} AS state FROM {name} WHERE {id}=? AND (? OR workspace_id=?)"
    ));
    let row = sqlx::query(sql)
        .bind(Uuid::from(meta.id).as_bytes().to_vec())
        .bind(!scoped)
        .bind(meta.workspace_id.bytes())
        .fetch_optional(&mut *conn)
        .await?;
    if let Some(row) = &row
        && row.try_get::<Vec<u8>, _>("workspace_id")? != meta.workspace_id.bytes()
    {
        return Err(Error::Conflict);
    }
    match (row, expected) {
        (None, None)
            if meta.revision == Revision::INITIAL
                && meta.state == Lifecycle::Active
                && meta.updated_at == meta.created_at => {}
        (Some(row), Some(expected)) => {
            if row.try_get::<i64, _>("local_revision")? != expected.get()
                || meta.revision != expected.next()?
                || row.try_get::<i64, _>("created_at_ms")? != meta.created_at.get()
            {
                return Err(Error::Conflict);
            }
            if row.try_get::<String, _>("state")? != "active" {
                return Err(Error::Retired);
            }
        }
        _ => return Err(Error::Conflict),
    }
    if !matches!(table, Table::Workspace) {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE workspace_id=? AND state='active')",
        )
        .bind(meta.workspace_id.bytes())
        .fetch_one(&mut *conn)
        .await?;
        if !active {
            return Err(Error::Constraint);
        }
    }
    if matches!(table, Table::Catalog) && meta.state != Lifecycle::Active {
        return Err(Error::Invalid);
    }
    Ok(())
}
async fn finish<I: Copy + Into<Uuid>, T: Serialize>(
    conn: &mut SqliteConnection,
    device: DeviceId,
    table: Table,
    meta: &Metadata<I>,
    expected: Option<Revision>,
    record: &T,
) -> Result<WriteOutcome>
where
    Uuid: From<I>,
{
    let post = canonical(record)?;
    let mutation = MutationId::new();
    let change = if meta.state == Lifecycle::Tombstoned {
        "tombstone"
    } else if expected.is_none() {
        "create"
    } else {
        "update"
    };
    let envelope = canonical(
        &serde_json::json!({"schema":"photara.local-command.v1","kind":table.names().2,"entity_id":Uuid::from(meta.id),"expected_local_revision":expected,"change":change,"post_state_sha256":hex(&sha(post.as_bytes()))}),
    )?;
    sqlx::query("INSERT INTO mutations(mutation_id,workspace_id,source_device_id,origin,command_schema,primary_entity_kind,primary_entity_id,created_at_ms,envelope_json,envelope_sha256) VALUES(?,?,?,'local',1,?,?,?,?,?)")
        .bind(mutation.bytes()).bind(meta.workspace_id.bytes()).bind(device.bytes()).bind(table.names().2).bind(Uuid::from(meta.id).as_bytes().to_vec()).bind(meta.updated_at.get()).bind(&envelope).bind(sha(envelope.as_bytes()).to_vec()).execute(&mut *conn).await?;
    sqlx::query("INSERT INTO local_changes(workspace_id,mutation_id,entity_kind,entity_id,local_revision,change_kind,changed_at_ms,post_state_json,post_state_sha256) VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(meta.workspace_id.bytes()).bind(mutation.bytes()).bind(table.names().2).bind(Uuid::from(meta.id).as_bytes().to_vec()).bind(meta.revision.get()).bind(change).bind(meta.updated_at.get()).bind(&post).bind(sha(post.as_bytes()).to_vec()).execute(&mut *conn).await?;
    Ok(WriteOutcome {
        mutation_id: mutation,
        duplicate_manual_handles: Vec::new(),
    })
}
fn canonical<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value).map_err(|_| Error::Invalid)?;
    let bytes = photara_core::canonical_json(&value).map_err(|_| Error::Invalid)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Limit);
    }
    String::from_utf8(bytes).map_err(|_| Error::Invalid)
}
fn json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(|_| Error::Corrupt)
}
fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut text, "{byte:02x}").expect("writing to String is infallible");
    }
    text
}
fn page(after: i64, limit: u32) -> Result<()> {
    if after < 0 || !(1..=500).contains(&limit) {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}

fn metadata<I: TryFrom<Uuid, Error = Error>>(
    row: &sqlx::sqlite::SqliteRow,
    id: &str,
) -> Result<Metadata<I>> {
    let uuid = Uuid::from_slice(&row.try_get::<Vec<u8>, _>(id)?).map_err(|_| Error::Corrupt)?;
    let state: String = row.try_get("state")?;
    Ok(Metadata {
        id: I::try_from(uuid)?,
        workspace_id: WorkspaceId::from_bytes(&row.try_get::<Vec<u8>, _>("workspace_id")?)?,
        revision: Revision::try_from(row.try_get::<i64, _>("local_revision")?)?,
        created_at: Timestamp::try_from(row.try_get::<i64, _>("created_at_ms")?)?,
        updated_at: Timestamp::try_from(row.try_get::<i64, _>("updated_at_ms")?)?,
        state: Lifecycle::parse(if state == "retired" {
            "tombstoned"
        } else {
            &state
        })?,
    })
}

#[cfg(test)]
mod tests;
