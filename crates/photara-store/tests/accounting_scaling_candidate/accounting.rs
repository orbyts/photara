//! Unfrozen typed accounting trees. Pure bytes only; no portable or writable authority.
use super::packed;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
pub(super) fn ensure(v: bool, e: &'static str) -> Result<()> {
    if v { Ok(()) } else { Err(e) }
}
pub(super) fn encode(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
pub(super) fn reference(v: &Value) -> Value {
    let b = encode(v);
    json!({"kind":"json","sha256":packed::hash(&b),"byte_length":b.len().to_string()})
}
fn fields(v: &Value, keys: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("object")?;
    ensure(
        m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)),
        "exact fields",
    )
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or("string")
}
pub(super) fn number(v: &Value) -> Result<u64> {
    let s = string(v)?;
    let n = s.parse::<u64>().map_err(|_| "u64 range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("array")?;
    ensure(a.len() <= 4096, "array bound")?;
    Ok(a)
}
fn uuid(v: &Value) -> Result<String> {
    let s = string(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!u.is_nil() && u.to_string() == s, "canonical nonnil UUID")?;
    Ok(s.into())
}
fn digest(v: &Value) -> Result<()> {
    let s = string(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )
}
fn schema(v: &Value, name: &str, extra: &[&str]) -> Result<()> {
    let mut names = vec!["schema", "project_id", "extensions"];
    names.extend_from_slice(extra);
    fields(v, &names)?;
    ensure(
        v["schema"] == json!({"id":name,"version":1}) && v["project_id"] == PROJECT,
        "schema dispatch",
    )?;
    ensure(
        v["extensions"].as_object().is_some_and(|m| {
            m.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "extension grammar",
    )
}
fn checked_ref(r: &Value) -> Result<String> {
    fields(r, &["kind", "sha256", "byte_length"])?;
    digest(&r["sha256"])?;
    ensure(
        r["kind"] == "json" && number(&r["byte_length"])? > 0,
        "ObjectRef",
    )?;
    Ok(string(&r["sha256"])?.into())
}
fn rounded(n: u64) -> Result<u64> {
    n.checked_add(4095)
        .and_then(|n| (n / 4096).checked_mul(4096))
        .ok_or("allocation rounding overflow")
}
fn path(v: &Value) -> Result<Vec<String>> {
    let values = array(v)?;
    ensure(!values.is_empty() && values.len() <= 32, "path depth")?;
    let parts = values
        .iter()
        .map(|v| string(v).map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    photara_core::contracts::resource::RelativeComponents::new(parts.clone())
        .map_err(|_| "portable path")?;
    Ok(parts)
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Key {
    Id(String),
    Path(String, Vec<String>),
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Kind {
    Observation,
    Retained,
    Charge,
}
impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Observation => "photara.storage.local-observation",
            Self::Retained => "photara.package.retained-file-charge",
            Self::Charge => "photara.storage.charge",
        }
    }
    fn key(self, v: &Value) -> Result<Key> {
        if self == Self::Retained {
            fields(v, &["conversion_id", "path"])?;
            Ok(Key::Path(uuid(&v["conversion_id"])?, path(&v["path"])?))
        } else {
            Ok(Key::Id(uuid(v)?))
        }
    }
    fn entry_key(self, v: &Value) -> Result<Key> {
        self.key(
            &v[match self {
                Self::Observation => "observation_id",
                Self::Retained => "key",
                Self::Charge => "allocation_id",
            }],
        )
    }
    fn entry_charge(self, v: &Value) -> Result<u64> {
        if self == Self::Observation {
            Ok(0)
        } else {
            number(
                &v[if self == Self::Retained {
                    "registered_charge"
                } else {
                    "charged_high_water"
                }],
            )
        }
    }
}
#[derive(Clone)]
pub(super) struct Store {
    pub(super) objects: BTreeMap<String, Value>,
}
impl Store {
    pub(super) fn load(c: &Value) -> Result<Self> {
        let mut objects = BTreeMap::new();
        let rows = c["records"].as_object().ok_or("records")?;
        ensure(rows.len() <= 1024, "record bound")?;
        for (hash, row) in rows {
            let v = row_value(row)?;
            ensure(hash == string(&row["sha256"])?, "record key")?;
            objects.insert(hash.clone(), v);
        }
        Ok(Self { objects })
    }
    pub(super) fn get(&self, r: &Value) -> Result<Value> {
        let k = checked_ref(r)?;
        let v = self.objects.get(&k).ok_or("referenced object absent")?;
        ensure(reference(v) == *r, "referenced exact bytes")?;
        Ok(v.clone())
    }
}
fn row_value(row: &Value) -> Result<Value> {
    let raw = string(&row["canonical"])?.as_bytes();
    ensure(raw.len() <= 65536, "record byte cap")?;
    let v = photara_store::package::parse_canonical_json(
        raw,
        photara_store::package::JsonLimits {
            max_bytes: 65536,
            max_depth: 32,
            max_members: 256,
            max_array_elements: 4096,
        },
    )
    .map_err(|_| "canonical record")?;
    ensure(
        v == row["input"]
            && packed::hash(raw) == row["sha256"]
            && number(&row["byte_length"])? == raw.len() as u64,
        "fixed canonical vector",
    )?;
    let frame = packed::unhex(string(&row["frame_hex"])?)?;
    let tag = if v["schema"]["id"] == "photara.package.conversion-source" {
        1
    } else {
        2
    };
    ensure(
        packed::frame_ranges(&frame, "data")? == vec![(0, frame.len(), tag)]
            && frame[16..] == *raw
            && packed::hash(&frame) == row["frame_sha256"],
        "exact candidate frame",
    )?;
    Ok(v)
}
pub(super) struct Tree {
    pub(super) entries: Vec<Value>,
    pub(super) nodes: BTreeSet<String>,
    pub(super) charge: u64,
    pub(super) depth: usize,
}
struct Node {
    entries: Vec<Value>,
    first: Key,
    last: Key,
    count: u64,
    charge: u64,
    depth: usize,
}
fn summary(kind: Kind, c: &Value) -> Result<(Key, Key, u64, u64)> {
    let mut names = vec!["first", "last", "count", "child"];
    if kind != Kind::Observation {
        names.push("charged_high_water");
    }
    fields(c, &names)?;
    checked_ref(&c["child"])?;
    let first = kind.key(&c["first"])?;
    let last = kind.key(&c["last"])?;
    let count = number(&c["count"])?;
    ensure(
        first <= last && count > 0 && count <= 4096,
        "child range/count",
    )?;
    Ok((
        first,
        last,
        count,
        if kind == Kind::Observation {
            0
        } else {
            number(&c["charged_high_water"])?
        },
    ))
}
fn checked_entries(kind: Kind, entries: &[Value]) -> Result<(Key, Key, u64)> {
    ensure(!entries.is_empty() && entries.len() <= 2, "leaf capacity")?;
    let mut prior = None;
    let mut total = 0u64;
    for e in entries {
        match kind {
            Kind::Observation => {
                fields(e, &["observation_id", "observation"])?;
                checked_ref(&e["observation"])?;
            }
            Kind::Retained => {
                fields(
                    e,
                    &[
                        "key",
                        "conversion_source",
                        "source_file",
                        "registered_charge",
                        "observation",
                    ],
                )?;
                checked_ref(&e["conversion_source"])?;
                checked_ref(&e["observation"])?;
                fields(&e["source_file"], &["byte_length", "sha256"])?;
                number(&e["source_file"]["byte_length"])?;
                digest(&e["source_file"]["sha256"])?;
            }
            Kind::Charge => {
                fields(e, &["allocation_id", "charge", "charged_high_water"])?;
                checked_ref(&e["charge"])?;
            }
        }
        let k = kind.entry_key(e)?;
        ensure(prior.as_ref().is_none_or(|p| p < &k), "leaf exact ordering")?;
        prior = Some(k);
        total = total
            .checked_add(kind.entry_charge(e)?)
            .ok_or("leaf charge overflow")?;
    }
    Ok((
        kind.entry_key(&entries[0])?,
        kind.entry_key(entries.last().unwrap())?,
        total,
    ))
}
fn node_header(v: &Value, kind: Kind) -> Result<bool> {
    let leaf = v["schema"]["id"] == format!("{}-leaf", kind.prefix());
    let mut fields = vec!["count", if leaf { "entries" } else { "children" }];
    if kind != Kind::Observation {
        fields.push("charged_high_water");
    }
    schema(
        v,
        &format!("{}-{}", kind.prefix(), if leaf { "leaf" } else { "branch" }),
        &fields,
    )?;
    Ok(leaf)
}
fn walk(
    store: &Store,
    r: &Value,
    kind: Kind,
    nodes: &mut BTreeSet<String>,
    level: usize,
) -> Result<Node> {
    ensure(
        level < 16 && nodes.len() < 256 && nodes.insert(checked_ref(r)?),
        "bounded acyclic tree",
    )?;
    let v = store.get(r)?;
    let leaf = node_header(&v, kind)?;
    let result = if leaf {
        let es = array(&v["entries"])?;
        let (first, last, charge) = checked_entries(kind, es)?;
        Node {
            entries: es.clone(),
            first,
            last,
            count: es.len() as u64,
            charge,
            depth: 1,
        }
    } else {
        let children = array(&v["children"])?;
        ensure((2..=4).contains(&children.len()), "branch capacity")?;
        let mut entries = vec![];
        let mut previous = None;
        let mut first = None;
        let mut last = None;
        let mut count = 0u64;
        let mut charge = 0u64;
        let mut depth = 0;
        for c in children {
            let (lower, upper, members, registered) = summary(kind, c)?;
            ensure(
                previous.as_ref().is_none_or(|p| p < &lower),
                "disjoint child ranges",
            )?;
            let child = walk(store, &c["child"], kind, nodes, level + 1)?;
            ensure(
                (
                    child.first.clone(),
                    child.last.clone(),
                    child.count,
                    child.charge,
                ) == (lower.clone(), upper.clone(), members, registered),
                "exact child summary",
            )?;
            if first.is_none() {
                first = Some(lower);
            }
            last = Some(upper.clone());
            previous = Some(upper);
            count = count.checked_add(members).ok_or("branch count overflow")?;
            charge = charge
                .checked_add(registered)
                .ok_or("branch charge overflow")?;
            depth = depth.max(child.depth);
            entries.extend(child.entries);
        }
        Node {
            entries,
            first: first.unwrap(),
            last: last.unwrap(),
            count,
            charge,
            depth: depth + 1,
        }
    };
    ensure(
        result.count == number(&v["count"])?
            && result.count <= 4096
            && (kind == Kind::Observation || result.charge == number(&v["charged_high_water"])?),
        "exact tree aggregate",
    )?;
    Ok(result)
}
pub(super) fn tree(store: &Store, r: &Value, kind: Kind) -> Result<Tree> {
    let mut nodes = BTreeSet::new();
    let n = walk(store, r, kind, &mut nodes, 0)?;
    Ok(Tree {
        entries: n.entries,
        nodes,
        charge: n.charge,
        depth: n.depth,
    })
}
fn observation(v: &Value, profile: &Value, incarnation: &Value) -> Result<()> {
    schema(
        v,
        "photara.storage.local-observation",
        &[
            "observation_id",
            "profile",
            "incarnation",
            "subject",
            "physical",
            "measured_extent",
            "charged_high_water",
        ],
    )?;
    uuid(&v["observation_id"])?;
    uuid(&v["incarnation"])?;
    photara_core::contracts::schema::QualifiedName::parse(string(&v["profile"])?.to_owned())
        .map_err(|_| "profile name")?;
    ensure(
        v["profile"] == *profile && v["incarnation"] == *incarnation,
        "recorded observation scope",
    )?;
    fields(&v["physical"], &["device", "inode"])?;
    number(&v["physical"]["device"])?;
    ensure(number(&v["physical"]["inode"])? > 0, "nonzero inode")?;
    ensure(
        number(&v["charged_high_water"])? == rounded(number(&v["measured_extent"])?)?,
        "synthetic original allocation charge",
    )?;
    let subject = &v["subject"];
    match string(&subject["kind"])? {
        "pack" => {
            fields(subject, &["kind", "allocation_id", "arena"])?;
            uuid(&subject["allocation_id"])?;
            ensure(
                matches!(string(&subject["arena"])?, "data" | "metadata"),
                "pack arena",
            )?;
        }
        "retained-file" => {
            fields(subject, &["kind", "conversion_id", "namespace", "path"])?;
            uuid(&subject["conversion_id"])?;
            path(&subject["namespace"])?;
            path(&subject["path"])?;
        }
        _ => return Err("observation subject dispatch"),
    }
    Ok(())
}
fn charge(v: &Value, obs: &Value, profile: &Value, inc: &Value) -> Result<()> {
    schema(
        v,
        "photara.storage.sealed-charge",
        &[
            "allocation_id",
            "arena",
            "domain_incarnation",
            "measured_extent",
            "charged_high_water",
            "observation",
        ],
    )?;
    observation(obs, profile, inc)?;
    ensure(
        reference(obs) == v["observation"]
            && obs["subject"]
                == json!({"kind":"pack","allocation_id":v["allocation_id"],"arena":v["arena"]})
            && v["domain_incarnation"] == *inc
            && v["measured_extent"] == obs["measured_extent"]
            && v["charged_high_water"] == obs["charged_high_water"],
        "sealed charge observation identity",
    )
}
pub(super) struct Proof {
    pub(super) total: u64,
    pub(super) observations: usize,
    pub(super) retained: usize,
    pub(super) depth: usize,
    pub(super) scope_matches: bool,
}
#[allow(
    clippy::too_many_lines,
    reason = "Explicit typed accounting closure and original file evidence"
)]
pub(super) fn verify(
    c: &Value,
    runtime_profile: &str,
    runtime_incarnation: &str,
    require_local_scope: bool,
) -> Result<Proof> {
    ensure(
        c["status"] == "unfrozen-accounting-scaling-candidate" && c["qualification"] == false,
        "unqualified fixture",
    )?;
    let store = Store::load(c)?;
    let original_bytes =
        include_bytes!("../../../../docs/architecture/proposals/ps2/joined/linked-joined.json");
    ensure(
        c["joined_sha256"] == packed::hash(original_bytes),
        "unchanged joined source epoch",
    )?;
    let joined: Value =
        serde_json::from_slice(original_bytes).map_err(|_| "original joined fixture")?;

    let ledger = store.get(&c["ledger"])?;
    schema(
        &ledger,
        "example.ps2.accounting-scaling-ledger",
        &[
            "profile",
            "incarnation",
            "observation_root",
            "retained_file_charge_root",
            "sealed_charge_root",
            "conversion_source",
            "registered_charge",
        ],
    )?;
    let obs = tree(&store, &ledger["observation_root"], Kind::Observation)?;
    let files = tree(&store, &ledger["retained_file_charge_root"], Kind::Retained)?;
    let charges = tree(&store, &ledger["sealed_charge_root"], Kind::Charge)?;
    let mut observed = BTreeMap::new();
    for e in &obs.entries {
        let v = store.get(&e["observation"])?;
        observation(&v, &ledger["profile"], &ledger["incarnation"])?;
        ensure(
            v["observation_id"] == e["observation_id"]
                && observed
                    .insert(string(&e["observation"]["sha256"])?.to_owned(), v)
                    .is_none(),
            "unique original observation identity",
        )?;
    }
    ensure(
        c["sealed_files"].as_object().ok_or("sealed files")?.len() == charges.entries.len(),
        "exact sealed file set",
    )?;
    let mut used = BTreeSet::new();
    for e in &charges.entries {
        let v = store.get(&e["charge"])?;
        let h = string(&v["observation"]["sha256"])?;
        let obs = observed.get(h).ok_or("selected pack observation missing")?;
        charge(&v, obs, &ledger["profile"], &ledger["incarnation"])?;
        ensure(
            v["allocation_id"] == e["allocation_id"]
                && v["charged_high_water"] == e["charged_high_water"]
                && used.insert(h.to_owned()),
            "exact pack charge entry",
        )?;
        let file = &c["sealed_files"][string(&v["allocation_id"])?];
        ensure(
            *file == joined["allocations"][string(&v["allocation_id"])?],
            "unchanged joined sealed allocation evidence",
        )?;
        let raw = packed::unhex(string(&file["hex"])?)?;
        ensure(
            packed::hash(&raw) == file["sha256"],
            "actual sealed byte digest",
        )?;
        ensure(
            raw.len() as u64 == number(&v["measured_extent"])?
                && file["arena"] == v["arena"]
                && file["witness"]["profile"] == ledger["profile"]
                && file["witness"]["incarnation"] == ledger["incarnation"]
                && file["witness"]["allocation_id"] == v["allocation_id"]
                && file["witness"]["device"] == obs["physical"]["device"]
                && file["witness"]["inode"] == obs["physical"]["inode"],
            "actual original sealed generation",
        )?;
    }
    let conversion = store.get(&ledger["conversion_source"])?;
    ensure(
        joined["records"]
            .as_object()
            .ok_or("original records")?
            .values()
            .any(|r| {
                r["input"] == conversion
                    && r["canonical"]
                        .as_str()
                        .is_some_and(|s| s.as_bytes() == encode(&conversion))
            }),
        "unchanged original ConversionSource bytes",
    )?;
    schema(
        &conversion,
        "photara.package.conversion-source",
        &[
            "conversion_id",
            "snapshot_directory",
            "source_bootstrap_sha256",
            "source_commit_id",
            "source_format_version",
            "source_head_sha256",
            "files",
        ],
    )?;
    ensure(
        c["source_witnesses"] == joined["source_witnesses"],
        "unchanged original retained witnesses",
    )?;
    let mut originals = BTreeMap::new();
    for f in array(&conversion["files"])? {
        fields(f, &["components", "byte_length", "sha256"])?;
        let key = path(&f["components"])?;
        ensure(
            originals.insert(key, f).is_none(),
            "duplicate original source path",
        )?;
    }
    ensure(
        c["source_files"].as_object().ok_or("source files")?.len() == originals.len()
            && c["source_witnesses"]
                .as_object()
                .ok_or("source witnesses")?
                .len()
                == originals.len(),
        "exact retained physical file set",
    )?;
    ensure(
        files.entries.len() == originals.len(),
        "exact original file coverage",
    )?;
    for e in &files.entries {
        ensure(
            e["conversion_source"] == ledger["conversion_source"]
                && e["key"]["conversion_id"] == conversion["conversion_id"],
            "original conversion identity",
        )?;
        let components = path(&e["key"]["path"])?;
        let original = originals
            .get(&components)
            .ok_or("unknown original source file")?;
        ensure(
            e["source_file"]
                == json!({"byte_length":original["byte_length"],"sha256":original["sha256"]}),
            "original logical commitment unchanged",
        )?;
        let h = string(&e["observation"]["sha256"])?;
        let observation = observed
            .get(h)
            .ok_or("selected retained observation missing")?;
        ensure(
            observation["subject"]
                == json!({"kind":"retained-file","conversion_id":conversion["conversion_id"],"namespace":conversion["snapshot_directory"],"path":e["key"]["path"]})
                && observation["measured_extent"] == original["byte_length"]
                && observation["charged_high_water"] == e["registered_charge"]
                && used.insert(h.to_owned()),
            "retained original local observation",
        )?;
        let name = components.join("/");
        let raw = packed::unhex(string(&c["source_files"][&name])?)?;
        let w = &c["source_witnesses"][&name];
        ensure(
            raw.len() as u64 == number(&original["byte_length"])?
                && packed::hash(&raw) == original["sha256"]
                && w["profile"] == ledger["profile"]
                && w["incarnation"] == ledger["incarnation"]
                && w["namespace"] == conversion["snapshot_directory"]
                && w["path"] == e["key"]["path"]
                && w["device"] == observation["physical"]["device"]
                && w["inode"] == observation["physical"]["inode"],
            "actual retained source bytes/witness",
        )?;
    }
    ensure(
        used == observed.keys().cloned().collect(),
        "exact observation closure",
    )?;
    let total = files
        .charge
        .checked_add(charges.charge)
        .ok_or("ledger charge overflow")?;
    ensure(
        total == number(&ledger["registered_charge"])?,
        "exact selected accounting aggregate",
    )?;
    let scope_matches =
        ledger["profile"] == runtime_profile && ledger["incarnation"] == runtime_incarnation;
    ensure(
        !require_local_scope || scope_matches,
        "local profile/incarnation mutation fenced",
    )?;
    Ok(Proof {
        total,
        observations: observed.len(),
        retained: files.entries.len(),
        depth: obs.depth.max(files.depth).max(charges.depth),
        scope_matches,
    })
}
/// Standalone rooted inclusion proof: sibling summaries are commitments, not fetch edges.
/// Authenticity of the supplied original root remains the selected admission's duty.
pub(super) fn sparse(proof: &Value, root: &Value, profile: &str, incarnation: &str) -> Result<u64> {
    fields(
        proof,
        &["root", "allocation_id", "nodes", "charge", "observation"],
    )?;
    ensure(proof["root"] == *root, "original sparse root")?;
    let nodes = array(&proof["nodes"])?;
    ensure(
        !nodes.is_empty() && nodes.len() <= 16 && encode(proof).len() <= 65536,
        "bounded sparse proof",
    )?;
    let target = Key::Id(uuid(&proof["allocation_id"])?);
    let mut expected = root.clone();
    let mut ancestors: Vec<(Key, Key, u64, u64)> = vec![];
    let mut seen = BTreeSet::new();
    let mut found = None;
    for (i, row) in nodes.iter().enumerate() {
        let v = row_value(row)?;
        ensure(
            reference(&v) == expected && seen.insert(string(&expected["sha256"])?.to_owned()),
            "exact sparse path commitment",
        )?;
        let leaf = node_header(&v, Kind::Charge)?;
        if leaf {
            ensure(i + 1 == nodes.len(), "no extraneous proof nodes")?;
            let entries = array(&v["entries"])?;
            let (lower, upper, registered) = checked_entries(Kind::Charge, entries)?;
            ensure(
                number(&v["count"])? == entries.len() as u64
                    && number(&v["charged_high_water"])? == registered,
                "sparse leaf aggregate",
            )?;
            found = Some(
                entries
                    .iter()
                    .find(|e| Kind::Charge.entry_key(e).ok().as_ref() == Some(&target))
                    .ok_or("source absent from proof")?
                    .clone(),
            );
            if let Some((af, al, an, aq)) = ancestors.last() {
                ensure(
                    (lower, upper, entries.len() as u64, registered)
                        == (af.clone(), al.clone(), *an, *aq),
                    "sparse chosen leaf summary",
                )?;
            }
        } else {
            let children = array(&v["children"])?;
            ensure((2..=4).contains(&children.len()), "sparse branch fanout")?;
            let mut first = None;
            let mut last = None;
            let mut count = 0u64;
            let mut charge = 0u64;
            let mut chosen = None;
            for child in children {
                let (lower, upper, members, registered) = summary(Kind::Charge, child)?;
                ensure(
                    last.as_ref().is_none_or(|p| p < &lower),
                    "sparse sibling ranges",
                )?;
                if first.is_none() {
                    first = Some(lower.clone());
                }
                last = Some(upper.clone());
                count = count.checked_add(members).ok_or("sparse count overflow")?;
                charge = charge
                    .checked_add(registered)
                    .ok_or("sparse charge overflow")?;
                if lower <= target && target <= upper {
                    ensure(chosen.is_none(), "ambiguous source range")?;
                    chosen = Some((child["child"].clone(), (lower, upper, members, registered)));
                }
            }
            ensure(
                count == number(&v["count"])? && charge == number(&v["charged_high_water"])?,
                "sparse sibling summary aggregates",
            )?;
            if let Some((lower, upper, members, registered)) = ancestors.last() {
                ensure(
                    (first.unwrap(), last.unwrap(), count, charge)
                        == (lower.clone(), upper.clone(), *members, *registered),
                    "sparse chosen branch summary",
                )?;
            }
            let (next, summary) = chosen.ok_or("sparse source range absent")?;
            expected = next;
            ancestors.push(summary);
        }
    }
    let entry = found.ok_or("incomplete sparse path")?;
    let c = row_value(&proof["charge"])?;
    let obs = row_value(&proof["observation"])?;
    ensure(
        reference(&c) == entry["charge"]
            && c["allocation_id"] == proof["allocation_id"]
            && c["charged_high_water"] == entry["charged_high_water"],
        "exact original source charge",
    )?;
    charge(&c, &obs, &json!(profile), &json!(incarnation))?;
    number(&c["charged_high_water"])
}
