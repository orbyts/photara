use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CommandId, Connection, ConnectionId, DefinitionResolver, Diagnostic, DiagnosticSeverity,
    GraphDocument, GraphId, GraphRevision, NodeDefinition, NodeDefinitionRef, NodeInstance,
    NodeInstanceId, PortCardinality, PortCompatibilityError, PortId, SchemaRef, SchemaValue,
    ValueTypeRegistry, validate_port_connection,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GraphCommandEnvelope {
    pub command_id: CommandId,
    pub graph_id: GraphId,
    pub expected_revision: GraphRevision,
    pub command: GraphCommand,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GraphCommand {
    AddNode {
        instance: NodeInstance,
    },
    Connect {
        connection: Connection,
    },
    SetConfiguration {
        node_id: NodeInstanceId,
        configuration: SchemaValue,
    },
    SetAuthoredState {
        node_id: NodeInstanceId,
        authored_state: Option<SchemaValue>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GraphCommandResult {
    pub command_id: CommandId,
    pub previous_revision: GraphRevision,
    pub revision: GraphRevision,
    pub graph: GraphDocument,
}

/// Applies one semantic command to a graph snapshot.
///
/// The returned graph is a new value with exactly one revision increment. The
/// caller remains responsible for a compare-and-swap persistence write against
/// `expected_revision`.
///
/// # Errors
///
/// Returns [`GraphCommandError`] when identity/revision checks fail or the
/// command would violate registered definition, schema, port, cardinality, or
/// acyclic-graph contracts.
pub fn apply_graph_command<R: DefinitionResolver>(
    graph: &GraphDocument,
    envelope: &GraphCommandEnvelope,
    definitions: &R,
    value_types: &ValueTypeRegistry,
) -> Result<GraphCommandResult, GraphCommandError> {
    if graph.id != envelope.graph_id {
        return Err(GraphCommandError::GraphMismatch {
            expected: graph.id,
            actual: envelope.graph_id,
        });
    }
    if graph.revision != envelope.expected_revision {
        return Err(GraphCommandError::RevisionConflict {
            expected: envelope.expected_revision,
            actual: graph.revision,
        });
    }

    let mut updated = graph.clone();
    match &envelope.command {
        GraphCommand::AddNode { instance } => {
            add_node(&mut updated, instance.clone(), definitions)?;
        }
        GraphCommand::Connect { connection } => {
            connect(&mut updated, connection.clone(), definitions, value_types)?;
        }
        GraphCommand::SetConfiguration {
            node_id,
            configuration,
        } => {
            let node = node_mut(&mut updated, *node_id)?;
            let definition = resolve_definition(definitions, node)?;
            validate_schema(
                *node_id,
                "configuration",
                &definition.config_schema,
                &configuration.schema,
            )?;
            node.configuration = configuration.clone();
        }
        GraphCommand::SetAuthoredState {
            node_id,
            authored_state,
        } => {
            let node = node_mut(&mut updated, *node_id)?;
            let definition = resolve_definition(definitions, node)?;
            validate_authored_state(*node_id, definition, authored_state.as_ref())?;
            node.authored_state.clone_from(authored_state);
        }
    }

    let previous_revision = updated.revision;
    let revision = previous_revision
        .checked_next()
        .ok_or(GraphCommandError::RevisionExhausted)?;
    updated.revision = revision;
    Ok(GraphCommandResult {
        command_id: envelope.command_id,
        previous_revision,
        revision,
        graph: updated,
    })
}

fn add_node<R: DefinitionResolver>(
    graph: &mut GraphDocument,
    instance: NodeInstance,
    definitions: &R,
) -> Result<(), GraphCommandError> {
    if graph.nodes.iter().any(|node| node.id == instance.id) {
        return Err(GraphCommandError::DuplicateNode(instance.id));
    }
    let definition = resolve_definition(definitions, &instance)?;
    validate_schema(
        instance.id,
        "configuration",
        &definition.config_schema,
        &instance.configuration.schema,
    )?;
    validate_authored_state(instance.id, definition, instance.authored_state.as_ref())?;
    graph.nodes.push(instance);
    Ok(())
}

fn connect<R: DefinitionResolver>(
    graph: &mut GraphDocument,
    connection: Connection,
    definitions: &R,
    value_types: &ValueTypeRegistry,
) -> Result<(), GraphCommandError> {
    if graph
        .connections
        .iter()
        .any(|existing| existing.id == connection.id)
    {
        return Err(GraphCommandError::DuplicateConnection(connection.id));
    }
    if graph
        .connections
        .iter()
        .any(|existing| existing.output == connection.output && existing.input == connection.input)
    {
        return Err(GraphCommandError::DuplicateEndpoints {
            output_node: connection.output.node_id,
            output_port: connection.output.port_id.clone(),
            input_node: connection.input.node_id,
            input_port: connection.input.port_id.clone(),
        });
    }

    let output_node = node(graph, connection.output.node_id)?;
    let input_node = node(graph, connection.input.node_id)?;
    let output_definition = resolve_definition(definitions, output_node)?;
    let input_definition = resolve_definition(definitions, input_node)?;
    let output_port = output_definition
        .port(&connection.output.port_id)
        .ok_or_else(|| GraphCommandError::UnknownPort {
            node_id: connection.output.node_id,
            port_id: connection.output.port_id.clone(),
        })?;
    let input_port = input_definition
        .port(&connection.input.port_id)
        .ok_or_else(|| GraphCommandError::UnknownPort {
            node_id: connection.input.node_id,
            port_id: connection.input.port_id.clone(),
        })?;

    validate_port_connection(value_types, output_port, input_port)
        .map_err(|error| GraphCommandError::PortCompatibility { error })?;
    if input_port.cardinality != PortCardinality::Many
        && graph.connections.iter().any(|existing| {
            existing.input.node_id == connection.input.node_id
                && existing.input.port_id == connection.input.port_id
        })
    {
        return Err(GraphCommandError::InputCardinalityExceeded {
            node_id: connection.input.node_id,
            port_id: connection.input.port_id.clone(),
            cardinality: input_port.cardinality,
        });
    }
    if would_create_cycle(graph, &connection) {
        return Err(GraphCommandError::CycleDetected {
            output_node: connection.output.node_id,
            input_node: connection.input.node_id,
        });
    }
    graph.connections.push(connection);
    Ok(())
}

fn node(graph: &GraphDocument, id: NodeInstanceId) -> Result<&NodeInstance, GraphCommandError> {
    graph
        .nodes
        .iter()
        .find(|node| node.id == id)
        .ok_or(GraphCommandError::UnknownNode(id))
}

fn node_mut(
    graph: &mut GraphDocument,
    id: NodeInstanceId,
) -> Result<&mut NodeInstance, GraphCommandError> {
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == id)
        .ok_or(GraphCommandError::UnknownNode(id))
}

fn resolve_definition<'a, R: DefinitionResolver>(
    definitions: &'a R,
    node: &NodeInstance,
) -> Result<&'a NodeDefinition, GraphCommandError> {
    definitions
        .resolve(&node.definition)
        .ok_or_else(|| GraphCommandError::DefinitionUnavailable {
            node_id: node.id,
            definition: node.definition.clone(),
        })
}

fn validate_schema(
    node_id: NodeInstanceId,
    state_kind: &'static str,
    expected: &SchemaRef,
    actual: &SchemaRef,
) -> Result<(), GraphCommandError> {
    if expected != actual {
        return Err(GraphCommandError::SchemaMismatch {
            node_id,
            state_kind: state_kind.to_owned(),
            expected: expected.clone(),
            actual: actual.clone(),
        });
    }
    Ok(())
}

fn validate_authored_state(
    node_id: NodeInstanceId,
    definition: &NodeDefinition,
    authored_state: Option<&SchemaValue>,
) -> Result<(), GraphCommandError> {
    match (&definition.authored_state_schema, authored_state) {
        (None, Some(_)) => Err(GraphCommandError::AuthoredStateUnsupported { node_id }),
        (Some(expected), Some(actual)) => {
            validate_schema(node_id, "authored-state", expected, &actual.schema)
        }
        (None | Some(_), None) => Ok(()),
    }
}

fn would_create_cycle(graph: &GraphDocument, connection: &Connection) -> bool {
    if connection.output.node_id == connection.input.node_id {
        return true;
    }
    let mut pending = vec![connection.input.node_id];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(current) = pending.pop() {
        if current == connection.output.node_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        pending.extend(
            graph
                .connections
                .iter()
                .filter(|existing| existing.output.node_id == current)
                .map(|existing| existing.input.node_id),
        );
    }
    false
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "kebab-case")]
pub enum GraphCommandError {
    #[error("command targets graph {actual}, but snapshot is graph {expected}")]
    GraphMismatch { expected: GraphId, actual: GraphId },
    #[error("graph revision conflict: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: GraphRevision,
        actual: GraphRevision,
    },
    #[error("graph revision space is exhausted")]
    RevisionExhausted,
    #[error("node {0} already exists")]
    DuplicateNode(NodeInstanceId),
    #[error("node {0} does not exist")]
    UnknownNode(NodeInstanceId),
    #[error("definition {definition:?} for node {node_id} is unavailable")]
    DefinitionUnavailable {
        node_id: NodeInstanceId,
        definition: NodeDefinitionRef,
    },
    #[error(
        "node {node_id} {state_kind} schema mismatch: expected {expected:?}, actual {actual:?}"
    )]
    SchemaMismatch {
        node_id: NodeInstanceId,
        state_kind: String,
        expected: SchemaRef,
        actual: SchemaRef,
    },
    #[error("node {node_id} definition does not support authored state")]
    AuthoredStateUnsupported { node_id: NodeInstanceId },
    #[error("connection {0} already exists")]
    DuplicateConnection(ConnectionId),
    #[error(
        "connection from {output_node}:{output_port} to {input_node}:{input_port} already exists"
    )]
    DuplicateEndpoints {
        output_node: NodeInstanceId,
        output_port: PortId,
        input_node: NodeInstanceId,
        input_port: PortId,
    },
    #[error("node {node_id} does not define port {port_id}")]
    UnknownPort {
        node_id: NodeInstanceId,
        port_id: PortId,
    },
    #[error("invalid port connection: {error}")]
    PortCompatibility { error: PortCompatibilityError },
    #[error("node {node_id} input {port_id} exceeds {cardinality:?} cardinality")]
    InputCardinalityExceeded {
        node_id: NodeInstanceId,
        port_id: PortId,
        cardinality: PortCardinality,
    },
    #[error("connection from node {output_node} to node {input_node} creates a cycle")]
    CycleDetected {
        output_node: NodeInstanceId,
        input_node: NodeInstanceId,
    },
}

impl GraphCommandError {
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let (node_instance_id, port_id) = match self {
            Self::DuplicateNode(node_id)
            | Self::UnknownNode(node_id)
            | Self::AuthoredStateUnsupported { node_id }
            | Self::DefinitionUnavailable { node_id, .. }
            | Self::SchemaMismatch { node_id, .. } => (Some(*node_id), None),
            Self::UnknownPort { node_id, port_id }
            | Self::InputCardinalityExceeded {
                node_id, port_id, ..
            } => (Some(*node_id), Some(port_id.clone())),
            Self::CycleDetected { input_node, .. } => (Some(*input_node), None),
            Self::GraphMismatch { .. }
            | Self::RevisionConflict { .. }
            | Self::RevisionExhausted
            | Self::DuplicateConnection(_)
            | Self::DuplicateEndpoints { .. }
            | Self::PortCompatibility { .. } => (None, None),
        };
        Diagnostic {
            code: self.code().to_owned(),
            severity: DiagnosticSeverity::Error,
            message: self.to_string(),
            node_instance_id,
            port_id,
        }
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::GraphMismatch { .. } => "graph-mismatch",
            Self::RevisionConflict { .. } => "revision-conflict",
            Self::RevisionExhausted => "revision-exhausted",
            Self::DuplicateNode(_) => "duplicate-node",
            Self::UnknownNode(_) => "unknown-node",
            Self::DefinitionUnavailable { .. } => "definition-unavailable",
            Self::SchemaMismatch { .. } => "schema-mismatch",
            Self::AuthoredStateUnsupported { .. } => "authored-state-unsupported",
            Self::DuplicateConnection(_) => "duplicate-connection",
            Self::DuplicateEndpoints { .. } => "duplicate-endpoints",
            Self::UnknownPort { .. } => "unknown-port",
            Self::PortCompatibility { .. } => "port-incompatible",
            Self::InputCardinalityExceeded { .. } => "input-cardinality-exceeded",
            Self::CycleDetected { .. } => "cycle-detected",
        }
    }
}
