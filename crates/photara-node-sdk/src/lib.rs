//! Internal package surface shared by built-in and future downloadable nodes.

use std::collections::{BTreeMap, BTreeSet};

use photara_core::{
    DefinitionRegistryError, DefinitionResolver, NodeDefinition, NodeDefinitionError,
    NodeDefinitionId, NodeDefinitionRef, NodeDefinitionRegistry, NodeDefinitionVersion,
    NodePackageId, PackageRequirement, PackageVersion, SchemaVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Namespaced extension carried by an exact node definition's portable metadata.
pub const NODE_PRESENTATION_EXTENSION_KEY: &str = "photara.presentation";

/// Catalog visibility is presentation policy, never a Core evaluator kind.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeCatalogVisibility {
    #[default]
    Visible,
    Hidden,
}

/// Independent brand identity owned by one exact node definition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeBrandMetadata {
    pub name: String,
    /// Neutral package resource identifier resolved by each native client.
    pub icon_resource_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_srgb_hex: Option<String>,
}

/// Immutable presentation contribution advertised by an exact definition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePresentationMetadata {
    pub brand: NodeBrandMetadata,
    #[serde(default)]
    pub catalog_path: Vec<String>,
    #[serde(default)]
    pub search_terms: Vec<String>,
    #[serde(default)]
    pub catalog_visibility: NodeCatalogVisibility,
    /// Neutral contribution identifier rendered inside the generic Inspector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspector_contribution_id: Option<String>,
    /// Neutral contribution identifier for an optional rich authoring Workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_contribution_id: Option<String>,
}

impl NodePresentationMetadata {
    /// Validates client-neutral presentation metadata.
    ///
    /// # Errors
    ///
    /// Returns an error for empty brand/resource/category/contribution fields.
    pub fn validate(&self) -> Result<(), NodePresentationMetadataError> {
        if self.brand.name.trim().is_empty() {
            return Err(NodePresentationMetadataError::EmptyBrandName);
        }
        if self.brand.icon_resource_id.trim().is_empty() {
            return Err(NodePresentationMetadataError::EmptyIconResource);
        }
        if self.catalog_path.iter().any(|part| part.trim().is_empty()) {
            return Err(NodePresentationMetadataError::EmptyCatalogPathComponent);
        }
        if self.search_terms.iter().any(|term| term.trim().is_empty()) {
            return Err(NodePresentationMetadataError::EmptySearchTerm);
        }
        for contribution in [
            self.inspector_contribution_id.as_deref(),
            self.workspace_contribution_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if contribution.trim().is_empty() {
                return Err(NodePresentationMetadataError::EmptyContributionId);
            }
        }
        Ok(())
    }
}

/// Adds typed presentation metadata to an exact definition.
///
/// # Errors
///
/// Returns an error when the metadata is invalid or cannot be serialized.
pub fn set_node_presentation(
    definition: &mut NodeDefinition,
    presentation: NodePresentationMetadata,
) -> Result<(), NodePresentationMetadataError> {
    presentation.validate()?;
    let value = serde_json::to_value(presentation)
        .map_err(|error| NodePresentationMetadataError::Encoding(error.to_string()))?;
    definition
        .extensions
        .insert(NODE_PRESENTATION_EXTENSION_KEY.to_owned(), value);
    Ok(())
}

/// Reads typed presentation metadata without making Core interpret it.
///
/// # Errors
///
/// Returns an error when the namespaced extension is malformed.
pub fn node_presentation(
    definition: &NodeDefinition,
) -> Result<Option<NodePresentationMetadata>, NodePresentationMetadataError> {
    let Some(value) = definition.extensions.get(NODE_PRESENTATION_EXTENSION_KEY) else {
        return Ok(None);
    };
    let presentation: NodePresentationMetadata = serde_json::from_value(value.clone())
        .map_err(|error| NodePresentationMetadataError::Decoding(error.to_string()))?;
    presentation.validate()?;
    Ok(Some(presentation))
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodePresentationMetadataError {
    #[error("node brand name must not be empty")]
    EmptyBrandName,
    #[error("node icon resource identifier must not be empty")]
    EmptyIconResource,
    #[error("node catalog path components must not be empty")]
    EmptyCatalogPathComponent,
    #[error("node search terms must not be empty")]
    EmptySearchTerm,
    #[error("node presentation contribution identifiers must not be empty")]
    EmptyContributionId,
    #[error("could not encode node presentation metadata: {0}")]
    Encoding(String),
    #[error("could not decode node presentation metadata: {0}")]
    Decoding(String),
}

/// Serializable metadata for one exact node-package release.
///
/// This is registration metadata, not an installer or distribution record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePackageManifest {
    pub manifest_schema_version: SchemaVersion,
    pub package_id: NodePackageId,
    pub package_version: PackageVersion,
    pub display_name: String,
    pub definitions: Vec<NodeDefinition>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl NodePackageManifest {
    /// Validates the minimum bundled-package registration contract.
    ///
    /// # Errors
    ///
    /// Returns [`NodePackageManifestError`] when the manifest schema, display
    /// name, definition namespace, or definition identities are invalid.
    pub fn validate(&self) -> Result<(), NodePackageManifestError> {
        if self.manifest_schema_version != SchemaVersion::first() {
            return Err(NodePackageManifestError::UnsupportedSchema(
                self.manifest_schema_version,
            ));
        }
        if self.display_name.trim().is_empty() {
            return Err(NodePackageManifestError::EmptyDisplayName);
        }
        if self.definitions.is_empty() {
            return Err(NodePackageManifestError::NoDefinitions);
        }

        let namespace = format!("{}.", self.package_id);
        let mut definitions = BTreeSet::new();
        for definition in &self.definitions {
            if !definition.id.as_str().starts_with(&namespace) {
                return Err(NodePackageManifestError::DefinitionOutsidePackage {
                    package_id: self.package_id.clone(),
                    definition_id: definition.id.clone(),
                });
            }
            definition
                .validate()
                .map_err(|error| NodePackageManifestError::InvalidDefinition {
                    definition_id: definition.id.clone(),
                    definition_version: definition.version,
                    error,
                })?;
            node_presentation(definition).map_err(|error| {
                NodePackageManifestError::InvalidPresentation {
                    definition_id: definition.id.clone(),
                    definition_version: definition.version,
                    error,
                }
            })?;
            let coordinate = (definition.id.clone(), definition.version);
            if !definitions.insert(coordinate) {
                return Err(NodePackageManifestError::DuplicateDefinition {
                    definition_id: definition.id.clone(),
                    definition_version: definition.version,
                });
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn requirement(&self) -> PackageRequirement {
        PackageRequirement {
            package_id: self.package_id.clone(),
            package_version: self.package_version.clone(),
        }
    }
}

pub trait NodePackage {
    fn manifest(&self) -> NodePackageManifest;
}

/// Exact in-memory lookup assembled from ordinary package manifests.
#[derive(Clone, Debug, Default)]
pub struct NodePackageRegistry {
    manifests: BTreeMap<PackageRequirement, NodePackageManifest>,
    definitions: NodeDefinitionRegistry,
}

impl NodePackageRegistry {
    /// Registers the exact manifest returned by a bundled or external package.
    ///
    /// # Errors
    ///
    /// Returns [`NodePackageRegistryError`] when the manifest is invalid, the
    /// exact package release is already registered, or a definition cannot be
    /// added to the exact Core definition lookup.
    pub fn register_package<P: NodePackage + ?Sized>(
        &mut self,
        package: &P,
    ) -> Result<(), NodePackageRegistryError> {
        self.register_manifest(package.manifest())
    }

    /// Registers a previously persisted exact package manifest.
    ///
    /// # Errors
    ///
    /// Returns [`NodePackageRegistryError`] under the same conditions as
    /// [`Self::register_package`].
    pub fn register_manifest(
        &mut self,
        manifest: NodePackageManifest,
    ) -> Result<(), NodePackageRegistryError> {
        manifest.validate()?;
        let requirement = manifest.requirement();
        if self.manifests.contains_key(&requirement) {
            return Err(NodePackageRegistryError::AlreadyRegistered(requirement));
        }

        for definition in &manifest.definitions {
            self.definitions.register(
                NodeDefinitionRef {
                    package_id: manifest.package_id.clone(),
                    package_version: manifest.package_version.clone(),
                    definition_id: definition.id.clone(),
                    definition_version: definition.version,
                },
                definition.clone(),
            )?;
        }
        self.manifests.insert(requirement, manifest);
        Ok(())
    }

    #[must_use]
    pub fn manifest(&self, requirement: &PackageRequirement) -> Option<&NodePackageManifest> {
        self.manifests.get(requirement)
    }

    /// Iterates installed exact package manifests in stable coordinate order.
    pub fn manifests(&self) -> impl Iterator<Item = &NodePackageManifest> {
        self.manifests.values()
    }
}

impl DefinitionResolver for NodePackageRegistry {
    fn resolve(&self, reference: &NodeDefinitionRef) -> Option<&NodeDefinition> {
        self.definitions.resolve(reference)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodePackageManifestError {
    #[error("unsupported node-package manifest schema version {0:?}")]
    UnsupportedSchema(SchemaVersion),
    #[error("node-package display name must not be empty")]
    EmptyDisplayName,
    #[error("node-package manifest must contain at least one definition")]
    NoDefinitions,
    #[error("definition {definition_id} is outside package namespace {package_id}")]
    DefinitionOutsidePackage {
        package_id: NodePackageId,
        definition_id: NodeDefinitionId,
    },
    #[error("invalid definition {definition_id}@{definition_version:?}: {error}")]
    InvalidDefinition {
        definition_id: NodeDefinitionId,
        definition_version: NodeDefinitionVersion,
        error: NodeDefinitionError,
    },
    #[error("invalid presentation for {definition_id}@{definition_version:?}: {error}")]
    InvalidPresentation {
        definition_id: NodeDefinitionId,
        definition_version: NodeDefinitionVersion,
        error: NodePresentationMetadataError,
    },
    #[error("duplicate definition {definition_id}@{definition_version:?}")]
    DuplicateDefinition {
        definition_id: NodeDefinitionId,
        definition_version: NodeDefinitionVersion,
    },
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NodePackageRegistryError {
    #[error(transparent)]
    InvalidManifest(#[from] NodePackageManifestError),
    #[error("package release {0:?} is already registered")]
    AlreadyRegistered(PackageRequirement),
    #[error(transparent)]
    Definition(Box<DefinitionRegistryError>),
}

impl From<DefinitionRegistryError> for NodePackageRegistryError {
    fn from(error: DefinitionRegistryError) -> Self {
        Self::Definition(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use photara_core::{
        DefinitionResolver, NodeDefinitionId, NodeDefinitionVersion, PortCardinality,
        PortDefinition, PortDirection, PortId, SchemaId, SchemaRef, ValueTypeId, ValueTypeRef,
        ValueTypeVersion,
    };
    use serde_json::json;

    use super::*;

    fn schema(id: &str) -> SchemaRef {
        SchemaRef {
            id: SchemaId::parse(id).unwrap(),
            version: SchemaVersion::first(),
        }
    }

    fn manifest() -> NodePackageManifest {
        NodePackageManifest {
            manifest_schema_version: SchemaVersion::first(),
            package_id: NodePackageId::parse("example.text").unwrap(),
            package_version: PackageVersion::new(1, 2, 3),
            display_name: "Text".to_owned(),
            definitions: vec![NodeDefinition {
                id: NodeDefinitionId::parse("example.text.source").unwrap(),
                version: NodeDefinitionVersion::first(),
                display_name: "Text Source".to_owned(),
                ports: vec![PortDefinition {
                    id: PortId::parse("text").unwrap(),
                    direction: PortDirection::Output,
                    value_type: ValueTypeRef {
                        id: ValueTypeId::parse("example.text.value").unwrap(),
                        version: ValueTypeVersion::first(),
                    },
                    cardinality: PortCardinality::One,
                }],
                config_schema: schema("example.text.source.config"),
                authored_state_schema: None,
                capabilities: BTreeSet::new(),
                extensions: BTreeMap::new(),
            }],
            extensions: BTreeMap::from([("future-field".to_owned(), json!({"kept": true}))]),
        }
    }

    #[test]
    fn manifest_round_trip_preserves_unknown_fields() {
        let manifest = manifest();
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert_eq!(
            serde_json::from_str::<NodePackageManifest>(&json).unwrap(),
            manifest
        );
    }

    #[test]
    fn registry_resolves_only_exact_package_and_definition_coordinates() {
        let manifest = manifest();
        let definition = &manifest.definitions[0];
        let exact = NodeDefinitionRef {
            package_id: manifest.package_id.clone(),
            package_version: manifest.package_version.clone(),
            definition_id: definition.id.clone(),
            definition_version: definition.version,
        };
        let mut registry = NodePackageRegistry::default();
        registry.register_manifest(manifest.clone()).unwrap();

        assert_eq!(registry.resolve(&exact), Some(definition));
        assert_eq!(registry.manifest(&manifest.requirement()), Some(&manifest));
        assert!(
            registry
                .resolve(&NodeDefinitionRef {
                    package_version: PackageVersion::new(1, 2, 4),
                    ..exact
                })
                .is_none()
        );
    }

    #[test]
    fn manifest_rejects_definitions_outside_the_package_namespace() {
        let mut manifest = manifest();
        manifest.definitions[0].id = NodeDefinitionId::parse("other.text.source").unwrap();
        assert!(matches!(
            manifest.validate(),
            Err(NodePackageManifestError::DefinitionOutsidePackage { .. })
        ));
    }
}
