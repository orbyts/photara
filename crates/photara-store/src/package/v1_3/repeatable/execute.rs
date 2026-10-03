//! Ordered execution of precompiled, original-bound bytes. Native admission is
//! intentionally unavailable until the independently qualified registrar exists.
use super::{
    plan::{RepeatablePlan, SelectedStage, journal_frame},
    wire::{encode, hash},
};
use crate::package::planning::io::RegisteredCooperativeLease;
use serde_json::Value;

/// Every effect must retain the opaque lease and recheck safe native pins. An
/// adapter must report uncertainty even when a failed barrier followed a write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectFailure {
    NotPerformed,
    OutcomeUnknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionError {
    InvalidObservation,
    Authorization,
    Effect(EffectFailure),
}
/// Observed bytes, never a durable acknowledgment or permission.
pub struct Observation {
    pub head: Value,
    pub allocations: std::collections::BTreeMap<String, Vec<u8>>,
}
/// Trusted adapter seam: implementations must validate actual pins, registered
/// charge policy, host request/provenance authorization, and the held lease.
/// Pure caller-supplied receipt strings are never authorization evidence.
pub trait RepeatableIo {
    type Lease;
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn authorize(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        original: &Value,
        receipt: &Value,
    ) -> Result<(), ExecutionError>;
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn observe(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
    ) -> Result<Observation, ExecutionError>;
    /// Revalidate the exact selected commit/control set and required journal or
    /// original-receipt evidence for the observed stage. Unknown control roles
    /// and unrelated journal frames must never be silently adopted.
    /// # Errors
    /// Refuses any selected proof or durable evidence inconsistency.
    fn verify_progress(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &RepeatablePlan,
        stage: Option<usize>,
    ) -> Result<(), ExecutionError>;
    /// Reissue barriers for the observed selected controls/HEAD, required journal
    /// and completed allocation prefixes. Reopening equal bytes alone never
    /// proves a barrier that may have failed before interruption. Reconcile only
    /// exact operation-owned obsolete controls to the original coexistence bound.
    /// # Errors
    /// Preserves uncertainty if any required reflush or owned cleanup fails.
    fn stabilize_progress(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), EffectFailure>;
    /// Independently inspect the complete prospective closure using the shared
    /// reader, exact generated controls and original registered observations.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn verify_prospective(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &RepeatablePlan,
    ) -> Result<(), ExecutionError>;
    /// No-replace control installation, file barriers, exact HEAD comparison,
    /// atomic replacement and directory barrier, all completed on success.
    /// Before returning, reconcile exact predecessor control roles so subsequent
    /// stages stay inside the checked adjacent-snapshot control bound.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn select(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        expected_head: &Value,
        stage: &SelectedStage,
    ) -> Result<(), EffectFailure>;
    /// Append the exact original journal frame idempotently and barrier it.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn journal(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        frame: &Value,
    ) -> Result<(), EffectFailure>;
    /// Append at the checked offset, with no replacement/truncation, then barrier.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn append(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        allocation: &str,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), EffectFailure>;
    /// Reopen and independently verify the selected closure and all original
    /// resource/registration identities before recording its original receipt.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn verify_selected(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), ExecutionError>;
    /// Preserve exact original-ID receipt. Does not mint a production Saved token.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn record_receipt(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        original: &Value,
        receipt: &Value,
    ) -> Result<(), EffectFailure>;
    /// Remove only obsolete operation-owned control roles recorded in this plan,
    /// preserving immutable packed history, and barrier their directory.
    /// # Errors
    /// Refuses failed authorization, changed observations, or incomplete effects.
    fn cleanup_controls(
        &mut self,
        lease: &RegisteredCooperativeLease<Self::Lease>,
        plan: &RepeatablePlan,
    ) -> Result<(), EffectFailure>;
}
/// Successful protocol observation, deliberately distinct from a production
/// Saved receipt: qualification and public writer construction remain gated.
pub struct Completed {
    receipt: Value,
    original_sha256: String,
}
impl Completed {
    #[must_use]
    pub fn receipt(&self) -> &Value {
        &self.receipt
    }
    #[must_use]
    pub fn original_sha256(&self) -> &str {
        &self.original_sha256
    }
}
fn effect<T>(r: Result<T, EffectFailure>) -> Result<T, ExecutionError> {
    r.map_err(ExecutionError::Effect)
}
/// Resume only the exact original plan. Any failure returns immediately; an
/// uncertain effect is never replaced by a later definite failure or success.
/// # Errors
/// Refuses unrelated selectors, altered/excess allocation suffixes, changed
/// authorization, incomplete independent closure proof, or any failed effect.
pub fn execute<I: RepeatableIo>(
    io: &mut I,
    lease: &RegisteredCooperativeLease<I::Lease>,
    plan: &RepeatablePlan,
) -> Result<Completed, ExecutionError> {
    io.authorize(lease, &plan.original, &plan.prepared.receipt)?;
    io.verify_prospective(lease, plan)?;
    let observed = io.observe(lease)?;
    let current = if observed.head == plan.old.head {
        None
    } else {
        Some(
            plan.stages
                .iter()
                .position(|s| s.head == observed.head)
                .ok_or(ExecutionError::InvalidObservation)?,
        )
    };
    io.verify_progress(lease, plan, current)?;
    if observed.allocations.len() != plan.files.len() {
        return Err(ExecutionError::InvalidObservation);
    }
    for (id, target) in &plan.files {
        let bytes = observed
            .allocations
            .get(id)
            .ok_or(ExecutionError::InvalidObservation)?;
        let start = plan.old.allocations[id].bytes.len();
        // Only original-bound partial exact append bytes can be resumed. The
        // whole allocation remains charged by the original reservation.
        if bytes.len() < start || bytes.len() > target.len() || !target.starts_with(bytes) {
            return Err(ExecutionError::InvalidObservation);
        }
        let allowed = match current {
            None | Some(0) => start,
            Some(1 | 2) => *plan.payload_ends.get(id).unwrap_or(&start),
            _ => target.len(),
        };
        if bytes.len() > allowed {
            return Err(ExecutionError::InvalidObservation);
        }
        if current.is_some_and(|n| n >= 2)
            && bytes.len() < *plan.payload_ends.get(id).unwrap_or(&start)
        {
            return Err(ExecutionError::InvalidObservation);
        }
        if current.is_some_and(|n| n >= 4) && bytes.len() != target.len() {
            return Err(ExecutionError::InvalidObservation);
        }
    }
    if let Some(stage) = current {
        effect(io.stabilize_progress(lease, plan, stage))?;
    }
    let mut head = observed.head;
    let mut lengths = observed
        .allocations
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for next in 0..plan.stages.len() {
        if current.is_some_and(|n| n >= next) {
            continue;
        }
        match next {
            1 => effect(io.journal(lease, &journal_frame(&plan.prepared.receipt)))?,
            2 | 4 => {
                for (id, target) in &plan.files {
                    let end = if next == 2 {
                        *plan.payload_ends.get(id).unwrap_or(&lengths[id])
                    } else {
                        target.len()
                    };
                    let start = lengths[id];
                    if start > end {
                        return Err(ExecutionError::InvalidObservation);
                    }
                    // Even an already complete observed suffix may follow a
                    // failed flush. An empty append still reissues its barrier.
                    if end > plan.old.allocations[id].bytes.len() {
                        effect(io.append(lease, id, start, &target[start..end]))?;
                        lengths.insert(id.clone(), end);
                    }
                }
            }
            6 => {
                io.verify_selected(lease, plan, 5)?;
                effect(io.record_receipt(lease, &plan.original, &plan.prepared.receipt))?;
                effect(io.cleanup_controls(lease, plan))?;
            }
            _ => {}
        }
        effect(io.select(lease, &head, &plan.stages[next]))?;
        head = plan.stages[next].head.clone();
    }
    // A receipt already recorded before interruption never substitutes for the
    // selected candidate's current full verification.
    io.verify_selected(lease, plan, 6)?;
    effect(io.record_receipt(lease, &plan.original, &plan.prepared.receipt))?;
    Ok(Completed {
        receipt: plan.prepared.receipt.clone(),
        original_sha256: hash(&encode(&plan.original)),
    })
}
