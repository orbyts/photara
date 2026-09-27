//! Proposed settled coordinates, bounded canonical parsing and schema-directed closure.
use super::{accounting, resources};
use photara_core::contracts::schema::QualifiedName;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) type Key = (String, u64);
pub(super) const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
const PROFILE: &str = "example.ps2.synthetic-local-profile-v1";
const INCARNATION: &str = "96000000-0000-4000-8000-000000000001";
const FEATURES: [&str; 6] = [
    "photara.canonical-json.v1",
    "photara.resource-backings.v1",
    "photara.resource-state-trees.v1",
    "photara.scalable-storage.v1",
    "photara.sealed-roots.v1",
    "photara.storage-accounting.v1",
];
pub(super) fn ensure(v: bool, e: &'static str) -> Result<()> {
    if v { Ok(()) } else { Err(e) }
}
pub(super) fn encode(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
pub(super) fn hash(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(b))
}
pub(super) fn parse(b: &[u8]) -> Result<Value> {
    photara_store::package::parse_canonical_json(
        b,
        photara_store::package::JsonLimits {
            max_bytes: 1024 * 1024,
            max_depth: 64,
            max_members: 4096,
            max_array_elements: 4096,
        },
    )
    .map_err(|_| "bounded canonical JSON")
}
pub(super) fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("string")
}
pub(super) fn number(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "decimal range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
pub(super) fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|s| m.contains_key(*s)),
        "exact fields",
    )
}
fn extensions(v: &Value) -> Result<()> {
    ensure(
        v.as_object()
            .is_some_and(|m| m.keys().all(|s| QualifiedName::parse(s.clone()).is_ok())),
        "qualified extension grammar",
    )
}
pub(super) fn schema(v: &Value, id: &str, version: u64, extra: &[&str]) -> Result<()> {
    let mut keys = vec!["schema", "project_id", "extensions"];
    keys.extend(extra);
    fields(v, &keys)?;
    ensure(
        v["schema"] == json!({"id":id,"version":version}) && v["project_id"] == PROJECT,
        "schema/project dispatch",
    )?;
    extensions(&v["extensions"])
}
fn digest(v: &Value) -> Result<&str> {
    let s = text(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )?;
    Ok(s)
}
fn uuid(v: &Value) -> Result<&str> {
    let s = text(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!u.is_nil() && u.to_string() == s, "canonical UUID")?;
    Ok(s)
}
pub(super) fn key(v: &Value) -> Result<Key> {
    fields(v, &["kind", "sha256", "byte_length"])?;
    ensure(
        v["kind"] == "json" && number(&v["byte_length"])? > 0,
        "JSON ObjectRef",
    )?;
    Ok((digest(&v["sha256"])?.into(), number(&v["byte_length"])?))
}
pub(super) fn reference(v: &Value) -> Value {
    let b = encode(v);
    json!({"kind":"json","sha256":hash(&b),"byte_length":b.len().to_string()})
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .filter(|a| a.len() <= 4096)
        .ok_or("bounded array")
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    ensure(
        s.len().is_multiple_of(2)
            && s.len() <= 32 * 1024 * 1024
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "bounded hex",
    )?;
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "hex"))
        .collect()
}
fn rounded(n: u64) -> Result<u64> {
    n.checked_add(4095)
        .map(|n| n / 4096 * 4096)
        .ok_or("charge overflow")
}
#[derive(Clone)]
pub(super) struct Allocation {
    pub bytes: Vec<u8>,
    pub arena: String,
    pub witness: Value,
}
#[derive(Clone)]
pub(super) struct World {
    pub manifest: Value,
    pub head: Value,
    pub commit: Value,
    pub loose: BTreeMap<String, Vec<u8>>,
    pub allocations: BTreeMap<String, Allocation>,
    pub source: BTreeMap<String, Vec<u8>>,
    pub source_witnesses: Value,
    pub journal: Vec<Value>,
}
impl World {
    pub(super) fn load_bytes(bytes: &[u8]) -> Result<Self> {
        let value = photara_store::package::parse_json(
            bytes,
            photara_store::package::JsonLimits {
                max_bytes: 8 * 1024 * 1024,
                max_depth: 64,
                max_members: 4096,
                max_array_elements: 4096,
            },
        )
        .map_err(|_| "bounded fixture input")?;
        Self::load(&value)
    }
    pub(super) fn load(v: &Value) -> Result<Self> {
        ensure(
            v["status"] == "unfrozen-proposed-settled-integrated" && v["qualification"] == false,
            "fixture dispatch",
        )?;
        Self::load_fields(v)
    }
    /// New test-only transition harness; semantic dispatch still occurs in verification.
    pub(super) fn load_transition(v: &Value) -> Result<Self> {
        ensure(
            matches!(
                v["status"].as_str(),
                Some(
                    "unfrozen-admission-only-no-packed-effects"
                        | "unfrozen-prospective-sizing-only-not-selected"
                )
            ) && v["qualification"] == false,
            "transition fixture dispatch",
        )?;
        Self::load_fields(v)
    }
    fn load_fields(v: &Value) -> Result<Self> {
        let entries = v["allocations"].as_object().ok_or("allocations")?;
        ensure(entries.len() <= 16, "allocation count bound")?;
        let mut total = 0;
        let mut allocations = BTreeMap::new();
        for (id, a) in entries {
            uuid(&json!(id))?;
            ensure(a["arena"] == "data" || a["arena"] == "metadata", "arena")?;
            let bytes = unhex(text(&a["hex"])?)?;
            total += bytes.len();
            ensure(total <= 16 * 1024 * 1024, "aggregate allocation bound")?;
            fields(&a["witness"], &["device", "inode"])?;
            number(&a["witness"]["device"])?;
            ensure(number(&a["witness"]["inode"])? > 0, "inode")?;
            allocations.insert(
                id.clone(),
                Allocation {
                    bytes,
                    arena: text(&a["arena"])?.into(),
                    witness: a["witness"].clone(),
                },
            );
        }
        let loose = v["loose"].as_object().ok_or("loose")?;
        ensure(loose.len() <= 24, "control count bound")?;
        let mut loose_bytes = 0usize;
        for b in loose.values() {
            let b = text(b)?.as_bytes();
            ensure(b.len() <= 1024 * 1024, "loose input bound")?;
            loose_bytes = loose_bytes
                .checked_add(b.len())
                .ok_or("loose aggregate overflow")?;
            ensure(loose_bytes <= 2 * 1024 * 1024, "loose aggregate bound")?;
            parse(b)?;
        }
        let source = v["source_files"].as_object().ok_or("source")?;
        ensure(source.len() <= 256, "source file bound")?;
        let mut source_bytes = 0usize;
        for b in source.values() {
            let n = text(b)?.len();
            source_bytes = source_bytes
                .checked_add(n)
                .ok_or("source aggregate overflow")?;
            ensure(source_bytes <= 32 * 1024 * 1024, "source aggregate bound")?;
        }
        Ok(Self {
            manifest: parse(text(&v["bootstrap"]["manifest"])?.as_bytes())?,
            head: parse(text(&v["bootstrap"]["head"])?.as_bytes())?,
            commit: parse(text(&v["bootstrap"]["commit"])?.as_bytes())?,
            loose: loose
                .iter()
                .map(|(k, v)| Ok((k.clone(), text(v)?.as_bytes().to_vec())))
                .collect::<Result<_>>()?,
            allocations,
            source: source
                .iter()
                .map(|(p, v)| Ok((p.clone(), unhex(text(v)?)?)))
                .collect::<Result<_>>()?,
            source_witnesses: v["source_witnesses"].clone(),
            journal: array(&v["journal"])?
                .iter()
                .map(|s| parse(text(s)?.as_bytes()))
                .collect::<Result<_>>()?,
        })
    }
    pub(super) fn loose(&self, r: &Value) -> Result<Value> {
        let k = key(r)?;
        let b = self.loose.get(&k.0).ok_or("missing loose control")?;
        ensure(b.len() as u64 == k.1 && hash(b) == k.0, "exact loose bytes")?;
        parse(b)
    }
    pub(super) fn rehash_head(&mut self) {
        self.head["commit_sha256"] = json!(hash(&encode(&self.commit)));
    }
    pub(super) fn bootstrap(&self) -> Result<Value> {
        self.bootstrap_with(&[])
    }
    fn bootstrap_with(&self, extra_features: &[&str]) -> Result<Value> {
        self.bootstrap_transition(extra_features, None)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep exact original/successor bootstrap validation in one pass"
    )]
    fn bootstrap_transition(
        &self,
        extra_features: &[&str],
        transition: Option<&Transition<'_>>,
    ) -> Result<Value> {
        self.bootstrap_context(
            extra_features,
            transition,
            transition.and_then(|t| t.selection),
        )
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Exact bootstrap dispatch shared by selected proof modes"
    )]
    fn bootstrap_context(
        &self,
        extra_features: &[&str],
        transition: Option<&Transition<'_>>,
        operation: Option<&OperationSelection>,
    ) -> Result<Value> {
        fields(
            &self.head,
            &["schema", "project_id", "commit_id", "commit_sha256"],
        )?;
        ensure(
            self.head["schema"] == json!({"id":"photara.package.head","version":1})
                && self.head["project_id"] == PROJECT
                && self.head["commit_id"] == self.commit["commit_id"]
                && self.head["commit_sha256"] == hash(&encode(&self.commit)),
            "HEAD exact commit",
        )?;
        schema(
            &self.commit,
            "photara.package.commit",
            1,
            &[
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
            ],
        )?;
        let mut features = FEATURES
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<BTreeSet<_>>();
        for f in extra_features {
            features.insert((*f).to_owned());
        }
        for f in array(&self.manifest["required_features"])? {
            features.insert(text(f)?.into());
        }
        ensure(
            self.commit["required_features"] == json!(features)
                && self.commit["minimum_reader"] == json!({"major":1,"minor":3}),
            "required capability/floor dispatch",
        )?;
        uuid(&self.commit["commit_id"])?;
        uuid(&self.commit["write_id"])?;
        if let Some(selected) = operation {
            ensure(
                self.commit == selected.expected_commit,
                "exact original-derived operation selection",
            )?;
        } else if let Some(t) = transition {
            ensure(
                self.manifest == t.old.manifest
                    && self.commit["package_revision"]
                        == json!(
                            number(&t.old.commit["package_revision"])?
                                .checked_add(1)
                                .ok_or("successor revision overflow")?
                                .to_string()
                        )
                    && self.commit["parent"]
                        == json!({"commit_id":t.old.commit["commit_id"],"commit_sha256":hash(&encode(&t.old.commit))})
                    && self.commit["commit_id"] != t.old.commit["commit_id"]
                    && self.commit["write_id"] != t.old.commit["write_id"]
                    && self.commit["required_features"] == t.old.commit["required_features"]
                    && self.commit["minimum_reader"] == t.old.commit["minimum_reader"],
                "exact authenticated successor provenance",
            )?;
        } else {
            ensure(
                self.commit["parent"].is_null() && self.commit["package_revision"] == "1",
                "settled bootstrap provenance",
            )?;
        }
        photara_core::context::value::Timestamp::try_from(
            text(&self.commit["created_at"])?.to_owned(),
        )
        .map_err(|_| "timestamp")?;
        ensure(
            self.manifest["project_id"] == PROJECT
                && self.commit["bootstrap_sha256"] == hash(&encode(&self.manifest)),
            "original bootstrap",
        )?;
        let root = &self.commit["root_set"];
        schema(
            root,
            "photara.package.root-set",
            2,
            &[
                "library_id",
                "bootstrap_sha256",
                "kind",
                "active",
                "recovery",
                "pinned_roots",
                "operation_index",
                "conversion_source",
                "retention_evidence",
                "inventory",
                "placement",
            ],
        )?;
        ensure(
            root["kind"] == "sealed"
                && root["library_id"] == LIBRARY
                && root["bootstrap_sha256"] == self.commit["bootstrap_sha256"],
            "RootSet identity",
        )?;
        fields(
            &root["placement"],
            &["generation", "root_placements", "accounting"],
        )?;
        ensure(
            number(&root["placement"]["generation"])? > 0,
            "placement generation",
        )?;
        Ok(root.clone())
    }
}
/// Parse every frame boundary before accepting a direct ref; padding is owned bytes, never an object.
fn frames(a: &Allocation) -> Result<BTreeMap<(u64, u64), u8>> {
    let mut at = 0usize;
    let mut out = BTreeMap::new();
    while at < a.bytes.len() {
        let h = a.bytes.get(at..at + 16).ok_or("truncated frame")?;
        ensure(&h[..8] == b"PS2PKD01" && h[9..12] == [0; 3], "frame header")?;
        let tag = h[8];
        ensure(
            tag <= 3
                && (tag != 3 || a.arena == "metadata")
                && (tag == 0 || tag == 3 || a.arena == "data"),
            "frame arena/tag",
        )?;
        let len = u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize;
        let end = at
            .checked_add(16)
            .and_then(|n| n.checked_add(len))
            .ok_or("frame overflow")?;
        let body = a.bytes.get(at + 16..end).ok_or("truncated frame body")?;
        if tag == 0 {
            ensure(body.iter().all(|b| *b == 0), "zero padding")?;
        } else {
            ensure(len > 0 && len <= 1024 * 1024, "frame JSON bound")?;
        }
        out.insert((at as u64, (end - at) as u64), tag);
        at = end;
    }
    Ok(out)
}
fn physical(w: &World, r: &Value, tag: u8, used: &mut BTreeSet<String>) -> Result<Value> {
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
    let id = uuid(&r["allocation_id"])?;
    let a = w.allocations.get(id).ok_or("missing physical allocation")?;
    ensure(r["arena"] == a.arena, "physical arena")?;
    let start = number(&r["offset"])?;
    let len = number(&r["byte_length"])?;
    ensure(
        frames(a)?.get(&(start, len)) == Some(&tag) && tag > 0,
        "physical exact frame/tag",
    )?;
    let start = usize::try_from(start).map_err(|_| "offset")?;
    let len = usize::try_from(len).map_err(|_| "length")?;
    let frame = a
        .bytes
        .get(start..start.checked_add(len).ok_or("range overflow")?)
        .ok_or("physical extent")?;
    ensure(
        hash(frame) == digest(&r["record_sha256"])?,
        "physical frame hash",
    )?;
    used.insert(id.into());
    parse(&frame[16..])
}
fn physical_key(r: &Value) -> String {
    hash(&encode(r))
}
fn physical_tree(
    w: &World,
    r: &Value,
    kind: &str,
    used: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    depth: usize,
) -> Result<Vec<Value>> {
    ensure(
        depth < 16 && seen.len() < 256 && seen.insert(physical_key(r)),
        "physical tree alias/cycle/bound",
    )?;
    let v = physical(w, r, 3, used)?;
    let leaf = v["schema"]["id"] == format!("photara.storage.{kind}-leaf");
    schema(
        &v,
        &format!(
            "photara.storage.{kind}-{}",
            if leaf { "leaf" } else { "branch" }
        ),
        1,
        &["count", if leaf { "entries" } else { "children" }],
    )?;
    let mut entries = vec![];
    if leaf {
        entries.clone_from(array(&v["entries"])?);
        ensure(entries.len() <= 64, "physical leaf bound")?;
    } else {
        let cs = array(&v["children"])?;
        ensure((2..=8).contains(&cs.len()), "physical branch fanout")?;
        for c in cs {
            fields(c, &["first", "last", "count", "child"])?;
            let es = physical_tree(w, &c["child"], kind, used, seen, depth + 1)?;
            let field = if kind == "locator" { "object" } else { "root" };
            ensure(
                !es.is_empty()
                    && es[0][field] == c["first"]
                    && es[es.len() - 1][field] == c["last"]
                    && es.len() as u64 == number(&c["count"])?,
                "physical child range/count",
            )?;
            entries.extend(es);
        }
    }
    ensure(
        entries.len() <= 4096 && entries.len() as u64 == number(&v["count"])?,
        "physical exact count",
    )?;
    let field = if kind == "locator" { "object" } else { "root" };
    let mut prior = None;
    for e in &entries {
        let k = key(&e[field])?;
        ensure(
            prior.as_ref().is_none_or(|p| p < &k),
            "physical sorted unique",
        )?;
        prior = Some(k);
    }
    Ok(entries)
}
#[derive(Clone)]
pub(super) struct Resolver {
    pub objects: BTreeMap<Key, (Value, u8)>,
    pub used: BTreeSet<String>,
}
impl Resolver {
    fn new(w: &World, r: &Value) -> Result<Self> {
        let mut used = BTreeSet::new();
        let es = physical_tree(w, r, "locator", &mut used, &mut BTreeSet::new(), 0)?;
        let mut objects = BTreeMap::new();
        for e in es {
            fields(&e, &["object", "membership", "physical"])?;
            let tag = match text(&e["membership"])? {
                "semantic" => 1,
                "ownership" => 2,
                _ => return Err("locator membership"),
            };
            let v = physical(w, &e["physical"], tag, &mut used)?;
            let k = key(&e["object"])?;
            ensure(reference(&v) == e["object"], "located canonical object")?;
            ensure(
                objects.insert(k, (v, tag)).is_none(),
                "duplicate locator key",
            )?;
        }
        Ok(Self { objects, used })
    }
    pub(super) fn get(&self, r: &Value, tag: u8) -> Result<Value> {
        let (v, t) = self.objects.get(&key(r)?).ok_or("missing located object")?;
        ensure(*t == tag, "typed membership")?;
        Ok(v.clone())
    }
    pub(super) fn any(&self, r: &Value) -> Result<Value> {
        let (v, t) = self
            .objects
            .get(&key(r)?)
            .ok_or("missing located accounting")?;
        let expected = if v["schema"]["id"].as_str().is_some_and(|s| {
            s.starts_with("photara.storage.")
                || s.starts_with("photara.package.retained-file-charge")
        }) {
            2
        } else {
            1
        };
        ensure(*t == expected, "accounting typed membership")?;
        Ok(v.clone())
    }
}
#[derive(Clone, Copy)]
enum Kind {
    Inventory,
    Id,
    Ordinal,
    Ownership,
    Pin,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Inventory => "photara.package.inventory",
            Self::Id => "photara.package.operation-id",
            Self::Ordinal => "photara.package.operation-ordinal",
            Self::Ownership => "photara.storage.ownership",
            Self::Pin => "photara.package.retained-root",
        }
    }
    fn entry_key(self, v: &Value) -> Result<String> {
        match self {
            Self::Inventory => {
                let (s, n) = key(v)?;
                Ok(format!("{s}:{n:020}"))
            }
            Self::Id => Ok(uuid(&v["operation_id"])?.into()),
            Self::Ordinal => Ok(format!("{:020}", number(&v["acceptance_ordinal"])?)),
            Self::Ownership => Ok(uuid(&v["allocation_id"])?.into()),
            Self::Pin => Ok(uuid(&v["pin_id"])?.into()),
        }
    }
    fn summary_key(self, v: &Value) -> Result<String> {
        match self {
            Self::Inventory => self.entry_key(v),
            Self::Ordinal => Ok(format!("{:020}", number(v)?)),
            _ => Ok(uuid(v)?.into()),
        }
    }
}
fn tree(
    res: &Resolver,
    r: &Value,
    kind: Kind,
    nodes: &mut BTreeSet<Key>,
    path: &mut BTreeSet<Key>,
    depth: usize,
) -> Result<Vec<Value>> {
    let k = key(r)?;
    ensure(
        depth < 16 && path.len() < 512 && path.insert(k.clone()),
        "typed tree alias/cycle/bound",
    )?;
    nodes.insert(k);
    let tag = if matches!(kind, Kind::Ownership) {
        2
    } else {
        1
    };
    let v = res.get(r, tag)?;
    let leaf = v["schema"]["id"] == format!("{}-leaf", kind.name());
    schema(
        &v,
        &format!("{}-{}", kind.name(), if leaf { "leaf" } else { "branch" }),
        1,
        &["count", if leaf { "entries" } else { "children" }],
    )?;
    let mut entries = vec![];
    if leaf {
        entries.clone_from(array(&v["entries"])?);
        ensure(entries.len() <= 4, "typed leaf bound")?;
    } else {
        let children = array(&v["children"])?;
        ensure((2..=4).contains(&children.len()), "typed branch fanout")?;
        for c in children {
            fields(c, &["first", "last", "count", "child"])?;
            let es = tree(res, &c["child"], kind, nodes, path, depth + 1)?;
            ensure(
                !es.is_empty()
                    && kind.entry_key(&es[0])? == kind.summary_key(&c["first"])?
                    && kind.entry_key(&es[es.len() - 1])? == kind.summary_key(&c["last"])?
                    && es.len() as u64 == number(&c["count"])?,
                "typed exact child range/count",
            )?;
            entries.extend(es);
        }
    }
    ensure(
        entries.len() <= 4096 && entries.len() as u64 == number(&v["count"])?,
        "typed exact aggregate",
    )?;
    let mut prev = None;
    for e in &entries {
        let k = kind.entry_key(e)?;
        ensure(prev.as_ref().is_none_or(|p| p < &k), "typed sorted unique")?;
        prev = Some(k);
    }
    Ok(entries)
}
fn walk(res: &Resolver, r: &Value, kind: Kind, nodes: &mut BTreeSet<Key>) -> Result<Vec<Value>> {
    tree(res, r, kind, nodes, &mut BTreeSet::new(), 0)
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
        1,
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
    number(&v["acceptance_ordinal"])?;
    number(&v["journal_sequence"])?;
    for c in [&v["before"], &v["after"]] {
        fields(c, &["revision", "digest"])?;
        number(&c["revision"])?;
        digest(&c["digest"])?;
    }
    provenance(&v["provenance"])
}

fn original_operations() -> &'static Value {
    static ORIGINAL: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    ORIGINAL.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/architecture/proposals/ps2/operations/linked-operations.json"
        ))
        .unwrap()
    })
}
fn original_conversion() -> Value {
    let c: Value = serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/resource-conversion/linked.json"
    ))
    .unwrap();
    let c = &c["scenarios"]["valid"];
    c["records"][text(&c["conversion"]["sha256"]).unwrap()]["input"].clone()
}
fn authored_closure(
    store: &Resolver,
    r: &Value,
    seen: &mut BTreeSet<Key>,
    transition: Option<&Transition<'_>>,
) -> Result<()> {
    let k = key(r)?;
    if !seen.insert(k.clone()) {
        return Ok(());
    }
    let v = store.get(r, 1)?;
    // Explicit preservation scope: these legacy record codecs are not reimplemented here.
    ensure(
        original_operations()["records"]
            .as_object()
            .unwrap()
            .values()
            .any(|row| row["input"] == v)
            || transition.is_some_and(|t| t.prepared_authored.get(&k) == Some(&v)),
        "unchanged supported original authored record",
    )?;
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
        authored_closure(store, &edge, seen, transition)?;
    }
    Ok(())
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep the bounded schema and exact closure proof in one auditable pass"
)]
fn operations(
    _w: &World,
    res: &Resolver,
    state: &Value,
    seen: &mut BTreeSet<Key>,
    transition: Option<&Transition<'_>>,
) -> Result<Vec<Value>> {
    let r = &state["operation_index"];
    seen.insert(key(r)?);
    let index = res.get(r, 1)?;
    schema(
        &index,
        "photara.package.operation-index",
        2,
        &[
            "library_id",
            "bootstrap_sha256",
            "accepted",
            "by_id",
            "by_ordinal",
        ],
    )?;
    ensure(
        index["library_id"] == LIBRARY
            && index["bootstrap_sha256"] == state["bootstrap_sha256"]
            && index["accepted"] == state["accepted"],
        "index identity/selected prefix",
    )?;
    fields(&index["accepted"], &["through_ordinal", "prefix_sha256"])?;
    let ids = walk(res, &index["by_id"], Kind::Id, seen)?;
    let ordinals = walk(res, &index["by_ordinal"], Kind::Ordinal, seen)?;
    let count = number(&index["accepted"]["through_ordinal"])?;
    ensure(
        count > 0 && count <= 32 && ids.len() as u64 == count && ordinals.len() as u64 == count,
        "operation counts",
    )?;
    let mapped = ids
        .iter()
        .map(|e| Ok((uuid(&e["operation_id"])?, e)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut prefix = hash(&encode(
        &json!({"domain":"photara.package.accepted-prefix.v1","project_id":PROJECT,"library_id":LIBRARY,"bootstrap_sha256":state["bootstrap_sha256"],"through_ordinal":"0"}),
    ));
    let mut records: Vec<Value> = vec![];
    let mut sequence = 0;
    for (i, e) in ordinals.iter().enumerate() {
        fields(
            e,
            &[
                "operation_id",
                "request_sha256",
                "acceptance_ordinal",
                "receipt",
            ],
        )?;
        ensure(
            mapped.get(uuid(&e["operation_id"])?) == Some(&e)
                && number(&e["acceptance_ordinal"])? == i as u64 + 1,
            "reciprocal contiguous operation indexes",
        )?;
        let r = res.get(&e["receipt"], 1)?;
        seen.insert(key(&e["receipt"])?);
        receipt(&r, text(&state["bootstrap_sha256"])?)?;
        ensure(
            ["operation_id", "request_sha256", "acceptance_ordinal"]
                .iter()
                .all(|k| r[k] == e[k])
                && number(&r["journal_sequence"])? > sequence,
            "original indexed receipt",
        )?;
        sequence = number(&r["journal_sequence"])?;
        if let Some(p) = records.last() {
            ensure(p["after"] == r["before"], "receipt authored continuity")?;
        }
        prefix = hash(&encode(
            &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":prefix,"acceptance_ordinal":r["acceptance_ordinal"],"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"receipt_sha256":reference(&r)["sha256"]}),
        ));
        records.push(r);
    }
    ensure(
        index["accepted"]["prefix_sha256"] == prefix,
        "accepted prefix recomputed",
    )?;
    authored_closure(res, &state["authored"], seen, transition)?;
    authored_closure(res, &state["history"], seen, transition)?;
    let last = records.last().ok_or("receipt")?;
    let authored = res.get(&state["authored"], 1)?;
    ensure(
        last["after"]["digest"] == state["authored"]["sha256"]
            && last["after"]["revision"] == state["authored_revision"]
            && authored["authored_revision"] == state["authored_revision"],
        "selected authored result",
    )?;
    let j = &state["journal_inclusion"];
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
    for r in &records {
        let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":r["journal_id"],"sequence":r["journal_sequence"],"operation_id":r["operation_id"],"request_sha256":r["request_sha256"],"receipt_sha256":reference(r)["sha256"]});
        ensure(
            frame["journal_id"] == j["journal_id"],
            "journal original receipt identity",
        )?;
        prefix = hash(&encode(
            &json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":prefix,"frame_sha256":hash(&encode(&frame))}),
        ));
        included += 1;
    }
    ensure(
        included == records.len()
            && j["through_sequence"] == last["journal_sequence"]
            && j["prefix_sha256"] == prefix
            && j["resulting_authored_revision"] == last["after"]["revision"]
            && j["resulting_authored_sha256"] == last["after"]["digest"],
        "journal prefix/result",
    )?;
    Ok(records)
}
#[derive(Clone)]
pub(super) struct RoleProof {
    pub state: Value,
    pub semantic: BTreeSet<Key>,
    pub inventory_nodes: BTreeSet<Key>,
    pub receipts: Vec<Value>,
    pub supported: bool,
    pub associations: BTreeMap<String, Value>,
}
pub(super) struct Contribution {
    pub logical: BTreeSet<Key>,
    pub implementation: BTreeSet<Key>,
}
impl Contribution {
    fn all(&self) -> BTreeSet<Key> {
        self.logical.union(&self.implementation).cloned().collect()
    }
}
pub(super) struct EvidenceContext<'a> {
    pub root: &'a Value,
    pub pins: &'a [Value],
    pub placements: &'a BTreeMap<Key, Value>,
    pub states: &'a BTreeMap<Key, Value>,
    pub roles: &'a BTreeMap<String, RoleProof>,
    pub recovery_only: bool,
}
pub(super) trait ResourcePolicy {
    fn extra_features(&self) -> &'static [&'static str] {
        &[]
    }
    fn resources(&self, state: &Value, res: &Resolver) -> Result<resources::Proof> {
        resources::verify(&state["resource_state"], text(&state["root_id"])?, |r| {
            res.get(r, 1)
        })
    }
    fn evidence(&self, ctx: &EvidenceContext<'_>, res: &Resolver) -> Result<Contribution> {
        let retention = res.get(&ctx.root["retention_evidence"], 1)?;
        schema(
            &retention,
            "photara.resource.retention-evidence-leaf",
            1,
            &["count", "entries"],
        )?;
        ensure(
            retention["count"] == "0" && array(&retention["entries"])?.is_empty(),
            "empty portable evidence supported subset",
        )?;
        Ok(Contribution {
            logical: BTreeSet::from([key(&ctx.root["retention_evidence"])?]),
            implementation: BTreeSet::new(),
        })
    }
}
struct AuthoredOnly;
impl ResourcePolicy for AuthoredOnly {}
fn role(
    policy: &impl ResourcePolicy,
    w: &World,
    res: &Resolver,
    r: &Value,
    entry: &Value,
    transition: Option<&Transition<'_>>,
) -> Result<RoleProof> {
    let state = res.get(r, 1)?;
    schema(
        &state,
        "photara.package.state-root",
        2,
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
    ensure(
        state["library_id"] == LIBRARY
            && state["bootstrap_sha256"] == w.commit["bootstrap_sha256"]
            && state["root_id"] == entry["root_id"],
        "StateRoot identity/provenance",
    )?;
    if let Some(t) = transition {
        if !t.verified.roles.values().any(|p| p.state == state) {
            let old_active = &t.verified.roles["active"].state;
            ensure(
                *r == w.commit["root_set"]["active"]
                    && state["predecessor"]
                        == json!({"root_sha256":reference(old_active)["sha256"],"authored_revision":old_active["authored_revision"]})
                    && state["root_id"] != old_active["root_id"],
                "exact old active StateRoot predecessor",
            )?;
        }
    } else {
        ensure(
            state["predecessor"].is_null(),
            "StateRoot identity/provenance",
        )?;
    }
    uuid(&state["root_id"])?;
    ensure(
        res.get(&state["authored"], 1)?["schema"]
            == json!({"id":"photara.project.authored","version":2})
            && res.get(&state["history"], 1)?["schema"]
                == json!({"id":"photara.project.history","version":2}),
        "StateRoot authored/history typed edges",
    )?;
    let mut semantic = BTreeSet::new();
    let receipts = operations(w, res, &state, &mut semantic, transition)?;
    let resources = policy.resources(&state, res)?;
    semantic.extend(resources.closure);
    let mut inventory_nodes = BTreeSet::new();
    let expected = walk(
        res,
        &state["inventory"],
        Kind::Inventory,
        &mut inventory_nodes,
    )?
    .iter()
    .map(key)
    .collect::<Result<BTreeSet<_>>>()?;
    ensure(expected == semantic, "exact StateRoot semantic inventory")?;
    Ok(RoleProof {
        state,
        semantic,
        inventory_nodes,
        receipts,
        supported: resources.supported,
        associations: resources.associations,
    })
}
fn ownership(
    w: &World,
    res: &Resolver,
    entry: &Value,
    shared: &BTreeSet<String>,
    accounting: &accounting::Evidence,
    selected: &accounting::Proof,
) -> Result<BTreeSet<Key>> {
    let mut nodes = BTreeSet::new();
    let entries = walk(res, &entry["ownership"], Kind::Ownership, &mut nodes)?;
    let mut claimed = BTreeSet::new();
    for e in entries {
        fields(&e, &["allocation_id", "claim"])?;
        let v = res.get(&e["claim"], 2)?;
        nodes.insert(key(&e["claim"])?);
        schema(
            &v,
            "photara.storage.allocation-claim",
            1,
            &[
                "allocation_id",
                "arena",
                "layout",
                "owned_extent",
                "authenticated_prefix",
                "sealed",
                "sealed_charge",
            ],
        )?;
        let id = uuid(&v["allocation_id"])?;
        ensure(
            v["allocation_id"] == e["allocation_id"] && claimed.insert(id.to_owned()),
            "unique ownership key",
        )?;
        let actual = w.allocations.get(id).ok_or("missing claimed allocation")?;
        let registered = accounting
            .allocations
            .get(id)
            .ok_or("unaccounted ownership")?;
        let observation = selected
            .observations
            .get(id)
            .ok_or("selected allocation observation")?;
        ensure(
            observation["physical"] == actual.witness
                && number(&observation["measured_extent"])? == actual.bytes.len() as u64
                && number(&observation["charged_high_water"])? == registered.registered_charge,
            "supplied owned allocation original witness",
        )?;

        ensure(
            v["layout"] == "framed-json"
                && v["arena"] == actual.arena
                && number(&v["owned_extent"])? == actual.bytes.len() as u64
                && registered.extent == actual.bytes.len() as u64,
            "ownership full extent/layout",
        )?;
        fields(&v["authenticated_prefix"], &["byte_length", "sha256"])?;
        let end = usize::try_from(number(&v["authenticated_prefix"]["byte_length"])?)
            .map_err(|_| "prefix length")?;
        let prefix = actual.bytes.get(..end).ok_or("prefix range")?;
        ensure(
            hash(prefix) == digest(&v["authenticated_prefix"]["sha256"])?,
            "ownership strong prefix",
        )?;
        if v["sealed"] == true {
            ensure(
                selected.sealed_refs.get(id) == Some(&key(&v["sealed_charge"])?),
                "exact selected sealed charge reference",
            )?;
            ensure(end == actual.bytes.len(), "sealed complete prefix")?;
            let charge = res.get(&v["sealed_charge"], 2)?;
            ensure(
                charge["allocation_id"] == id
                    && number(&charge["measured_extent"])? == registered.extent
                    && number(&charge["charged_high_water"])? == registered.registered_charge,
                "ownership sealed charge",
            )?;
        } else {
            ensure(
                selected.tip_ids.contains(id)
                    && v["sealed"] == false
                    && v["sealed_charge"].is_null()
                    && end == 0,
                "selected growable claim classification",
            )?;
        }
    }
    let expected = res.used.union(shared).cloned().collect::<BTreeSet<_>>();
    ensure(claimed == expected, "exact per-root owned allocations")?;
    Ok(nodes)
}
fn evidence(w: &World, _conversion: &Value) -> Result<accounting::Evidence> {
    let allocations = w
        .allocations
        .iter()
        .map(|(id, a)| {
            Ok((
                id.clone(),
                accounting::AllocationEvidence {
                    arena: a.arena.clone(),
                    subject_kind: "pack".into(),
                    extent: a.bytes.len() as u64,
                    registered_charge: rounded(a.bytes.len() as u64)?,
                    device: number(&a.witness["device"])?,
                    inode: number(&a.witness["inode"])?,
                },
            ))
        })
        .collect::<Result<_>>()?;
    let retained_files = w
        .source
        .iter()
        .map(|(p, b)| {
            Ok((
                p.split('/').map(str::to_owned).collect(),
                accounting::FileEvidence {
                    bytes: b.clone(),
                    registered_charge: rounded(b.len() as u64)?,
                    device: number(&w.source_witnesses[p]["device"])?,
                    inode: number(&w.source_witnesses[p]["inode"])?,
                },
            ))
        })
        .collect::<Result<_>>()?;
    Ok(accounting::Evidence {
        project_id: PROJECT.into(),
        profile: PROFILE.into(),
        incarnation: INCARNATION.into(),
        allocations,
        retained_files,
        conversion: Some(encode(&original_conversion())),
        standing_control: 131_072,
        directory_allowance: 16384,
        retained_directory_allowance: 16384,
    })
}
#[derive(Clone)]
pub(super) struct Proof {
    pub total: u64,
    pub roles: BTreeMap<String, RoleProof>,
    pub global: BTreeSet<Key>,
    pub controls: u64,
}
fn selected_controls(w: &World, root: &Value) -> Result<(Value, Value, Value, u64)> {
    selected_operation_controls(w, root, None)
}
fn selected_operation_controls(
    w: &World,
    root: &Value,
    selection: Option<&OperationSelection>,
) -> Result<(Value, Value, Value, u64)> {
    let envelope = w.loose(&root["placement"]["accounting"])?;
    schema(
        &envelope,
        "photara.storage.accounting-envelope",
        1,
        &["ledger", "holds"],
    )?;
    let ledger = w.loose(&envelope["ledger"])?;
    ensure(
        ledger["project_id"] == root["project_id"]
            && ledger["conversion_source"] == root["conversion_source"]
            && ledger["profile"] == PROFILE
            && ledger["incarnation"] == INCARNATION,
        "selected ledger root/scope identity",
    )?;
    let overlay = w.loose(&root["inventory"])?;
    schema(
        &overlay,
        "photara.package.inventory-overlay",
        1,
        &["base", "controls", "count"],
    )?;
    let mut controls = vec![
        key(&root["placement"]["accounting"])?,
        key(&envelope["ledger"])?,
    ];
    if let Some(selected) = selection {
        controls = selected.controls.iter().map(key).collect::<Result<_>>()?;
        ensure(
            controls.contains(&key(&root["placement"]["accounting"])?)
                && controls.contains(&key(&envelope["ledger"])?)
                && controls.contains(&key(&envelope["holds"])?),
            "operation mandatory selected controls",
        )?;
    }
    controls.sort();
    ensure(
        controls.windows(2).all(|p| p[0] < p[1]),
        "unique selected controls",
    )?;
    ensure(
        array(&overlay["controls"])?
            .iter()
            .map(key)
            .collect::<Result<Vec<_>>>()?
            == controls,
        "exact overlay controls",
    )?;
    let expected = controls
        .iter()
        .map(|k| k.0.clone())
        .chain([key(&root["inventory"])?.0])
        .collect::<BTreeSet<_>>();
    ensure(
        w.loose.keys().cloned().collect::<BTreeSet<_>>() == expected,
        "exact loose control keep-set",
    )?;
    let mut charge = 0u64;
    for b in w.loose.values() {
        parse(b)?;
        charge = charge
            .checked_add(rounded(b.len() as u64)?)
            .ok_or("control charge overflow")?;
    }
    for v in [&w.manifest, &w.head, &w.commit] {
        charge = charge
            .checked_add(rounded(encode(v).len() as u64)?)
            .ok_or("control charge overflow")?;
    }
    ensure(
        charge <= 131_072 && number(&ledger["standing_control"])? == 131_072,
        "selected standing control pool",
    )?;
    Ok((envelope, ledger, overlay, charge))
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep the bounded schema and exact closure proof in one auditable pass"
)]
pub(super) fn verify(w: &World) -> Result<Proof> {
    verify_with(w, &AuthoredOnly)
}
#[allow(
    clippy::too_many_lines,
    reason = "Shared full selected closure proof with explicit resource policy"
)]
pub(super) fn verify_with(w: &World, policy: &impl ResourcePolicy) -> Result<Proof> {
    verify_inner(w, policy, None)
}
/// Test-only prepared transition. The original package is independently verified here;
/// the caller supplies only canonical authored bytes produced by its actual Core/PS1 oracle.
pub(super) fn verify_transition(
    w: &World,
    old: &World,
    prepared_authored: &BTreeMap<Key, Value>,
) -> Result<Proof> {
    let t = Transition {
        old,
        verified: verify(old)?,
        prepared_authored,
        selection: None,
    };
    verify_inner(w, &AuthoredOnly, Some(&t))
}
/// Trusted compiled route proof, derived from parsed original bytes and its finite stage chain.
/// It does not replace actual physical, ledger, locator or global-union validation.
pub(super) struct OperationSelection {
    pub expected_commit: Value,
    pub controls: Vec<Value>,
    pub packed_hold: Value,
}
pub(super) fn verify_selected_operation(
    w: &World,
    old: &World,
    prepared_authored: &BTreeMap<Key, Value>,
    selection: &OperationSelection,
) -> Result<Proof> {
    let original_envelope = old.loose(&old.commit["root_set"]["placement"]["accounting"])?;
    ensure(
        selection.packed_hold == original_envelope["holds"],
        "selected operation original packed hold",
    )?;
    // The route must first prove exact O/P/F identity, finite phase fields and R/C.
    let t = Transition {
        old,
        verified: verify(old)?,
        prepared_authored,
        selection: Some(selection),
    };
    verify_inner(w, &AuthoredOnly, Some(&t))
}
struct Transition<'a> {
    old: &'a World,
    verified: Proof,
    prepared_authored: &'a BTreeMap<Key, Value>,
    selection: Option<&'a OperationSelection>,
}
#[allow(
    clippy::too_many_lines,
    reason = "One exact shared selected closure traversal"
)]
fn verify_inner(
    w: &World,
    policy: &impl ResourcePolicy,
    transition: Option<&Transition<'_>>,
) -> Result<Proof> {
    let root = w.bootstrap_transition(policy.extra_features(), transition)?;
    let operation = transition.and_then(|t| t.selection);
    let (envelope, ledger, overlay, controls) = selected_operation_controls(w, &root, operation)?;
    let mut shared = BTreeSet::new();
    let placements = physical_tree(
        w,
        &root["placement"]["root_placements"],
        "root-placement",
        &mut shared,
        &mut BTreeSet::new(),
        0,
    )?;
    let mut byroot = BTreeMap::new();
    let mut byid = BTreeMap::new();
    for p in placements {
        fields(&p, &["root", "root_id", "locator", "ownership"])?;
        let id = uuid(&p["root_id"])?;
        if let Some(prior) = byid.insert(id.to_owned(), key(&p["root"])?) {
            ensure(prior == key(&p["root"])?, "unique retained root UUID")?;
        }
        byroot.insert(key(&p["root"])?, p);
    }
    let active = byroot
        .get(&key(&root["active"])?)
        .ok_or("active placement")?;
    let active_res = Resolver::new(w, &active["locator"])?;
    let mut pin_nodes = BTreeSet::new();
    let pins = walk(
        &active_res,
        &root["pinned_roots"],
        Kind::Pin,
        &mut pin_nodes,
    )?;
    ensure(!pins.is_empty(), "nonempty retained pin specimen")?;
    let mut selected = vec![
        ("active".to_owned(), root["active"].clone()),
        ("recovery".to_owned(), root["recovery"].clone()),
    ];
    for p in &pins {
        fields(p, &["pin_id", "reason", "root"])?;
        ensure(
            ["explicit-history", "unresolved-recovery", "undo"].contains(&text(&p["reason"])?),
            "existing retained pin reason",
        )?;
        selected.push((format!("pin:{}", uuid(&p["pin_id"])?), p["root"].clone()));
    }
    ensure(
        byroot.keys().cloned().collect::<BTreeSet<_>>()
            == selected
                .iter()
                .map(|(_, r)| key(r))
                .collect::<Result<BTreeSet<_>>>()?,
        "exact selected root placements",
    )?;
    let conversion = active_res.get(&root["conversion_source"], 1)?;
    ensure(
        conversion["source_bootstrap_sha256"] == w.commit["bootstrap_sha256"]
            && w.source.get("manifest.json") == Some(&encode(&w.manifest)),
        "conversion exact original bootstrap",
    )?;
    let ev = evidence(w, &conversion)?;
    let hold_ref = operation.map_or(&envelope["holds"], |s| &s.packed_hold);
    let holds = active_res.get(hold_ref, 2)?;
    let account = accounting::verify(&ledger, &holds, |r| active_res.any(r), &ev)?;
    ensure(
        account.allocations == w.allocations.keys().cloned().collect(),
        "exact all physical allocation charges",
    )?;
    let mut prepared = BTreeMap::new();
    let mut selected_states = BTreeMap::new();
    let mut selected_roles = BTreeMap::new();
    for (label, r) in &selected {
        let entry = byroot.get(&key(r)?).ok_or("selected placement")?;
        let res = Resolver::new(w, &entry["locator"])?;
        let proof = role(policy, w, &res, r, entry, transition)?;
        selected_states.insert(key(r)?, proof.state.clone());
        selected_roles.insert(label.clone(), proof.clone());
        prepared.insert(label.clone(), (res, proof));
    }
    let extra = policy
        .evidence(
            &EvidenceContext {
                root: &root,
                pins: &pins,
                placements: &byroot,
                states: &selected_states,
                roles: &selected_roles,
                recovery_only: false,
            },
            &active_res,
        )?
        .all();
    for k in &extra {
        active_res.get(
            &json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()}),
            1,
        )?;
    }
    let mut global = account.logical.clone();
    if let Some(selected) = operation {
        for r in &selected.controls {
            w.loose(r)?;
            global.insert(key(r)?);
        }
    }
    global.extend(account.implementation.clone());
    global.extend(pin_nodes.clone());
    global.extend(extra.clone());
    global.insert(key(&root["placement"]["accounting"])?);
    let mut global_nodes = BTreeSet::new();
    let base = walk(
        &active_res,
        &overlay["base"],
        Kind::Inventory,
        &mut global_nodes,
    )?
    .iter()
    .map(key)
    .collect::<Result<BTreeSet<_>>>()?;
    let mut roles = BTreeMap::new();
    let mut associations = BTreeMap::new();
    let mut used = shared.clone();
    for (label, r) in selected {
        let entry = byroot.get(&key(&r)?).ok_or("selected placement")?;
        let (res, proof) = prepared.remove(&label).ok_or("prepared selected role")?;
        if label == "active" {
            ensure(
                ["authored", "history", "inventory"]
                    .iter()
                    .all(|k| w.commit[k] == proof.state[k])
                    && root["operation_index"] == proof.state["operation_index"],
                "outer active fields",
            )?;
        }
        for (id, v) in &proof.associations {
            if let Some(old) = associations.insert(id.clone(), v.clone()) {
                ensure(old == *v, "immutable association identity across roots")?;
            }
        }
        global.extend(proof.semantic.clone());
        global.insert(key(&r)?);
        let owner = ownership(w, &res, entry, &shared, &ev, &account)?;
        let mut expected = proof.semantic.clone();
        expected.extend(proof.inventory_nodes.clone());
        expected.insert(key(&r)?);
        expected.extend(
            account
                .logical
                .iter()
                .filter(|k| **k != key(&envelope["ledger"]).unwrap())
                .cloned(),
        );
        expected.extend(account.implementation.clone());
        expected.extend(pin_nodes.clone());
        expected.extend(extra.clone());
        expected.extend(global_nodes.clone());
        expected.extend(owner);
        ensure(
            expected == res.objects.keys().cloned().collect(),
            "exact per-root locator logical closure",
        )?;
        // Every independently readable role carries the same authenticated global control dependencies.
        for k in account
            .logical
            .iter()
            .chain(&account.implementation)
            .chain(&pin_nodes)
            .chain(&global_nodes)
            .chain(&extra)
        {
            if *k != key(&envelope["ledger"])? {
                ensure(
                    res.objects.get(k) == active_res.objects.get(k),
                    "independent global dependency bytes",
                )?;
            }
        }
        used.extend(res.used);
        roles.insert(label, proof);
    }
    ensure(
        used == w.allocations.keys().cloned().collect(),
        "no unowned extra allocation",
    )?;
    let a = &roles["active"].receipts;
    for p in roles.values() {
        ensure(
            p.receipts.len() <= a.len() && a[..p.receipts.len()] == p.receipts,
            "shared original receipt prefix",
        )?;
    }
    let controlkeys = array(&overlay["controls"])?
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    ensure(base.is_disjoint(&controlkeys), "overlay disjoint controls")?;
    let union = base.union(&controlkeys).cloned().collect::<BTreeSet<_>>();
    ensure(
        union == global && union.len() as u64 == number(&overlay["count"])?,
        "exact global inventory union",
    )?;
    Ok(Proof {
        total: account.total,
        roles,
        global,
        controls,
    })
}

pub(super) fn receipt_for(proof: &Proof, id: &str, request: &str) -> Result<Value> {
    let receipt = proof.roles["active"]
        .receipts
        .iter()
        .find(|r| r["operation_id"] == id)
        .ok_or("unknown original operation")?;
    ensure(
        receipt["request_sha256"] == request,
        "same-ID request conflict",
    )?;
    Ok(receipt.clone())
}
/// Independent role readability only. It deliberately cannot authorize writes,
/// settlement or deletion while another selected root's allocations are absent.
#[allow(
    clippy::too_many_lines,
    reason = "Keep the bounded schema and exact closure proof in one auditable pass"
)]
pub(super) fn recovery(w: &World) -> Result<RoleProof> {
    recovery_with(w, &AuthoredOnly)
}
#[allow(
    clippy::too_many_lines,
    reason = "Shared independent recovery closure with explicit policy limits"
)]
pub(super) fn recovery_with(w: &World, policy: &impl ResourcePolicy) -> Result<RoleProof> {
    recovery_inner(w, policy, None)
}
/// Read-only supplied recovery closure. The route must validate selected O/P/F metadata first.
pub(super) fn recovery_operation(w: &World, selection: &OperationSelection) -> Result<RoleProof> {
    recovery_inner(w, &AuthoredOnly, Some(selection))
}
#[allow(
    clippy::too_many_lines,
    reason = "One exact independent supplied recovery closure proof"
)]
fn recovery_inner(
    w: &World,
    policy: &impl ResourcePolicy,
    operation: Option<&OperationSelection>,
) -> Result<RoleProof> {
    let root = w.bootstrap_context(policy.extra_features(), None, operation)?;
    let (envelope, ledger, overlay, _) = selected_operation_controls(w, &root, operation)?;
    let mut shared = BTreeSet::new();
    let placements = physical_tree(
        w,
        &root["placement"]["root_placements"],
        "root-placement",
        &mut shared,
        &mut BTreeSet::new(),
        0,
    )?;
    let mut byroot = BTreeMap::new();
    let mut ids = BTreeMap::new();
    for p in placements {
        fields(&p, &["root", "root_id", "locator", "ownership"])?;
        let id = uuid(&p["root_id"])?;
        if let Some(old) = ids.insert(id.to_owned(), key(&p["root"])?) {
            ensure(old == key(&p["root"])?, "unique retained root UUID")?;
        }
        byroot.insert(key(&p["root"])?, p);
    }
    let entry = byroot
        .get(&key(&root["recovery"])?)
        .ok_or("recovery placement")?;
    let res = Resolver::new(w, &entry["locator"])?;
    let proof = role(policy, w, &res, &root["recovery"], entry, None)?;
    let hold_ref = operation.map_or(&envelope["holds"], |s| &s.packed_hold);
    let holds = res.get(hold_ref, 2)?;
    let accounting = accounting::metadata(&ledger, &holds, |r| res.any(r))?;
    let conversion = res.get(&root["conversion_source"], 1)?;
    ensure(
        conversion == original_conversion()
            && ledger["conversion_source"] == root["conversion_source"]
            && conversion["source_bootstrap_sha256"] == root["bootstrap_sha256"],
        "recovery original conversion identity",
    )?;
    let ev = evidence(w, &conversion)?;
    let owner = ownership(w, &res, entry, &shared, &ev, &accounting)?;
    let mut pins = BTreeSet::new();
    let entries = walk(&res, &root["pinned_roots"], Kind::Pin, &mut pins)?;
    let mut selected = BTreeSet::from([key(&root["active"])?, key(&root["recovery"])?]);
    for p in &entries {
        fields(p, &["pin_id", "reason", "root"])?;
        ensure(
            ["explicit-history", "unresolved-recovery", "undo"].contains(&text(&p["reason"])?),
            "existing retained pin reason",
        )?;
        selected.insert(key(&p["root"])?);
    }
    ensure(
        selected == byroot.keys().cloned().collect(),
        "exact selected root placements",
    )?;
    let selected_states = BTreeMap::from([(key(&root["recovery"])?, proof.state.clone())]);
    let selected_roles = BTreeMap::from([("recovery".to_owned(), proof.clone())]);
    let extra = policy
        .evidence(
            &EvidenceContext {
                root: &root,
                pins: &entries,
                placements: &byroot,
                states: &selected_states,
                roles: &selected_roles,
                recovery_only: true,
            },
            &res,
        )?
        .all();
    for k in &extra {
        res.get(
            &json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()}),
            1,
        )?;
    }
    let mut global_nodes = BTreeSet::new();
    let base = walk(&res, &overlay["base"], Kind::Inventory, &mut global_nodes)?
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    let controls = array(&overlay["controls"])?
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    ensure(
        base.is_disjoint(&controls)
            && base.len() + controls.len()
                == usize::try_from(number(&overlay["count"])?).map_err(|_| "overlay count")?,
        "recovery overlay count/disjointness",
    )?;
    let mut required = accounting.logical.clone();
    if let Some(selected) = operation {
        for r in &selected.controls {
            w.loose(r)?;
            required.insert(key(r)?);
        }
    }
    required.extend(accounting.implementation.clone());
    required.extend(pins.clone());
    required.extend(proof.semantic.clone());
    required.extend(selected);
    required.extend(extra.clone());
    required.insert(key(&root["placement"]["accounting"])?);
    ensure(
        required.is_subset(&base.union(&controls).cloned().collect()),
        "recovery required global control/semantic members",
    )?;
    let mut exact = proof.semantic.clone();
    exact.extend(proof.inventory_nodes.clone());
    exact.insert(key(&root["recovery"])?);
    exact.extend(
        accounting
            .logical
            .iter()
            .filter(|k| **k != key(&envelope["ledger"]).unwrap())
            .cloned(),
    );
    exact.extend(accounting.implementation);
    exact.extend(pins);
    exact.extend(extra);
    exact.extend(global_nodes);
    exact.extend(owner);
    ensure(
        exact == res.objects.keys().cloned().collect(),
        "exact independent recovery locator closure",
    )?;
    Ok(proof)
}
