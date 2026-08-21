//! Portable, UI-independent semantic core for the generation-two application.

mod diagnostic;
mod graph;
mod identity;
mod node;

pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use graph::{GraphDocument, GraphRevision, NodeInstance};
pub use identity::{
    CanonicalIdError, CapabilityId, GraphId, NodeInstanceId, NodePackageId, NodeTypeId, PortId,
    SchemaId, ValueTypeId,
};
pub use node::{
    NodeDefinition, NodeDefinitionVersion, PortCardinality, PortDefinition, PortDirection,
};

/// Current version of the semantic application facade exposed to clients.
pub const APPLICATION_API_VERSION: u32 = 1;
