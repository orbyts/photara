//! Portable, UI-independent semantic core for the generation-two application.

mod asset;
mod canonical;
mod command;
mod diagnostic;
mod evaluation;
mod graph;
mod identity;
mod node;
mod project;
mod value;

pub use asset::{
    ASSET_SET_SCHEMA_ID, ASSET_SET_VALUE_TYPE_ID, AssetContextError, AssetSet, AssetSetValueError,
    FLATTENED_IMAGE_CAPABILITY_ID, FingerprintAlgorithm, HDR_CAPABILITY_ID,
    HDR_REPRESENTATION_ROLE_ID, IMAGE_CAPABILITY_ID, MaterializedRepresentation, ProjectAsset,
    ProjectAssetContext, RepresentationAvailability, RepresentationBinding,
    RepresentationDescriptor, RepresentationFingerprint, RepresentationMaterializationError,
    RepresentationMaterializationRequest, RepresentationMaterializer, SDR_CAPABILITY_ID,
    SDR_REPRESENTATION_ROLE_ID, TIFF_CAPABILITY_ID, asset_set_value_type_descriptor,
    asset_set_value_type_ref,
};
pub use canonical::{CanonicalDigest, canonical_digest, canonical_json};
pub use command::{
    GraphCommand, GraphCommandEnvelope, GraphCommandError, GraphCommandResult, apply_graph_command,
};
pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use evaluation::{
    CancellationToken, EvaluationError, EvaluationOutcome, EvaluationPhase, EvaluationProgress,
    EvaluationRequest, NodeEvaluationOutput, NodeEvaluationRecord, NodeEvaluationRequest,
    NodeExecutionError, NodeRuntime, evaluate_graph,
};
pub use graph::{Connection, GraphDocument, GraphRevision, NodeInstance, PortEndpoint};
pub use identity::{
    AssetId, AssetRepresentationId, CanonicalIdError, CapabilityId, CommandId, ConnectionId,
    EvaluationId, GraphId, NodeDefinitionId, NodeDefinitionVersion, NodeInstanceId, NodePackageId,
    PackageVersion, PortId, ProjectId, ProjectResourceId, RepresentationCapabilityId,
    RepresentationRoleId, RequestId, SchemaId, SchemaVersion, ValueTypeId, ValueTypeVersion,
    VersionError,
};
pub use node::{
    DefinitionRegistryError, DefinitionResolver, NodeDefinition, NodeDefinitionError,
    NodeDefinitionRef, NodeDefinitionRegistry, PortCardinality, PortCompatibilityError,
    PortDefinition, PortDirection, validate_port_connection,
};
pub use project::{
    NodeGraphDocument, NodeGraphMetadata, PackageRequirement, PortableDocumentError,
    ProjectDocument, ProjectMetadata, ProjectRelativePath, ProjectRelativePathError,
    ProjectResourceRef, ProjectRevision, ProjectValidationError,
};
pub use value::{
    SchemaRef, SchemaValue, TypedValue, ValueTypeDescriptor, ValueTypeRef, ValueTypeRegistry,
    ValueTypeRegistryError,
};

/// Current version of the semantic application facade exposed to clients.
pub const APPLICATION_API_VERSION: u32 = 1;

#[cfg(test)]
mod vertical_tests;
