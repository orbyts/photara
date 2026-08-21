use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::*;

const PACKAGE_ID: &str = "example.text";
const SOURCE_ID: &str = "example.text.source";
const UPPERCASE_ID: &str = "example.text.uppercase";
const TEXT_TYPE_ID: &str = "example.text.value";

struct Fixture {
    definitions: NodeDefinitionRegistry,
    value_types: ValueTypeRegistry,
    source: NodeDefinitionRef,
    uppercase: NodeDefinitionRef,
}

impl Fixture {
    fn new() -> Self {
        let text_type = ValueTypeRef {
            id: ValueTypeId::parse(TEXT_TYPE_ID).unwrap(),
            version: ValueTypeVersion::first(),
        };
        let mut value_types = ValueTypeRegistry::default();
        value_types
            .register(ValueTypeDescriptor {
                value_type: text_type.clone(),
                display_name: "Text".to_owned(),
                schema: schema("example.text.value.payload"),
            })
            .unwrap();

        let source = definition_ref(SOURCE_ID);
        let uppercase = definition_ref(UPPERCASE_ID);
        let mut definitions = NodeDefinitionRegistry::default();
        definitions
            .register(
                source.clone(),
                NodeDefinition {
                    id: source.definition_id.clone(),
                    version: source.definition_version,
                    display_name: "Text Source".to_owned(),
                    ports: vec![port(
                        "text",
                        PortDirection::Output,
                        text_type.clone(),
                        PortCardinality::One,
                    )],
                    config_schema: schema("example.text.source.config"),
                    authored_state_schema: None,
                    capabilities: BTreeSet::new(),
                },
            )
            .unwrap();
        definitions
            .register(
                uppercase.clone(),
                NodeDefinition {
                    id: uppercase.definition_id.clone(),
                    version: uppercase.definition_version,
                    display_name: "Uppercase".to_owned(),
                    ports: vec![
                        port(
                            "input",
                            PortDirection::Input,
                            text_type.clone(),
                            PortCardinality::One,
                        ),
                        port(
                            "text",
                            PortDirection::Output,
                            text_type.clone(),
                            PortCardinality::One,
                        ),
                    ],
                    config_schema: schema("example.text.uppercase.config"),
                    authored_state_schema: None,
                    capabilities: BTreeSet::new(),
                },
            )
            .unwrap();
        Self {
            definitions,
            value_types,
            source,
            uppercase,
        }
    }

    fn source_node(&self, id: NodeInstanceId, text: &str) -> NodeInstance {
        NodeInstance {
            id,
            definition: self.source.clone(),
            configuration: SchemaValue {
                schema: schema("example.text.source.config"),
                value: json!({"text": text}),
            },
            authored_state: None,
            extensions: BTreeMap::new(),
        }
    }

    fn uppercase_node(&self, id: NodeInstanceId) -> NodeInstance {
        NodeInstance {
            id,
            definition: self.uppercase.clone(),
            configuration: SchemaValue {
                schema: schema("example.text.uppercase.config"),
                value: json!({}),
            },
            authored_state: None,
            extensions: BTreeMap::new(),
        }
    }
}

fn definition_ref(id: &str) -> NodeDefinitionRef {
    NodeDefinitionRef {
        package_id: NodePackageId::parse(PACKAGE_ID).unwrap(),
        package_version: PackageVersion::new(1, 0, 0),
        definition_id: NodeDefinitionId::parse(id).unwrap(),
        definition_version: NodeDefinitionVersion::first(),
    }
}

fn schema(id: &str) -> SchemaRef {
    SchemaRef {
        id: SchemaId::parse(id).unwrap(),
        version: SchemaVersion::first(),
    }
}

fn port(
    id: &str,
    direction: PortDirection,
    value_type: ValueTypeRef,
    cardinality: PortCardinality,
) -> PortDefinition {
    PortDefinition {
        id: PortId::parse(id).unwrap(),
        direction,
        value_type,
        cardinality,
    }
}

fn apply(
    graph: &GraphDocument,
    fixture: &Fixture,
    command: GraphCommand,
) -> Result<GraphDocument, GraphCommandError> {
    apply_graph_command(
        graph,
        &GraphCommandEnvelope {
            command_id: CommandId::new(),
            graph_id: graph.id,
            expected_revision: graph.revision,
            command,
        },
        &fixture.definitions,
        &fixture.value_types,
    )
    .map(|result| result.graph)
}

#[derive(Clone, Copy)]
struct TextRuntime;

impl NodeRuntime for TextRuntime {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<CanonicalDigest> {
        canonical_digest(definition).ok()
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError> {
        if request.cancellation.is_cancelled() {
            return Err(NodeExecutionError {
                code: "cancelled".to_owned(),
                message: "cancelled before node execution".to_owned(),
            });
        }
        let text = match request.node.definition.definition_id.as_str() {
            SOURCE_ID => request.node.configuration.value["text"]
                .as_str()
                .expect("test source text is a string")
                .to_owned(),
            UPPERCASE_ID => request.inputs[&PortId::parse("input").unwrap()][0]
                .value
                .as_str()
                .expect("test input is a string")
                .to_uppercase(),
            other => {
                return Err(NodeExecutionError {
                    code: "unknown-definition".to_owned(),
                    message: format!("no test implementation for {other}"),
                });
            }
        };
        Ok(NodeEvaluationOutput {
            outputs: BTreeMap::from([(
                PortId::parse("text").unwrap(),
                vec![TypedValue {
                    value_type: ValueTypeRef {
                        id: ValueTypeId::parse(TEXT_TYPE_ID).unwrap(),
                        version: ValueTypeVersion::first(),
                    },
                    value: json!(text),
                }],
            )]),
        })
    }
}

fn evaluation_request(graph: &GraphDocument) -> EvaluationRequest {
    EvaluationRequest {
        request_id: RequestId::new(),
        evaluation_id: EvaluationId::new(),
        graph_id: graph.id,
        revision: graph.revision,
        environment: canonical_digest(&"test-environment-v1").unwrap(),
    }
}

struct TextGraph {
    graph: GraphDocument,
    source_id: NodeInstanceId,
    uppercase_id: NodeInstanceId,
    unrelated_id: NodeInstanceId,
}

fn build_text_graph(fixture: &Fixture) -> TextGraph {
    let source_id = NodeInstanceId::new();
    let uppercase_id = NodeInstanceId::new();
    let unrelated_id = NodeInstanceId::new();
    let mut graph = GraphDocument::new(GraphId::new());
    for instance in [
        fixture.source_node(source_id, "hello"),
        fixture.uppercase_node(uppercase_id),
        fixture.source_node(unrelated_id, "stable"),
    ] {
        graph = apply(&graph, fixture, GraphCommand::AddNode { instance }).unwrap();
    }
    graph = apply(
        &graph,
        fixture,
        GraphCommand::Connect {
            connection: Connection {
                id: ConnectionId::new(),
                output: PortEndpoint {
                    node_id: source_id,
                    port_id: PortId::parse("text").unwrap(),
                },
                input: PortEndpoint {
                    node_id: uppercase_id,
                    port_id: PortId::parse("input").unwrap(),
                },
                extensions: BTreeMap::new(),
            },
        },
    )
    .unwrap();
    TextGraph {
        graph,
        source_id,
        uppercase_id,
        unrelated_id,
    }
}

fn evaluate_text_graph(
    graph: &GraphDocument,
    fixture: &Fixture,
) -> (EvaluationOutcome, Vec<EvaluationProgress>) {
    let mut progress = Vec::new();
    let outcome = evaluate_graph(
        graph,
        &evaluation_request(graph),
        &fixture.definitions,
        &fixture.value_types,
        &TextRuntime,
        &CancellationToken::default(),
        |event| progress.push(event),
    )
    .unwrap();
    (outcome, progress)
}

#[test]
fn commands_build_and_evaluate_a_deterministic_general_graph() {
    let fixture = Fixture::new();
    let TextGraph {
        mut graph,
        source_id,
        uppercase_id,
        unrelated_id,
    } = build_text_graph(&fixture);
    let (first, progress) = evaluate_text_graph(&graph, &fixture);
    let endpoint = PortEndpoint {
        node_id: uppercase_id,
        port_id: PortId::parse("text").unwrap(),
    };
    assert_eq!(first.outputs(&endpoint).unwrap()[0].value, json!("HELLO"));
    assert!(
        progress
            .iter()
            .any(|event| event.phase == EvaluationPhase::Planning)
    );
    assert_eq!(progress.last().unwrap().phase, EvaluationPhase::Completed);

    let (second, _) = evaluate_text_graph(&graph, &fixture);
    assert_eq!(first.evaluation_key, second.evaluation_key);
    assert!(second.dirty_nodes_since(&first).is_empty());

    graph = apply(
        &graph,
        &fixture,
        GraphCommand::SetConfiguration {
            node_id: source_id,
            configuration: SchemaValue {
                schema: schema("example.text.source.config"),
                value: json!({"text": "world"}),
            },
        },
    )
    .unwrap();
    let (changed, _) = evaluate_text_graph(&graph, &fixture);
    assert_eq!(changed.outputs(&endpoint).unwrap()[0].value, json!("WORLD"));
    assert_eq!(
        changed.dirty_nodes_since(&first),
        BTreeSet::from([source_id, uppercase_id])
    );
    assert_eq!(
        changed.nodes[&unrelated_id].evaluation_key,
        first.nodes[&unrelated_id].evaluation_key
    );
}

#[test]
fn stale_commands_return_a_structured_revision_conflict() {
    let fixture = Fixture::new();
    let graph = GraphDocument::new(GraphId::new());
    let envelope = GraphCommandEnvelope {
        command_id: CommandId::new(),
        graph_id: graph.id,
        expected_revision: graph.revision.next(),
        command: GraphCommand::AddNode {
            instance: fixture.source_node(NodeInstanceId::new(), "hello"),
        },
    };
    let error = apply_graph_command(
        &graph,
        &envelope,
        &fixture.definitions,
        &fixture.value_types,
    )
    .unwrap_err();
    assert_eq!(error.code(), "revision-conflict");
    assert_eq!(error.diagnostic().code, "revision-conflict");
    let encoded = serde_json::to_value(error).unwrap();
    assert_eq!(encoded["code"], "revision-conflict");
}

#[test]
fn commands_reject_schema_cardinality_and_cycle_violations() {
    let fixture = Fixture::new();
    let TextGraph {
        graph,
        uppercase_id,
        unrelated_id,
        ..
    } = build_text_graph(&fixture);
    let schema_error = apply(
        &graph,
        &fixture,
        GraphCommand::SetConfiguration {
            node_id: unrelated_id,
            configuration: SchemaValue {
                schema: schema("example.text.wrong.config"),
                value: json!({}),
            },
        },
    )
    .unwrap_err();
    assert!(matches!(
        schema_error,
        GraphCommandError::SchemaMismatch { .. }
    ));

    let cardinality_error = apply(
        &graph,
        &fixture,
        GraphCommand::Connect {
            connection: Connection {
                id: ConnectionId::new(),
                output: PortEndpoint {
                    node_id: unrelated_id,
                    port_id: PortId::parse("text").unwrap(),
                },
                input: PortEndpoint {
                    node_id: uppercase_id,
                    port_id: PortId::parse("input").unwrap(),
                },
                extensions: BTreeMap::new(),
            },
        },
    )
    .unwrap_err();
    assert!(matches!(
        cardinality_error,
        GraphCommandError::InputCardinalityExceeded { .. }
    ));

    let isolated_id = NodeInstanceId::new();
    let graph = apply(
        &graph,
        &fixture,
        GraphCommand::AddNode {
            instance: fixture.uppercase_node(isolated_id),
        },
    )
    .unwrap();
    let cycle_error = apply(
        &graph,
        &fixture,
        GraphCommand::Connect {
            connection: Connection {
                id: ConnectionId::new(),
                output: PortEndpoint {
                    node_id: isolated_id,
                    port_id: PortId::parse("text").unwrap(),
                },
                input: PortEndpoint {
                    node_id: isolated_id,
                    port_id: PortId::parse("input").unwrap(),
                },
                extensions: BTreeMap::new(),
            },
        },
    )
    .unwrap_err();
    assert!(matches!(
        cycle_error,
        GraphCommandError::CycleDetected { .. }
    ));
}

#[test]
fn evaluation_reports_cooperative_cancellation() {
    let fixture = Fixture::new();
    let mut graph = GraphDocument::new(GraphId::new());
    for text in ["one", "two"] {
        graph = apply(
            &graph,
            &fixture,
            GraphCommand::AddNode {
                instance: fixture.source_node(NodeInstanceId::new(), text),
            },
        )
        .unwrap();
    }
    let request = evaluation_request(&graph);
    let cancellation = CancellationToken::default();
    let mut progress = Vec::new();
    let error = evaluate_graph(
        &graph,
        &request,
        &fixture.definitions,
        &fixture.value_types,
        &TextRuntime,
        &cancellation,
        |event| {
            if event.phase == EvaluationPhase::Evaluating && event.completed_nodes == 1 {
                cancellation.cancel();
            }
            progress.push(event);
        },
    )
    .unwrap_err();
    assert!(matches!(error, EvaluationError::Cancelled { .. }));
    assert_eq!(progress.last().unwrap().phase, EvaluationPhase::Cancelled);
    assert!(progress.iter().all(|event| {
        event.request_id == request.request_id && event.evaluation_id == request.evaluation_id
    }));
}
