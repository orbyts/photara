//! Deterministic append compiler for the additive persisted-input layout.
use super::{
    Limits,
    input::{OriginalPackage, OriginalProof, PlannerInputs, PreparedChange, TreeGeometry},
    wire,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, hash, key, reference, text};
fn get(pool: &BTreeMap<wire::Key, Value>, r: &Value) -> Result<Value> {
    pool.get(&key(r)?)
        .cloned()
        .ok_or("decoder original object absent")
}
struct Builder {
    project: String,
    geometry: TreeGeometry,
    limits: Limits,
    pool: BTreeMap<wire::Key, Value>,
    files: BTreeMap<String, Vec<u8>>,
    locations: BTreeMap<String, BTreeMap<wire::Key, Value>>,
    corridors: BTreeMap<String, u64>,
}
impl Builder {
    fn object(&self, name: &str, mut value: Value) -> Value {
        value["schema"] = json!({"id":name,"version":1});
        value["project_id"] = json!(self.project);
        value["extensions"] = json!({});
        value
    }

    fn add(&mut self, v: Value) -> Result<Value> {
        let r = reference(&v);
        let k = key(&r)?;
        ensure(
            self.pool.contains_key(&k) || self.pool.len() < self.limits.max_objects,
            "object pool budget",
        )?;
        self.pool.insert(k, v);
        Ok(r)
    }
    #[allow(
        clippy::many_single_char_names,
        reason = "Frame offsets and canonical references in a bounded decoder"
    )]
    fn load(old: &OriginalPackage, inputs: &PlannerInputs, limits: Limits) -> Result<Self> {
        let mut b = Self {
            project: text(&old.manifest["project_id"])?.into(),
            geometry: inputs.geometry.clone(),
            limits,
            pool: BTreeMap::new(),
            files: BTreeMap::new(),
            locations: BTreeMap::new(),
            corridors: inputs
                .corridors
                .iter()
                .map(|c| {
                    Ok((
                        c.allocation_id.clone(),
                        wire::number(&json!(c.maximum_end))?,
                    ))
                })
                .collect::<Result<_>>()?,
        };
        ensure(
            old.allocations.len() <= limits.max_objects && old.loose.len() <= limits.max_objects,
            "original entry budget",
        )?;
        let mut frames = 0usize;
        let mut total = 0u64;
        let mut json_bytes = 0usize;
        for (id, a) in &old.allocations {
            total = total
                .checked_add(a.bytes.len() as u64)
                .ok_or("allocation byte overflow")?;
            ensure(
                a.bytes.len() as u64 <= limits.max_allocation_bytes
                    && total <= limits.max_total_allocation_bytes,
                "original allocation byte budget",
            )?;
            let mut locations = BTreeMap::new();
            let mut p = 0;
            while p < a.bytes.len() {
                frames = frames.checked_add(1).ok_or("frame budget")?;
                ensure(frames <= limits.max_frames, "frame budget")?;
                ensure(a.bytes.len() - p >= 16, "decoder original frame header")?;
                ensure(
                    &a.bytes[p..p + 8] == b"PS2PKD01"
                        && a.bytes[p + 8] <= 3
                        && a.bytes[p + 9..p + 12] == [0, 0, 0],
                    "original frame header",
                )?;
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
                    json_bytes = json_bytes.checked_add(n).ok_or("JSON byte overflow")?;
                    ensure(
                        json_bytes <= limits.semantic.max_total_json_bytes,
                        "original JSON total budget",
                    )?;
                    let v = crate::package::parse_canonical_json(
                        &a.bytes[p + 16..end],
                        limits.semantic.json,
                    )
                    .map_err(|_| "original canonical record budget")?;
                    let r = b.add(v)?;
                    locations.insert(key(&r)?,json!({"allocation_id":id,"arena":a.arena,"offset":p.to_string(),"byte_length":(end-p).to_string(),"record_sha256":hash(&a.bytes[p..end])}));
                }
                p = end;
            }
            b.files.insert(id.clone(), a.bytes.clone());
            b.locations.insert(id.clone(), locations);
        }
        for raw in old.loose.values() {
            json_bytes = json_bytes
                .checked_add(raw.len())
                .ok_or("JSON byte overflow")?;
            ensure(
                json_bytes <= limits.semantic.max_total_json_bytes,
                "original JSON total budget",
            )?;
            b.add(
                crate::package::parse_canonical_json(raw, limits.semantic.json)
                    .map_err(|_| "control canonical budget")?,
            )?;
        }
        Ok(b)
    }
    fn pack(&mut self, id: &str, arena: &str, items: Vec<(Value, u8)>) -> Result<()> {
        let id = id.to_owned();
        for (v, tag) in items {
            let r = self.add(v.clone())?;
            let k = key(&r)?;
            if self.locations[&id].contains_key(&k) {
                continue;
            }
            let bytes = encode(&v);
            ensure(
                bytes.len() <= self.limits.semantic.json.max_bytes,
                "generated record byte budget",
            )?;
            let projected = self
                .files
                .get(&id)
                .ok_or("allocation")?
                .len()
                .checked_add(16)
                .and_then(|n| n.checked_add(bytes.len()))
                .ok_or("allocation overflow")? as u64;
            ensure(
                projected <= self.limits.max_allocation_bytes
                    && self
                        .corridors
                        .get(&id)
                        .is_some_and(|end| projected.checked_add(16).is_some_and(|n| n <= *end)),
                "preflight corridor exhausted",
            )?;
            let total = self.files.iter().try_fold(0u64, |sum, (k, b)| {
                sum.checked_add(if k == &id { projected } else { b.len() as u64 })
                    .ok_or("allocation total overflow")
            })?;
            ensure(
                total <= self.limits.max_total_allocation_bytes,
                "total allocation byte budget",
            )?;
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
        ensure(
            entries.len() <= self.limits.max_entries && size > 0 && self.geometry.branch >= 2,
            "tree entry budget",
        )?;
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
            let v = self.object(
                &format!("{name}-leaf"),
                json!({"count":part.len().to_string(),"entries":part}),
            );
            let r = self.add(v.clone())?;
            nodes.push(v);
            level.push(json!({"first":part.first().map(|v|ek(v,kind)),"last":part.last().map(|v|ek(v,kind)),"count":part.len().to_string(),"child":r}));
        }
        while level.len() > 1 {
            let mut next = Vec::new();
            for part in level.chunks(self.geometry.branch as usize) {
                if part.len() == 1 {
                    next.push(part[0].clone());
                    continue;
                }
                let count = part.iter().try_fold(0u64, |a, v| {
                    a.checked_add(wire::number(&v["count"])?)
                        .ok_or("decoder tree count")
                })?;
                let v = self.object(
                    &format!("{name}-branch"),
                    json!({"count":count.to_string(),"children":part}),
                );
                let r = self.add(v.clone())?;
                nodes.push(v);
                next.push(json!({"first":part[0]["first"],"last":part.last().unwrap()["last"],"count":count.to_string(),"child":r}));
            }
            level = next;
        }
        Ok((level[0]["child"].clone(), nodes))
    }
    fn closure(&self, roots: &[Value]) -> Result<BTreeMap<wire::Key, Value>> {
        let mut out = BTreeMap::new();
        let mut pending = roots.to_vec();
        while let Some(r) = pending.pop() {
            let k = key(&r)?;
            if out.contains_key(&k) {
                continue;
            }
            ensure(out.len() < self.limits.max_objects, "closure object budget")?;
            let v = get(&self.pool, &r)?;
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
                id if !id.starts_with("photara.storage.")
                    && !id.starts_with("photara.resource.")
                    && !id.starts_with("photara.package.") =>
                {
                    let mut refs = Vec::new();
                    crate::package::v1_1::references(&v, &mut refs)
                        .map_err(|_| "legacy typed edges")?;
                    edges.extend(
                        refs.iter()
                            .map(|r| serde_json::to_value(r).expect("ObjectRef serialization")),
                    );
                }
                _ => {}
            }
            ensure(edges.len() <= self.limits.max_entries, "edge entry budget")?;
            for edge in edges {
                if !edge.is_null() {
                    pending.push(edge);
                }
            }
            ensure(
                pending.len() <= self.limits.max_objects,
                "pending closure budget",
            )?;
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

impl Builder {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Tree construction consumes the owned entry batch conceptually"
    )]
    fn physical_tree(
        &mut self,
        name: &str,
        entries: Vec<Value>,
        field: &str,
        allocation: &str,
    ) -> Result<Value> {
        ensure(
            entries.len() <= self.limits.max_entries
                && self.geometry.physical_leaf > 0
                && self.geometry.branch >= 2,
            "physical tree entry budget",
        )?;
        let mut level = Vec::new();
        let parts = if entries.is_empty() {
            vec![&[][..]]
        } else {
            entries
                .chunks(self.geometry.physical_leaf as usize)
                .collect()
        };
        for part in parts {
            let value = self.object(
                &format!("photara.storage.{name}-leaf"),
                json!({"count":part.len().to_string(),"entries":part}),
            );
            self.pack(allocation, "metadata", vec![(value.clone(), 3)])?;
            let physical = self.locations[allocation][&key(&reference(&value))?].clone();
            level.push(json!({"first":part.first().map(|v|v[field].clone()),"last":part.last().map(|v|v[field].clone()),"count":part.len().to_string(),"child":physical}));
        }
        while level.len() > 1 {
            let mut next = Vec::new();
            for part in level.chunks(self.geometry.branch as usize) {
                if part.len() == 1 {
                    next.push(part[0].clone());
                    continue;
                }
                let count = part.iter().try_fold(0u64, |n, v| {
                    n.checked_add(wire::number(&v["count"])?)
                        .ok_or("physical count overflow")
                })?;
                let value = self.object(
                    &format!("photara.storage.{name}-branch"),
                    json!({"count":count.to_string(),"children":part}),
                );
                self.pack(allocation, "metadata", vec![(value.clone(), 3)])?;
                let physical = self.locations[allocation][&key(&reference(&value))?].clone();
                next.push(json!({"first":part[0]["first"],"last":part.last().unwrap()["last"],"count":count.to_string(),"child":physical}));
            }
            level = next;
        }
        Ok(level[0]["child"].clone())
    }
}
pub(crate) struct Layout {
    pub files: BTreeMap<String, Vec<u8>>,
    pub target: Value,
    pub base_inventory: Value,
    pub root_placements: Value,
}
#[expect(
    clippy::too_many_lines,
    reason = "Finite deterministic layout preserves explicit stage dependency order"
)]
pub(crate) fn compile(
    old: &OriginalPackage,
    proof: &OriginalProof,
    prepared: &PreparedChange,
    inputs: &PlannerInputs,
    retained_controls: &BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
    limits: Limits,
) -> Result<Layout> {
    inputs.validate(old, proof, limits)?;
    let mut b = Builder::load(old, inputs, limits)?;
    ensure(
        !b.pool.values().any(|v| {
            v["schema"]["id"] == "photara.package.state-root"
                && v["root_id"] == inputs.active_root_id
        }),
        "retained root identity collision",
    )?;
    for mapping in &inputs.associations {
        ensure(
            !b.pool
                .values()
                .any(|v| v["association_id"] == mapping.replacement),
            "retained association collision",
        )?;
    }
    for v in prepared.authored.values() {
        b.add(v.clone())?;
    }
    b.add(prepared.receipt.clone())?;
    let root = &old.commit["root_set"];
    let mut active = proof.roles["active"].state.clone();
    let oldactive = active.clone();
    let mut rs = get(&b.pool, &active["resource_state"])?;
    let mut sources = get(&b.pool, &rs["retention_sources"])?;
    ensure(
        sources["schema"]["id"] == "photara.resource.selection-leaf",
        "bounded source association leaf codec",
    )?;
    let active_id = inputs.active_root_id.clone();
    let replacements = inputs
        .associations
        .iter()
        .map(|a| (&a.original, &a.replacement))
        .collect::<BTreeMap<_, _>>();
    let mut used_associations = BTreeSet::new();
    for e in sources["entries"]
        .as_array_mut()
        .ok_or("decoder associations")?
        .iter_mut()
    {
        let mut a = get(&b.pool, &e["record"])?;
        ensure(
            a["origin"] == json!({"kind":"authored","source_id":oldactive["root_id"]}),
            "unsupported active retention origin",
        )?;
        let original = text(&a["association_id"])?.to_owned();
        let replacement = replacements
            .get(&original)
            .ok_or("missing association replacement")?;
        used_associations.insert(original);
        a["association_id"] = json!(replacement);
        a["origin"] = json!({"kind":"authored","source_id":active_id});
        e["id"] = a["association_id"].clone();
        e["record"] = b.add(a)?;
    }
    sources["entries"]
        .as_array_mut()
        .ok_or("associations")?
        .sort_by_key(|v| v["id"].as_str().unwrap_or("").to_owned());
    rs["retention_sources"] = b.add(sources)?;
    active["resource_state"] = b.add(rs)?;
    ensure(
        used_associations.len() == inputs.associations.len(),
        "exact association replacements",
    )?;
    let mut receipts = proof.roles["active"].receipts.clone();
    receipts.push(prepared.receipt.clone());
    let entries=receipts.iter().map(|r|json!({"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"acceptance_ordinal":r["acceptance_ordinal"],"receipt":reference(r)})).collect::<Vec<_>>();
    let (ir, _) = b.tree(
        "photara.package.operation-id",
        entries.clone(),
        "operation_id",
        inputs.geometry.operation_leaf as usize,
    )?;
    let (or, _) = b.tree(
        "photara.package.operation-ordinal",
        entries,
        "acceptance_ordinal",
        inputs.geometry.operation_leaf as usize,
    )?;
    let prefix = hash(&encode(
        &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":oldactive["accepted"]["prefix_sha256"],"acceptance_ordinal":prepared.receipt["acceptance_ordinal"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]}),
    ));
    let accepted =
        json!({"through_ordinal":prepared.receipt["acceptance_ordinal"],"prefix_sha256":prefix});
    let mut index = b.object(
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
    active["root_id"] = json!(active_id);
    active["authored_revision"] = prepared.receipt["after"]["revision"].clone();
    active["authored"] = reference(authored);
    active["operation_index"] = b.add(index)?;
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
            inputs.geometry.semantic_leaf as usize,
        )?
        .0;
    let mut states = Vec::new();
    let active_ref = b.add(active.clone())?;
    for role in &inputs.roles {
        let state = if role.role == "active" {
            active_ref.clone()
        } else if role.role == "recovery" {
            root["active"].clone()
        } else {
            reference(&proof.roles[&role.role].state)
        };
        states.push(state);
    }
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
    let mut globals = proof
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

    for r in retained_controls.keys() {
        let k = (r.sha256.as_str().to_owned(), r.byte_length.get());
        globals.insert(
            k.clone(),
            b.pool.get(&k).cloned().ok_or("retained terminal control")?,
        );
    }

    let mut union = globals.clone();
    for records in &roles {
        union.extend(
            records
                .iter()
                .filter(|(_, v)| {
                    !v["schema"]["id"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with("photara.package.inventory-")
                })
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }
    let (base_inventory, globalnodes) = b.tree(
        "photara.package.inventory",
        union.values().map(reference).collect(),
        "ref",
        inputs.geometry.semantic_leaf as usize,
    )?;
    let ends = inputs
        .corridors
        .iter()
        .map(|c| {
            Ok((
                c.allocation_id.clone(),
                usize::try_from(wire::number(&json!(c.maximum_end))?)
                    .map_err(|_| "corridor platform bound")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut placements = Vec::new();
    for ((state, role), allocation) in states.iter().zip(roles).zip(&inputs.roles) {
        let mut records = role;
        records.extend(globals.clone());
        records.extend(
            globalnodes
                .iter()
                .map(|v| (key(&reference(v)).unwrap(), v.clone())),
        );
        let items = records
            .iter()
            .filter(|(k, _)| {
                !inputs
                    .shared_sealed
                    .iter()
                    .any(|id| b.locations[id].contains_key(*k))
            })
            .map(|(_, v)| (v.clone(), tag(v)))
            .collect();
        b.pack(&allocation.data, "data", items)?;
        let mut claims = Vec::new();
        let units = inputs.shared_sealed.iter().cloned().chain([
            allocation.data.clone(),
            allocation.locator.clone(),
            inputs.root_placement.clone(),
        ]);
        for id in units {
            let sealed = inputs.shared_sealed.contains(&id);
            let raw = if sealed { b.files[&id].as_slice() } else { &[] };
            let charge = if sealed {
                reference(
                    globals
                        .values()
                        .find(|v| {
                            v["schema"]["id"] == "photara.storage.sealed-charge"
                                && v["allocation_id"] == id
                        })
                        .ok_or("original sealed charge")?,
                )
            } else {
                Value::Null
            };
            claims.push(b.object("photara.storage.allocation-claim",json!({"allocation_id":id,"arena":old.allocations[&id].arena,"layout":"framed-json","owned_extent":if sealed{raw.len()}else{ends[&id]}.to_string(),"authenticated_prefix":{"byte_length":raw.len().to_string(),"sha256":hash(raw)},"sealed":sealed,"sealed_charge":charge})));
        }
        let (owner, ownnodes) = b.tree(
            "photara.storage.ownership",
            claims
                .iter()
                .map(|v| json!({"allocation_id":v["allocation_id"],"claim":reference(v)}))
                .collect(),
            "allocation_id",
            inputs.geometry.ownership_leaf as usize,
        )?;
        b.pack(
            &allocation.data,
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
        let mut locations = BTreeMap::new();
        for id in &inputs.shared_sealed {
            locations.extend(b.locations[id].clone());
        }
        locations.extend(b.locations[&allocation.data].clone());
        let entries=records.iter().map(|(k,v)|Ok(json!({"object":reference(v),"membership":if tag(v)==2{"ownership"}else{"semantic"},"physical":locations.get(k).ok_or("missing planned location")?}))).collect::<Result<Vec<_>>>()?;
        let locator = b.physical_tree("locator", entries, "object", &allocation.locator)?;
        placements.push(json!({"root":state,"root_id":get(&b.pool,state)?["root_id"],"locator":locator,"ownership":owner}));
    }
    placements.sort_by_key(|p| key(&p["root"]).unwrap());
    let root_placements =
        b.physical_tree("root-placement", placements, "root", &inputs.root_placement)?;
    let final_total = b.files.iter().try_fold(0u64, |n, (id, bytes)| {
        n.checked_add(ends.get(id).copied().unwrap_or(bytes.len()) as u64)
            .ok_or("allocation total overflow")
    })?;
    ensure(
        final_total <= limits.max_total_allocation_bytes,
        "total target byte budget",
    )?;
    for (id, end) in &ends {
        let f = b.files.get_mut(id).ok_or("writable original allocation")?;
        let gap = end
            .checked_sub(f.len())
            .and_then(|v| v.checked_sub(16))
            .ok_or("preflight corridor exhausted")?;
        let length = u32::try_from(gap).map_err(|_| "padding frame bound")?;
        f.extend(b"PS2PKD01");
        f.extend([0, 0, 0, 0]);
        f.extend(length.to_le_bytes());
        f.resize(*end, 0);
    }
    Ok(Layout {
        files: b.files,
        target: json!({"active":active_ref,"recovery":root["active"],"pinned_roots":root["pinned_roots"],"operation_index":active["operation_index"],"conversion_source":root["conversion_source"]}),
        base_inventory,
        root_placements,
    })
}

pub(super) fn original_objects(
    old: &OriginalPackage,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<BTreeMap<wire::Key, Value>> {
    Ok(Builder::load(old, inputs, limits)?.pool)
}
