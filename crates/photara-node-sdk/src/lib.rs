//! Internal package surface shared by built-in and future downloadable nodes.

use photara_core::{NodeDefinition, NodePackageId, PackageVersion, SchemaVersion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePackageManifest {
    pub manifest_schema_version: SchemaVersion,
    pub package_id: NodePackageId,
    pub package_version: PackageVersion,
    pub display_name: String,
    pub definitions: Vec<NodeDefinition>,
}

pub trait NodePackage {
    fn manifest(&self) -> NodePackageManifest;
}
