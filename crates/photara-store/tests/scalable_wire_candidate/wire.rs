//! Private, disposable byte/closure candidate. No production reader calls this.
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) const FEATURE: &str = "example.ps2.scalable-storage.draft-v1";
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
const MAGIC: &[u8; 8] = b"PS2PKD01";
const MAX_BYTES: usize = 1024 * 1024;
pub(super) type Key = (String, u64);

pub(super) fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    ensure(
        s.len().is_multiple_of(2) && s.len() <= MAX_BYTES * 2,
        "hex bound",
    )?;
    s.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| "hex UTF8")?;
            u8::from_str_radix(text, 16).map_err(|_| "hex digits")
        })
        .collect()
}
fn decimal(s: &str) -> Result<u64> {
    let n = s.parse::<u64>().map_err(|_| "decimal range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn number(v: &Value) -> Result<u64> {
    decimal(v.as_str().ok_or("decimal string")?)
}
fn digest(s: &str) -> Result<()> {
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )
}
fn id(s: &str) -> Result<()> {
    let parsed = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(
        !parsed.is_nil() && parsed.to_string() == s,
        "canonical UUID",
    )
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("array")
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let map = v.as_object().ok_or("object")?;
    ensure(
        map.len() == names.len() && names.iter().all(|n| map.contains_key(*n)),
        "exact fields",
    )
}
fn schema(v: &Value, name: &str, names: &[&str]) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"] == format!("example.ps2.{name}")
            && v["schema"]["version"] == 1
            && v["project_id"] == PROJECT,
        "typed schema/project",
    )?;
    let extensions = v["extensions"].as_object().ok_or("extensions")?;
    ensure(
        extensions
            .keys()
            .all(|s| s.contains('.') && !s.starts_with('.')),
        "extension namespace",
    )
}
fn parse(bytes: &[u8]) -> Result<Value> {
    ensure(bytes.len() <= MAX_BYTES, "JSON bound")?;
    photara_store::package::parse_canonical_json(
        bytes,
        photara_store::package::JsonLimits::default(),
    )
    .map_err(|_| "canonical JSON")
}

pub(super) fn frame_ranges(bytes: &[u8], arena: &str) -> Result<Vec<(usize, usize, u8)>> {
    ensure(bytes.len() <= MAX_BYTES, "allocation bound")?;
    let mut ranges = vec![];
    let mut start = 0usize;
    while start < bytes.len() {
        let header = bytes
            .get(start..start.checked_add(16).ok_or("frame overflow")?)
            .ok_or("truncated header")?;
        ensure(
            &header[..8] == MAGIC && header[9..12] == [0, 0, 0] && header[8] <= 3,
            "frame header",
        )?;
        let body_len =
            u32::from_le_bytes(header[12..16].try_into().map_err(|_| "length header")?) as usize;
        let end = start
            .checked_add(16)
            .and_then(|n| n.checked_add(body_len))
            .ok_or("frame overflow")?;
        let body = bytes.get(start + 16..end).ok_or("truncated frame")?;
        if header[8] == 0 {
            ensure(
                !body.is_empty() && body.iter().all(|b| *b == 0),
                "exact bounded zero padding",
            )?;
        } else {
            ensure(
                (header[8] == 3) == (arena == "metadata"),
                "frame arena/type",
            )?;
            parse(body)?;
        }
        ranges.push((start, end, header[8]));
        start = end;
    }
    Ok(ranges)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ObjectRef {
    kind: String,
    sha256: String,
    byte_length: String,
}
impl ObjectRef {
    fn read(v: &Value) -> Result<Self> {
        let r: Self = serde_json::from_value(v.clone()).map_err(|_| "ObjectRef shape")?;
        ensure(r.kind == "json", "ObjectRef kind")?;
        digest(&r.sha256)?;
        decimal(&r.byte_length)?;
        Ok(r)
    }
    fn key(&self) -> Result<Key> {
        Ok((self.sha256.clone(), decimal(&self.byte_length)?))
    }
    fn verifies(&self, bytes: &[u8]) -> Result<()> {
        ensure(
            decimal(&self.byte_length)? == bytes.len() as u64 && hash(bytes) == self.sha256,
            "logical referenced bytes",
        )
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Physical {
    allocation_id: String,
    arena: String,
    offset: String,
    byte_length: String,
    record_sha256: String,
}
impl Physical {
    fn read(v: &Value) -> Result<Self> {
        let r: Self = serde_json::from_value(v.clone()).map_err(|_| "PhysicalRef shape")?;
        id(&r.allocation_id)?;
        ensure(r.arena == "data" || r.arena == "metadata", "arena")?;
        decimal(&r.offset)?;
        decimal(&r.byte_length)?;
        digest(&r.record_sha256)?;
        Ok(r)
    }
}
#[derive(Clone)]
pub(super) struct Store {
    pub(super) allocations: BTreeMap<String, (String, Vec<u8>)>,
    loose: BTreeMap<String, Vec<u8>>,
    pub(super) head: Vec<u8>,
    pub(super) commit: Vec<u8>,
    pub(super) manifest: Vec<u8>,
}
impl Store {
    pub(super) fn load(corpus: &Value, scenario: &str) -> Result<Self> {
        let s = &corpus["scenarios"][scenario];
        let record = |name: &str| -> Result<Vec<u8>> {
            let sha = s["records"][name].as_str().ok_or("record name")?;
            Ok(corpus["records"][sha]["canonical"]
                .as_str()
                .ok_or("record bytes")?
                .as_bytes()
                .to_vec())
        };
        let mut allocations = BTreeMap::new();
        for (allocation, v) in s["allocations"].as_object().ok_or("allocations")? {
            let sha = v["sha256"].as_str().ok_or("allocation digest")?;
            let bytes = unhex(
                corpus["allocation_bytes"][sha]
                    .as_str()
                    .ok_or("allocation bytes")?,
            )?;
            ensure(
                hash(&bytes) == sha
                    && bytes.len() as u64
                        == v["byte_length"].as_u64().ok_or("allocation length")?,
                "allocation vector",
            )?;
            allocations.insert(
                allocation.clone(),
                (
                    v["arena"].as_str().ok_or("allocation arena")?.to_owned(),
                    bytes,
                ),
            );
        }
        let loose = s["loose"]
            .as_object()
            .ok_or("loose")?
            .iter()
            .map(|(sha, name)| Ok((sha.clone(), record(name.as_str().ok_or("loose name")?)?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            allocations,
            loose,
            head: record("head")?,
            commit: record("commit")?,
            manifest: record("manifest")?,
        })
    }
    fn loose(&self, v: &Value) -> Result<Value> {
        let r = ObjectRef::read(v)?;
        let bytes = self
            .loose
            .get(&r.sha256)
            .ok_or("missing bootstrap object")?;
        r.verifies(bytes)?;
        parse(bytes)
    }
    fn physical(&self, r: &Physical, tag: u8) -> Result<(Value, Vec<u8>)> {
        let (arena, bytes) = self
            .allocations
            .get(&r.allocation_id)
            .ok_or("missing allocation")?;
        ensure(
            *arena == r.arena && ((tag == 3) == (arena == "metadata")),
            "frame arena/type",
        )?;
        let offset = usize::try_from(decimal(&r.offset)?).map_err(|_| "offset")?;
        let length = usize::try_from(decimal(&r.byte_length)?).map_err(|_| "length")?;
        let ranges = frame_ranges(bytes, arena)?;
        let (start, end, actual_tag) = ranges
            .into_iter()
            .find(|(start, _, _)| *start == offset)
            .ok_or("reference is not a frame boundary")?;
        let frame = &bytes[start..end];
        ensure(actual_tag != 0, "padding is not referenceable")?;
        ensure(
            frame.len() == length && hash(frame) == r.record_sha256 && actual_tag == tag,
            "exact framed reference",
        )?;
        let body = frame[16..].to_vec();
        Ok((parse(&body)?, body))
    }
}
#[derive(Clone)]
struct Entry {
    object: ObjectRef,
    membership: String,
    physical: Physical,
}
struct Resolver<'a> {
    store: &'a Store,
    entries: BTreeMap<Key, Entry>,
    physical: Vec<Physical>,
    seen_pages: BTreeSet<(String, String)>,
}
impl<'a> Resolver<'a> {
    fn new(store: &'a Store, root: &Value) -> Result<Self> {
        let mut r = Self {
            store,
            entries: BTreeMap::new(),
            physical: vec![],
            seen_pages: BTreeSet::new(),
        };
        r.locator(&Physical::read(root)?)?;
        Ok(r)
    }
    fn read(&mut self, p: &Physical, tag: u8) -> Result<(Value, Vec<u8>)> {
        ensure(self.physical.len() < 128, "bounded closure")?;
        let result = self.store.physical(p, tag)?;
        self.physical.push(p.clone());
        Ok(result)
    }
    fn locator(&mut self, p: &Physical) -> Result<Vec<Key>> {
        ensure(
            self.seen_pages
                .insert((p.allocation_id.clone(), p.offset.clone())),
            "locator cycle/duplicate page",
        )?;
        let (v, _) = self.read(p, 3)?;
        let mut keys = vec![];
        if v["schema"]["id"] == "example.ps2.locator-leaf" {
            schema(&v, "locator-leaf", &["count", "entries"])?;
            for entry in array(&v["entries"])? {
                fields(entry, &["object", "membership", "physical"])?;
                let object = ObjectRef::read(&entry["object"])?;
                let key = object.key()?;
                let membership = entry["membership"].as_str().ok_or("membership")?.to_owned();
                ensure(
                    membership == "semantic" || membership == "ownership",
                    "membership tag",
                )?;
                ensure(
                    self.entries
                        .insert(
                            key.clone(),
                            Entry {
                                object,
                                membership,
                                physical: Physical::read(&entry["physical"])?,
                            },
                        )
                        .is_none(),
                    "duplicate locator key",
                )?;
                keys.push(key);
            }
        } else {
            schema(&v, "locator-branch", &["count", "children"])?;
            ensure(array(&v["children"])?.len() >= 2, "branch fanout")?;
            for child in array(&v["children"])? {
                fields(child, &["first", "last", "count", "child"])?;
                let child_keys = self.locator(&Physical::read(&child["child"])?)?;
                ensure(
                    child_keys.first() == Some(&ObjectRef::read(&child["first"])?.key()?)
                        && child_keys.last() == Some(&ObjectRef::read(&child["last"])?.key()?)
                        && child_keys.len() as u64 == number(&child["count"])?,
                    "locator range summary",
                )?;
                keys.extend(child_keys);
            }
        }
        ensure(
            !keys.is_empty()
                && keys.windows(2).all(|k| k[0] < k[1])
                && keys.len() as u64 == number(&v["count"])?,
            "ordered locator/count",
        )?;
        Ok(keys)
    }
    fn object(&mut self, reference: &Value, membership: &str) -> Result<Value> {
        let r = ObjectRef::read(reference)?;
        let entry = self
            .entries
            .get(&r.key()?)
            .ok_or("dangling logical locator")?
            .clone();
        ensure(
            entry.membership == membership && entry.object == r,
            "wrong locator membership",
        )?;
        let (v, bytes) = self.read(
            &entry.physical,
            if membership == "semantic" { 1 } else { 2 },
        )?;
        r.verifies(&bytes)?;
        Ok(v)
    }
    fn semantic(
        &mut self,
        reference: &Value,
        seen: &mut BTreeSet<Key>,
        pending: &mut BTreeSet<Key>,
    ) -> Result<()> {
        let key = ObjectRef::read(reference)?.key()?;
        if seen.contains(&key) {
            return Ok(());
        }
        ensure(pending.insert(key.clone()), "semantic cycle")?;
        let v = self.object(reference, "semantic")?;
        match v["schema"]["id"].as_str().ok_or("semantic schema")? {
            "example.ps2.probe-leaf" => {
                schema(&v, "probe-leaf", &["label", "links"])?;
                ensure(v["label"].is_string(), "probe label")?;
                for edge in array(&v["links"])? {
                    self.semantic(edge, seen, pending)?;
                }
            }
            "example.ps2.history-probe" => {
                schema(&v, "history-probe", &["retained"])?;
                for edge in array(&v["retained"])? {
                    self.semantic(edge, seen, pending)?;
                }
            }
            "example.ps2.operation-index-root" => {
                schema(
                    &v,
                    "operation-index-root",
                    &["accepted", "operation_ids", "ordinals"],
                )?;
                fields(&v["accepted"], &["through_ordinal", "prefix_sha256"])?;
                ensure(
                    number(&v["accepted"]["through_ordinal"])? == 0
                        && array(&v["operation_ids"])?.is_empty()
                        && array(&v["ordinals"])?.is_empty(),
                    "empty operation probe only",
                )?;
            }
            _ => return Err("unsupported semantic schema"),
        }
        pending.remove(&key);
        seen.insert(key);
        Ok(())
    }
    fn owners(
        &mut self,
        p: &Physical,
        nodes: &mut BTreeSet<Key>,
        claims: &mut BTreeMap<String, Value>,
    ) -> Result<Vec<String>> {
        let (v, bytes) = self.read(p, 2)?;
        let key = (hash(&bytes), bytes.len() as u64);
        ensure(nodes.insert(key.clone()), "ownership cycle/duplicate")?;
        let entry = self.entries.get(&key).ok_or("missing ownership locator")?;
        ensure(
            entry.membership == "ownership" && entry.physical == *p,
            "ownership typed placement",
        )?;
        if v["schema"]["id"] == "example.ps2.ownership-claim" {
            schema(
                &v,
                "ownership-claim",
                &[
                    "allocation_id",
                    "arena",
                    "owned_extent",
                    "authenticated_prefix",
                    "sealed",
                    "sealed_charge",
                ],
            )?;
            let allocation = v["allocation_id"].as_str().ok_or("claim allocation")?;
            id(allocation)?;
            let key = allocation.to_owned();
            ensure(
                claims.insert(key.clone(), v).is_none(),
                "duplicate ownership claim",
            )?;
            Ok(vec![key])
        } else {
            schema(&v, "ownership-branch", &["count", "children"])?;
            let children = array(&v["children"])?;
            ensure(children.len() >= 2, "ownership branch fanout")?;
            let mut keys = vec![];
            let mut prior = None;
            for child in children {
                fields(child, &["allocation_id", "node"])?;
                let allocation = child["allocation_id"].as_str().ok_or("owner separator")?;
                ensure(prior.is_none_or(|p| p < allocation), "ownership key order")?;
                let child_keys = self.owners(&Physical::read(&child["node"])?, nodes, claims)?;
                ensure(
                    child_keys.first().map(String::as_str) == Some(allocation),
                    "ownership separator mismatch",
                )?;
                keys.extend(child_keys);
                prior = Some(allocation);
            }
            ensure(
                keys.len() as u64 == number(&v["count"])? && keys.windows(2).all(|k| k[0] < k[1]),
                "ownership branch range/count",
            )?;
            Ok(keys)
        }
    }
}
fn inventory(v: &Value) -> Result<BTreeSet<Key>> {
    schema(v, "inventory-root", &["count", "entries"])?;
    let keys = array(&v["entries"])?
        .iter()
        .map(|r| ObjectRef::read(r)?.key())
        .collect::<Result<Vec<_>>>()?;
    ensure(
        keys.len() as u64 == number(&v["count"])? && keys.windows(2).all(|k| k[0] < k[1]),
        "exact inventory order/count",
    )?;
    Ok(keys.into_iter().collect())
}

pub(super) struct RootProof {
    pub(super) logical: BTreeSet<Key>,
    pub(super) allocations: BTreeSet<String>,
    claims: BTreeMap<String, Value>,
    state: Value,
}
fn accounting(store: &Store, reference: &Value) -> Result<Value> {
    let v = store.loose(reference)?;
    schema(
        &v,
        "physical-accounting",
        &[
            "domain_incarnation",
            "standing_control",
            "tips",
            "sealed_charges",
            "unresolved",
            "total_charge",
        ],
    )?;
    id(v["domain_incarnation"].as_str().ok_or("domain")?)?;
    ensure(
        array(&v["unresolved"])?.is_empty(),
        "snapshot accounting only",
    )?;
    let mut total = number(&v["standing_control"])?;
    let mut allocations = BTreeSet::new();
    let mut previous = None;
    for tip in array(&v["tips"])? {
        fields(
            tip,
            &[
                "allocation_id",
                "arena",
                "owned_extent",
                "charged_high_water",
            ],
        )?;
        let allocation = tip["allocation_id"].as_str().ok_or("tip allocation")?;
        id(allocation)?;
        ensure(
            previous.is_none_or(|p| p < allocation) && allocations.insert(allocation.to_owned()),
            "tip order/uniqueness",
        )?;
        ensure(
            tip["arena"] == "data" || tip["arena"] == "metadata",
            "tip arena",
        )?;
        let charge = number(&tip["charged_high_water"])?;
        ensure(charge >= number(&tip["owned_extent"])?, "tip highwater")?;
        total = total.checked_add(charge).ok_or("charge overflow")?;
        previous = Some(allocation);
    }
    for reference in array(&v["sealed_charges"])? {
        let charge = store.loose(reference)?;
        schema(
            &charge,
            "sealed-charge",
            &[
                "allocation_id",
                "arena",
                "domain_incarnation",
                "measured_extent",
                "charged_high_water",
                "observation",
            ],
        )?;
        fields(&charge["observation"], &["kind", "observation_id"])?;
        ensure(
            charge["observation"]["kind"] == "synthetic-vector",
            "synthetic observation only",
        )?;
        id(charge["observation"]["observation_id"]
            .as_str()
            .ok_or("observation")?)?;
        ensure(
            charge["domain_incarnation"] == v["domain_incarnation"]
                && allocations.insert(
                    charge["allocation_id"]
                        .as_str()
                        .ok_or("charge allocation")?
                        .to_owned(),
                ),
            "unique domain charge",
        )?;
        let amount = number(&charge["charged_high_water"])?;
        ensure(
            amount >= number(&charge["measured_extent"])?,
            "sealed highwater",
        )?;
        total = total.checked_add(amount).ok_or("charge overflow")?;
    }
    ensure(total == number(&v["total_charge"])?, "accounted total")?;
    Ok(v)
}
#[allow(
    clippy::too_many_lines,
    reason = "Ordered exact bootstrap schema and capability checks"
)]
fn bootstrap(store: &Store, reader_minor: u64) -> Result<Value> {
    let manifest = parse(&store.manifest)?;
    fields(
        &manifest,
        &[
            "format",
            "format_version",
            "project_id",
            "created_at",
            "canonical_json",
            "required_features",
        ],
    )?;
    fields(&manifest["format_version"], &["major", "minor"])?;
    ensure(
        manifest["format"] == "photara.project-package"
            && manifest["project_id"] == PROJECT
            && manifest["canonical_json"] == "photara.canonical-json.v1"
            && manifest["format_version"]["major"] == 1
            && manifest["format_version"]["minor"] == 1,
        "bootstrap identity",
    )?;
    let head = parse(&store.head)?;
    fields(
        &head,
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    fields(&head["schema"], &["id", "version"])?;
    ensure(
        head["schema"]["id"] == "photara.package.head"
            && head["schema"]["version"] == 1
            && head["project_id"] == PROJECT
            && head["commit_sha256"] == hash(&store.commit),
        "HEAD selects exact commit",
    )?;
    id(head["commit_id"].as_str().ok_or("commit ID")?)?;
    let commit = parse(&store.commit)?;
    fields(
        &commit,
        &[
            "schema",
            "project_id",
            "commit_id",
            "package_revision",
            "parent",
            "write_id",
            "created_at",
            "minimum_reader",
            "required_features",
            "authored",
            "history",
            "inventory",
            "root_set",
            "extensions",
        ],
    )?;
    fields(&commit["schema"], &["id", "version"])?;
    ensure(
        commit["schema"]["id"] == "photara.package.commit"
            && commit["schema"]["version"] == 1
            && commit["project_id"] == PROJECT
            && commit["commit_id"] == head["commit_id"],
        "outer commit identity",
    )?;
    fields(&commit["minimum_reader"], &["major", "minor"])?;
    ensure(
        commit["minimum_reader"]["major"] == 1
            && commit["minimum_reader"]["minor"] == 3
            && reader_minor >= 3,
        "reader capability floor",
    )?;
    let features = array(&commit["required_features"])?;
    ensure(
        features
            == &[
                Value::String(FEATURE.into()),
                Value::String("photara.sealed-roots.v1".into()),
            ]
            && array(&manifest["required_features"])?
                .iter()
                .all(|v| features.contains(v)),
        "required scalable capability",
    )?;
    ensure(
        number(&commit["package_revision"])? > 0 && commit["parent"].is_null(),
        "candidate bootstrap provenance",
    )?;
    id(commit["write_id"].as_str().ok_or("write ID")?)?;
    ensure(
        commit["extensions"]
            .as_object()
            .is_some_and(serde_json::Map::is_empty),
        "outer probe extensions",
    )?;
    let root = &commit["root_set"];
    schema(
        root,
        "root-set",
        &[
            "library_id",
            "bootstrap_sha256",
            "kind",
            "active",
            "recovery",
            "pinned_roots",
            "operation_index",
            "conversion_source",
            "inventory",
            "placement",
        ],
    )?;
    ensure(
        root["library_id"] == LIBRARY
            && root["bootstrap_sha256"] == hash(&store.manifest)
            && root["kind"] == "scalable-draft",
        "RootSet bootstrap/discriminator",
    )?;
    ensure(
        array(&root["pinned_roots"])?.is_empty() && root["conversion_source"].is_null(),
        "initial vector pin/conversion scope",
    )?;
    fields(
        &root["placement"],
        &[
            "generation",
            "active_locator",
            "recovery_locator",
            "active_ownership",
            "recovery_ownership",
            "accounting",
        ],
    )?;
    ensure(
        number(&root["placement"]["generation"])? > 0,
        "placement generation",
    )?;
    Ok(commit)
}

#[allow(
    clippy::too_many_lines,
    reason = "Keep the three distinct closure equalities and ownership proof together"
)]
pub(super) fn verify_role(store: &Store, role: &str, reader_minor: u64) -> Result<RootProof> {
    ensure(role == "active" || role == "recovery", "root role")?;
    let commit = bootstrap(store, reader_minor)?;
    let root = &commit["root_set"];
    let placement = &root["placement"];
    let accounts = accounting(store, &placement["accounting"])?;
    let mut resolver = Resolver::new(store, &placement[format!("{role}_locator")])?;
    let state = resolver.object(&root[role], "semantic")?;
    schema(
        &state,
        "state-root",
        &[
            "library_id",
            "bootstrap_sha256",
            "root_id",
            "authored_revision",
            "authored",
            "history",
            "resource_state",
            "operation_index",
            "accepted",
            "journal_inclusion",
            "predecessor",
            "inventory",
        ],
    )?;
    id(state["root_id"].as_str().ok_or("root ID")?)?;
    ensure(
        state["library_id"] == LIBRARY
            && state["bootstrap_sha256"] == hash(&store.manifest)
            && number(&state["authored_revision"])? > 0
            && state["resource_state"].is_null()
            && state["journal_inclusion"].is_null(),
        "independent StateRoot identity/scope",
    )?;
    fields(&state["predecessor"], &["root_sha256", "authored_revision"])?;
    digest(
        state["predecessor"]["root_sha256"]
            .as_str()
            .ok_or("predecessor digest")?,
    )?;
    number(&state["predecessor"]["authored_revision"])?;
    let mut logical = BTreeSet::new();
    for field in ["authored", "history", "operation_index"] {
        resolver.semantic(&state[field], &mut logical, &mut BTreeSet::new())?;
    }
    let operations = resolver.object(&state["operation_index"], "semantic")?;
    let prefix = serde_json::json!({"domain":"photara.package.accepted-prefix.v1", "project_id":PROJECT,"library_id":LIBRARY,"bootstrap_sha256":hash(&store.manifest),"through_ordinal":"0"});
    let prefix = hash(&photara_core::canonical_json(&prefix).map_err(|_| "prefix encoding")?);
    ensure(
        state["accepted"] == operations["accepted"] && state["accepted"]["prefix_sha256"] == prefix,
        "accepted prefix identity",
    )?;
    let inv = resolver.object(&state["inventory"], "semantic")?;
    ensure(
        inventory(&inv)? == logical,
        "StateRoot exact semantic inventory",
    )?;
    logical.insert(ObjectRef::read(&root[role])?.key()?);
    logical.insert(ObjectRef::read(&state["inventory"])?.key()?);
    if role == "active" {
        ensure(
            commit["authored"] == state["authored"]
                && commit["history"] == state["history"]
                && commit["inventory"] == state["inventory"]
                && root["operation_index"] == state["operation_index"],
            "outer active equalities",
        )?;
    }
    let mut owner_nodes = BTreeSet::new();
    let mut claims = BTreeMap::new();
    resolver.owners(
        &Physical::read(&placement[format!("{role}_ownership")])?,
        &mut owner_nodes,
        &mut claims,
    )?;
    let semantic_entries = resolver
        .entries
        .iter()
        .filter(|(_, e)| e.membership == "semantic")
        .map(|(k, _)| k.clone())
        .collect::<BTreeSet<_>>();
    let owner_entries = resolver
        .entries
        .iter()
        .filter(|(_, e)| e.membership == "ownership")
        .map(|(k, _)| k.clone())
        .collect::<BTreeSet<_>>();
    ensure(
        semantic_entries == logical && owner_entries == owner_nodes,
        "exact typed locator membership closure",
    )?;
    let used = resolver
        .physical
        .iter()
        .map(|r| r.allocation_id.clone())
        .collect::<BTreeSet<_>>();
    ensure(
        used == claims.keys().cloned().collect(),
        "exact physical ownership coverage",
    )?;
    for (allocation, claim) in &claims {
        let (arena, bytes) = store
            .allocations
            .get(allocation)
            .ok_or("claimed allocation absent")?;
        let extent = number(&claim["owned_extent"])?;
        fields(&claim["authenticated_prefix"], &["byte_length", "sha256"])?;
        let prefix_len = number(&claim["authenticated_prefix"]["byte_length"])?;
        let prefix = bytes
            .get(..usize::try_from(prefix_len).map_err(|_| "prefix offset")?)
            .ok_or("prefix extent")?;
        ensure(
            claim["arena"] == *arena
                && extent == bytes.len() as u64
                && prefix_len <= extent
                && claim["authenticated_prefix"]["sha256"] == hash(prefix),
            "owned extent/prefix",
        )?;
        for reference in resolver
            .physical
            .iter()
            .filter(|r| r.allocation_id == *allocation)
        {
            ensure(
                decimal(&reference.offset)?
                    .checked_add(decimal(&reference.byte_length)?)
                    .is_some_and(|end| end <= extent),
                "physical edge outside ownership",
            )?;
        }
        if claim["sealed"].as_bool().ok_or("sealed boolean")? {
            ensure(
                prefix_len == extent
                    && array(&accounts["sealed_charges"])?.contains(&claim["sealed_charge"]),
                "selected canonical sealed charge",
            )?;
            let charge = store.loose(&claim["sealed_charge"])?;
            ensure(
                charge["allocation_id"] == *allocation
                    && charge["arena"] == *arena
                    && number(&charge["measured_extent"])? == extent,
                "sealed claim charge identity",
            )?;
        } else {
            ensure(
                claim["sealed_charge"].is_null(),
                "growable has no sealed charge",
            )?;
            let tip = array(&accounts["tips"])?
                .iter()
                .find(|t| t["allocation_id"] == *allocation)
                .ok_or("growable registration absent")?;
            ensure(
                tip["arena"] == *arena && number(&tip["owned_extent"])? == extent,
                "growable current-tip registration",
            )?;
        }
    }
    Ok(RootProof {
        logical,
        allocations: used,
        claims,
        state,
    })
}

pub(super) fn verify_package(store: &Store, reader_minor: u64) -> Result<(RootProof, RootProof)> {
    let commit = bootstrap(store, reader_minor)?;
    let root = &commit["root_set"];
    let active = verify_role(store, "active", reader_minor)?;
    let recovery = verify_role(store, "recovery", reader_minor)?;
    for (allocation, claim) in &active.claims {
        if let Some(other) = recovery.claims.get(allocation) {
            ensure(claim == other, "inconsistent shared allocation claim")?;
        }
    }
    ensure(
        number(&recovery.state["accepted"]["through_ordinal"])?
            <= number(&active.state["accepted"]["through_ordinal"])?,
        "recovery acceptance bound",
    )?;
    let accounts = accounting(store, &root["placement"]["accounting"])?;
    let mut union = active
        .logical
        .union(&recovery.logical)
        .cloned()
        .collect::<BTreeSet<_>>();
    union.insert(ObjectRef::read(&root["placement"]["accounting"])?.key()?);
    for charge in array(&accounts["sealed_charges"])? {
        union.insert(ObjectRef::read(charge)?.key()?);
    }
    ensure(
        inventory(&store.loose(&root["inventory"])?)? == union,
        "RootSet exact logical union",
    )?;
    let used = active
        .allocations
        .union(&recovery.allocations)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut registered = array(&accounts["tips"])?
        .iter()
        .map(|t| {
            t["allocation_id"]
                .as_str()
                .ok_or("registered allocation")
                .map(str::to_owned)
        })
        .collect::<Result<BTreeSet<_>>>()?;
    for charge in array(&accounts["sealed_charges"])? {
        registered.insert(
            store.loose(charge)?["allocation_id"]
                .as_str()
                .ok_or("sealed allocation")?
                .to_owned(),
        );
    }
    ensure(
        used == registered,
        "exact selected allocation accounting union",
    )?;
    let mut loose_refs = array(&accounts["sealed_charges"])?.clone();
    loose_refs.push(root["placement"]["accounting"].clone());
    loose_refs.push(root["inventory"].clone());
    let mut loose_keys = BTreeSet::new();
    let mut control_bytes = (store.head.len() + store.commit.len() + store.manifest.len()) as u64;
    for reference in loose_refs {
        let reference = ObjectRef::read(&reference)?;
        if loose_keys.insert(reference.key()?) {
            control_bytes = control_bytes
                .checked_add(decimal(&reference.byte_length)?)
                .ok_or("bootstrap bound overflow")?;
        }
    }
    ensure(
        control_bytes <= number(&accounts["standing_control"])?,
        "selected bootstrap bytes exceed allowance",
    )?;
    Ok((active, recovery))
}
