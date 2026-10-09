//! Default-off, freshly registered disposable `LL2a` authority. Never upgrades or
//! adopts an existing database. SQL validation is not a storage qualification.
use photara_store::package::v1_3::activation::v2::{self, SelectionStore, Snapshot};
use photara_store::package::{JsonLimits, parse_canonical_json};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
mod lifecycle;
mod slots;
#[cfg(test)]
mod tests;
pub type Result<T> = io::Result<T>;
fn err(message: &str) -> io::Error {
    io::Error::other(message.to_owned())
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err consumes the SQL error into an IO boundary"
)]
fn sql(error: rusqlite::Error) -> io::Error {
    err(&format!("registered local SQL: {error}"))
}
fn ensure(value: bool) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(err("local authority, identity or revision refused"))
    }
}
fn canonical(value: &Value) -> Result<Vec<u8>> {
    photara_core::canonical_json(value).map_err(|_| err("canonical value refused"))
}
fn sha(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|b| {
            [
                char::from(b"0123456789abcdef"[usize::from(b >> 4)]),
                char::from(b"0123456789abcdef"[usize::from(b & 0x0f)]),
            ]
        })
        .collect()
}
fn uuid(value: &Value) -> Result<Uuid> {
    let text = value.as_str().ok_or_else(|| err("UUID expected"))?;
    let id = Uuid::parse_str(text).map_err(|_| err("UUID refused"))?;
    ensure(!id.is_nil() && id.to_string() == text)?;
    Ok(id)
}
fn decimal(value: &Value) -> Result<i64> {
    let text = value.as_str().ok_or_else(|| err("decimal expected"))?;
    let n = text.parse::<i64>().map_err(|_| err("decimal overflow"))?;
    ensure(n >= 0 && n.to_string() == text)?;
    Ok(n)
}
fn digest(value: &str) -> Result<Vec<u8>> {
    ensure(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )?;
    (0..32)
        .map(|i| {
            u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| err("digest refused"))
        })
        .collect()
}
fn fields(value: &Value, names: &[&str]) -> Result<()> {
    let o = value.as_object().ok_or_else(|| err("object expected"))?;
    ensure(o.len() == names.len() && names.iter().all(|n| o.contains_key(*n)))
}
fn now() -> Result<i64> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| err("clock before epoch"))?
            .as_millis(),
    )
    .map_err(|_| err("clock overflow"))
}
/// Independently registered disposable fixture bounds, never product defaults.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub database_id: Uuid,
    pub epoch: Uuid,
    pub device_id: Uuid,
    pub workspace_slot_id: Uuid,
    pub principal_id: Uuid,
    pub slot_scope_sha256: String,
    pub max_snapshot_bytes: usize,
    pub max_records: usize,
    pub max_database_bytes: u64,
    pub coexistence_bytes: u64,
    pub namespace_bytes: u64,
    pub allow_faults: bool,
}
impl Registration {
    fn validate(&self) -> Result<()> {
        for id in [
            self.database_id,
            self.epoch,
            self.device_id,
            self.workspace_slot_id,
            self.principal_id,
        ] {
            ensure(!id.is_nil())?;
        }
        digest(&self.slot_scope_sha256)?;
        ensure(
            self.max_snapshot_bytes > 0
                && self.max_records > 0
                && self.max_database_bytes >= 4096
                && self.max_database_bytes.is_multiple_of(4096)
                && i64::try_from(self.max_database_bytes).is_ok()
                && self.coexistence_bytes
                    >= self
                        .max_database_bytes
                        .checked_mul(2)
                        .and_then(|v| v.checked_add(self.namespace_bytes))
                        .ok_or_else(|| err("budget overflow"))?,
        )
    }
    fn limits(&self) -> v2::Limits {
        v2::Limits {
            json: JsonLimits {
                max_bytes: self.max_snapshot_bytes,
                max_depth: 128,
                max_members: 4096,
                max_array_elements: self.max_records.max(4096),
            },
            max_snapshot_bytes: self.max_snapshot_bytes,
            max_records: self.max_records,
        }
    }
    fn authority(&self) -> Value {
        json!({"kind":"local","database_id":self.database_id,"epoch":self.epoch})
    }
    fn principal(&self) -> Value {
        json!({"kind":"local","id":self.principal_id})
    }
    fn scope(&self) -> Value {
        json!({"device_id":self.device_id,"workspace_slot_id":self.workspace_slot_id,"database_id":self.database_id,"epoch":self.epoch,"principal_id":self.principal_id,"slot_scope_sha256":self.slot_scope_sha256,"max_snapshot_bytes":self.max_snapshot_bytes.to_string(),"max_records":self.max_records.to_string()})
    }
}
/// Exact directory/database/cooperative-lock observation retained by registrar.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub directory: [u64; 2],
    pub database: [u64; 2],
    pub lock: [u64; 2],
}
fn pin(file: &File) -> Result<[u64; 2]> {
    let m = file.metadata()?;
    Ok([m.dev(), m.ino()])
}
fn file(path: &Path, create: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed());
    if create {
        options.create_new(true).mode(0o600);
    }
    let f = options.open(path)?;
    let m = f.metadata()?;
    ensure(m.is_file() && m.nlink() == 1 && m.permissions().mode().trailing_zeros() >= 6)?;
    Ok(f)
}
/// Existing bootstrap domains; only initial bootstrap derives the principal.
/// Later create checks the stored current controller, including transfers.
#[must_use]
pub fn bootstrap_ids(database: Uuid) -> (Uuid, Uuid) {
    let derive = |domain: &[u8]| {
        let mut h = Sha256::new();
        h.update(domain);
        h.update(database.as_bytes());
        let bytes = h.finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&bytes[..16]);
        Uuid::from_bytes(id)
    };
    (
        derive(b"photara.default-library.v2"),
        derive(b"photara.local-principal.v2"),
    )
}
/// A single held connection and separately pinned scope. No Saved constructor.
pub struct LocalDatabase {
    connection: Connection,
    path: PathBuf,
    directory: File,
    database: File,
    lock: File,
    pins: Pins,
    registration: Registration,
    fault: Option<String>,
}
impl LocalDatabase {
    /// Creates only an absent disposable database and provisions its exact empty
    /// selection once. Caller registers returned inode observations independently.
    /// # Errors
    /// Refuses existing files, unsafe parents, unsupported pragmas or invalid scope.
    pub fn create_fresh(path: &Path, registration: Registration, at: i64) -> Result<Self> {
        registration.validate()?;
        ensure(at >= 0)?;
        let (library, principal) = bootstrap_ids(registration.database_id);
        ensure(registration.principal_id == principal)?;
        let mut this = Self::connect(path, registration, None)?;
        let tx = this
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql)?;
        tx.execute_batch(include_str!(
            "../../migrations/generation_two/0001_local_identity.sql"
        ))
        .map_err(sql)?;
        tx.execute("INSERT INTO schema_metadata VALUES(1,'photara.local.g2',?,1,1,1,'photara.canonical-json.v1',?)",params![this.registration.database_id.as_bytes(),at]).map_err(sql)?;
        tx.execute(
            "INSERT INTO local_device VALUES(1,?,'Registered disposable device',?)",
            params![this.registration.device_id.as_bytes(), at],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO normalization_policies VALUES(1,'16.0.0',?,?)",
            params![sha(crate::gen2::POLICY.as_bytes()), crate::gen2::POLICY],
        )
        .map_err(sql)?;
        for ddl in DDL {
            tx.execute_batch(ddl).map_err(sql)?;
        }
        tx.execute_batch(include_str!("schema.sql")).map_err(sql)?;
        tx.execute(
            "INSERT INTO libraries VALUES(?,'My Library','active',1,?,?,NULL,1,'{}')",
            params![library.as_bytes(), at, at],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO library_contract_state VALUES(?,1,'local-only',?,1,1,1,?,?)",
            params![library.as_bytes(), principal.as_bytes(), at, at],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO lifecycle_authorities VALUES(?,'local',?,?,NULL)",
            params![
                this.registration.epoch.as_bytes(),
                this.registration.epoch.as_bytes(),
                this.registration.database_id.as_bytes()
            ],
        )
        .map_err(sql)?;
        let empty=Snapshot::from_body(json!({"device_id":this.registration.device_id,"workspace_slot_id":this.registration.workspace_slot_id,"revision":"0","request_generation":"0","committed_generation":"0","slot_scope_sha256":this.registration.slot_scope_sha256,"active":null,"pending":null,"records":[]}),this.registration.limits()).map_err(|e|err(&e.to_string()))?;
        tx.execute(
            "INSERT INTO local_activation_slots VALUES(?,?,?,0,0,0,?,?,NULL,NULL)",
            params![
                this.registration.device_id.as_bytes(),
                this.registration.workspace_slot_id.as_bytes(),
                digest(&this.registration.slot_scope_sha256)?,
                empty.bytes(),
                sha(empty.bytes())
            ],
        )
        .map_err(sql)?;
        tx.commit().map_err(sql)?;
        this.database.sync_all()?;
        this.directory.sync_all()?;
        this.verify()?;
        Ok(this)
    }
    /// Opens only the exact registered fresh floor6 database; never initializes.
    /// # Errors
    /// Refuses missing/replaced files, wrong floor/identity/slot or settings.
    pub fn reopen(path: &Path, registration: Registration, pins: Pins) -> Result<Self> {
        let this = Self::connect(path, registration, Some(pins))?;
        this.verify()?;
        Ok(this)
    }
    fn connect(path: &Path, registration: Registration, expected: Option<Pins>) -> Result<Self> {
        registration.validate()?;
        ensure(path.is_absolute())?;
        for ancestor in path
            .parent()
            .ok_or_else(|| err("parent missing"))?
            .ancestors()
        {
            ensure(
                !std::fs::symlink_metadata(ancestor)?
                    .file_type()
                    .is_symlink(),
            )?;
        }
        let parent = path.parent().ok_or_else(|| err("parent missing"))?;
        let directory = File::open(parent)?;
        ensure(directory.metadata()?.permissions().mode().trailing_zeros() >= 6)?;
        let database = file(path, expected.is_none())?;
        let lockpath = parent.join(".ll2a-writer-lock");
        let lock = file(&lockpath, expected.is_none())?;
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
        let pins = Pins {
            directory: pin(&directory)?,
            database: pin(&database)?,
            lock: pin(&lock)?,
        };
        if let Some(expected) = expected {
            ensure(pins == expected)?;
        }
        let journal = path.with_file_name(format!(
            "{}-journal",
            path.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| err("database name"))?
        ));
        if std::fs::symlink_metadata(&journal).is_ok() {
            file(&journal, false)?;
        }
        let connection = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(sql)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(2))
            .map_err(sql)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA recursive_triggers=ON; PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA fullfsync=ON; PRAGMA checkpoint_fullfsync=ON;").map_err(sql)?;
        connection
            .pragma_update(
                None,
                "max_page_count",
                i64::try_from(registration.max_database_bytes / 4096)
                    .map_err(|_| err("page budget overflow"))?,
            )
            .map_err(sql)?;
        let this = Self {
            connection,
            path: path.to_owned(),
            directory,
            database,
            lock,
            pins,
            registration,
            fault: None,
        };
        this.check_pins()?;
        this.settings()?;
        this.capacity()?;
        Ok(this)
    }
    #[must_use]
    pub fn pins(&self) -> Pins {
        self.pins.clone()
    }
    #[must_use]
    pub fn registration(&self) -> &Registration {
        &self.registration
    }
    /// Captured engine settings; observations do not certify storage hardware.
    /// # Errors
    /// Refuses any connection setting mismatch.
    pub fn settings(&self) -> Result<Value> {
        let mode: String = self
            .connection
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .map_err(sql)?;
        ensure(mode == "delete")?;
        for (name, want) in [
            ("foreign_keys", 1),
            ("recursive_triggers", 1),
            ("synchronous", 2),
            ("fullfsync", 1),
            ("checkpoint_fullfsync", 1),
            ("page_size", 4096),
        ] {
            let actual: i64 = self
                .connection
                .pragma_query_value(None, name, |r| r.get(0))
                .map_err(sql)?;
            ensure(actual == want)?;
        }
        let maximum: i64 = self
            .connection
            .pragma_query_value(None, "max_page_count", |r| r.get(0))
            .map_err(sql)?;
        ensure(u64::try_from(maximum).ok() == Some(self.registration.max_database_bytes / 4096))?;
        Ok(
            json!({"journal_mode":mode,"synchronous":2,"fullfsync":1,"foreign_keys":1,"page_size":4096,"max_page_count":maximum}),
        )
    }
    fn check_pins(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let facts = rustix::fs::fstatfs(&self.database)?;
            let filesystem = facts
                .f_fstypename
                .iter()
                .take_while(|c| **c != 0)
                .map(|c| u8::try_from(*c).map_err(|_| err("filesystem encoding")))
                .collect::<Result<Vec<_>>>()?;
            ensure(filesystem == b"apfs" && facts.f_flags & 0x1000 != 0 && facts.f_flags & 1 == 0)?;
        }

        ensure(
            pin(&self.database)? == self.pins.database
                && pin(&self.directory)? == self.pins.directory
                && pin(&self.lock)? == self.pins.lock
                && pin(&file(&self.path, false)?)? == self.pins.database
                && pin(&File::open(
                    self.path.parent().ok_or_else(|| err("parent missing"))?,
                )?)? == self.pins.directory
                && pin(&file(
                    &self
                        .path
                        .parent()
                        .ok_or_else(|| err("parent missing"))?
                        .join(".ll2a-writer-lock"),
                    false,
                )?)? == self.pins.lock,
        )
    }
    pub(super) fn barriers(&self) -> Result<()> {
        self.database.sync_all()?;
        #[cfg(target_os = "macos")]
        rustix::fs::fcntl_fullfsync(&self.database)?;
        self.directory.sync_all()?;
        #[cfg(target_os = "macos")]
        rustix::fs::fcntl_fullfsync(&self.directory)?;
        Ok(())
    }
    fn capacity(&self) -> Result<()> {
        ensure(self.database.metadata()?.len() <= self.registration.max_database_bytes)?;
        let directory = self.directory.metadata()?;
        ensure(
            directory.len().max(
                directory
                    .blocks()
                    .checked_mul(512)
                    .ok_or_else(|| err("directory allocation overflow"))?,
            ) <= self.registration.namespace_bytes,
        )?;
        let parent = self.path.parent().ok_or_else(|| err("parent missing"))?;
        let name = self
            .path
            .file_name()
            .ok_or_else(|| err("name missing"))?
            .to_str()
            .ok_or_else(|| err("name encoding"))?;
        let mut bytes = self.registration.namespace_bytes;
        for row in std::fs::read_dir(parent)? {
            let row = row?;
            let n = row.file_name();
            let n = n.to_str().ok_or_else(|| err("unexpected file"))?;
            ensure(n == name || n == format!("{name}-journal") || n == ".ll2a-writer-lock")?;
            let m = std::fs::symlink_metadata(row.path())?;
            ensure(m.is_file() && m.nlink() == 1)?;
            bytes = bytes
                .checked_add(
                    m.len().max(
                        m.blocks()
                            .checked_mul(512)
                            .ok_or_else(|| err("allocation overflow"))?,
                    ),
                )
                .ok_or_else(|| err("budget overflow"))?;
        }
        ensure(bytes <= self.registration.coexistence_bytes)
    }
    fn verify(&self) -> Result<()> {
        self.check_pins()?;
        self.settings()?;
        self.capacity()?;
        let meta:(String,Vec<u8>,i64,i64,i64)=self.connection.query_row("SELECT schema_family,database_id,schema_epoch,minimum_reader,minimum_writer FROM schema_metadata WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(sql)?;
        ensure(
            meta.0 == "photara.local.g2"
                && meta.1 == self.registration.database_id.as_bytes()
                && meta.2 == 1
                && meta.3 == 6
                && meta.4 == 6,
        )?;
        let dev: Vec<u8> = self
            .connection
            .query_row(
                "SELECT device_id FROM local_device WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(sql)?;
        ensure(dev == self.registration.device_id.as_bytes())?;
        let authority:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM lifecycle_authorities WHERE authority_id=? AND kind='local' AND epoch=? AND database_id=?)",params![self.registration.epoch.as_bytes(),self.registration.epoch.as_bytes(),self.registration.database_id.as_bytes()],|r|r.get(0)).map_err(sql)?;
        ensure(authority)?;
        let integrity: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(sql)?;
        ensure(integrity == "ok")?;
        ensure(
            !self
                .connection
                .prepare("PRAGMA foreign_key_check")
                .map_err(sql)?
                .exists([])
                .map_err(sql)?,
        )?;
        self.load_checked()?;
        Ok(())
    }
    /// Faults are available only when independently enabled in fixture registration.
    /// # Errors
    /// Refuses unregistered or unknown fault injection.
    pub fn inject_fault(&mut self, point: &str) -> Result<()> {
        ensure(
            self.registration.allow_faults
                && matches!(
                    point,
                    "before-commit"
                        | "after-commit"
                        | "crash:before-commit"
                        | "crash:after-commit"
                        | "before-activation-commit"
                        | "after-activation-commit"
                        | "crash:before-activation-commit"
                        | "crash:after-activation-commit"
                ),
        )?;
        self.fault = Some(point.to_owned());
        Ok(())
    }
    fn trip(fault: &mut Option<String>, point: &str) -> Result<()> {
        if fault.as_deref() == Some(&format!("crash:{point}")) {
            std::process::exit(86);
        }
        if fault.as_deref() == Some(point) {
            *fault = None;
            return Err(err("registered unknown SQL outcome"));
        }
        Ok(())
    }
}
const DDL: &[&str] = &[
    include_str!("../../migrations/generation_two/0002_typed_library.sql"),
    include_str!("../../migrations/generation_two/0003_catalog_device.sql"),
    include_str!("../../migrations/generation_two/0004_mutations_sync.sql"),
    include_str!("../../migrations/generation_two/0005_durable_recovery.sql"),
    include_str!("../../migrations/generation_two/0006_invariant_guards.sql"),
    include_str!("../../migrations/generation_two/0007_library_project_access.sql"),
    include_str!("../../migrations/generation_two/0008_storage_locations_bindings.sql"),
    include_str!("../../migrations/generation_two/0009_library_context.sql"),
    include_str!("../../migrations/generation_two/0010_context_apply_recovery.sql"),
    include_str!("../../migrations/generation_two/0011_scoped_sync.sql"),
    include_str!("../../migrations/generation_two/0012_d19_guards_and_floor.sql"),
    include_str!("../../migrations/generation_two/0013_onboarding.sql"),
    include_str!("../../migrations/generation_two/0014_onboarding_dispositions.sql"),
    include_str!("../../migrations/generation_two/0015_project_creation.sql"),
];
pub use slots::ProjectRegistration;
