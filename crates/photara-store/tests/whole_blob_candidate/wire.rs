//! Unfrozen whole-allocation Blob projection. Not a production package opener.
use photara_store::package::{ObjectKind, ObjectRef};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn bytes(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn ensure(ok: bool, e: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(e) }
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|n| m.contains_key(*n)),
        "exact fields",
    )
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("string")
}
fn num(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "decimal")?;
    ensure(n.to_string() == s, "canonical decimal")?;
    Ok(n)
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("array")
}
fn id(v: &Value) -> Result<&str> {
    let s = text(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
    ensure(!u.is_nil() && u.to_string() == s, "canonical UUID")?;
    Ok(s)
}
fn rounded(n: u64) -> Result<u64> {
    n.checked_add(4095)
        .map(|n| n / 4096 * 4096)
        .ok_or("charge overflow")
}
pub(super) fn reference(v: &Value) -> Value {
    json!({"kind":"json","sha256":hash(&bytes(v)),"byte_length":bytes(v).len().to_string()})
}
fn object_ref(v: &Value) -> Result<ObjectRef> {
    serde_json::from_value(v.clone()).map_err(|_| "ObjectRef")
}
fn schema(v: &Value, name: &str, project: &Value, names: &[&str]) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    ensure(
        v["schema"] == json!({"id":name,"version":1}) && v["project_id"] == *project,
        "schema/project",
    )?;
    ensure(
        v["extensions"].as_object().is_some_and(|m| {
            m.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "extension namespace",
    )
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    ensure(
        s.len().is_multiple_of(2)
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "hex",
    )?;
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "hex"))
        .collect()
}
#[derive(Clone)]
pub(super) struct Allocation {
    pub bytes: Vec<u8>,
    pub evidence: Value,
}
#[derive(Clone)]
pub(super) struct Store {
    pub records: BTreeMap<String, Vec<u8>>,
    pub allocations: BTreeMap<String, Allocation>,
    pub selection: Value,
}
impl Store {
    pub fn load(c: &Value) -> Result<Self> {
        let records = c["records"]
            .as_object()
            .ok_or("records")?
            .iter()
            .map(|(k, v)| Ok((k.clone(), text(&v["canonical"])?.as_bytes().to_vec())))
            .collect::<Result<_>>()?;
        let allocations = c["allocations"]
            .as_object()
            .ok_or("allocations")?
            .iter()
            .map(|(k, v)| {
                let mut evidence = v.clone();
                evidence.as_object_mut().unwrap().remove("hex");
                Ok((
                    k.clone(),
                    Allocation {
                        bytes: unhex(text(&v["hex"])?)?,
                        evidence,
                    },
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            records,
            allocations,
            selection: c["selection"].clone(),
        })
    }
    fn get(&self, r: &Value, seen: &mut BTreeSet<String>) -> Result<Value> {
        let r = object_ref(r)?;
        ensure(r.kind == ObjectKind::Json, "JSON control reference")?;
        let raw = self
            .records
            .get(r.sha256.as_str())
            .ok_or("missing control")?;
        ensure(
            raw.len() as u64 == r.byte_length.get() && hash(raw) == r.sha256.as_str(),
            "exact control bytes",
        )?;
        let v: Value = serde_json::from_slice(raw).map_err(|_| "JSON")?;
        ensure(bytes(&v) == *raw, "canonical control bytes")?;
        seen.insert(r.sha256.as_str().to_owned());
        Ok(v)
    }
}
pub(super) struct Proof {
    pub total_charge: u64,
    pub raw_blob_bytes: u64,
    pub blobs: BTreeMap<ObjectRef, String>,
    pub controls: BTreeSet<String>,
}
/// Structural placement/observation checking intentionally does not hash media.
#[allow(
    clippy::too_many_lines,
    reason = "One explicit typed closure ties logical Blob semantics to whole-file layout, ownership and qualified-scope evidence"
)]
pub(super) fn structural(s: &Store) -> Result<Proof> {
    let mut seen = BTreeSet::new();
    let root = s.get(&s.selection, &mut seen)?;
    let project = &root["project_id"];
    id(project)?;
    schema(
        &root,
        "example.ps2.whole-blob-selection",
        project,
        &[
            "managed",
            "representation",
            "blobs",
            "locator",
            "claims",
            "charges",
            "observations",
            "scope",
            "required_features",
            "namespace_allowance",
            "directory_allowance",
            "standing_control",
            "registered_data_charge",
            "total_charge",
        ],
    )?;
    ensure(
        root["required_features"] == json!(["photara.whole-blob-storage.v1"]),
        "whole Blob capability",
    )?;
    fields(&root["scope"], &["profile", "incarnation"])?;
    id(&root["scope"]["incarnation"])?;
    let managed = s.get(&root["managed"], &mut seen)?;
    fields(
        &managed,
        &[
            "schema",
            "project_id",
            "resource_id",
            "version_id",
            "blob",
            "media",
            "retention",
            "original_name",
            "provenance",
        ],
    )?;
    ensure(
        managed["schema"] == json!({"id":"photara.project.managed-resource","version":1})
            && managed["project_id"] == *project
            && managed["retention"] == "managed-project",
        "existing managed resource",
    )?;
    let managed_blob = object_ref(&managed["blob"])?;
    ensure(
        managed_blob.kind == ObjectKind::Blob,
        "existing managed Blob kind",
    )?;
    id(&managed["resource_id"])?;
    id(&managed["version_id"])?;
    fields(&managed["media"], &["media_type", "byte_length"])?;
    ensure(
        num(&managed["media"]["byte_length"])? == managed_blob.byte_length.get(),
        "existing media length",
    )?;
    serde_json::from_value::<photara_core::contracts::schema::MediaType>(
        managed["media"]["media_type"].clone(),
    )
    .map_err(|_| "media type")?;
    let representation = s.get(&root["representation"], &mut seen)?;
    fields(
        &representation,
        &[
            "schema",
            "project_id",
            "asset_id",
            "representation_id",
            "content_revision_id",
            "media",
            "fingerprint",
            "lineage",
            "binding",
        ],
    )?;
    ensure(
        representation["schema"]
            == json!({"id":"photara.project.representation-content","version":2})
            && representation["project_id"] == *project,
        "existing representation v2",
    )?;
    for field in ["asset_id", "representation_id", "content_revision_id"] {
        id(&representation[field])?;
    }
    ensure(
        representation["binding"]
            == json!({"kind":"managed","resource_id":managed["resource_id"],"version_id":managed["version_id"],"version":root["managed"]})
            && representation["media"] == managed["media"]
            && representation["fingerprint"]
                == json!({"kind":"content-digest","algorithm":"sha256","value":managed_blob.sha256,"evidence_strength":"verified-bytes"})
            && representation["lineage"] == json!([]),
        "existing v2 managed binding",
    )?;
    let required = array(&root["blobs"])?
        .iter()
        .map(object_ref)
        .collect::<Result<Vec<_>>>()?;
    ensure(
        !required.is_empty()
            && required.len() <= 16
            && required.iter().all(|r| r.kind == ObjectKind::Blob)
            && required.windows(2).all(|w| w[0] < w[1])
            && required.contains(&managed_blob),
        "exact selected Blob order",
    )?;
    let locator = s.get(&root["locator"], &mut seen)?;
    schema(
        &locator,
        "photara.storage.locator-leaf",
        project,
        &["count", "entries"],
    )?;
    let entries = array(&locator["entries"])?;
    ensure(
        num(&locator["count"])? == required.len() as u64 && entries.len() == required.len(),
        "exact locator count",
    )?;
    let mut blobs = BTreeMap::new();
    let mut allocated = BTreeSet::new();
    let mut raw_blob_bytes = 0u64;
    for (entry, logical) in entries.iter().zip(&required) {
        fields(entry, &["object", "membership", "physical"])?;
        ensure(
            object_ref(&entry["object"])? == *logical && entry["membership"] == "blob",
            "typed Blob locator membership",
        )?;
        let p = &entry["physical"];
        fields(p, &["kind", "allocation_id", "byte_length", "sha256"])?;
        ensure(
            p["kind"] == "whole-blob"
                && num(&p["byte_length"])? == logical.byte_length.get()
                && p["sha256"] == logical.sha256.as_str(),
            "whole Blob reference dispatch",
        )?;
        let allocation = id(&p["allocation_id"])?;
        ensure(
            allocated.insert(allocation.to_owned()),
            "one immutable allocation per logical Blob",
        )?;
        let file = s
            .allocations
            .get(allocation)
            .ok_or("missing Blob allocation")?;
        fields(
            &file.evidence,
            &[
                "layout",
                "observation_id",
                "profile",
                "incarnation",
                "physical",
            ],
        )?;
        ensure(
            file.evidence["layout"] == "whole-blob"
                && file.bytes.len() as u64 == logical.byte_length.get(),
            "whole Blob layout/extent",
        )?;
        raw_blob_bytes = raw_blob_bytes
            .checked_add(file.bytes.len() as u64)
            .ok_or("extent overflow")?;
        blobs.insert(logical.clone(), allocation.to_owned());
    }
    ensure(
        allocated == s.allocations.keys().cloned().collect(),
        "exact allocation membership",
    )?;
    let mut observations = BTreeMap::new();
    for r in array(&root["observations"])? {
        let o = s.get(r, &mut seen)?;
        schema(
            &o,
            "photara.storage.local-observation",
            project,
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
        let subject = &o["subject"];
        fields(subject, &["kind", "allocation_id", "arena"])?;
        ensure(
            subject["kind"] == "whole-blob" && subject["arena"] == "data",
            "whole Blob observation kind",
        )?;
        let allocation = id(&subject["allocation_id"])?;
        id(&o["observation_id"])?;
        let file = s
            .allocations
            .get(allocation)
            .ok_or("observation allocation")?;
        fields(&o["physical"], &["device", "inode"])?;
        num(&o["physical"]["device"])?;
        ensure(num(&o["physical"]["inode"])? > 0, "observation inode")?;
        ensure(
            o["profile"] == root["scope"]["profile"]
                && o["incarnation"] == root["scope"]["incarnation"]
                && file.evidence["profile"] == o["profile"]
                && file.evidence["incarnation"] == o["incarnation"]
                && file.evidence["observation_id"] == o["observation_id"]
                && file.evidence["physical"] == o["physical"],
            "original local allocation observation",
        )?;
        ensure(
            num(&o["measured_extent"])? == file.bytes.len() as u64
                && num(&o["charged_high_water"])? == rounded(file.bytes.len() as u64)?,
            "synthetic observed data highwater",
        )?;
        ensure(
            observations.insert(allocation.to_owned(), o).is_none(),
            "duplicate observed allocation",
        )?;
    }
    ensure(
        observations.keys().cloned().collect::<BTreeSet<_>>() == allocated,
        "exact observed allocations",
    )?;
    let mut charges = BTreeMap::new();
    let mut registered = 0u64;
    for r in array(&root["charges"])? {
        let c = s.get(r, &mut seen)?;
        schema(
            &c,
            "photara.storage.sealed-charge",
            project,
            &[
                "allocation_id",
                "arena",
                "domain_incarnation",
                "measured_extent",
                "charged_high_water",
                "observation",
            ],
        )?;
        let allocation = id(&c["allocation_id"])?;
        let o = observations.get(allocation).ok_or("charge observation")?;
        ensure(
            c["arena"] == "data"
                && c["domain_incarnation"] == root["scope"]["incarnation"]
                && c["measured_extent"] == o["measured_extent"]
                && c["charged_high_water"] == o["charged_high_water"]
                && c["observation"] == reference(o),
            "immutable Blob charge observation",
        )?;
        registered = registered
            .checked_add(num(&c["charged_high_water"])?)
            .ok_or("charge sum overflow")?;
        ensure(
            charges.insert(allocation.to_owned(), c).is_none(),
            "duplicate Blob charge",
        )?;
    }
    let mut claimed = BTreeSet::new();
    for r in array(&root["claims"])? {
        let c = s.get(r, &mut seen)?;
        schema(
            &c,
            "photara.storage.allocation-claim",
            project,
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
        let allocation = id(&c["allocation_id"])?;
        let charge = charges.get(allocation).ok_or("ownership lacks charge")?;
        let (logical, _) = blobs
            .iter()
            .find(|(_, id)| id.as_str() == allocation)
            .ok_or("ownership logical Blob")?;
        ensure(
            c["arena"] == "data"
                && c["layout"] == "whole-blob"
                && c["sealed"] == true
                && num(&c["owned_extent"])? == logical.byte_length.get()
                && c["authenticated_prefix"]
                    == json!({"byte_length":logical.byte_length,"sha256":logical.sha256})
                && c["sealed_charge"] == reference(charge),
            "full Blob ownership",
        )?;
        ensure(
            claimed.insert(allocation.to_owned()),
            "duplicate Blob ownership",
        )?;
    }
    ensure(
        claimed == allocated && charges.keys().cloned().collect::<BTreeSet<_>>() == allocated,
        "exact owned and charged Blobs",
    )?;
    ensure(
        seen == s.records.keys().cloned().collect(),
        "exact bounded control closure",
    )?;
    let standing = num(&root["standing_control"])?;
    let actual = s.records.values().try_fold(0u64, |n, b| {
        n.checked_add(rounded(b.len() as u64)?)
            .ok_or("control overflow")
    })?;
    ensure(actual <= standing, "actual rounded control bound")?;
    let namespace = num(&root["namespace_allowance"])?;
    let directory = num(&root["directory_allowance"])?;
    ensure(
        namespace
            >= u64::try_from(allocated.len())
                .map_err(|_| "count")?
                .checked_mul(4096)
                .ok_or("namespace overflow")?
            && directory >= 4096,
        "conservative empty file/namespace allowance",
    )?;
    let total = registered
        .checked_add(standing)
        .and_then(|n| n.checked_add(namespace))
        .and_then(|n| n.checked_add(directory))
        .ok_or("accounting overflow")?;
    ensure(
        num(&root["registered_data_charge"])? == registered && num(&root["total_charge"])? == total,
        "exact Blob accounting total",
    )?;
    Ok(Proof {
        total_charge: total,
        raw_blob_bytes,
        blobs,
        controls: seen,
    })
}
/// Full media audit/capture is explicit; routine structural open only inspects length.
pub(super) fn audit(s: &Store) -> Result<Proof> {
    let proof = structural(s)?;
    for (logical, allocation) in &proof.blobs {
        ensure(
            hash(&s.allocations[allocation].bytes) == logical.sha256.as_str(),
            "actual full Blob digest",
        )?;
    }
    Ok(proof)
}
