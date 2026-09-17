//! Future PS2 adapter boundary only; no filesystem implementation or coordinator.
use super::super::Sha256Hex;
use super::{CheckpointPlan, HeadToken, OwnerEpoch, PlanError, WriteId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageKind {
    LocalApfs,
    Network,
    ProviderManaged,
    Unqualified,
}
/// Assertions from a future separately qualified adapter; never inferred from a
/// volume name or successful rename. PS1 validates primitive policy, not hardware
/// durability, registration, authorization or permission to publish.
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
}
impl CapabilityProfile {
    /// Necessary primitive policy only; success cannot construct writer admission.
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
        {
            return Err(PlanError::UnsupportedStorage);
        }
        Ok(())
    }
}

/// Expected coordinates are caller-visible data, never admission evidence.
/// No supported protocol digest is selected by this interface-only slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionExpectation {
    pub head: HeadToken,
    pub volume_identity: Sha256Hex,
    pub owner_epoch: OwnerEpoch,
    pub protocol_digest: Sha256Hex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionFailure {
    NotRegistered,
    IdentityOrHeadChanged,
    OwnerChanged,
    ProtocolMismatch,
}

/// Opaque, non-cloneable registered per-package admission plus the held OS lease.
/// No production constructor exists: a future trusted registrar must establish
/// host authorization, package registration, storage qualification, safe pins and
/// lock ownership before minting this evidence. A path, profile, or raw lock cannot
/// grant it. Synthetic unit tests alone can construct it in this slice.
///
/// ```compile_fail
/// use photara_store::package::planning::io::RegisteredCooperativeLease;
/// // Even an adapter's raw lock cannot be promoted by an outside caller.
/// let admission: RegisteredCooperativeLease<()> = RegisteredCooperativeLease::from(());
/// ```
/// ```compile_fail
/// use photara_store::package::planning::io::RegisteredCooperativeLease;
/// let admission: RegisteredCooperativeLease<()> = Default::default();
/// ```
/// ```compile_fail
/// use photara_store::package::planning::io::{AdmissionExpectation, RegisteredCooperativeLease};
/// fn forge(binding: AdmissionExpectation) -> RegisteredCooperativeLease<()> {
///     RegisteredCooperativeLease { platform_lease: (), binding }
/// }
/// ```
/// ```compile_fail
/// use photara_store::package::planning::io::{CapabilityProfile, PackageIo};
/// fn profile_is_not_permission<T: PackageIo>(
///     io: &mut T, profile: &mut CapabilityProfile,
///     root: &T::DirectoryPin, file: &T::FilePin,
/// ) {
///     profile.check_policy().unwrap();
///     io.publish_no_replace(profile, root, file, "objects/example").unwrap();
/// }
/// ```
pub struct RegisteredCooperativeLease<L> {
    platform_lease: L,
    binding: AdmissionExpectation,
}
impl<L> RegisteredCooperativeLease<L> {
    /// Borrow the retained platform lock for adapter checks, never transfer it out.
    #[must_use]
    pub fn platform_lease(&self) -> &L {
        &self.platform_lease
    }
}

/// Pure necessary binding check, not OS revalidation or authorization renewal.
/// Success creates no permission and never replaces the opaque lease parameter.
/// # Errors
/// Refuses absent registration, changed exact HEAD/incarnation/volume, owner or
/// protocol. Even matching evidence still requires pin/lock/HEAD rechecks at I/O.
pub fn check_admission<L>(
    admission: Option<&RegisteredCooperativeLease<L>>,
    expected: &AdmissionExpectation,
) -> Result<(), AdmissionFailure> {
    let binding = &admission.ok_or(AdmissionFailure::NotRegistered)?.binding;
    if binding.protocol_digest != expected.protocol_digest {
        return Err(AdmissionFailure::ProtocolMismatch);
    }
    if binding.owner_epoch != expected.owner_epoch {
        return Err(AdmissionFailure::OwnerChanged);
    }
    if binding.head != expected.head || binding.volume_identity != expected.volume_identity {
        return Err(AdmissionFailure::IdentityOrHeadChanged);
    }
    Ok(())
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockFailure {
    /// Another owner (GUI, CLI, agent or otherwise) holds the lifetime lease.
    /// No routing, handoff, timeout theft or progress guarantee is implied.
    WriterBusy,
    Io(IoFailure),
}
/// A PS2 implementer must expose opaque owned pins, not raw paths/descriptors.
/// Every method is a separately injectable before/after/unknown boundary.
/// Every mutation requires opaque registered admission, not just primitive policy
/// or a raw lock. No code in PS1 calls these methods; no production admission or
/// durable receipt can be constructed here. An implementation is a trusted adapter,
/// not a way to exclude arbitrary programs that ignore this cooperative protocol.
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
    /// Acquire an existing stable .writer-lock inode; never unlink or steal by
    /// elapsed time. Lock-file bootstrap/registration is not implemented here.
    /// # Errors
    /// Returns `WriterBusy` for a competing owner, otherwise phase-tagged I/O failure.
    fn lock(&mut self, root: &Self::DirectoryPin) -> Result<Self::Lease, LockFailure>;
    /// Recheck root/parent/volume/lock identity, manifest and exact old HEAD under lock.
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn recheck(
        &mut self,
        root: &Self::DirectoryPin,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        expected: &HeadToken,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn create_temporary(
        &mut self,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        root: &Self::DirectoryPin,
        plan: &CheckpointPlan,
        write: WriteId,
    ) -> Result<Self::FilePin, IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn write_all(
        &mut self,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        file: &Self::FilePin,
        bytes: &[u8],
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn full_flush(
        &mut self,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        file: &Self::FilePin,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn publish_no_replace(
        &mut self,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        root: &Self::DirectoryPin,
        file: &Self::FilePin,
        relative_name: &str,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn flush_directory(
        &mut self,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        directory: &Self::DirectoryPin,
    ) -> Result<(), IoFailure>;
    /// # Errors
    /// Returns a phase-tagged pre-effect failure or unknown post-effect outcome.
    fn replace_head(
        &mut self,
        root: &Self::DirectoryPin,
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
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
        admission: &mut RegisteredCooperativeLease<Self::Lease>,
        root: &Self::DirectoryPin,
        file: Self::FilePin,
    ) -> Result<(), IoFailure>;
}

#[cfg(test)]
mod tests;
