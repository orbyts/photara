//! Typed JSON/whole-Blob locator access. Structural opening never reads media.
use super::super::{
    DecimalU64, ObjectKind, ObjectRef, PackageError, PackageUuid, Sha256Hex, digest,
};
use super::{FrameKind, PhysicalRecords, PhysicalRef, PhysicalTree, PhysicalTreeKind, TreeLimits};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Membership {
    Semantic,
    Ownership,
    Blob,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEntry {
    object: ObjectRef,
    membership: Membership,
    physical: Value,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WholeBlob {
    kind: String,
    allocation_id: PackageUuid,
    byte_length: DecimalU64,
    sha256: Sha256Hex,
}

/// Metadata inspection and explicit strong audit are separate provider surfaces.
pub trait BlobMetadata {
    /// # Errors
    /// Missing/nonregular/unsafe or changed allocations must refuse.
    fn extent(&self, allocation: PackageUuid) -> Result<u64, PackageError>;
}
pub trait BlobAudit: BlobMetadata {
    /// # Errors
    /// Opens the same pinned allocation described by `extent`, never an external handle.
    fn open(&self, allocation: PackageUuid) -> Result<Box<dyn Read + '_>, PackageError>;
}

/// A structurally checked raw allocation. It does not certify its content digest.
#[derive(Clone, Debug)]
pub struct StructuralBlob {
    object: ObjectRef,
    allocation_id: PackageUuid,
}

pub struct Locator<'a, P> {
    tree: PhysicalTree<'a, P>,
    provider: &'a P,
    root: PhysicalRef,
}
impl<'a, P: PhysicalRecords> Locator<'a, P> {
    #[must_use]
    pub fn new(
        provider: &'a P,
        project: PackageUuid,
        root: PhysicalRef,
        limits: TreeLimits,
    ) -> Self {
        Self {
            tree: PhysicalTree::new(provider, project, PhysicalTreeKind::Locator, limits),
            provider,
            root,
        }
    }
    /// # Errors
    /// Requires an exact JSON locator identity, membership, frame and content digest.
    pub fn json(&self, object: &ObjectRef, membership: Membership) -> Result<Value, PackageError> {
        if object.kind != ObjectKind::Json || membership == Membership::Blob {
            return Err(PackageError::Record);
        }
        let entry = self.entry(object)?;
        if entry.membership != membership {
            return Err(PackageError::Integrity);
        }
        let physical: PhysicalRef =
            serde_json::from_value(entry.physical).map_err(|_| PackageError::Record)?;
        let kind = match membership {
            Membership::Semantic => FrameKind::Semantic,
            Membership::Ownership => FrameKind::Ownership,
            Membership::Blob => return Err(PackageError::Record),
        };
        let value = self.provider.resolve(&physical, kind)?;
        let bytes = photara_core::canonical_json(&value).map_err(|_| PackageError::Record)?;
        if bytes.len() as u64 != object.byte_length.get() || digest(&bytes) != object.sha256 {
            return Err(PackageError::Integrity);
        }
        Ok(value)
    }
    /// Reads extent metadata only. Neither this method nor tree lookup calls `open`.
    /// # Errors
    /// Refuses mismatched whole-object digest/extent, kind, allocation or membership.
    pub fn blob(
        &self,
        object: &ObjectRef,
        metadata: &impl BlobMetadata,
    ) -> Result<StructuralBlob, PackageError> {
        if object.kind != ObjectKind::Blob {
            return Err(PackageError::Record);
        }
        let entry = self.entry(object)?;
        let physical: WholeBlob =
            serde_json::from_value(entry.physical).map_err(|_| PackageError::Record)?;
        if entry.membership != Membership::Blob
            || physical.kind != "whole-blob"
            || physical.allocation_id.as_uuid().is_nil()
            || physical.byte_length != object.byte_length
            || physical.sha256 != object.sha256
            || metadata.extent(physical.allocation_id)? != object.byte_length.get()
        {
            return Err(PackageError::Integrity);
        }
        Ok(StructuralBlob {
            object: object.clone(),
            allocation_id: physical.allocation_id,
        })
    }
    fn entry(&self, object: &ObjectRef) -> Result<WireEntry, PackageError> {
        let value = self
            .tree
            .lookup(&self.root, object)?
            .ok_or(PackageError::Integrity)?;
        let entry: WireEntry = serde_json::from_value(value).map_err(|_| PackageError::Record)?;
        if entry.object != *object {
            return Err(PackageError::Integrity);
        }
        Ok(entry)
    }
}
impl StructuralBlob {
    #[must_use]
    pub fn object(&self) -> &ObjectRef {
        &self.object
    }
    #[must_use]
    pub fn allocation_id(&self) -> PackageUuid {
        self.allocation_id
    }

    /// Explicit streaming content audit with fixed working memory. Rechecks extent
    /// before and after reading; an exact-length corruption still fails its digest.
    /// # Errors
    /// Refuses IO errors, length changes, digest mismatch or caller byte budget.
    pub fn audit(&self, provider: &impl BlobAudit, max_bytes: u64) -> Result<(), PackageError> {
        let expected = self.object.byte_length.get();
        if expected > max_bytes {
            return Err(PackageError::Limit);
        }
        if provider.extent(self.allocation_id)? != expected {
            return Err(PackageError::Integrity);
        }
        let mut source = provider.open(self.allocation_id)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        let mut total = 0u64;
        loop {
            // At most one byte beyond the advertised extent is inspected, solely
            // to reject a longer allocation without hashing arbitrary extra data.
            let take = usize::try_from((expected - total).min(buffer.len() as u64))
                .map_err(|_| PackageError::Limit)?
                .max(1);
            let n = loop {
                match source.read(&mut buffer[..take]) {
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    result => break result.map_err(|e| PackageError::Io(e.kind()))?,
                }
            };
            if n == 0 {
                break;
            }
            total = total.checked_add(n as u64).ok_or(PackageError::Limit)?;
            if total > expected {
                return Err(PackageError::Integrity);
            }
            hasher.update(&buffer[..n]);
        }
        if total != expected
            || format!("{:x}", hasher.finalize()) != self.object.sha256.as_str()
            || provider.extent(self.allocation_id)? != expected
        {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
}

pub(super) fn validate_entry(value: &Value) -> Result<(), PackageError> {
    let entry: WireEntry =
        serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)?;
    if entry.membership == Membership::Blob {
        let physical: WholeBlob =
            serde_json::from_value(entry.physical).map_err(|_| PackageError::Record)?;
        if entry.object.kind != ObjectKind::Blob
            || physical.kind != "whole-blob"
            || physical.allocation_id.as_uuid().is_nil()
            || physical.sha256 != entry.object.sha256
            || physical.byte_length != entry.object.byte_length
        {
            return Err(PackageError::Record);
        }
    } else {
        let physical: PhysicalRef =
            serde_json::from_value(entry.physical).map_err(|_| PackageError::Record)?;
        if entry.object.kind != ObjectKind::Json
            || entry.object.byte_length.get() == 0
            || physical.allocation_id.as_uuid().is_nil()
            || physical.arena != super::Arena::Data
        {
            return Err(PackageError::Record);
        }
    }
    Ok(())
}
