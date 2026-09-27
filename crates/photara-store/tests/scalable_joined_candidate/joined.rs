//! Complete settled candidate traversal. No file writes or production dispatch.
use super::{operations, packed, resource, trees};
use packed::{Result, ensure, frame_ranges, hash, unhex};
use resource::{fields, number, parse};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use trees::{Key, Kind, encode, key, reference, tree};
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
const PROFILE: &str = "example.ps2.synthetic-local-profile-v1";
const INCARNATION: &str = "71000000-0000-4000-8000-000000000001";
fn id(v: &Value) -> Result<String> {
    let s = v.as_str().ok_or("UUID")?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!u.is_nil() && u.to_string() == s, "canonical UUID")?;
    Ok(s.into())
}
fn digest(v: &Value) -> Result<String> {
    let s = v.as_str().ok_or("digest")?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )?;
    Ok(s.into())
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
            && v["extensions"]
                .as_object()
                .is_some_and(|o| o.keys().all(|s| s.contains('.'))),
        "typed selected schema",
    )
}
fn witness(v: &Value, allocation: &str) -> Result<()> {
    fields(
        v,
        &["profile", "incarnation", "allocation_id", "device", "inode"],
    )?;
    ensure(
        v["profile"] == PROFILE
            && v["incarnation"] == INCARNATION
            && id(&v["allocation_id"])? == allocation,
        "local allocation witness",
    )?;
    number(&v["device"])?;
    number(&v["inode"])?;
    Ok(())
}
fn rounded(n: u64) -> Result<u64> {
    n.checked_add(4095)
        .and_then(|v| (v / 4096).checked_mul(4096))
        .ok_or("charge overflow")
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("charge overflow")
}
#[derive(Clone)]
pub(super) struct Allocation {
    pub(super) bytes: Vec<u8>,
    pub(super) arena: String,
    pub(super) witness: Value,
}
#[derive(Clone)]
pub(super) struct World {
    pub(super) manifest: Value,
    pub(super) head: Value,
    pub(super) commit: Value,
    pub(super) allocations: BTreeMap<String, Allocation>,
    pub(super) loose: BTreeMap<String, Vec<u8>>,
    pub(super) source: BTreeMap<String, Vec<u8>>,
    pub(super) source_witnesses: BTreeMap<String, Value>,
}
impl World {
    pub(super) fn load(c: &Value) -> Result<Self> {
        ensure(
            c["status"] == "unfrozen-joined-settled-candidate" && c["qualification"] == false,
            "joined candidate scope",
        )?;
        let get = |name: &str| {
            parse(
                c["bootstrap"][name]
                    .as_str()
                    .ok_or("bootstrap bytes")?
                    .as_bytes(),
            )
        };
        let manifest = get("manifest")?;
        let head = get("head")?;
        let commit = get("commit")?;
        let mut allocations = BTreeMap::new();
        for (allocation, v) in c["allocations"].as_object().ok_or("allocations")? {
            id(&json!(allocation))?;
            fields(v, &["arena", "hex", "sha256", "witness"])?;
            let bytes = unhex(v["hex"].as_str().ok_or("allocation bytes")?)?;
            let arena = v["arena"].as_str().ok_or("arena")?.to_string();
            ensure(
                matches!(arena.as_str(), "data" | "metadata"),
                "allocation arena dispatch",
            )?;
            ensure(
                hash(&bytes) == digest(&v["sha256"])?,
                "allocation fixed digest",
            )?;
            frame_ranges(&bytes, &arena)?;
            witness(&v["witness"], allocation)?;
            allocations.insert(
                allocation.clone(),
                Allocation {
                    bytes,
                    arena,
                    witness: v["witness"].clone(),
                },
            );
        }
        let loose = c["loose"]
            .as_object()
            .ok_or("loose bootstrap")?
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.clone(),
                    v.as_str().ok_or("loose bytes")?.as_bytes().to_vec(),
                ))
            })
            .collect::<Result<_>>()?;
        let source = c["source_files"]
            .as_object()
            .ok_or("conversion snapshot")?
            .iter()
            .map(|(k, v)| Ok((k.clone(), unhex(v.as_str().ok_or("snapshot hex")?)?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            manifest,
            head,
            commit,
            allocations,
            loose,
            source,
            source_witnesses: c["source_witnesses"]
                .as_object()
                .ok_or("snapshot witnesses")?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        })
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Validate unchanged outer envelope and selected candidate dispatch before packed reads"
    )]
    pub(super) fn bootstrap(&self, ops: &Value) -> Result<Value> {
        ensure(
            self.manifest == ops["records"]["manifest"]["input"],
            "original bootstrap identity",
        )?;
        fields(
            &self.head,
            &["schema", "project_id", "commit_id", "commit_sha256"],
        )?;
        fields(&self.head["schema"], &["id", "version"])?;
        ensure(
            self.head["schema"] == json!({"id":"photara.package.head","version":1})
                && self.head["project_id"] == PROJECT
                && self.head["commit_id"] == self.commit["commit_id"]
                && self.head["commit_sha256"] == hash(&encode(&self.commit)),
            "HEAD is the sole selected commit",
        )?;
        fields(
            &self.commit,
            &[
                "schema",
                "project_id",
                "commit_id",
                "package_revision",
                "bootstrap_sha256",
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
        ensure(
            self.commit["schema"] == json!({"id":"photara.package.commit","version":1})
                && self.commit["project_id"] == PROJECT
                && self.commit["parent"].is_null()
                && self.commit["package_revision"] == "1"
                && self.commit["bootstrap_sha256"] == hash(&encode(&self.manifest))
                && self.commit["minimum_reader"] == json!({"major":1,"minor":3}),
            "outer commit dispatch",
        )?;
        photara_core::context::value::Timestamp::try_from(
            self.commit["created_at"]
                .as_str()
                .ok_or("commit timestamp")?
                .to_owned(),
        )
        .map_err(|_| "commit timestamp")?;
        ensure(
            self.commit["extensions"]
                .as_object()
                .is_some_and(|o| o.keys().all(|k| k.contains('.'))),
            "commit extension namespace",
        )?;
        id(&self.commit["commit_id"])?;
        id(&self.commit["write_id"])?;
        number(&self.commit["package_revision"])?;
        let mut features = ops["records"]["manifest"]["input"]["required_features"]
            .as_array()
            .ok_or("manifest features")?
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        features.extend(
            [
                "photara.sealed-roots.v1",
                "photara.resource-backings.v1",
                "example.ps2.scalable-storage.draft-v1",
                "example.ps2.typed-branches.draft-v1",
                "example.ps2.owned-accounting.draft-v1",
            ]
            .map(String::from),
        );
        features.sort();
        ensure(
            self.commit["required_features"] == json!(features),
            "required joined capabilities",
        )?;
        let root = self.commit["root_set"].clone();
        schema(
            &root,
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
                && root["bootstrap_sha256"] == hash(&encode(&self.manifest))
                && root["kind"] == "scalable-draft"
                && root["pinned_roots"] == json!([]),
            "selected RootSet identity/pins",
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
        number(&root["placement"]["generation"])?;
        Ok(root)
    }
    pub(super) fn rehash_head(&mut self) {
        self.head["commit_sha256"] = json!(hash(&encode(&self.commit)));
    }
    fn loose(&self, r: &Value) -> Result<Value> {
        let k = key(r)?;
        let bytes = self.loose.get(&k.0).ok_or("missing direct accounting")?;
        ensure(
            bytes.len() as u64 == k.1 && hash(bytes) == k.0,
            "direct accounting bytes",
        )?;
        parse(bytes)
    }
    fn physical(&self, r: &Value, tag: u8, used: &mut BTreeSet<String>) -> Result<Value> {
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
        let allocation = id(&r["allocation_id"])?;
        let f = self
            .allocations
            .get(&allocation)
            .ok_or("missing physical allocation")?;
        ensure(r["arena"] == f.arena, "physical arena")?;
        let start = usize::try_from(number(&r["offset"])?).map_err(|_| "offset")?;
        let end = start
            .checked_add(usize::try_from(number(&r["byte_length"])?).map_err(|_| "length")?)
            .ok_or("physical range overflow")?;
        ensure(
            frame_ranges(&f.bytes, &f.arena)?.contains(&(start, end, tag)),
            "exact typed frame boundary",
        )?;
        let frame = f.bytes.get(start..end).ok_or("physical extent")?;
        ensure(
            hash(frame) == digest(&r["record_sha256"])?,
            "physical frame hash",
        )?;
        used.insert(allocation);
        parse(&frame[16..])
    }
}
struct Resolver {
    objects: BTreeMap<Key, (Value, u8)>,
    used: BTreeSet<String>,
}
fn locator(
    w: &World,
    r: &Value,
    visited: &mut BTreeSet<String>,
    used: &mut BTreeSet<String>,
    depth: usize,
) -> Result<Vec<Value>> {
    ensure(
        depth < 16 && visited.len() < 256 && visited.insert(hash(&encode(r))),
        "locator alias/cycle/bound",
    )?;
    let v = w.physical(r, 3, used)?;
    let mut entries = vec![];
    if v["schema"]["id"] == "example.ps2.locator-leaf" {
        schema(&v, "locator-leaf", &["count", "entries"])?;
        entries.clone_from(v["entries"].as_array().ok_or("locator entries")?);
        ensure(
            !entries.is_empty() && entries.len() <= 256,
            "locator leaf bound",
        )?;
    } else {
        schema(&v, "locator-branch", &["count", "children"])?;
        let children = v["children"].as_array().ok_or("locator children")?;
        ensure((2..=4).contains(&children.len()), "locator fanout")?;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            let items = locator(w, &child["child"], visited, used, depth + 1)?;
            ensure(
                items.len() as u64 == number(&child["count"])?
                    && items.first().ok_or("empty locator")?["object"] == child["first"]
                    && items.last().ok_or("empty locator")?["object"] == child["last"],
                "exact locator child range/count",
            )?;
            entries.extend(items);
        }
    }
    ensure(
        entries.len() as u64 == number(&v["count"])?,
        "locator total",
    )?;
    let mut prior = None;
    for entry in &entries {
        fields(entry, &["object", "membership", "physical"])?;
        let k = key(&entry["object"])?;
        ensure(
            prior.as_ref().is_none_or(|p| p < &k),
            "locator unique order",
        )?;
        prior = Some(k);
    }
    Ok(entries)
}
impl Resolver {
    fn new(w: &World, r: &Value) -> Result<Self> {
        let mut used = BTreeSet::new();
        let entries = locator(w, r, &mut BTreeSet::new(), &mut used, 0)?;
        let mut objects = BTreeMap::new();
        for e in entries {
            let tag = match e["membership"].as_str() {
                Some("semantic") => 1,
                Some("ownership") => 2,
                _ => return Err("locator membership"),
            };
            let v = w.physical(&e["physical"], tag, &mut used)?;
            let k = key(&e["object"])?;
            ensure(key(&reference(&v))? == k, "located exact object bytes")?;
            ensure(objects.insert(k, (v, tag)).is_none(), "locator duplicate")?;
        }
        Ok(Self { objects, used })
    }
    fn get(&self, r: &Value, tag: u8) -> Result<Value> {
        let (v, t) = self
            .objects
            .get(&key(r)?)
            .ok_or("missing selected typed edge")?;
        ensure(*t == tag, "selected edge membership")?;
        Ok(v.clone())
    }
    fn tree(&self, r: &Value, kind: Kind) -> Result<trees::Proof> {
        tree(
            &trees::Store {
                selector: Value::Null,
                objects: self.objects.clone(),
            },
            r,
            kind,
        )
    }
}
pub(super) struct RoleProof {
    pub(super) semantic: BTreeSet<Key>,
    pub(super) physical_objects: BTreeSet<Key>,
    pub(super) used: BTreeSet<String>,
    pub(super) claims: Vec<Value>,
    pub(super) state: Value,
    pub(super) resource_supported: bool,
}
fn source_store(w: &World, r: &Resolver, root: &Value, profile: &Value) -> resource::Store {
    resource::Store {
        records: r
            .objects
            .iter()
            .map(|(k, (v, _))| (k.0.clone(), encode(v)))
            .collect(),
        roots: Value::Null,
        conversion: root["conversion_source"].clone(),
        source: w.source.clone(),
        profile: profile.clone(),
    }
}
fn owner(
    world: &World,
    res: &Resolver,
    root: &Value,
    seen: &mut BTreeSet<String>,
    nodes: &mut BTreeSet<Key>,
    used: &mut BTreeSet<String>,
    depth: usize,
) -> Result<Vec<Value>> {
    ensure(
        depth < 16 && seen.len() < 256 && seen.insert(hash(&encode(root))),
        "ownership alias/cycle/bound",
    )?;
    let value = world.physical(root, 2, used)?;
    let object = key(&reference(&value))?;
    ensure(
        res.objects.get(&object) == Some(&(value.clone(), 2)),
        "owner exact locator membership",
    )?;
    nodes.insert(object);
    if value["schema"]["id"] == "example.ps2.ownership-claim" {
        schema(
            &value,
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
        let allocation = id(&value["allocation_id"])?;
        let allocation_file = world
            .allocations
            .get(&allocation)
            .ok_or("claim allocation")?;
        fields(&value["authenticated_prefix"], &["byte_length", "sha256"])?;
        let extent = number(&value["owned_extent"])?;
        let end = usize::try_from(number(&value["authenticated_prefix"]["byte_length"])?)
            .map_err(|_| "prefix extent")?;
        ensure(
            value["arena"] == allocation_file.arena
                && extent == allocation_file.bytes.len() as u64
                && end <= allocation_file.bytes.len()
                && value["authenticated_prefix"]["sha256"] == hash(&allocation_file.bytes[..end]),
            "exact owned extent/prefix",
        )?;
        if value["sealed"] == true {
            ensure(
                end == allocation_file.bytes.len() && !value["sealed_charge"].is_null(),
                "sealed full prefix",
            )?;
        } else {
            ensure(
                value["sealed"] == false && value["sealed_charge"].is_null() && end == 0,
                "growable prefix fixture",
            )?;
        }
        Ok(vec![value])
    } else {
        schema(&value, "ownership-branch", &["count", "children"])?;
        let children = value["children"].as_array().ok_or("ownership children")?;
        ensure((2..=8).contains(&children.len()), "ownership fanout")?;
        let mut claims = vec![];
        for child in children {
            fields(child, &["allocation_id", "node"])?;
            let sub = owner(world, res, &child["node"], seen, nodes, used, depth + 1)?;
            ensure(
                sub.first().ok_or("empty owner")?["allocation_id"] == child["allocation_id"],
                "ownership separator",
            )?;
            claims.extend(sub);
        }
        ensure(
            claims.len() as u64 == number(&value["count"])?,
            "ownership count",
        )?;
        let mut prior = None;
        for claim in &claims {
            let allocation_id = id(&claim["allocation_id"])?;
            ensure(
                prior.as_ref().is_none_or(|p| p < &allocation_id),
                "unique allocation ownership",
            )?;
            prior = Some(allocation_id);
        }
        Ok(claims)
    }
}
struct Accounting {
    global: BTreeSet<Key>,
    charges: BTreeMap<String, Value>,
    value: Value,
    total: u64,
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep selected pack and retained-file accounting checks in exact authority order"
)]
fn accounting(w: &World, res: &Resolver, root: &Value, profile: &Value) -> Result<Accounting> {
    let value = w.loose(&root["placement"]["accounting"])?;
    schema(
        &value,
        "accounting-tree",
        &[
            "profile",
            "incarnation",
            "standing_control",
            "tips",
            "sealed_charge_tree",
            "sealed_observations",
            "tickets",
            "retained_conversion",
            "total_charge",
        ],
    )?;
    ensure(
        value["profile"] == PROFILE
            && value["incarnation"] == INCARNATION
            && value["tickets"] == json!([]),
        "settled accounting scope",
    )?;
    let proof = res.tree(&value["sealed_charge_tree"], Kind::Charge)?;
    let mut global = proof.nodes;
    let mut charges = BTreeMap::new();
    let observations = value["sealed_observations"]
        .as_array()
        .ok_or("original sealed observations")?;
    ensure(
        observations.len() == proof.entries.len(),
        "exact sealed observation coverage",
    )?;
    let mut observed_ids = BTreeSet::new();
    for observation in observations {
        fields(observation, &["observation_id", "witness"])?;
        ensure(
            observed_ids.insert(id(&observation["observation_id"])?),
            "unique sealed observation",
        )?;
    }

    for entry in proof.entries {
        let c = res.get(&entry["charge"], 2)?;
        schema(
            &c,
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
        fields(&c["observation"], &["kind", "observation_id"])?;
        id(&c["observation"]["observation_id"])?;
        let allocation = id(&c["allocation_id"])?;
        let file = w
            .allocations
            .get(&allocation)
            .ok_or("sealed contents required")?;
        ensure(
            c["allocation_id"] == entry["allocation_id"]
                && c["domain_incarnation"] == INCARNATION
                && c["arena"] == file.arena
                && number(&c["measured_extent"])? == file.bytes.len() as u64
                && number(&c["charged_high_water"])? == rounded(file.bytes.len() as u64)?
                && entry["charged_high_water"] == c["charged_high_water"]
                && c["observation"]["kind"] == "synthetic-vector",
            "exact supplied sealed charge",
        )?;
        let observed = observations
            .iter()
            .find(|o| o["observation_id"] == c["observation"]["observation_id"])
            .ok_or("unregistered sealed observation")?;
        witness(&observed["witness"], &allocation)?;
        ensure(
            observed["witness"] == file.witness,
            "original sealed allocation witness",
        )?;
        ensure(
            charges.insert(allocation, c).is_none(),
            "charge allocation duplicate",
        )?;
        global.insert(key(&entry["charge"])?);
    }
    let retained = &value["retained_conversion"];
    fields(
        retained,
        &[
            "source",
            "files",
            "logical_bytes",
            "registered_charge",
            "directory_control",
        ],
    )?;
    ensure(
        retained["source"] == root["conversion_source"],
        "selected conversion accounting identity",
    )?;
    let conversion = res.get(&root["conversion_source"], 1)?;
    ensure(
        conversion["source_bootstrap_sha256"] == root["bootstrap_sha256"]
            && w.source.get("manifest.json") == Some(&encode(&w.manifest)),
        "retained source is original selected bootstrap",
    )?;
    let rs = source_store(w, res, root, profile);
    resource::Intent::new(&rs)?.verify(&rs)?;
    global.insert(key(&root["conversion_source"])?);
    let rows = retained["files"]
        .as_array()
        .ok_or("conversion accounting files")?;
    ensure(
        rows.len() == w.source.len(),
        "conversion exact file coverage",
    )?;
    ensure(
        w.source_witnesses.keys().collect::<BTreeSet<_>>() == w.source.keys().collect(),
        "snapshot witness exact coverage",
    )?;
    let mut logical = 0;
    let mut registered = 0;
    for (row, (path, bytes)) in rows.iter().zip(&w.source) {
        fields(
            row,
            &["path", "logical_bytes", "registered_charge", "witness"],
        )?;
        fields(
            &row["witness"],
            &[
                "profile",
                "incarnation",
                "namespace",
                "path",
                "device",
                "inode",
            ],
        )?;
        ensure(
            row["witness"]["profile"] == PROFILE
                && row["witness"]["incarnation"] == INCARNATION
                && row["witness"]["namespace"] == conversion["snapshot_directory"]
                && row["witness"]["path"] == json!(path.split('/').collect::<Vec<_>>())
                && w.source_witnesses.get(path) == Some(&row["witness"]),
            "original retained file witness",
        )?;
        number(&row["witness"]["device"])?;
        number(&row["witness"]["inode"])?;
        ensure(
            row["path"] == *path
                && number(&row["logical_bytes"])? == bytes.len() as u64
                && number(&row["registered_charge"])? == rounded(bytes.len() as u64)?,
            "per-path retained conversion charge",
        )?;
        logical = add(logical, bytes.len() as u64)?;
        registered = add(registered, rounded(bytes.len() as u64)?)?;
    }
    let directory = number(&retained["directory_control"])?;
    ensure(
        number(&retained["logical_bytes"])? == logical
            && number(&retained["registered_charge"])? == registered
            && directory >= encode(&retained["files"]).len() as u64,
        "conversion logical/highwater/directory bounds",
    )?;
    let standing = number(&value["standing_control"])?;
    let control = encode(&w.head).len()
        + encode(&w.commit).len()
        + encode(&w.manifest).len()
        + encode(&value).len();
    ensure(
        standing >= control as u64,
        "standing selected control bound",
    )?;
    let mut total = add(add(add(standing, proof.charge)?, registered)?, directory)?;
    let mut ids = BTreeSet::new();
    for tip in value["tips"].as_array().ok_or("accounting tips")? {
        fields(tip, &["witness", "arena", "extent", "registered_charge"])?;
        let allocation = id(&tip["witness"]["allocation_id"])?;
        witness(&tip["witness"], &allocation)?;
        ensure(
            ids.insert(allocation.clone())
                && !charges.contains_key(&allocation)
                && matches!(tip["arena"].as_str(), Some("data" | "metadata"))
                && number(&tip["registered_charge"])? == rounded(number(&tip["extent"])?)?,
            "unique disjoint observed tip registry",
        )?;
        total = add(total, number(&tip["registered_charge"])?)?;
    }
    ensure(
        total == number(&value["total_charge"])?,
        "exact once-only accounting total",
    )?;
    Ok(Accounting {
        global,
        charges,
        value,
        total,
    })
}
#[allow(
    clippy::too_many_lines,
    reason = "One ordered pass ties semantic, physical and accounting roots"
)]
pub(super) fn verify_role(
    w: &World,
    ops: &Value,
    profile: &Value,
    role: &str,
) -> Result<RoleProof> {
    ensure(matches!(role, "active" | "recovery"), "role")?;
    let root = w.bootstrap(ops)?;
    let place = &root["placement"];
    let res = Resolver::new(w, &place[format!("{role}_locator")])?;
    let selected = res.get(&root[role], 1)?;
    schema(
        &selected,
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
    id(&selected["root_id"])?;
    ensure(
        selected["library_id"] == LIBRARY
            && selected["bootstrap_sha256"] == root["bootstrap_sha256"],
        "StateRoot identity",
    )?;
    ensure(
        selected["predecessor"].is_null(),
        "settled fixture has no invented predecessor",
    )?;
    let original = operations::Store::load(ops)?;
    let original_proof = operations::verify_role(&original, role)?;
    let mut semantic = original_proof.closure;
    let old = original.get(&original.roots[role]["operation_index"])?;
    let index = res.get(&selected["operation_index"], 1)?;
    let mut normalized = index.clone();
    for r in [
        &original.roots[role]["operation_index"],
        &old["by_id"],
        &old["by_ordinal"],
    ] {
        semantic.remove(&key(r)?);
    }
    for (field, kind) in [("by_id", Kind::Id), ("by_ordinal", Kind::Ordinal)] {
        let proof = res.tree(&index[field], kind)?;
        ensure(
            Some(&proof.entries) == original.get(&old[field])?["entries"].as_array(),
            "original reciprocal receipt indexes",
        )?;
        semantic.extend(proof.nodes);
        normalized[field] = old[field].clone();
    }
    ensure(normalized == old, "original operation root")?;
    semantic.insert(key(&selected["operation_index"])?);
    for k in &operations::verify_role(&original, role)?.closure {
        if semantic.contains(k) {
            let r = json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()});
            ensure(
                res.get(&r, 1)? == original.get(&r)?,
                "original real semantic bytes",
            )?;
        }
    }
    for field in ["authored", "history", "accepted", "journal_inclusion"] {
        ensure(
            selected[field] == original.roots[role][field],
            "original selected operation coordinate",
        )?;
    }
    let authored = res.get(&selected["authored"], 1)?;
    ensure(
        selected["authored_revision"] == authored["authored_revision"],
        "selected authored revision",
    )?;
    let resource_store = source_store(w, &res, &root, profile);
    let (resources, supported) = resource::verify_selected_state(
        &resource_store,
        &selected["resource_state"],
        &selected["root_id"],
    )?;
    for k in &resources {
        res.get(
            &json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()}),
            1,
        )?;
    }
    semantic.extend(resources);
    let inventory = res.tree(&selected["inventory"], Kind::Inventory)?;
    ensure(
        inventory
            .entries
            .iter()
            .map(key)
            .collect::<Result<BTreeSet<_>>>()?
            == semantic,
        "exact joined StateRoot inventory",
    )?;
    semantic.extend(inventory.nodes);
    semantic.insert(key(&root[role])?);
    let accounting = accounting(w, &res, &root, profile)?;
    let union = res.tree(&root["inventory"], Kind::Inventory)?;
    let mut physical_objects = semantic.clone();
    physical_objects.extend(accounting.global);
    physical_objects.extend(union.nodes);
    let mut used = res.used.clone();
    let mut owner_nodes = BTreeSet::new();
    let claims = owner(
        w,
        &res,
        &place[format!("{role}_ownership")],
        &mut BTreeSet::new(),
        &mut owner_nodes,
        &mut used,
        0,
    )?;
    physical_objects.extend(owner_nodes);
    ensure(
        physical_objects == res.objects.keys().cloned().collect(),
        "exact joined role locator membership",
    )?;
    let owned = claims
        .iter()
        .map(|c| id(&c["allocation_id"]))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure(used == owned, "all and only required allocations owned")?;
    for claim in &claims {
        let allocation = id(&claim["allocation_id"])?;
        if claim["sealed"] == true {
            let c = accounting
                .charges
                .get(&allocation)
                .ok_or("sealed ownership without charge")?;
            ensure(
                reference(c) == claim["sealed_charge"],
                "canonical sealed charge ownership",
            )?;
        } else {
            let tip = accounting.value["tips"]
                .as_array()
                .ok_or("tips")?
                .iter()
                .find(|t| t["witness"]["allocation_id"] == allocation)
                .ok_or("current ownership without registered tip")?;
            let file = w.allocations.get(&allocation).ok_or("tip contents")?;
            ensure(
                tip["witness"] == file.witness
                    && tip["arena"] == file.arena
                    && number(&tip["extent"])? == file.bytes.len() as u64,
                "original tip witness/extent",
            )?;
        }
    }
    if role == "active" {
        ensure(
            root["operation_index"] == selected["operation_index"],
            "RootSet operation index",
        )?;
        for field in ["authored", "history", "inventory"] {
            ensure(
                w.commit[field] == selected[field],
                "outer active projection",
            )?;
        }
    }
    Ok(RoleProof {
        semantic,
        physical_objects,
        used,
        claims,
        state: selected,
        resource_supported: supported,
    })
}
pub(super) fn verify(w: &World, ops: &Value, profile: &Value) -> Result<u64> {
    let root = w.bootstrap(ops)?;
    let active = verify_role(w, ops, profile, "active")?;
    let recovery = verify_role(w, ops, profile, "recovery")?;
    let res = Resolver::new(w, &root["placement"]["active_locator"])?;
    let accounting = accounting(w, &res, &root, profile)?;
    let mut union = active.semantic;
    union.extend(recovery.semantic);
    union.extend(accounting.global);
    union.insert(key(&root["placement"]["accounting"])?);
    let inventory = res.tree(&root["inventory"], Kind::Inventory)?;
    ensure(
        inventory
            .entries
            .iter()
            .map(key)
            .collect::<Result<BTreeSet<_>>>()?
            == union,
        "exact complete RootSet inventory",
    )?;
    let mut allocations = active.used;
    allocations.extend(recovery.used);
    ensure(
        allocations == w.allocations.keys().cloned().collect(),
        "exact complete allocation ownership",
    )?;
    let mut registered = accounting.charges.keys().cloned().collect::<BTreeSet<_>>();
    for tip in accounting.value["tips"].as_array().ok_or("tips")? {
        let allocation = id(&tip["witness"]["allocation_id"])?;
        let file = w
            .allocations
            .get(&allocation)
            .ok_or("registered tip absent")?;
        ensure(
            file.witness == tip["witness"]
                && file.arena == tip["arena"]
                && file.bytes.len() as u64 == number(&tip["extent"])?,
            "complete tip observation",
        )?;
        registered.insert(allocation);
    }
    ensure(
        registered == allocations,
        "charge coverage has no extra or missing allocation",
    )?;
    ensure(
        w.loose.keys().cloned().collect::<BTreeSet<_>>()
            == BTreeSet::from([key(&root["placement"]["accounting"])?.0]),
        "exact direct control object set",
    )?;
    Ok(accounting.total)
}
