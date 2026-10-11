//! Deterministic append compiler for the additive persisted-input layout.
use super::shared::OriginalPackage;
use super::{
    Limits,
    input::{OriginalProof, PlannerInputs, PreparedChange, TreeGeometry},
    wire,
};
use serde_json::{Value, json};
#[path = "incremental.rs"]
pub(super) mod incremental;
#[path = "overlay.rs"]
pub(super) mod overlay;
#[path = "retained_inventory.rs"]
mod retained_inventory;
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, hash, key, reference, text};
#[cfg(test)]
thread_local! {
    static LOAD_WORK: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
#[cfg(test)]
pub(super) fn take_load_work() -> (usize, usize) {
    LOAD_WORK.with(|work| work.replace((0, 0)))
}
fn get(pool: &super::object_pool::ObjectPool, r: &Value) -> Result<Value> {
    pool.get(&key(r)?)
        .cloned()
        .ok_or("decoder original object absent")
}
pub(super) struct Builder {
    incremental: bool,
    overlays: bool,
    changed_order: BTreeMap<wire::Key, usize>,
    project: String,
    geometry: TreeGeometry,
    limits: Limits,
    pub(super) pool: super::object_pool::ObjectPool,
    files: BTreeMap<String, super::shared::Bytes>,
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
    pub(super) fn load(
        old: &OriginalPackage,
        inputs: &PlannerInputs,
        limits: Limits,
    ) -> Result<Self> {
        Self::load_cached(old, inputs, limits, None)
    }
    pub(super) fn load_cached(
        old: &OriginalPackage,
        inputs: &PlannerInputs,
        limits: Limits,
        mut memo: Option<&mut super::canonical_memo::CanonicalMemo>,
    ) -> Result<Self> {
        #[cfg(test)]
        LOAD_WORK.with(|work| {
            let (loads, parsed) = work.get();
            work.set((loads + 1, parsed));
        });
        let mut b = Self {
            incremental: inputs.codec != "photara.codec.ps2-repeatable-layout-v1",
            overlays: inputs.codec == "photara.codec.ps2-repeatable-layout-v3",
            changed_order: BTreeMap::new(),
            project: text(&old.manifest["project_id"])?.into(),
            geometry: inputs.geometry.clone(),
            limits,
            pool: super::object_pool::ObjectPool::default(),
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
                    let k = b.original(&a.bytes[p + 16..end], memo.as_deref_mut())?;
                    locations.insert(k,json!({"allocation_id":id,"arena":a.arena,"offset":p.to_string(),"byte_length":(end-p).to_string(),"record_sha256":hash(&a.bytes[p..end])}));
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
            b.original(raw, memo.as_deref_mut())?;
        }
        Ok(b)
    }
    pub(super) fn load_selected(
        original: &super::VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: Limits,
    ) -> Result<Self> {
        let old = &original.package;
        let mut b = Self {
            incremental: inputs.codec != "photara.codec.ps2-repeatable-layout-v1",
            overlays: inputs.codec == "photara.codec.ps2-repeatable-layout-v3",
            changed_order: BTreeMap::new(),
            project: text(&old.manifest["project_id"])?.into(),
            geometry: inputs.geometry.clone(),
            limits,
            pool: super::object_pool::ObjectPool::default(),
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

        let selected = original
            .selected_frames
            .as_ref()
            .ok_or("snapshot selected frame proof")?;
        ensure(
            selected.len() <= limits.max_frames
                && old.allocations.len() <= limits.max_objects
                && old.loose.len() <= limits.max_objects,
            "snapshot loader bounds",
        )?;
        let frames = crate::package::v1_3::FrameLimits {
            json: limits.semantic.json,
            max_frames: limits.max_frames,
            max_allocation_bytes: usize::try_from(limits.max_allocation_bytes)
                .map_err(|_| "allocation platform limit")?,
        };
        let mut total = 0u64;
        let mut json_bytes = 0usize;
        for (id, allocation) in &old.allocations {
            total = total
                .checked_add(allocation.bytes.len() as u64)
                .ok_or("allocation total")?;
            ensure(
                total <= limits.max_total_allocation_bytes
                    && allocation.bytes.len() as u64 <= limits.max_allocation_bytes,
                "snapshot allocation bound",
            )?;
            b.files.insert(id.clone(), allocation.bytes.clone());
            b.locations.insert(id.clone(), BTreeMap::new());
            let allocation_id =
                crate::package::PackageUuid::parse(id).map_err(|_| "allocation UUID")?;
            let arena =
                serde_json::from_value(json!(allocation.arena)).map_err(|_| "allocation arena")?;
            let reader = crate::package::v1_3::SelectedAllocation::open(
                allocation_id,
                arena,
                &allocation.bytes,
                frames,
            )
            .map_err(|_| "selected allocation")?;
            for (physical, kind) in selected
                .iter()
                .filter(|(r, _)| r.allocation_id == allocation_id)
            {
                let value = reader
                    .resolve(physical, *kind)
                    .map_err(|_| "selected frame bytes")?;
                let raw = encode(&value);
                json_bytes = json_bytes
                    .checked_add(raw.len())
                    .ok_or("snapshot JSON total")?;
                ensure(
                    json_bytes <= limits.semantic.max_total_json_bytes,
                    "snapshot JSON bound",
                )?;
                let reference = b.add(value)?;
                b.locations.get_mut(id).ok_or("allocation")?.insert(
                    key(&reference)?,
                    serde_json::to_value(physical).map_err(|_| "physical ref")?,
                );
            }
        }
        for raw in old.loose.values() {
            json_bytes = json_bytes
                .checked_add(raw.len())
                .ok_or("snapshot JSON total")?;
            ensure(
                json_bytes <= limits.semantic.max_total_json_bytes,
                "snapshot JSON bound",
            )?;
            b.original(raw, None)?;
        }
        Ok(b)
    }
    // Every physical occurrence still consumes frame/JSON budgets. A repeated
    // exact canonical digest reuses the already checked immutable body.
    fn original(
        &mut self,
        raw: &[u8],
        memo: Option<&mut super::canonical_memo::CanonicalMemo>,
    ) -> Result<wire::Key> {
        let k = (hash(raw), raw.len() as u64);
        if !self.pool.contains_key(&k) {
            ensure(
                self.pool.len() < self.limits.max_objects,
                "object pool budget",
            )?;
            let summary = if let Some(memo) = memo {
                memo.validate(
                    raw,
                    &k,
                    self.limits.semantic.json,
                    usize::try_from(self.limits.max_total_allocation_bytes).unwrap_or(usize::MAX)
                        / 4,
                )?
            } else {
                let value = crate::package::parse_canonical_json(raw, self.limits.semantic.json)
                    .map_err(|_| "original canonical record budget")?;
                count_decode();
                super::canonical_memo::Summary::from_value(&value)
            };
            self.pool.original(k.clone(), raw, summary);
        }
        Ok(k)
    }
    fn pack(&mut self, id: &str, arena: &str, mut items: Vec<(Value, u8)>) -> Result<()> {
        if self.incremental && arena == "data" {
            // Preserve all ordinary object slots. Changed logical tree slots use
            // final postorder, deduplicated across the existing forest build order.
            let slots = items
                .iter()
                .enumerate()
                .filter_map(|(i, (v, _))| {
                    self.changed_order
                        .get(&key(&reference(v)).ok()?)
                        .map(|rank| (i, *rank))
                })
                .collect::<Vec<_>>();
            let mut ordered = slots
                .iter()
                .map(|(i, rank)| (*rank, items[*i].clone()))
                .collect::<Vec<_>>();
            ordered.sort_by_key(|(rank, _)| *rank);
            for ((i, _), (_, item)) in slots.into_iter().zip(ordered) {
                items[i] = item;
            }
        }
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
            #[cfg(test)]
            if self
                .corridors
                .get(&id)
                .is_some_and(|end| projected.saturating_add(16) > *end)
            {
                eprintln!(
                    "corridor overflow allocation={id} projected={projected} ceiling={:?} record={}",
                    self.corridors.get(&id),
                    v["schema"]["id"]
                );
            }
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
    fn physical_value(&self, r: &Value) -> Result<Value> {
        let id = text(&r["allocation_id"])?;
        let bytes = self.files.get(id).ok_or("seed allocation")?;
        let start = usize::try_from(wire::number(&r["offset"])?).map_err(|_| "seed offset")?;
        let length =
            usize::try_from(wire::number(&r["byte_length"])?).map_err(|_| "seed length")?;
        let end = start.checked_add(length).ok_or("seed range")?;
        let frame = bytes.get(start..end).ok_or("seed truncated")?;
        ensure(
            r["arena"] == "metadata"
                && frame.len() >= 16
                && &frame[..8] == b"PS2PKD01"
                && frame[8..12] == [3, 0, 0, 0]
                && u32::from_le_bytes(frame[12..16].try_into().map_err(|_| "seed header")?)
                    as usize
                    == frame.len() - 16
                && hash(frame) == r["record_sha256"],
            "seed frame commitment",
        )?;
        let k = (hash(&frame[16..]), (frame.len() - 16) as u64);
        self.pool.get(&k).cloned().ok_or("seed canonical object")
    }
    fn inventory_seed(
        &mut self,
        entries: Vec<Value>,
        seed: &Value,
        purpose: &str,
    ) -> Result<(Value, Vec<Value>)> {
        if self.overlays {
            overlay::build(self, seed, entries, purpose, None)
        } else {
            self.tree_seed(
                "photara.package.inventory",
                entries,
                "ref",
                self.geometry.semantic_leaf as usize,
                Some(seed),
            )
        }
    }
    fn locator_seed(
        &mut self,
        entries: Vec<Value>,
        allocation: &str,
        seed: Option<&Value>,
        role: &str,
    ) -> Result<Value> {
        if self.overlays {
            overlay::build(
                self,
                seed.ok_or("overlay locator predecessor")?,
                entries,
                "",
                Some((allocation, role)),
            )
            .map(|v| v.0)
        } else {
            self.physical_seed("locator", entries, "object", allocation, seed)
        }
    }
    fn tree_seed(
        &mut self,
        name: &str,
        entries: Vec<Value>,
        kind: &str,
        size: usize,
        seed: Option<&Value>,
    ) -> Result<(Value, Vec<Value>)> {
        if self.incremental {
            incremental::tree(
                self,
                seed,
                entries,
                incremental::Spec {
                    name,
                    field: kind,
                    capacity: size,
                    allocation: None,
                },
            )
        } else {
            self.tree(name, entries, kind, size)
        }
    }
    fn physical_seed(
        &mut self,
        name: &str,
        entries: Vec<Value>,
        field: &str,
        allocation: &str,
        seed: Option<&Value>,
    ) -> Result<Value> {
        if self.incremental {
            let full = format!("photara.storage.{name}");
            incremental::tree(
                self,
                seed,
                entries,
                incremental::Spec {
                    name: &full,
                    field,
                    capacity: self.geometry.physical_leaf as usize,
                    allocation: Some(allocation),
                },
            )
            .map(|v| v.0)
        } else {
            self.physical_tree(name, entries, field, allocation)
        }
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
    #[expect(
        clippy::too_many_lines,
        reason = "Typed closure dispatch keeps legacy and additive dependencies explicit"
    )]
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
                    if v["schema"]["version"] == 3 {
                        edges.push(v["base"].clone());
                    }
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
                "photara.package.inventory-overlay" if v["schema"]["version"] == 2 => {
                    let resolved = overlay::resolve(self, &r, text(&v["purpose"])?, None)?;
                    for (node, body) in resolved.nodes {
                        out.insert(key(&node)?, body);
                    }
                    ensure(
                        out.len() <= self.limits.max_objects,
                        "overlay closure budget",
                    )?;
                    edges.extend(resolved.entries);
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
    pub files: BTreeMap<String, super::shared::Bytes>,
    pub target: Value,
    pub active_state: Value,
    pub base_count: u64,
    pub base_inventory: Value,
    pub root_placements: Value,
    pub retained_inventory: Option<Value>,
    pub inventory_nodes: BTreeSet<wire::Key>,
}
pub(crate) fn compile(
    b: Builder,
    old: &OriginalPackage,
    proof: &OriginalProof,
    prepared: &PreparedChange,
    inputs: &PlannerInputs,
    retained_controls: &BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
    limits: Limits,
) -> Result<Layout> {
    compile_change(
        b,
        old,
        proof,
        Change::Graph(prepared),
        inputs,
        retained_controls,
        limits,
    )
}

pub(super) fn compile_checkpoint(
    b: Builder,
    old: &OriginalPackage,
    proof: &OriginalProof,
    base: &Value,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<Layout> {
    compile_change(
        b,
        old,
        proof,
        Change::Checkpoint(base),
        inputs,
        &BTreeMap::new(),
        limits,
    )
}
#[derive(Clone, Copy)]
enum Change<'a> {
    Graph(&'a PreparedChange),
    Checkpoint(&'a Value),
}
#[expect(
    clippy::too_many_lines,
    reason = "Finite deterministic layout preserves explicit stage dependency order"
)]
fn compile_change(
    mut b: Builder,
    old: &OriginalPackage,
    proof: &OriginalProof,
    operation: Change<'_>,
    inputs: &PlannerInputs,
    retained_controls: &BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
    limits: Limits,
) -> Result<Layout> {
    inputs.validate_context(
        old,
        proof,
        limits,
        matches!(operation, Change::Graph(_))
            && proof.roles["active"].state["schema"]["version"] == 3,
    )?;
    ensure(
        !b.pool.collision(&inputs.active_root_id, true),
        "retained root identity collision",
    )?;
    for mapping in &inputs.associations {
        ensure(
            !b.pool.collision(&mapping.replacement, false),
            "retained association collision",
        )?;
    }
    if let Change::Graph(prepared) = &operation {
        for v in prepared.authored.values() {
            b.add(v.clone())?;
        }
        b.add(prepared.receipt.clone())?;
    }
    let root = &old.commit["root_set"];
    let retained = if b.incremental && matches!(operation, Change::Graph(_)) {
        Some(retained_inventory::build(&mut b, old, retained_controls)?)
    } else {
        None
    };
    let mut old_placements = BTreeMap::new();
    if b.incremental {
        let entries = incremental::audit(
            &b,
            &root["placement"]["root_placements"],
            incremental::Spec {
                name: "photara.storage.root-placement",
                field: "root",
                capacity: inputs.geometry.physical_leaf as usize,
                allocation: Some(&inputs.root_placement),
            },
        )?;
        for entry in entries {
            old_placements.insert(key(&entry["root"])?, entry);
        }
    }

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
    active["root_id"] = json!(active_id);
    let mut archive_records = BTreeMap::new();
    match &operation {
        Change::Graph(prepared) => {
            let mut receipts = proof.roles["active"].receipts.clone();
            receipts.push(prepared.receipt.clone());
            let entries=receipts.iter().map(|r|json!({"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"acceptance_ordinal":r["acceptance_ordinal"],"receipt":reference(r)})).collect::<Vec<_>>();
            let old_index = get(&b.pool, &oldactive["operation_index"])?;
            let (ir, _) = b.tree_seed(
                "photara.package.operation-id",
                entries.clone(),
                "operation_id",
                inputs.geometry.operation_leaf as usize,
                Some(&old_index["by_id"]),
            )?;
            let (or, _) = b.tree_seed(
                "photara.package.operation-ordinal",
                entries,
                "acceptance_ordinal",
                inputs.geometry.operation_leaf as usize,
                Some(&old_index["by_ordinal"]),
            )?;
            let prefix = hash(&encode(
                &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":oldactive["accepted"]["prefix_sha256"],"acceptance_ordinal":prepared.receipt["acceptance_ordinal"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]}),
            ));
            let accepted = json!({"through_ordinal":prepared.receipt["acceptance_ordinal"],"prefix_sha256":prefix});
            let mut index = b.object(
        "photara.package.operation-index",
        json!({"library_id":root["library_id"],"bootstrap_sha256":root["bootstrap_sha256"],"accepted":accepted,"by_id":ir,"by_ordinal":or}),
    );
            index["schema"]["version"] = json!(2);
            if oldactive["schema"]["version"] == 3 {
                ensure(
                    old_index["schema"]["version"] == 3,
                    "snapshot index version",
                )?;
                index["schema"]["version"] = json!(3);
                index["base"] = old_index["base"].clone();
            }

            let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":prepared.receipt["journal_id"],"sequence":prepared.receipt["journal_sequence"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]});
            let journal = json!({"journal_id":prepared.receipt["journal_id"],"through_sequence":prepared.receipt["journal_sequence"],"prefix_sha256":hash(&encode(&json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":oldactive["journal_inclusion"]["prefix_sha256"],"frame_sha256":hash(&encode(&frame))}))),"resulting_authored_revision":prepared.receipt["after"]["revision"],"resulting_authored_sha256":prepared.receipt["after"]["digest"]});
            let authored = prepared
                .authored
                .iter()
                .find(|(k, _)| k.0 == prepared.receipt["after"]["digest"])
                .ok_or("actual new authored body")?
                .1;
            active["authored_revision"] = prepared.receipt["after"]["revision"].clone();
            active["authored"] = reference(authored);
            active["operation_index"] = b.add(index)?;
            active["accepted"] = accepted;
            active["journal_inclusion"] = journal;
            active["predecessor"] = json!({"root_sha256":root["active"]["sha256"],"authored_revision":oldactive["authored_revision"]});
        }
        Change::Checkpoint(base) => {
            let old_index = get(&b.pool, &oldactive["operation_index"])?;
            ensure(
                oldactive["schema"]["version"] == 2
                    && base["schema"]
                        == json!({"id":"photara.package.acceptance-base","version":1})
                    && base["source_state"] == reference(&oldactive)
                    && base["source_commit"]
                        == json!({"commit_id":old.commit["commit_id"],"commit_sha256":hash(&encode(&old.commit))})
                    && base["accepted"] == oldactive["accepted"]
                    && base["journal_inclusion"] == oldactive["journal_inclusion"]
                    && base["receipts_by_id"] == old_index["by_id"]
                    && base["receipts_by_ordinal"] == old_index["by_ordinal"],
                "checkpoint exact old operation base",
            )?;
            archive_records = b.closure(&[oldactive["operation_index"].clone()])?;
            for bytes in old.loose.values() {
                let value = wire::parse(bytes)?;
                archive_records.insert(key(&reference(&value))?, value);
            }
            let base_ref = b.add((*base).clone())?;
            let (by_id, _) = b.tree(
                "photara.package.operation-id",
                Vec::new(),
                "operation_id",
                inputs.geometry.operation_leaf as usize,
            )?;
            let (by_ordinal, _) = b.tree(
                "photara.package.operation-ordinal",
                Vec::new(),
                "acceptance_ordinal",
                inputs.geometry.operation_leaf as usize,
            )?;
            let mut index = b.object("photara.package.operation-index",json!({"library_id":root["library_id"],"bootstrap_sha256":root["bootstrap_sha256"],"base":base_ref,"accepted":oldactive["accepted"],"by_id":by_id,"by_ordinal":by_ordinal}));
            index["schema"]["version"] = json!(3);
            active["schema"]["version"] = json!(3);
            active["operation_index"] = b.add(index)?;
            active["predecessor"] = json!({"root_sha256":root["active"]["sha256"],"authored_revision":oldactive["authored_revision"]});
        }
    }
    let semantic = b.closure(&[
        active["authored"].clone(),
        active["history"].clone(),
        active["resource_state"].clone(),
        active["operation_index"].clone(),
    ])?;
    active["inventory"] = b
        .inventory_seed(
            semantic.values().map(reference).collect(),
            &oldactive["inventory"],
            "state",
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
    let archive = if matches!(operation, Change::Checkpoint(_)) {
        Some(b.checkpoint_archive(old, inputs, archive_records)?)
    } else {
        None
    };
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

    if archive.is_some() {
        // Historical implementation/original controls remain archived physically,
        // but are no longer mandatory live global members after the audited base.
        globals.retain(|_, value| {
            !matches!(
                value["schema"]["id"].as_str(),
                Some(
                    "photara.storage.original-admission"
                        | "photara.storage.operation-phase"
                        | "photara.storage.finalization-control"
                        | "photara.storage.retained-control-inventory-leaf"
                        | "photara.storage.retained-control-inventory-branch"
                )
            )
        });
    }
    for r in retained_controls.keys() {
        let k = (r.sha256.as_str().to_owned(), r.byte_length.get());
        globals.insert(
            k.clone(),
            b.pool.get(&k).cloned().ok_or("retained terminal control")?,
        );
    }

    if let Some((_, nodes)) = &retained {
        for value in nodes {
            globals.insert(key(&reference(value))?, value.clone());
        }
    }
    let inventory_nodes = globals
        .iter()
        .filter(|(_, v)| {
            v["schema"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("photara.storage.retained-control-inventory-"))
        })
        .map(|(k, _)| k.clone())
        .collect();
    let mut union = globals.clone();
    if let Some((_, ownership)) = &archive {
        // Archive ownership is an independent physical partition, not a shared
        // dependency required in every current/recovery/pinned role locator.
        union.extend(ownership.clone());
    }
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
    let (base_inventory, globalnodes) = b.inventory_seed(
        union.values().map(reference).collect(),
        &oldoverlay["base"],
        "global",
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
        let snapshot_role = get(&b.pool, state)?["schema"]["version"] == 3;
        // A snapshot base does not make archived receipts live dependencies.
        // Claim shared sealed allocations only when this role actually locates
        // one of their records. Archive/recovery custody still retains them.
        let units = inputs
            .shared_sealed
            .iter()
            .filter(|id| {
                !snapshot_role || records.keys().any(|key| b.locations[*id].contains_key(key))
            })
            .cloned()
            .chain([
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
            let mut claim = b.object("photara.storage.allocation-claim",json!({"allocation_id":id,"arena":old.allocations[&id].arena,"layout":"framed-json","owned_extent":if sealed{raw.len()}else{ends[&id]}.to_string(),"authenticated_prefix":{"byte_length":raw.len().to_string(),"sha256":hash(raw)},"sealed":sealed,"sealed_charge":charge}));
            if get(&b.pool, state)?["schema"]["version"] == 3 {
                claim["schema"]["version"] = json!(2);
                claim
                    .as_object_mut()
                    .ok_or("claim")?
                    .remove("authenticated_prefix");
                claim["verification"] = json!("selected-records");
            }
            claims.push(claim);
        }
        let predecessor =
            old_placements.get(&key(&reference(&proof.roles[&allocation.role].state))?);
        let (owner, ownnodes) = b.tree_seed(
            "photara.storage.ownership",
            claims
                .iter()
                .map(|v| json!({"allocation_id":v["allocation_id"],"claim":reference(v)}))
                .collect(),
            "allocation_id",
            inputs.geometry.ownership_leaf as usize,
            predecessor.map(|p| &p["ownership"]),
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
        let locator = b.locator_seed(
            entries,
            &allocation.locator,
            predecessor.map(|p| &p["locator"]),
            &allocation.role,
        )?;
        placements.push(json!({"root":state,"root_id":get(&b.pool,state)?["root_id"],"locator":locator,"ownership":owner}));
    }
    placements.sort_by_key(|p| key(&p["root"]).unwrap());
    let root_placements = b.physical_seed(
        "root-placement",
        placements,
        "root",
        &inputs.root_placement,
        Some(&root["placement"]["root_placements"]),
    )?;
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
    let mut target = json!({"active":active_ref,"recovery":root["active"],"pinned_roots":root["pinned_roots"],"operation_index":active["operation_index"],"conversion_source":root["conversion_source"]});
    if let Some((placement, _)) = archive {
        target["archive_placement"] = placement;
    }
    let base_count = wire::number(&get(&b.pool, &base_inventory)?["count"])?;
    Ok(Layout {
        active_state: active,
        base_count,
        retained_inventory: retained.map(|v| v.0),
        inventory_nodes,
        files: b.files,
        target,
        base_inventory,
        root_placements,
    })
}

pub(super) fn original_objects(
    old: &OriginalPackage,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<super::object_pool::ObjectPool> {
    Ok(Builder::load(old, inputs, limits)?.pool)
}

pub(super) fn count_decode() {
    #[cfg(test)]
    LOAD_WORK.with(|work| {
        let (loads, parsed) = work.get();
        work.set((loads, parsed + 1));
    });
}

impl Builder {
    /// Initial checkpoint archives already-authenticated operation/control bytes
    /// in place. Only missing loose controls and new ownership/index pages append.
    fn checkpoint_archive(
        &mut self,
        old: &OriginalPackage,
        inputs: &PlannerInputs,
        mut records: BTreeMap<wire::Key, Value>,
    ) -> Result<(Value, BTreeMap<wire::Key, Value>)> {
        let data = &inputs
            .roles
            .iter()
            .find(|r| r.role == "active")
            .ok_or("active archive allocation")?
            .data;
        let existing = self
            .locations
            .values()
            .flat_map(|map| map.keys().cloned())
            .collect::<BTreeSet<_>>();
        self.pack(
            data,
            "data",
            records
                .iter()
                .filter(|(key, _)| !existing.contains(*key))
                .map(|(_, value)| (value.clone(), tag(value)))
                .collect(),
        )?;
        let mut ownership = BTreeMap::new();
        let mut claims = Vec::new();
        for (id, allocation) in &old.allocations {
            let sealed = inputs.shared_sealed.contains(id);
            let extent = if sealed {
                allocation.bytes.len() as u64
            } else {
                *self
                    .corridors
                    .get(id)
                    .ok_or("archive original tip corridor")?
            };
            let charge = if sealed {
                let value = self
                    .pool
                    .keys()
                    .filter_map(|key| self.pool.get(key))
                    .find(|v| {
                        v["schema"]["id"] == "photara.storage.sealed-charge"
                            && v["allocation_id"] == *id
                    })
                    .ok_or("archive sealed charge")?
                    .clone();
                let reference = reference(&value);
                ownership.insert(key(&reference)?, value);
                reference
            } else {
                Value::Null
            };
            let mut claim = self.object("photara.storage.allocation-claim", json!({"allocation_id":id,"arena":allocation.arena,"layout":"framed-json","owned_extent":extent.to_string(),"verification":"selected-records","sealed":sealed,"sealed_charge":charge}));
            claim["schema"]["version"] = json!(2);
            let reference = self.add(claim.clone())?;
            ownership.insert(key(&reference)?, claim);
            claims.push(json!({"allocation_id":id,"claim":reference}));
        }
        let (owner, nodes) = self.tree(
            "photara.storage.ownership",
            claims,
            "allocation_id",
            inputs.geometry.ownership_leaf as usize,
        )?;
        for node in nodes {
            ownership.insert(key(&reference(&node))?, node);
        }
        self.pack(
            data,
            "data",
            ownership
                .iter()
                .filter(|(key, _)| {
                    !inputs
                        .shared_sealed
                        .iter()
                        .any(|id| self.locations[id].contains_key(*key))
                })
                .map(|(_, value)| (value.clone(), 2))
                .collect(),
        )?;
        records.extend(ownership.clone());
        let locations = self
            .locations
            .values()
            .flat_map(|map| map.iter().map(|(key, value)| (key.clone(), value.clone())))
            .collect::<BTreeMap<_, _>>();
        let entries=records.iter().map(|(key,value)| Ok(json!({"object":reference(value),"membership":if tag(value)==2{"ownership"}else{"semantic"},"physical":locations.get(key).ok_or("archive object location")?}))).collect::<Result<Vec<_>>>()?;
        let locator = self.physical_tree("locator", entries, "object", &inputs.root_placement)?;
        Ok((json!({"locator":locator,"ownership":owner}), ownership))
    }
}
