//! Unfrozen pure metadata projection. No package writer or external-media resolver.
use photara_core::{
    context::value::Timestamp,
    contracts::{
        resource::{ExternalCoordinate, RelativeComponents},
        schema::{MediaType, QualifiedName},
    },
};
use photara_store::package::{JsonLimits, MemoryPackage, PackageLimits, parse_canonical_json};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) type Key = (String, u64);
pub(super) const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
pub(super) const LIBRARY: &str = "10000000-0000-4000-8000-000000000002";
pub(super) fn ensure(v: bool, e: &'static str) -> Result<()> {
    if v { Ok(()) } else { Err(e) }
}
pub(super) fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub(super) fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("string")
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("array")?;
    ensure(a.len() <= 128, "array bound")?;
    Ok(a)
}
pub(super) fn number(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "decimal range")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
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
            .all(|s| s.contains('.') && !s.starts_with('.')),
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
pub(super) fn reference(v: &Value) -> Result<Key> {
    fields(v, &["kind", "sha256", "byte_length"])?;
    ensure(v["kind"] == "json", "reference kind")?;
    digest(&v["sha256"])?;
    Ok((text(&v["sha256"])?.into(), number(&v["byte_length"])?))
}
pub(super) fn parse(b: &[u8]) -> Result<Value> {
    ensure(b.len() <= 1024 * 1024, "JSON bound")?;
    parse_canonical_json(b, JsonLimits::default()).map_err(|_| "canonical JSON")
}
#[derive(Clone)]
pub(super) struct Store {
    pub records: BTreeMap<String, Vec<u8>>,
    pub roots: Value,
    pub conversion: Value,
    pub source: BTreeMap<String, Vec<u8>>,
    pub profile: Value,
}
impl Store {
    pub fn load(v: &Value) -> Result<Self> {
        ensure(
            v["records"].as_object().ok_or("records")?.len() <= 128
                && v["source_files"].as_object().ok_or("source files")?.len() <= 128,
            "store count bound",
        )?;
        let records = v["records"]
            .as_object()
            .ok_or("records")?
            .iter()
            .map(|(k, v)| Ok((k.clone(), text(&v["canonical"])?.as_bytes().to_vec())))
            .collect::<Result<_>>()?;
        let source = v["source_files"]
            .as_object()
            .ok_or("source files")?
            .iter()
            .map(|(k, v)| Ok((k.clone(), unhex(text(v)?)?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            records,
            roots: v["roots"].clone(),
            conversion: v["conversion"].clone(),
            source,
            profile: v["fixture_profile"].clone(),
        })
    }
    pub fn get(&self, r: &Value, closure: &mut BTreeSet<Key>) -> Result<Value> {
        let key = reference(r)?;
        ensure(closure.len() < 128, "closure bound")?;
        let b = self.records.get(&key.0).ok_or("missing object")?;
        ensure(
            b.len() as u64 == key.1 && hash(b) == key.0,
            "referenced bytes",
        )?;
        closure.insert(key);
        parse(b)
    }
    pub fn root(&self, role: &str) -> Result<Value> {
        self.get(&self.roots[role], &mut BTreeSet::new())
    }
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    ensure(
        s.len().is_multiple_of(2) && s.len() <= 2 * 1024 * 1024,
        "hex bound",
    )?;
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| {
            u8::from_str_radix(std::str::from_utf8(b).map_err(|_| "hex")?, 16).map_err(|_| "hex")
        })
        .collect()
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
fn selections(
    s: &Store,
    state: &Value,
    field: &str,
    key: &str,
    id_key: &str,
    closure: &mut BTreeSet<Key>,
) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    let mut last = None;
    for item in array(&state[field])? {
        fields(item, &[id_key, key])?;
        id(&item[id_key])?;
        let name = text(&item[id_key])?;
        ensure(
            last.is_none_or(|prev: &str| prev < name),
            "selection order/unique",
        )?;
        last = Some(name);
        let v = s.get(&item[key], closure)?;
        ensure(v[id_key] == item[id_key], "selected target ID")?;
        out.insert(name.to_owned(), v);
    }
    Ok(out)
}
pub(super) fn verify_resources_structural(s: &Store, role: &str) -> Result<(BTreeSet<Key>, bool)> {
    let root = s.root(role)?;
    schema(
        &root,
        "example.ps2.resource-closure-projection",
        1,
        &[
            "library_id",
            "root_id",
            "resource_state",
            "representation",
            "features",
            "pin_sources",
            "inventory",
        ],
    )?;
    ensure(root["library_id"] == LIBRARY, "root library")?;
    id(&root["root_id"])?;
    ensure(
        root["features"] == serde_json::json!(["photara.resource-backings.v1"]),
        "resource feature before lookup",
    )?;
    let (closure, supported) = selected(s, &root, true)?;
    let listed = array(&root["inventory"])?
        .iter()
        .map(reference)
        .collect::<Result<Vec<_>>>()?;
    ensure(
        listed.windows(2).all(|w| w[0] < w[1])
            && listed.into_iter().collect::<BTreeSet<_>>() == closure,
        "exact resource inventory",
    )?;
    Ok((closure, supported))
}
/// Strict fixture evidence-support admission, separate from structural readability.
pub(super) fn verify_resources(s: &Store, role: &str) -> Result<BTreeSet<Key>> {
    let (closure, supported) = verify_resources_structural(s, role)?;
    ensure(supported, "fixture obligation unsupported")?;
    Ok(closure)
}
/// Caller derives `root_id/state` from its already verified selected `StateRoot` and checks the capability.
/// This helper does not select roots, make invented authored edges or resolve external backing bytes.
pub(super) fn verify_selected_state(
    s: &Store,
    state: &Value,
    root_id: &Value,
) -> Result<(BTreeSet<Key>, bool)> {
    id(root_id)?;
    selected(
        s,
        &serde_json::json!({"root_id":root_id,"resource_state":state,"pin_sources":[{"kind":"authored","source_id":root_id}]}),
        false,
    )
}
#[expect(
    clippy::too_many_lines,
    reason = "Closed schema and cross-record checks kept in validation order"
)]
fn selected(
    s: &Store,
    root: &Value,
    include_representation: bool,
) -> Result<(BTreeSet<Key>, bool)> {
    let mut closure = BTreeSet::new();
    let mut supported = true;
    let state = s.get(&root["resource_state"], &mut closure)?;
    schema(
        &state,
        "photara.resource.state",
        1,
        &[
            "library_id",
            "identities",
            "working_bindings",
            "versions",
            "backings",
            "obligations",
        ],
    )?;
    ensure(state["library_id"] == LIBRARY, "state library")?;
    let identities = selections(
        s,
        &state,
        "identities",
        "identity",
        "resource_id",
        &mut closure,
    )?;
    let working = selections(
        s,
        &state,
        "working_bindings",
        "binding",
        "binding_id",
        &mut closure,
    )?;
    let versions = selections(s, &state, "versions", "version", "version_id", &mut closure)?;
    let backings = selections(s, &state, "backings", "backing", "backing_id", &mut closure)?;
    let obligations = selections(
        s,
        &state,
        "obligations",
        "obligation",
        "obligation_id",
        &mut closure,
    )?;
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
        let e = s.get(&v["capture_evidence"], &mut closure)?;
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
    for (bid, v) in &backings {
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
        let e = s.get(&v["publication_evidence"], &mut closure)?;
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
    let mut pins = BTreeSet::new();
    for pin in array(&root["pin_sources"])? {
        fields(pin, &["kind", "source_id"])?;
        ensure(
            matches!(text(&pin["kind"])?, "authored" | "history" | "recovery")
                && pin["source_id"] == root["root_id"],
            "projection pin context",
        )?;
        ensure(
            pins.insert((text(&pin["kind"])?, text(&pin["source_id"])?)),
            "pin unique",
        )?;
    }
    for v in obligations.values() {
        schema(
            v,
            "photara.resource.retention-obligation",
            1,
            &[
                "obligation_id",
                "revision",
                "resource_id",
                "version_id",
                "origin",
                "minimum_qualified_copies",
                "required_backing_ids",
                "required_qualification",
            ],
        )?;
        id(&v["obligation_id"])?;
        number(&v["revision"])?;
        let version = versions
            .get(text(&v["version_id"])?)
            .ok_or("obligation selected version")?;
        ensure(
            version["resource_id"] == v["resource_id"],
            "obligation resource",
        )?;
        fields(&v["origin"], &["kind", "source_id"])?;
        id(&v["origin"]["source_id"])?;
        ensure(
            pins.contains(&(
                text(&v["origin"]["kind"])?,
                text(&v["origin"]["source_id"])?,
            )),
            "resolved typed pin source",
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
    if include_representation {
        let rep = s.get(&root["representation"], &mut closure)?;
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
        ensure(array(&rep["lineage"])?.is_empty(), "fixture empty lineage")?;
        fields(
            &rep["binding"],
            &["kind", "resource_id", "version_id", "version"],
        )?;
        ensure(
            rep["binding"]["kind"] == "managed-captured",
            "v3 binding kind",
        )?;
        let v = versions
            .get(text(&rep["binding"]["version_id"])?)
            .ok_or("binding selected version")?;
        ensure(
            rep["binding"]["resource_id"] == v["resource_id"]
                && s.get(&rep["binding"]["version"], &mut closure)? == *v,
            "binding exact selected version",
        )?;
        fields(&rep["media"], &["media_type", "byte_length"])?;
        fields(
            &rep["fingerprint"],
            &["kind", "algorithm", "value", "evidence_strength"],
        )?;
        ensure(
            rep["media"]["media_type"] == v["media_type"]
                && rep["media"]["byte_length"] == v["byte_length"]
                && rep["fingerprint"]["kind"] == "content-digest"
                && rep["fingerprint"]["algorithm"] == "sha256"
                && rep["fingerprint"]["value"] == v["sha256"]
                && rep["fingerprint"]["evidence_strength"] == "verified-bytes",
            "binding media/fingerprint",
        )?;
    }
    Ok((closure, supported))
}

fn path(v: &Value) -> Result<String> {
    let parts = array(v)?
        .iter()
        .map(|v| text(v).map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    RelativeComponents::new(parts.clone()).map_err(|_| "portable path")?;
    Ok(parts.join("/"))
}
/// Admission holds exact originals and the three registered exclusions. This is a pure byte-map plan.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Registration {
    pub conversion_id: Value,
    pub descriptor_sha256: String,
    pub namespace: String,
}
#[derive(Clone)]
pub(super) struct Intent {
    pub conversion_id: Value,
    pub source: BTreeMap<String, Vec<u8>>,
    pub stable_lock: String,
    pub staging: String,
    pub namespace: String,
    pub descriptor: Value,
}
impl Intent {
    pub fn new(s: &Store) -> Result<Self> {
        let d = s.get(&s.conversion, &mut BTreeSet::new())?;
        let namespace = path(&d["snapshot_directory"])?;
        Ok(Self {
            conversion_id: d["conversion_id"].clone(),
            source: s.source.clone(),
            stable_lock: ".photara-write-lock".into(),
            staging: "staging/this-attempt/intent.json".into(),
            namespace,
            descriptor: d,
        })
    }
    pub fn registration(&self) -> Registration {
        Registration {
            conversion_id: self.conversion_id.clone(),
            descriptor_sha256: hash(
                &photara_core::canonical_json(&self.descriptor).expect("verified descriptor"),
            ),
            namespace: self.namespace.clone(),
        }
    }
    pub fn enumerate(&self, current: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        let actual = current
            .iter()
            .filter(|(p, _)| {
                *p != &self.stable_lock
                    && *p != &self.staging
                    && !p.starts_with(&(self.namespace.clone() + "/"))
            })
            .map(|(p, b)| (p.clone(), b.clone()))
            .collect::<BTreeMap<_, _>>();
        ensure(actual == self.source, "exact admitted source inventory")
    }
    pub fn verify(&self, s: &Store) -> Result<()> {
        let d = s.get(&s.conversion, &mut BTreeSet::new())?;
        ensure(
            d == self.descriptor && d["conversion_id"] == self.conversion_id,
            "immutable conversion intent",
        )?;
        schema(
            &d,
            "photara.package.conversion-source",
            1,
            &[
                "conversion_id",
                "source_format_version",
                "source_bootstrap_sha256",
                "source_head_sha256",
                "source_commit_id",
                "snapshot_directory",
                "files",
            ],
        )?;
        id(&d["conversion_id"])?;
        id(&d["source_commit_id"])?;
        digest(&d["source_bootstrap_sha256"])?;
        digest(&d["source_head_sha256"])?;
        ensure(
            d["snapshot_directory"]
                == serde_json::json!(["conversion-sources", text(&d["conversion_id"])?, "package"]),
            "exact conversion namespace",
        )?;
        let mut found = BTreeMap::new();
        let mut last = None;
        for entry in array(&d["files"])? {
            fields(entry, &["components", "sha256", "byte_length"])?;
            let p = path(&entry["components"])?;
            ensure(
                last.as_ref().is_none_or(|prev: &String| prev < &p),
                "file path order/unique",
            )?;
            last = Some(p.clone());
            digest(&entry["sha256"])?;
            let b = self.source.get(&p).ok_or("manifest source missing")?;
            ensure(
                number(&entry["byte_length"])? == b.len() as u64 && entry["sha256"] == hash(b),
                "snapshot exact bytes",
            )?;
            found.insert(p, b.clone());
        }
        ensure(found == self.source, "complete original inventory")?;
        let manifest = parse(found.get("manifest.json").ok_or("source manifest")?)?;
        let head = parse(found.get("HEAD.json").ok_or("source HEAD")?)?;
        ensure(
            d["source_format_version"] == manifest["format_version"]
                && d["source_bootstrap_sha256"] == hash(&found["manifest.json"])
                && d["source_head_sha256"] == hash(&found["HEAD.json"])
                && d["source_commit_id"] == head["commit_id"]
                && manifest["project_id"] == PROJECT
                && head["project_id"] == PROJECT,
            "legacy source identity",
        )?;
        // Legacy MemoryPackage admits only internal names; the complete manifest above
        // independently preserves every unknown optional byte without parsing it.
        let memory = MemoryPackage::new(legacy_internal(&found), PackageLimits::default())
            .map_err(|_| "legacy memory")?;
        photara_store::package::v1_1::validate_memory(&memory).map_err(|_| "legacy reader")?;
        Ok(())
    }
    /// Validate all occupied bytes before constructing a completed map: no overwrite, no adoption.
    pub fn resume(
        &self,
        s: &Store,
        current: &BTreeMap<String, Vec<u8>>,
        partial: &BTreeMap<String, Vec<u8>>,
        registration: Option<&Registration>,
    ) -> Result<BTreeMap<String, Vec<u8>>> {
        self.verify(s)?;
        ensure(
            registration == Some(&self.registration()),
            "same-attempt snapshot registration",
        )?;
        self.enumerate(current)?;
        let prefix = self.namespace.clone() + "/";
        let occupied = current
            .iter()
            .filter_map(|(p, b)| p.strip_prefix(&prefix).map(|p| (p.to_owned(), b.clone())))
            .collect::<BTreeMap<_, _>>();
        ensure(occupied == *partial, "exact registered snapshot occupancy")?;
        for (p, b) in partial {
            ensure(
                self.source.get(p) == Some(b),
                "conflicting snapshot occupancy",
            )?;
        }
        Ok(self.source.clone())
    }
}

pub(super) fn legacy_internal(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    files
        .iter()
        .filter(|(p, _)| {
            p.as_str() == "HEAD.json"
                || p.as_str() == "manifest.json"
                || p.starts_with("commits/")
                || p.starts_with("objects/")
        })
        .map(|(p, b)| (p.clone(), b.clone()))
        .collect()
}
