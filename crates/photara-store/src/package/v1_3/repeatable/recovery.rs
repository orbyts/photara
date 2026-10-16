//! Original reconstruction and exact retry dispatch. Neither operation grants
//! writable admission or turns selected receipt JSON into a Saved acknowledgment.
use super::shared::OriginalPackage;
use super::{
    Attempt, Limits, RepeatablePlan, VerifiedOriginal, compile, layout,
    wire::{self, encode, hash, key, number},
};
use crate::package::PackageError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub enum Admission {
    ExistingReceipt(Value),
    Prepared(Box<RepeatablePlan>),
}
/// Exact original operation lookup precedes expected-coordinate validation.
/// # Errors
/// Refuses reuse of an operation ID for different bytes, including different
/// expected coordinates; a stale exact retry returns its original receipt.
pub fn admit(
    original: &VerifiedOriginal,
    intent: &Value,
    receipt: &Value,
    attempt: Attempt,
    limits: Limits,
) -> Result<Admission, PackageError> {
    let active = original
        .proof
        .roles
        .get("active")
        .ok_or(PackageError::Integrity)?;
    if let Some(existing) = active
        .receipts
        .iter()
        .find(|r| r["operation_id"] == intent["operation_id"])
    {
        if existing["request_sha256"] != hash(&encode(intent)) {
            return Err(PackageError::Integrity);
        }
        return Ok(Admission::ExistingReceipt(existing.clone()));
    }
    compile(original, intent, receipt, attempt, limits).map(|p| Admission::Prepared(Box::new(p)))
}
/// Recompiles all selectors and append bytes from persisted original inputs.
/// # Errors
/// Refuses any mismatch in canonical original bytes or replayed Core result.
/// The verified wrapper must describe the exact reconstructed original, not the
/// current partially appended package. `execute` separately checks actual suffixes.
pub fn recover(
    original: &VerifiedOriginal,
    record: &Value,
    intent: &Value,
    receipt: &Value,
    limits: Limits,
) -> Result<RepeatablePlan, PackageError> {
    let attempt: Attempt = serde_json::from_value(record["payload_recipe"]["planner"].clone())
        .map_err(|_| PackageError::Record)?;
    let plan = compile(original, intent, receipt, attempt, limits)?;
    if plan.original() != record {
        return Err(PackageError::Integrity);
    }
    Ok(plan)
}
/// Constructs untrusted original bytes for full shared-reader revalidation.
/// Exact prefix digests bound truncation; actual suffixes remain unmodified and
/// must independently pass the resumed phase's append-range checks.
/// # Errors
/// Refuses malformed recipe, missing controls, changed witness, truncated original
/// allocations, altered original prefix, or explicit caller resource bounds.
pub fn reconstruct_original(
    current: &super::input::OriginalPackage,
    record: &Value,
    limits: Limits,
) -> Result<super::input::OriginalPackage, PackageError> {
    reconstruct(&current.clone().into(), record, limits, None, None)
        .map(|p| p.public())
        .map_err(|_| PackageError::Integrity)
}
pub(crate) fn reconstruct_shared_original(
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
) -> Result<OriginalPackage, PackageError> {
    reconstruct(current, record, limits, None, None).map_err(|_| PackageError::Integrity)
}
pub(super) fn reconstruct_cached(
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
    old: &OriginalPackage,
    objects: &BTreeSet<wire::Key>,
    prefixes: Option<&super::restart::prefix_index::Prefixes<'_>>,
) -> Result<OriginalPackage, PackageError> {
    reconstruct(current, record, limits, Some((old, objects)), prefixes)
        .map_err(|_| PackageError::Integrity)
}
#[expect(
    clippy::too_many_lines,
    reason = "Original commitment reconstruction and bounded retained-control recovery kept together"
)]
fn reconstruct(
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
    cached: Option<(&OriginalPackage, &BTreeSet<wire::Key>)>,
    prefixes: Option<&super::restart::prefix_index::Prefixes<'_>>,
) -> wire::Result<OriginalPackage> {
    if record["original_codec"] == "photara.codec.ps2-repeatable-admission-v4" {
        return reconstruct_snapshot(current, record, limits);
    }
    wire::ensure(
        matches!(
            (
                record["original_codec"].as_str(),
                record["payload_recipe"]["codec"].as_str()
            ),
            (
                Some("photara.codec.ps2-repeatable-admission-v1"),
                Some("photara.codec.ps2-repeatable-layout-v1")
            ) | (
                Some("photara.codec.ps2-repeatable-admission-v2"),
                Some("photara.codec.ps2-repeatable-layout-v2")
            ) | (
                Some("photara.codec.ps2-repeatable-admission-v3"),
                Some("photara.codec.ps2-repeatable-layout-v3")
            )
        ),
        "repeatable codecs",
    )?;
    let attempt: Attempt = serde_json::from_value(record["payload_recipe"]["planner"].clone())
        .map_err(|_| "persisted planner")?;
    // Current tail may be partial, so collecting packed objects must not scan it.
    let mut allocations = BTreeMap::new();
    let ranges = record["payload_recipe"]["allocations"]
        .as_array()
        .ok_or("original allocations")?;
    wire::ensure(
        ranges.len() == current.allocations.len() && ranges.len() <= limits.max_objects,
        "allocation count budget",
    )?;
    let mut total = 0u64;
    for range in ranges {
        let id = wire::text(&range["allocation_id"])?;
        let a = current.allocations.get(id).ok_or("allocation")?;
        wire::ensure(
            a.witness == range["witness"] && a.arena == range["arena"],
            "original witness",
        )?;
        let end = usize::try_from(number(&range["original_end"])?)
            .map_err(|_| "prefix platform limit")?;
        wire::ensure(
            end as u64 <= limits.max_allocation_bytes,
            "prefix byte limit",
        )?;
        let prefix = a.bytes.get(..end).ok_or("original truncated")?;
        wire::ensure(
            prefixes.map_or_else(|| hash(prefix), |p| p.hash(id, prefix))
                == range["original_sha256"],
            "original prefix digest",
        )?;
        total = total
            .checked_add(end as u64)
            .ok_or("prefix total overflow")?;
        wire::ensure(
            total <= limits.max_total_allocation_bytes,
            "prefix total budget",
        )?;
        let copy = super::shared::Allocation {
            bytes: a.bytes.prefix(end).ok_or("original truncated")?,
            arena: a.arena.clone(),
            witness: a.witness.clone(),
        };
        wire::ensure(
            allocations.insert(id.to_owned(), copy).is_none(),
            "unique original allocation",
        )?;
    }
    wire::ensure(
        allocations.len() == current.allocations.len(),
        "exact original allocations",
    )?;
    let mut old = OriginalPackage {
        manifest: current.manifest.clone(),
        head: record["old"]["head"].clone(),
        commit: record["old"]["commit"].clone(),
        loose: BTreeMap::new(),
        allocations,
    };
    let mut available = if let Some((saved, objects)) = cached {
        wire::ensure(
            saved.manifest == old.manifest && saved.head == old.head && saved.commit == old.commit,
            "cached original identity",
        )?;
        // Null is only a presence sentinel for immutable authenticated prefix
        // object keys. Exact controls below come from the original decoded view.
        objects
            .iter()
            .cloned()
            .map(|key| (key, Value::Null))
            .collect()
    } else {
        layout::original_objects(&old, &attempt.planner, limits)?
    };
    for name in ["ledger", "envelope", "overlay"] {
        let v = &record["old"][name];
        available.insert(key(&wire::reference(v))?, v.clone());
    }
    wire::ensure(
        current.loose.len() <= limits.max_objects,
        "control count budget",
    )?;
    let mut total_json = 0usize;
    for bytes in current.loose.values() {
        total_json = total_json
            .checked_add(bytes.len())
            .ok_or("control byte overflow")?;
        wire::ensure(
            total_json <= limits.semantic.max_total_json_bytes,
            "control byte budget",
        )?;
        let v = crate::package::parse_canonical_json(bytes, limits.semantic.json)
            .map_err(|_| "control canonical budget")?;
        available.insert(key(&wire::reference(&v))?, v);
    }
    let overlay = &record["old"]["overlay"];
    let refs = overlay["controls"].as_array().ok_or("old controls")?;
    wire::ensure(refs.len() <= limits.max_objects, "control object bound")?;
    let missing = refs
        .iter()
        .map(key)
        .collect::<wire::Result<BTreeSet<_>>>()?
        .into_iter()
        .filter(|k| !available.contains_key(k))
        .collect::<BTreeSet<_>>();
    available.extend(retained_controls(current, &missing, limits)?);
    wire::ensure(
        available.len() <= limits.max_objects,
        "restored object budget",
    )?;
    for r in refs {
        let k = key(r)?;
        let value = available.get(&k).ok_or("preserved original control")?;
        let bytes = if value.is_null() {
            let saved = cached
                .ok_or("cached control source")?
                .0
                .loose
                .get(&k.0)
                .ok_or("cached original control")?;
            wire::ensure(
                saved.len() as u64 == k.1 && hash(saved) == k.0,
                "cached control commitment",
            )?;
            saved.clone()
        } else {
            encode(value)
        };
        old.loose.insert(k.0, bytes);
    }
    old.loose.insert(hash(&encode(overlay)), encode(overlay));
    Ok(old)
}

struct SnapshotRecords<'a> {
    package: &'a OriginalPackage,
    limits: Limits,
}
impl crate::package::v1_3::PhysicalRecords for SnapshotRecords<'_> {
    fn resolve(
        &self,
        reference: &crate::package::v1_3::PhysicalRef,
        kind: crate::package::v1_3::FrameKind,
    ) -> Result<Value, PackageError> {
        let allocation = self
            .package
            .allocations
            .get(&reference.allocation_id.to_string())
            .ok_or(PackageError::Integrity)?;
        let arena = serde_json::from_value(Value::String(allocation.arena.clone()))
            .map_err(|_| PackageError::Record)?;
        crate::package::v1_3::SelectedAllocation::open(
            reference.allocation_id,
            arena,
            &allocation.bytes,
            crate::package::v1_3::FrameLimits {
                json: self.limits.semantic.json,
                max_frames: self.limits.max_frames,
                max_allocation_bytes: usize::try_from(self.limits.max_allocation_bytes)
                    .map_err(|_| PackageError::Limit)?,
            },
        )?
        .resolve(reference, kind)
    }
}
fn snapshot_control(
    current: &OriginalPackage,
    reference: &Value,
    limits: Limits,
) -> Result<Vec<u8>, PackageError> {
    use crate::package::v1_3::{
        Locator, Membership, SelectedEnvelope, SelectedRole, SelectionIdentity, TreeLimits,
    };
    let identity = SelectionIdentity {
        project: crate::package::PackageUuid::parse(
            wire::text(&current.manifest["project_id"]).map_err(|_| PackageError::Record)?,
        )?,
        library: crate::package::PackageUuid::parse(
            wire::text(&current.commit["root_set"]["library_id"])
                .map_err(|_| PackageError::Record)?,
        )?,
        bootstrap_sha256: crate::package::Sha256Hex::parse(
            wire::text(&current.commit["bootstrap_sha256"]).map_err(|_| PackageError::Record)?,
        )?,
    };
    let project = identity.project;
    let envelope = SelectedEnvelope::parse(
        &encode(&current.manifest),
        &encode(&current.head),
        &encode(&current.commit),
        identity,
        limits.semantic.json,
    )?;
    let provider = SnapshotRecords {
        package: current,
        limits,
    };
    let bounds = TreeLimits {
        max_depth: limits.semantic.json.max_depth,
        max_pages: limits.max_objects,
        max_entries: limits.max_entries,
        max_leaf_entries: limits.max_entries,
        max_branch_children: limits.max_entries,
    };
    let object = crate::package::v1_3::selected::json_ref(reference)?;
    for role in [SelectedRole::Active, SelectedRole::Recovery] {
        let selected = envelope.open_role(&provider, role, bounds)?;
        let locator = Locator::new(&provider, project, selected.locator, bounds);
        for membership in [Membership::Ownership, Membership::Semantic] {
            if let Ok(value) = locator.json(&object, membership) {
                return Ok(encode(&value));
            }
        }
    }
    let archive = &current.commit["root_set"]["archive_placement"]["locator"];
    let root = serde_json::from_value(archive.clone()).map_err(|_| PackageError::Record)?;
    let locator = Locator::new(&provider, project, root, bounds);
    for membership in [Membership::Ownership, Membership::Semantic] {
        if let Ok(value) = locator.json(&object, membership) {
            return Ok(encode(&value));
        }
    }
    Err(PackageError::Integrity)
}

// V4 commits the selected old HEAD rather than every archived byte. Rebuild
// only the selected input view; the shared reader must independently verify it
// before the retained original may regenerate a plan. No archive scan occurs.
#[expect(
    clippy::too_many_lines,
    reason = "Exact snapshot prefix and selected-control reconstruction share one bounded input"
)]
fn reconstruct_snapshot(
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
) -> wire::Result<OriginalPackage> {
    wire::ensure(
        record["payload_recipe"]["codec"] == "photara.codec.ps2-repeatable-layout-v4",
        "snapshot codec",
    )?;
    let head = hash(&encode(&record["old"]["head"]));
    let ranges = record["payload_recipe"]["allocations"]
        .as_array()
        .ok_or("snapshot allocations")?;
    wire::ensure(
        ranges.len() == current.allocations.len() && ranges.len() <= limits.max_objects,
        "snapshot allocation count",
    )?;
    let mut allocations = BTreeMap::new();
    let mut total = 0u64;
    for range in ranges {
        crate::package::v1_3::tree::fields(
            range,
            &[
                "allocation_id",
                "arena",
                "witness",
                "original_end",
                "final_end",
                "frame_count",
                "framed_sha256",
                "base_head_sha256",
            ],
        )
        .map_err(|_| "snapshot range fields")?;
        wire::ensure(
            range["base_head_sha256"] == head,
            "snapshot HEAD commitment",
        )?;
        let id = wire::text(&range["allocation_id"])?;
        let a = current.allocations.get(id).ok_or("snapshot allocation")?;
        let end =
            usize::try_from(number(&range["original_end"])?).map_err(|_| "snapshot extent")?;
        total = total.checked_add(end as u64).ok_or("snapshot total")?;
        wire::ensure(
            a.arena == range["arena"]
                && a.witness == range["witness"]
                && end as u64 <= limits.max_allocation_bytes
                && total <= limits.max_total_allocation_bytes,
            "snapshot allocation bound",
        )?;
        let bytes = a.bytes.prefix(end).ok_or("snapshot truncated")?;
        wire::ensure(
            allocations
                .insert(
                    id.to_owned(),
                    super::shared::Allocation {
                        bytes,
                        arena: a.arena.clone(),
                        witness: a.witness.clone(),
                    },
                )
                .is_none(),
            "snapshot duplicate allocation",
        )?;
    }
    let mut available = BTreeMap::new();
    for name in ["ledger", "envelope", "overlay"] {
        let value = &record["old"][name];
        available.insert(key(&wire::reference(value))?, encode(value));
    }
    let overlay = &record["old"]["overlay"];
    let refs = overlay["controls"].as_array().ok_or("snapshot controls")?;
    wire::ensure(refs.len() <= limits.max_objects, "snapshot control count")?;
    let mut loose = BTreeMap::new();
    let mut bytes = 0usize;
    for reference in refs
        .iter()
        .chain(std::iter::once(&wire::reference(overlay)))
    {
        let k = key(reference)?;
        if !available.contains_key(&k) && !current.loose.contains_key(&k.0) {
            let raw = snapshot_control(current, reference, limits)
                .map_err(|_| "snapshot retained locator control")?;
            available.insert(k.clone(), raw);
        }
        let raw = available
            .get(&k)
            .or_else(|| current.loose.get(&k.0))
            .ok_or("snapshot retained control")?;
        bytes = bytes
            .checked_add(raw.len())
            .ok_or("snapshot control bytes")?;
        wire::ensure(
            raw.len() as u64 == k.1
                && hash(raw) == k.0
                && bytes <= limits.semantic.max_total_json_bytes,
            "snapshot control commitment",
        )?;
        crate::package::parse_canonical_json(raw, limits.semantic.json)
            .map_err(|_| "snapshot canonical control")?;
        wire::ensure(
            loose.insert(k.0, raw.clone()).is_none(),
            "snapshot duplicate control",
        )?;
    }
    Ok(OriginalPackage {
        manifest: current.manifest.clone(),
        head: record["old"]["head"].clone(),
        commit: record["old"]["commit"].clone(),
        loose,
        allocations,
    })
}

// A completed successor has moved prior loose controls into its immutable
// append. Read only exact requested digest/length commitments from complete
// current frames; never interpret or truncate an incomplete current tail.
fn retained_controls(
    current: &OriginalPackage,
    wanted: &BTreeSet<wire::Key>,
    limits: Limits,
) -> wire::Result<BTreeMap<wire::Key, Value>> {
    let mut found = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(found);
    }
    let mut total = 0u64;
    let mut frames = 0usize;
    let mut parsed = 0usize;
    for a in current.allocations.values() {
        total = total
            .checked_add(a.bytes.len() as u64)
            .ok_or("current byte overflow")?;
        wire::ensure(
            a.bytes.len() as u64 <= limits.max_allocation_bytes
                && total <= limits.max_total_allocation_bytes,
            "current allocation budget",
        )?;
        let mut at = 0usize;
        while at < a.bytes.len() {
            frames = frames.checked_add(1).ok_or("current frame overflow")?;
            wire::ensure(frames <= limits.max_frames, "current frame budget")?;
            let Some(header) = a
                .bytes
                .get(at..at.checked_add(16).ok_or("current frame overflow")?)
            else {
                break;
            };
            wire::ensure(
                &header[..8] == b"PS2PKD01"
                    && header[9..12] == [0, 0, 0]
                    && ((a.arena == "data" && header[8] <= 2)
                        || (a.arena == "metadata" && matches!(header[8], 0 | 3))),
                "current frame header",
            )?;
            let length = u32::from_le_bytes(
                header[12..16]
                    .try_into()
                    .map_err(|_| "current frame length")?,
            ) as usize;
            let end = at
                .checked_add(16)
                .and_then(|n| n.checked_add(length))
                .ok_or("current frame overflow")?;
            let Some(body) = a.bytes.get(at + 16..end) else {
                break;
            };
            if matches!(header[8], 1 | 2) && wanted.iter().any(|(_, len)| *len == length as u64) {
                let k = (hash(body), length as u64);
                if wanted.contains(&k) && !found.contains_key(&k) {
                    parsed = parsed
                        .checked_add(length)
                        .ok_or("retained control overflow")?;
                    wire::ensure(
                        parsed <= limits.semantic.max_total_json_bytes,
                        "retained control byte budget",
                    )?;
                    let value = crate::package::parse_canonical_json(body, limits.semantic.json)
                        .map_err(|_| "retained canonical control")?;
                    found.insert(k, value);
                }
            }
            at = end;
        }
    }
    wire::ensure(found.len() == wanted.len(), "missing retained control")?;
    Ok(found)
}
