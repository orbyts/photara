//! Ordinary built-in Disk node with portable accepted membership.
//!
//! Folder paths and platform permission tokens are runtime bindings held by the
//! host. Evaluation performs no filesystem I/O and emits the last explicitly
//! accepted `AssetSet`.

use std::collections::{BTreeMap, BTreeSet};

use photara_core::{
    AssetSet, CanonicalDigest, NodeDefinition, NodeDefinitionId, NodeDefinitionRef,
    NodeDefinitionVersion, NodeEvaluationOutput, NodeEvaluationRequest, NodeExecutionError,
    NodePackageId, NodeRuntime, PackageVersion, PortCardinality, PortDefinition, PortDirection,
    PortId, SchemaId, SchemaRef, SchemaValue, SchemaVersion, asset_set_value_type_ref,
    canonical_digest,
};
use photara_node_sdk::{
    NodeBrandMetadata, NodeCatalogVisibility, NodePackage, NodePackageManifest,
    NodePresentationMetadata, set_node_presentation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

mod provider;

pub use provider::{DiskFolderProvider, DiskReconciliation, DiskRevisionMode};

pub const PACKAGE_ID: &str = "photara.disk";
pub const DEFINITION_ID: &str = "photara.disk.folder";
pub const STATE_SCHEMA_ID: &str = "photara.disk.folder.state";

#[derive(Clone, Copy, Debug, Default)]
pub struct DiskNodePackage;

#[derive(Clone, Copy, Debug, Default)]
pub struct DiskNodeRuntime;

/// Portable authored state. The binding ID is safe to share; its locator is not.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiskFolderState {
    pub folder_binding_id: Uuid,
    pub recursive: bool,
    pub accepted_assets: AssetSet,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl Default for DiskFolderState {
    fn default() -> Self {
        Self {
            folder_binding_id: Uuid::new_v4(),
            recursive: false,
            accepted_assets: AssetSet::default(),
            extensions: BTreeMap::new(),
        }
    }
}

impl DiskFolderState {
    /// Encodes portable state without paths or permission material.
    ///
    /// # Errors
    ///
    /// Returns a JSON encoding error if the state cannot be represented.
    pub fn to_schema_value(&self) -> Result<SchemaValue, serde_json::Error> {
        Ok(SchemaValue {
            schema: disk_state_schema(),
            value: serde_json::to_value(self)?,
        })
    }

    /// Decodes the exact Disk authored-state schema.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong schema or malformed state payload.
    pub fn from_schema_value(value: &SchemaValue) -> Result<Self, String> {
        if value.schema != disk_state_schema() {
            return Err(format!("unexpected Disk state schema {:?}", value.schema));
        }
        serde_json::from_value(value.value.clone()).map_err(|error| error.to_string())
    }
}

#[must_use]
/// Returns the exact portable Disk authored-state schema.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in schema ID is invalid.
pub fn disk_state_schema() -> SchemaRef {
    SchemaRef {
        id: SchemaId::parse(STATE_SCHEMA_ID).expect("built-in schema ID is valid"),
        version: SchemaVersion::first(),
    }
}

impl NodePackage for DiskNodePackage {
    fn manifest(&self) -> NodePackageManifest {
        let mut definition = NodeDefinition {
            id: NodeDefinitionId::parse(DEFINITION_ID).expect("built-in definition ID is valid"),
            version: NodeDefinitionVersion::first(),
            display_name: "Disk".to_owned(),
            ports: vec![PortDefinition {
                id: PortId::parse("assets").expect("built-in port ID is valid"),
                direction: PortDirection::Output,
                value_type: asset_set_value_type_ref(),
                cardinality: PortCardinality::One,
            }],
            config_schema: SchemaRef {
                id: SchemaId::parse("photara.disk.folder.config")
                    .expect("built-in schema ID is valid"),
                version: SchemaVersion::first(),
            },
            authored_state_schema: Some(disk_state_schema()),
            capabilities: BTreeSet::new(),
            extensions: BTreeMap::new(),
        };
        set_node_presentation(
            &mut definition,
            NodePresentationMetadata {
                brand: NodeBrandMetadata {
                    name: "Disk".to_owned(),
                    icon_resource_id: "photara.disk.folder".to_owned(),
                    accent_srgb_hex: Some("#5FBF73".to_owned()),
                },
                catalog_path: vec!["Input".to_owned(), "Filesystem".to_owned()],
                search_terms: vec![
                    "disk".to_owned(),
                    "folder".to_owned(),
                    "files".to_owned(),
                    "assets".to_owned(),
                ],
                catalog_visibility: NodeCatalogVisibility::Visible,
                inspector_contribution_id: Some("photara.disk.inspector".to_owned()),
                workspace_contribution_id: None,
                default_activation_id: Some("photara.disk.open-folder".to_owned()),
            },
        )
        .expect("built-in presentation metadata is valid");
        NodePackageManifest {
            manifest_schema_version: SchemaVersion::first(),
            package_id: NodePackageId::parse(PACKAGE_ID).expect("built-in package ID is valid"),
            package_version: PackageVersion::new(0, 2, 0),
            display_name: "Disk".to_owned(),
            definitions: vec![definition],
            extensions: BTreeMap::new(),
        }
    }
}

impl NodeRuntime for DiskNodeRuntime {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<CanonicalDigest> {
        is_disk_definition(definition).then(|| {
            canonical_digest(&(DEFINITION_ID, 1_u32, "photara-disk-runtime-v1"))
                .expect("built-in fingerprint is canonical")
        })
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError> {
        if !is_disk_definition(&request.node.definition) {
            return Err(execution_error(
                "photara.disk.wrong-definition",
                "Disk runtime received another definition",
            ));
        }
        if request.cancellation.is_cancelled() {
            return Err(execution_error(
                "photara.disk.cancelled",
                "Disk evaluation was cancelled",
            ));
        }
        let state = request.node.authored_state.as_ref().ok_or_else(|| {
            execution_error("photara.disk.missing-state", "Disk has no authored state")
        })?;
        let state = DiskFolderState::from_schema_value(state)
            .map_err(|message| execution_error("photara.disk.invalid-state", &message))?;
        Ok(NodeEvaluationOutput {
            outputs: BTreeMap::from([(
                PortId::parse("assets").expect("built-in port ID is valid"),
                vec![state.accepted_assets.to_typed_value().map_err(|error| {
                    execution_error("photara.disk.invalid-output", &error.to_string())
                })?],
            )]),
        })
    }
}

fn is_disk_definition(definition: &NodeDefinitionRef) -> bool {
    definition.package_id.as_str() == PACKAGE_ID
        && definition.package_version == PackageVersion::new(0, 2, 0)
        && definition.definition_id.as_str() == DEFINITION_ID
        && definition.definition_version == NodeDefinitionVersion::first()
}

fn execution_error(code: &str, message: &str) -> NodeExecutionError {
    NodeExecutionError {
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photara_node_sdk::{NodePackage, node_presentation};

    #[test]
    fn definition_owns_visible_brand_and_no_workspace() {
        let manifest = DiskNodePackage.manifest();
        manifest.validate().unwrap();
        let presentation = node_presentation(&manifest.definitions[0])
            .unwrap()
            .unwrap();
        assert_eq!(presentation.catalog_path, ["Input", "Filesystem"]);
        assert_eq!(presentation.brand.icon_resource_id, "photara.disk.folder");
        assert!(presentation.workspace_contribution_id.is_none());
        assert_eq!(
            presentation.default_activation_id.as_deref(),
            Some("photara.disk.open-folder")
        );
    }

    #[test]
    fn portable_state_contains_identity_and_membership_but_no_locator() {
        let state = DiskFolderState::default();
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("folder_binding_id"));
        assert!(!json.contains("path"));
        assert!(!json.contains("bookmark"));
        assert_eq!(
            DiskFolderState::from_schema_value(&state.to_schema_value().unwrap()).unwrap(),
            state
        );
    }
}
