//! Frozen resource-state v2 metadata validation; no external media reads or qualification.
use super::super::{ObjectRef, PackageError};
use super::{LogicalRecords, Membership, SelectionIdentity, TreeLimits};
use photara_core::{
    context::value::Timestamp,
    contracts::{
        resource::ExternalCoordinate,
        schema::{MediaType, QualifiedName},
    },
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
type Key = (String, u64);
type Result<T> = std::result::Result<T, &'static str>;
const FEATURE: &str = "photara.resource-state-trees.v1";
fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("resource string")
}
fn number(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "resource decimal")?;
    ensure(n.to_string() == s, "resource canonical decimal")?;
    Ok(n)
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn reference(v: &Value) -> Result<Key> {
    let r: ObjectRef = serde_json::from_value(v.clone()).map_err(|_| "resource reference")?;
    ensure(
        r.kind == super::super::ObjectKind::Json && r.byte_length.get() > 0,
        "resource JSON ref",
    )?;
    Ok((r.sha256.as_str().into(), r.byte_length.get()))
}
#[derive(Clone, Copy, Debug)]
pub struct ResourceLimits {
    pub tree: TreeLimits,
    pub max_objects: usize,
    pub max_record_bytes: usize,
}
/// Exact typed metadata and associations. Origin authority must be resolved by
/// the selected root/history/recovery/pin/pending validator before package acceptance.
pub struct ResourceMetadata {
    pub closure: BTreeSet<Key>,
    pub requirements: BTreeMap<String, Value>,
    pub associations: BTreeMap<String, Value>,
}
struct Proof {
    closure: BTreeSet<Key>,
    requirements: BTreeMap<String, Value>,
    associations: BTreeMap<String, Value>,
}
struct Store<'a, P> {
    provider: &'a P,
    project: String,
    library: String,
    context: Value,
    limits: ResourceLimits,
    error: std::cell::RefCell<Option<PackageError>>,
}
impl<P: LogicalRecords> Store<'_, P> {
    fn get(&self, r: &Value, seen: &mut BTreeSet<Key>) -> Result<Value> {
        let key = reference(r)?;
        ensure(
            key.1 <= self.limits.max_record_bytes as u64,
            "resource work limit",
        )?;
        ensure(
            seen.contains(&key) || seen.len() < self.limits.max_objects,
            "resource work limit",
        )?;
        let reference: ObjectRef = serde_json::from_value(r.clone()).map_err(|_| "resource ref")?;
        let value = self
            .provider
            .resolve(&reference, Membership::Semantic)
            .map_err(|error| {
                *self.error.borrow_mut() = Some(error);
                "resource resolution"
            })?;
        let bytes = photara_core::canonical_json(&value).map_err(|_| "resource canonical")?;
        ensure(
            bytes.len() as u64 == key.1 && hash(&bytes) == key.0,
            "resource bytes",
        )?;
        seen.insert(key);
        Ok(value)
    }
}
/// Validates exact resource records, selection trees and immutable cross-links.
/// It deliberately does not certify origin authority, retention availability,
/// profile qualification or current write access from historical record bytes.
/// # Errors
/// Refuses unsupported/malformed resource schemas and caller work-budget exhaustion.
pub fn inspect_resources(
    provider: &impl LogicalRecords,
    state_ref: &ObjectRef,
    root_id: super::super::PackageUuid,
    identity: &SelectionIdentity,
    limits: ResourceLimits,
) -> std::result::Result<ResourceMetadata, PackageError> {
    let context = json!({"root_id":root_id,"resource_state":state_ref,"required_features":["photara.resource-backings.v1",FEATURE]});
    let store = Store {
        provider,
        project: identity.project.to_string(),
        library: identity.library.to_string(),
        context,
        limits,
        error: std::cell::RefCell::new(None),
    };
    let proof = verify_with_origin(&store, "selected", |source, context| {
        if source["origin"]["kind"] == "authored" {
            ensure(
                source["origin"]["source_id"] == context["root_id"],
                "resource authored origin",
            )?;
        }
        ensure(
            matches!(
                text(&source["origin"]["kind"])?,
                "authored" | "history" | "recovery" | "explicit" | "pending"
            ),
            "resource origin dispatch",
        )
    })
    .map_err(|why| {
        store
            .error
            .borrow_mut()
            .take()
            .unwrap_or(if why == "resource work limit" {
                PackageError::Limit
            } else {
                PackageError::Integrity
            })
    })?;
    Ok(ResourceMetadata {
        closure: proof.closure,
        requirements: proof.requirements,
        associations: proof.associations,
    })
}
fn array(v: &Value, limit: usize) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("array")?;
    ensure(a.len() <= limit, "array bound")?;
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
fn schema(
    s: &Store<'_, impl LogicalRecords>,
    v: &Value,
    name: &str,
    version: u64,
    extra: &[&str],
) -> Result<()> {
    let mut keys = vec!["schema", "project_id", "extensions"];
    keys.extend(extra);
    fields(v, &keys)?;
    fields(&v["schema"], &["id", "version"])?;
    ensure(
        v["schema"]["id"] == name
            && v["schema"]["version"] == version
            && v["project_id"] == s.project,
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
fn location(s: &Store<'_, impl LogicalRecords>, v: &Value) -> Result<()> {
    fields(v, &["library_id", "storage_location_id", "coordinate"])?;
    ensure(v["library_id"] == s.library, "location library")?;
    id(&v["storage_location_id"])?;
    serde_json::from_value::<ExternalCoordinate>(v["coordinate"].clone())
        .map(|_| ())
        .map_err(|_| "portable coordinate")
}
fn evidence(s: &Store<'_, impl LogicalRecords>, v: &Value) -> Result<()> {
    schema(
        s,
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
            location(s, &d["location"])?;
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
    s: &Store<'_, impl LogicalRecords>,
    root: &Value,
    kind: &str,
    closure: &mut BTreeSet<Key>,
    path: &mut BTreeSet<Key>,
    depth: usize,
) -> Result<Vec<Value>> {
    let identity = reference(root)?;
    ensure(
        depth < s.limits.tree.max_depth
            && path.len() < s.limits.tree.max_pages
            && path.insert(identity),
        "tree cycle/alias/depth bound",
    )?;
    let node = s.get(root, closure)?;
    let leaf = node["schema"]["id"] == "photara.resource.selection-leaf";
    schema(
        s,
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
        let values = array(&node["entries"], s.limits.tree.max_leaf_entries)?;
        ensure(values.len() <= s.limits.tree.max_leaf_entries, "leaf bound")?;
        entries.clone_from(values);
    } else {
        let children = array(&node["children"], s.limits.tree.max_branch_children)?;
        ensure(
            children.len() >= 2 && children.len() <= s.limits.tree.max_branch_children,
            "branch fanout",
        )?;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            id(&child["first"])?;
            id(&child["last"])?;
            let values = tree(s, &child["child"], kind, closure, path, depth + 1)?;
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
        entries.len() <= s.limits.tree.max_entries
            && entries.len() as u64 == number(&node["count"])?,
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
    s: &Store<'_, impl LogicalRecords>,
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
fn verify_with_origin(
    s: &Store<'_, impl LogicalRecords>,
    role: &str,
    mut origin: impl FnMut(&Value, &Value) -> Result<()>,
) -> Result<Proof> {
    let context = &s.context;
    let _ = role;
    fields(context, &["root_id", "resource_state", "required_features"])?;
    id(&context["root_id"])?;
    ensure(
        context["required_features"] == json!(["photara.resource-backings.v1", FEATURE]),
        "factored capability",
    )?;
    let mut closure = BTreeSet::new();
    let state = s.get(&context["resource_state"], &mut closure)?;
    schema(
        s,
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
    ensure(state["library_id"] == s.library, "state library")?;
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
    let sources = selection(
        s,
        &state["retention_sources"],
        "retention_sources",
        &mut closure,
    )?;
    let mut covered = BTreeMap::new();
    for source in sources.values() {
        schema(
            s,
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
    semantics(s, &selected, &mut closure)?;
    Ok(Proof {
        closure,
        requirements: selected.remove("requirements").ok_or("requirements")?,
        associations: sources,
    })
}
// SEMANTICS
#[expect(
    clippy::too_many_lines,
    reason = "Direct immutable record and cross-record semantic checks preserve the existing resource contract in one pass"
)]
fn semantics(
    s: &Store<'_, impl LogicalRecords>,
    selections: &Selections,
    closure: &mut BTreeSet<Key>,
) -> Result<()> {
    let identities = &selections["identities"];
    let working = &selections["working_bindings"];
    let versions = &selections["versions"];
    let backings = &selections["backings"];
    let requirements = &selections["requirements"];
    for v in identities.values() {
        schema(
            s,
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
            s,
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
            location(s, &v["location"])?;
        }
    }
    for v in versions.values() {
        schema(
            s,
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
        evidence(s, &e)?;
        ensure(
            e["evidence_kind"] == "capture"
                && e["operation_id"] == v["capture_operation_id"]
                && e["established_at"] == v["captured_at"],
            "capture identity/time",
        )?;
        version_agrees(v, &e)?;
    }
    for v in backings.values() {
        schema(
            s,
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
        location(s, &v["location"])?;
        let version = versions
            .get(text(&v["version_id"])?)
            .ok_or("backing selected version")?;
        version_agrees(version, v)?;
        let e = s.get(&v["publication_evidence"], closure)?;
        evidence(s, &e)?;
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
    }
    for v in requirements.values() {
        schema(
            s,
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
        // Qualification availability and replica policy are separate from metadata validity.
        let mut last = None;
        for bid in array(&v["required_backing_ids"], s.limits.tree.max_entries)? {
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
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Records(BTreeMap<ObjectRef, Value>);
    impl LogicalRecords for Records {
        fn resolve(
            &self,
            reference: &ObjectRef,
            membership: Membership,
        ) -> std::result::Result<Value, PackageError> {
            if membership != Membership::Semantic {
                return Err(PackageError::Record);
            }
            self.0
                .get(reference)
                .cloned()
                .ok_or(PackageError::Integrity)
        }
    }
    #[test]
    fn integrated_resource_metadata_retains_exact_typed_closure_and_refuses_missing_evidence() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        let mut records=Records(corpus["records"].as_object().unwrap().values().map(|row|{let value:Value=serde_json::from_str(row["canonical"].as_str().unwrap()).unwrap();let bytes=photara_core::canonical_json(&value).unwrap();let reference: ObjectRef=serde_json::from_value(json!({"kind":"json","sha256":hash(&bytes),"byte_length":bytes.len().to_string()})).unwrap();(reference,value)}).collect());
        let state = records
            .0
            .values()
            .find(|v| v["schema"]["id"] == "photara.package.state-root")
            .unwrap()
            .clone();
        let identity = SelectionIdentity {
            project: super::super::selected::nonnil(&state["project_id"]).unwrap(),
            library: super::super::selected::nonnil(&state["library_id"]).unwrap(),
            bootstrap_sha256: super::super::selected::sha(&state["bootstrap_sha256"]).unwrap(),
        };
        let root_id = super::super::selected::nonnil(&state["root_id"]).unwrap();
        let reference = serde_json::from_value(state["resource_state"].clone()).unwrap();
        let limits = ResourceLimits {
            tree: TreeLimits {
                max_depth: 16,
                max_pages: 512,
                max_entries: 4096,
                max_leaf_entries: 64,
                max_branch_children: 8,
            },
            max_objects: 4096,
            max_record_bytes: 1 << 20,
        };
        let proof = inspect_resources(&records, &reference, root_id, &identity, limits).unwrap();
        assert!(!proof.associations.is_empty());
        assert!(!proof.requirements.is_empty());
        let evidence = records
            .0
            .iter()
            .find(|(_, v)| v["schema"]["id"] == "photara.resource.publication-evidence")
            .unwrap()
            .0
            .clone();
        records.0.remove(&evidence);
        assert!(inspect_resources(&records, &reference, root_id, &identity, limits).is_err());
    }
}
