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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_uses_optimistic_revision_checks() {
        let id = GraphId::new();
        let graph = GraphDocument::new(id);
        let mut repository = InMemoryGraphRepository::default();
        repository.create(graph.clone()).unwrap();

        let mut replacement = graph;
        replacement.revision = replacement.revision.next();
        repository
            .replace(replacement.clone(), GraphRevision::initial())
            .unwrap();

        let error = repository
            .replace(replacement, GraphRevision::initial())
            .unwrap_err();
        assert!(matches!(error, StoreError::RevisionConflict { .. }));
    }
}
