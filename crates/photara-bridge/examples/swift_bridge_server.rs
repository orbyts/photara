use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, BufRead, Write},
    sync::mpsc,
    time::Duration,
};

use photara_bridge::{
    EvaluationProgressDto, application_info, command_applied, command_rejected, evaluation_error,
};
use photara_core::{
    CancellationToken, CanonicalDigest, CommandId, EvaluationId, EvaluationPhase,
    EvaluationRequest, GraphCommand, GraphCommandEnvelope, GraphDocument, GraphId, NodeDefinition,
    NodeDefinitionId, NodeDefinitionRef, NodeDefinitionRegistry, NodeDefinitionVersion,
    NodeEvaluationOutput, NodeEvaluationRequest, NodeExecutionError, NodeInstance, NodeInstanceId,
    NodePackageId, NodeRuntime, PackageVersion, PortCardinality, PortDefinition, PortDirection,
    PortId, ProjectDocument, ProjectId, RequestId, SchemaId, SchemaRef, SchemaValue, SchemaVersion,
    TypedValue, ValueTypeDescriptor, ValueTypeId, ValueTypeRef, ValueTypeRegistry,
    ValueTypeVersion, apply_graph_command, canonical_digest, evaluate_graph,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const PACKAGE_ID: &str = "example.bridge";
const DEFINITION_ID: &str = "example.bridge.source";
const VALUE_TYPE_ID: &str = "example.bridge.text";

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case")]
enum ClientRequest {
    Start {
        command_id: CommandId,
        request_id: RequestId,
        evaluation_id: EvaluationId,
    },
    Cancel {
        request_id: RequestId,
        evaluation_id: EvaluationId,
    },
}

#[derive(Serialize)]
struct Event<'a, T> {
    event: &'a str,
    payload: T,
}

struct Fixture {
    definitions: NodeDefinitionRegistry,
    value_types: ValueTypeRegistry,
    definition: NodeDefinitionRef,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let value_type = ValueTypeRef {
            id: ValueTypeId::parse(VALUE_TYPE_ID)?,
            version: ValueTypeVersion::first(),
        };
        let mut value_types = ValueTypeRegistry::default();
        value_types.register(ValueTypeDescriptor {
            value_type: value_type.clone(),
            display_name: "Bridge text".to_owned(),
            schema: schema("example.bridge.text.payload")?,
        })?;

        let definition = NodeDefinitionRef {
            package_id: NodePackageId::parse(PACKAGE_ID)?,
            package_version: PackageVersion::new(1, 0, 0),
            definition_id: NodeDefinitionId::parse(DEFINITION_ID)?,
            definition_version: NodeDefinitionVersion::first(),
        };
        let mut definitions = NodeDefinitionRegistry::default();
        definitions.register(
            definition.clone(),
            NodeDefinition {
                id: definition.definition_id.clone(),
                version: definition.definition_version,
                display_name: "Bridge source".to_owned(),
                ports: vec![PortDefinition {
                    id: PortId::parse("text")?,
                    direction: PortDirection::Output,
                    value_type,
                    cardinality: PortCardinality::One,
                }],
                config_schema: schema("example.bridge.source.config")?,
                authored_state_schema: None,
                capabilities: BTreeSet::new(),
            },
        )?;
        Ok(Self {
            definitions,
            value_types,
            definition,
        })
    }

    fn node(&self) -> Result<NodeInstance, Box<dyn Error>> {
        Ok(NodeInstance {
            id: NodeInstanceId::new(),
            definition: self.definition.clone(),
            configuration: SchemaValue {
                schema: schema("example.bridge.source.config")?,
                value: json!({"text": "hello from Rust"}),
            },
            authored_state: None,
            extensions: BTreeMap::new(),
        })
    }
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
        let text = request.node.configuration.value["text"]
            .as_str()
            .ok_or_else(|| NodeExecutionError {
                code: "invalid-text".to_owned(),
                message: "source configuration has no text".to_owned(),
            })?;
        Ok(NodeEvaluationOutput {
            outputs: BTreeMap::from([(
                PortId::parse("text").expect("static port ID is valid"),
                vec![TypedValue {
                    value_type: ValueTypeRef {
                        id: ValueTypeId::parse(VALUE_TYPE_ID)
                            .expect("static value type ID is valid"),
                        version: ValueTypeVersion::first(),
                    },
                    value: json!(text),
                }],
            )]),
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    emit("ready", application_info())?;
    let (command_id, request_id, evaluation_id) = read_start()?;
    let requests = spawn_request_reader();
    run_spike(command_id, request_id, evaluation_id, &requests)
}

fn read_start() -> Result<(CommandId, RequestId, EvaluationId), Box<dyn Error>> {
    let mut start = String::new();
    if io::stdin().read_line(&mut start)? == 0 {
        return Err("Swift closed stdin before start".into());
    }
    let ClientRequest::Start {
        command_id,
        request_id,
        evaluation_id,
    } = serde_json::from_str(&start)?
    else {
        return Err("first request must be start".into());
    };
    Ok((command_id, request_id, evaluation_id))
}

fn spawn_request_reader() -> mpsc::Receiver<ClientRequest> {
    let (requests_tx, requests_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if let Ok(request) = serde_json::from_str::<ClientRequest>(&line)
                && requests_tx.send(request).is_err()
            {
                break;
            }
        }
    });
    requests_rx
}

fn run_spike(
    command_id: CommandId,
    request_id: RequestId,
    evaluation_id: EvaluationId,
    requests_rx: &mpsc::Receiver<ClientRequest>,
) -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new()?;
    let empty = GraphDocument::new(GraphId::new());
    let envelope = GraphCommandEnvelope {
        command_id,
        graph_id: empty.id,
        expected_revision: empty.revision,
        command: GraphCommand::AddNode {
            instance: fixture.node()?,
        },
    };
    let applied = apply_graph_command(
        &empty,
        &envelope,
        &fixture.definitions,
        &fixture.value_types,
    )?;
    emit("command-response", command_applied(&applied)?)?;

    let rejected = apply_graph_command(
        &applied.graph,
        &envelope,
        &fixture.definitions,
        &fixture.value_types,
    )
    .expect_err("the original expected revision must now be stale");
    emit("command-response", command_rejected(command_id, &rejected)?)?;

    let project = ProjectDocument::new(ProjectId::new(), "Swift bridge spike", applied.graph)?;
    let project_json = project.to_pretty_json()?;
    let graph_export = project.export_node_graph("Shared bridge graph")?;
    let graph_json = graph_export.to_pretty_json()?;
    emit(
        "portable-documents",
        json!({
            "project": serde_json::from_str::<Value>(&project_json)?,
            "node_graph": serde_json::from_str::<Value>(&graph_json)?,
            "project_digest": project.digest()?,
            "node_graph_digest": graph_export.digest()?,
        }),
    )?;

    let request = EvaluationRequest {
        request_id,
        evaluation_id,
        graph_id: project.graph.id,
        revision: project.graph.revision,
        environment: canonical_digest(&"swift-bridge-spike-environment-v1")?,
    };
    let cancellation = CancellationToken::default();
    let mut cancellation_handshake_done = false;
    let result = evaluate_graph(
        &project.graph,
        &request,
        &fixture.definitions,
        &fixture.value_types,
        &TextRuntime,
        &cancellation,
        |progress| {
            emit(
                "evaluation-progress",
                EvaluationProgressDto::from(&progress),
            )
            .expect("stdout must remain writable during the spike");
            if progress.phase == EvaluationPhase::Evaluating && !cancellation_handshake_done {
                cancellation_handshake_done = true;
                if let Ok(ClientRequest::Cancel {
                    request_id: cancel_request,
                    evaluation_id: cancel_evaluation,
                }) = requests_rx.recv_timeout(Duration::from_secs(5))
                    && cancel_request == request_id
                    && cancel_evaluation == evaluation_id
                {
                    cancellation.cancel();
                }
            }
        },
    );
    let error = result.expect_err("Swift must cancel the spike evaluation");
    emit("evaluation-error", evaluation_error(&error)?)?;
    emit(
        "complete",
        json!({
            "request_id": request_id,
            "evaluation_id": evaluation_id,
            "cancelled": cancellation.is_cancelled(),
        }),
    )?;
    Ok(())
}

fn schema(id: &str) -> Result<SchemaRef, Box<dyn Error>> {
    Ok(SchemaRef {
        id: SchemaId::parse(id)?,
        version: SchemaVersion::first(),
    })
}

fn emit<T: Serialize>(event: &'static str, payload: T) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &Event { event, payload })?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
