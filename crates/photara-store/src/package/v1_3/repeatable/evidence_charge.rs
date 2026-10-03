//! Combined standing-control accounting, without choosing a persistent evidence
//! namespace, journal encoding, retention policy or production admission authority.
use super::{RepeatablePlan, wire};
use crate::package::PackageError;
use std::collections::BTreeSet;

/// Native identities must come from pinned registration. Reservation slots are
/// local pre-effect plan coordinates, not filenames or portable wire identities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceAllocationId {
    Registered { device: u64, inode: u64 },
    Reserved { slot: u64 },
}
/// One distinct physical journal/receipt/index/scratch allocation. Equal content
/// in different files is still charged twice. Unknown/unlisted roles cannot be
/// justified by this comparison: exhaustive inventory remains an adapter duty.
#[derive(Clone, Copy)]
pub struct EvidenceAllocationCharge {
    pub identity: EvidenceAllocationId,
    pub current_extent: u64,
    pub maximum_extent: u64,
    pub registered_charge: u64,
}
/// Arithmetic result only. It cannot establish complete native registration,
/// mint writable admission, permit evidence cleanup, or acknowledge durability.
pub struct EvidenceCharge {
    total_peak: u64,
    evidence_peak: u64,
    allocation_count: u64,
    remaining_standing: u64,
}
impl EvidenceCharge {
    #[must_use]
    pub fn total_peak(&self) -> u64 {
        self.total_peak
    }
    #[must_use]
    pub fn evidence_peak(&self) -> u64 {
        self.evidence_peak
    }
    #[must_use]
    pub fn allocation_count(&self) -> u64 {
        self.allocation_count
    }
    #[must_use]
    pub fn remaining_standing(&self) -> u64 {
        self.remaining_standing
    }
}
fn round(value: u64, unit: u64) -> Result<u64, PackageError> {
    value
        .checked_add(unit.checked_sub(1).ok_or(PackageError::Integrity)?)
        .and_then(|v| v.checked_div(unit))
        .and_then(|v| v.checked_mul(unit))
        .ok_or(PackageError::Limit)
}
/// Counts all supplied retained and prospective evidence against the *already
/// charged* original standing allowance. Namespace growth must have an explicit
/// independently bounded charge; it is not inferred to be free. No subtraction
/// for prospective deletion, package receipt inclusion, or duplicate contents.
///
/// Extents must use the original independently qualified charging policy;
/// these numeric inputs cannot establish that qualification themselves.
/// This closes arithmetic preflight only. A production adapter still needs a
/// reviewed persistent evidence namespace/encoding and trusted exhaustive capture.
/// # Errors
/// Refuses duplicate physical identities/reservations, unregistered current
/// extents, shrinking envelopes, overflow, or combined original byte/count limits.
pub fn check_evidence_charge(
    plan: &RepeatablePlan,
    allocations: &[EvidenceAllocationCharge],
    namespace_charge: u64,
) -> Result<EvidenceCharge, PackageError> {
    let number = |v| wire::number(v).map_err(|_| PackageError::Integrity);
    let unit = plan
        .attempt
        .charge_unit
        .parse::<u64>()
        .map_err(|_| PackageError::Integrity)?;
    if unit == 0 {
        return Err(PackageError::Integrity);
    }
    let standing = number(&plan.original["old"]["ledger"]["standing_control"])?;
    let bound = number(&plan.original["control_bounds"]["aggregate_control_bytes"])?;
    let count_bound = number(&plan.original["control_bounds"]["aggregate_control_count"])?;
    let count = plan
        .control_role_peak
        .checked_add(u64::try_from(allocations.len()).map_err(|_| PackageError::Limit)?)
        .ok_or(PackageError::Limit)?;
    if count > count_bound || !namespace_charge.is_multiple_of(unit) {
        return Err(PackageError::Limit);
    }
    let mut identities = BTreeSet::new();
    let mut evidence = namespace_charge;
    for allocation in allocations {
        if !identities.insert(allocation.identity)
            || allocation.maximum_extent < allocation.current_extent
        {
            return Err(PackageError::Integrity);
        }
        match allocation.identity {
            EvidenceAllocationId::Registered { inode, .. } if inode != 0 => {
                if allocation.registered_charge < round(allocation.current_extent, unit)? {
                    return Err(PackageError::Integrity);
                }
            }
            EvidenceAllocationId::Reserved { .. }
                if allocation.current_extent == 0
                    && allocation.registered_charge == 0
                    && allocation.maximum_extent > 0 => {}
            _ => return Err(PackageError::Integrity),
        }
        let charge = round(allocation.maximum_extent, unit)?.max(allocation.registered_charge);
        evidence = evidence.checked_add(charge).ok_or(PackageError::Limit)?;
    }
    let total = plan.peak.checked_add(evidence).ok_or(PackageError::Limit)?;
    if total > bound || bound > standing {
        return Err(PackageError::Limit);
    }
    Ok(EvidenceCharge {
        total_peak: total,
        evidence_peak: evidence,
        allocation_count: count,
        remaining_standing: standing - total,
    })
}
