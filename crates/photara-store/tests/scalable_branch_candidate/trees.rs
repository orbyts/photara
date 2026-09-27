//! Disposable bounded typed traversal; this is not a production package reader.
use super::packed::{Result, ensure, frame_ranges, hash, unhex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Key = (String, u64);
pub(super) const FEATURE: &str = "example.ps2.typed-branches.draft-v1";
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
pub(super) fn encode(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
pub(super) fn number(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("decimal")?;
    let n = s.parse::<u64>().map_err(|_| "decimal overflow")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let o = v.as_object().ok_or("object")?;
    ensure(
        o.len() == names.len() && names.iter().all(|n| o.contains_key(*n)),
        "exact typed fields",
    )
}
fn digest(v: &Value) -> Result<String> {
    let s = v.as_str().ok_or("digest")?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest shape",
    )?;
    Ok(s.into())
}
fn id(v: &Value) -> Result<String> {
    let s = v.as_str().ok_or("UUID")?;
    let parsed = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(
        !parsed.is_nil() && parsed.to_string() == s,
        "canonical UUID",
    )?;
    Ok(s.into())
}
pub(super) fn key(v: &Value) -> Result<Key> {
    fields(v, &["kind", "sha256", "byte_length"])?;
    ensure(v["kind"] == "json", "JSON reference")?;
    Ok((digest(&v["sha256"])?, number(&v["byte_length"])?))
}
pub(super) fn reference(v: &Value) -> Value {
    let bytes = encode(v);
    json!({"kind":"json","sha256":hash(&bytes),"byte_length":bytes.len().to_string()})
}
fn schema(v: &Value, name: &str, extra: &[&str]) -> Result<()> {
    let mut names = vec!["schema", "project_id", "extensions"];
    names.extend_from_slice(extra);
    fields(v, &names)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"] == format!("example.ps2.{name}")
            && v["schema"]["version"] == 1
            && v["project_id"] == PROJECT
            && v["extensions"].is_object(),
        "typed schema dispatch",
    )
}
#[derive(Clone)]
pub(super) struct Store {
    pub(super) selector: Value,
    pub(super) objects: BTreeMap<Key, (Value, u8)>,
}
impl Store {
    pub(super) fn load(c: &Value) -> Result<Self> {
        let selector = c["selector"].clone();
        dispatch(&selector)?;
        let allocations = c["allocations"].as_object().ok_or("allocations")?;
        ensure(allocations.len() == 2, "two observed tip allocations")?;
        let tips = selector["tips"].as_array().ok_or("tips")?;
        ensure(
            tips.len() == allocations.len(),
            "exact tip allocation coverage",
        )?;
        let mut tip_ids = BTreeSet::new();
        for tip in tips {
            fields(
                tip,
                &[
                    "allocation_id",
                    "arena",
                    "owned_extent",
                    "charged_high_water",
                ],
            )?;
            ensure(
                tip_ids.insert(id(&tip["allocation_id"])?),
                "unique tip identity",
            )?;
        }
        let mut packs = BTreeMap::new();
        for (allocation, v) in allocations {
            id(&json!(allocation))?;
            let bytes = unhex(v["hex"].as_str().ok_or("pack hex")?)?;
            ensure(hash(&bytes) == v["sha256"], "allocation digest")?;
            let arena = v["arena"].as_str().ok_or("arena")?;
            frame_ranges(&bytes, arena)?;
            let tip = selector["tips"]
                .as_array()
                .ok_or("tips")?
                .iter()
                .find(|t| t["allocation_id"] == *allocation)
                .ok_or("unowned tree storage")?;
            ensure(
                tip["arena"] == arena
                    && number(&tip["owned_extent"])? == bytes.len() as u64
                    && number(&tip["charged_high_water"])? >= bytes.len() as u64,
                "observed tip bounds",
            )?;
            packs.insert(allocation.clone(), (bytes, arena.to_string()));
        }
        let (locator, tag) = physical(&packs, &selector["locator"])?;
        ensure(tag == 3, "direct locator frame")?;
        schema(&locator, "locator-leaf", &["count", "entries"])?;
        let entries = locator["entries"].as_array().ok_or("locator entries")?;
        ensure(
            entries.len() as u64 == number(&locator["count"])? && entries.len() <= 256,
            "locator bound",
        )?;
        let mut objects = BTreeMap::new();
        let mut previous = None;
        for entry in entries {
            fields(entry, &["object", "membership", "physical"])?;
            let k = key(&entry["object"])?;
            ensure(
                previous.as_ref().is_none_or(|p| p < &k),
                "unique locator order",
            )?;
            let (v, tag) = physical(&packs, &entry["physical"])?;
            ensure(
                key(&reference(&v))? == k
                    && ((tag == 1 && entry["membership"] == "semantic")
                        || (tag == 2 && entry["membership"] == "ownership")),
                "locator identity/membership",
            )?;
            previous = Some(k.clone());
            objects.insert(k, (v, tag));
        }
        Ok(Self { selector, objects })
    }
    pub(super) fn get(&self, r: &Value, tag: u8) -> Result<Value> {
        let k = key(r)?;
        let (v, actual) = self.objects.get(&k).ok_or("missing typed edge")?;
        ensure(
            *actual == tag && key(&reference(v))? == k,
            "typed bytes/identity",
        )?;
        Ok(v.clone())
    }
    pub(super) fn append(&mut self, v: &Value, tag: u8) -> Value {
        let r = reference(v);
        self.objects.insert(key(&r).unwrap(), (v.clone(), tag));
        r
    }
}
fn physical(packs: &BTreeMap<String, (Vec<u8>, String)>, r: &Value) -> Result<(Value, u8)> {
    fields(
        r,
        &[
            "allocation_id",
            "arena",
            "offset",
            "byte_length",
            "record_sha256",
        ],
    )?;
    let (pack, arena) = packs
        .get(&id(&r["allocation_id"])?)
        .ok_or("physical allocation")?;
    ensure(r["arena"] == *arena, "physical arena")?;
    let start = usize::try_from(number(&r["offset"])?).map_err(|_| "offset")?;
    let end = start
        .checked_add(usize::try_from(number(&r["byte_length"])?).map_err(|_| "length")?)
        .ok_or("physical overflow")?;
    let tag = frame_ranges(pack, arena)?
        .into_iter()
        .find(|(s, e, _)| *s == start && *e == end)
        .ok_or("physical frame boundary")?
        .2;
    ensure(tag != 0, "padding is not an object")?;
    let frame = pack.get(start..end).ok_or("frame range")?;
    ensure(
        hash(frame) == digest(&r["record_sha256"])?,
        "physical frame digest",
    )?;
    let v = photara_store::package::parse_canonical_json(
        &frame[16..],
        photara_store::package::JsonLimits::default(),
    )
    .map_err(|_| "canonical frame JSON")?;
    Ok((v, tag))
}
pub(super) fn dispatch(s: &Value) -> Result<()> {
    fields(
        s,
        &[
            "kind",
            "minimum_reader",
            "required_features",
            "roles",
            "owned_charges",
            "domain_incarnation",
            "tips",
            "sealed_observations",
            "total_charge",
            "locator",
        ],
    )?;
    fields(&s["roles"], &["active", "recovery"])?;
    id(&s["domain_incarnation"])?;
    ensure(
        s["kind"] == "typed-branches-draft",
        "flat/tree selector dispatch",
    )?;
    fields(&s["minimum_reader"], &["major", "minor"])?;
    ensure(
        s["minimum_reader"]["major"] == 1 && s["minimum_reader"]["minor"] == 3,
        "reader floor",
    )?;
    let f = s["required_features"].as_array().ok_or("features")?;
    ensure(
        f == &vec![
            json!("example.ps2.scalable-storage.draft-v1"),
            json!(FEATURE),
        ],
        "required capability dispatch",
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Inventory,
    Id,
    Ordinal,
    Charge,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Inventory => "inventory",
            Self::Id => "operation-id",
            Self::Ordinal => "operation-ordinal",
            Self::Charge => "owned-charge",
        }
    }
    fn tag(self) -> u8 {
        if self == Self::Charge { 2 } else { 1 }
    }
    fn order(self, v: &Value) -> Result<Order> {
        Ok(match self {
            Self::Inventory => Order::Object(key(v)?),
            Self::Ordinal => Order::Number(number(v)?),
            Self::Id | Self::Charge => Order::Id(id(v)?),
        })
    }
    fn entry(self, v: &Value) -> Result<Value> {
        match self {
            Self::Inventory => {
                key(v)?;
                Ok(v.clone())
            }
            Self::Id | Self::Ordinal => {
                fields(
                    v,
                    &[
                        "operation_id",
                        "request_sha256",
                        "acceptance_ordinal",
                        "receipt",
                    ],
                )?;
                id(&v["operation_id"])?;
                digest(&v["request_sha256"])?;
                ensure(number(&v["acceptance_ordinal"])? > 0, "positive ordinal")?;
                key(&v["receipt"])?;
                Ok(v[if self == Self::Id {
                    "operation_id"
                } else {
                    "acceptance_ordinal"
                }]
                .clone())
            }
            Self::Charge => {
                fields(v, &["allocation_id", "charge", "charged_high_water"])?;
                id(&v["allocation_id"])?;
                key(&v["charge"])?;
                number(&v["charged_high_water"])?;
                Ok(v["allocation_id"].clone())
            }
        }
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Order {
    Object(Key),
    Id(String),
    Number(u64),
}
pub(super) struct Proof {
    pub(super) entries: Vec<Value>,
    pub(super) nodes: BTreeSet<Key>,
    pub(super) depth: usize,
    pub(super) charge: u64,
}
pub(super) fn tree(store: &Store, r: &Value, kind: Kind) -> Result<Proof> {
    let mut nodes = BTreeSet::new();
    let (entries, depth, charge) = walk(store, r, kind, &mut nodes, 0)?;
    Ok(Proof {
        entries,
        nodes,
        depth,
        charge,
    })
}
fn sum(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("tree aggregate overflow")
}
#[allow(
    clippy::too_many_lines,
    reason = "One schema-directed recursion validates exact subtree summaries"
)]
fn walk(
    store: &Store,
    r: &Value,
    kind: Kind,
    nodes: &mut BTreeSet<Key>,
    level: usize,
) -> Result<(Vec<Value>, usize, u64)> {
    ensure(level < 16 && nodes.len() < 256, "tree traversal bound")?;
    ensure(nodes.insert(key(r)?), "tree alias/cycle")?;
    let v = store.get(r, kind.tag())?;
    let count = number(&v["count"])?;
    let is_charge = kind == Kind::Charge;
    let leaf = v["schema"]["id"] == format!("example.ps2.{}-leaf", kind.name());
    let mut extra = vec!["count", if leaf { "entries" } else { "children" }];
    if is_charge {
        extra.push("charged_high_water");
    }
    schema(
        &v,
        &format!("{}-{}", kind.name(), if leaf { "leaf" } else { "branch" }),
        &extra,
    )?;
    let mut entries = vec![];
    let mut depth = 1;
    let mut charge = 0;
    if leaf {
        entries.clone_from(v["entries"].as_array().ok_or("leaf entries")?);
        ensure(
            !entries.is_empty() && entries.len() <= 3,
            "leaf probe bound",
        )?;
        for e in &entries {
            kind.entry(e)?;
            if is_charge {
                charge = sum(charge, number(&e["charged_high_water"])?)?;
            }
        }
    } else {
        let children = v["children"].as_array().ok_or("children")?;
        ensure((2..=4).contains(&children.len()), "branch fanout")?;
        let mut claimed_count = 0;
        let mut claimed_charge = 0;
        for child in children {
            let mut names = vec!["first", "last", "count", "child"];
            if is_charge {
                names.push("charged_high_water");
            }
            fields(child, &names)?;
            kind.order(&child["first"])?;
            kind.order(&child["last"])?;
            claimed_count = sum(claimed_count, number(&child["count"])?)?;
            if is_charge {
                claimed_charge = sum(claimed_charge, number(&child["charged_high_water"])?)?;
            }
        }
        ensure(claimed_count == count, "branch summary total")?;
        if is_charge {
            ensure(
                claimed_charge == number(&v["charged_high_water"])?,
                "branch charge summary total",
            )?;
        }
        for child in children {
            let (items, height, subtotal) = walk(store, &child["child"], kind, nodes, level + 1)?;
            ensure(
                items.len() as u64 == number(&child["count"])?
                    && kind.entry(items.first().ok_or("empty child")?)? == child["first"]
                    && kind.entry(items.last().ok_or("empty child")?)? == child["last"],
                "exact child range/count",
            )?;
            if is_charge {
                ensure(
                    subtotal == number(&child["charged_high_water"])?,
                    "exact child charge",
                )?;
            }
            depth = depth.max(height + 1);
            charge = sum(charge, subtotal)?;
            entries.extend(items);
        }
    }
    ensure(
        count == entries.len() as u64 && count <= 4096,
        "exact subtree count/bound",
    )?;
    let mut prior = None;
    for e in &entries {
        let k = kind.order(&kind.entry(e)?)?;
        ensure(
            prior.as_ref().is_none_or(|p| p < &k),
            "disjoint ordered unique coverage",
        )?;
        prior = Some(k);
    }
    if is_charge {
        ensure(
            charge == number(&v["charged_high_water"])?,
            "exact subtree charge",
        )?;
    }
    Ok((entries, depth, charge))
}
