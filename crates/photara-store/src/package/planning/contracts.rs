//! Pure checkpoint contracts. None of these types acknowledge durable I/O.
use super::super::{DecimalU64, PackageUuid, Sha256Hex};
use super::PlanError;
use photara_core::{
    GraphCommandEnvelope, GraphId,
    contracts::ids::{CommitId, OperationId},
    creation::PackageExtension,
};
use serde::Serialize;
use std::collections::BTreeMap;

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(PackageUuid);
        impl $name {
            /// # Errors
            /// Requires a canonical, nonnil UUID.
            pub fn parse(value: &str) -> Result<Self, PlanError> {
                let id = PackageUuid::parse(value)?;
                if id.as_uuid().is_nil() {
                    return Err(PlanError::Validation);
                }
                Ok(Self(id))
            }
            #[must_use]
            pub fn uuid(self) -> PackageUuid {
                self.0
            }
        }
    };
}
identity!(WriteId);
identity!(IncarnationId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GraphCoordinate {
    pub revision: DecimalU64,
    /// Core `GraphDocument` canonical digest, distinct from the saved envelope.
    pub semantic_digest: Sha256Hex,
    /// Exact raw graph payload digest, including fields a Core decoder may not know.
    pub payload_digest: Sha256Hex,
    pub envelope_digest: Sha256Hex,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthoredCoordinate {
    pub revision: DecimalU64,
    pub authored_digest: Sha256Hex,
    pub graphs: BTreeMap<GraphId, GraphCoordinate>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AuthoredCommand {
    ProjectMetadata {
        title: String,
        description: String,
    },
    RenameGraph {
        graph_id: GraphId,
        name: String,
    },
    /// Core semantics on existing graph members. `AddNode` requires a future
    /// context/contract creation mapping and is explicitly refused in PS1.
    Graph {
        envelope: Box<GraphCommandEnvelope>,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct MutationRequest {
    pub version: u32,
    pub operation_id: OperationId,
    pub expected: AuthoredCoordinate,
    pub command: AuthoredCommand,
    /// Supplied once, never sampled from a clock by the planner.
    pub updated_at: String,
}
#[derive(Clone, Copy, Debug)]
pub struct CheckpointIds {
    pub write_id: WriteId,
    pub commit_id: CommitId,
}
/// Outer naming is admission policy only, never portable object data.
#[derive(Clone, Debug)]
pub struct PackageNamingPolicy {
    pub write_extension: PackageExtension,
    pub legacy_read_extensions: Vec<PackageExtension>,
}
impl PackageNamingPolicy {
    #[must_use]
    pub fn can_read(&self, extension: &PackageExtension) -> bool {
        extension == &self.write_extension || self.legacy_read_extensions.contains(extension)
    }
    /// # Errors
    /// Even an accepted read alias cannot authorize writes before filename cutover.
    pub fn admit_write(&self, observed: &PackageExtension) -> Result<(), PlanError> {
        if observed != &self.write_extension {
            return Err(PlanError::FilenameCutoverRequired);
        }
        Ok(())
    }
}

/// Prepared semantic receipt only. No journal sequence or durable marker exists.
#[derive(Clone, Debug)]
pub struct PreparedReceipt {
    pub(super) operation_id: OperationId,
    pub(super) request_digest: Sha256Hex,
    pub(super) before: AuthoredCoordinate,
    pub(super) after: AuthoredCoordinate,
}
impl PreparedReceipt {
    #[must_use]
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    #[must_use]
    pub fn request_digest(&self) -> &Sha256Hex {
        &self.request_digest
    }
    #[must_use]
    pub fn before(&self) -> &AuthoredCoordinate {
        &self.before
    }
    #[must_use]
    pub fn after(&self) -> &AuthoredCoordinate {
        &self.after
    }
}
