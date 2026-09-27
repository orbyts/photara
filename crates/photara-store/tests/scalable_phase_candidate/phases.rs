//! Unfrozen candidate phase/physical closure verifier. Pure memory only.
use super::{operations, packed};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
type Key = (String, u64);
type RoleProof = (BTreeSet<Key>, BTreeMap<String, Value>);
type OwnerProof = (BTreeSet<Key>, BTreeMap<String, Value>, BTreeSet<String>);
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const PROFILE: &str = "example.ps2.synthetic-local-profile-v1";
const INCARNATION: &str = "71000000-0000-4000-8000-000000000001";
const CODEC: &str = "example.ps2.phase-canonical-v1";
pub(super) fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
fn encode(v: &Value) -> Result<Vec<u8>> {
    photara_core::canonical_json(v).map_err(|_| "canonical JSON")
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|n| m.contains_key(*n)),
        "exact fields",
    )
}
fn schema(v: &Value, name: &str, names: &[&str]) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"]
            == if matches!(name, "root-set" | "state-root") {
                format!("example.ps2.{name}")
            } else {
                format!("example.ps2.phase-{name}")
            }
            && v["schema"]["version"] == 1
            && v["project_id"] == PROJECT
            && v["extensions"]
                .as_object()
                .is_some_and(|m| m.keys().all(|k| k.contains('.'))),
        "schema/project/required fields",
    )
}
pub(super) fn decimal(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("decimal string")?;
    let n = s.parse::<u64>().map_err(|_| "decimal range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or("string")
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("array")
}
fn key(r: &Value) -> Result<Key> {
    fields(r, &["kind", "sha256", "byte_length"])?;
    let h = string(&r["sha256"])?;
    ensure(
        r["kind"] == "json"
            && h.len() == 64
            && h.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "ObjectRef",
    )?;
    Ok((h.to_owned(), decimal(&r["byte_length"])?))
}
fn object_ref(v: &Value) -> Result<Value> {
    let b = encode(v)?;
    Ok(
        serde_json::json!({"kind":"json","sha256":packed::hash(&b),"byte_length":b.len().to_string()}),
    )
}
fn parse(b: &[u8]) -> Result<Value> {
    let v = serde_json::from_slice(b).map_err(|_| "JSON")?;
    ensure(encode(&v)? == b, "canonical exact bytes")?;
    Ok(v)
}
fn layout(bytes: &[u8], arena: &str) -> Result<Vec<(usize, usize, u8)>> {
    ensure(bytes.len() <= 1024 * 1024, "bounded allocation")?;
    let mut ranges = vec![];
    let mut at = 0usize;
    while at < bytes.len() {
        let h = bytes
            .get(at..at.checked_add(16).ok_or("header overflow")?)
            .ok_or("partial header")?;
        ensure(
            &h[..8] == b"PS2PKD01"
                && h[9..12] == [0, 0, 0]
                && h[8] <= 3
                && (h[8] == 0 || (h[8] == 3) == (arena == "metadata")),
            "frame structure/type",
        )?;
        let len = u32::from_le_bytes(h[12..16].try_into().map_err(|_| "frame length")?) as usize;
        let end = at
            .checked_add(16)
            .and_then(|n| n.checked_add(len))
            .ok_or("frame overflow")?;
        let body = bytes.get(at + 16..end).ok_or("partial frame")?;
        if h[8] == 0 {
            ensure(
                !body.is_empty() && body.iter().all(|b| *b == 0),
                "zero padding",
            )?;
        }
        ranges.push((at, end, h[8]));
        at = end;
    }
    Ok(ranges)
}
fn observed(bytes: usize) -> Result<u64> {
    (bytes as u64)
        .checked_add(4095)
        .map(|n| (n / 4096) * 4096)
        .ok_or("synthetic observation overflow")
}
pub(super) struct World<'a> {
    pub(super) corpus: &'a Value,
    pub(super) ops: &'a Value,
    objects: BTreeMap<Key, Vec<u8>>,
}
impl<'a> World<'a> {
    pub(super) fn new(corpus: &'a Value, ops: &'a Value, ops_bytes: &[u8]) -> Result<Self> {
        ensure(
            corpus["status"] == "unfrozen-linked-phase-candidate"
                && corpus["qualification"] == false
                && corpus["operation_corpus_sha256"] == packed::hash(ops_bytes),
            "candidate/dependency identity",
        )?;
        let mut objects = BTreeMap::new();
        for pool in [&corpus["records"], &ops["records"]] {
            for entry in pool.as_object().ok_or("record pool")?.values() {
                let b = string(&entry["canonical"])?.as_bytes().to_vec();
                let v = parse(&b)?;
                ensure(
                    encode(&entry["input"])? == b
                        && entry["sha256"] == packed::hash(&b)
                        && entry["byte_length"].as_u64() == Some(b.len() as u64),
                    "fixed input/bytes/hash/length",
                )?;
                let k = key(&object_ref(&v)?)?;
                if let Some(old) = objects.insert(k, b.clone()) {
                    ensure(old == b, "conflicting fixed object")?;
                }
                if let Some(hex) = entry["frame_hex"].as_str() {
                    let f = packed::unhex(hex)?;
                    let arena = if f.get(8) == Some(&3) {
                        "metadata"
                    } else {
                        "data"
                    };
                    ensure(
                        packed::frame_ranges(&f, arena)?.len() == 1
                            && f.get(16..) == Some(b.as_slice())
                            && entry["frame_sha256"] == packed::hash(&f)
                            && entry["frame_length"].as_u64() == Some(f.len() as u64),
                        "fixed candidate frame",
                    )?;
                }
            }
        }
        Ok(Self {
            corpus,
            ops,
            objects,
        })
    }
    pub(super) fn named(&self, name: &str) -> Result<Value> {
        parse(string(&self.corpus["records"][name]["canonical"])?.as_bytes())
    }
    fn object(&self, r: &Value) -> Result<Value> {
        parse(
            self.objects
                .get(&key(r)?)
                .ok_or("missing original object bytes")?,
        )
    }
    fn scope(&self, r: &Value) -> Result<()> {
        let v = self.object(r)?;
        schema(
            &v,
            "local-scope",
            &[
                "profile",
                "incarnation",
                "qualification",
                "codec",
                "project_binding_sha256",
            ],
        )?;
        ensure(
            v["profile"] == PROFILE
                && v["incarnation"] == INCARNATION
                && v["qualification"] == false
                && v["codec"] == CODEC
                && v["project_binding_sha256"] == self.ops["records"]["manifest"]["sha256"],
            "local profile/incarnation/codec",
        )
    }
    fn witness(w: &Value) -> Result<()> {
        fields(
            w,
            &["profile", "incarnation", "allocation_id", "device", "inode"],
        )?;
        ensure(
            w["profile"] == PROFILE && w["incarnation"] == INCARNATION,
            "witness scope",
        )?;
        let id = string(&w["allocation_id"])?;
        ensure(
            uuid::Uuid::parse_str(id).is_ok_and(|v| !v.is_nil() && v.to_string() == id),
            "allocation ID",
        )?;
        decimal(&w["device"])?;
        decimal(&w["inode"])?;
        Ok(())
    }
    fn registered_witness(&self, id: &str) -> Result<Value> {
        let a = self.named("admission")?;
        let w = self.named("witnessed")?;
        array(&a["source_witnesses"])?
            .iter()
            .chain(array(&w["births"])?)
            .find(|w| w["allocation_id"] == id)
            .cloned()
            .ok_or("unbound generation")
    }
    fn file(&self, s: &Value, id: &str) -> Result<(String, Vec<u8>)> {
        let entry = &s["files"][id];
        fields(entry, &["arena", "hex", "witness"])?;
        Self::witness(&entry["witness"])?;
        ensure(
            entry["witness"] == self.registered_witness(id)?,
            "same-byte replacement/profile substitution",
        )?;
        let arena = string(&entry["arena"])?.to_owned();
        let bytes = packed::unhex(string(&entry["hex"])?)?;
        layout(&bytes, &arena)?;
        Ok((arena, bytes))
    }
    fn physical(&self, s: &Value, p: &Value, tag: u8) -> Result<(Value, String)> {
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
        let id = string(&p["allocation_id"])?;
        let (arena, bytes) = self.file(s, id)?;
        ensure(p["arena"] == arena, "physical arena")?;
        let start = usize::try_from(decimal(&p["offset"])?).map_err(|_| "offset")?;
        let end = start
            .checked_add(usize::try_from(decimal(&p["byte_length"])?).map_err(|_| "length")?)
            .ok_or("range overflow")?;
        ensure(
            layout(&bytes, &arena)?.contains(&(start, end, tag)),
            "physical frame boundary/tag",
        )?;
        let f = bytes.get(start..end).ok_or("physical slice")?;
        ensure(
            p["record_sha256"] == packed::hash(f),
            "physical exact frame hash",
        )?;
        Ok((parse(&f[16..])?, id.into()))
    }
    fn owners(&self, s: &Value, p: &Value) -> Result<OwnerProof> {
        let (root, alloc) = self.physical(s, p, 2)?;
        schema(&root, "ownership-branch", &["children"])?;
        let mut keys = BTreeSet::from([key(&object_ref(&root)?)?]);
        let mut claims = BTreeMap::new();
        let mut used = BTreeSet::from([alloc]);
        let mut prior = None;
        for child in array(&root["children"])? {
            fields(child, &["object", "physical", "allocation_id"])?;
            let (leaf, alloc) = self.physical(s, &child["physical"], 2)?;
            used.insert(alloc);
            schema(&leaf, "ownership-leaf", &["claim"])?;
            ensure(
                object_ref(&leaf)? == child["object"],
                "ownership edge bytes",
            )?;
            ensure(
                keys.insert(key(&child["object"])?),
                "duplicate ownership record",
            )?;
            let c = &leaf["claim"];
            fields(
                c,
                &[
                    "allocation_id",
                    "arena",
                    "extent",
                    "prefix_end",
                    "prefix_sha256",
                    "sealed",
                    "registered_charge",
                ],
            )?;
            let id = string(&c["allocation_id"])?.to_owned();
            ensure(
                child["allocation_id"] == id && prior.as_ref().is_none_or(|p| p < &id),
                "ordered unique allocation claims",
            )?;
            prior = Some(id.clone());
            let (arena, bytes) = self.file(s, &id)?;
            let extent = decimal(&c["extent"])?;
            let prefix = usize::try_from(decimal(&c["prefix_end"])?).map_err(|_| "prefix")?;
            ensure(
                c["arena"] == arena
                    && extent == bytes.len() as u64
                    && prefix <= bytes.len()
                    && c["prefix_sha256"] == packed::hash(&bytes[..prefix]),
                "ownership extent/prefix integrity",
            )?;
            if c["sealed"] == true {
                ensure(
                    prefix == bytes.len()
                        && decimal(&c["registered_charge"])? == observed(bytes.len())?,
                    "sealed exact charge",
                )?;
            } else {
                ensure(
                    c["sealed"] == false && c["registered_charge"].is_null(),
                    "growable charge belongs to selected tip ledger",
                )?;
            }
            claims.insert(id, c.clone());
        }
        Ok((keys, claims, used))
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded bootstrap selection checks together"
    )]
    pub(super) fn bootstrap(&self, s: &Value, reader: u64) -> Result<Value> {
        fields(&s["bootstrap"], &["manifest", "head", "commit"])?;
        let manifest_bytes = string(&s["bootstrap"]["manifest"])?.as_bytes();
        let manifest = parse(manifest_bytes)?;
        ensure(
            manifest == self.ops["records"]["manifest"]["input"],
            "unchanged original manifest identity",
        )?;
        let head = parse(string(&s["bootstrap"]["head"])?.as_bytes())?;
        fields(
            &head,
            &["schema", "project_id", "commit_id", "commit_sha256"],
        )?;
        let commit_bytes = string(&s["bootstrap"]["commit"])?.as_bytes();
        let commit = parse(commit_bytes)?;
        let checked: photara_store::package::Commit = serde_json::from_value(commit.clone())
            .map_err(|_| "checked outer commit scalar types")?;
        ensure(
            !checked.commit_id.as_uuid().is_nil()
                && !checked.write_id.as_uuid().is_nil()
                && checked
                    .parent
                    .as_ref()
                    .is_none_or(|p| !p.commit_id.as_uuid().is_nil()),
            "nonnil outer identifiers",
        )?;
        ensure(
            commit["created_at"] == "2026-09-27T00:00:00.000Z",
            "fixed candidate timestamp",
        )?;
        ensure(
            head["schema"] == serde_json::json!({"id":"photara.package.head","version":1})
                && head["project_id"] == PROJECT
                && head["commit_sha256"] == packed::hash(commit_bytes),
            "HEAD selects exact outer commit",
        )?;
        fields(
            &commit,
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
            commit["schema"] == serde_json::json!({"id":"photara.package.commit","version":1})
                && commit["project_id"] == PROJECT
                && commit["bootstrap_sha256"] == packed::hash(manifest_bytes)
                && commit["commit_id"] == head["commit_id"]
                && commit["minimum_reader"] == serde_json::json!({"major":1,"minor":3})
                && reader >= 3,
            "outer commit identity/reader floor",
        )?;
        let mut features = array(&manifest["required_features"])?.clone();
        features.extend([
            Value::String("example.ps2.scalable-storage.draft-v1".into()),
            Value::String("photara.sealed-roots.v1".into()),
        ]);
        features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        ensure(
            commit["required_features"] == Value::Array(features),
            "required scalable capability dispatch",
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
            root["library_id"] == self.ops["records"]["receipt-1"]["input"]["library_id"]
                && root["bootstrap_sha256"] == packed::hash(manifest_bytes)
                && root["kind"] == "scalable-draft"
                && array(&root["pinned_roots"])?.is_empty()
                && root["conversion_source"].is_null()
                && object_ref(root)? == s["root"],
            "RootSet bootstrap and selected original identity",
        )?;
        Ok(root.clone())
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded candidate closure and phase invariants in one explicit audit"
    )]
    pub(super) fn role(&self, name: &str, s: &Value, role: &str) -> Result<RoleProof> {
        ensure(["active", "recovery"].contains(&role), "role")?;
        let operation_store = operations::Store::load(self.ops)?;
        let root = self.bootstrap(s, 3)?;
        let semantic_role = if name == "old" { "recovery" } else { role };
        let proof = operations::verify_role(&operation_store, semantic_role)?;
        let (owner_keys, claims, mut used) =
            self.owners(s, &root["placement"][format!("{role}_ownership")])?;
        let role_claims = claims.keys().cloned().collect::<BTreeSet<_>>();
        let (loc, alloc) = self.physical(s, &root["placement"][format!("{role}_locator")], 3)?;
        used.insert(alloc);
        schema(&loc, "locator-leaf", &["count", "entries"])?;
        let entries = array(&loc["entries"])?;
        ensure(
            decimal(&loc["count"])? == entries.len() as u64,
            "locator count",
        )?;
        let mut semantic = BTreeMap::new();
        let mut own = BTreeSet::new();
        let mut prior = None;
        for e in entries {
            fields(e, &["object", "membership", "physical"])?;
            let k = key(&e["object"])?;
            ensure(
                prior.as_ref().is_none_or(|p| p < &k),
                "ordered unique locator",
            )?;
            prior = Some(k.clone());
            let tag = match string(&e["membership"])? {
                "semantic" => 1,
                "ownership" => 2,
                _ => return Err("typed locator membership"),
            };
            let (value, alloc) = self.physical(s, &e["physical"], tag)?;
            used.insert(alloc);
            ensure(
                object_ref(&value)? == e["object"],
                "locator exact object bytes",
            )?;
            if tag == 1 {
                semantic.insert(k, value);
            } else {
                own.insert(k);
            }
        }
        ensure(own == owner_keys, "exact ownership locator closure")?;
        let state_key = key(&root[role])?;
        let state = semantic
            .get(&state_key)
            .ok_or("state not independently located")?;
        schema(
            state,
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
        ensure(
            state["library_id"] == self.ops["records"]["receipt-1"]["input"]["library_id"]
                && state["bootstrap_sha256"] == self.ops["records"]["manifest"]["sha256"]
                && state["resource_state"].is_null()
                && state["authored_revision"]
                    == self.ops["selected_roots"][semantic_role]["journal_inclusion"]["resulting_authored_revision"],
            "StateRoot identity/coordinate",
        )?;
        for field in [
            "authored",
            "history",
            "operation_index",
            "accepted",
            "journal_inclusion",
        ] {
            ensure(
                state[field] == self.ops["selected_roots"][semantic_role][field],
                "original operation coordinate/roots changed",
            )?;
        }
        let expected_predecessor = if state["authored_revision"] == "3" {
            let old = self.named("old-root")?;
            serde_json::json!({"root_sha256":old["active"]["sha256"],"authored_revision":"2"})
        } else {
            Value::Null
        };
        ensure(
            state["predecessor"] == expected_predecessor,
            "actual prior StateRoot provenance",
        )?;
        if role == "active" {
            let commit = parse(string(&s["bootstrap"]["commit"])?.as_bytes())?;
            ensure(
                commit["authored"] == state["authored"]
                    && commit["history"] == state["history"]
                    && commit["inventory"] == state["inventory"]
                    && root["operation_index"] == state["operation_index"],
                "unchanged outer commit equalities",
            )?;
        }
        let inv_key = key(&state["inventory"])?;
        let inv = semantic
            .get(&inv_key)
            .ok_or("inventory not independently located")?;
        schema(inv, "inventory-leaf", &["count", "entries"])?;
        let inventory: Vec<_> = array(&inv["entries"])?
            .iter()
            .map(key)
            .collect::<Result<_>>()?;
        ensure(
            decimal(&inv["count"])? == inventory.len() as u64
                && inventory.windows(2).all(|w| w[0] < w[1])
                && inventory.iter().cloned().collect::<BTreeSet<_>>() == proof.closure,
            "exact typed semantic inventory",
        )?;
        let mut expected = proof.closure;
        expected.insert(state_key);
        expected.insert(inv_key);
        ensure(
            semantic.keys().cloned().collect::<BTreeSet<_>>() == expected,
            "exact independent semantic locator closure",
        )?;
        for k in &expected {
            if let Some(original) = self.objects.get(k) {
                ensure(
                    encode(semantic.get(k).unwrap())? == *original,
                    "actual remapped semantic bytes changed",
                )?;
            }
        }
        ensure(
            used == role_claims,
            "exact physical ownership coverage including locator/owner pages",
        )?;
        Ok((expected, claims))
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded candidate closure and phase invariants in one explicit audit"
    )]
    pub(super) fn snapshot(&self, name: &str, s: &Value) -> Result<u64> {
        let root = self.bootstrap(s, 3)?;
        let mut all = BTreeSet::new();
        let mut all_claims = BTreeMap::new();
        for role in ["active", "recovery"] {
            let (logical, claims) = self.role(name, s, role)?;
            all.extend(logical);
            for (id, c) in claims {
                if let Some(old) = all_claims.insert(id, c.clone()) {
                    ensure(old == c, "inconsistent shared charge")?;
                }
            }
        }
        let accounting = self.object(&root["placement"]["accounting"])?;
        schema(
            &accounting,
            "accounting",
            &[
                "profile",
                "incarnation",
                "standing_control",
                "tips",
                "sealed",
                "tickets",
                "total_charge",
            ],
        )?;
        ensure(
            accounting["profile"] == PROFILE && accounting["incarnation"] == INCARNATION,
            "accounting scope",
        )?;
        ensure(
            accounting["standing_control"] == self.named("admission")?["standing_control"],
            "selected standing control pool",
        )?;
        let original_retirement = self.named("retirement-admission")?;
        let expected_tickets = if matches!(name, "released" | "unlinked") {
            serde_json::json!([{"token":"102","allocation_id":original_retirement["source"]["witness"]["allocation_id"],"registered_charge":original_retirement["source"]["registered_charge"]}])
        } else {
            serde_json::json!([])
        };
        ensure(
            accounting["tickets"] == expected_tickets,
            "exact original retirement ticket",
        )?;
        let mut pending_growth = 0u64;
        let mut registered = BTreeSet::new();
        let mut total = decimal(&accounting["standing_control"])?;
        for tip in array(&accounting["tips"])? {
            fields(tip, &["witness", "arena", "extent", "registered_charge"])?;
            Self::witness(&tip["witness"])?;
            let id = string(&tip["witness"]["allocation_id"])?;
            let (arena, bytes) = self.file(s, id)?;
            let c = all_claims.get(id).ok_or("tip ownership absent")?;
            ensure(
                tip["witness"] == self.registered_witness(id)?
                    && tip["arena"] == arena
                    && c["sealed"] == false
                    && registered.insert(id.to_owned()),
                "tip binding/unique accounting",
            )?;
            let end = decimal(&tip["extent"])?;
            ensure(
                end <= bytes.len() as u64
                    && (end == bytes.len() as u64 || name == "released" || name == "unlinked"),
                "unadmitted tip growth",
            )?;
            let registered_charge = decimal(&tip["registered_charge"])?;
            if matches!(name, "released" | "unlinked") {
                let published = self.named("published-accounting")?;
                let original = array(&published["tips"])?
                    .iter()
                    .find(|t| t["witness"]["allocation_id"] == id)
                    .ok_or("original tip registration")?;
                ensure(
                    tip == original
                        && registered_charge
                            == observed(usize::try_from(end).map_err(|_| "tip end")?)?,
                    "pending growth must retain original observed tip charge",
                )?;
                pending_growth = pending_growth
                    .checked_add(
                        observed(bytes.len())?
                            .checked_sub(registered_charge)
                            .ok_or("tip charge decrease")?,
                    )
                    .ok_or("growth overflow")?;
            } else {
                ensure(
                    registered_charge == observed(bytes.len())?,
                    "exact synthetic tip allocation observation",
                )?;
            }
            total = total
                .checked_add(registered_charge)
                .ok_or("charge overflow")?;
        }
        for sealed in array(&accounting["sealed"])? {
            fields(sealed, &["allocation_id", "registered_charge"])?;
            let id = string(&sealed["allocation_id"])?;
            let c = all_claims.get(id).ok_or("sealed ownership absent")?;
            ensure(
                c["sealed"] == true
                    && sealed["registered_charge"] == c["registered_charge"]
                    && registered.insert(id.to_owned()),
                "unique sealed accounting",
            )?;
            total = total
                .checked_add(decimal(&sealed["registered_charge"])?)
                .ok_or("charge overflow")?;
        }
        ensure(
            registered.len() == all_claims.len(),
            "every owned generation charged once",
        )?;
        if pending_growth > 0 {
            let original = self.named("retirement-admission")?;
            ensure(
                decimal(&original["reserve"])?
                    .checked_sub(pending_growth)
                    .is_some_and(|n| n >= decimal(&original["cleanup_bound"]).unwrap_or(u64::MAX)),
                "pending observed growth consumes only original reserve with cleanup retained",
            )?;
        }
        for ticket in array(&accounting["tickets"])? {
            fields(ticket, &["token", "allocation_id", "registered_charge"])?;
            ensure(
                ticket["token"] == "102"
                    && !all_claims.contains_key(string(&ticket["allocation_id"])?),
                "ticket not physical retention edge",
            )?;
            total = total
                .checked_add(decimal(&ticket["registered_charge"])?)
                .ok_or("ticket overflow")?;
        }
        ensure(
            total == decimal(&accounting["total_charge"])?,
            "exact attributed aggregate",
        )?;
        all.insert(key(&root["placement"]["accounting"])?);
        let union = self.object(&root["inventory"])?;
        schema(&union, "inventory-leaf", &["count", "entries"])?;
        let union_keys: Vec<_> = array(&union["entries"])?
            .iter()
            .map(key)
            .collect::<Result<_>>()?;
        ensure(
            decimal(&union["count"])? == union_keys.len() as u64
                && union_keys.windows(2).all(|w| w[0] < w[1])
                && union_keys.into_iter().collect::<BTreeSet<_>>() == all,
            "exact outer inventory union",
        )?;
        Ok(total)
    }
    pub(super) fn dispatch(
        reader: u64,
        codec: &str,
        bytes: &[u8],
        original_sha: &str,
    ) -> Result<Value> {
        ensure(
            matches!(reader, 1 | 2) && codec == CODEC,
            "unsupported original codec/reader dispatch",
        )?;
        ensure(
            packed::hash(bytes) == original_sha,
            "original admitted byte identity changed",
        )?;
        parse(bytes)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded candidate closure and phase invariants in one explicit audit"
    )]
    pub(super) fn admission(&self, v: &Value) -> Result<()> {
        schema(
            v,
            "original-admission",
            &[
                "token",
                "scope",
                "request",
                "receipt",
                "original_selection",
                "target_selection",
                "reserve",
                "cleanup_bound",
                "maximum_control_frame",
                "maximum_simultaneous_controls",
                "standing_control",
                "original_charge",
                "source_witnesses",
                "payload_sha256",
                "births",
                "existing_tip",
                "original_head_sha256",
                "target_head_sha256",
                "codec",
            ],
        )?;
        self.scope(&v["scope"])?;
        ensure(
            v["token"] == "101"
                && v["codec"] == CODEC
                && v["request"] == object_ref(&self.ops["records"]["intent-3"]["input"])?
                && v["receipt"] == object_ref(&self.ops["records"]["receipt-3"]["input"])?
                && v["original_selection"] == self.corpus["snapshots"]["old"]["root"]
                && v["target_selection"] == self.corpus["snapshots"]["published"]["root"]
                && v["payload_sha256"] == self.ops["pack_sha256"],
            "immutable operation/base/payload/target",
        )?;
        let sources = array(&v["source_witnesses"])?;
        ensure(sources.len() == 2, "bounded source bindings")?;
        for w in sources {
            Self::witness(w)?;
            ensure(
                *w == self.registered_witness(string(&w["allocation_id"])?)?,
                "original source witness changed",
            )?;
        }
        let slots = array(&v["births"])?;
        ensure(slots.len() == 1, "bounded birth slots")?;
        let mut maximum_payload = 0u64;
        let mut ids = BTreeSet::new();
        for slot in slots {
            fields(slot, &["allocation_id", "arena", "nonce", "maximum_end"])?;
            ensure(
                ids.insert(string(&slot["allocation_id"])?.to_owned())
                    && ["data", "metadata"].contains(&string(&slot["arena"])?)
                    && string(&slot["nonce"])?.len() == 64,
                "birth identity/nonce",
            )?;
            let end = decimal(&slot["maximum_end"])?;
            ensure(end <= 262_144, "finite original corridor")?;
            maximum_payload = maximum_payload
                .checked_add(end)
                .ok_or("corridor overflow")?;
            let (arena, bytes) = self.file(
                &self.corpus["snapshots"]["published"],
                string(&slot["allocation_id"])?,
            )?;
            ensure(
                arena == slot["arena"] && bytes.len() as u64 <= end,
                "finalizer exceeds original corridor",
            )?;
        }
        let tip = &v["existing_tip"];
        fields(
            tip,
            &[
                "witness",
                "arena",
                "original_end",
                "original_sha256",
                "maximum_end",
            ],
        )?;
        Self::witness(&tip["witness"])?;
        let (_, old_bytes) = self.file(
            &self.corpus["snapshots"]["old"],
            string(&tip["witness"]["allocation_id"])?,
        )?;
        let (_, new_bytes) = self.file(
            &self.corpus["snapshots"]["published"],
            string(&tip["witness"]["allocation_id"])?,
        )?;
        ensure(
            tip["arena"] == "metadata"
                && decimal(&tip["original_end"])? == old_bytes.len() as u64
                && tip["original_sha256"] == packed::hash(&old_bytes)
                && new_bytes.starts_with(&old_bytes)
                && new_bytes.len() as u64 <= decimal(&tip["maximum_end"])?,
            "original enrolled tip corridor",
        )?;
        maximum_payload = maximum_payload
            .checked_add(decimal(&tip["maximum_end"])?)
            .ok_or("corridor overflow")?;
        ensure(
            v["original_head_sha256"]
                == packed::hash(
                    string(&self.corpus["snapshots"]["old"]["bootstrap"]["head"])?.as_bytes(),
                )
                && v["target_head_sha256"]
                    == packed::hash(
                        string(&self.corpus["snapshots"]["published"]["bootstrap"]["head"])?
                            .as_bytes(),
                    ),
            "original full HEAD selectors",
        )?;
        let frame = decimal(&v["maximum_control_frame"])?;
        let simultaneous = decimal(&v["maximum_simultaneous_controls"])?;
        let standing = decimal(&v["standing_control"])?;
        let cleanup = decimal(&v["cleanup_bound"])?;
        ensure(
            simultaneous == 4
                && frame <= 16384
                && cleanup >= 65536
                && frame
                    .checked_mul(simultaneous)
                    .is_some_and(|n| n <= standing),
            "standing simultaneous control/cleanup bound",
        )?;
        let mut largest = encode(v)?.len() as u64;
        for name in ["worst-width-sizing-only", "retirement-admission"] {
            largest = largest.max(encode(&self.named(name)?)?.len() as u64);
        }
        for r in array(&self.corpus["phase_order"])? {
            largest = largest.max(encode(&self.object(r)?)?.len() as u64);
        }
        for snapshot in self.corpus["snapshots"]
            .as_object()
            .ok_or("snapshots")?
            .values()
        {
            for role in ["head", "commit"] {
                largest = largest.max(string(&snapshot["bootstrap"][role])?.len() as u64);
            }
        }
        ensure(
            largest <= frame,
            "worst-width later control exceeds preadmission envelope",
        )?;
        let minimum = maximum_payload
            .checked_add(frame.checked_mul(simultaneous).ok_or("control overflow")?)
            .and_then(|n| n.checked_add(cleanup))
            .ok_or("reserve overflow")?;
        ensure(
            decimal(&v["reserve"])? >= minimum,
            "whole original reserve before effects",
        )?;
        ensure(
            decimal(&v["original_charge"])?
                == self.snapshot("old", &self.corpus["snapshots"]["old"])?,
            "original exact charged base",
        )
    }
    fn suffix(&self, m: &Value, original: &[u8], complete: &[u8], arena: &str) -> Result<()> {
        fields(
            m,
            &[
                "witness",
                "arena",
                "original_end",
                "original_sha256",
                "final_end",
                "frame_count",
                "framed_bytes",
                "framed_sha256",
            ],
        )?;
        Self::witness(&m["witness"])?;
        let id = string(&m["witness"]["allocation_id"])?;
        ensure(
            m["witness"] == self.registered_witness(id)? && m["arena"] == arena,
            "original destination witness",
        )?;
        ensure(
            decimal(&m["original_end"])? == original.len() as u64
                && m["original_sha256"] == packed::hash(original)
                && decimal(&m["final_end"])? == complete.len() as u64
                && complete.starts_with(original),
            "original destination prefix/exact end",
        )?;
        let suffix = &complete[original.len()..];
        let frames = packed::frame_ranges(suffix, arena)?;
        ensure(
            decimal(&m["frame_count"])? == frames.len() as u64
                && decimal(&m["framed_bytes"])? == suffix.len() as u64
                && m["framed_sha256"] == packed::hash(suffix),
            "complete raw suffix including unreachable frames",
        )
    }
    pub(super) fn compact(&self, v: &Value, data: &[u8], meta: &[u8]) -> Result<()> {
        schema(
            v,
            "compact-plan",
            &[
                "mode",
                "full_data",
                "full_metadata",
                "data",
                "metadata",
                "target",
            ],
        )?;
        let original_data = packed::unhex(string(&self.corpus["original_data_prefix_hex"])?)?;
        let original_meta = packed::unhex(string(&self.corpus["original_metadata_prefix_hex"])?)?;
        ensure(
            v["target"] == self.corpus["snapshots"]["released"]["root"],
            "original typed target",
        )?;
        match string(&v["mode"])? {
            "compact" => {
                ensure(
                    v["full_data"].is_null()
                        && v["full_metadata"].is_null()
                        && !v["data"].is_null()
                        && !v["metadata"].is_null(),
                    "exactly one recipe mode",
                )?;
                self.suffix(&v["data"], &original_data, data, "data")?;
                self.suffix(&v["metadata"], &original_meta, meta, "metadata")
            }
            "full" => {
                ensure(
                    v["data"].is_null()
                        && v["metadata"].is_null()
                        && !v["full_data"].is_null()
                        && !v["full_metadata"].is_null(),
                    "exactly one recipe mode",
                )?;
                for (name, original, complete, arena) in [
                    ("full_data", &original_data, data, "data"),
                    ("full_metadata", &original_meta, meta, "metadata"),
                ] {
                    let full = &v[name];
                    fields(full, &["manifest", "framed_hex"])?;
                    self.suffix(&full["manifest"], original, complete, arena)?;
                    ensure(
                        packed::unhex(string(&full["framed_hex"])?)? == complete[original.len()..],
                        "full original framed bytes",
                    )?;
                }
                Ok(())
            }
            _ => Err("unknown required recipe mode"),
        }
    }
    pub(super) fn birth(&self, v: &Value) -> Result<()> {
        schema(
            v,
            "birth-proof",
            &[
                "original",
                "marker",
                "witness",
                "events",
                "promotion_pair",
                "final_links",
                "stage_absent",
                "namespace_observation",
            ],
        )?;
        let original = self.named("admission")?;
        ensure(
            v["original"] == object_ref(&original)?,
            "same original birth token",
        )?;
        let marker = self.object(&v["marker"])?;
        schema(
            &marker,
            "birth-marker",
            &["token", "scope", "allocation_id", "nonce"],
        )?;
        let slot = &original["births"][0];
        ensure(
            marker["token"] == original["token"]
                && marker["scope"] == original["scope"]
                && marker["allocation_id"] == slot["allocation_id"]
                && marker["nonce"] == slot["nonce"],
            "exclusive original marker identity",
        )?;
        Self::witness(&v["witness"])?;
        ensure(
            v["witness"] == self.registered_witness(string(&slot["allocation_id"])?)?,
            "original bound birth witness",
        )?;
        ensure(
            v["events"]
                == serde_json::json!([
                    "exclusive-marker-create",
                    "marker-barrier",
                    "binding-selected",
                    "truncate-bound-stage",
                    "link-no-replace",
                    "directory-barrier",
                    "unlink-known-stage",
                    "directory-barrier"
                ]),
            "binding before truncate and no-replace promotion ordering",
        )?;
        fields(&v["promotion_pair"], &["stage", "final"])?;
        for role in ["stage", "final"] {
            fields(&v["promotion_pair"][role], &["witness", "links"])?;
            ensure(
                v["promotion_pair"][role]["witness"] == v["witness"]
                    && v["promotion_pair"][role]["links"] == "2",
                "only exact stage/final two-link pair",
            )?;
        }
        ensure(
            v["final_links"] == "1"
                && v["stage_absent"] == true
                && v["namespace_observation"] == "synthetic-profile-only",
            "bound promotion completion",
        )
    }
    pub(super) fn retirement(&self, v: &Value) -> Result<()> {
        schema(
            v,
            "retirement-admission",
            &[
                "token",
                "scope",
                "original_selection",
                "target_selection",
                "original_head_sha256",
                "target_head_sha256",
                "source",
                "reserve",
                "cleanup_bound",
                "maximum_control_frame",
                "maximum_simultaneous_controls",
                "standing_control",
                "compact",
                "codec",
            ],
        )?;
        self.scope(&v["scope"])?;
        ensure(
            v["token"] == "102" && v["codec"] == CODEC,
            "retirement token/codec",
        )?;
        for (field, snapshot) in [("original", "published"), ("target", "released")] {
            ensure(
                v[format!("{field}_selection")] == self.corpus["snapshots"][snapshot]["root"]
                    && v[format!("{field}_head_sha256")]
                        == packed::hash(
                            string(&self.corpus["snapshots"][snapshot]["bootstrap"]["head"])?
                                .as_bytes(),
                        ),
                "original retirement full selector",
            )?;
        }
        fields(
            &v["source"],
            &["witness", "extent", "sha256", "registered_charge"],
        )?;
        Self::witness(&v["source"]["witness"])?;
        let id = string(&v["source"]["witness"]["allocation_id"])?;
        let (_, bytes) = self.file(&self.corpus["snapshots"]["published"], id)?;
        ensure(
            v["source"]["witness"] == self.registered_witness(id)?
                && decimal(&v["source"]["extent"])? == bytes.len() as u64
                && v["source"]["sha256"] == packed::hash(&bytes)
                && decimal(&v["source"]["registered_charge"])? == observed(bytes.len())?,
            "original exact synthetic source charge",
        )?;
        let compact = self.object(&v["compact"])?;
        let data = packed::unhex(string(&self.corpus["complete_data_hex"])?)?;
        let meta = packed::unhex(string(&self.corpus["complete_metadata_hex"])?)?;
        self.compact(&compact, &data, &meta)?;
        let control = decimal(&v["maximum_control_frame"])?;
        let simultaneous = decimal(&v["maximum_simultaneous_controls"])?;
        let cleanup = decimal(&v["cleanup_bound"])?;
        ensure(
            control >= encode(v)?.len() as u64
                && control <= 16384
                && simultaneous == 4
                && cleanup >= 65536
                && control
                    .checked_mul(simultaneous)
                    .is_some_and(|n| n <= decimal(&v["standing_control"]).unwrap_or(0)),
            "retirement control/cleanup bound",
        )?;
        let framed = decimal(&compact["data"]["framed_bytes"])?
            .checked_add(decimal(&compact["metadata"]["framed_bytes"])?)
            .ok_or("suffix overflow")?;
        let minimum = framed
            .checked_add(
                control
                    .checked_mul(simultaneous)
                    .ok_or("control overflow")?,
            )
            .and_then(|n| n.checked_add(cleanup))
            .ok_or("reserve overflow")?;
        ensure(
            decimal(&v["reserve"])? >= minimum,
            "retirement whole original reserve",
        )
    }
    pub(super) fn compact_snapshot(&self, s: &Value) -> Result<()> {
        let descriptor = self.named("compact")?;
        let mut bytes = Vec::new();
        for arena in ["data", "metadata"] {
            let witness = &descriptor[arena]["witness"];
            let (_, raw) = self.file(s, string(&witness["allocation_id"])?)?;
            bytes.push(raw);
        }
        self.compact(&descriptor, &bytes[0], &bytes[1])
    }
    pub(super) fn credit_retry(&self, s: &Value, completed: &Value) -> Result<u64> {
        ensure(
            *completed == self.named("credited")?,
            "exact original terminal record",
        )?;
        self.source_absent(s)?;
        self.compact_snapshot(s)?;
        self.snapshot("credited", s)?;
        ensure(
            completed["phase"] == "credited"
                && completed["selection"] == s["root"]
                && completed["head_sha256"]
                    == packed::hash(string(&s["bootstrap"]["head"])?.as_bytes()),
            "exact already-consumed selection",
        )?;
        Ok(0)
    }
    pub(super) fn eligible(&self, s: &Value, pins: &[(String, Value)]) -> Result<()> {
        let admission = self.named("retirement-admission")?;
        let id = string(&admission["source"]["witness"]["allocation_id"])?;
        let root = self.object(&s["root"])?;
        for role in ["active", "recovery"] {
            let (_, claims, _) = self.owners(s, &root["placement"][format!("{role}_ownership")])?;
            ensure(
                !claims.contains_key(id),
                "selected source ownership still retains generation",
            )?;
        }
        let allowed = [
            "AcceptedJournal",
            "Undo",
            "History",
            "ConversionSource",
            "Export",
            "Backup",
            "ReaderLease",
            "UnresolvedIntent",
            "Relocation",
            "ResourceObligation",
        ];
        for (class, pin) in pins {
            ensure(allowed.contains(&class.as_str()), "unknown pin class")?;
            // The fixed pin's original bytes remain independently verified. Its
            // old growable prefix is not rebound to a newly observed generation.
            self.snapshot("published", pin)?;
            let p = self.object(&pin["root"])?;
            for role in ["active", "recovery"] {
                let (_, claims, _) =
                    self.owners(pin, &p["placement"][format!("{role}_ownership")])?;
                ensure(!claims.contains_key(id), "extra pin retains exact source")?;
            }
        }
        Ok(())
    }
    pub(super) fn source_present(&self, s: &Value) -> Result<()> {
        let original = self.named("retirement-admission")?;
        let expected = &original["source"];
        let id = string(&expected["witness"]["allocation_id"])?;
        let (_, bytes) = self.file(s, id)?;
        ensure(
            s["files"][id]["witness"] == expected["witness"]
                && bytes.len() as u64 == decimal(&expected["extent"])?
                && packed::hash(&bytes) == expected["sha256"],
            "original retained source identity/bytes",
        )
    }
    pub(super) fn source_absent(&self, s: &Value) -> Result<()> {
        let original = self.named("retirement-admission")?;
        let id = string(&original["source"]["witness"]["allocation_id"])?;
        ensure(
            s["files"].as_object().ok_or("files")?.get(id).is_none(),
            "unexpected source reappearance",
        )
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded candidate closure and phase invariants in one explicit audit"
    )]
    pub(super) fn transitions(&self) -> Result<()> {
        let mut previous: Option<Value> = None;
        for name in ["old", "published", "released", "credited"] {
            let snapshot = &self.corpus["snapshots"][name];
            self.bootstrap(snapshot, 3)?;
            let commit = parse(string(&snapshot["bootstrap"]["commit"])?.as_bytes())?;
            if let Some(old) = previous {
                ensure(
                    decimal(&commit["package_revision"])?
                        == decimal(&old["package_revision"])?
                            .checked_add(1)
                            .ok_or("revision overflow")?
                        && commit["parent"]
                            == serde_json::json!({"commit_id":old["commit_id"],"sha256":packed::hash(&encode(&old)?)}),
                    "exact adjacent outer commit transition",
                )?;
            } else {
                ensure(
                    commit["package_revision"] == "1" && commit["parent"].is_null(),
                    "initial outer commit",
                )?;
            }
            previous = Some(commit);
        }
        ensure(
            self.corpus["snapshots"]["released"]["bootstrap"]
                == self.corpus["snapshots"]["unlinked"]["bootstrap"],
            "phase-only HEAD unchanged",
        )
    }
    #[allow(clippy::too_many_lines, reason = "Explicit bounded phase chain")]
    pub(super) fn chain(&self, phases: &[Value]) -> Result<()> {
        self.transitions()?;
        let names = [
            "admitted",
            "witnessed",
            "payload",
            "finalized",
            "published",
            "clean",
            "retirement-held",
            "released",
            "unlink-authorized",
            "unlinked",
            "absence",
            "credited",
        ];
        ensure(phases.len() == names.len(), "complete ordered phase chain")?;
        let admission = self.named("admission")?;
        self.admission(&admission)?;
        let retire = self.named("retirement-admission")?;
        self.retirement(&retire)?;
        let r = decimal(&admission["reserve"])?;
        let cleanup = decimal(&admission["cleanup_bound"])?;
        let old = self.snapshot("old", &self.corpus["snapshots"]["old"])?;
        let published = self.snapshot("published", &self.corpus["snapshots"]["published"])?;
        let c = published
            .checked_sub(old)
            .ok_or("consumed charge decrease")?;
        ensure(
            c == decimal(&self.corpus["expected_consumed"])?
                && r.checked_sub(c).is_some_and(|n| n >= cleanup),
            "C<=R with original cleanup retained",
        )?;
        let released = self.snapshot("released", &self.corpus["snapshots"]["released"])?;
        ensure(
            released == published,
            "source charge transferred to ticket without credit or duplicate growth",
        )?;
        for name in ["released", "unlinked", "credited"] {
            self.compact_snapshot(&self.corpus["snapshots"][name])?;
        }
        self.eligible(&self.corpus["snapshots"]["released"], &[])?;
        self.source_present(&self.corpus["snapshots"]["released"])?;
        self.source_absent(&self.corpus["snapshots"]["unlinked"])?;
        let compact = self.object(&retire["compact"])?;
        let data = packed::unhex(string(&self.corpus["complete_data_hex"])?)?;
        let meta = packed::unhex(string(&self.corpus["complete_metadata_hex"])?)?;
        self.compact(&compact, &data, &meta)?;
        let credited = self.snapshot("credited", &self.corpus["snapshots"]["credited"])?;
        self.source_absent(&self.corpus["snapshots"]["credited"])?;
        let growth = decimal(&self.corpus["expected_retirement_growth"])?;
        let credit = decimal(&retire["source"]["registered_charge"])?;
        ensure(
            credited
                == released
                    .checked_add(growth)
                    .and_then(|n| n.checked_sub(credit))
                    .ok_or("credit arithmetic")?,
            "once-only exact project credit",
        )?;
        for (i, (p, name)) in phases.iter().zip(names).enumerate() {
            let mut allowed = vec![
                "previous",
                "phase",
                "original",
                "selection",
                "head_sha256",
                "consumed",
                "held",
            ];
            match i {
                0 => allowed.push("births"),
                1 => allowed.extend(["births", "birth_proof"]),
                2 => allowed.extend(["births", "payload_sha256", "barrier"]),
                3 => allowed.extend([
                    "births",
                    "payload_sha256",
                    "barrier",
                    "observed_charge",
                    "finalizer_selection",
                ]),
                4 => allowed.push("cleanup_pending"),
                5 => allowed.push("cleanup_barrier"),
                6 | 7 => allowed.push("source_charge_retained"),
                8 => allowed.extend(["source_charge_retained", "authorization_sha256"]),
                9 | 10 => allowed.extend([
                    "source_charge_retained",
                    "authorization_sha256",
                    "source_absent",
                    "directory_barrier",
                ]),
                _ => allowed.extend([
                    "source_charge_retained",
                    "authorization_sha256",
                    "source_absent",
                    "directory_barrier",
                    "project_credit",
                    "filesystem_credit",
                ]),
            }
            schema(p, "selected-phase", &allowed)?;
            ensure(
                p["phase"] == name
                    && p["previous"]
                        == if i == 0 {
                            Value::Null
                        } else {
                            object_ref(&phases[i - 1])?
                        },
                "original monotonic phase predecessor",
            )?;
            let original = if i < 6 { &admission } else { &retire };
            ensure(
                encode(p)?.len() as u64 <= decimal(&original["maximum_control_frame"])?,
                "actual supplied control frame bound",
            )?;
            if i < 4 {
                let births = if i == 0 {
                    Vec::new()
                } else {
                    array(&admission["births"])?
                        .iter()
                        .map(|b| self.registered_witness(string(&b["allocation_id"])?))
                        .collect::<Result<Vec<_>>>()?
                };
                ensure(
                    p["births"] == Value::Array(births),
                    "exact original newborn witness set",
                )?;
            }
            ensure(
                p["original"] == object_ref(original)?,
                "unchanged original admitted bytes",
            )?;
            let selected = if i < 4 {
                "old"
            } else if i < 7 {
                "published"
            } else if i < 11 {
                "released"
            } else {
                "credited"
            };
            ensure(
                p["selection"] == self.corpus["snapshots"][selected]["root"]
                    && p["head_sha256"]
                        == packed::hash(
                            string(&self.corpus["snapshots"][selected]["bootstrap"]["head"])?
                                .as_bytes(),
                        ),
                "phase-only or joint selection boundary",
            )?;
            let (consumed, held) = if i < 4 {
                (0, r)
            } else if i == 4 {
                (c, r - c)
            } else if i == 5 {
                (c, 0)
            } else if i < 11 {
                (0, decimal(&retire["reserve"])?)
            } else {
                (growth, 0)
            };
            ensure(
                decimal(&p["consumed"])? == consumed && decimal(&p["held"])? == held,
                "immutable R atomic C/R-C and cleanup settlement",
            )?;
            if i == 1 {
                self.birth(&self.object(&p["birth_proof"])?)?;
            }
            if i == 1 || i == 2 || i == 3 {
                let births = array(&p["births"])?;
                ensure(births.len() == 1, "all births witnessed before payload")?;
                for w in births {
                    Self::witness(w)?;
                    ensure(
                        *w == self.registered_witness(string(&w["allocation_id"])?)?,
                        "birth witness substitution",
                    )?;
                }
            }
            if i == 2 || i == 3 {
                ensure(
                    p["barrier"] == true && p["payload_sha256"] == admission["payload_sha256"],
                    "original payload before finalization",
                )?;
            }
            if i == 3 {
                ensure(
                    decimal(&p["observed_charge"])? == c
                        && p["finalizer_selection"] == admission["target_selection"],
                    "actual finalization evidence not sizing sentinel",
                )?;
            }
            if i == 4 {
                ensure(p["cleanup_pending"] == true, "cleanup corridor retained")?;
            }
            if i == 5 {
                ensure(p["cleanup_barrier"] == true, "cleanup release barrier")?;
            }
            if (6..11).contains(&i) {
                ensure(
                    p["source_charge_retained"] == true,
                    "source charge held through absence",
                )?;
            }
            if i >= 8 {
                ensure(
                    p["authorization_sha256"] == packed::hash(&encode(&retire)?),
                    "original selected unlink authorization",
                )?;
            }
            if i >= 9 {
                ensure(
                    p["source_absent"] == true && p["directory_barrier"] == Value::Bool(i >= 10),
                    "unlink/barrier/fresh absence ordering",
                )?;
            }
            if i == 11 {
                ensure(
                    p["source_charge_retained"] == false
                        && decimal(&p["project_credit"])? == credit
                        && p["filesystem_credit"] == "0",
                    "exact project credit no OS-space credit",
                )?;
            }
        }
        Ok(())
    }
}
