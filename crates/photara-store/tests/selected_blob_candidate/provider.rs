//! Metadata-only interface for shared structural verification. No raw byte getter.
use photara_store::package::{ObjectKind, ObjectRef};
use std::collections::BTreeSet;
pub(crate) type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawDescription {
    pub extent: u64,
    pub device: u64,
    pub inode: u64,
}
pub(crate) trait RawMetadata {
    fn allocation_ids(&self) -> Result<BTreeSet<String>>;
    fn describe(&self, allocation_id: &str) -> Result<RawDescription>;
}
/// Explicit audit callers receive this stronger interface; the structural driver does not.
pub(crate) trait RawAudit: RawMetadata {
    fn audit(
        &self,
        reference: &ObjectRef,
        allocation_id: &str,
        registered: &RawDescription,
    ) -> Result<()>;
}
pub(crate) fn observe_blob(
    provider: &dyn RawMetadata,
    reference: &ObjectRef,
    allocation_id: &str,
    registered: &RawDescription,
) -> Result<RawDescription> {
    if reference.kind != ObjectKind::Blob {
        return Err("raw reference kind");
    }
    let id = uuid::Uuid::parse_str(allocation_id).map_err(|_| "raw allocation UUID")?;
    if id.is_nil() || id.to_string() != allocation_id {
        return Err("raw canonical allocation UUID");
    }
    if !provider.allocation_ids()?.contains(allocation_id) {
        return Err("raw allocation absent");
    }
    let observed = provider.describe(allocation_id)?;
    if observed.inode == 0
        || observed != *registered
        || observed.extent != reference.byte_length.get()
    {
        return Err("raw registered extent/witness");
    }
    Ok(observed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotDescription {
    pub regular: bool,
    pub physical: RawDescription,
}
/// Private snapshot metadata only; no selected descriptor authorizes byte access.
pub(crate) trait SnapshotMetadata {
    fn paths(&self) -> Result<BTreeSet<Vec<String>>>;
    fn describe_file(&self, path: &[String]) -> Result<SnapshotDescription>;
}
pub(crate) trait SnapshotAudit: SnapshotMetadata {
    fn audit_file(
        &self,
        path: &[String],
        digest: &str,
        registered: &SnapshotDescription,
    ) -> Result<()>;
}
