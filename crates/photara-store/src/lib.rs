//! Backend-neutral persistence contracts for authoritative Core state.

use std::collections::BTreeMap;

use photara_core::{GraphDocument, GraphId, GraphRevision};
use thiserror::Error;

pub trait GraphRepository {
    /// Inserts a graph that does not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::AlreadyExists`] when the graph ID is present.
    fn create(&mut self, graph: GraphDocument) -> Result<(), StoreError>;

    /// Loads a graph by ID, returning `None` when it is absent.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific [`StoreError`] when the read cannot finish.
    fn load(&self, id: GraphId) -> Result<Option<GraphDocument>, StoreError>;

    /// Replaces a graph after verifying its expected revision.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] when the graph is absent or
    /// [`StoreError::RevisionConflict`] when its revision changed.
    fn replace(
        &mut self,
        graph: GraphDocument,
        expected_revision: GraphRevision,
    ) -> Result<(), StoreError>;
}

#[derive(Default)]
pub struct InMemoryGraphRepository {
    graphs: BTreeMap<GraphId, GraphDocument>,
}

impl GraphRepository for InMemoryGraphRepository {
    fn create(&mut self, graph: GraphDocument) -> Result<(), StoreError> {
        if self.graphs.contains_key(&graph.id) {
            return Err(StoreError::AlreadyExists(graph.id));
        }
        self.graphs.insert(graph.id, graph);
        Ok(())
    }

    fn load(&self, id: GraphId) -> Result<Option<GraphDocument>, StoreError> {
        Ok(self.graphs.get(&id).cloned())
    }

    fn replace(
        &mut self,
        graph: GraphDocument,
        expected_revision: GraphRevision,
    ) -> Result<(), StoreError> {
        let current = self
            .graphs
            .get(&graph.id)
            .ok_or(StoreError::NotFound(graph.id))?;
        if current.revision != expected_revision {
            return Err(StoreError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        let required_revision = expected_revision
            .checked_next()
            .ok_or(StoreError::RevisionExhausted)?;
        if graph.revision != required_revision {
            return Err(StoreError::InvalidReplacementRevision {
                expected: required_revision,
                actual: graph.revision,
            });
        }
        self.graphs.insert(graph.id, graph);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum StoreError {
    #[error("graph {0} already exists")]
    AlreadyExists(GraphId),
    #[error("graph {0} does not exist")]
    NotFound(GraphId),
    #[error("graph revision conflict: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: GraphRevision,
        actual: GraphRevision,
    },
    #[error("graph revision space is exhausted")]
    RevisionExhausted,
    #[error("replacement revision must be {expected:?}, got {actual:?}")]
    InvalidReplacementRevision {
        expected: GraphRevision,
        actual: GraphRevision,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use photara_core::{
        CommandId, GraphCommand, GraphCommandEnvelope, NodeDefinition, NodeDefinitionId,
        NodeDefinitionRef, NodeDefinitionRegistry, NodeDefinitionVersion, NodeInstance,
        NodeInstanceId, NodePackageId, PackageVersion, SchemaId, SchemaRef, SchemaValue,
        SchemaVersion, ValueTypeRegistry, apply_graph_command,
    };
    use serde_json::json;

    use super::*;

    fn schema(id: &str) -> SchemaRef {
        SchemaRef {
            id: SchemaId::parse(id).unwrap(),
            version: SchemaVersion::first(),
        }
    }

    #[test]
    fn replacement_uses_optimistic_revision_checks() {
        let id = GraphId::new();
        let graph = GraphDocument::new(id);
        let definition_ref = NodeDefinitionRef {
            package_id: NodePackageId::parse("example.persistence").unwrap(),
            package_version: PackageVersion::new(1, 0, 0),
            definition_id: NodeDefinitionId::parse("example.persistence.value").unwrap(),
            definition_version: NodeDefinitionVersion::first(),
        };
        let mut definitions = NodeDefinitionRegistry::default();
        definitions
            .register(
                definition_ref.clone(),
                NodeDefinition {
                    id: definition_ref.definition_id.clone(),
                    version: definition_ref.definition_version,
                    display_name: "Value".to_owned(),
                    ports: Vec::new(),
                    config_schema: schema("example.persistence.value.config"),
                    authored_state_schema: Some(schema("example.persistence.value.state")),
                    capabilities: BTreeSet::new(),
                },
            )
            .unwrap();
        let mut repository = InMemoryGraphRepository::default();
        repository.create(graph.clone()).unwrap();

        let replacement = apply_graph_command(
            &graph,
            &GraphCommandEnvelope {
                command_id: CommandId::new(),
                graph_id: id,
                expected_revision: graph.revision,
                command: GraphCommand::AddNode {
                    instance: NodeInstance {
                        id: NodeInstanceId::new(),
                        definition: definition_ref,
                        configuration: SchemaValue {
                            schema: schema("example.persistence.value.config"),
                            value: json!({"value": 42}),
                        },
                        authored_state: Some(SchemaValue {
                            schema: schema("example.persistence.value.state"),
                            value: json!({"locked": true}),
                        }),
                        extensions: BTreeMap::new(),
                    },
                },
            },
            &definitions,
            &ValueTypeRegistry::default(),
        )
        .unwrap()
        .graph;
        repository
            .replace(replacement.clone(), GraphRevision::initial())
            .unwrap();
        assert_eq!(repository.load(id).unwrap().unwrap(), replacement);

        let error = repository
            .replace(replacement, GraphRevision::initial())
            .unwrap_err();
        assert!(matches!(error, StoreError::RevisionConflict { .. }));
    }
}
