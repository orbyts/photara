//! Original reconstruction and exact retry dispatch. Neither operation grants
//! writable admission or turns selected receipt JSON into a Saved acknowledgment.
use super::{
    Attempt, Limits, OriginalPackage, RepeatablePlan, VerifiedOriginal, compile, layout,
    wire::{self, encode, hash, key, number},
};
use crate::package::PackageError;
use serde_json::Value;
use std::collections::BTreeMap;

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
    for r in refs {
        let k = key(r)?;
        let v = available.get(&k).ok_or("preserved original control")?;
        old.loose.insert(k.0, encode(v));
    }
    old.loose.insert(hash(&encode(overlay)), encode(overlay));
    Ok(old)
}
