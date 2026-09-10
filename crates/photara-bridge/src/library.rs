//! Host-owned Library facade. Native views receive immutable values, never SQL.
use crate::BridgeError;
use photara_library::{
    LibraryQuery, LibraryRecord, LibraryRecordHeader, LibraryRepository, SqliteLibraryRepository,
};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum BridgeLibraryKind {
    Person,
    Client,
    Location,
    Scene,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeLibraryRecordDto {
    pub record_id: String,
    pub owner_id: String,
    pub revision: u64,
    pub kind: BridgeLibraryKind,
    pub display_name: String,
    pub aliases: Vec<String>,
    /// People roles, client entity category, or scene tags according to kind.
    pub labels: Vec<String>,
    pub parent_id: Option<String>,
    pub detail: String,
    pub deleted: bool,
    pub thumbnail_digest: Option<String>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeLibraryEdit {
    pub record_id: Option<String>,
    pub expected_revision: Option<u64>,
    pub kind: BridgeLibraryKind,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub labels: Vec<String>,
    pub parent_id: Option<String>,
    pub detail: String,
    pub deleted: bool,
    pub thumbnail_digest: Option<String>,
}

/// One local owner scope. The connection is created on first read/write.
#[derive(uniffi::Object)]
pub struct PhotaraLibrary {
    path: PathBuf,
    owner_id: String,
    repository: Mutex<Option<SqliteLibraryRepository>>,
}

pub(crate) fn library_error(error: impl std::fmt::Display) -> BridgeError {
    BridgeError::State {
        message: error.to_string(),
    }
}
pub(crate) fn library_uuid(id: &str) -> Result<Uuid, BridgeError> {
    Uuid::parse_str(id).map_err(library_error)
}

impl PhotaraLibrary {
    fn with_repository<T>(
        &self,
        action: impl FnOnce(&mut SqliteLibraryRepository) -> Result<T, BridgeError>,
    ) -> Result<T, BridgeError> {
        let mut guard = self.repository.lock().map_err(library_error)?;
        if guard.is_none() {
            *guard = Some(SqliteLibraryRepository::open(&self.path).map_err(library_error)?);
        }
        action(
            guard
                .as_mut()
                .ok_or_else(|| library_error("Library unavailable"))?,
        )
    }
    pub(crate) fn record(&self, id: &str) -> Result<LibraryRecord, BridgeError> {
        self.with_repository(|repo| {
            repo.get(&self.owner_id, library_uuid(id)?)
                .map_err(library_error)?
                .filter(|r| !r.header().deleted)
                .ok_or_else(|| library_error("Library record unavailable"))
        })
    }
}

#[uniffi::export]
impl PhotaraLibrary {
    #[uniffi::constructor]
    /// Creates a lazy owner-scoped local Library handle.
    ///
    /// # Errors
    /// Returns an error when the owner scope is empty.
    pub fn local(path: String, owner_id: String) -> Result<Arc<Self>, BridgeError> {
        if owner_id.trim().is_empty() {
            return Err(library_error("Library workspace is required"));
        }
        Ok(Arc::new(Self {
            path: PathBuf::from(path),
            owner_id,
            repository: Mutex::new(None),
        }))
    }

    /// Imports a bounded host-prepared PNG. Only its digest enters Library records.
    /// # Errors
    /// Returns an I/O error or rejects oversized/non-PNG input.
    pub fn import_thumbnail(&self, source_path: String) -> Result<String, BridgeError> {
        let file = std::fs::File::open(source_path).map_err(library_error)?;
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(library_error)?;
        if bytes.len() > 4 * 1024 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(library_error("Thumbnail must be a PNG smaller than 4 MiB"));
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let root = self
            .path
            .parent()
            .ok_or_else(|| library_error("Library path has no parent"))?
            .join("media");
        std::fs::create_dir_all(&root).map_err(library_error)?;
        let target = root.join(format!("{digest}.png"));
        if !target.exists() {
            let temporary = root.join(format!("{}.tmp", Uuid::new_v4()));
            std::fs::write(&temporary, bytes).map_err(library_error)?;
            std::fs::rename(&temporary, &target).map_err(library_error)?;
        }
        Ok(digest)
    }

    /// Resolves a device-local thumbnail path without putting it in a portable record.
    /// # Errors
    /// Returns an error for an invalid digest or unavailable local media.
    #[allow(clippy::needless_pass_by_value)] // UniFFI exports owned arguments.
    pub fn thumbnail_path(&self, digest: String) -> Result<String, BridgeError> {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(library_error("Invalid thumbnail identity"));
        }
        let path = self
            .path
            .parent()
            .ok_or_else(|| library_error("Library path has no parent"))?
            .join("media")
            .join(format!("{digest}.png"));
        if !path.is_file() {
            return Err(library_error("Thumbnail is unavailable on this Mac"));
        }
        Ok(path.to_string_lossy().into_owned())
    }

    /// Searches up to 500 live records within this owner scope.
    ///
    /// # Errors
    /// Returns a local database, schema or record decoding error.
    pub fn search(&self, text: String) -> Result<Vec<BridgeLibraryRecordDto>, BridgeError> {
        self.with_repository(|repo| {
            repo.search(&LibraryQuery {
                owner_id: self.owner_id.clone(),
                kind: None,
                text,
                limit: 500,
            })
            .map(|records| records.iter().map(BridgeLibraryRecordDto::from).collect())
            .map_err(library_error)
        })
    }

    /// Creates or replaces a Library record at an observed revision.
    ///
    /// # Errors
    /// Returns validation, hierarchy, revision conflict or storage errors.
    pub fn edit(&self, edit: BridgeLibraryEdit) -> Result<BridgeLibraryRecordDto, BridgeError> {
        let id = edit
            .record_id
            .as_deref()
            .map(library_uuid)
            .transpose()?
            .unwrap_or_else(Uuid::new_v4);
        if edit.record_id.is_some() != edit.expected_revision.is_some()
            || (edit.deleted && edit.expected_revision.is_none())
        {
            return Err(library_error(
                "Existing records require their observed revision",
            ));
        }
        let revision = edit
            .expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| library_error("Revision exhausted"))?;
        let updated_at_millis = i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(library_error)?
                .as_millis(),
        )
        .map_err(library_error)?;
        let header = LibraryRecordHeader {
            owner_id: self.owner_id.clone(),
            record_id: id,
            revision,
            updated_at_millis,
            deleted: edit.deleted,
            thumbnail_digest: edit.thumbnail_digest,
        };
        let display_name = edit.display_name.trim().to_owned();
        let record = match edit.kind {
            BridgeLibraryKind::Person => LibraryRecord::Person {
                header,
                display_name,
                aliases: edit.aliases,
                roles: edit.labels,
            },
            BridgeLibraryKind::Client => LibraryRecord::Client {
                header,
                display_name,
                aliases: edit.aliases,
                organization: !edit.labels.iter().any(|v| v == "individual"),
            },
            BridgeLibraryKind::Location => LibraryRecord::Location {
                header,
                display_name,
                aliases: edit.aliases,
                parent_location_id: edit.parent_id.as_deref().map(library_uuid).transpose()?,
                address: Some(edit.detail),
            },
            BridgeLibraryKind::Scene => LibraryRecord::Scene {
                header,
                display_name,
                aliases: edit.aliases,
                tags: edit.labels,
                description: edit.detail,
            },
        };
        self.with_repository(|repo| {
            repo.put(&record, edit.expected_revision)
                .map_err(library_error)?;
            Ok(BridgeLibraryRecordDto::from(&record))
        })
    }
}

impl From<&LibraryRecord> for BridgeLibraryRecordDto {
    fn from(record: &LibraryRecord) -> Self {
        let h = record.header();
        let (kind, aliases, labels, parent_id, detail) = match record {
            LibraryRecord::Person { aliases, roles, .. } => (
                BridgeLibraryKind::Person,
                aliases.clone(),
                roles.clone(),
                None,
                String::new(),
            ),
            LibraryRecord::Client {
                aliases,
                organization,
                ..
            } => (
                BridgeLibraryKind::Client,
                aliases.clone(),
                vec![
                    if *organization {
                        "organization"
                    } else {
                        "individual"
                    }
                    .into(),
                ],
                None,
                String::new(),
            ),
            LibraryRecord::Location {
                aliases,
                parent_location_id,
                address,
                ..
            } => (
                BridgeLibraryKind::Location,
                aliases.clone(),
                vec![],
                parent_location_id.map(|v| v.to_string()),
                address.clone().unwrap_or_default(),
            ),
            LibraryRecord::Scene {
                aliases,
                tags,
                description,
                ..
            } => (
                BridgeLibraryKind::Scene,
                aliases.clone(),
                tags.clone(),
                None,
                description.clone(),
            ),
        };
        Self {
            record_id: h.record_id.to_string(),
            owner_id: h.owner_id.clone(),
            revision: h.revision,
            kind,
            display_name: record.display_name().into(),
            aliases,
            labels,
            parent_id,
            detail,
            deleted: h.deleted,
            thumbnail_digest: h.thumbnail_digest.clone(),
        }
    }
}
