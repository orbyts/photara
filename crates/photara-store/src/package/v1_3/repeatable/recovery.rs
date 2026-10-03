//! Original reconstruction and exact retry dispatch. Neither operation grants
//! writable admission or turns selected receipt JSON into a Saved acknowledgment.
use super::{
    Attempt, Limits, OriginalPackage, RepeatablePlan, VerifiedOriginal, compile, layout,
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
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
) -> Result<OriginalPackage, PackageError> {
    reconstruct(current, record, limits).map_err(|_| PackageError::Integrity)
}
#[expect(
    clippy::too_many_lines,
    reason = "Original commitment reconstruction and bounded retained-control recovery kept together"
)]
fn reconstruct(
    current: &OriginalPackage,
    record: &Value,
    limits: Limits,
) -> wire::Result<OriginalPackage> {
    wire::ensure(
        record["original_codec"] == "photara.codec.ps2-repeatable-admission-v1"
            && record["payload_recipe"]["codec"] == "photara.codec.ps2-repeatable-layout-v1",
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
            hash(prefix) == range["original_sha256"],
            "original prefix digest",
        )?;
        total = total
            .checked_add(end as u64)
            .ok_or("prefix total overflow")?;
        wire::ensure(
            total <= limits.max_total_allocation_bytes,
            "prefix total budget",
        )?;
        let copy = super::Allocation {
            bytes: prefix.to_vec(),
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
    let pool = layout::original_objects(&old, &attempt.planner, limits)?;
    let mut available = pool;
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
        let v = available.get(&k).ok_or("preserved original control")?;
        old.loose.insert(k.0, encode(v));
    }
    old.loose.insert(hash(&encode(overlay)), encode(overlay));
    Ok(old)
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
