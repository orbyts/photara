//! Deterministic specimen decoder. Inputs are authenticated old bytes and actual Core output.
//! No prospective corpus, future allocation bytes, or diagnostic record cache is read.
use super::{prepare, resources, wire};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, hash, key, reference, text};
const STEP: usize = 262_144;
fn uid(n: u64) -> String {
    format!("96000000-0000-4000-8000-{n:012}")
}
fn aid(n: u64) -> String {
    uid(1000 + n)
}
fn object(name: &str, fields: Value) -> Value {
    let mut v = fields;
    v["schema"] = json!({"id":name,"version":1});
    v["project_id"] = json!(wire::PROJECT);
    v["extensions"] = json!({});
    v
}
fn get(pool: &BTreeMap<wire::Key, Value>, r: &Value) -> Result<Value> {
    pool.get(&key(r)?)
        .cloned()
        .ok_or("decoder original object absent")
}
struct Builder {
    pool: BTreeMap<wire::Key, Value>,
    files: BTreeMap<String, Vec<u8>>,
    locations: BTreeMap<String, BTreeMap<wire::Key, Value>>,
}
impl Builder {
    fn add(&mut self, v: Value) -> Value {
        let r = reference(&v);
        self.pool.insert(key(&r).unwrap(), v);
        r
    }
    #[allow(
        clippy::many_single_char_names,
        reason = "Frame offsets and canonical references in a bounded decoder"
    )]
    fn load(old: &wire::World) -> Result<Self> {
        let mut b = Self {
            pool: BTreeMap::new(),
            files: BTreeMap::new(),
            locations: BTreeMap::new(),
        };
        for (id, a) in &old.allocations {
            let mut locations = BTreeMap::new();
            let mut p = 0;
            while p < a.bytes.len() {
                ensure(a.bytes.len() - p >= 16, "decoder original frame header")?;
                let n = usize::try_from(u32::from_le_bytes(
                    a.bytes[p + 12..p + 16].try_into().unwrap(),
                ))
                .map_err(|_| "decoder length")?;
                let end = p
                    .checked_add(16)
                    .and_then(|v| v.checked_add(n))
                    .ok_or("decoder frame overflow")?;
                ensure(end <= a.bytes.len(), "decoder original frame end")?;
                if a.bytes[p + 8] != 0 {
                    let v = wire::parse(&a.bytes[p + 16..end])?;
                    let r = b.add(v);
                    locations.insert(key(&r)?,json!({"allocation_id":id,"arena":a.arena,"offset":p.to_string(),"byte_length":(end-p).to_string(),"record_sha256":hash(&a.bytes[p..end])}));
                }
                p = end;
            }
            b.files.insert(id.clone(), a.bytes.clone());
            b.locations.insert(id.clone(), locations);
        }
        for raw in old.loose.values() {
            b.add(wire::parse(raw)?);
        }
        Ok(b)
    }
    fn pack(&mut self, n: u64, arena: &str, items: Vec<(Value, u8)>) -> Result<()> {
        let id = aid(n);
        for (v, tag) in items {
            let r = self.add(v.clone());
            let k = key(&r)?;
            if self.locations[&id].contains_key(&k) {
                continue;
            }
            let bytes = encode(&v);
            let len = u32::try_from(bytes.len()).map_err(|_| "decoder frame body bound")?;
            let mut frame = b"PS2PKD01".to_vec();
            frame.extend([tag, 0, 0, 0]);
            frame.extend(len.to_le_bytes());
            frame.extend(bytes);
            let file = self
                .files
                .get_mut(&id)
                .ok_or("decoder admitted allocation")?;
            let physical = json!({"allocation_id":id,"arena":arena,"offset":file.len().to_string(),"byte_length":frame.len().to_string(),"record_sha256":hash(&frame)});
            file.extend(frame);
            self.locations.get_mut(&id).unwrap().insert(k, physical);
        }
        Ok(())
    }
    fn tree(
        &mut self,
        name: &str,
        mut entries: Vec<Value>,
        kind: &str,
        size: usize,
    ) -> Result<(Value, Vec<Value>)> {
        fn ek(v: &Value, kind: &str) -> Value {
            match kind {
                "ref" => v.clone(),
                _ => v[kind].clone(),
            }
        }
        entries.sort_by(|a, b| {
            let a = ek(a, kind);
            let b = ek(b, kind);
            if kind == "ref" {
                key(&a).unwrap().cmp(&key(&b).unwrap())
            } else if kind == "acceptance_ordinal" {
                wire::number(&a).unwrap().cmp(&wire::number(&b).unwrap())
            } else {
                text(&a).unwrap().cmp(text(&b).unwrap())
            }
        });
        let mut nodes = Vec::new();
        let mut level = Vec::new();
        let chunks = if entries.is_empty() {
            vec![&[][..]]
        } else {
            entries.chunks(size).collect()
        };
        for part in chunks {
            let v = object(
                &format!("{name}-leaf"),
                json!({"count":part.len().to_string(),"entries":part}),
            );
            let r = self.add(v.clone());
            nodes.push(v);
            level.push(json!({"first":part.first().map(|v|ek(v,kind)),"last":part.last().map(|v|ek(v,kind)),"count":part.len().to_string(),"child":r}));
        }
        while level.len() > 1 {
            let mut next = Vec::new();
            for part in level.chunks(4) {
                if part.len() == 1 {
                    next.push(part[0].clone());
                    continue;
                }
                let count = part.iter().try_fold(0u64, |a, v| {
                    a.checked_add(wire::number(&v["count"])?)
                        .ok_or("decoder tree count")
                })?;
                let v = object(
                    &format!("{name}-branch"),
                    json!({"count":count.to_string(),"children":part}),
                );
                let r = self.add(v.clone());
                nodes.push(v);
                next.push(json!({"first":part[0]["first"],"last":part.last().unwrap()["last"],"count":count.to_string(),"child":r}));
            }
            level = next;
        }
        Ok((level[0]["child"].clone(), nodes))
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep exact typed edge dispatch together"
    )]
    fn closure(&self, roots: &[Value]) -> Result<BTreeMap<wire::Key, Value>> {
        #[allow(
            clippy::too_many_lines,
            reason = "Keep exact typed edge dispatch together"
        )]
        fn walk(b: &Builder, r: &Value, out: &mut BTreeMap<wire::Key, Value>) -> Result<()> {
            let k = key(r)?;
            if out.contains_key(&k) {
                return Ok(());
            }
            ensure(out.len() < 4096, "decoder closure bound")?;
            let v = get(&b.pool, r)?;
            out.insert(k, v.clone());
            let mut edges = Vec::new();
            match text(&v["schema"]["id"])? {
                "photara.package.state-root" => {
                    for f in [
                        "authored",
                        "history",
                        "resource_state",
                        "operation_index",
                        "inventory",
                    ] {
                        edges.push(v[f].clone());
                    }
                }
                "photara.project.authored" => {
                    for f in [
                        "party_assignments",
                        "location_assignments",
                        "asset_ledger",
                        "resource_ledger",
                        "context",
                    ] {
                        edges.push(v[f].clone());
                    }
                    for g in v["graphs"].as_array().ok_or("decoder graphs")? {
                        edges.push(g["document"].clone());
                    }
                }
                "photara.project.saved-graph" => {
                    edges.push(v["context"].clone());
                    for f in ["required_packages", "node_contracts"] {
                        for x in v[f].as_array().ok_or("decoder contracts")? {
                            edges.push(x["manifest"].clone());
                        }
                    }
                }
                "photara.context.authored" => {
                    for v in v["node_contexts"].as_array().ok_or("decoder context")? {
                        edges.push(v["context"].clone());
                    }
                }
                "photara.resource.state" => {
                    for f in [
                        "identities",
                        "working_bindings",
                        "versions",
                        "backings",
                        "requirements",
                        "retention_sources",
                    ] {
                        edges.push(v[f].clone());
                    }
                }
                "photara.resource.selection-leaf" => {
                    for e in v["entries"].as_array().ok_or("decoder resource leaf")? {
                        edges.push(e["record"].clone());
                    }
                }
                "photara.resource.selection-branch" => {
                    for e in v["children"].as_array().ok_or("decoder resource branch")? {
                        edges.push(e["child"].clone());
                    }
                }
                "photara.resource.retention-source" => edges.push(v["requirements"].clone()),
                "photara.resource.captured-version" => edges.push(v["capture_evidence"].clone()),
                "photara.resource.backing" => edges.push(v["publication_evidence"].clone()),
                "photara.package.operation-index" => {
                    edges.push(v["by_id"].clone());
                    edges.push(v["by_ordinal"].clone());
                }
                "photara.package.operation-id-leaf" | "photara.package.operation-ordinal-leaf" => {
                    for e in v["entries"].as_array().ok_or("decoder operation leaf")? {
                        edges.push(e["receipt"].clone());
                    }
                }
                "photara.package.operation-id-branch"
                | "photara.package.operation-ordinal-branch"
                | "photara.package.inventory-branch" => {
                    for e in v["children"].as_array().ok_or("decoder branch")? {
                        edges.push(e["child"].clone());
                    }
                }
                "photara.package.inventory-leaf" => edges.extend(
                    v["entries"]
                        .as_array()
                        .ok_or("decoder inventory leaf")?
                        .clone(),
                ),
                _ => {}
            }
            for e in edges {
                if !e.is_null() {
                    walk(b, &e, out)?;
                }
            }
            Ok(())
        }
        let mut out = BTreeMap::new();
        for r in roots {
            walk(self, r, &mut out)?;
        }
        Ok(out)
    }
}
fn tag(v: &Value) -> u8 {
    if v["schema"]["id"].as_str().is_some_and(|s| {
        s.starts_with("photara.storage.") || s.starts_with("photara.package.retained-file-charge")
    }) {
        2
    } else {
        1
    }
}
pub(super) struct Plan {
    pub(super) files: BTreeMap<String, Vec<u8>>,
    pub(super) target: Value,
    pub(super) base_inventory: Value,
    pub(super) root_placements: Value,
}
#[allow(
    clippy::too_many_lines,
    reason = "Deterministic pure append layout from authenticated base and Core output"
)]
pub(super) fn regenerate(
    old: &wire::World,
    proof: &wire::Proof,
    prepared: &prepare::Prepared,
) -> Result<Plan> {
    let mut b = Builder::load(old)?;
    for v in prepared.authored.values() {
        b.add(v.clone());
    }
    b.add(prepared.receipt.clone());
    let root = &old.commit["root_set"];
    let mut active = proof.roles["active"].state.clone();
    let oldactive = active.clone();
    let mut rs = get(&b.pool, &active["resource_state"])?;
    let mut sources = get(&b.pool, &rs["retention_sources"])?;
    ensure(
        sources["schema"]["id"] == "photara.resource.selection-leaf",
        "bounded source association leaf codec",
    )?;
    for (i, e) in sources["entries"]
        .as_array_mut()
        .ok_or("decoder associations")?
        .iter_mut()
        .enumerate()
    {
        let mut a = get(&b.pool, &e["record"])?;
        a["association_id"] = json!(uid(
            8000 + u64::try_from(i).map_err(|_| "association ordinal")?
        ));
        a["origin"]["source_id"] = json!(uid(8001));
        e["id"] = a["association_id"].clone();
        e["record"] = b.add(a);
    }
    rs["retention_sources"] = b.add(sources);
    active["resource_state"] = b.add(rs);
    resources::verify(&active["resource_state"], &uid(8001), |r| get(&b.pool, r))?;
    let mut receipts = proof.roles["active"].receipts.clone();
    receipts.push(prepared.receipt.clone());
    let entries=receipts.iter().map(|r|json!({"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"acceptance_ordinal":r["acceptance_ordinal"],"receipt":reference(r)})).collect::<Vec<_>>();
    let (ir, _) = b.tree(
        "photara.package.operation-id",
        entries.clone(),
        "operation_id",
        1,
    )?;
    let (or, _) = b.tree(
        "photara.package.operation-ordinal",
        entries,
        "acceptance_ordinal",
        1,
    )?;
    let prefix = hash(&encode(
        &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":oldactive["accepted"]["prefix_sha256"],"acceptance_ordinal":prepared.receipt["acceptance_ordinal"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]}),
    ));
    let accepted =
        json!({"through_ordinal":prepared.receipt["acceptance_ordinal"],"prefix_sha256":prefix});
    let mut index = object(
        "photara.package.operation-index",
        json!({"library_id":root["library_id"],"bootstrap_sha256":root["bootstrap_sha256"],"accepted":accepted,"by_id":ir,"by_ordinal":or}),
    );
    index["schema"]["version"] = json!(2);
    let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":prepared.receipt["journal_id"],"sequence":prepared.receipt["journal_sequence"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]});
    let journal = json!({"journal_id":prepared.receipt["journal_id"],"through_sequence":prepared.receipt["journal_sequence"],"prefix_sha256":hash(&encode(&json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":oldactive["journal_inclusion"]["prefix_sha256"],"frame_sha256":hash(&encode(&frame))}))),"resulting_authored_revision":prepared.receipt["after"]["revision"],"resulting_authored_sha256":prepared.receipt["after"]["digest"]});
    let authored = prepared
        .authored
        .iter()
        .find(|(k, _)| k.0 == prepared.receipt["after"]["digest"])
        .ok_or("actual new authored body")?
        .1;
    active["root_id"] = json!(uid(8001));
    active["authored_revision"] = prepared.receipt["after"]["revision"].clone();
    active["authored"] = reference(authored);
    active["operation_index"] = b.add(index);
    active["accepted"] = accepted;
    active["journal_inclusion"] = journal;
    active["predecessor"] = json!({"root_sha256":root["active"]["sha256"],"authored_revision":oldactive["authored_revision"]});
    let semantic = b.closure(&[
        active["authored"].clone(),
        active["history"].clone(),
        active["resource_state"].clone(),
        active["operation_index"].clone(),
    ])?;
    active["inventory"] = b
        .tree(
            "photara.package.inventory",
            semantic.values().map(reference).collect(),
            "ref",
            4,
        )?
        .0;
    let retained = proof
        .roles
        .iter()
        .find(|(label, _)| label.starts_with("pin:"))
        .ok_or("original retained root")?
        .1
        .state
        .clone();
    let states = [
        b.add(active.clone()),
        root["active"].clone(),
        reference(&retained),
    ];
    let roles = states
        .iter()
        .map(|r| b.closure(std::slice::from_ref(r)))
        .collect::<Result<Vec<_>>>()?;
    let oldsemantic = proof
        .roles
        .values()
        .flat_map(|p| {
            p.semantic
                .iter()
                .cloned()
                .chain([key(&reference(&p.state)).unwrap()])
        })
        .collect::<BTreeSet<_>>();
    let oldoverlay = old.loose(&root["inventory"])?;
    let controls = oldoverlay["controls"]
        .as_array()
        .ok_or("original controls")?
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    let globals = proof
        .global
        .iter()
        .filter(|k| !oldsemantic.contains(*k) && !controls.contains(*k))
        .map(|k| {
            Ok((
                k.clone(),
                b.pool.get(k).cloned().ok_or("original global record")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut union = globals.clone();
    for records in &roles {
        union.extend(
            records
                .iter()
                .filter(|(_, v)| {
                    !v["schema"]["id"]
                        .as_str()
                        .unwrap()
                        .starts_with("photara.package.inventory-")
                })
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }
    let (base_inventory, globalnodes) = b.tree(
        "photara.package.inventory",
        union.values().map(reference).collect(),
        "ref",
        4,
    )?;
    let ends = (2..9)
        .map(|n| {
            Ok((
                aid(n),
                b.files[&aid(n)]
                    .len()
                    .checked_add(STEP)
                    .ok_or("decoder admitted corridor overflow")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let sealed = globals
        .values()
        .find(|v| v["schema"]["id"] == "photara.storage.sealed-charge")
        .ok_or("original sealed charge")?
        .clone();
    let mut placements = Vec::new();
    for ((state, role), n) in states.iter().zip(roles).zip([2, 4, 6]) {
        let mut records = role;
        records.extend(globals.clone());
        records.extend(
            globalnodes
                .iter()
                .map(|v| (key(&reference(v)).unwrap(), v.clone())),
        );
        let items = records
            .iter()
            .filter(|(k, _)| !b.locations[&aid(1)].contains_key(*k))
            .map(|(_, v)| (v.clone(), tag(v)))
            .collect();
        b.pack(n, "data", items)?;
        let mut claims = Vec::new();
        for unit in [1, n, n + 1, 8] {
            let issealed = unit == 1;
            let raw = if issealed {
                b.files[&aid(1)].as_slice()
            } else {
                &[]
            };
            claims.push(object("photara.storage.allocation-claim",json!({"allocation_id":aid(unit),"arena":if unit==n+1||unit==8{"metadata"}else{"data"},"layout":"framed-json","owned_extent":if issealed{raw.len()}else{ends[&aid(unit)]}.to_string(),"authenticated_prefix":{"byte_length":raw.len().to_string(),"sha256":hash(raw)},"sealed":issealed,"sealed_charge":if issealed{reference(&sealed)}else{Value::Null}})));
        }
        let (owner, ownnodes) = b.tree(
            "photara.storage.ownership",
            claims
                .iter()
                .map(|v| json!({"allocation_id":v["allocation_id"],"claim":reference(v)}))
                .collect(),
            "allocation_id",
            2,
        )?;
        b.pack(
            n,
            "data",
            claims
                .iter()
                .chain(&ownnodes)
                .map(|v| (v.clone(), 2))
                .collect(),
        )?;
        records.extend(
            claims
                .iter()
                .chain(&ownnodes)
                .map(|v| (key(&reference(v)).unwrap(), v.clone())),
        );
        let mut locations = b.locations[&aid(1)].clone();
        locations.extend(b.locations[&aid(n)].clone());
        let entries=records.iter().map(|(k,v)|json!({"object":reference(v),"membership":if tag(v)==2{"ownership"}else{"semantic"},"physical":locations[k]})).collect::<Vec<_>>();
        let mut children = Vec::new();
        for part in entries.chunks(64) {
            let v = object(
                "photara.storage.locator-leaf",
                json!({"count":part.len().to_string(),"entries":part}),
            );
            b.pack(n + 1, "metadata", vec![(v.clone(), 3)])?;
            children.push(json!({"first":part[0]["object"],"last":part.last().unwrap()["object"],"count":part.len().to_string(),"child":b.locations[&aid(n+1)][&key(&reference(&v))?]}));
        }
        let locator = object(
            "photara.storage.locator-branch",
            json!({"count":entries.len().to_string(),"children":children}),
        );
        b.pack(n + 1, "metadata", vec![(locator.clone(), 3)])?;
        placements.push(json!({"root":state,"root_id":get(&b.pool,state)?["root_id"],"locator":b.locations[&aid(n+1)][&key(&reference(&locator))?],"ownership":owner}));
    }
    placements.sort_by_key(|p| key(&p["root"]).unwrap());
    let mut children = Vec::new();
    for entry in &placements {
        let v = object(
            "photara.storage.root-placement-leaf",
            json!({"count":"1","entries":[entry]}),
        );
        b.pack(8, "metadata", vec![(v.clone(), 3)])?;
        children.push(json!({"first":entry["root"],"last":entry["root"],"count":"1","child":b.locations[&aid(8)][&key(&reference(&v))?]}));
    }
    let r = object(
        "photara.storage.root-placement-branch",
        json!({"count":placements.len().to_string(),"children":children}),
    );
    b.pack(8, "metadata", vec![(r.clone(), 3)])?;
    let root_placements = b.locations[&aid(8)][&key(&reference(&r))?].clone();
    for n in 2..9 {
        let f = b.files.get_mut(&aid(n)).unwrap();
        let gap = ends[&aid(n)]
            .checked_sub(f.len())
            .and_then(|v| v.checked_sub(16))
            .ok_or("decoder preflight corridor exhausted")?;
        let length = u32::try_from(gap).map_err(|_| "decoder padding bound")?;
        f.extend(b"PS2PKD01");
        f.extend([0, 0, 0, 0]);
        f.extend(length.to_le_bytes());
        f.resize(ends[&aid(n)], 0);
    }
    Ok(Plan {
        files: b.files,
        target: json!({"active":states[0],"recovery":states[1],"pinned_roots":root["pinned_roots"],"operation_index":active["operation_index"],"conversion_source":root["conversion_source"]}),
        base_inventory,
        root_placements,
    })
}
