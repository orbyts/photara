//! Pure operation-index candidate; reads exact framed records, never files.
use super::packed::{Result, ensure, frame_ranges, hash, unhex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
type Key = (String, u64);
fn encode(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn decimal(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("decimal string")?;
    let n = s.parse::<u64>().map_err(|_| "decimal range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let o = v.as_object().ok_or("object")?;
    ensure(
        o.len() == names.len() && names.iter().all(|k| o.contains_key(*k)),
        "exact fields",
    )
}
fn schema(v: &Value, id: &str, names: &[&str]) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"] == id
            && v["schema"]["version"] == 1
            && v["project_id"] == PROJECT
            && v["extensions"].is_object(),
        "typed operation schema",
    )
}
fn digest(v: &Value) -> Result<&str> {
    let s = v.as_str().ok_or("digest string")?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest shape",
    )?;
    Ok(s)
}
fn uuid(v: &Value) -> Result<&str> {
    let s = v.as_str().ok_or("UUID string")?;
    let id = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!id.is_nil() && id.to_string() == s, "canonical UUID")?;
    Ok(s)
}
fn key(v: &Value) -> Result<Key> {
    fields(v, &["kind", "sha256", "byte_length"])?;
    ensure(v["kind"] == "json", "JSON ObjectRef")?;
    Ok((digest(&v["sha256"])?.into(), decimal(&v["byte_length"])?))
}
fn reference(v: &Value) -> Value {
    let b = encode(v);
    json!({"kind":"json","sha256":hash(&b),"byte_length":b.len().to_string()})
}

#[derive(Clone)]
pub(super) struct Store {
    pub(super) roots: Value,
    pub(super) journal: Vec<Value>,
    pub(super) bootstrap: String,
    pack: Vec<u8>,
    locations: BTreeMap<Key, (usize, usize, String)>,
}
impl Store {
    pub(super) fn load(c: &Value) -> Result<Self> {
        let pack = unhex(c["pack_hex"].as_str().ok_or("pack")?)?;
        ensure(hash(&pack) == c["pack_sha256"], "fixed operation pack")?;
        let locator = &c["records"]["locator"]["input"];
        schema(locator, "example.ps2.locator-leaf", &["count", "entries"])?;
        let entries = locator["entries"].as_array().ok_or("locator entries")?;
        ensure(
            entries.len() as u64 == decimal(&locator["count"])?,
            "locator count",
        )?;
        let mut locations = BTreeMap::new();
        let mut prior = None;
        for e in entries {
            fields(e, &["object", "membership", "physical"])?;
            let k = key(&e["object"])?;
            ensure(
                e["membership"] == "semantic" && prior.as_ref().is_none_or(|p| p < &k),
                "ordered operation locator",
            )?;
            let p = &e["physical"];
            fields(
                p,
                &[
                    "allocation_id",
                    "arena",
                    "offset",
                    "byte_length",
                    "record_sha256",
                ],
            )?;
            ensure(
                p["allocation_id"] == c["allocation_id"] && p["arena"] == "data",
                "operation allocation",
            )?;
            locations.insert(
                k.clone(),
                (
                    usize::try_from(decimal(&p["offset"])?).map_err(|_| "offset")?,
                    usize::try_from(decimal(&p["byte_length"])?).map_err(|_| "length")?,
                    digest(&p["record_sha256"])?.into(),
                ),
            );
            prior = Some(k);
        }
        Ok(Self {
            roots: c["selected_roots"].clone(),
            journal: (1..=3)
                .map(|n| c["records"][format!("journal-frame-{n}")]["input"].clone())
                .collect(),
            bootstrap: c["records"]["manifest"]["sha256"]
                .as_str()
                .ok_or("bootstrap")?
                .into(),
            pack,
            locations,
        })
    }
    pub(super) fn get(&self, r: &Value) -> Result<Value> {
        let k = key(r)?;
        let (start, len, digest) = self.locations.get(&k).ok_or("missing operation edge")?;
        let ranges = frame_ranges(&self.pack, "data")?;
        ensure(
            ranges.contains(&(*start, start.checked_add(*len).ok_or("range")?, 1)),
            "operation frame boundary",
        )?;
        let frame = self.pack.get(*start..start + len).ok_or("frame extent")?;
        ensure(
            hash(frame) == *digest
                && frame.len() == *len
                && hash(&frame[16..]) == k.0
                && (frame.len() - 16) as u64 == k.1,
            "exact operation bytes",
        )?;
        photara_store::package::parse_canonical_json(
            &frame[16..],
            photara_store::package::JsonLimits::default(),
        )
        .map_err(|_| "operation canonical JSON")
    }
    pub(super) fn append(&mut self, v: &Value) -> Value {
        let body = encode(v);
        let r = reference(v);
        let mut frame = b"PS2PKD01".to_vec();
        frame.extend([1, 0, 0, 0]);
        frame.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
        frame.extend(body);
        self.locations.insert(
            key(&r).unwrap(),
            (self.pack.len(), frame.len(), hash(&frame)),
        );
        self.pack.extend(frame);
        r
    }
    pub(super) fn retain(&mut self, keys: &BTreeSet<Key>) -> Result<()> {
        let values = keys
            .iter()
            .map(|(sha, len)| {
                self.get(&json!({"kind":"json","sha256":sha,"byte_length":len.to_string()}))
            })
            .collect::<Result<Vec<_>>>()?;
        self.locations.clear();
        self.pack.clear();
        for v in values {
            self.append(&v);
        }
        Ok(())
    }
    pub(super) fn fingerprint(&self) -> String {
        hash(&encode(
            &json!({"roots":self.roots,"journal":self.journal,"pack":hash(&self.pack),"locations":self.locations.iter().map(|(k,v)|json!([k.0,k.1,v.0,v.1,v.2])).collect::<Vec<_>>()}),
        ))
    }
}
fn read(store: &Store, r: &Value, seen: &mut BTreeSet<Key>) -> Result<Value> {
    seen.insert(key(r)?);
    store.get(r)
}
fn provenance(v: &Value) -> Result<()> {
    fields(
        v,
        &[
            "principal",
            "actor",
            "grantor",
            "grant_ref",
            "effective_scope",
            "policy_decision_sha256",
        ],
    )?;
    for principal in [&v["principal"], &v["grantor"]] {
        fields(principal, &["kind", "account_id"])?;
        ensure(principal["kind"] == "account", "account specimen")?;
        uuid(&principal["account_id"])?;
    }
    fields(&v["actor"], &["kind", "actor_id"])?;
    ensure(v["actor"]["kind"] == "photara.gui", "actor kind")?;
    uuid(&v["actor"]["actor_id"])?;
    fields(&v["grant_ref"], &["kind", "reference_id"])?;
    ensure(
        v["grant_ref"]["kind"] == "photara.project-grant",
        "grant kind",
    )?;
    uuid(&v["grant_ref"]["reference_id"])?;
    fields(&v["effective_scope"], &["project_id", "actions"])?;
    ensure(
        v["effective_scope"]["project_id"] == PROJECT && v["effective_scope"]["actions"] == 71,
        "historical scope",
    )?;
    digest(&v["policy_decision_sha256"])?;
    Ok(())
}
fn receipt(v: &Value, bootstrap: &str) -> Result<()> {
    schema(
        v,
        "photara.package.operation-receipt",
        &[
            "library_id",
            "bootstrap_sha256",
            "operation_id",
            "request_sha256",
            "acceptance_ordinal",
            "journal_id",
            "journal_sequence",
            "outcome",
            "before",
            "after",
            "undo_group_id",
            "provenance",
        ],
    )?;
    ensure(
        v["library_id"] == LIBRARY
            && v["bootstrap_sha256"] == bootstrap
            && v["outcome"] == "accepted"
            && v["undo_group_id"].is_null(),
        "original receipt identity",
    )?;
    uuid(&v["operation_id"])?;
    uuid(&v["journal_id"])?;
    digest(&v["request_sha256"])?;
    decimal(&v["acceptance_ordinal"])?;
    decimal(&v["journal_sequence"])?;
    for c in [&v["before"], &v["after"]] {
        fields(c, &["revision", "digest"])?;
        decimal(&c["revision"])?;
        digest(&c["digest"])?;
    }
    provenance(&v["provenance"])
}
fn initial_prefix(bootstrap: &str) -> String {
    hash(&encode(
        &json!({"domain":"photara.package.accepted-prefix.v1","project_id":PROJECT,"library_id":LIBRARY,"bootstrap_sha256":bootstrap,"through_ordinal":"0"}),
    ))
}
fn accepted_link(previous: &str, r: &Value) -> String {
    hash(&encode(
        &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":previous,"acceptance_ordinal":r["acceptance_ordinal"],"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"receipt_sha256":reference(r)["sha256"]}),
    ))
}
#[derive(Clone)]
pub(super) struct Proof {
    pub(super) records: Vec<Value>,
    pub(super) closure: BTreeSet<Key>,
    pub(super) accepted_prefix: String,
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep reciprocal index validation and original receipt prefix in one pass"
)]
fn indexes(store: &Store, root: &Value, seen: &mut BTreeSet<Key>) -> Result<Proof> {
    let index = read(store, root, seen)?;
    schema(
        &index,
        "example.ps2.operation-index-tree",
        &[
            "library_id",
            "bootstrap_sha256",
            "accepted",
            "by_id",
            "by_ordinal",
        ],
    )?;
    ensure(
        index["library_id"] == LIBRARY && index["bootstrap_sha256"] == store.bootstrap,
        "index identity",
    )?;
    fields(&index["accepted"], &["through_ordinal", "prefix_sha256"])?;
    let count = decimal(&index["accepted"]["through_ordinal"])?;
    ensure((1..=32).contains(&count), "bounded linked operations")?;
    let ids = read(store, &index["by_id"], seen)?;
    let ordinals = read(store, &index["by_ordinal"], seen)?;
    schema(&ids, "example.ps2.operation-id-leaf", &["count", "entries"])?;
    schema(
        &ordinals,
        "example.ps2.operation-ordinal-leaf",
        &["count", "entries"],
    )?;
    let by_id = ids["entries"].as_array().ok_or("ID entries")?;
    let by_ordinal = ordinals["entries"].as_array().ok_or("ordinal entries")?;
    ensure(
        by_id.len() as u64 == count
            && by_ordinal.len() as u64 == count
            && decimal(&ids["count"])? == count
            && decimal(&ordinals["count"])? == count,
        "two index counts",
    )?;
    let mut mapped = BTreeMap::new();
    let mut prior = None;
    for entry in by_id {
        fields(
            entry,
            &[
                "operation_id",
                "request_sha256",
                "acceptance_ordinal",
                "receipt",
            ],
        )?;
        let id = uuid(&entry["operation_id"])?;
        ensure(
            prior.is_none_or(|p| p < id) && mapped.insert(id, entry).is_none(),
            "unique sorted operation IDs",
        )?;
        prior = Some(id);
    }
    let mut prefix = initial_prefix(&store.bootstrap);
    let mut records = vec![];
    let mut operations = BTreeSet::new();
    let mut sequence = 0;
    for (slot, entry) in by_ordinal.iter().enumerate() {
        fields(
            entry,
            &[
                "operation_id",
                "request_sha256",
                "acceptance_ordinal",
                "receipt",
            ],
        )?;
        let id = uuid(&entry["operation_id"])?;
        ensure(
            decimal(&entry["acceptance_ordinal"])? == slot as u64 + 1 && operations.insert(id),
            "contiguous unique ordinals",
        )?;
        ensure(mapped.get(id) == Some(&entry), "two indexes disagree")?;
        let r = read(store, &entry["receipt"], seen)?;
        receipt(&r, &store.bootstrap)?;
        ensure(
            r["operation_id"] == entry["operation_id"]
                && r["request_sha256"] == entry["request_sha256"]
                && r["acceptance_ordinal"] == entry["acceptance_ordinal"],
            "index original receipt mismatch",
        )?;
        let next_sequence = decimal(&r["journal_sequence"])?;
        ensure(next_sequence > sequence, "journal sequence order")?;
        sequence = next_sequence;
        if let Some(previous) = records.last() {
            let previous: &Value = previous;
            ensure(
                previous["after"] == r["before"],
                "authored receipt continuity",
            )?;
        }
        prefix = accepted_link(&prefix, &r);
        records.push(r);
    }
    ensure(
        index["accepted"]["prefix_sha256"] == prefix,
        "recomputed accepted prefix",
    )?;
    Ok(Proof {
        records,
        closure: seen.clone(),
        accepted_prefix: prefix,
    })
}
fn authored_closure(store: &Store, r: &Value, seen: &mut BTreeSet<Key>) -> Result<()> {
    let k = key(r)?;
    if !seen.insert(k) {
        return Ok(());
    }
    let v = store.get(r)?;
    ensure(v["project_id"] == PROJECT, "authored project")?;
    let mut edges = vec![];
    match v["schema"]["id"].as_str().ok_or("authored schema")? {
        "photara.project.authored" => {
            for field in [
                "party_assignments",
                "location_assignments",
                "asset_ledger",
                "resource_ledger",
                "context",
            ] {
                edges.push(v[field].clone());
            }
            for g in v["graphs"].as_array().ok_or("graphs")? {
                edges.push(g["document"].clone());
            }
        }
        "photara.project.saved-graph" => {
            let _: photara_core::GraphDocument =
                serde_json::from_value(v["graph"].clone()).map_err(|_| "actual Core Graph")?;
            edges.push(v["context"].clone());
            for p in v["required_packages"].as_array().ok_or("packages")? {
                edges.push(p["manifest"].clone());
            }
            for n in v["node_contracts"].as_array().ok_or("node contracts")? {
                edges.push(n["manifest"].clone());
            }
        }
        "photara.context.authored" => {
            for field in [
                "variables",
                "expressions",
                "captures",
                "metadata_selections",
            ] {
                ensure(
                    v[field].as_array().is_some_and(Vec::is_empty),
                    "empty context specimen",
                )?;
            }
            for n in v["node_contexts"].as_array().ok_or("node contexts")? {
                edges.push(n["context"].clone());
            }
        }
        "photara.project.party-assignments" | "photara.project.location-assignments" => ensure(
            v["assignments"].as_array().is_some_and(Vec::is_empty),
            "empty assignments",
        )?,
        "photara.project.asset-ledger" => ensure(
            v["assets"].as_array().is_some_and(Vec::is_empty),
            "empty assets",
        )?,
        "photara.project.resource-ledger" => {
            for field in ["managed_resources", "external_resources", "artifacts"] {
                ensure(
                    v[field].as_array().is_some_and(Vec::is_empty),
                    "empty resources",
                )?;
            }
        }
        "photara.project.history" => {
            for field in [
                "runs",
                "operations",
                "evidence",
                "snapshots",
                "metadata_observations",
                "proposals",
                "apply_receipts",
                "artifacts",
            ] {
                ensure(
                    v[field].as_array().is_some_and(Vec::is_empty),
                    "empty history",
                )?;
            }
        }
        "photara.node.manifest" => {}
        _ => return Err("unsupported authored specimen schema"),
    }
    for edge in edges {
        authored_closure(store, &edge, seen)?;
    }
    Ok(())
}
pub(super) fn verify_role(store: &Store, role: &str) -> Result<Proof> {
    let selected = &store.roots[role];
    fields(
        selected,
        &[
            "authored",
            "history",
            "operation_index",
            "accepted",
            "journal_inclusion",
        ],
    )?;
    let mut seen = BTreeSet::new();
    let mut proof = indexes(store, &selected["operation_index"], &mut seen)?;
    fields(&selected["accepted"], &["through_ordinal", "prefix_sha256"])?;
    ensure(
        decimal(&selected["accepted"]["through_ordinal"])? == proof.records.len() as u64
            && selected["accepted"]["prefix_sha256"] == proof.accepted_prefix,
        "selected accepted coordinate",
    )?;
    authored_closure(store, &selected["authored"], &mut seen)?;
    authored_closure(store, &selected["history"], &mut seen)?;
    let authored = store.get(&selected["authored"])?;
    let last = proof.records.last().ok_or("nonempty specimen")?;
    ensure(
        last["after"]["digest"] == selected["authored"]["sha256"]
            && last["after"]["revision"] == authored["authored_revision"],
        "last receipt authored result",
    )?;
    let j = &selected["journal_inclusion"];
    fields(
        j,
        &[
            "journal_id",
            "through_sequence",
            "prefix_sha256",
            "resulting_authored_revision",
            "resulting_authored_sha256",
        ],
    )?;
    uuid(&j["journal_id"])?;
    let mut prefix = hash(&encode(
        &json!({"domain":"photara.package.journal-prefix.v1","journal_id":j["journal_id"],"through_sequence":"0"}),
    ));
    let mut included = 0;
    for frame in &store.journal {
        fields(
            frame,
            &[
                "schema",
                "journal_id",
                "sequence",
                "operation_id",
                "request_sha256",
                "receipt_sha256",
            ],
        )?;
        fields(&frame["schema"], &["id", "version"])?;
        ensure(
            frame["schema"]["id"] == "photara.package.accepted-journal-frame"
                && frame["schema"]["version"] == 1
                && frame["journal_id"] == j["journal_id"],
            "journal typed identity",
        )?;
        if decimal(&frame["sequence"])? > decimal(&j["through_sequence"])? {
            continue;
        }
        let r = proof
            .records
            .get(included)
            .ok_or("extra included journal frame")?;
        ensure(
            frame["sequence"] == r["journal_sequence"]
                && frame["operation_id"] == r["operation_id"]
                && frame["request_sha256"] == r["request_sha256"]
                && frame["receipt_sha256"] == reference(r)["sha256"]
                && r["journal_id"] == j["journal_id"],
            "journal original receipt binding",
        )?;
        prefix = hash(&encode(
            &json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":prefix,"frame_sha256":hash(&encode(frame))}),
        ));
        included += 1;
    }
    ensure(
        included == proof.records.len()
            && j["through_sequence"] == last["journal_sequence"]
            && j["prefix_sha256"] == prefix
            && j["resulting_authored_revision"] == last["after"]["revision"]
            && j["resulting_authored_sha256"] == last["after"]["digest"],
        "journal inclusion prefix/result",
    )?;
    proof.closure = seen;
    Ok(proof)
}
pub(super) fn verify(store: &Store) -> Result<(Proof, Proof)> {
    let a = verify_role(store, "active")?;
    let r = verify_role(store, "recovery")?;
    ensure(
        r.records.len() <= a.records.len() && a.records[..r.records.len()] == r.records,
        "independent shared receipt prefix",
    )?;
    let union = a
        .closure
        .union(&r.closure)
        .cloned()
        .collect::<BTreeSet<_>>();
    ensure(
        union == store.locations.keys().cloned().collect(),
        "exact retained operation closure",
    )?;
    Ok((a, r))
}
pub(super) fn retry(store: &Store, intent: &Value) -> Result<Value> {
    let proof = verify_role(store, "active")?;
    let id = uuid(&intent["operation_id"])?;
    let r = proof
        .records
        .iter()
        .find(|r| r["operation_id"] == id)
        .ok_or("operation unknown")?;
    ensure(
        r["request_sha256"] == hash(&encode(intent)),
        "same ID different semantic intent",
    )?;
    Ok(r.clone())
}
pub(super) fn rewrite_leaf(
    store: &mut Store,
    field: &str,
    edit: impl FnOnce(&mut Value),
) -> Result<()> {
    let mut index = store.get(&store.roots["active"]["operation_index"])?;
    let mut leaf = store.get(&index[field])?;
    edit(&mut leaf);
    index[field] = store.append(&leaf);
    let reference = store.append(&index);
    store.roots["active"]["operation_index"] = reference;
    Ok(())
}
pub(super) fn replace_original_provenance(store: &mut Store) -> Result<()> {
    let mut proof = verify_role(store, "active")?;
    proof.records[0]["provenance"]["actor"]["actor_id"] =
        json!("50000000-0000-4000-8000-000000000099");
    let replacement = store.append(&proof.records[0]);
    let mut index = store.get(&store.roots["active"]["operation_index"])?;
    for field in ["by_id", "by_ordinal"] {
        let mut leaf = store.get(&index[field])?;
        for entry in leaf["entries"].as_array_mut().ok_or("entries")? {
            if entry["operation_id"] == proof.records[0]["operation_id"] {
                entry["receipt"] = replacement.clone();
            }
        }
        index[field] = store.append(&leaf);
    }
    let mut prefix = initial_prefix(&store.bootstrap);
    for receipt in &proof.records {
        prefix = accepted_link(&prefix, receipt);
    }
    index["accepted"]["prefix_sha256"] = json!(prefix);
    store.roots["active"]["accepted"] = index["accepted"].clone();
    let root = store.append(&index);
    store.roots["active"]["operation_index"] = root;
    Ok(())
}
