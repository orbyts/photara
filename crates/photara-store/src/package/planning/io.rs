//! Future PS2 adapter boundary only; no filesystem implementation or coordinator.
use super::super::Sha256Hex;
use super::{CheckpointPlan, HeadToken, PlanError, WriteId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageKind {
    LocalApfs,
    Network,
    ProviderManaged,
    Unqualified,
}
/// Assertions from a future separately qualified adapter; never inferred from a
/// volume name or successful rename. PS1 validates policy, not hardware durability.
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent required storage guarantees, not state flags"
)]
#[derive(Clone, Debug)]
pub struct CapabilityProfile {
    pub version: u32,
    pub kind: StorageKind,
    pub volume_identity: Sha256Hex,
    pub safe_handles: bool,
    pub exclusive_lifetime_lock: bool,
    pub immutable_no_replace: bool,
    pub atomic_same_volume_head: bool,
    pub full_file_flush: bool,
    pub directory_flush: bool,
    pub excludes_uncooperative_writers: bool,
}
impl CapabilityProfile {
    /// # Errors
    /// Refuses unknown, provider-managed, remote or incompletely qualified storage.
    pub fn check_policy(&self) -> Result<(), PlanError> {
        if self.version != 1
            || self.kind != StorageKind::LocalApfs
            || !self.safe_handles
            || !self.exclusive_lifetime_lock
            || !self.immutable_no_replace
            || !self.atomic_same_volume_head
            || !self.full_file_flush
            || !self.directory_flush
            || !self.excludes_uncooperative_writers
        {
            return Err(PlanError::UnsupportedStorage);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritePhase {
    Pin,
    Lock,
    Recheck,
    Create,
    Write,
    FileFlush,
    PublishImmutable,
    DirectoryFlush,
    ReplaceHead,
    Verify,
    JournalReceipt,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoFailure {
    NotPerformed(WritePhase),
    OutcomeUnknown(WritePhase),
}
/// A PS2 implementer must expose opaque owned pins, not raw paths/descriptors.
/// Every method is a separately injectable before/after/unknown boundary.
/// No code in PS1 calls these methods; no durable receipt can be constructed here.
pub trait PackageIo {
    type Locator;
    type DirectoryPin;
    type FilePin;
    type Lease;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn pin(&mut self, locator: &Self::Locator) -> Result<Self::DirectoryPin, IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn qualify(&mut self, root: &Self::DirectoryPin) -> Result<CapabilityProfile, IoFailure>;
    /// Stable .writer-lock inode; never unlink or steal by elapsed time.
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn lock(&mut self, root: &Self::DirectoryPin) -> Result<Self::Lease, IoFailure>;
    /// Recheck root/parent/volume/lock identity, manifest and exact old HEAD under lock.
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn recheck(
        &mut self,
        root: &Self::DirectoryPin,
        lease: &Self::Lease,
        expected: &HeadToken,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn create_temporary(
        &mut self,
        root: &Self::DirectoryPin,
        plan: &CheckpointPlan,
        write: WriteId,
    ) -> Result<Self::FilePin, IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn write_all(&mut self, file: &Self::FilePin, bytes: &[u8]) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn full_flush(&mut self, file: &Self::FilePin) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn publish_no_replace(
        &mut self,
        root: &Self::DirectoryPin,
        file: &Self::FilePin,
        relative_name: &str,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn flush_directory(&mut self, directory: &Self::DirectoryPin) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn replace_head(
        &mut self,
        root: &Self::DirectoryPin,
        lease: &Self::Lease,
        file: &Self::FilePin,
        expected: &HeadToken,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn verify_candidate(
        &mut self,
        root: &Self::DirectoryPin,
        plan: &CheckpointPlan,
    ) -> Result<(), IoFailure>;
    /// Only an attempt-owned temporary pin; never recursive cleanup or GC.
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn unlink_owned_temporary(
        &mut self,
        root: &Self::DirectoryPin,
        file: Self::FilePin,
    ) -> Result<(), IoFailure>;
}
