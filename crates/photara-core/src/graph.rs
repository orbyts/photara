use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{GraphId, NodeDefinitionVersion, NodeInstanceId, NodeTypeId};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct GraphRevision(u64);

impl GraphRevision {
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeInstance {
    pub id: NodeInstanceId,
    pub definition: NodeTypeId,
    pub definition_version: NodeDefinitionVersion,
    pub configuration: Value,
    #[serde(default)]
    pub authored_state: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GraphDocument {
    pub schema_version: u32,
    pub id: GraphId,
    pub revision: GraphRevision,
    pub nodes: Vec<NodeInstance>,
}

impl GraphDocument {
    #[must_use]
    pub fn new(id: GraphId) -> Self {
        Self {
            schema_version: 1,
            id,
            revision: GraphRevision::initial(),
            nodes: Vec::new(),
        }
    }
}
