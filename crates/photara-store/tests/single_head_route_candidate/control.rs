//! Disposable simultaneous-control allocation model, not provider qualification.
//! Caller authenticates every selected role and all preplanned payload/phase bytes.
use super::{joined, packed};
use serde_json::Value;
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, &'static str>;
const BLOCK: u64 = 4096;
fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
fn number(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("control decimal")?;
    let n = s.parse::<u64>().map_err(|_| "control decimal range")?;
    ensure(n.to_string() == s, "control canonical decimal")?;
    Ok(n)
}
fn encoded(v: &Value) -> Result<Vec<u8>> {
    photara_core::canonical_json(v).map_err(|_| "control canonical encoding")
}
fn rounded(bytes: usize) -> Result<u64> {
    let n = u64::try_from(bytes).map_err(|_| "control length range")?;
    n.checked_add(BLOCK - 1)
        .and_then(|n| (n / BLOCK).checked_mul(BLOCK))
        .ok_or("control allocation overflow")
}
fn insert(
    files: &mut BTreeMap<(String, String), Vec<u8>>,
    role: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    ensure(!bytes.is_empty(), "empty immutable control")?;
    let key = (role.to_owned(), packed::hash(&bytes));
    if let Some(prior) = files.get(&key) {
        ensure(*prior == bytes, "control same-hash content mismatch")?;
    } else {
        files.insert(key, bytes);
    }
    Ok(())
}
pub(super) fn peak(a: &joined::World, b: &joined::World) -> Result<(u64, u64)> {
    ensure(
        a.manifest == b.manifest,
        "control original manifest changed",
    )?;
    let mut immutable = BTreeMap::new();
    insert(&mut immutable, "manifest", encoded(&a.manifest)?)?;
    for world in [a, b] {
        insert(&mut immutable, "commit", encoded(&world.commit)?)?;
        for (name, raw) in &world.loose {
            ensure(
                *name == packed::hash(raw),
                "control loose content-address mismatch",
            )?;
            insert(&mut immutable, "loose", raw.clone())?;
        }
    }
    // Mutable HEAD versions occupy separate roles even if their bytes coincide.
    let mut bytes = 0u64;
    for raw in immutable.values() {
        bytes = bytes
            .checked_add(rounded(raw.len())?)
            .ok_or("control sum overflow")?;
    }
    for world in [a, b] {
        let head = encoded(&world.head)?;
        ensure(head.len() <= 4096, "control staged HEAD bound")?;
        bytes = bytes
            .checked_add(rounded(head.len())?)
            .ok_or("control HEAD sum overflow")?;
    }
    // Original-token intent, staged HEAD and directory bookkeeping coexist.
    bytes = bytes
        .checked_add(3 * BLOCK)
        .ok_or("control corridor overflow")?;
    let count = u64::try_from(immutable.len())
        .map_err(|_| "control count range")?
        .checked_add(5)
        .ok_or("control count overflow")?;
    Ok((bytes, count))
}
/// Check every adjacent preplanned transition before admission, without effects.
/// Loose controls are immutable files shared only by exact role/hash identity.
/// `candidates` must include all supported intermediate and cleanup selections.
pub(super) fn preflight(
    old: &joined::World,
    candidates: &[joined::World],
    original: &Value,
) -> Result<()> {
    ensure(
        !candidates.is_empty() && candidates.len() <= 16,
        "control bounded complete candidate sequence",
    )?;
    let cap = number(&original["maximum_control_bytes"])?;
    let count = number(&original["maximum_control_count"])?;
    let standing = number(&original["old"]["ledger"]["standing_control"])?;
    ensure(
        cap > 0 && cap <= standing && (1..=64).contains(&count),
        "control original finite pool",
    )?;
    let mut previous = old;
    for candidate in candidates {
        let (required, roles) = peak(previous, candidate)?;
        ensure(
            required <= cap && required <= standing,
            "control simultaneous rounded byte cap",
        )?;
        ensure(roles <= count, "control simultaneous role count")?;
        previous = candidate;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn world(version: u64) -> joined::World {
        let raw = encoded(&json!({"original":true})).unwrap();
        joined::World {
            manifest: json!({"manifest":1}),
            head: json!({"head":version}),
            commit: json!({"commit":version}),
            allocations: BTreeMap::new(),
            loose: BTreeMap::from([(packed::hash(&raw), raw)]),
            source: BTreeMap::new(),
            source_witnesses: BTreeMap::new(),
        }
    }
    fn original(cap: u64, count: u64) -> Value {
        json!({"maximum_control_bytes":cap.to_string(),"maximum_control_count":count.to_string(),"old":{"ledger":{"standing_control":"131072"}}})
    }
    #[test]
    fn control_exact_rounded_bound_and_shared_immutable_roles() -> Result<()> {
        let old = world(1);
        let next = world(2);
        let (bytes, count) = peak(&old, &next)?;
        // Manifest + two commits + one shared loose + two HEADs + three corridor roles.
        ensure(
            (bytes, count) == (9 * 4096, 9),
            "unexpected fixture coexistence",
        )?;
        preflight(&old, std::slice::from_ref(&next), &original(bytes, count))?;
        ensure(
            preflight(
                &old,
                std::slice::from_ref(&next),
                &original(bytes - 1, count),
            )
            .is_err(),
            "under byte cap",
        )?;
        ensure(
            preflight(
                &old,
                std::slice::from_ref(&next),
                &original(bytes, count - 1),
            )
            .is_err(),
            "under role cap",
        )?;
        let third = world(3);
        preflight(&old, &[next, third], &original(bytes, count))
    }
    #[test]
    fn control_unknown_large_role_and_forged_address_refuse() -> Result<()> {
        let old = world(1);
        let mut next = world(2);
        let huge = vec![b'x'; 131_072];
        next.loose.insert(packed::hash(&huge), huge);
        ensure(
            preflight(&old, &[next], &original(131_072, 24)).is_err(),
            "unknown large control exceeds pool",
        )?;
        let mut forged = world(2);
        forged.loose.insert("0".repeat(64), b"different".to_vec());
        ensure(
            preflight(&old, &[forged], &original(131_072, 24)).is_err(),
            "forged immutable address",
        )?;
        let mut head = world(2);
        head.head = json!({"oversized":"x".repeat(4096)});
        ensure(
            preflight(&old, &[head], &original(131_072, 24)).is_err(),
            "staged HEAD exceeded allowance",
        )?;
        Ok(())
    }
}
