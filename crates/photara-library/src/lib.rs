//! User-level Library identity and persistence.
//!
//! This crate is deliberately separate from the node catalog and visual asset
//! Gallery. Every client works against a local repository. Optional cloud
//! providers synchronize records through a host-owned service; node packages
//! never receive SQL, credentials, or network authority.

pub mod gen2;

use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryMode {
    OnThisMac,
    PhotaraCloud,
    ICloud,
}

impl LibraryMode {
    /// All modes retain a local working copy; cloud choices add synchronization.
    #[must_use]
    pub const fn uses_local_sqlite(self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryRecordKind {
    Person,
    Client,
    Location,
    Scene,
}

impl LibraryRecordKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Client => "client",
            Self::Location => "location",
            Self::Scene => "scene",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryRecordHeader {
    /// User or future studio/library boundary. Authentication mapping is host-owned.
    pub owner_id: String,
    pub record_id: Uuid,
    pub revision: u64,
    pub updated_at_millis: i64,
    pub deleted: bool,
    /// Content-addressed PNG thumbnail; bytes belong to the host media store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LibraryRecord {
    Person {
        header: LibraryRecordHeader,
        display_name: String,
        aliases: Vec<String>,
        /// Semantic roles such as `model`, `photographer`, or `stylist`.
        roles: Vec<String>,
    },
    Client {
        header: LibraryRecordHeader,
        display_name: String,
        aliases: Vec<String>,
        #[serde(default)]
        organization: bool,
    },
    Location {
        header: LibraryRecordHeader,
        display_name: String,
        parent_location_id: Option<Uuid>,
        address: Option<String>,
        aliases: Vec<String>,
    },
    Scene {
        header: LibraryRecordHeader,
        display_name: String,
        aliases: Vec<String>,
        tags: Vec<String>,
        #[serde(default)]
        description: String,
    },
}

impl LibraryRecord {
    #[must_use]
    pub const fn header(&self) -> &LibraryRecordHeader {
        match self {
            Self::Person { header, .. }
            | Self::Client { header, .. }
            | Self::Location { header, .. }
            | Self::Scene { header, .. } => header,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> LibraryRecordKind {
        match self {
            Self::Person { .. } => LibraryRecordKind::Person,
            Self::Client { .. } => LibraryRecordKind::Client,
            Self::Location { .. } => LibraryRecordKind::Location,
            Self::Scene { .. } => LibraryRecordKind::Scene,
        }
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Person { display_name, .. }
            | Self::Client { display_name, .. }
            | Self::Location { display_name, .. }
            | Self::Scene { display_name, .. } => display_name,
        }
    }

    fn search_text(&self) -> String {
        let mut values = vec![self.display_name().to_owned()];
        match self {
            Self::Person { aliases, roles, .. } => {
                values.extend(aliases.iter().cloned());
                values.extend(roles.iter().cloned());
            }
            Self::Client { aliases, .. } | Self::Scene { aliases, .. } => {
                values.extend(aliases.iter().cloned());
                if let Self::Scene {
                    tags, description, ..
                } = self
                {
                    values.extend(tags.iter().cloned());
                    values.push(description.clone());
                }
            }
            Self::Location {
                aliases, address, ..
            } => {
                values.extend(aliases.iter().cloned());
                values.extend(address.iter().cloned());
            }
        }
        values.join(" ").to_lowercase()
    }

    /// Validates portable record fields.
    ///
    /// # Errors
    /// Returns an invalid-record error for missing identity or display values.
    pub fn validate(&self) -> Result<(), LibraryError> {
        let header = self.header();
        if header.owner_id.trim().is_empty() {
            return Err(LibraryError::InvalidRecord("owner_id is empty"));
        }
        if let Some(digest) = &header.thumbnail_digest
            && (digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        {
            return Err(LibraryError::InvalidRecord("invalid thumbnail digest"));
        }
        if header.revision == 0 {
            return Err(LibraryError::InvalidRecord("revision must begin at one"));
        }
        if self.display_name().trim().is_empty() {
            return Err(LibraryError::InvalidRecord("display_name is empty"));
        }
        Ok(())
    }
}

/// Portable project reference. The snapshot keeps old projects intelligible if
/// a global Library record is renamed, deleted, offline, or owned elsewhere.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLibraryReference {
    pub owner_id: String,
    pub record_id: Uuid,
    pub kind: LibraryRecordKind,
    pub relationship: String,
    pub display_name_snapshot: String,
    pub record_revision_snapshot: u64,
}

/// Unique project occurrence, including a historical Library snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLibraryAssignment {
    pub assignment_id: Uuid,
    pub reference: ProjectLibraryReference,
    pub date: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLibraryContext {
    pub revision: u64,
    pub assignments: Vec<ProjectLibraryAssignment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryQuery {
    pub owner_id: String,
    pub kind: Option<LibraryRecordKind>,
    pub text: String,
    pub limit: usize,
}

/// Incremental change identity; payload is resolved through `get`, including tombstones.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryChange {
    pub sequence: u64,
    pub record_id: Uuid,
    pub revision: u64,
    pub changed_at_millis: i64,
}

pub trait LibraryRepository {
    /// Reads a bounded owner-scoped change page after an exclusive cursor.
    ///
    /// # Errors
    /// Returns serialization or storage errors.
    fn changes(
        &self,
        owner_id: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<LibraryChange>, LibraryError>;

    /// Creates at revision 1 when `expected_revision` is `None`; otherwise
    /// atomically replaces the matching revision with its immediate successor.
    ///
    /// # Errors
    ///
    /// Returns validation, revision-conflict, serialization, or storage errors.
    fn put(
        &mut self,
        record: &LibraryRecord,
        expected_revision: Option<u64>,
    ) -> Result<(), LibraryError>;
    /// Returns a record owned by `owner_id`, when present.
    ///
    /// # Errors
    ///
    /// Returns serialization or storage errors.
    fn get(&self, owner_id: &str, record_id: Uuid) -> Result<Option<LibraryRecord>, LibraryError>;

    /// Finds live records within one owner boundary.
    ///
    /// # Errors
    ///
    /// Returns serialization or storage errors.
    fn search(&self, query: &LibraryQuery) -> Result<Vec<LibraryRecord>, LibraryError>;
}

/// First production adapter: created lazily in Application Support on first
/// launch, never by the installer. WAL and foreign-key enforcement are explicit.
pub struct SqliteLibraryRepository {
    connection: Connection,
}

impl SqliteLibraryRepository {
    /// Opens or creates the local Library at `path` and applies its schema.
    ///
    /// # Errors
    ///
    /// Returns an I/O, `SQLite`, or unsupported-schema error.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LibraryError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        Self::configure(connection)
    }

    /// Creates an isolated in-memory Library, primarily for tests and previews.
    ///
    /// # Errors
    ///
    /// Returns a `SQLite` or unsupported-schema error.
    pub fn open_in_memory() -> Result<Self, LibraryError> {
        Self::configure(Connection::open_in_memory()?)
    }

    fn configure(mut connection: Connection) -> Result<Self, LibraryError> {
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: i64 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if current == 0 {
            transaction.execute_batch(
                "CREATE TABLE library_records (
                   owner_id TEXT NOT NULL,
                   record_id TEXT NOT NULL,
                   kind TEXT NOT NULL,
                   revision INTEGER NOT NULL CHECK (revision > 0),
                   updated_at_millis INTEGER NOT NULL,
                   deleted INTEGER NOT NULL CHECK (deleted IN (0, 1)),
                   display_name TEXT NOT NULL,
                   search_text TEXT NOT NULL,
                   payload_json TEXT NOT NULL,
                   PRIMARY KEY (owner_id, record_id)
                 );
                 CREATE INDEX library_records_owner_kind_name
                   ON library_records(owner_id, kind, display_name COLLATE NOCASE);
                 CREATE TABLE library_changes (
                   sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                   owner_id TEXT NOT NULL,
                   record_id TEXT NOT NULL,
                   revision INTEGER NOT NULL,
                   changed_at_millis INTEGER NOT NULL
                 );
                 PRAGMA user_version = 1;",
            )?;
        } else if current != SCHEMA_VERSION {
            return Err(LibraryError::UnsupportedSchema(current));
        }
        transaction.commit()?;
        Ok(Self { connection })
    }
}

impl LibraryRepository for SqliteLibraryRepository {
    fn changes(
        &self,
        owner_id: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<LibraryChange>, LibraryError> {
        let cursor =
            i64::try_from(after).map_err(|_| LibraryError::InvalidRecord("cursor out of range"))?;
        let mut statement = self.connection.prepare(
            "SELECT sequence, record_id, revision, changed_at_millis FROM library_changes WHERE owner_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3"
        )?;
        let rows = statement.query_map(
            params![
                owner_id,
                cursor,
                i64::try_from(limit.clamp(1, 500)).unwrap_or(500)
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?;
        rows.map(|row| {
            let (sequence, id, revision, changed_at_millis) = row?;
            let sequence = u64::try_from(sequence)
                .map_err(|_| LibraryError::InvalidRecord("negative sequence"))?;
            let revision = u64::try_from(revision)
                .map_err(|_| LibraryError::InvalidRecord("negative revision"))?;
            let record_id = Uuid::parse_str(&id)
                .map_err(|_| LibraryError::InvalidRecord("invalid stored identity"))?;
            Ok(LibraryChange {
                sequence,
                record_id,
                revision,
                changed_at_millis,
            })
        })
        .collect()
    }

    fn put(
        &mut self,
        record: &LibraryRecord,
        expected_revision: Option<u64>,
    ) -> Result<(), LibraryError> {
        record.validate()?;
        let header = record.header();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<i64> = transaction
            .query_row(
                "SELECT revision FROM library_records WHERE owner_id = ?1 AND record_id = ?2",
                params![header.owner_id, header.record_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        let current = current
            .map(u64::try_from)
            .transpose()
            .map_err(|_| LibraryError::InvalidRecord("stored revision is negative"))?;
        match (current, expected_revision) {
            (None, None) if header.revision == 1 => {}
            (Some(actual), Some(expected))
                if actual == expected && Some(header.revision) == expected.checked_add(1) => {}
            (actual, expected) => return Err(LibraryError::RevisionConflict { expected, actual }),
        }
        let previous: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM library_records WHERE owner_id = ?1 AND record_id = ?2",
                params![header.owner_id, header.record_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(json) = previous {
            let previous: LibraryRecord = serde_json::from_str(&json)?;
            if previous.kind() != record.kind() {
                return Err(LibraryError::InvalidRecord("record kind is immutable"));
            }
        }
        validate_location_links(&transaction, record)?;
        let revision = i64::try_from(header.revision)
            .map_err(|_| LibraryError::InvalidRecord("revision exceeds SQLite integer range"))?;
        let payload = serde_json::to_string(record)?;
        transaction.execute(
            "INSERT INTO library_records
             (owner_id, record_id, kind, revision, updated_at_millis, deleted, display_name, search_text, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(owner_id, record_id) DO UPDATE SET
               kind=excluded.kind, revision=excluded.revision,
               updated_at_millis=excluded.updated_at_millis, deleted=excluded.deleted,
               display_name=excluded.display_name, search_text=excluded.search_text,
               payload_json=excluded.payload_json",
            params![header.owner_id, header.record_id.to_string(), record.kind().as_str(),
                revision, header.updated_at_millis, header.deleted, record.display_name(),
                record.search_text(), payload],
        )?;
        transaction.execute(
            "INSERT INTO library_changes(owner_id, record_id, revision, changed_at_millis)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                header.owner_id,
                header.record_id.to_string(),
                revision,
                header.updated_at_millis
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn get(&self, owner_id: &str, record_id: Uuid) -> Result<Option<LibraryRecord>, LibraryError> {
        self.connection
            .query_row(
                "SELECT payload_json FROM library_records WHERE owner_id = ?1 AND record_id = ?2",
                params![owner_id, record_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|json| serde_json::from_str(&json).map_err(LibraryError::from))
            .transpose()
    }

    fn search(&self, query: &LibraryQuery) -> Result<Vec<LibraryRecord>, LibraryError> {
        let kind = query.kind.map(LibraryRecordKind::as_str);
        let pattern = query.text.trim().to_lowercase();
        let limit = i64::try_from(query.limit.clamp(1, 500)).unwrap_or(500);
        let mut statement = self.connection.prepare(
            "SELECT payload_json FROM library_records
             WHERE owner_id = ?1 AND deleted = 0
               AND (?2 IS NULL OR kind = ?2)
               AND (?3 = '' OR instr(search_text, ?3) > 0)
             ORDER BY display_name COLLATE NOCASE, record_id
             LIMIT ?4",
        )?;
        let rows = statement.query_map(params![query.owner_id, kind, pattern, limit], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
}

fn validate_location_links(
    transaction: &rusqlite::Transaction<'_>,
    record: &LibraryRecord,
) -> Result<(), LibraryError> {
    let header = record.header();
    if let LibraryRecord::Location {
        parent_location_id, ..
    } = record
    {
        let mut parent = *parent_location_id;
        let mut visited = std::collections::HashSet::from([header.record_id]);
        while let Some(id) = parent {
            if !visited.insert(id) {
                return Err(LibraryError::InvalidRecord(
                    "location hierarchy contains a cycle",
                ));
            }
            let json: Option<String> = transaction.query_row(
                    "SELECT payload_json FROM library_records WHERE owner_id = ?1 AND record_id = ?2",
                    params![header.owner_id, id.to_string()], |row| row.get(0),
                ).optional()?;
            match json
                .map(|s| serde_json::from_str::<LibraryRecord>(&s))
                .transpose()?
            {
                Some(LibraryRecord::Location {
                    header,
                    parent_location_id,
                    ..
                }) if !header.deleted => parent = parent_location_id,
                _ => {
                    return Err(LibraryError::InvalidRecord(
                        "parent must be a live location in the same library",
                    ));
                }
            }
        }
        if header.deleted {
            let children: i64 = transaction.query_row(
                    "SELECT count(*) FROM library_records WHERE owner_id = ?1 AND kind = 'location' AND deleted = 0 AND json_extract(payload_json, '$.parent_location_id') = ?2",
                    params![header.owner_id, header.record_id.to_string()], |row| row.get(0),
                )?;
            if children > 0 {
                return Err(LibraryError::InvalidRecord(
                    "move or delete sub-locations first",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("invalid Library record: {0}")]
    InvalidRecord(&'static str),
    #[error("Library revision conflict: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("unsupported local Library schema version {0}")]
    UnsupportedSchema(i64),
    #[error("local Library I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("local Library database failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Library record serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: Uuid, revision: u64, name: &str) -> LibraryRecord {
        LibraryRecord::Person {
            header: LibraryRecordHeader {
                owner_id: "owner-1".into(),
                record_id: id,
                revision,
                updated_at_millis: i64::try_from(revision).unwrap(),
                deleted: false,
                thumbnail_digest: None,
            },
            display_name: name.into(),
            aliases: vec!["Suhail".into()],
            roles: vec!["model".into()],
        }
    }

    #[test]
    fn sqlite_creates_searches_and_revision_checks_records() {
        let mut repository = SqliteLibraryRepository::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        repository
            .put(&person(id, 1, "Suhail Model"), None)
            .unwrap();
        let results = repository
            .search(&LibraryQuery {
                owner_id: "owner-1".into(),
                kind: Some(LibraryRecordKind::Person),
                text: "model".into(),
                limit: 20,
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            repository.get("owner-1", id).unwrap(),
            Some(person(id, 1, "Suhail Model"))
        );
        assert!(matches!(
            repository.put(&person(id, 2, "Renamed"), Some(0)),
            Err(LibraryError::RevisionConflict { .. })
        ));
        repository.put(&person(id, 2, "Renamed"), Some(1)).unwrap();
        assert_eq!(
            repository
                .get("owner-1", id)
                .unwrap()
                .unwrap()
                .display_name(),
            "Renamed"
        );
    }

    #[test]
    fn every_mode_retains_an_offline_store() {
        assert!(LibraryMode::OnThisMac.uses_local_sqlite());
        assert!(LibraryMode::PhotaraCloud.uses_local_sqlite());
        assert!(LibraryMode::ICloud.uses_local_sqlite());
    }
    #[test]
    fn durable_tombstones_changes_and_owner_isolation() {
        let root = std::env::temp_dir().join(format!("photara-library-test-{}", Uuid::new_v4()));
        let path = root.join("nested/library.sqlite");
        let id = Uuid::new_v4();
        {
            let mut repo = SqliteLibraryRepository::open(&path).unwrap();
            repo.put(&person(id, 1, "Original"), None).unwrap();
            assert!(repo.get("another-owner", id).unwrap().is_none());
            assert!(repo.changes("another-owner", 0, 50).unwrap().is_empty());
            let mut deleted = person(id, 2, "Original");
            if let LibraryRecord::Person { header, .. } = &mut deleted {
                header.deleted = true;
            }
            repo.put(&deleted, Some(1)).unwrap();
            assert!(repo.put(&person(id, 2, "Stale"), Some(1)).is_err());
        }
        let repo = SqliteLibraryRepository::open(&path).unwrap();
        assert!(repo.get("owner-1", id).unwrap().unwrap().header().deleted);
        let changes = repo.changes("owner-1", 0, 1).unwrap();
        assert_eq!(changes[0].revision, 1);
        let next = repo.changes("owner-1", changes[0].sequence, 50).unwrap();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].revision, 2);
        assert!(
            repo.search(&LibraryQuery {
                owner_id: "owner-1".into(),
                kind: None,
                text: String::new(),
                limit: 50
            })
            .unwrap()
            .is_empty()
        );
        drop(repo);
        fs::remove_dir_all(root).unwrap();
    }

    fn location(id: Uuid, revision: u64, parent: Option<Uuid>) -> LibraryRecord {
        LibraryRecord::Location {
            header: person(id, revision, "Location").header().clone(),
            display_name: "Location".into(),
            parent_location_id: parent,
            address: None,
            aliases: vec![],
        }
    }
    #[test]
    fn location_hierarchy_is_owner_scoped_acyclic_and_transactional() {
        let mut repo = SqliteLibraryRepository::open_in_memory().unwrap();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        repo.put(&location(parent, 1, None), None).unwrap();
        repo.put(&location(child, 1, Some(parent)), None).unwrap();
        assert!(
            repo.put(&location(parent, 2, Some(child)), Some(1))
                .is_err()
        );
        assert!(
            repo.put(&location(Uuid::new_v4(), 1, Some(Uuid::new_v4())), None)
                .is_err()
        );
        let mut deleted = location(parent, 2, None);
        if let LibraryRecord::Location { header, .. } = &mut deleted {
            header.deleted = true;
        }
        assert!(repo.put(&deleted, Some(1)).is_err());
        assert_eq!(repo.changes("owner-1", 0, 50).unwrap().len(), 2);
        assert_eq!(
            repo.get("owner-1", parent)
                .unwrap()
                .unwrap()
                .header()
                .revision,
            1
        );
    }

    #[test]
    fn unknown_schema_and_kind_changes_are_rejected() {
        let mut repo = SqliteLibraryRepository::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        repo.put(&person(id, 1, "Person"), None).unwrap();
        assert!(repo.put(&location(id, 2, None), Some(1)).is_err());
        let connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 999).unwrap();
        assert!(matches!(
            SqliteLibraryRepository::configure(connection),
            Err(LibraryError::UnsupportedSchema(999))
        ));
    }
}
