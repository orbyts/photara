//! Nonnil, canonical UUID coordinates; no clock or random-ID allocation.

use super::{ContractError, Result};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! ids {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Uuid);
        impl $name {
            /// Validates an existing semantic coordinate without allocating an ID.
            /// # Errors
            /// Rejects nil UUIDs.
            pub const fn from_uuid(value: Uuid) -> Result<Self> {
                if value.is_nil() { Err(ContractError::Identity) } else { Ok(Self(value)) }
            }
            #[must_use] pub const fn uuid(self) -> Uuid { self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = ContractError;
            fn try_from(value: String) -> Result<Self> { value.parse() }
        }
        impl FromStr for $name {
            type Err = ContractError;
            fn from_str(value: &str) -> Result<Self> {
                let id = Uuid::parse_str(value).map_err(|_| ContractError::Identity)?;
                if id.hyphenated().to_string() != value { return Err(ContractError::Identity); }
                Self::from_uuid(id)
            }
        }
        impl From<$name> for String { fn from(value: $name) -> Self { value.0.to_string() } }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
        }
    )+};
}
ids!(
    LibraryId,
    ProjectId,
    GraphId,
    NodeInstanceId,
    AssetId,
    AssetRepresentationId,
    ContentRevisionId,
    StorageLocationId,
    StorageSlotId,
    HostBindingId,
    DeviceId,
    ExternalResourceRefId,
    ExternalResourceRevisionId,
    ProjectResourceId,
    ResourceVersionId,
    AssetSetSnapshotId,
    AccountId,
    LocalPrincipalId,
    ProjectAccessGrantId,
    InvitationId,
    OperationId,
    ReceiptId,
    RunId,
    AttemptId,
    VariableId,
    VariableValueId,
    ExpressionId,
    ContextSnapshotId,
    RunOverrideId,
    CommitId,
    RequestId,
    PersonId,
    OrganizationId,
    LocationId,
    LocationKindId
);

impl StorageLocationId {
    /// Explicit adapter for the unchanged physical `StorageRoot` UUID.
    /// # Errors
    /// Rejects nil; performs no classification or rebind.
    pub const fn from_legacy_storage_root_uuid(value: Uuid) -> Result<Self> {
        Self::from_uuid(value)
    }
    #[must_use]
    pub const fn legacy_storage_root_uuid(self) -> Uuid {
        self.uuid()
    }
}
macro_rules! legacy {
    ($($id:ident),+) => {$ (
        impl TryFrom<crate::$id> for $id {
            type Error = ContractError;
            fn try_from(old: crate::$id) -> Result<Self> { Self::from_uuid(old.as_uuid()) }
        }
        impl From<$id> for crate::$id {
            fn from(value: $id) -> Self { Self::from_uuid(value.uuid()) }
        }
    )+};
}
legacy!(
    ProjectId,
    GraphId,
    NodeInstanceId,
    AssetId,
    AssetRepresentationId,
    ProjectResourceId,
    RequestId
);

/// Historical external resolver identity, deliberately not a `HostBindingId`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LegacyExternalResolverId(Uuid);
impl TryFrom<crate::RepresentationStorageBindingId> for LegacyExternalResolverId {
    type Error = ContractError;
    fn try_from(value: crate::RepresentationStorageBindingId) -> Result<Self> {
        if value.as_uuid().is_nil() {
            Err(ContractError::Identity)
        } else {
            Ok(Self(value.as_uuid()))
        }
    }
}
impl TryFrom<String> for LegacyExternalResolverId {
    type Error = ContractError;
    fn try_from(value: String) -> Result<Self> {
        Ok(Self(value.parse::<ExternalResourceRefId>()?.uuid()))
    }
}
impl From<LegacyExternalResolverId> for String {
    fn from(v: LegacyExternalResolverId) -> Self {
        v.0.to_string()
    }
}
impl LegacyExternalResolverId {
    #[must_use]
    pub const fn legacy(self) -> crate::RepresentationStorageBindingId {
        crate::RepresentationStorageBindingId::from_uuid(self.0)
    }
}
