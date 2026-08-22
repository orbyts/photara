//! Independently namespaced Layout node package shipped with the application.

mod model;
mod runtime;

pub use model::{
    BundledCanvasProfile, CellArrangement, CellContentMode, FrameDecoration, LayoutCanvas,
    LayoutCell, LayoutCellId, LayoutColor, LayoutCommand, LayoutCommandError, LayoutCommandResult,
    LayoutFrame, LayoutFrameId, LayoutPlan, LayoutProxySet, LayoutState, LayoutStateCodecError,
    LayoutValidationError, NormalizedInsets, NormalizedPoint, NormalizedRect, NormalizedUnit,
    PixelRect, PixelSize, QuarterTurn, ResolvedCell, ResolvedFrame, apply_layout_command,
    layout_plan_value_type_descriptor, layout_plan_value_type_ref, layout_state_schema,
    request_layout_proxies, resolve_layout,
};
pub use runtime::LayoutNodeRuntime;

use std::collections::{BTreeMap, BTreeSet};

use photara_core::{
    NodeDefinition, NodeDefinitionId, NodeDefinitionVersion, NodePackageId, PackageVersion,
    PortCardinality, PortDefinition, PortDirection, PortId, SchemaId, SchemaRef, SchemaVersion,
    ValueTypeId, ValueTypeRef, ValueTypeVersion, asset_set_value_type_ref,
};
use photara_node_sdk::{
    NodeBrandMetadata, NodeCatalogVisibility, NodePackage, NodePackageManifest,
    NodePresentationMetadata, set_node_presentation,
};

pub const PACKAGE_ID: &str = "photara.layout";
pub const DEFINITION_ID: &str = "photara.layout.compose";
pub const ASSET_SET_TYPE_ID: &str = photara_core::ASSET_SET_VALUE_TYPE_ID;
pub const LAYOUT_PLAN_TYPE_ID: &str = "photara.layout-plan";

#[cfg(test)]
mod stage7_tests;

#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutNodePackage;

fn schema(id: &str) -> SchemaRef {
    SchemaRef {
        id: SchemaId::parse(id).expect("built-in schema ID is valid"),
        version: SchemaVersion::new(1).expect("built-in schema version is valid"),
    }
}

fn value_type(id: &str) -> ValueTypeRef {
    ValueTypeRef {
        id: ValueTypeId::parse(id).expect("built-in value type ID is valid"),
        version: ValueTypeVersion::new(1).expect("built-in value type version is valid"),
    }
}

impl NodePackage for LayoutNodePackage {
    fn manifest(&self) -> NodePackageManifest {
        let mut definition = NodeDefinition {
            id: NodeDefinitionId::parse(DEFINITION_ID).expect("built-in definition ID is valid"),
            version: NodeDefinitionVersion::new(1).expect("built-in version is valid"),
            display_name: "Layout".to_owned(),
            ports: vec![
                PortDefinition {
                    id: PortId::parse("assets").expect("built-in port ID is valid"),
                    direction: PortDirection::Input,
                    value_type: asset_set_value_type_ref(),
                    cardinality: PortCardinality::One,
                },
                PortDefinition {
                    id: PortId::parse("layout").expect("built-in port ID is valid"),
                    direction: PortDirection::Output,
                    value_type: value_type(LAYOUT_PLAN_TYPE_ID),
                    cardinality: PortCardinality::One,
                },
            ],
            config_schema: schema("photara.layout.config"),
            authored_state_schema: Some(layout_state_schema()),
            capabilities: BTreeSet::new(),
            extensions: BTreeMap::new(),
        };
        set_node_presentation(
            &mut definition,
            NodePresentationMetadata {
                brand: NodeBrandMetadata {
                    name: "Layout".to_owned(),
                    icon_resource_id: "photara.layout.compose".to_owned(),
                    accent_srgb_hex: Some("#9A68E8".to_owned()),
                },
                catalog_path: vec!["Create".to_owned(), "Layout".to_owned()],
                search_terms: vec![
                    "layout".to_owned(),
                    "frames".to_owned(),
                    "contact sheet".to_owned(),
                ],
                catalog_visibility: NodeCatalogVisibility::Visible,
                inspector_contribution_id: Some("photara.layout.inspector".to_owned()),
                workspace_contribution_id: Some("photara.layout.workspace".to_owned()),
            },
        )
        .expect("built-in presentation metadata is valid");
        definition.validate().expect("built-in definition is valid");

        NodePackageManifest {
            manifest_schema_version: SchemaVersion::new(1)
                .expect("manifest schema version is valid"),
            package_id: NodePackageId::parse(PACKAGE_ID).expect("built-in package ID is valid"),
            package_version: PackageVersion::new(0, 2, 0),
            display_name: "Layout".to_owned(),
            definitions: vec![definition],
            extensions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::PathBuf};

    use photara_core::{
        CommandId, DefinitionResolver, GraphCommand, GraphCommandEnvelope, GraphDocument, GraphId,
        NodeDefinitionRef, NodeInstance, NodeInstanceId, ProjectDocument, ProjectId,
        ProjectRevision, SchemaValue, ValueTypeRegistry, apply_graph_command,
    };
    use photara_node_sdk::NodePackageRegistry;
    use photara_store::{
        FileSystemStateStore, PackageManifestRepository, ProjectRepository, StoreError,
    };
    use serde_json::json;

    use super::*;

    fn dummy_definition() -> NodeDefinition {
        NodeDefinition {
            id: NodeDefinitionId::parse("example.text.uppercase").unwrap(),
            version: NodeDefinitionVersion::new(7).unwrap(),
            display_name: "Uppercase".to_owned(),
            ports: vec![PortDefinition {
                id: PortId::parse("text").unwrap(),
                direction: PortDirection::Input,
                value_type: value_type("example.text"),
                cardinality: PortCardinality::Optional,
            }],
            config_schema: schema("example.text.uppercase.config"),
            authored_state_schema: None,
            capabilities: BTreeSet::new(),
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn built_in_layout_uses_the_same_definition_contract_as_an_unrelated_node() {
        let manifest = LayoutNodePackage.manifest();
        let dummy = dummy_definition();

        assert_eq!(manifest.package_id.as_str(), PACKAGE_ID);
        assert_eq!(manifest.package_version.to_string(), "0.2.0");
        assert_eq!(manifest.definitions.len(), 1);
        assert_eq!(manifest.definitions[0].id.as_str(), DEFINITION_ID);
        assert_eq!(
            manifest.definitions[0].ports[0].value_type,
            asset_set_value_type_ref()
        );
        manifest.definitions[0].validate().unwrap();
        dummy.validate().unwrap();
        assert_ne!(manifest.definitions[0].id, dummy.id);
    }

    #[test]
    fn layout_round_trips_through_generic_project_and_graph_documents() {
        let manifest = LayoutNodePackage.manifest();
        let definition = &manifest.definitions[0];
        let instance = NodeInstance {
            id: NodeInstanceId::new(),
            definition: NodeDefinitionRef {
                package_id: manifest.package_id.clone(),
                package_version: manifest.package_version.clone(),
                definition_id: definition.id.clone(),
                definition_version: definition.version,
            },
            configuration: SchemaValue {
                schema: definition.config_schema.clone(),
                value: json!({"canvas-profile": "portrait-3x4"}),
            },
            authored_state: Some(SchemaValue {
                schema: definition.authored_state_schema.clone().unwrap(),
                value: json!({
                    "frames": [{
                        "cells": [{
                            "fit": "crop",
                            "quarter-turns": 1,
                            "future-layout-field": {"preserved": true}
                        }]
                    }]
                }),
            }),
            extensions: BTreeMap::new(),
        };
        let mut graph = GraphDocument::new(GraphId::new());
        graph.nodes.push(instance);
        let project = ProjectDocument::new(ProjectId::new(), "Layout Project", graph).unwrap();

        let project_json = project.to_pretty_json().unwrap();
        let reopened = ProjectDocument::from_json(&project_json).unwrap();
        assert_eq!(reopened, project);
        assert_eq!(
            reopened.required_packages[0].package_id.as_str(),
            PACKAGE_ID
        );
        assert_eq!(
            reopened.graph.nodes[0].authored_state,
            project.graph.nodes[0].authored_state
        );

        let shared = project.export_node_graph("Portrait Layout").unwrap();
        let shared_json = shared.to_pretty_json().unwrap();
        assert_eq!(
            photara_core::NodeGraphDocument::from_json(&shared_json).unwrap(),
            shared
        );
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("photara-layout-stage-4a-{}", ProjectId::new())))
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn ordinary_layout_registration_and_authored_state_survive_durable_reopen() {
        let root = TestRoot::new();
        let manifest = LayoutNodePackage.manifest();
        let definition = &manifest.definitions[0];
        let node_id = NodeInstanceId::new();
        let instance = NodeInstance {
            id: node_id,
            definition: NodeDefinitionRef {
                package_id: manifest.package_id.clone(),
                package_version: manifest.package_version.clone(),
                definition_id: definition.id.clone(),
                definition_version: definition.version,
            },
            configuration: SchemaValue {
                schema: definition.config_schema.clone(),
                value: json!({"canvas-profile": "portrait-3x4"}),
            },
            authored_state: Some(SchemaValue {
                schema: definition.authored_state_schema.clone().unwrap(),
                value: json!({"frames": [{"cells": [{"fit": "contain"}]}]}),
            }),
            extensions: BTreeMap::new(),
        };

        let mut registry = NodePackageRegistry::default();
        registry.register_package(&LayoutNodePackage).unwrap();
        assert_eq!(registry.resolve(&instance.definition), Some(definition));

        let mut graph = GraphDocument::new(GraphId::new());
        graph.nodes.push(instance.clone());
        let project_id = ProjectId::new();
        let project = ProjectDocument::new(project_id, "Persistent Layout", graph).unwrap();

        let mut store = FileSystemStateStore::open(&root.0).unwrap();
        store.register_manifest(manifest.clone()).unwrap();
        store.create_project(project.clone()).unwrap();
        drop(store);

        let mut reopened_store = FileSystemStateStore::open(&root.0).unwrap();
        let persisted_manifest = reopened_store
            .load_manifest(&manifest.requirement())
            .unwrap()
            .unwrap();
        let mut reopened_registry = NodePackageRegistry::default();
        reopened_registry
            .register_manifest(persisted_manifest)
            .unwrap();
        let reopened = reopened_store.load_project(project_id).unwrap().unwrap();
        assert_eq!(reopened, project);
        assert_eq!(
            reopened_registry.resolve(&reopened.graph.nodes[0].definition),
            Some(definition)
        );
        assert_eq!(reopened.graph.nodes[0].definition, instance.definition);
        assert_eq!(
            reopened.graph.nodes[0].authored_state,
            instance.authored_state
        );

        let new_authored_state = SchemaValue {
            schema: definition.authored_state_schema.clone().unwrap(),
            value: json!({
                "frames": [{"cells": [{"fit": "crop", "quarter-turns": 1}]}],
                "future-layout-field": {"preserved": true}
            }),
        };
        let command_result = apply_graph_command(
            &reopened.graph,
            &GraphCommandEnvelope {
                command_id: CommandId::new(),
                graph_id: reopened.graph.id,
                expected_revision: reopened.graph.revision,
                command: GraphCommand::SetAuthoredState {
                    node_id,
                    authored_state: Some(new_authored_state.clone()),
                },
            },
            &reopened_registry,
            &ValueTypeRegistry::default(),
        )
        .unwrap();
        let mut replacement = reopened;
        replacement.graph = command_result.graph;
        replacement.revision = replacement.revision.checked_next().unwrap();
        reopened_store
            .replace_project(replacement.clone(), ProjectRevision::initial())
            .unwrap();
        drop(reopened_store);

        let mut final_store = FileSystemStateStore::open(&root.0).unwrap();
        let final_project = final_store.load_project(project_id).unwrap().unwrap();
        assert_eq!(
            final_project.graph.nodes[0].authored_state,
            Some(new_authored_state)
        );
        assert!(matches!(
            final_store.replace_project(project, ProjectRevision::initial()),
            Err(StoreError::ProjectRevisionConflict { .. })
        ));
    }
}
