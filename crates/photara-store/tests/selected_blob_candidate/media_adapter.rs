//! Fixture-specific adapter; the shared provider module knows no concrete media storage.
use super::{
    media::{Description, Media},
    provider::{RawAudit, RawDescription, RawMetadata, Result},
};
use photara_store::package::ObjectRef;
use std::collections::BTreeSet;
pub(super) struct InstrumentedMedia {
    media: Media,
    ids: BTreeSet<String>,
}
impl InstrumentedMedia {
    /// Exact names and private media come from the same bounded fixture input.
    pub(super) fn load(corpus: &serde_json::Value) -> Result<Self> {
        let media = Media::load(corpus)?;
        let ids = corpus["allocations"]
            .as_object()
            .ok_or("raw allocation map")?
            .iter()
            .filter(|(_, v)| v["layout"] == "whole-blob")
            .map(|(id, _)| id.clone())
            .collect();
        Ok(Self { media, ids })
    }
    pub(super) fn counters(&self) -> (u64, u64) {
        self.media.counters()
    }
    pub(super) fn corrupt_byte(&mut self, id: &str) -> Result<()> {
        self.media.corrupt_byte(id)
    }
    pub(super) fn replace_inode(&mut self, id: &str) -> Result<()> {
        self.media.replace_inode(id)
    }
}
fn numeric(s: &str) -> Result<u64> {
    let n = s.parse::<u64>().map_err(|_| "raw decimal range")?;
    if n.to_string() != s {
        return Err("raw canonical decimal");
    }
    Ok(n)
}
impl RawMetadata for InstrumentedMedia {
    fn allocation_ids(&self) -> Result<BTreeSet<String>> {
        Ok(self.ids.clone())
    }
    fn describe(&self, id: &str) -> Result<RawDescription> {
        if !self.ids.contains(id) {
            return Err("unregistered raw provider name");
        }
        let d = self.media.describe(id)?;
        Ok(RawDescription {
            extent: d.extent,
            device: numeric(&d.device)?,
            inode: numeric(&d.inode)?,
        })
    }
}
impl RawAudit for InstrumentedMedia {
    fn audit(&self, r: &ObjectRef, id: &str, registered: &RawDescription) -> Result<()> {
        super::provider::observe_blob(self, r, id, registered)?;
        self.media.audit(
            r,
            id,
            &Description {
                extent: registered.extent,
                device: registered.device.to_string(),
                inode: registered.inode.to_string(),
            },
        )
    }
}
