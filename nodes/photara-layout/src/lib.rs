//! Independently namespaced Layout node package shipped with the application.

use std::collections::BTreeSet;

use photara_core::{
    NodeDefinition, NodeDefinitionVersion, NodePackageId, NodeTypeId, PortCardinality,
    PortDefinition, PortDirection, PortId, SchemaId, ValueTypeId,
};
use photara_node_sdk::{NodePackage, NodePackageManifest, PackageVersion};

pub const PACKAGE_ID: &str = "photara.layout";
pub const DEFINITION_ID: &str = "photara.layout.compose";

#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutNodePackage;

impl NodePackage for LayoutNodePackage {
    fn manifest(&self) -> NodePackageManifest {
        let definition = NodeDefinition {
            type_id: NodeTypeId::parse(DEFINITION_ID).expect("built-in definition ID is valid"),
            version: NodeDefinitionVersion::new(1).expect("built-in version is valid"),
            display_name: "Layout".to_owned(),
            ports: vec![
                PortDefinition {
                    id: PortId::parse("assets").expect("built-in port ID is valid"),
                    direction: PortDirection::Input,
                    value_type: ValueTypeId::parse("photara.asset-set/v1")
                        .expect("built-in value type is valid"),
                    cardinality: PortCardinality::One,
                },
                PortDefinition {
                    id: PortId::parse("layout").expect("built-in port ID is valid"),
                    direction: PortDirection::Output,
                    value_type: ValueTypeId::parse("photara.layout-plan/v1")
                        .expect("built-in value type is valid"),
                    cardinality: PortCardinality::One,
                },
            ],
            config_schema: SchemaId::parse("photara.layout.config/v1")
                .expect("built-in schema ID is valid"),
            authored_state_schema: Some(
                SchemaId::parse("photara.layout.state/v1").expect("built-in schema ID is valid"),
            ),
            capabilities: BTreeSet::new(),
        };
        definition.validate().expect("built-in definition is valid");

        NodePackageManifest {
            schema_version: 1,
            package_id: NodePackageId::parse(PACKAGE_ID).expect("built-in package ID is valid"),
            package_version: PackageVersion {
                major: 0,
                minor: 2,
                patch: 0,
            },
            display_name: "Layout".to_owned(),
            definitions: vec![definition],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_layout_uses_the_ordinary_package_contract() {
        let manifest = LayoutNodePackage.manifest();
        assert_eq!(manifest.package_id.as_str(), PACKAGE_ID);
        assert_eq!(manifest.definitions.len(), 1);
        assert_eq!(manifest.definitions[0].type_id.as_str(), DEFINITION_ID);
        manifest.definitions[0].validate().unwrap();
    }
}
