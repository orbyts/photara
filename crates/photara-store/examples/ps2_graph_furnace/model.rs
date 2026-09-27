//! Shared disposable genuine Core Graph command/receipt model. No permanent wire.
use photara_core::{
    GraphCommand, GraphCommandEnvelope, GraphDocument, NodeDefinitionRegistry, ValueTypeRegistry,
    apply_graph_command, canonical_json,
};
use photara_store::package::{
    self,
    planning::{AuthoredCommand, AuthoredCoordinate, GraphCoordinate, MutationRequest},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[path = "oracle.rs"]
#[allow(
    dead_code,
    reason = "PS1 package oracle is used only by the original furnace tests"
)]
pub(crate) mod oracle;
type Result<T> = std::result::Result<T, String>;
pub(crate) const TIME: &str = "2026-09-17T00:00:00.000Z";
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub(crate) fn id(n: u64) -> String {
    format!("73000000-0000-4000-8000-{n:012}")
}
fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.into()) }
}
fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    canonical_json(&serde_json::to_value(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct State {
    pub(crate) graph: GraphDocument,
    pub(crate) authored_revision: u64,
    pub(crate) prefix: String,
    pub(crate) count: u64,
}
impl State {
    pub(crate) fn initial(nodes: usize) -> Self {
        let mut graph = oracle::graph();
        let template = graph.nodes[0].clone();
        graph.nodes = (0..nodes)
            .map(|n| {
                let mut v = template.clone();
                v.id = decode(json!(id(1_000_000 + n as u64)));
                v
            })
            .collect();
        Self {
            graph,
            authored_revision: 1,
            prefix: hash(b"fixture-genesis"),
            count: 0,
        }
    }
    pub(crate) fn coordinate(&self) -> Result<AuthoredCoordinate> {
        let digest = hash(&encode(&self.graph)?);
        let parse = |s: &str| package::Sha256Hex::parse(s).map_err(|e| e.to_string());
        let graph = GraphCoordinate {
            revision: package::DecimalU64::parse(&self.graph.revision.get().to_string()).unwrap(),
            semantic_digest: parse(&digest)?,
            payload_digest: parse(&digest)?,
            envelope_digest: parse(&hash(&encode(
                &json!({"fixture_graph":self.graph,"updated_at":TIME}),
            )?))?,
        };
        Ok(AuthoredCoordinate {
            revision: package::DecimalU64::parse(&self.authored_revision.to_string()).unwrap(),
            authored_digest: parse(&hash(&encode(
                &json!({"fixture_graph":self.graph,"authored_revision":self.authored_revision}),
            )?))?,
            graphs: BTreeMap::from([(self.graph.id, graph)]),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Record {
    pub(crate) id: String,
    pub(crate) request: String,
    pub(crate) ordinal: u64,
    pub(crate) intent: Value,
    pub(crate) before: Value,
    pub(crate) after: Value,
    pub(crate) unchanged: bool,
    pub(crate) fixture_inverse_of: Option<String>,
    pub(crate) previous_prefix: String,
}
pub(crate) fn execute(
    state: &mut State,
    request: &MutationRequest,
    inverse: Option<String>,
) -> Result<Record> {
    ensure(
        request.expected == state.coordinate()?,
        "stale expected coordinate",
    )?;
    let AuthoredCommand::Graph { envelope } = &request.command else {
        return Err("fixture Graph commands only".into());
    };
    ensure(
        envelope.command_id.to_string() == request.operation_id.to_string(),
        "command identity",
    )?;
    let result = apply_graph_command(
        &state.graph,
        envelope,
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .map_err(|e| e.to_string())?;
    let mut normalized = result.graph.clone();
    normalized.revision = state.graph.revision;
    let unchanged = normalized == state.graph;
    let before = serde_json::to_value(state.coordinate()?).unwrap();
    if !unchanged {
        state.graph = result.graph;
        state.authored_revision += 1;
    }
    let record = Record {
        id: request.operation_id.to_string(),
        request: hash(&encode(request)?),
        ordinal: state.count + 1,
        intent: serde_json::to_value(request).unwrap(),
        before,
        after: serde_json::to_value(state.coordinate()?).unwrap(),
        unchanged,
        fixture_inverse_of: inverse,
        previous_prefix: state.prefix.clone(),
    };
    state.count += 1;
    state.prefix = hash(&encode(&record)?);
    Ok(record)
}
pub(crate) fn request(state: &State, n: u64, command: GraphCommand) -> Result<MutationRequest> {
    Ok(MutationRequest {
        version: 1,
        operation_id: decode(json!(id(n))),
        expected: state.coordinate()?,
        command: AuthoredCommand::Graph {
            envelope: Box::new(GraphCommandEnvelope {
                command_id: decode(json!(id(n))),
                graph_id: state.graph.id,
                expected_revision: state.graph.revision,
                command,
            }),
        },
        updated_at: TIME.into(),
    })
}
pub(crate) fn workload(state: &mut State, n: u64) -> Result<Record> {
    let signed = i64::try_from(n).map_err(|_| "operation bound")?;
    let slot = ((n / 8) as usize) % state.graph.nodes.len();
    let node = &state.graph.nodes[slot];
    let pos = &node.extensions["photara.graph-position"];
    let (x, y) = if n % 8 == 2 {
        (pos["x"].as_i64().unwrap(), pos["y"].as_i64().unwrap())
    } else if n % 8 == 3 {
        (signed - 3, -(signed - 3))
    } else {
        (signed, -signed)
    };
    let cmd = GraphCommand::SetNodePosition {
        node_id: node.id,
        x,
        y,
    };
    let command = if n % 8 == 4 {
        GraphCommand::Batch {
            commands: vec![
                cmd,
                GraphCommand::SetNodePosition {
                    node_id: state.graph.nodes[(slot + 1) % state.graph.nodes.len()].id,
                    x: x + 1,
                    y,
                },
            ],
        }
    } else {
        cmd
    };
    let req = request(state, n, command)?;
    execute(state, &req, if n % 8 == 3 { Some(id(n - 2)) } else { None })
}
pub(crate) fn replay(state: &mut State, r: &Record) -> Result<()> {
    ensure(
        r.intent["expected"] == serde_json::to_value(state.coordinate()?).unwrap(),
        "replay expected",
    )?;
    let envelope: GraphCommandEnvelope =
        serde_json::from_value(r.intent["command"]["envelope"].clone())
            .map_err(|e| e.to_string())?;
    let req = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(r.intent["operation_id"].clone())
            .map_err(|e| e.to_string())?,
        expected: state.coordinate()?,
        command: AuthoredCommand::Graph {
            envelope: Box::new(envelope),
        },
        updated_at: r.intent["updated_at"].as_str().ok_or("intent time")?.into(),
    };
    ensure(
        serde_json::to_value(&req).unwrap() == r.intent,
        "canonical request preservation",
    )?;
    ensure(
        execute(state, &req, r.fixture_inverse_of.clone())? == *r,
        "original receipt replay",
    )
}
