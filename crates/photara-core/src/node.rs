use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    CapabilityId, NodeDefinitionId, NodeDefinitionVersion, NodePackageId, PackageVersion, PortId,
    SchemaRef, ValueTypeRef, ValueTypeRegistry,
};

/// The exact package release and definition version pinned by a node instance.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NodeDefinitionRef {
    pub package_id: NodePackageId,
    pub package_version: PackageVersion,
    pub definition_id: NodeDefinitionId,
    pub definition_version: NodeDefinitionVersion,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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
    pub value_type: ValueTypeRef,
    pub cardinality: PortCardinality,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeDefinition {
    pub id: NodeDefinitionId,
    pub version: NodeDefinitionVersion,
    pub display_name: String,
    pub ports: Vec<PortDefinition>,
    pub config_schema: SchemaRef,
    pub authored_state_schema: Option<SchemaRef>,
    pub capabilities: BTreeSet<CapabilityId>,
    /// Namespaced, version-pinned definition metadata ignored by Core semantics.
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
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

    #[must_use]
    pub fn port(&self, id: &PortId) -> Option<&PortDefinition> {
        self.ports.iter().find(|port| port.id == *id)
    }
}

/// Read-only lookup of exact package-release and definition-version pins.
pub trait DefinitionResolver {
    fn resolve(&self, reference: &NodeDefinitionRef) -> Option<&NodeDefinition>;
}

/// An in-memory exact-definition lookup used by Core commands and evaluation.
///
/// Package installation policy and lifecycle remain outside this registry.
#[derive(Clone, Debug, Default)]
pub struct NodeDefinitionRegistry {
    definitions: BTreeMap<NodeDefinitionRef, NodeDefinition>,
}

impl NodeDefinitionRegistry {
    /// Registers one exact package-release and definition-version coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`DefinitionRegistryError`] when the definition is invalid,
    /// disagrees with its lookup key, or is already registered.
    pub fn register(
        &mut self,
        reference: NodeDefinitionRef,
        definition: NodeDefinition,
    ) -> Result<(), DefinitionRegistryError> {
        definition
            .validate()
            .map_err(DefinitionRegistryError::InvalidDefinition)?;
        if reference.definition_id != definition.id
            || reference.definition_version != definition.version
        {
            return Err(DefinitionRegistryError::IdentityMismatch {
                reference,
                definition_id: definition.id,
                definition_version: definition.version,
            });
        }
        if self.definitions.contains_key(&reference) {
            return Err(DefinitionRegistryError::AlreadyRegistered(reference));
        }
        self.definitions.insert(reference, definition);
        Ok(())
    }
}

impl DefinitionResolver for NodeDefinitionRegistry {
    fn resolve(&self, reference: &NodeDefinitionRef) -> Option<&NodeDefinition> {
        self.definitions.get(reference)
    }
}

/// Validates the type and direction of one output-to-input connection.
///
/// Converter-aware compatibility can be added to the registry later without
/// changing node definitions or persisted connection endpoints.
///
/// # Errors
///
/// Returns [`PortCompatibilityError`] when directions are invalid, either type
/// is unregistered, or the exact value-type versions differ.
pub fn validate_port_connection(
    registry: &ValueTypeRegistry,
    output: &PortDefinition,
    input: &PortDefinition,
) -> Result<(), PortCompatibilityError> {
    if output.direction != PortDirection::Output {
        return Err(PortCompatibilityError::SourceIsNotOutput(output.id.clone()));
    }
    if input.direction != PortDirection::Input {
        return Err(PortCompatibilityError::TargetIsNotInput(input.id.clone()));
    }
    if registry.get(&output.value_type).is_none() {
        return Err(PortCompatibilityError::UnknownValueType(
            output.value_type.clone(),
        ));
    }
    if registry.get(&input.value_type).is_none() {
        return Err(PortCompatibilityError::UnknownValueType(
            input.value_type.clone(),
        ));
    }
    if !registry.are_directly_compatible(&output.value_type, &input.value_type) {
        return Err(PortCompatibilityError::IncompatibleTypes {
            output: output.value_type.clone(),
            input: input.value_type.clone(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodeDefinitionError {
    #[error("node definition display name must not be empty")]
    EmptyDisplayName,
    #[error("node definition contains duplicate port {0}")]
    DuplicatePort(PortId),
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "kebab-case")]
pub enum PortCompatibilityError {
    #[error("connection source port {0} is not an output")]
    SourceIsNotOutput(PortId),
    #[error("connection target port {0} is not an input")]
    TargetIsNotInput(PortId),
    #[error("connection references unregistered value type {0:?}")]
    UnknownValueType(ValueTypeRef),
    #[error("output type {output:?} is incompatible with input type {input:?}")]
    IncompatibleTypes {
        output: ValueTypeRef,
        input: ValueTypeRef,
    },
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DefinitionRegistryError {
    #[error("invalid node definition: {0}")]
    InvalidDefinition(NodeDefinitionError),
    #[error(
        "definition key {reference:?} does not match definition {definition_id}@{definition_version:?}"
    )]
    IdentityMismatch {
        reference: NodeDefinitionRef,
        definition_id: NodeDefinitionId,
        definition_version: NodeDefinitionVersion,
    },
    #[error("node definition {0:?} is already registered")]
    AlreadyRegistered(NodeDefinitionRef),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SchemaId, SchemaVersion, ValueTypeDescriptor, ValueTypeId, ValueTypeVersion};

    fn value_type(id: &str, version: u32) -> ValueTypeRef {
        ValueTypeRef {
            id: ValueTypeId::parse(id).unwrap(),
            version: ValueTypeVersion::new(version).unwrap(),
        }
    }

    fn register(registry: &mut ValueTypeRegistry, value_type: ValueTypeRef) {
        registry
            .register(ValueTypeDescriptor {
                display_name: value_type.id.to_string(),
                schema: SchemaRef {
                    id: SchemaId::parse(format!("{}.payload", value_type.id)).unwrap(),
                    version: SchemaVersion::new(1).unwrap(),
                },
                value_type,
            })
            .unwrap();
    }

    fn port(id: &str, direction: PortDirection, value_type: ValueTypeRef) -> PortDefinition {
        PortDefinition {
            id: PortId::parse(id).unwrap(),
            direction,
            value_type,
            cardinality: PortCardinality::One,
        }
    }

    #[test]
    fn identical_registered_value_types_connect() {
        let asset = value_type("example.asset", 1);
        let mut registry = ValueTypeRegistry::default();
        register(&mut registry, asset.clone());

        let output = port("assets", PortDirection::Output, asset.clone());
        let input = port("source", PortDirection::Input, asset);
        assert_eq!(validate_port_connection(&registry, &output, &input), Ok(()));
    }

    #[test]
    fn different_type_versions_do_not_connect_implicitly() {
        let version_one = value_type("example.asset", 1);
        let version_two = value_type("example.asset", 2);
        let mut registry = ValueTypeRegistry::default();
        register(&mut registry, version_one.clone());
        register(&mut registry, version_two.clone());

        let output = port("assets", PortDirection::Output, version_one);
        let input = port("source", PortDirection::Input, version_two);
        assert!(matches!(
            validate_port_connection(&registry, &output, &input),
            Err(PortCompatibilityError::IncompatibleTypes { .. })
        ));
    }
}
