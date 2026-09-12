//! Portable resource descriptions and deliberately nonserializable live authority.
use super::{
    ContractError, Result,
    ids::{
        DeviceId, ExternalResourceRefId, ExternalResourceRevisionId, HostBindingId, LibraryId,
        OperationId, ProjectId, ProjectResourceId, ResourceVersionId, StorageLocationId,
        StorageSlotId,
    },
    schema::{
        DecimalU64, Digest, LocalName, LocalRevision, ObjectKind, ObjectRef, QualifiedName,
        display_text, ordered_unique, unique,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceRight {
    Read,
    List,
    Create,
    Replace,
    Delete,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<ResourceRight>", into = "Vec<ResourceRight>")]
pub struct ResourceRights(BTreeSet<ResourceRight>);
impl ResourceRights {
    /// Builds a rights declaration, never a grant.
    /// # Errors
    /// Duplicate rights are rejected.
    pub fn new(rights: Vec<ResourceRight>) -> Result<Self> {
        Self::try_from(rights)
    }
    #[must_use]
    pub fn contains(&self, right: ResourceRight) -> bool {
        self.0.contains(&right)
    }
    #[must_use]
    pub fn permits(&self, requested: &Self) -> bool {
        requested.0.is_subset(&self.0)
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    #[must_use]
    pub fn writes(&self) -> bool {
        self.0.iter().any(|r| {
            matches!(
                r,
                ResourceRight::Create | ResourceRight::Replace | ResourceRight::Delete
            )
        })
    }
}
impl TryFrom<Vec<ResourceRight>> for ResourceRights {
    type Error = ContractError;
    fn try_from(v: Vec<ResourceRight>) -> Result<Self> {
        unique(&v)?;
        Ok(Self(v.into_iter().collect()))
    }
}
impl From<ResourceRights> for Vec<ResourceRight> {
    fn from(r: ResourceRights) -> Self {
        r.0.into_iter().collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct RelativeComponents(Vec<String>);
impl RelativeComponents {
    /// Validates exact UTF-8 components without rewriting/casefolding them.
    /// # Errors
    /// Rejects traversal, platform-reserved syntax/names, controls and bounds.
    pub fn new(parts: Vec<String>) -> Result<Self> {
        Self::try_from(parts)
    }
    #[must_use]
    pub fn parts(&self) -> &[String] {
        &self.0
    }
    /// An explicit join with validated components; no filesystem interpretation.
    /// # Errors
    /// Rejects aggregate bounds.
    pub fn join(&self, child: &Self) -> Result<Self> {
        let mut parts = self.0.clone();
        parts.extend(child.0.clone());
        Self::new(parts)
    }
}
impl TryFrom<Vec<String>> for RelativeComponents {
    type Error = ContractError;
    fn try_from(parts: Vec<String>) -> Result<Self> {
        if parts.is_empty()
            || parts.len() > 128
            || parts.iter().map(|s| s.len() + 1).sum::<usize>() > 4096
        {
            return Err(ContractError::Limit);
        }
        for s in &parts {
            if s.is_empty()
                || s.len() > 255
                || s == "."
                || s == ".."
                || s.ends_with(['.', ' '])
                || s.chars()
                    .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
            {
                return Err(ContractError::Coordinate);
            }
            let stem = s.split('.').next().unwrap_or_default().to_ascii_uppercase();
            if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"].contains(&stem.as_str())
                || ["COM", "LPT"].iter().any(|p| {
                    stem.strip_prefix(p).is_some_and(|n| {
                        ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&n)
                    })
                })
            {
                return Err(ContractError::Coordinate);
            }
        }
        Ok(Self(parts))
    }
}
impl From<RelativeComponents> for Vec<String> {
    fn from(v: RelativeComponents) -> Self {
        v.0
    }
}

/// Nonsecret adapter coordinate. It is opaque data, never a URL to fetch or path.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProviderCoordinate(String);
impl ProviderCoordinate {
    /// # Errors
    /// Rejects blank, control-bearing and overlong adapter coordinates.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        Self::try_from(s.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ProviderCoordinate {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        display_text(&s, 1024)?;
        Ok(Self(s))
    }
}
impl From<ProviderCoordinate> for String {
    fn from(v: ProviderCoordinate) -> Self {
        v.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StorageKind {
    Filesystem,
    Provider { provider_id: QualifiedName },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lifecycle {
    Active,
    Tombstoned,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageLocationDescriptor {
    pub library_id: LibraryId,
    pub location_id: StorageLocationId,
    pub revision: LocalRevision,
    pub kind: StorageKind,
    pub display_name: String,
    pub purpose: LocalName,
    pub supported_rights: ResourceRights,
    pub lifecycle: Lifecycle,
}
impl StorageLocationDescriptor {
    /// # Errors
    /// Rejects invalid labels; registered provider support is a host concern.
    pub fn validate(&self) -> Result<()> {
        display_text(&self.display_name, 1024)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageSlotDescriptor {
    pub library_id: LibraryId,
    pub slot_id: StorageSlotId,
    pub revision: LocalRevision,
    pub current_name: LocalName,
    pub claimed_names: Vec<LocalName>,
    pub display_name: String,
    pub location_id: StorageLocationId,
    pub lifecycle: Lifecycle,
}
impl StorageSlotDescriptor {
    /// # Errors
    /// Rejects unclaimed current names, invalid labels, mismatched/retired targets.
    pub fn validate_target(&self, target: &StorageLocationDescriptor) -> Result<()> {
        target.validate()?;
        display_text(&self.display_name, 1024)?;
        ordered_unique(&self.claimed_names)?;
        if !self.claimed_names.contains(&self.current_name) {
            return Err(ContractError::Missing);
        }
        if self.library_id != target.library_id || self.location_id != target.location_id {
            return Err(ContractError::Scope);
        }
        if self.lifecycle == Lifecycle::Active && target.lifecycle != Lifecycle::Active {
            return Err(ContractError::Union);
        }
        Ok(())
    }
    /// Captures the current target/revision without establishing access.
    #[must_use]
    pub const fn capture(&self) -> SlotCapture {
        SlotCapture {
            library_id: self.library_id,
            slot_id: self.slot_id,
            slot_revision: self.revision,
            location_id: self.location_id,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotCapture {
    pub library_id: LibraryId,
    pub slot_id: StorageSlotId,
    pub slot_revision: LocalRevision,
    pub location_id: StorageLocationId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageClass {
    ExternalSource,
    ManagedProject,
    ExternalOutput,
    TransientCache,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExternalCoordinate {
    Filesystem {
        components: RelativeComponents,
    },
    Provider {
        provider_id: QualifiedName,
        namespace: ProviderCoordinate,
        object_id: ProviderCoordinate,
        revision: Option<ProviderCoordinate>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ContentEvidence {
    Sha256 { digest: Digest },
    ProviderRevision { revision: ProviderCoordinate },
    Unverified,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRevisionSpec {
    pub project_id: ProjectId,
    pub external_ref_id: ExternalResourceRefId,
    pub revision_id: ExternalResourceRevisionId,
    pub source_library_id: LibraryId,
    pub storage_location_id: StorageLocationId,
    pub coordinate: ExternalCoordinate,
    pub content_evidence: ContentEvidence,
    pub storage_class: StorageClass,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ExternalRevisionSpec", into = "ExternalRevisionSpec")]
pub struct ExternalResourceRevision(ExternalRevisionSpec);
impl ExternalResourceRevision {
    #[must_use]
    pub const fn spec(&self) -> &ExternalRevisionSpec {
        &self.0
    }
    /// Validates the explicit logical target; does not resolve host bindings.
    /// # Errors
    /// Rejects scope, storage-kind/provider mismatch or tombstoned target.
    pub fn validate_location(&self, location: &StorageLocationDescriptor) -> Result<()> {
        location.validate()?;
        if self.0.source_library_id != location.library_id
            || self.0.storage_location_id != location.location_id
        {
            return Err(ContractError::Scope);
        }
        if location.lifecycle != Lifecycle::Active {
            return Err(ContractError::Union);
        }
        match (&self.0.coordinate, &location.kind) {
            (ExternalCoordinate::Filesystem { .. }, StorageKind::Filesystem) => Ok(()),
            (
                ExternalCoordinate::Provider { provider_id, .. },
                StorageKind::Provider {
                    provider_id: expected,
                },
            ) if provider_id == expected => Ok(()),
            _ => Err(ContractError::Union),
        }
    }
}
impl TryFrom<ExternalRevisionSpec> for ExternalResourceRevision {
    type Error = ContractError;
    fn try_from(v: ExternalRevisionSpec) -> Result<Self> {
        if !matches!(
            v.storage_class,
            StorageClass::ExternalSource | StorageClass::ExternalOutput
        ) {
            return Err(ContractError::Union);
        }
        if let ContentEvidence::ProviderRevision { revision: expected } = &v.content_evidence {
            match &v.coordinate {
                ExternalCoordinate::Provider {
                    revision: Some(actual),
                    ..
                } if actual == expected => {}
                _ => return Err(ContractError::Union),
            }
        }
        Ok(Self(v))
    }
}
impl From<ExternalResourceRevision> for ExternalRevisionSpec {
    fn from(v: ExternalResourceRevision) -> Self {
        v.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalResourceRef {
    pub project_id: ProjectId,
    pub external_ref_id: ExternalResourceRefId,
    pub selected_revision_id: Option<ExternalResourceRevisionId>,
}
impl ExternalResourceRef {
    /// # Errors
    /// Rejects a revision outside this reference or its current explicit selection.
    pub fn validate_selection(&self, revision: &ExternalResourceRevision) -> Result<()> {
        let r = revision.spec();
        if self.project_id != r.project_id
            || self.external_ref_id != r.external_ref_id
            || self.selected_revision_id != Some(r.revision_id)
        {
            return Err(ContractError::Scope);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedResourceSpec {
    pub project_id: ProjectId,
    pub resource_id: ProjectResourceId,
    pub version_id: ResourceVersionId,
    pub blob: ObjectRef,
    pub media_type: super::schema::MediaType,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ManagedResourceSpec", into = "ManagedResourceSpec")]
pub struct ManagedProjectResource(ManagedResourceSpec);
impl ManagedProjectResource {
    #[must_use]
    pub const fn spec(&self) -> &ManagedResourceSpec {
        &self.0
    }
}
impl TryFrom<ManagedResourceSpec> for ManagedProjectResource {
    type Error = ContractError;
    fn try_from(v: ManagedResourceSpec) -> Result<Self> {
        if v.blob.kind != ObjectKind::Blob {
            return Err(ContractError::Union);
        }
        Ok(Self(v))
    }
}
impl From<ManagedProjectResource> for ManagedResourceSpec {
    fn from(v: ManagedProjectResource) -> Self {
        v.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HostPlace {
    Home,
    Downloads,
    Desktop,
    Documents,
    Pictures,
    Temp,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ResourceDescriptor {
    External { revision: ExternalResourceRevision },
    Managed { resource: ManagedProjectResource },
    ProjectRoot { project_id: ProjectId },
    ProjectArtifacts { project_id: ProjectId },
    HostPlace { place: HostPlace },
}
impl ResourceDescriptor {
    #[must_use]
    pub const fn project_id(&self) -> Option<ProjectId> {
        match self {
            Self::External { revision } => Some(revision.spec().project_id),
            Self::Managed { resource } => Some(resource.spec().project_id),
            Self::ProjectRoot { project_id } | Self::ProjectArtifacts { project_id } => {
                Some(*project_id)
            }
            Self::HostPlace { .. } => None,
        }
    }
    /// Rights shape only; host/controller still supplies authorization.
    /// # Errors
    /// Rejects package writes, writes to external-source descriptions and empty rights.
    pub fn validate_requested_rights(&self, rights: &ResourceRights) -> Result<()> {
        if rights.is_empty() {
            return Err(ContractError::Missing);
        }
        match self {
            Self::ProjectArtifacts { .. } => Err(ContractError::Unsupported), // publisher target, never direct materialization
            Self::ProjectRoot { .. } | Self::Managed { .. } if rights.writes() => {
                Err(ContractError::Union)
            }
            Self::External { revision }
                if revision.spec().storage_class == StorageClass::ExternalSource
                    && rights.writes() =>
            {
                Err(ContractError::Union)
            }
            _ => Ok(()),
        }
    }
}
/// A publisher intention, not an objects-directory lease or an implemented publisher.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedArtifactTarget {
    pub project_id: ProjectId,
    pub operation_id: OperationId,
    pub storage_class: StorageClass,
}
impl ManagedArtifactTarget {
    /// # Errors
    /// Only managed-project publication targets are valid.
    pub fn validate(&self) -> Result<()> {
        if self.storage_class == StorageClass::ManagedProject {
            Ok(())
        } else {
            Err(ContractError::Union)
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionStatus {
    Ready,
    NotBound,
    Unavailable,
    Denied,
    Stale,
    Ambiguous,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostKind {
    Macos,
    Windows,
    Linux,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingState {
    Candidate,
    Verified,
    Retired,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostBindingStatus {
    pub device_id: DeviceId,
    pub library_id: LibraryId,
    pub location_id: StorageLocationId,
    pub binding_id: HostBindingId,
    pub host: HostKind,
    pub generation: LocalRevision,
    pub state: BindingState,
    pub selected: bool,
    pub status: ResolutionStatus,
}
impl HostBindingStatus {
    /// # Errors
    /// Rejects a selected or ready unverified binding. Evidence validation is host-owned.
    pub fn validate(&self) -> Result<()> {
        if (self.selected || self.status == ResolutionStatus::Ready)
            && self.state != BindingState::Verified
        {
            Err(ContractError::Union)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CollisionPolicy {
    FailIfPresent,
    Replace { expected_target: Digest },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRequest {
    pub project_id: ProjectId,
    pub operation_id: OperationId,
    pub descriptor: ResourceDescriptor,
    pub expected_content: Option<Digest>,
    pub rights: ResourceRights,
    pub collision: CollisionPolicy,
    pub max_bytes: DecimalU64,
}
impl ResolutionRequest {
    /// # Errors
    /// Rejects scope/rights conflicts, unpinned reads and replacement without bounds.
    pub fn validate(&self) -> Result<()> {
        if self
            .descriptor
            .project_id()
            .is_some_and(|p| p != self.project_id)
        {
            return Err(ContractError::Scope);
        }
        self.descriptor.validate_requested_rights(&self.rights)?;
        let described_digest = match &self.descriptor {
            ResourceDescriptor::External { revision } => match revision.spec().content_evidence {
                ContentEvidence::Sha256 { digest } => Some(digest),
                _ => None,
            },
            ResourceDescriptor::Managed { resource } => Some(resource.spec().blob.sha256),
            _ => None,
        };
        if self.expected_content.is_some()
            && described_digest.is_some()
            && self.expected_content != described_digest
        {
            return Err(ContractError::Digest);
        }
        if self.rights.contains(ResourceRight::Read)
            && self.expected_content.is_none()
            && !matches!(
                self.descriptor,
                ResourceDescriptor::ProjectRoot { .. } | ResourceDescriptor::HostPlace { .. }
            )
        {
            return Err(ContractError::Missing);
        }
        if matches!(self.collision, CollisionPolicy::Replace { .. })
            != self.rights.contains(ResourceRight::Replace)
        {
            return Err(ContractError::Union);
        }
        Ok(())
    }
}

/// Host-issued live authority. Deliberately no Serialize, Deserialize or Clone.
/// The host alone implements containment, expiry, revocation and actual I/O.
///
/// ```compile_fail
/// use photara_core::contracts::resource::MaterializedResourceLease;
/// fn persist<T: serde::Serialize>() {}
/// persist::<MaterializedResourceLease<()>>();
/// ```
pub struct MaterializedResourceLease<T> {
    lease_id: MaterializedHandleId,
    request: ResolutionRequest,
    host_handle: T,
}
/// A live identity, deliberately not a persistable UUID coordinate.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MaterializedHandleId(uuid::Uuid);
impl MaterializedHandleId {
    /// # Errors
    /// Rejects nil host-issued IDs; does not allocate authority.
    pub const fn from_host_uuid(id: uuid::Uuid) -> Result<Self> {
        if id.is_nil() {
            Err(ContractError::Identity)
        } else {
            Ok(Self(id))
        }
    }
}
impl fmt::Debug for MaterializedHandleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MaterializedHandleId([redacted])")
    }
}
impl<T> MaterializedResourceLease<T> {
    /// Wraps a host-verified exact resource/operation request; does not grant it.
    /// # Errors
    /// Rejects an invalid request shape.
    pub fn from_host(
        lease_id: MaterializedHandleId,
        request: ResolutionRequest,
        host_handle: T,
    ) -> Result<Self> {
        request.validate()?;
        Ok(Self {
            lease_id,
            request,
            host_handle,
        })
    }
    #[must_use]
    pub const fn lease_id(&self) -> MaterializedHandleId {
        self.lease_id
    }
    /// # Errors
    /// Rejects reuse for any other operation, descriptor, expected revision or bounds.
    pub fn for_request(&self, request: &ResolutionRequest) -> Result<&T> {
        if self.request != *request {
            return Err(ContractError::Scope);
        }
        Ok(&self.host_handle)
    }
}
impl<T> fmt::Debug for MaterializedResourceLease<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MaterializedResourceLease([redacted])")
    }
}
/// Opaque credential authority: no persistence, comparison, formatting or cloning.
/// ```compile_fail
/// use photara_core::contracts::resource::SecretRef;
/// fn persist<T: serde::Serialize>() {}
/// persist::<SecretRef<()>>();
/// ```
pub struct SecretRef<T>(T);
impl<T> SecretRef<T> {
    #[must_use]
    pub const fn from_host(handle: T) -> Self {
        Self(handle)
    }
    pub fn into_host(self) -> T {
        self.0
    }
}
impl<T> fmt::Debug for SecretRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretRef([redacted])")
    }
}
/// A portable status never carries the lease; this union itself cannot serialize.
pub enum ResourceResolution<T> {
    Ready(Box<MaterializedResourceLease<T>>),
    Unavailable(ResolutionFailure),
}

/// Non-ready outcomes cannot accidentally represent a ready result without a lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionFailure {
    NotBound,
    Unavailable,
    Denied,
    Stale,
    Ambiguous,
    Unsupported,
}
