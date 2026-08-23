//! Explicit, portable `AssetSet` source node.
//!
//! Asset membership is authored graph state. Asset metadata and representation
//! bindings remain owned by the surrounding project's Asset Context.

use std::collections::{BTreeMap, BTreeSet};

use photara_core::{
    ASSET_SET_SCHEMA_ID, AssetSet, CanonicalDigest, NodeDefinition, NodeDefinitionId,
    NodeDefinitionRef, NodeDefinitionVersion, NodeEvaluationOutput, NodeEvaluationRequest,
    NodeExecutionError, NodePackageId, NodeRuntime, PackageVersion, PortCardinality,
    PortDefinition, PortDirection, PortId, SchemaId, SchemaRef, SchemaVersion,
    asset_set_value_type_ref, canonical_digest,
};
use photara_node_sdk::{
    NodeBrandMetadata, NodeCatalogVisibility, NodePackage, NodePackageManifest,
    NodePresentationMetadata, set_node_presentation,
};

pub const PACKAGE_ID: &str = "photara.asset-set-source";
pub const DEFINITION_ID: &str = "photara.asset-set-source.project-assets";

#[derive(Clone, Copy, Debug, Default)]
pub struct AssetSetNodePackage;

#[derive(Clone, Copy, Debug, Default)]
pub struct AssetSetNodeRuntime;

#[must_use]
/// Returns the portable schema used for explicit `AssetSet` authored state.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in schema ID is invalid.
pub fn asset_set_state_schema() -> SchemaRef {
    SchemaRef {
        id: SchemaId::parse(ASSET_SET_SCHEMA_ID).expect("built-in schema ID is valid"),
        version: SchemaVersion::first(),
    }
}

impl NodePackage for AssetSetNodePackage {
    fn manifest(&self) -> NodePackageManifest {
        let mut definition = NodeDefinition {
            id: NodeDefinitionId::parse(DEFINITION_ID).expect("built-in definition ID is valid"),
            version: NodeDefinitionVersion::new(1).expect("built-in version is valid"),
            display_name: "Project Assets".to_owned(),
            ports: vec![PortDefinition {
                id: PortId::parse("assets").expect("built-in port ID is valid"),
                direction: PortDirection::Output,
                value_type: asset_set_value_type_ref(),
                cardinality: PortCardinality::One,
            }],
            config_schema: SchemaRef {
                id: SchemaId::parse("photara.asset-set-source.config")
                    .expect("built-in schema ID is valid"),
                version: SchemaVersion::first(),
            },
            authored_state_schema: Some(asset_set_state_schema()),
            capabilities: BTreeSet::new(),
            extensions: BTreeMap::new(),
        };
        set_node_presentation(
            &mut definition,
            NodePresentationMetadata {
                brand: NodeBrandMetadata {
                    name: "Project Assets".to_owned(),
                    icon_resource_id: "photara.project.assets".to_owned(),
                    theme_color_role: Some("node.native".to_owned()),
                    accent_srgb_hex: Some("#58BE78".to_owned()),
                },
                catalog_path: vec!["Input".to_owned(), "Project".to_owned()],
                search_terms: vec!["assets".to_owned(), "project".to_owned()],
                catalog_visibility: NodeCatalogVisibility::Hidden,
                inspector_contribution_id: Some("photara.asset-set.inspector".to_owned()),
                workspace_contribution_id: None,
                default_activation_id: None,
            },
        )
        .expect("built-in presentation metadata is valid");
        NodePackageManifest {
            manifest_schema_version: SchemaVersion::first(),
            package_id: NodePackageId::parse(PACKAGE_ID).expect("built-in package ID is valid"),
            package_version: PackageVersion::new(0, 2, 0),
            display_name: "Project Assets".to_owned(),
            definitions: vec![definition],
            extensions: BTreeMap::new(),
        }
    }
}

impl NodeRuntime for AssetSetNodeRuntime {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<CanonicalDigest> {
        is_asset_set_definition(definition).then(|| {
            canonical_digest(&(DEFINITION_ID, 1_u32, "photara.asset-set-source-runtime-v1"))
                .expect("built-in implementation fingerprint is canonical")
        })
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError> {
        if !is_asset_set_definition(&request.node.definition) {
            return Err(execution_error(
                "photara.asset-set-source.wrong-definition",
                "AssetSet runtime received a different node definition",
            ));
        }
        if request.cancellation.is_cancelled() {
            return Err(execution_error(
                "photara.asset-set-source.cancelled",
                "AssetSet evaluation was cancelled",
            ));
        }
        let state = request.node.authored_state.as_ref().ok_or_else(|| {
            execution_error(
                "photara.asset-set-source.missing-authored-state",
                "AssetSet source has no authored state",
            )
        })?;
        if state.schema != asset_set_state_schema() {
            return Err(execution_error(
                "photara.asset-set-source.wrong-schema",
                "AssetSet source authored state uses the wrong schema",
            ));
        }
        let assets: AssetSet = serde_json::from_value(state.value.clone()).map_err(|error| {
            execution_error("photara.asset-set-source.invalid-state", error.to_string())
        })?;
        let output = assets.to_typed_value().map_err(|error| {
            execution_error(
                "photara.asset-set-source.output-serialization",
                error.to_string(),
            )
        })?;
        Ok(NodeEvaluationOutput {
            outputs: BTreeMap::from([(
                PortId::parse("assets").expect("built-in port ID is valid"),
                vec![output],
            )]),
        })
    }
}

fn is_asset_set_definition(definition: &NodeDefinitionRef) -> bool {
    definition.package_id.as_str() == PACKAGE_ID
        && definition.package_version == PackageVersion::new(0, 2, 0)
        && definition.definition_id.as_str() == DEFINITION_ID
        && definition.definition_version.get() == 1
}

fn execution_error(code: impl Into<String>, message: impl Into<String>) -> NodeExecutionError {
    NodeExecutionError {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use photara_core::{CancellationToken, NodeInstance, NodeInstanceId, SchemaValue};
    use photara_node_sdk::NodePackage;
    use serde_json::json;

    use super::*;

    #[test]
    fn manifest_and_runtime_expose_explicit_asset_set() {
        let manifest = AssetSetNodePackage.manifest();
        manifest.validate().unwrap();
        let definition = &manifest.definitions[0];
        let reference = NodeDefinitionRef {
            package_id: manifest.package_id.clone(),
            package_version: manifest.package_version.clone(),
            definition_id: definition.id.clone(),
            definition_version: definition.version,
        };
        let output = AssetSetNodeRuntime
            .evaluate(NodeEvaluationRequest {
                request_id: photara_core::RequestId::new(),
                evaluation_id: photara_core::EvaluationId::new(),
                evaluation_key: canonical_digest(&"asset-set-test").unwrap(),
                node: NodeInstance {
                    id: NodeInstanceId::new(),
                    definition: reference,
                    configuration: SchemaValue {
                        schema: definition.config_schema.clone(),
                        value: json!({}),
                    },
                    authored_state: Some(SchemaValue {
                        schema: asset_set_state_schema(),
                        value: json!({"assets": []}),
                    }),
                    extensions: BTreeMap::new(),
                },
                inputs: BTreeMap::new(),
                cancellation: CancellationToken::default(),
            })
            .unwrap();
        assert!(
            output
                .outputs
                .contains_key(&PortId::parse("assets").unwrap())
        );
    }
}
