use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConnectionId, GraphId, NodeDefinitionRef, NodeInstanceId, PortId, SchemaValue, SchemaVersion,
};

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
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PortEndpoint {
    pub node_id: NodeInstanceId,
    pub port_id: PortId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Connection {
    pub id: ConnectionId,
    pub output: PortEndpoint,
    pub input: PortEndpoint,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeInstance {
    pub id: NodeInstanceId,
    pub definition: NodeDefinitionRef,
    pub configuration: SchemaValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_state: Option<SchemaValue>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GraphDocument {
    pub schema_version: SchemaVersion,
    pub id: GraphId,
    pub revision: GraphRevision,
    pub nodes: Vec<NodeInstance>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl GraphDocument {
    #[must_use]
    pub fn new(id: GraphId) -> Self {
        Self {
            schema_version: SchemaVersion::first(),
            id,
            revision: GraphRevision::initial(),
            nodes: Vec::new(),
            connections: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{NodeDefinitionId, NodeDefinitionVersion, NodePackageId, PackageVersion, SchemaId};

    fn schema(id: &str) -> crate::SchemaRef {
        crate::SchemaRef {
            id: SchemaId::parse(id).unwrap(),
            version: SchemaVersion::new(1).unwrap(),
        }
    }

    #[test]
    fn graph_round_trip_preserves_definition_pin_and_separate_authored_state() {
        let mut graph = GraphDocument::new(GraphId::new());
        graph.nodes.push(NodeInstance {
            id: NodeInstanceId::new(),
            definition: NodeDefinitionRef {
                package_id: NodePackageId::parse("example.generator").unwrap(),
                package_version: PackageVersion::new(1, 4, 2),
                definition_id: NodeDefinitionId::parse("example.generator.noise").unwrap(),
                definition_version: NodeDefinitionVersion::new(3).unwrap(),
            },
            configuration: SchemaValue {
                schema: schema("example.generator.noise.config"),
                value: json!({"seed": 42}),
            },
            authored_state: Some(SchemaValue {
                schema: schema("example.generator.noise.state"),
                value: json!({"locked": true}),
            }),
            extensions: BTreeMap::new(),
        });

        let encoded = serde_json::to_vec(&graph).unwrap();
        let decoded: GraphDocument = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, graph);
        assert_ne!(
            decoded.nodes[0].configuration.schema,
            decoded.nodes[0].authored_state.as_ref().unwrap().schema
        );
    }
}
