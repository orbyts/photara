//! Exact completed-operation proof within the selected local segment only.
//! The retained checkpoint base terminates this walk; archive ancestry is not replayed.
use super::super::{VerifiedOriginal, recovery, restart, shared::OriginalPackage, wire};
use super::{ActivationLimits, CheckpointIdentity, CheckpointRecords, ReopenState, check};
use crate::package::{PackageError, v1_3::AllocationInspection};
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};

pub(crate) struct Source {
    pub selector: Vec<u8>,
    pub base: Vec<u8>,
    pub header: Value,
    pub segment: Vec<u8>,
    pub records: Arc<dyn CheckpointRecords>,
}
impl Source {
    pub(crate) fn state(&self, current: VerifiedOriginal) -> ReopenState {
        ReopenState {
            selector: self.selector.clone(),
            base: self.base.clone(),
            header: self.header.clone(),
            segment: self.segment.clone(),
            current,
            records: self.records.clone(),
        }
    }
    fn completed<I: AllocationInspection>(
        &self,
        package: &OriginalPackage,
        context: &restart::RestartContext<'_, I>,
        memo: &mut restart::CompiledPlans,
        journal: &super::super::journal::Journal,
        scope: &Value,
    ) -> Result<Option<VerifiedOriginal>, PackageError> {
        // The executor has already independently checked the exact clean
        // current closure. Reuse only that proof after fresh package/provider
        // checks and after binding its Original to this validated local segment.
        if let Some(verified) = memo.completed_current(package, context)? {
            let completed = verified
                .completed_original
                .as_ref()
                .ok_or(PackageError::Integrity)?;
            let record = journal
                .records()
                .iter()
                .find(|record| {
                    record.value()["kind"] == "CheckpointIntent"
                        && wire::reference(&record.value()["body"]["original"])
                            == serde_json::to_value(completed).expect("typed reference")
                })
                .ok_or(PackageError::Integrity)?;
            check(
                record.value()["body"]["original"]["payload_recipe"]["checkpoint_base"]
                    == self.header["base"]
                    && record.value()["body"]["original"]["scope"] == *scope,
            )?;
            return Ok(Some(verified));
        }
        Ok(None)
    }
    pub(crate) fn verify<I: AllocationInspection>(
        &self,
        package: &OriginalPackage,
        context: &restart::RestartContext<'_, I>,
        memo: &mut restart::CompiledPlans,
        identity: &CheckpointIdentity,
        limits: ActivationLimits<'_>,
    ) -> Result<VerifiedOriginal, PackageError> {
        let scope = serde_json::json!({"profile":context.registration.profile,"incarnation":context.registration.incarnation.to_string(),"directory":{"device":context.directory.device.to_string(),"inode":context.directory.inode.to_string()}});
        let (_, base, journal, original) = super::activate::custody_records(
            &self.selector,
            &self.base,
            &self.header,
            &self.segment,
            self.records.as_ref(),
            &scope,
            identity,
            limits,
        )?;
        if let Some(verified) = self.completed(package, context, memo, &journal, &scope)? {
            return Ok(verified);
        }
        let mut current = package.clone();
        let mut steps = Vec::new();
        let mut seen = BTreeSet::new();
        loop {
            let envelope = control(
                &current,
                &current.commit["root_set"]["placement"]["accounting"],
                limits,
            )?;
            let hold_sha = envelope["holds"]["sha256"]
                .as_str()
                .ok_or(PackageError::Record)?;
            let Some(bytes) = current.loose.get(hold_sha) else {
                break;
            };
            let hold = crate::package::parse_canonical_json(bytes, limits.reader.json)?;
            check(
                wire::hash(bytes) == hold_sha
                    && hold["schema"]["id"] == "photara.storage.hold-leaf"
                    && hold["schema"]["version"] == 1
                    && hold["count"] == "1",
            )?;
            let entries = hold["entries"].as_array().ok_or(PackageError::Record)?;
            check(entries.len() == 1 && steps.len() < context.max_originals)?;
            let original = control(&current, &entries[0]["original"], limits)?;
            check(
                original["original_codec"] == "photara.codec.ps2-repeatable-admission-v4"
                    && original["payload_recipe"]["checkpoint_base"] == self.header["base"]
                    && seen.insert(wire::hash(&wire::encode(&original))),
            )?;
            let (index, record) = journal
                .records()
                .iter()
                .enumerate()
                .find(|(_, r)| {
                    r.value()["kind"] == "CheckpointIntent"
                        && r.value()["body"]["original"] == original
                })
                .ok_or(PackageError::Integrity)?;
            check(index > 0)?; // The first record must remain the exact BaseReceipt.
            let end = journal
                .record_end(&journal.records()[index - 1].value()["record_id"])
                .ok_or(PackageError::Integrity)?;
            if let Some((_, _, later)) = steps.last() {
                check(end < *later)?;
            }
            let old = recovery::reconstruct_shared_original(&current, &original, limits.planner)?;
            steps.push((current, record.value()["body"].clone(), end));
            current = old;
        }
        let mut verified =
            restart::verify_anchor_prefix(&current, base.value(), &original, context)?;
        // Even an empty suffix must bind to the registered local base/selector.
        let mut base_state = self.state(verified.shared_clone());
        if let Some((_, _, end)) = steps.last() {
            base_state.segment.truncate(*end);
        }
        let mut activated = super::activate::reopen_state(base_state, identity, limits)?;
        for (package, body, end) in steps.into_iter().rev() {
            let mut state = self.state(verified.shared_clone());
            state.segment.truncate(end);
            activated = super::activate::reopen_state(state, identity, limits)?;
            let plan = activated.restore_operation(
                &verified,
                &body["semantic_intent"],
                &body["operation_receipt"],
                &wire::encode(&body["original"]),
                limits.planner,
            )?;
            verified = restart::verify_plan_prefix(&package, &plan, context)?;
        }
        // Keep the independent base proof mandatory even with no operations.
        check(activated.base().value()["checkpoint"] != Value::Null)?;
        Ok(verified)
    }
}
fn control(
    package: &OriginalPackage,
    reference: &Value,
    limits: ActivationLimits<'_>,
) -> Result<Value, PackageError> {
    let reference = crate::package::v1_3::selected::json_ref(reference)?;
    let bytes = package
        .loose
        .get(reference.sha256.as_str())
        .ok_or(PackageError::Integrity)?;
    check(
        bytes.len() as u64 == reference.byte_length.get()
            && wire::hash(bytes) == reference.sha256.as_str(),
    )?;
    crate::package::parse_canonical_json(bytes, limits.reader.json)
}
