//! Private fixture snapshot bytes; structural callers receive metadata only.
use super::{
    legacy,
    provider::{RawDescription, Result, SnapshotAudit, SnapshotDescription, SnapshotMetadata},
};
use serde_json::Value;
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};
struct File {
    bytes: Vec<u8>,
    description: SnapshotDescription,
}
pub(super) struct Snapshot {
    files: BTreeMap<Vec<String>, File>,
    media_reads: Cell<u64>,
    media_hashes: Cell<u64>,
    audited_files: Cell<u64>,
}
impl Snapshot {
    pub(super) fn load(corpus: &Value) -> Result<Self> {
        let rows = corpus["snapshot_files"]
            .as_object()
            .ok_or("snapshot files")?;
        if rows.len() > 256 {
            return Err("snapshot count bound");
        }
        let mut files = BTreeMap::new();
        let mut total = 0usize;
        for (name, row) in rows {
            let path = name.split('/').map(str::to_owned).collect::<Vec<_>>();
            photara_core::contracts::resource::RelativeComponents::new(path.clone())
                .map_err(|_| "snapshot portable path")?;
            let bytes = super::wire::unhex(row["hex"].as_str().ok_or("snapshot hex")?)?;
            total = total
                .checked_add(bytes.len())
                .ok_or("snapshot aggregate overflow")?;
            if total > 16 * 1024 * 1024 {
                return Err("snapshot aggregate bound");
            }
            let description = SnapshotDescription {
                regular: row["regular"].as_bool().ok_or("snapshot regular type")?,
                physical: RawDescription {
                    extent: bytes.len() as u64,
                    device: super::wire::number(&row["witness"]["device"])?,
                    inode: super::wire::number(&row["witness"]["inode"])?,
                },
            };
            files.insert(path, File { bytes, description });
        }
        Ok(Self {
            files,
            media_reads: Cell::new(0),
            media_hashes: Cell::new(0),
            audited_files: Cell::new(0),
        })
    }
    pub(super) fn counters(&self) -> (u64, u64, u64) {
        (
            self.media_reads.get(),
            self.media_hashes.get(),
            self.audited_files.get(),
        )
    }
    pub(super) fn corrupt(&mut self, path: &[String]) -> Result<()> {
        let file = self.files.get_mut(path).ok_or("snapshot path")?;
        let first = file.bytes.first_mut().ok_or("empty snapshot file")?;
        *first ^= 1;
        Ok(())
    }
    pub(super) fn replace_inode(&mut self, path: &[String]) -> Result<()> {
        self.files
            .get_mut(path)
            .ok_or("snapshot path")?
            .description
            .physical
            .inode += 1;
        Ok(())
    }
    pub(super) fn nonregular(&mut self, path: &[String]) -> Result<()> {
        self.files
            .get_mut(path)
            .ok_or("snapshot path")?
            .description
            .regular = false;
        Ok(())
    }
    pub(super) fn remove(&mut self, path: &[String]) {
        self.files.remove(path);
    }
    pub(super) fn add_foreign(&mut self) {
        self.files.insert(
            vec!["foreign.bin".into()],
            File {
                bytes: vec![1],
                description: SnapshotDescription {
                    regular: true,
                    physical: RawDescription {
                        extent: 1,
                        device: 7,
                        inode: 99_999,
                    },
                },
            },
        );
    }
}
impl SnapshotMetadata for Snapshot {
    fn paths(&self) -> Result<BTreeSet<Vec<String>>> {
        Ok(self.files.keys().cloned().collect())
    }
    fn describe_file(&self, path: &[String]) -> Result<SnapshotDescription> {
        Ok(self
            .files
            .get(path)
            .ok_or("snapshot file absent")?
            .description
            .clone())
    }
}
impl SnapshotAudit for Snapshot {
    fn audit_file(
        &self,
        path: &[String],
        digest: &str,
        registered: &SnapshotDescription,
    ) -> Result<()> {
        let file = self.files.get(path).ok_or("snapshot file absent")?;
        if !file.description.regular || file.description != *registered {
            return Err("snapshot original witness");
        }
        if path.starts_with(&["objects".into(), "blobs".into(), "sha256".into()]) {
            self.media_reads
                .set(self.media_reads.get() + file.bytes.len() as u64);
            self.media_hashes.set(self.media_hashes.get() + 1);
        }
        self.audited_files.set(self.audited_files.get() + 1);
        if legacy::hash(&file.bytes) != digest {
            return Err("snapshot strong audit digest");
        }
        Ok(())
    }
}
