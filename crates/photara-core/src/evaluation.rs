use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CanonicalDigest, DefinitionResolver, Diagnostic, DiagnosticSeverity, EvaluationId,
    GraphDocument, GraphId, GraphRevision, NodeDefinition, NodeDefinitionRef, NodeInstance,
    NodeInstanceId, PortCardinality, PortCompatibilityError, PortDirection, PortEndpoint, PortId,
    RequestId, SchemaRef, TypedValue, ValueTypeRef, ValueTypeRegistry, canonical_digest,
    validate_port_connection,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationRequest {
    pub request_id: RequestId,
    pub evaluation_id: EvaluationId,
    pub graph_id: GraphId,
    pub revision: GraphRevision,
    pub environment: CanonicalDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluationPhase {
    Validating,
    Planning,
    Evaluating,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationProgress {
    pub request_id: RequestId,
    pub evaluation_id: EvaluationId,
    pub phase: EvaluationPhase,
    pub completed_nodes: usize,
    pub total_nodes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeInstanceId>,
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub struct NodeEvaluationRequest {
    pub request_id: RequestId,
    pub evaluation_id: EvaluationId,
    pub node: NodeInstance,
    pub inputs: BTreeMap<PortId, Vec<TypedValue>>,
    pub evaluation_key: CanonicalDigest,
    pub cancellation: CancellationToken,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct NodeEvaluationOutput {
    pub outputs: BTreeMap<PortId, Vec<TypedValue>>,
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[error("{code}: {message}")]
pub struct NodeExecutionError {
    pub code: String,
    pub message: String,
}

/// Dispatches exact node-definition pins to their implementations.
pub trait NodeRuntime {
    fn implementation_fingerprint(&self, definition: &NodeDefinitionRef)
    -> Option<CanonicalDigest>;

    /// Executes one node after Core has validated and assembled its typed inputs.
    ///
    /// # Errors
    ///
    /// Returns a structured [`NodeExecutionError`] supplied by the node runtime.
    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError>;
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeEvaluationRecord {
    pub node_id: NodeInstanceId,
    pub evaluation_key: CanonicalDigest,
    pub outputs: BTreeMap<PortId, Vec<TypedValue>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EvaluationOutcome {
    pub request_id: RequestId,
    pub evaluation_id: EvaluationId,
    pub graph_id: GraphId,
    pub source_revision: GraphRevision,
    pub evaluation_key: CanonicalDigest,
    pub nodes: BTreeMap<NodeInstanceId, NodeEvaluationRecord>,
}

impl EvaluationOutcome {
    #[must_use]
    pub fn dirty_nodes_since(&self, previous: &Self) -> BTreeSet<NodeInstanceId> {
        self.nodes
            .iter()
            .filter_map(|(node_id, current)| {
                let is_dirty = previous
                    .nodes
                    .get(node_id)
                    .is_none_or(|old| old.evaluation_key != current.evaluation_key);
                is_dirty.then_some(*node_id)
            })
            .collect()
    }

    #[must_use]
    pub fn outputs(&self, endpoint: &PortEndpoint) -> Option<&[TypedValue]> {
        self.nodes
            .get(&endpoint.node_id)?
            .outputs
            .get(&endpoint.port_id)
            .map(Vec::as_slice)
    }
}

/// Validates, plans, and evaluates a graph in deterministic topological order.
///
/// This is intentionally a minimal local executor. The runtime callback keeps
/// node semantics outside Core while the request/progress/cancellation and
/// deterministic-key contracts remain usable by richer executors later.
///
/// # Errors
///
/// Returns [`EvaluationError`] for stale requests, invalid graphs, cooperative
/// cancellation, unavailable implementations, or structured node failures.
pub fn evaluate_graph<D, R, P>(
    graph: &GraphDocument,
    request: &EvaluationRequest,
    definitions: &D,
    value_types: &ValueTypeRegistry,
    runtime: &R,
    cancellation: &CancellationToken,
    mut progress: P,
) -> Result<EvaluationOutcome, EvaluationError>
where
    D: DefinitionResolver,
    R: NodeRuntime,
    P: FnMut(EvaluationProgress),
{
    if request.graph_id != graph.id {
        return Err(EvaluationError::GraphMismatch {
            expected: graph.id,
            actual: request.graph_id,
        });
    }
    if request.revision != graph.revision {
        return Err(EvaluationError::RevisionConflict {
            expected: request.revision,
            actual: graph.revision,
        });
    }

    report(
        request,
        EvaluationPhase::Validating,
        0,
        graph.nodes.len(),
        None,
        &mut progress,
    );
    check_cancelled(request, cancellation, 0, graph.nodes.len(), &mut progress)?;
    let plan = plan(graph, definitions, value_types)?;
    report(
        request,
        EvaluationPhase::Planning,
        0,
        plan.len(),
        None,
        &mut progress,
    );

    let records = execute_plan(
        graph,
        request,
        &plan,
        &ExecutionServices {
            definitions,
            value_types,
            runtime,
            cancellation,
        },
        &mut progress,
    )?;
    check_cancelled(request, cancellation, plan.len(), plan.len(), &mut progress)?;

    let node_keys: Vec<_> = records
        .iter()
        .map(|(node_id, record)| (*node_id, record.evaluation_key))
        .collect();
    let evaluation_key = canonical_digest(&GraphKeyMaterial {
        graph_id: graph.id,
        node_keys: &node_keys,
    })
    .map_err(|error| EvaluationError::Canonicalization {
        message: error.to_string(),
    })?;
    report(
        request,
        EvaluationPhase::Completed,
        plan.len(),
        plan.len(),
        None,
        &mut progress,
    );
    Ok(EvaluationOutcome {
        request_id: request.request_id,
        evaluation_id: request.evaluation_id,
        graph_id: graph.id,
        source_revision: graph.revision,
        evaluation_key,
        nodes: records,
    })
}

struct ExecutionServices<'a, D, R> {
    definitions: &'a D,
    value_types: &'a ValueTypeRegistry,
    runtime: &'a R,
    cancellation: &'a CancellationToken,
}

fn execute_plan<D, R, P>(
    graph: &GraphDocument,
    request: &EvaluationRequest,
    plan: &[NodeInstanceId],
    services: &ExecutionServices<'_, D, R>,
    progress: &mut P,
) -> Result<BTreeMap<NodeInstanceId, NodeEvaluationRecord>, EvaluationError>
where
    D: DefinitionResolver,
    R: NodeRuntime,
    P: FnMut(EvaluationProgress),
{
    let nodes: BTreeMap<_, _> = graph.nodes.iter().map(|node| (node.id, node)).collect();
    let mut records = BTreeMap::new();
    for (completed, node_id) in plan.iter().enumerate() {
        check_cancelled(
            request,
            services.cancellation,
            completed,
            plan.len(),
            progress,
        )?;
        let node = nodes
            .get(node_id)
            .copied()
            .ok_or(EvaluationError::UnknownNode(*node_id))?;
        let definition = resolve_definition(services.definitions, node)?;
        let inputs = collect_inputs(graph, *node_id, &records)?;
        validate_input_values(node.id, definition, &inputs)?;
        let implementation = services
            .runtime
            .implementation_fingerprint(&node.definition)
            .ok_or_else(|| EvaluationError::ImplementationUnavailable {
                node_id: node.id,
                definition: node.definition.clone(),
            })?;
        let key = node_evaluation_key(node, &inputs, request.environment, implementation)?;

        report(
            request,
            EvaluationPhase::Evaluating,
            completed,
            plan.len(),
            Some(*node_id),
            progress,
        );
        let output = services.runtime.evaluate(NodeEvaluationRequest {
            request_id: request.request_id,
            evaluation_id: request.evaluation_id,
            node: node.clone(),
            inputs,
            evaluation_key: key,
            cancellation: services.cancellation.clone(),
        });
        check_cancelled(
            request,
            services.cancellation,
            completed,
            plan.len(),
            progress,
        )?;
        let output = output.map_err(|error| EvaluationError::NodeExecution {
            node_id: node.id,
            error,
        })?;
        validate_outputs(node.id, definition, services.value_types, &output.outputs)?;
        records.insert(
            node.id,
            NodeEvaluationRecord {
                node_id: node.id,
                evaluation_key: key,
                outputs: output.outputs,
            },
        );
        report(
            request,
            EvaluationPhase::Evaluating,
            completed + 1,
            plan.len(),
            Some(*node_id),
            progress,
        );
    }
    Ok(records)
}

fn node_evaluation_key(
    node: &NodeInstance,
    inputs: &BTreeMap<PortId, Vec<TypedValue>>,
    environment: CanonicalDigest,
    implementation: CanonicalDigest,
) -> Result<CanonicalDigest, EvaluationError> {
    canonical_digest(&NodeKeyMaterial {
        definition: &node.definition,
        configuration: &node.configuration,
        authored_state: &node.authored_state,
        inputs,
        environment,
        implementation,
    })
    .map_err(|error| EvaluationError::Canonicalization {
        message: error.to_string(),
    })
}

#[derive(Serialize)]
struct NodeKeyMaterial<'a> {
    definition: &'a NodeDefinitionRef,
    configuration: &'a crate::SchemaValue,
    authored_state: &'a Option<crate::SchemaValue>,
    inputs: &'a BTreeMap<PortId, Vec<TypedValue>>,
    environment: CanonicalDigest,
    implementation: CanonicalDigest,
}

#[derive(Serialize)]
struct GraphKeyMaterial<'a> {
    graph_id: GraphId,
    node_keys: &'a [(NodeInstanceId, CanonicalDigest)],
}

fn plan<D: DefinitionResolver>(
    graph: &GraphDocument,
    definitions: &D,
    value_types: &ValueTypeRegistry,
) -> Result<Vec<NodeInstanceId>, EvaluationError> {
    let nodes: BTreeMap<_, _> = graph.nodes.iter().map(|node| (node.id, node)).collect();
    for node in &graph.nodes {
        let definition = resolve_definition(definitions, node)?;
        validate_node_state(node, definition)?;
    }

    let mut incoming: BTreeMap<NodeInstanceId, usize> =
        graph.nodes.iter().map(|node| (node.id, 0)).collect();
    let mut outgoing: BTreeMap<NodeInstanceId, BTreeSet<NodeInstanceId>> = BTreeMap::new();
    let mut input_connections: BTreeMap<(NodeInstanceId, PortId), usize> = BTreeMap::new();
    for connection in &graph.connections {
        let output_node = nodes
            .get(&connection.output.node_id)
            .copied()
            .ok_or(EvaluationError::UnknownNode(connection.output.node_id))?;
        let input_node = nodes
            .get(&connection.input.node_id)
            .copied()
            .ok_or(EvaluationError::UnknownNode(connection.input.node_id))?;
        let output_definition = resolve_definition(definitions, output_node)?;
        let input_definition = resolve_definition(definitions, input_node)?;
        let output_port = output_definition
            .port(&connection.output.port_id)
            .ok_or_else(|| EvaluationError::UnknownPort {
                node_id: output_node.id,
                port_id: connection.output.port_id.clone(),
            })?;
        let input_port = input_definition
            .port(&connection.input.port_id)
            .ok_or_else(|| EvaluationError::UnknownPort {
                node_id: input_node.id,
                port_id: connection.input.port_id.clone(),
            })?;
        validate_port_connection(value_types, output_port, input_port)
            .map_err(|error| EvaluationError::PortCompatibility { error })?;
        *input_connections
            .entry((input_node.id, input_port.id.clone()))
            .or_default() += 1;
        if outgoing
            .entry(output_node.id)
            .or_default()
            .insert(input_node.id)
        {
            *incoming
                .get_mut(&input_node.id)
                .ok_or(EvaluationError::UnknownNode(input_node.id))? += 1;
        }
    }

    for node in &graph.nodes {
        let definition = resolve_definition(definitions, node)?;
        for port in definition
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Input)
        {
            let count = input_connections
                .get(&(node.id, port.id.clone()))
                .copied()
                .unwrap_or_default();
            let valid = match port.cardinality {
                PortCardinality::One => count == 1,
                PortCardinality::Optional => count <= 1,
                PortCardinality::Many => true,
            };
            if !valid {
                return Err(EvaluationError::InputCardinality {
                    node_id: node.id,
                    port_id: port.id.clone(),
                    cardinality: port.cardinality,
                    actual: count,
                });
            }
        }
    }

    let mut ready: BTreeSet<_> = incoming
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(*node_id))
        .collect();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(node_id) = ready.pop_first() {
        order.push(node_id);
        if let Some(targets) = outgoing.get(&node_id) {
            for target in targets {
                let count = incoming
                    .get_mut(target)
                    .ok_or(EvaluationError::UnknownNode(*target))?;
                *count -= 1;
                if *count == 0 {
                    ready.insert(*target);
                }
            }
        }
    }
    if order.len() != nodes.len() {
        return Err(EvaluationError::CycleDetected);
    }
    Ok(order)
}

fn resolve_definition<'a, D: DefinitionResolver>(
    definitions: &'a D,
    node: &NodeInstance,
) -> Result<&'a NodeDefinition, EvaluationError> {
    definitions
        .resolve(&node.definition)
        .ok_or_else(|| EvaluationError::DefinitionUnavailable {
            node_id: node.id,
            definition: node.definition.clone(),
        })
}

fn validate_node_state(
    node: &NodeInstance,
    definition: &NodeDefinition,
) -> Result<(), EvaluationError> {
    validate_schema(
        node.id,
        "configuration",
        &definition.config_schema,
        &node.configuration.schema,
    )?;
    match (&definition.authored_state_schema, &node.authored_state) {
        (None, Some(_)) => return Err(EvaluationError::AuthoredStateUnsupported(node.id)),
        (Some(expected), Some(actual)) => {
            validate_schema(node.id, "authored-state", expected, &actual.schema)?;
        }
        (None | Some(_), None) => {}
    }
    Ok(())
}

fn validate_schema(
    node_id: NodeInstanceId,
    state_kind: &'static str,
    expected: &SchemaRef,
    actual: &SchemaRef,
) -> Result<(), EvaluationError> {
    if expected != actual {
        return Err(EvaluationError::SchemaMismatch {
            node_id,
            state_kind: state_kind.to_owned(),
            expected: expected.clone(),
            actual: actual.clone(),
        });
    }
    Ok(())
}

fn collect_inputs(
    graph: &GraphDocument,
    node_id: NodeInstanceId,
    records: &BTreeMap<NodeInstanceId, NodeEvaluationRecord>,
) -> Result<BTreeMap<PortId, Vec<TypedValue>>, EvaluationError> {
    let mut inputs: BTreeMap<PortId, Vec<TypedValue>> = BTreeMap::new();
    for connection in graph
        .connections
        .iter()
        .filter(|connection| connection.input.node_id == node_id)
    {
        let source = records.get(&connection.output.node_id).ok_or(
            EvaluationError::MissingUpstreamResult {
                node_id: connection.output.node_id,
            },
        )?;
        let values = source
            .outputs
            .get(&connection.output.port_id)
            .ok_or_else(|| EvaluationError::MissingOutput {
                node_id: connection.output.node_id,
                port_id: connection.output.port_id.clone(),
            })?;
        inputs
            .entry(connection.input.port_id.clone())
            .or_default()
            .extend(values.iter().cloned());
    }
    Ok(inputs)
}

fn validate_input_values(
    node_id: NodeInstanceId,
    definition: &NodeDefinition,
    inputs: &BTreeMap<PortId, Vec<TypedValue>>,
) -> Result<(), EvaluationError> {
    for port in definition
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Input)
    {
        let values = inputs.get(&port.id).map_or(&[][..], Vec::as_slice);
        let valid_count = match port.cardinality {
            PortCardinality::One => values.len() == 1,
            PortCardinality::Optional => values.len() <= 1,
            PortCardinality::Many => true,
        };
        if !valid_count {
            return Err(EvaluationError::InputCardinality {
                node_id,
                port_id: port.id.clone(),
                cardinality: port.cardinality,
                actual: values.len(),
            });
        }
        for value in values {
            if value.value_type != port.value_type {
                return Err(EvaluationError::ValueTypeMismatch {
                    node_id,
                    port_id: port.id.clone(),
                    expected: port.value_type.clone(),
                    actual: value.value_type.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_outputs(
    node_id: NodeInstanceId,
    definition: &NodeDefinition,
    value_types: &ValueTypeRegistry,
    outputs: &BTreeMap<PortId, Vec<TypedValue>>,
) -> Result<(), EvaluationError> {
    for (port_id, values) in outputs {
        let port = definition
            .port(port_id)
            .ok_or_else(|| EvaluationError::UnknownPort {
                node_id,
                port_id: port_id.clone(),
            })?;
        if port.direction != PortDirection::Output {
            return Err(EvaluationError::RuntimeReturnedInputPort {
                node_id,
                port_id: port_id.clone(),
            });
        }
        if value_types.get(&port.value_type).is_none() {
            return Err(EvaluationError::UnknownValueType(port.value_type.clone()));
        }
        for value in values {
            if value.value_type != port.value_type {
                return Err(EvaluationError::ValueTypeMismatch {
                    node_id,
                    port_id: port_id.clone(),
                    expected: port.value_type.clone(),
                    actual: value.value_type.clone(),
                });
            }
        }
    }
    for port in definition
        .ports
        .iter()
        .filter(|port| port.direction == PortDirection::Output)
    {
        let count = outputs.get(&port.id).map_or(0, Vec::len);
        let valid = match port.cardinality {
            PortCardinality::One => count == 1,
            PortCardinality::Optional => count <= 1,
            PortCardinality::Many => true,
        };
        if !valid {
            return Err(EvaluationError::OutputCardinality {
                node_id,
                port_id: port.id.clone(),
                cardinality: port.cardinality,
                actual: count,
            });
        }
    }
    Ok(())
}

fn report<P: FnMut(EvaluationProgress)>(
    request: &EvaluationRequest,
    phase: EvaluationPhase,
    completed_nodes: usize,
    total_nodes: usize,
    node_id: Option<NodeInstanceId>,
    progress: &mut P,
) {
    progress(EvaluationProgress {
        request_id: request.request_id,
        evaluation_id: request.evaluation_id,
        phase,
        completed_nodes,
        total_nodes,
        node_id,
    });
}

fn check_cancelled<P: FnMut(EvaluationProgress)>(
    request: &EvaluationRequest,
    cancellation: &CancellationToken,
    completed_nodes: usize,
    total_nodes: usize,
    progress: &mut P,
) -> Result<(), EvaluationError> {
    if cancellation.is_cancelled() {
        report(
            request,
            EvaluationPhase::Cancelled,
            completed_nodes,
            total_nodes,
            None,
            progress,
        );
        return Err(EvaluationError::Cancelled {
            request_id: request.request_id,
            evaluation_id: request.evaluation_id,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "kebab-case")]
pub enum EvaluationError {
    #[error("request targets graph {actual}, but snapshot is graph {expected}")]
    GraphMismatch { expected: GraphId, actual: GraphId },
    #[error("evaluation revision conflict: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: GraphRevision,
        actual: GraphRevision,
    },
    #[error("evaluation {evaluation_id} for request {request_id} was cancelled")]
    Cancelled {
        request_id: RequestId,
        evaluation_id: EvaluationId,
    },
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
    #[error("node {0} definition does not support authored state")]
    AuthoredStateUnsupported(NodeInstanceId),
    #[error("node {node_id} does not define port {port_id}")]
    UnknownPort {
        node_id: NodeInstanceId,
        port_id: PortId,
    },
    #[error("invalid port connection: {error}")]
    PortCompatibility { error: PortCompatibilityError },
    #[error("node {node_id} input {port_id} has {actual} values for {cardinality:?} cardinality")]
    InputCardinality {
        node_id: NodeInstanceId,
        port_id: PortId,
        cardinality: PortCardinality,
        actual: usize,
    },
    #[error("graph contains a cycle")]
    CycleDetected,
    #[error("node {node_id} has no registered implementation for {definition:?}")]
    ImplementationUnavailable {
        node_id: NodeInstanceId,
        definition: NodeDefinitionRef,
    },
    #[error("node {node_id} failed: {error}")]
    NodeExecution {
        node_id: NodeInstanceId,
        error: NodeExecutionError,
    },
    #[error("node {node_id} evaluation is missing upstream result")]
    MissingUpstreamResult { node_id: NodeInstanceId },
    #[error("node {node_id} did not return connected output {port_id}")]
    MissingOutput {
        node_id: NodeInstanceId,
        port_id: PortId,
    },
    #[error("node {node_id} runtime returned input port {port_id} as output")]
    RuntimeReturnedInputPort {
        node_id: NodeInstanceId,
        port_id: PortId,
    },
    #[error("value type {0:?} is not registered")]
    UnknownValueType(ValueTypeRef),
    #[error("node {node_id} port {port_id} expected value type {expected:?}, got {actual:?}")]
    ValueTypeMismatch {
        node_id: NodeInstanceId,
        port_id: PortId,
        expected: ValueTypeRef,
        actual: ValueTypeRef,
    },
    #[error("node {node_id} output {port_id} has {actual} values for {cardinality:?} cardinality")]
    OutputCardinality {
        node_id: NodeInstanceId,
        port_id: PortId,
        cardinality: PortCardinality,
        actual: usize,
    },
    #[error("canonical evaluation key could not be produced: {message}")]
    Canonicalization { message: String },
}

impl EvaluationError {
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        let (node_instance_id, port_id) = match self {
            Self::UnknownNode(node_id)
            | Self::AuthoredStateUnsupported(node_id)
            | Self::MissingUpstreamResult { node_id }
            | Self::DefinitionUnavailable { node_id, .. }
            | Self::SchemaMismatch { node_id, .. }
            | Self::ImplementationUnavailable { node_id, .. }
            | Self::NodeExecution { node_id, .. } => (Some(*node_id), None),
            Self::UnknownPort { node_id, port_id }
            | Self::InputCardinality {
                node_id, port_id, ..
            }
            | Self::MissingOutput { node_id, port_id }
            | Self::RuntimeReturnedInputPort { node_id, port_id }
            | Self::ValueTypeMismatch {
                node_id, port_id, ..
            }
            | Self::OutputCardinality {
                node_id, port_id, ..
            } => (Some(*node_id), Some(port_id.clone())),
            Self::GraphMismatch { .. }
            | Self::RevisionConflict { .. }
            | Self::Cancelled { .. }
            | Self::PortCompatibility { .. }
            | Self::CycleDetected
            | Self::UnknownValueType(_)
            | Self::Canonicalization { .. } => (None, None),
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
            Self::GraphMismatch { .. } => "evaluation-graph-mismatch",
            Self::RevisionConflict { .. } => "evaluation-revision-conflict",
            Self::Cancelled { .. } => "evaluation-cancelled",
            Self::UnknownNode(_) => "unknown-node",
            Self::DefinitionUnavailable { .. } => "definition-unavailable",
            Self::SchemaMismatch { .. } => "schema-mismatch",
            Self::AuthoredStateUnsupported(_) => "authored-state-unsupported",
            Self::UnknownPort { .. } => "unknown-port",
            Self::PortCompatibility { .. } => "port-incompatible",
            Self::InputCardinality { .. } => "input-cardinality-invalid",
            Self::CycleDetected => "cycle-detected",
            Self::ImplementationUnavailable { .. } => "implementation-unavailable",
            Self::NodeExecution { .. } => "node-execution-failed",
            Self::MissingUpstreamResult { .. } => "missing-upstream-result",
            Self::MissingOutput { .. } => "missing-output",
            Self::RuntimeReturnedInputPort { .. } => "runtime-returned-input-port",
            Self::UnknownValueType(_) => "unknown-value-type",
            Self::ValueTypeMismatch { .. } => "value-type-mismatch",
            Self::OutputCardinality { .. } => "output-cardinality-invalid",
            Self::Canonicalization { .. } => "evaluation-key-failed",
        }
    }
}
