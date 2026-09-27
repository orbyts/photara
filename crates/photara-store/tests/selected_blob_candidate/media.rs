//! Instrumented raw allocations: structural description never exposes media bytes.
use super::legacy::{Result, ensure, hash};
use photara_store::package::{ObjectKind, ObjectRef};
use serde_json::Value;
use std::cell::Cell;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Description {
    pub(super) extent: u64,
    pub(super) device: String,
    pub(super) inode: String,
}
#[derive(Clone)]
struct Allocation {
    bytes: Vec<u8>,
    description: Description,
}
#[derive(Clone, Default)]
pub(super) struct Media {
    allocations: BTreeMap<String, Allocation>,
    reads: Cell<u64>,
    hashes: Cell<u64>,
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    ensure(
        s.len() <= 32 * 1024 * 1024
            && s.len().is_multiple_of(2)
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "bounded hexadecimal input",
    )?;
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| {
            u8::from_str_radix(std::str::from_utf8(p).map_err(|_| "hex UTF8")?, 16)
                .map_err(|_| "hex byte")
        })
        .collect()
}
impl Media {
    /// Decoding supplied fixture bytes is setup; counters measure later access only.
    pub(super) fn load(corpus: &Value) -> Result<Self> {
        let mut result = Self::default();
        let allocations = corpus["allocations"].as_object().ok_or("allocations")?;
        ensure(allocations.len() <= 16, "allocation fixture bound")?;
        for (id, v) in allocations {
            if v["layout"] != "whole-blob" {
                continue;
            }
            ensure(v["arena"] == "data", "raw data arena")?;
            let bytes = unhex(v["hex"].as_str().ok_or("raw bytes")?)?;
            let description = Description {
                extent: bytes.len() as u64,
                device: v["witness"]["device"].as_str().ok_or("device")?.into(),
                inode: v["witness"]["inode"].as_str().ok_or("inode")?.into(),
            };
            result
                .allocations
                .insert(id.clone(), Allocation { bytes, description });
        }
        Ok(result)
    }
    pub(super) fn describe(&self, id: &str) -> Result<Description> {
        Ok(self
            .allocations
            .get(id)
            .ok_or("missing raw allocation")?
            .description
            .clone())
    }
    pub(super) fn structural(
        &self,
        reference: &ObjectRef,
        id: &str,
        registered: &Description,
    ) -> Result<u64> {
        ensure(reference.kind == ObjectKind::Blob, "raw ObjectRef kind")?;
        let current = self.describe(id)?;
        ensure(
            current == *registered && current.extent == reference.byte_length.get(),
            "original raw extent/witness",
        )?;
        Ok(current.extent)
    }
    pub(super) fn audit(
        &self,
        reference: &ObjectRef,
        id: &str,
        registered: &Description,
    ) -> Result<()> {
        self.structural(reference, id, registered)?;
        let allocation = self.allocations.get(id).ok_or("missing raw allocation")?;
        self.reads.set(
            self.reads
                .get()
                .checked_add(allocation.bytes.len() as u64)
                .ok_or("audit count overflow")?,
        );
        self.hashes.set(
            self.hashes
                .get()
                .checked_add(1)
                .ok_or("audit hash overflow")?,
        );
        ensure(
            hash(&allocation.bytes) == reference.sha256.as_str(),
            "explicit raw audit digest",
        )
    }
    pub(super) fn counters(&self) -> (u64, u64) {
        (self.reads.get(), self.hashes.get())
    }
    pub(super) fn corrupt_byte(&mut self, id: &str) -> Result<()> {
        let allocation = self
            .allocations
            .get_mut(id)
            .ok_or("missing raw allocation")?;
        *allocation
            .bytes
            .first_mut()
            .ok_or("nonempty corruption fixture")? ^= 1;
        Ok(())
    }
    pub(super) fn replace_inode(&mut self, id: &str) -> Result<()> {
        self.allocations
            .get_mut(id)
            .ok_or("missing raw allocation")?
            .description
            .inode = "999999".into();
        Ok(())
    }
}
