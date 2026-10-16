//! Snapshot recovery starts at the independently selected included coordinate.
//! Historical receipts/undo evidence are requested through authenticated roots.
use super::{
    Arc, BTreeMap, BTreeSet, CheckpointSeed, Journal, JournalRecord, Limits, MutationEvidence,
    Objects, PackageError, Value, VerifiedOriginal, check, checked, core, id, json, n,
    operation_target, plan, record_link, wire,
};
use crate::package::v1_3::repeatable::checkpoint::{LocalBase, lookup::History};

fn evidence(record: JournalRecord) -> Result<MutationEvidence, PackageError> {
    let pool = |name: &str| -> Result<Objects, PackageError> {
        record.value["body"]["patch"][name]
            .as_array()
            .ok_or(PackageError::Record)?
            .iter()
            .map(|row| Ok((checked(wire::key(&row["reference"]))?, row["value"].clone())))
            .collect()
    };
    Ok(MutationEvidence {
        before: pool("removed")?,
        after: pool("added")?,
        record,
        // Old prefix digests are deliberately unavailable, never invented as
        // checkpoint evidence. These entries are already included undo pins.
        accepted: Value::Null,
        journal_inclusion: Value::Null,
        undone_by: None,
        invalidated: false,
    })
}
impl CheckpointSeed {
    pub(crate) fn original_mutation(
        &self,
        current: &VerifiedOriginal,
        intent: &Value,
        owner: &Value,
        action: &Value,
        frames: crate::package::v1_3::FrameLimits,
        reader: &crate::package::v1_3::ReaderLimits,
    ) -> Result<Option<Value>, PackageError> {
        let Some((record, _)) = self.retained_mutation(
            current,
            intent["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
            frames,
            reader,
        )?
        else {
            return Ok(None);
        };
        let body = &record["body"];
        check(
            body["semantic_intent"] == *intent
                && body["owner"] == *owner
                && body["action"] == *action,
        )?;
        Ok(Some(record))
    }
    pub(crate) fn retained_mutation(
        &self,
        current: &VerifiedOriginal,
        operation: &str,
        frames: crate::package::v1_3::FrameLimits,
        reader: &crate::package::v1_3::ReaderLimits,
    ) -> Result<Option<(Value, String)>, PackageError> {
        let value = if let Some(prior) = self.mutations.get(operation) {
            Some((
                prior.record.value.clone(),
                prior.record.checksum().clone(),
            ))
        } else if let Some(history) = &self.history {
            history.mutation_evidence(operation)?
        } else {
            None
        };
        let Some((record, checksum)) = value else {
            return Ok(None);
        };
        let body = &record["body"];
        check(
            body["semantic_intent"]["operation_id"] == operation
                && crate::package::v1_3::reader::snapshot_receipt(
                    current,
                    id(&json!(operation))?,
                    frames,
                    reader,
                )?
                .as_ref()
                    == Some(&body["operation_receipt"]),
        )?;
        Ok(Some((record, checksum)))
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One current-inclusion and retained-undo reconstruction proof"
    )]
    pub(crate) fn recover(
        base: &LocalBase,
        history: Arc<History>,
        current: &VerifiedOriginal,
        journal: &Journal,
        frames: crate::package::v1_3::FrameLimits,
        reader: &crate::package::v1_3::ReaderLimits,
        limits: Limits,
    ) -> Result<Self, PackageError> {
        base.check_header(journal.header())?;
        check(journal.tail == super::super::JournalTail::Complete)?;
        let active = current
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?;
        let logical = checked(core::snapshot_closure(
            current,
            id(&journal.header["incarnation_id"])?,
            frames,
            reader,
            limits,
        ))?;
        let coordinate =
            serde_json::to_value(logical.coordinate()).map_err(|_| PackageError::Record)?;
        let included = n(&active.state["accepted"]["through_ordinal"])?;
        let mut mutations = BTreeMap::new();
        for target in base.value()["undo_roots"]
            .as_array()
            .ok_or(PackageError::Record)?
        {
            let operation = target["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?;
            let value = history
                .mutation(operation)?
                .ok_or(PackageError::Integrity)?;
            check(value["record_id"] == target["mutation"]["record_id"])?;
            check(
                crate::package::v1_3::reader::snapshot_receipt(
                    current,
                    id(&target["operation_id"])?,
                    frames,
                    reader,
                )?
                .as_ref()
                    == Some(&value["body"]["operation_receipt"]),
            )?;
            // The checksum is committed by both the pinned target and lookup.
            let entry = base
                .lookup_operation(
                    history.source.as_ref(),
                    &history.identity,
                    id(&target["operation_id"])?,
                    reader,
                )?
                .ok_or(PackageError::Integrity)?;
            check(entry["local"]["mutation"]["checksum"] == target["mutation"]["record_checksum"])?;
            let text = target["mutation"]["record_checksum"]
                .as_str()
                .ok_or(PackageError::Record)?;
            let mut checksum = [0u8; 32];
            for (out, pair) in checksum.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
                *out = u8::from_str_radix(
                    std::str::from_utf8(pair).map_err(|_| PackageError::Record)?,
                    16,
                )
                .map_err(|_| PackageError::Record)?;
            }
            let record = JournalRecord {
                value,
                checksum,
                frame_end: 0,
            };
            check(
                mutations
                    .insert(operation.to_owned(), evidence(record)?)
                    .is_none(),
            )?;
        }
        let mut start = if current.package.head == base.value()["package_head"] {
            Some(1)
        } else {
            None
        };
        let mut intents = BTreeMap::new();
        let mut completed = BTreeSet::new();
        let mut historical = BTreeSet::new();
        let mut target = json!({"coordinate":base.value()["coordinate"],"through":null,"accepted_frame":base.value()["accepted_frame"]});
        if !base.value()["mutation"].is_null() {
            target["through"] = json!({"record_id":base.value()["mutation"]["record_id"],"record_checksum":base.value()["mutation"]["checksum"]});
        }
        for (index, record) in journal.records.iter().enumerate().skip(1) {
            let body = &record.value["body"];
            if record.value["kind"] == "CheckpointIntent" {
                check(
                    intents
                        .insert(
                            record.value["record_id"]
                                .as_str()
                                .ok_or(PackageError::Record)?,
                            record,
                        )
                        .is_none(),
                )?;
            } else if record.value["kind"] == "Mutation" {
                let receipt = &body["operation_receipt"];
                if n(&receipt["acceptance_ordinal"])? > included {
                    continue;
                }
                check(active.receipts.iter().any(|v| v == receipt))?;
                let operation = receipt["operation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?;
                check(historical.insert(operation.to_owned()))?;
                check(
                    mutations
                        .insert(operation.to_owned(), evidence(record.clone())?)
                        .is_none(),
                )?;
                target = json!({"coordinate":body["result"],"through":record_link(record),"accepted_frame":body["accepted_frame"]});
            } else if record.value["kind"] == "CheckpointReceipt" {
                let intent = intents
                    .get(
                        body["intent_record_id"]
                            .as_str()
                            .ok_or(PackageError::Record)?,
                    )
                    .ok_or(PackageError::Integrity)?;
                check(
                    intent.checksum() == body["intent_record_checksum"]
                        && wire::hash(&wire::encode(&intent.value["body"]["original"]))
                            == body["original_sha256"]
                        && intent.value["body"]["operation_receipt"] == body["operation_receipt"],
                )?;
                let receipt = &body["operation_receipt"];
                check(
                    n(&receipt["acceptance_ordinal"])? <= included
                        && active.receipts.iter().any(|v| v == receipt),
                )?;
                let operation = receipt["operation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?;
                check(historical.contains(operation) && completed.insert(operation.to_owned()))?;
                if body["selected_head"] == current.package.head {
                    check(
                        body["selected_commit"] == current.package.commit
                            && n(&receipt["acceptance_ordinal"])? == included,
                    )?;
                    start = Some(index + 1);
                }
            }
        }
        check(completed.len() == active.receipts.len() && target["coordinate"] == coordinate)?;
        if let Some(last) = active.receipts.last() {
            check(
                target["accepted_frame"] == plan::journal_frame(last)
                    && active.state["journal_inclusion"]["through_sequence"]
                        == last["journal_sequence"]
                    && active.state["journal_inclusion"]["resulting_authored_revision"]
                        == target["coordinate"]["revision"]
                    && active.state["journal_inclusion"]["resulting_authored_sha256"]
                        == last["after"]["digest"],
            )?;
        }
        let start = start.ok_or(PackageError::Integrity)?;
        let mut order = mutations.keys().cloned().collect::<Vec<_>>();
        order.sort_by_key(|id| n(&mutations[id].record.value["sequence"]).unwrap_or(u64::MAX));
        // Retained non-position actions preserve exact owner/target relations.
        for operation in &order {
            let body = mutations[operation].record.value["body"].clone();
            for invalidated in body["redo_invalidated"]
                .as_array()
                .ok_or(PackageError::Record)?
            {
                if let Some(prior) =
                    mutations.get_mut(invalidated.as_str().ok_or(PackageError::Record)?)
                {
                    prior.invalidated = true;
                }
            }
            if matches!(body["action"]["kind"].as_str(), Some("undo" | "redo")) {
                let prior = mutations
                    .get_mut(
                        body["action"]["target"]["operation_id"]
                            .as_str()
                            .ok_or(PackageError::Record)?,
                    )
                    .ok_or(PackageError::Integrity)?;
                check(
                    operation_target(&prior.record) == body["action"]["target"]
                        && prior.record.value["body"]["owner"] == body["owner"],
                )?;
                prior.undone_by = (body["action"]["kind"] == "undo").then(|| operation.clone());
            }
        }
        Ok(Self {
            mutations,
            historical,
            history: Some(history),
            order,
            checkpoint_target: target,
            start,
        })
    }
}
