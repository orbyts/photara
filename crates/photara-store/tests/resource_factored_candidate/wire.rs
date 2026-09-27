//! Unfrozen additive resource metadata projection. No package writer or media access.
use super::legacy::{Key, Result, ensure, hash, number, parse, reference, text};
use photara_core::{
    context::value::Timestamp,
    contracts::{
        resource::ExternalCoordinate,
        schema::{MediaType, QualifiedName},
    },
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
pub(super) const FEATURE: &str = "photara.resource-state-trees.v1";
fn array(v: &Value) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("array")?;
    ensure(a.len() <= 256, "array bound")?;
    Ok(a)
}
fn id(v: &Value) -> Result<()> {
    let s = text(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!u.is_nil() && u.to_string() == s, "canonical UUID")
}
fn digest(v: &Value) -> Result<()> {
    let s = text(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )
}
pub(super) fn fields(v: &Value, keys: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("object")?;
    ensure(
        m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)),
        "exact fields",
    )
}
fn schema(v: &Value, name: &str, version: u64, extra: &[&str]) -> Result<()> {
    let mut keys = vec!["schema", "project_id", "extensions"];
    keys.extend(extra);
    fields(v, &keys)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"] == name
            && v["schema"]["version"] == version
            && v["project_id"] == PROJECT,
        "schema/project",
    )?;
    ensure(
        v["extensions"]
            .as_object()
            .ok_or("extensions")?
            .keys()
            .all(|s| QualifiedName::parse(s.clone()).is_ok()),
        "extension namespace",
    )
}
fn timestamp(v: &Value) -> Result<()> {
    Timestamp::try_from(text(v)?.to_owned())
        .map(|_| ())
        .map_err(|_| "timestamp")
}
fn location(v: &Value) -> Result<()> {
    fields(v, &["library_id", "storage_location_id", "coordinate"])?;
    ensure(v["library_id"] == LIBRARY, "location library")?;
    id(&v["storage_location_id"])?;
    serde_json::from_value::<ExternalCoordinate>(v["coordinate"].clone())
        .map(|_| ())
        .map_err(|_| "portable coordinate")
}
fn evidence(v: &Value) -> Result<()> {
    schema(
        v,
        "photara.resource.publication-evidence",
        1,
        &[
            "evidence_id",
            "operation_id",
            "request_sha256",
            "resource_id",
            "version_id",
            "sha256",
            "byte_length",
            "established_at",
            "evidence_kind",
            "destination",
            "qualification",
        ],
    )?;
    for k in ["evidence_id", "operation_id", "resource_id", "version_id"] {
        id(&v[k])?;
    }
    for k in ["request_sha256", "sha256"] {
        digest(&v[k])?;
    }
    number(&v["byte_length"])?;
    timestamp(&v["established_at"])?;
    match text(&v["evidence_kind"])? {
        "capture" => ensure(
            v["destination"].is_null() && v["qualification"].is_null(),
            "capture has no retention qualification",
        ),
        "retained-publication" => {
            let d = &v["destination"];
            fields(d, &["backing_id", "backing_revision", "location"])?;
            id(&d["backing_id"])?;
            number(&d["backing_revision"])?;
            location(&d["location"])?;
            let q = &v["qualification"];
            fields(
                q,
                &["profile_id", "profile_revision", "failure_model_sha256"],
            )?;
            id(&q["profile_id"])?;
            number(&q["profile_revision"])?;
            digest(&q["failure_model_sha256"])
        }
        _ => Err("evidence kind"),
    }
}
fn version_agrees(v: &Value, other: &Value) -> Result<()> {
    ensure(
        ["resource_id", "version_id", "sha256", "byte_length"]
            .iter()
            .all(|k| v[k] == other[k]),
        "version evidence agreement",
    )
}

#[derive(Clone)]
pub(super) struct Store {
    pub records: BTreeMap<String, Vec<u8>>,
    pub roots: Value,
    pub profile: Value,
    pub representation: Value,
}
impl Store {
    pub fn load(v: &Value) -> Result<Self> {
        let m = v["records"].as_object().ok_or("records")?;
        ensure(m.len() <= 4096, "record count bound")?;
        Ok(Self {
            records: m
                .iter()
                .map(|(k, v)| Ok((k.clone(), text(&v["canonical"])?.as_bytes().to_vec())))
                .collect::<Result<_>>()?,
            roots: v["roots"].clone(),
            profile: v["fixture_profile"].clone(),
            representation: v["representation"].clone(),
        })
    }
    pub fn get(&self, r: &Value, seen: &mut BTreeSet<Key>) -> Result<Value> {
        let k = reference(r)?;
        ensure(seen.len() < 4096, "closure bound")?;
        let b = self.records.get(&k.0).ok_or("missing object")?;
        ensure(b.len() as u64 == k.1 && hash(b) == k.0, "referenced bytes")?;
        seen.insert(k);
        parse(b)
    }
}
type Selections = BTreeMap<String, BTreeMap<String, Value>>;
fn kind_id(kind: &str) -> Result<&'static str> {
    match kind {
        "identities" => Ok("resource_id"),
        "working_bindings" => Ok("binding_id"),
        "versions" => Ok("version_id"),
        "backings" => Ok("backing_id"),
        "requirements" => Ok("requirement_id"),
        "retention_sources" => Ok("association_id"),
        _ => Err("selection kind"),
    }
}
fn tree(
    store: &Store,
    root: &Value,
    kind: &str,
    closure: &mut BTreeSet<Key>,
    path: &mut BTreeSet<Key>,
    depth: usize,
) -> Result<Vec<Value>> {
    let identity = reference(root)?;
    ensure(
        depth < 8 && path.len() < 256 && path.insert(identity),
        "tree cycle/alias/depth bound",
    )?;
    let node = store.get(root, closure)?;
    let leaf = node["schema"]["id"] == "photara.resource.selection-leaf";
    schema(
        &node,
        if leaf {
            "photara.resource.selection-leaf"
        } else {
            "photara.resource.selection-branch"
        },
        1,
        &[
            "selection_kind",
            "count",
            if leaf { "entries" } else { "children" },
        ],
    )?;
    ensure(node["selection_kind"] == kind, "tree selection kind")?;
    let mut entries = vec![];
    if leaf {
        let values = array(&node["entries"])?;
        ensure(values.len() <= 4, "leaf bound")?;
        entries.clone_from(values);
    } else {
        let children = array(&node["children"])?;
        ensure((2..=4).contains(&children.len()), "branch fanout")?;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            id(&child["first"])?;
            id(&child["last"])?;
            let values = tree(store, &child["child"], kind, closure, path, depth + 1)?;
            ensure(
                !values.is_empty()
                    && values[0]["id"] == child["first"]
                    && values[values.len() - 1]["id"] == child["last"]
                    && values.len() as u64 == number(&child["count"])?,
                "exact child range/count",
            )?;
            entries.extend(values);
        }
    }
    ensure(
        entries.len() <= 256 && entries.len() as u64 == number(&node["count"])?,
        "exact tree count/bound",
    )?;
    let mut previous = None;
    for e in &entries {
        fields(e, &["id", "record"])?;
        id(&e["id"])?;
        reference(&e["record"])?;
        let current = text(&e["id"])?;
        ensure(
            previous.is_none_or(|p| p < current),
            "selection unique order",
        )?;
        previous = Some(current);
    }
    Ok(entries)
}
fn selection(
    s: &Store,
    r: &Value,
    kind: &str,
    closure: &mut BTreeSet<Key>,
) -> Result<BTreeMap<String, Value>> {
    let mut values = BTreeMap::new();
    for e in tree(s, r, kind, closure, &mut BTreeSet::new(), 0)? {
        let v = s.get(&e["record"], closure)?;
        ensure(v[kind_id(kind)?] == e["id"], "selected target identity")?;
        values.insert(text(&e["id"])?.into(), v);
    }
    Ok(values)
}
pub(super) struct Proof {
    pub closure: BTreeSet<Key>,
    pub requirements: BTreeMap<String, Value>,
    pub supported: bool,
    pub root_local: BTreeSet<Key>,
    pub associations: BTreeMap<String, Value>,
}
pub(super) fn verify(s: &Store, role: &str) -> Result<Proof> {
    verify_with_origin(s, role, |source, context| {
        ensure(
            source["origin"] == json!({"kind":"authored","source_id":context["root_id"]}),
            "resolved selected source association",
        )
    })
}
/// Caller must subsequently establish non-authored authority through its selected
/// typed evidence. This hook changes no record schema, subset, or retention check.
pub(super) fn verify_with_origin(
    s: &Store,
    role: &str,
    mut origin: impl FnMut(&Value, &Value) -> Result<()>,
) -> Result<Proof> {
    let context = &s.roots[role];
    fields(context, &["root_id", "resource_state", "required_features"])?;
    id(&context["root_id"])?;
    ensure(
        context["required_features"] == json!(["photara.resource-backings.v1", FEATURE]),
        "factored capability",
    )?;
    let mut closure = BTreeSet::new();
    let state = s.get(&context["resource_state"], &mut closure)?;
    schema(
        &state,
        "photara.resource.state",
        2,
        &[
            "library_id",
            "identities",
            "working_bindings",
            "versions",
            "backings",
            "requirements",
            "retention_sources",
        ],
    )?;
    ensure(state["library_id"] == LIBRARY, "state library")?;
    let mut selected = Selections::new();
    for kind in [
        "identities",
        "working_bindings",
        "versions",
        "backings",
        "requirements",
    ] {
        selected.insert(
            kind.to_owned(),
            selection(s, &state[kind], kind, &mut closure)?,
        );
    }
    let common = closure.clone();
    let sources = selection(
        s,
        &state["retention_sources"],
        "retention_sources",
        &mut closure,
    )?;
    let mut covered = BTreeMap::new();
    let mut local = closure
        .difference(&common)
        .cloned()
        .collect::<BTreeSet<_>>();
    local.insert(reference(&context["resource_state"])?);
    for source in sources.values() {
        schema(
            source,
            "photara.resource.retention-source",
            1,
            &["association_id", "origin", "requirements"],
        )?;
        id(&source["association_id"])?;
        fields(&source["origin"], &["kind", "source_id"])?;
        id(&source["origin"]["source_id"])?;
        origin(source, context)?;
        let subset = selection(s, &source["requirements"], "requirements", &mut closure)?;
        ensure(!subset.is_empty(), "nonempty source subset")?;
        for (id, v) in subset {
            ensure(
                selected["requirements"].get(&id) == Some(&v),
                "source subset exact shared requirement",
            )?;
            if let Some(prior) = covered.insert(id, v.clone()) {
                ensure(prior == v, "source shared identity")?;
            }
        }
    }
    ensure(covered == selected["requirements"], "exact source coverage")?;
    let supported = semantics(s, &selected, &mut closure)?;
    Ok(Proof {
        closure,
        requirements: selected.remove("requirements").ok_or("requirements")?,
        supported,
        root_local: local,
        associations: sources,
    })
}
/// Across retained roots the same association identity must mean the same exact immutable record.
pub(super) fn verify_pair(s: &Store) -> Result<(Proof, Proof)> {
    let a = verify(s, "a")?;
    let b = verify(s, "b")?;
    for (id, v) in &a.associations {
        if let Some(other) = b.associations.get(id) {
            ensure(v == other, "immutable association identity across roots")?;
        }
    }
    Ok((a, b))
}
pub(super) fn retained(s: &Store, role: &str) -> Result<Proof> {
    let p = verify(s, role)?;
    ensure(p.supported, "fixture requirement unsupported")?;
    Ok(p)
}
pub(super) fn representation(s: &Store, role: &str) -> Result<()> {
    let proof = verify(s, role)?;
    let mut closure = proof.closure;
    let rep = s.get(&s.representation, &mut closure)?;
    schema(
        &rep,
        "photara.project.representation-content",
        3,
        &[
            "asset_id",
            "representation_id",
            "content_revision_id",
            "media",
            "fingerprint",
            "lineage",
            "binding",
        ],
    )?;
    for k in ["asset_id", "representation_id", "content_revision_id"] {
        id(&rep[k])?;
    }
    fields(
        &rep["binding"],
        &["kind", "resource_id", "version_id", "version"],
    )?;
    ensure(
        rep["binding"]["kind"] == "managed-captured",
        "v3 binding kind",
    )?;
    let state = s.get(&s.roots[role]["resource_state"], &mut closure)?;
    let versions = selection(s, &state["versions"], "versions", &mut closure)?;
    let v = versions
        .get(text(&rep["binding"]["version_id"])?)
        .ok_or("binding selected version")?;
    ensure(
        reference(&rep["binding"]["version"])?
            == (
                hash(&photara_core::canonical_json(v).map_err(|_| "canonical")?),
                photara_core::canonical_json(v)
                    .map_err(|_| "canonical")?
                    .len() as u64,
            )
            && rep["binding"]["resource_id"] == v["resource_id"]
            && rep["media"] == json!({"media_type":v["media_type"],"byte_length":v["byte_length"]})
            && rep["fingerprint"]
                == json!({"kind":"content-digest","algorithm":"sha256","value":v["sha256"],"evidence_strength":"verified-bytes"}),
        "v3 exact selected binding",
    )?;
    ensure(array(&rep["lineage"])?.is_empty(), "fixture empty lineage")
}

// SEMANTICS
#[expect(
    clippy::too_many_lines,
    reason = "Direct immutable record and cross-record semantic checks preserve the existing resource contract in one pass"
)]
pub(super) fn semantics(
    s: &Store,
    selections: &Selections,
    closure: &mut BTreeSet<Key>,
) -> Result<bool> {
    let identities = &selections["identities"];
    let working = &selections["working_bindings"];
    let versions = &selections["versions"];
    let backings = &selections["backings"];
    let requirements = &selections["requirements"];
    let mut supported = true;
    for v in identities.values() {
        schema(
            v,
            "photara.resource.identity",
            1,
            &["resource_id", "purpose", "custody", "producer"],
        )?;
        id(&v["resource_id"])?;
        serde_json::from_value::<QualifiedName>(v["purpose"].clone()).map_err(|_| "purpose")?;
        ensure(
            matches!(text(&v["custody"])?, "photara-managed" | "user-source"),
            "custody",
        )?;
        if !v["producer"].is_null() {
            fields(&v["producer"], &["kind", "producer_id"])?;
            serde_json::from_value::<QualifiedName>(v["producer"]["kind"].clone())
                .map_err(|_| "producer kind")?;
            id(&v["producer"]["producer_id"])?;
        }
    }
    let mut bound = BTreeSet::new();
    for v in working.values() {
        schema(
            v,
            "photara.resource.working-binding",
            1,
            &["resource_id", "binding_id", "revision", "location"],
        )?;
        ensure(
            identities.contains_key(text(&v["resource_id"])?),
            "working resource",
        )?;
        id(&v["binding_id"])?;
        number(&v["revision"])?;
        ensure(
            bound.insert(text(&v["resource_id"])?),
            "one working binding per resource",
        )?;
        if !v["location"].is_null() {
            location(&v["location"])?;
        }
    }
    for v in versions.values() {
        schema(
            v,
            "photara.resource.captured-version",
            1,
            &[
                "resource_id",
                "version_id",
                "media_type",
                "byte_length",
                "sha256",
                "captured_at",
                "capture_operation_id",
                "capture_evidence",
            ],
        )?;
        ensure(
            identities.contains_key(text(&v["resource_id"])?),
            "version resource",
        )?;
        id(&v["version_id"])?;
        id(&v["capture_operation_id"])?;
        number(&v["byte_length"])?;
        digest(&v["sha256"])?;
        timestamp(&v["captured_at"])?;
        serde_json::from_value::<MediaType>(v["media_type"].clone()).map_err(|_| "media type")?;
        let e = s.get(&v["capture_evidence"], closure)?;
        evidence(&e)?;
        ensure(
            e["evidence_kind"] == "capture"
                && e["operation_id"] == v["capture_operation_id"]
                && e["established_at"] == v["captured_at"],
            "capture identity/time",
        )?;
        version_agrees(v, &e)?;
    }
    let mut attestations = BTreeMap::new();
    for (bid, v) in backings {
        schema(
            v,
            "photara.resource.backing",
            1,
            &[
                "resource_id",
                "version_id",
                "backing_id",
                "revision",
                "sha256",
                "byte_length",
                "location",
                "publication_evidence",
            ],
        )?;
        id(&v["backing_id"])?;
        number(&v["revision"])?;
        location(&v["location"])?;
        let version = versions
            .get(text(&v["version_id"])?)
            .ok_or("backing selected version")?;
        version_agrees(version, v)?;
        let e = s.get(&v["publication_evidence"], closure)?;
        evidence(&e)?;
        version_agrees(v, &e)?;
        ensure(
            e["evidence_kind"] == "retained-publication",
            "backing needs retained publication",
        )?;
        ensure(
            e["destination"]["backing_id"] == v["backing_id"]
                && e["destination"]["backing_revision"] == v["revision"]
                && e["destination"]["location"] == v["location"],
            "publication destination",
        )?;
        attestations.insert(bid.clone(), e["qualification"].clone());
    }
    for v in requirements.values() {
        schema(
            v,
            "photara.resource.retention-requirement",
            1,
            &[
                "requirement_id",
                "revision",
                "resource_id",
                "version_id",
                "minimum_qualified_copies",
                "required_backing_ids",
                "required_qualification",
            ],
        )?;
        id(&v["requirement_id"])?;
        number(&v["revision"])?;
        let version = versions
            .get(text(&v["version_id"])?)
            .ok_or("obligation selected version")?;
        ensure(
            version["resource_id"] == v["resource_id"],
            "obligation resource",
        )?;
        let min = number(&v["minimum_qualified_copies"])?;
        ensure(min > 0, "positive obligation copies")?;
        let req = &v["required_qualification"];
        fields(
            req,
            &[
                "profile_id",
                "minimum_profile_revision",
                "failure_model_sha256",
            ],
        )?;
        id(&req["profile_id"])?;
        number(&req["minimum_profile_revision"])?;
        digest(&req["failure_model_sha256"])?;
        // A single explicit fixture profile only. No invented production replica/failure-domain policy.
        supported &= req["profile_id"] == s.profile["profile_id"]
            && req["failure_model_sha256"] == s.profile["failure_model_sha256"];
        let qualifies = |bid: &str| -> Result<bool> {
            let backing = backings.get(bid).ok_or("required backing selected")?;
            let q = attestations.get(bid).ok_or("required backing evidence")?;
            Ok(backing["version_id"] == v["version_id"]
                && backing["resource_id"] == v["resource_id"]
                && q["profile_id"] == req["profile_id"]
                && q["failure_model_sha256"] == req["failure_model_sha256"]
                && number(&q["profile_revision"])? >= number(&req["minimum_profile_revision"])?)
        };
        let mut last = None;
        for bid in array(&v["required_backing_ids"])? {
            id(bid)?;
            let bid = text(bid)?;
            ensure(last.is_none_or(|p: &str| p < bid), "required backing order")?;
            last = Some(bid);
            let backing = backings.get(bid).ok_or("required backing selected")?;
            ensure(
                backing["version_id"] == v["version_id"]
                    && backing["resource_id"] == v["resource_id"],
                "required backing version",
            )?;
            supported &= qualifies(bid)?;
        }
        let count = backings
            .keys()
            .map(|bid| qualifies(bid))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|v| *v)
            .count() as u64;
        supported &= count >= min && min == 1;
    }

    Ok(supported)
}
