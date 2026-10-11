//! V4 is entered only with an independently activated local checkpoint base.
//! Raw persisted planner fields retain their new meanings; normalization is
//! private and never modifies an old original or public `PlannerInputs` DTO.
use super::ActivatedCheckpoint;
use crate::package::PackageError;
use crate::package::v1_3::repeatable::{
    Attempt, Limits, RepeatablePlan, VerifiedOriginal, plan, wire,
};
use serde_json::{Value, json};

impl ActivatedCheckpoint {
    /// Prepare the original-bound journal completion after independently
    /// verifying the clean selected package. Journal barriers remain required.
    /// # Errors
    /// Refuses a different local base, selected plan, scope or original intent.
    pub fn prepare_receipt(
        &self,
        journal: &super::super::journal::Journal,
        current: &VerifiedOriginal,
        plan: &RepeatablePlan,
        intent: crate::package::PackageUuid,
        identity: super::super::journal::JournalRecordIdentity,
        limits: super::super::journal::JournalLimits,
    ) -> Result<super::super::journal::JournalAppend, PackageError> {
        self.base().check_header(journal.header())?;
        journal.snapshot_receipt(current, plan, intent, identity, limits)
    }
    /// Authenticate an original local retry without adding it to the new
    /// segment or resurrecting retired undo history. This is not Saved.
    /// # Errors
    /// Refuses changed intent, owner/action, requested archive or package receipt.
    pub fn original_mutation(
        &self,
        intent: &Value,
        owner: &Value,
        action: &Value,
        frames: crate::package::v1_3::FrameLimits,
        reader: &crate::package::v1_3::ReaderLimits,
    ) -> Result<Option<Value>, PackageError> {
        self.seed
            .original_mutation(&self.clean, intent, owner, action, frames, reader)
    }
    /// Compile a new graph admission from the selected snapshot base. This pure
    /// method performs no admission, journal append, authority check or effects.
    /// # Errors
    /// Refuses a different selected base/scope, known archived operation ID,
    /// changed HEAD corridor, malformed recipe, Core command or finite bound.
    pub fn compile_operation(
        &self,
        current: &VerifiedOriginal,
        intent: &Value,
        receipt: &Value,
        raw: &Value,
        limits: Limits,
    ) -> Result<RepeatablePlan, PackageError> {
        self.compile_operation_inner(current, intent, receipt, raw, limits)
            .map_err(|_| PackageError::Integrity)
    }
    pub(crate) fn compile_operation_inner(
        &self,
        current: &VerifiedOriginal,
        intent: &Value,
        receipt: &Value,
        raw: &Value,
        limits: Limits,
    ) -> wire::Result<RepeatablePlan> {
        wire::ensure(
            current.scope == self.clean.scope
                && current.package.manifest == self.clean.package.manifest
                && current.charge_unit == self.clean.charge_unit,
            "checkpoint registered scope",
        )?;
        self.seed
            .check_new_operation(wire::text(&intent["operation_id"])?)?;
        wire::ensure(
            raw["planner"]["codec"] == "photara.codec.ps2-repeatable-layout-v4",
            "snapshot continuation codec",
        )?;
        let mut normalized = raw.clone();
        normalized["planner"]["codec"] = json!("photara.codec.ps2-repeatable-layout-v3");
        let head = wire::hash(&wire::encode(&current.package.head));
        for corridor in normalized["planner"]["corridors"]
            .as_array_mut()
            .ok_or("corridors")?
        {
            wire::ensure(
                corridor["base_head_sha256"] == head && corridor.get("original_sha256").is_none(),
                "checkpoint HEAD corridor",
            )?;
            current
                .package
                .allocations
                .get(wire::text(&corridor["allocation_id"])?)
                .ok_or("registered corridor")?;
            corridor
                .as_object_mut()
                .ok_or("corridor")?
                .remove("base_head_sha256");
            corridor["original_sha256"] = json!(head);
        }
        let attempt: Attempt =
            serde_json::from_value(normalized).map_err(|_| "v4 planner fields")?;
        plan::compile_context(
            current,
            intent,
            receipt,
            attempt,
            limits,
            None,
            Some((self.base().value(), raw)),
        )
    }
    /// Exact regeneration of a retained v4 original from its independently
    /// verified old package. No new ID or recipe is chosen during retry.
    /// # Errors
    /// Refuses changed original, base edge, request/receipt or bounded plan.
    pub fn restore_operation(
        &self,
        old: &VerifiedOriginal,
        intent: &Value,
        receipt: &Value,
        original: &[u8],
        limits: Limits,
    ) -> Result<RepeatablePlan, PackageError> {
        let record = crate::package::parse_canonical_json(original, limits.semantic.json)?;
        let plan = self.compile_operation(
            old,
            intent,
            receipt,
            &record["payload_recipe"]["planner"],
            limits,
        )?;
        if plan.original_bytes() != original {
            return Err(PackageError::Integrity);
        }
        Ok(plan)
    }
}
