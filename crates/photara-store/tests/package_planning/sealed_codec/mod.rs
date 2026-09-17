//! Experimental disposable codec: identifiers/layout are deliberately test-only.
//! Real files, hashes, fsync and atomic HEAD rename; no power-loss qualification.
//! No production API imports this reader or writer.
use super::*;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, io::Write as _, path::Path};

mod codec;
mod recovery;
mod tests;

const FEATURE: &str = "example.fixture.experimental-sealed-roots";
const DISCRIMINATOR: &str = "example.fixture.root-set";
const LARGE: u64 = 8_000_000_000;

#[derive(Debug, PartialEq, Eq)]
enum Error {
    Integrity,
    Unsupported,
    Conflict,
    Unknown,
    Io,
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Ref {
    digest: String,
    length: u64,
}
impl Ref {
    fn path(&self) -> Result<String> {
        if self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::Integrity);
        }
        Ok(format!("fixture-objects/{}.json", self.digest))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Object {
    Graph {
        revision: u64,
        snapshot: Ref,
        opaque: Value,
    },
    History {
        retained: Vec<Ref>,
    },
    Snapshot {
        authored: String,
    },
    Resources {
        records: Vec<Ref>,
    },
    Resource {
        identity: String,
        version: Ref,
    },
    Version {
        resource: String,
        identity: String,
        backing: Ref,
        provenance: Ref,
        retention: Ref,
    },
    Backing {
        version: String,
        byte_length: u64,
        storage_location: String,
        opaque_locator: String,
        receipt: String,
    },
    Provenance {
        captured_digest: String,
        producer: String,
    },
    Retention {
        version: String,
        obligations: Vec<String>,
    },
    Operations {
        accepted: Vec<Accepted>,
    },
}
impl Object {
    fn dependencies(&self) -> Vec<Ref> {
        match self {
            Self::Graph { snapshot, .. } => vec![snapshot.clone()],
            Self::History { retained } => retained.clone(),
            Self::Resources { records } => records.clone(),
            Self::Resource { version, .. } => vec![version.clone()],
            Self::Version {
                backing,
                provenance,
                retention,
                ..
            } => vec![backing.clone(), provenance.clone(), retention.clone()],
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Accepted {
    operation: String,
    intent: String,
    resulting_graph: Ref,
    provenance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    project: String,
    library: String,
    bootstrap: String,
    revision: u64,
    authored: Ref,
    history: Ref,
    resources: Ref,
    operations: Ref,
    inventory: BTreeSet<Ref>,
    // Evidence only; not an ancestry edge or an indefinite media pin.
    predecessor: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootSet {
    discriminator: String,
    required_reader: String,
    active: Ref,
    recovery: Ref,
    pinned: Vec<Ref>,
    inventory: BTreeSet<Ref>,
    conversion: BTreeMap<String, Ref>,
}

struct Disk {
    temp: tempfile::TempDir,
    original: BTreeMap<String, Vec<u8>>,
    external_opens: Cell<u64>,
    external_bytes: Cell<u64>,
    package_reads: Cell<u64>,
}
impl Disk {
    fn new() -> Self {
        let mut original = build(add_history);
        original.insert(
            "unknown-extension/opaque.bin".into(),
            b"opaque-original-bytes".to_vec(),
        );
        let result = Self {
            temp: tempfile::tempdir().unwrap(),
            original,
            external_opens: Cell::new(0),
            external_bytes: Cell::new(0),
            package_reads: Cell::new(0),
        };
        for (path, bytes) in &result.original {
            result.write(path, bytes).unwrap();
        }
        // Unknown optional source bytes must survive conversion too.
        result
    }
    fn path(&self) -> &Path {
        self.temp.path()
    }
    fn write(&self, path: &str, bytes: &[u8]) -> Result<()> {
        let target = self.path().join(path);
        fs::create_dir_all(target.parent().ok_or(Error::Io)?).map_err(|_| Error::Io)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| Error::Io)?;
        file.write_all(bytes).map_err(|_| Error::Io)?;
        file.sync_all().map_err(|_| Error::Io)?;
        fs::File::open(target.parent().ok_or(Error::Io)?)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Io)
    }
    fn immutable(&self, path: &str, bytes: &[u8]) -> Result<()> {
        if self.path().join(path).exists() {
            if self.read(path)? != bytes {
                return Err(Error::Conflict);
            }
            return Ok(());
        }
        self.write(path, bytes)
    }
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        // Fixed codec paths or validated digest paths only; external locators
        // never reach this package I/O boundary.
        if path.starts_with('/') || path.split('/').any(|s| s == "..") {
            return Err(Error::Integrity);
        }
        let target = self.path().join(path);
        let meta = fs::symlink_metadata(&target).map_err(|_| Error::Io)?;
        if !meta.is_file() || meta.len() > 1_048_576 {
            return Err(Error::Integrity);
        }
        self.package_reads.set(self.package_reads.get() + 1);
        fs::read(target).map_err(|_| Error::Io)
    }
    fn put<T: Serialize>(&self, object: &T) -> Result<Ref> {
        let bytes = canonical(object)?;
        let reference = reference(&bytes);
        self.immutable(&reference.path()?, &bytes)?;
        Ok(reference)
    }
    fn get<T: for<'a> Deserialize<'a>>(&self, reference: &Ref) -> Result<T> {
        let bytes = self.read(&reference.path()?)?;
        if self::reference(&bytes) != *reference {
            return Err(Error::Integrity);
        }
        parse(&bytes)
    }
    fn replace_head(&self, bytes: &[u8]) -> Result<()> {
        self.write("fixture-next-head", bytes)?;
        fs::rename(
            self.path().join("fixture-next-head"),
            self.path().join("HEAD.json"),
        )
        .map_err(|_| Error::Io)
    }
    fn barrier(&self) -> Result<()> {
        fs::File::open(self.path())
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Io)
    }
    fn external(&self, explicit_capture_or_verify: bool) -> Result<()> {
        self.external_opens.set(self.external_opens.get() + 1);
        if !explicit_capture_or_verify {
            return Err(Error::Unsupported);
        }
        // Instrumented provider: logical bytes only, never allocates 8 GB.
        self.external_bytes.set(self.external_bytes.get() + LARGE);
        Ok(())
    }
}
fn reference(bytes: &[u8]) -> Ref {
    Ref {
        digest: hash(bytes),
        length: bytes.len() as u64,
    }
}
fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    photara_core::canonical_json(&serde_json::to_value(value).map_err(|_| Error::Integrity)?)
        .map_err(|_| Error::Integrity)
}
fn parse<T: for<'a> Deserialize<'a>>(bytes: &[u8]) -> Result<T> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::Integrity)?;
    if canon(&value) != bytes {
        return Err(Error::Integrity);
    }
    serde_json::from_value(value).map_err(|_| Error::Integrity)
}

fn file_paths(root: &Path) -> Result<BTreeSet<String>> {
    fn visit(base: &Path, directory: &Path, out: &mut BTreeSet<String>) -> Result<()> {
        for entry in fs::read_dir(directory).map_err(|_| Error::Io)? {
            let entry = entry.map_err(|_| Error::Io)?;
            let kind = entry.file_type().map_err(|_| Error::Io)?;
            if kind.is_dir() {
                visit(base, &entry.path(), out)?;
            } else if kind.is_file() {
                out.insert(
                    entry
                        .path()
                        .strip_prefix(base)
                        .map_err(|_| Error::Integrity)?
                        .to_str()
                        .ok_or(Error::Integrity)?
                        .into(),
                );
            } else {
                return Err(Error::Integrity);
            }
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    visit(root, root, &mut out)?;
    Ok(out)
}
