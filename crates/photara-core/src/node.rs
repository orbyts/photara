use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CapabilityId, NodeTypeId, PortId, SchemaId, ValueTypeId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NodeDefinitionVersion(u32);

impl NodeDefinitionVersion {
    /// Creates a nonzero node-definition version.
    ///
    /// # Errors
    ///
    /// Returns [`NodeDefinitionError::ZeroVersion`] when `value` is zero.
    pub fn new(value: u32) -> Result<Self, NodeDefinitionError> {
        if value == 0 {
            return Err(NodeDefinitionError::ZeroVersion);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortCardinality {
    One,
    Optional,
    Many,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortDefinition {
    pub id: PortId,
    pub direction: PortDirection,
    pub value_type: ValueTypeId,
    pub cardinality: PortCardinality,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeDefinition {
    pub type_id: NodeTypeId,
    pub version: NodeDefinitionVersion,
    pub display_name: String,
    pub ports: Vec<PortDefinition>,
    pub config_schema: SchemaId,
    pub authored_state_schema: Option<SchemaId>,
    pub capabilities: BTreeSet<CapabilityId>,
}

impl NodeDefinition {
    /// Validates the definition's user-facing name and port identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`NodeDefinitionError`] when the display name is empty or two
    /// ports use the same identifier.
    pub fn validate(&self) -> Result<(), NodeDefinitionError> {
        if self.display_name.trim().is_empty() {
            return Err(NodeDefinitionError::EmptyDisplayName);
        }
        let mut ports = BTreeSet::new();
        for port in &self.ports {
            if !ports.insert(&port.id) {
                return Err(NodeDefinitionError::DuplicatePort(port.id.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodeDefinitionError {
    #[error("node definition version must be greater than zero")]
    ZeroVersion,
    #[error("node definition display name must not be empty")]
    EmptyDisplayName,
    #[error("node definition contains duplicate port {0}")]
    DuplicatePort(PortId),
}
