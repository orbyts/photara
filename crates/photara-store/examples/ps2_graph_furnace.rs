//! Disposable genuine Graph/journal/checkpoint furnace, not a production codec.
//! Every acknowledgement below is an unqualified model observation: `sync_all` is
//! not an approved platform durability profile. No external media is accessed.
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
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Instant,
};
#[path = "ps2_graph_furnace/index.rs"]
mod index;
#[path = "ps2_graph_furnace/oracle.rs"]
mod oracle;
use index::{
    authored, btree_build, btree_insert, lookup, make_root, prefix, radix_build, radix_insert,
    root, seq_append, seq_build, seq_node,
};
type Result<T> = std::result::Result<T, String>;
const FAN: usize = 16;
const TIME: &str = "2026-09-17T00:00:00.000Z";
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn id(n: u64) -> String {
    format!("73000000-0000-4000-8000-{n:012}")
}
fn key(id: &str) -> String {
    hash(format!("fixture.operation-id:{id}").as_bytes())
}
fn is_digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn bit(k: &str, n: u16) -> u8 {
    let b = k.as_bytes()[usize::from(n / 4)];
    let v = if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
    (v >> (3 - n % 4)) & 1
}
fn first_diff(a: &str, b: &str) -> u16 {
    (0..256).find(|&n| bit(a, n) != bit(b, n)).unwrap_or(256)
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
fn io<T>(r: std::io::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
struct Ref {
    offset: u64,
    len: u64,
    sha: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
struct Entry {
    key: String,
    id: String,
    request: String,
    ordinal: u64,
    receipt: Ref,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum Kind {
    Radix,
    Btree,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "fixture_kind")]
enum Node {
    Receipt {
        record: Record,
    },
    Leaf {
        entries: Vec<Entry>,
    },
    Radix {
        bit: u16,
        anchor: String,
        left: Ref,
        right: Ref,
    },
    Btree {
        level: u8,
        keys: Vec<String>,
        children: Vec<Ref>,
    },
    Sequence {
        level: u8,
        count: u64,
        children: Vec<Ref>,
    },
    Authored {
        value: String,
    },
    Manifest {
        authored: Ref,
        map: Ref,
        sequence: Ref,
    },
    Root {
        kind: Kind,
        count: u64,
        map: Ref,
        sequence: Ref,
        manifest: Ref,
    },
}
#[derive(Clone, Debug, Default, Serialize)]
struct Stats {
    reads: u64,
    read_bytes: u64,
    writes: u64,
    write_bytes: u64,
    envelope_bytes: u64,
    sync_calls: u64,
}
struct Store {
    dir: PathBuf,
    file: File,
    stats: Stats,
    staged: Vec<Node>,
}
impl Store {
    fn open(dir: &Path) -> Result<Self> {
        Ok(Self {
            dir: dir.into(),
            file: io(OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(dir.join("objects.pack")))?,
            stats: Stats::default(),
            staged: vec![],
        })
    }
    fn put(&mut self, n: &Node) -> Result<Ref> {
        let bytes = encode(n)?;
        let r = Ref {
            offset: u64::MAX - self.staged.len() as u64,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        self.staged.push(n.clone());
        Ok(r)
    }
    fn get(&mut self, r: &Ref) -> Result<Node> {
        if r.offset > u64::MAX / 2 {
            return self
                .staged
                .get(usize::try_from(u64::MAX - r.offset).map_err(|_| "staging offset")?)
                .cloned()
                .ok_or("staging ref".into());
        }
        ensure(r.len <= 2 * 1024 * 1024, "node size cap")?;
        io(self.file.seek(SeekFrom::Start(r.offset)))?;
        let mut bytes = vec![0; usize::try_from(r.len).map_err(|_| "node length")?];
        io(self.file.read_exact(&mut bytes))?;
        self.stats.reads += 1;
        self.stats.read_bytes += r.len;
        ensure(hash(&bytes) == r.sha, "object digest")?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    fn materialize(&mut self, r: &Ref, done: &mut BTreeMap<u64, Ref>) -> Result<Ref> {
        if r.offset < u64::MAX / 2 {
            return Ok(r.clone());
        }
        if let Some(r) = done.get(&r.offset) {
            return Ok(r.clone());
        }
        let mut n = self.get(r)?;
        match &mut n {
            Node::Leaf { entries } => {
                for e in entries {
                    e.receipt = self.materialize(&e.receipt, done)?;
                }
            }
            Node::Radix { left, right, .. } => {
                *left = self.materialize(left, done)?;
                *right = self.materialize(right, done)?;
            }
            Node::Btree { children, .. } | Node::Sequence { children, .. } => {
                for c in children {
                    *c = self.materialize(c, done)?;
                }
            }
            Node::Manifest {
                authored,
                map,
                sequence,
            } => {
                *authored = self.materialize(authored, done)?;
                *map = self.materialize(map, done)?;
                *sequence = self.materialize(sequence, done)?;
            }
            Node::Root {
                map,
                sequence,
                manifest,
                ..
            } => {
                *map = self.materialize(map, done)?;
                *sequence = self.materialize(sequence, done)?;
                *manifest = self.materialize(manifest, done)?;
            }
            Node::Receipt { .. } | Node::Authored { .. } => {}
        }
        let bytes = encode(&n)?;
        let offset = io(self.file.seek(SeekFrom::End(0)))?;
        io(self.file.write_all(&bytes))?;
        self.stats.writes += 1;
        self.stats.write_bytes += bytes.len() as u64;
        let actual = Ref {
            offset,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        done.insert(r.offset, actual.clone());
        Ok(actual)
    }
    fn flush(&mut self) -> Result<()> {
        io(self.file.sync_all())?;
        self.stats.sync_calls += 1;
        Ok(())
    }
    fn directory(&mut self) -> Result<()> {
        io(io(File::open(&self.dir))?.sync_all())?;
        self.stats.sync_calls += 1;
        Ok(())
    }
    fn envelope<T: Serialize>(&mut self, name: &str, v: &T, exclusive: bool) -> Result<()> {
        let bytes = encode(v)?;
        let mut o = OpenOptions::new();
        o.write(true);
        if exclusive {
            o.create_new(true);
        } else {
            o.create(true).truncate(true);
        }
        let mut f = io(o.open(self.dir.join(name)))?;
        io(f.write_all(&bytes))?;
        self.stats.envelope_bytes += bytes.len() as u64;
        io(f.sync_all())?;
        self.stats.sync_calls += 1;
        self.directory()
    }
    fn read<T: serde::de::DeserializeOwned>(&mut self, name: &str) -> Result<T> {
        let b = io(fs::read(self.dir.join(name)))?;
        self.stats.reads += 1;
        self.stats.read_bytes += b.len() as u64;
        serde_json::from_slice(&b).map_err(|e| e.to_string())
    }
    fn allocated(&self) -> Result<u64> {
        let mut n = 0;
        for e in io(fs::read_dir(&self.dir))? {
            n += io(io(e)?.metadata())?.blocks() * 512;
        }
        Ok(n)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct State {
    graph: GraphDocument,
    authored_revision: u64,
    prefix: String,
    count: u64,
}
impl State {
    fn initial(nodes: usize) -> Self {
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
    fn coordinate(&self) -> Result<AuthoredCoordinate> {
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
struct Record {
    id: String,
    request: String,
    ordinal: u64,
    intent: Value,
    before: Value,
    after: Value,
    unchanged: bool,
    fixture_inverse_of: Option<String>,
    previous_prefix: String,
}
fn execute(
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
fn request(state: &State, n: u64, command: GraphCommand) -> Result<MutationRequest> {
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
fn workload(state: &mut State, n: u64) -> Result<Record> {
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
fn replay(state: &mut State, r: &Record) -> Result<()> {
    ensure(
        r.intent["expected"] == serde_json::to_value(state.coordinate()?).unwrap(),
        "replay expected",
    )?;
    let envelope: GraphCommandEnvelope =
        serde_json::from_value(r.intent["command"]["envelope"].clone())
            .map_err(|e| e.to_string())?;
    let req = MutationRequest {
        version: 1,
        operation_id: decode(r.intent["operation_id"].clone()),
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Head {
    active: Ref,
    recovery: Ref,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Group {
    old: Head,
    records: Vec<Record>,
    final_state: State,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cut {
    None,
    #[cfg(test)]
    Journal,
    Pack,
    HeadReplace,
    HeadBarrier,
    Ack,
}
struct Fixture {
    st: Store,
    head: Head,
    state: State,
}
impl Fixture {
    fn baseline(dir: &Path, kind: Kind, n: u64, nodes: usize) -> Result<Self> {
        ensure(n > 0, "baseline count")?;
        let mut st = Store::open(dir)?;
        let mut state = State::initial(nodes);
        let mut entries = vec![];
        let mut receipts = vec![];
        for i in 1..=n {
            let record = workload(&mut state, i)?;
            let r = st.put(&Node::Receipt {
                record: record.clone(),
            })?;
            let r = st.materialize(&r, &mut BTreeMap::new())?;
            st.staged.clear();
            entries.push(Entry {
                key: key(&record.id),
                id: record.id,
                request: record.request,
                ordinal: i,
                receipt: r.clone(),
            });
            receipts.push(r);
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        let map = match kind {
            Kind::Radix => radix_build(&mut st, &entries)?,
            Kind::Btree => btree_build(&mut st, &entries)?,
        };
        let sequence = seq_build(&mut st, &receipts)?;
        let authored = st.put(&Node::Authored {
            value: String::from_utf8(encode(&state)?).unwrap(),
        })?;
        let root = make_root(&mut st, kind, n, map, sequence, authored)?;
        let active = st.materialize(&root, &mut BTreeMap::new())?;
        st.staged.clear();
        st.flush()?;
        let head = Head {
            active: active.clone(),
            recovery: active,
        };
        st.envelope("HEAD", &head, true)?;
        Ok(Self { st, head, state })
    }
    fn open(dir: &Path) -> Result<Self> {
        let mut st = Store::open(dir)?;
        let head: Head = st.read("HEAD")?;
        let active = root(&mut st, &head.active)?;
        let old = root(&mut st, &head.recovery)?;
        prefix(&mut st, &old.sequence, &active.sequence)?;
        for r in [&active, &old] {
            ensure(
                seq_node(&mut st, &r.sequence)?.1 == r.count,
                "root ordinal count",
            )?;
            let _ = st.get(&r.map)?;
            authored(&mut st, r)?;
        }
        let a = authored(&mut st, &active)?;
        let Node::Authored { value } = st.get(&a)? else {
            return Err("authored".into());
        };
        let state: State = serde_json::from_str(&value).map_err(|e| e.to_string())?;
        ensure(state.count == active.count, "state index mismatch")?;
        state.coordinate()?;
        Ok(Self { st, head, state })
    }
    fn lookup(&mut self, op: &str, digest: &str) -> Result<Record> {
        let r = root(&mut self.st, &self.head.active)?;
        let e = lookup(&mut self.st, &r.map, op, r.kind)?.ok_or("unknown operation")?;
        ensure(e.id == op && e.request == digest, "conflicting request")?;
        let Node::Receipt { record } = self.st.get(&e.receipt)? else {
            return Err("receipt kind".into());
        };
        ensure(
            record.id == op && record.request == digest && record.ordinal == e.ordinal,
            "entry receipt mismatch",
        )?;
        Ok(record)
    }
    fn group(&self, count: u64) -> Result<Group> {
        let mut state = self.state.clone();
        let mut records = vec![];
        for _ in 0..count {
            let n = state.count + 1;
            records.push(workload(&mut state, n)?);
        }
        Ok(Group {
            old: self.head.clone(),
            records,
            final_state: state,
        })
    }
    fn accept(&mut self, g: &Group) -> Result<()> {
        ensure(g.old == self.head, "stale group")?;
        ensure(
            !g.records.is_empty() && g.records.len() <= 32,
            "finite group",
        )?;
        // Resolve existing durable identities before accepting any new journal
        // liability. Core command replay alone cannot detect ID reuse.
        let selected = root(&mut self.st, &self.head.active)?;
        let mut ids = std::collections::BTreeSet::new();
        let mut state = self.state.clone();
        for r in &g.records {
            ensure(ids.insert(&r.id), "duplicate identity inside group")?;
            ensure(
                lookup(&mut self.st, &selected.map, &r.id, selected.kind)?.is_none(),
                "already indexed identity; resolve original receipt instead",
            )?;
            replay(&mut state, r)?;
        }
        ensure(state == g.final_state, "group state")?;
        self.st.envelope("journal.group", g, true)
    }
    fn checkpoint(&mut self, g: &Group, cut: Cut) -> Result<()> {
        let mut r = root(&mut self.st, &self.head.active)?;
        for record in &g.records {
            ensure(
                lookup(&mut self.st, &r.map, &record.id, r.kind)?.is_none(),
                "duplicate operation",
            )?;
            ensure(record.ordinal == r.count + 1, "ordinal gap")?;
            let receipt = self.st.put(&Node::Receipt {
                record: record.clone(),
            })?;
            let entry = Entry {
                key: key(&record.id),
                id: record.id.clone(),
                request: record.request.clone(),
                ordinal: record.ordinal,
                receipt: receipt.clone(),
            };
            r.map = match r.kind {
                Kind::Radix => radix_insert(&mut self.st, &r.map, entry)?,
                Kind::Btree => btree_insert(&mut self.st, &r.map, entry)?,
            };
            r.sequence = seq_append(&mut self.st, &r.sequence, receipt)?;
            r.count += 1;
        }
        let authored = self.st.put(&Node::Authored {
            value: String::from_utf8(encode(&g.final_state)?).unwrap(),
        })?;
        let next = make_root(&mut self.st, r.kind, r.count, r.map, r.sequence, authored)?;
        let active = self.st.materialize(&next, &mut BTreeMap::new())?;
        self.st.staged.clear();
        self.st.flush()?;
        let head = Head {
            active,
            recovery: self.head.active.clone(),
        };
        self.st.envelope("candidate", &head, false)?;
        if cut == Cut::Pack {
            return Err("cut after pack".into());
        }
        self.st.envelope("HEAD.next", &head, false)?;
        io(fs::rename(
            self.st.dir.join("HEAD.next"),
            self.st.dir.join("HEAD"),
        ))?;
        if cut == Cut::HeadReplace {
            return Err("cut after HEAD replace".into());
        }
        self.st.directory()?;
        if cut == Cut::HeadBarrier {
            return Err("cut after HEAD barrier".into());
        }
        self.head = head;
        self.state = g.final_state.clone();
        self.finish(g, cut)
    }
    fn finish(&mut self, g: &Group, cut: Cut) -> Result<()> {
        for r in &g.records {
            ensure(
                self.lookup(&r.id, &r.request)? == *r,
                "selected original receipt",
            )?;
        }
        ensure(
            self.state == g.final_state,
            "checkpoint current authored state",
        )?;
        self.st.envelope("checkpoint.ack",&json!({"head":hash(&encode(&self.head)?),"prefix":self.state.prefix,"through":self.state.count,"authored":self.state.coordinate()?,"qualified":false}),false)?;
        if cut == Cut::Ack {
            return Err("cut after ack barrier".into());
        }
        io(fs::remove_file(self.st.dir.join("journal.group")))?;
        self.st.directory()?;
        Ok(())
    }
    #[cfg(test)]
    fn recover(dir: &Path) -> Result<Self> {
        let mut f = Self::open(dir)?;
        let g: Group = f.st.read("journal.group")?;
        if f.head == g.old {
            let mut state = f.state.clone();
            for r in &g.records {
                replay(&mut state, r)?;
            }
            ensure(state == g.final_state, "recovery replay")?;
            f.checkpoint(&g, Cut::None)?;
        } else {
            let candidate: Head = f.st.read("candidate")?;
            ensure(
                f.head == candidate && f.head.recovery == g.old.active,
                "unknown HEAD; preserve journal",
            )?;
            f.st.flush()?;
            f.st.directory()?;
            f.finish(&g, Cut::None)?;
        }
        Ok(f)
    }
    // Measurement-only physical root rollover. No new authored operation and no
    // Saved acknowledgement; crash-safe root-only intent protocol is out of scope.
    fn turnover(&mut self) -> Result<()> {
        let r = root(&mut self.st, &self.head.active)?;
        let a = authored(&mut self.st, &r)?;
        let next = make_root(&mut self.st, r.kind, r.count, r.map, r.sequence, a)?;
        let active = self.st.materialize(&next, &mut BTreeMap::new())?;
        self.st.staged.clear();
        self.st.flush()?;
        let head = Head {
            active,
            recovery: self.head.active.clone(),
        };
        self.st.envelope("HEAD.next", &head, false)?;
        io(fs::rename(
            self.st.dir.join("HEAD.next"),
            self.st.dir.join("HEAD"),
        ))?;
        self.st.directory()?;
        self.head = head;
        Ok(())
    }
    #[cfg(test)]
    fn selected_checkpoint_matches_current(&mut self) -> Result<bool> {
        if self.st.dir.join("journal.group").exists() {
            let pending: Group = self.st.read("journal.group")?;
            if pending.final_state != self.state {
                return Ok(false);
            }
        }
        if !self.st.dir.join("checkpoint.ack").exists() {
            return Ok(false);
        }
        let ack: Value = self.st.read("checkpoint.ack")?;
        Ok(ack["head"] == json!(hash(&encode(&self.head)?))
            && ack["prefix"] == json!(self.state.prefix)
            && ack["through"] == json!(self.state.count))
    }
    fn audit(&mut self) -> Result<u64> {
        let r = root(&mut self.st, &self.head.active)?;
        let mut state = State::initial(self.state.graph.nodes.len());
        let mut stack = vec![r.sequence];
        while let Some(r) = stack.pop() {
            match self.st.get(&r)? {
                Node::Sequence { children, .. } => stack.extend(children.into_iter().rev()),
                Node::Receipt { record } => {
                    replay(&mut state, &record)?;
                    ensure(
                        self.lookup(&record.id, &record.request)? == record,
                        "audit map/ordinal",
                    )?;
                }
                _ => return Err("audit sequence kind".into()),
            }
        }
        ensure(state == self.state, "audit authored replay")?;
        Ok(state.count)
    }
}
fn sample(kind: Kind, n: u64, nodes: usize) -> Result<Value> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut f = Fixture::baseline(dir.path(), kind, n, nodes)?;
    let baseline_ms = started.elapsed().as_secs_f64() * 1000.;
    let baseline_bytes = io(f.st.file.metadata())?.len();
    let oldest = f.lookup(&id(1), &{
        let mut s = State::initial(nodes);
        workload(&mut s, 1)?.request
    })?;
    let mut policies = vec![];
    for batch in [1, 8, 32] {
        let mut samples = vec![];
        for _ in 0..4 {
            f.st.stats = Stats::default();
            let alloc = f.st.allocated()?;
            let t = Instant::now();
            let group = f.group(batch)?;
            let plan_us = t.elapsed().as_micros();
            let t = Instant::now();
            f.accept(&group)?;
            let journal_us = t.elapsed().as_micros();
            let t = Instant::now();
            f.checkpoint(&group, Cut::None)?;
            let checkpoint_us = t.elapsed().as_micros();
            samples.push(json!({"plan_us":plan_us,"journal_barrier_us":journal_us,"checkpoint_barrier_us":checkpoint_us,"stats":f.st.stats,"allocated_delta":i128::from(f.st.allocated()?)-i128::from(alloc)}));
        }
        policies.push(json!({"batch":batch,"samples":samples}));
    }
    let t = Instant::now();
    let mut cold = Fixture::open(dir.path())?;
    let open_us = t.elapsed().as_micros();
    let open_stats = cold.st.stats.clone();
    cold.st.stats = Stats::default();
    let t = Instant::now();
    ensure(
        cold.lookup(&oldest.id, &oldest.request)? == oldest,
        "old retry",
    )?;
    let retry_us = t.elapsed().as_micros();
    let retry_stats = cold.st.stats.clone();
    cold.st.stats = Stats::default();
    let t = Instant::now();
    ensure(
        cold.lookup(&oldest.id, &oldest.request)? == oldest,
        "warm retry",
    )?;
    let warm_retry_us = t.elapsed().as_micros();
    let mut turnovers = vec![];
    for _ in 0..4 {
        cold.st.stats = Stats::default();
        let t = Instant::now();
        cold.turnover()?;
        turnovers.push(json!({"us":t.elapsed().as_micros(),"stats":cold.st.stats}));
    }
    let t = Instant::now();
    let audited = cold.audit()?;
    let audit_ms = t.elapsed().as_secs_f64() * 1000.;
    Ok(
        json!({"kind":kind,"lifetime_operations":n,"graph_nodes":nodes,"baseline_ms":baseline_ms,"baseline_pack_bytes":baseline_bytes,"policies":policies,"fresh_handle_os_cached_open_us":open_us,"open_stats":open_stats,"old_retry_us":retry_us,"warm_retry_us":warm_retry_us,"retry_stats":retry_stats,"measurement_only_root_turnovers":turnovers,"full_replay_audit_count":audited,"full_replay_audit_ms":audit_ms,"final_pack_bytes":io(cold.st.file.metadata())?.len(),"filesystem_allocated_bytes":cold.st.allocated()?,"wall_ms":started.elapsed().as_secs_f64()*1000.,"qualified_accepted":false,"qualified_saved":false,"external_media_bytes_read":0,"baseline_construction":"bulk index, genuine sequential Graph commands and all original receipts; one final barrier, not per-operation acceptance","physical_backend":"immutable append ranges, staged reachable-node coalescing; no liveness/reserve/GC integration"}),
    )
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let counts = if args.is_empty() {
        vec![1000]
    } else {
        args.iter()
            .map(|s| s.parse().map_err(|_| "count".to_owned()))
            .collect::<Result<Vec<u64>>>()?
    };
    for n in counts {
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", sample(kind, n, 16)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_commands_cannot_reaccept_conflicting_or_group_duplicate_ids() {
        let d = tempfile::tempdir().unwrap();
        let mut f = Fixture::baseline(d.path(), Kind::Radix, 16, 2).unwrap();
        for ids in [vec![1], vec![17, 17]] {
            let mut state = f.state.clone();
            let mut records = vec![];
            for id in ids {
                let req = request(
                    &state,
                    id,
                    GraphCommand::SetNodePosition {
                        node_id: state.graph.nodes[0].id,
                        x: 99,
                        y: 0,
                    },
                )
                .unwrap();
                records.push(execute(&mut state, &req, None).unwrap());
            }
            let g = Group {
                old: f.head.clone(),
                records,
                final_state: state,
            };
            assert!(f.accept(&g).is_err());
            assert!(!d.path().join("journal.group").exists());
        }
    }
    #[test]
    fn older_checkpoint_is_not_current_after_new_journal_acceptance() {
        let d = tempfile::tempdir().unwrap();
        let mut f = Fixture::baseline(d.path(), Kind::Btree, 16, 2).unwrap();
        let g = f.group(1).unwrap();
        f.accept(&g).unwrap();
        f.checkpoint(&g, Cut::None).unwrap();
        assert!(f.selected_checkpoint_matches_current().unwrap());
        let newer = f.group(1).unwrap();
        f.accept(&newer).unwrap();
        assert!(!f.selected_checkpoint_matches_current().unwrap());
        // Retry resolves the original receipt even though its expected state is stale.
        assert_eq!(
            f.lookup(&g.records[0].id, &g.records[0].request).unwrap(),
            g.records[0]
        );
        assert!(
            execute(
                &mut f.state.clone(),
                &request(
                    &State::initial(2),
                    1,
                    GraphCommand::SetNodePosition {
                        node_id: f.state.graph.nodes[0].id,
                        x: 1,
                        y: 1
                    }
                )
                .unwrap(),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn malformed_group_refuses_before_any_journal_write() {
        let d = tempfile::tempdir().unwrap();
        let mut f = Fixture::baseline(d.path(), Kind::Radix, 16, 2).unwrap();
        let mut g = f.group(8).unwrap();
        g.records[3].ordinal += 1;
        assert!(f.accept(&g).is_err());
        assert!(!d.path().join("journal.group").exists());
        let mut g = f.group(8).unwrap();
        g.records[3] = g.records[2].clone();
        assert!(f.accept(&g).is_err());
        assert!(!d.path().join("journal.group").exists());
    }
    #[test]
    fn ps1_planner_agrees_on_change_noop_inverse_and_batch() {
        use package::planning::{
            CheckpointIds, IncarnationId, PackageNamingPolicy, PlanOutcome, PlanRequest,
            VerifiedClosure, WriteId, plan,
        };
        use photara_core::creation::PackageExtension;
        let incarnation = IncarnationId::parse(&id(900_000)).unwrap();
        let mut base = VerifiedClosure::verify(
            package::MemoryPackage::new(
                oracle::build(oracle::add_node),
                package::PackageLimits::default(),
            )
            .unwrap(),
            incarnation,
        )
        .unwrap();
        let mut graph = oracle::graph();
        let policy = PackageNamingPolicy {
            write_extension: PackageExtension::try_from("jprtest".to_owned()).unwrap(),
            legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
        };
        for (i, x) in [15, 15, 0, 21].into_iter().enumerate() {
            let cmd = GraphCommand::SetNodePosition {
                node_id: graph.nodes[0].id,
                x,
                y: 0,
            };
            let command = if i == 3 {
                GraphCommand::Batch {
                    commands: vec![cmd],
                }
            } else {
                cmd
            };
            let envelope = GraphCommandEnvelope {
                command_id: decode(json!(id(900_001 + i as u64))),
                graph_id: graph.id,
                expected_revision: graph.revision,
                command,
            };
            let applied = apply_graph_command(
                &graph,
                &envelope,
                &NodeDefinitionRegistry::default(),
                &ValueTypeRegistry::default(),
            )
            .unwrap();
            let mut normalized = applied.graph.clone();
            normalized.revision = graph.revision;
            let unchanged = normalized == graph;
            let req = MutationRequest {
                version: 1,
                operation_id: decode(json!(id(900_001 + i as u64))),
                expected: base.coordinate().clone(),
                command: AuthoredCommand::Graph {
                    envelope: Box::new(envelope),
                },
                updated_at: TIME.into(),
            };
            let outcome = plan(
                &base,
                PlanRequest {
                    mutation: &req,
                    expected_head: base.token(),
                    ids: CheckpointIds {
                        write_id: WriteId::parse(&id(910_000 + 2 * i as u64)).unwrap(),
                        commit_id: decode(json!(id(910_001 + 2 * i as u64))),
                    },
                    naming: &policy,
                    observed_extension: &policy.write_extension,
                },
                &NodeDefinitionRegistry::default(),
                &ValueTypeRegistry::default(),
            )
            .unwrap();
            match outcome {
                PlanOutcome::Unchanged(receipt) => {
                    assert!(unchanged);
                    assert_eq!(receipt.before(), receipt.after());
                    assert_eq!(
                        serde_json::to_value(receipt.request_digest()).unwrap(),
                        json!(hash(&encode(&req).unwrap()))
                    );
                }
                PlanOutcome::Checkpoint(p) => {
                    assert!(!unchanged);
                    graph = applied.graph;
                    assert_eq!(
                        serde_json::to_value(
                            &p.receipt().after().graphs[&graph.id].semantic_digest
                        )
                        .unwrap(),
                        json!(hash(&encode(&graph).unwrap()))
                    );
                    assert_eq!(p.receipt().before(), base.coordinate());
                    let files = p
                        .candidate()
                        .files()
                        .files()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_vec()))
                        .collect();
                    base = VerifiedClosure::verify(
                        package::MemoryPackage::new(files, package::PackageLimits::default())
                            .unwrap(),
                        incarnation,
                    )
                    .unwrap();
                }
            }
        }
        assert_eq!(graph.revision.get(), 3);
    }
    #[test]
    fn graph_noop_inverse_and_batch_are_real_commands() {
        let mut s = State::initial(2);
        let start = s.graph.clone();
        let changed = workload(&mut s, 1).unwrap();
        let unchanged = workload(&mut s, 2).unwrap();
        let inverse = workload(&mut s, 3).unwrap();
        assert!(!changed.unchanged);
        assert!(unchanged.unchanged);
        assert_eq!(unchanged.before, unchanged.after);
        assert_eq!(inverse.fixture_inverse_of, Some(id(1)));
        let mut graph = s.graph.clone();
        graph.revision = start.revision;
        assert_eq!(graph, start);
        assert_eq!(s.graph.revision.get(), 2);
        workload(&mut s, 4).unwrap();
        assert_eq!(s.graph.revision.get(), 3);
    }
    #[test]
    fn original_receipts_recover_at_each_cut() {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                Cut::Journal,
                Cut::Pack,
                Cut::HeadReplace,
                Cut::HeadBarrier,
                Cut::Ack,
            ] {
                let d = tempfile::tempdir().unwrap();
                let mut f = Fixture::baseline(d.path(), kind, 24, 2).unwrap();
                let g = f.group(8).unwrap();
                f.accept(&g).unwrap();
                if cut != Cut::Journal {
                    assert!(f.checkpoint(&g, cut).is_err());
                }
                drop(f);
                let mut recovered = Fixture::recover(d.path()).unwrap();
                assert_eq!(recovered.state, g.final_state);
                assert_eq!(recovered.audit().unwrap(), 32);
                for r in &g.records {
                    assert_eq!(recovered.lookup(&r.id, &r.request).unwrap(), *r);
                }
                assert!(!d.path().join("journal.group").exists());
            }
        }
    }
    #[test]
    fn conflicting_digest_torn_journal_and_unknown_head_refuse() {
        let d = tempfile::tempdir().unwrap();
        let mut f = Fixture::baseline(d.path(), Kind::Radix, 8, 2).unwrap();
        assert!(f.lookup(&id(1), &hash(b"conflict")).is_err());
        let g = f.group(1).unwrap();
        f.accept(&g).unwrap();
        let mut unknown = g.old.clone();
        unknown.recovery.sha = hash(b"unknown");
        f.st.envelope("HEAD", &unknown, false).unwrap();
        assert!(Fixture::recover(d.path()).is_err());
        assert!(d.path().join("journal.group").exists());
        f.st.envelope("HEAD", &g.old, false).unwrap();
        fs::write(d.path().join("journal.group"), b"{").unwrap();
        assert!(Fixture::recover(d.path()).is_err());
    }
}
