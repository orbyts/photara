//! Internal package surface shared by built-in and future downloadable nodes.

use photara_core::{NodeDefinition, NodePackageId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PackageVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePackageManifest {
    pub schema_version: u32,
    pub package_id: NodePackageId,
    pub package_version: PackageVersion,
    pub display_name: String,
    pub definitions: Vec<NodeDefinition>,
}

pub trait NodePackage {
    fn manifest(&self) -> NodePackageManifest;
}
