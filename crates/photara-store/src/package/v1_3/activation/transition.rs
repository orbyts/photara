use super::{PackageError, Snapshot, Value, check, number, records};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    Retry,
    Prepared,
    Progressed,
    Activated,
    Retained,
}
fn increment(old: &Value, new: &Value, key: &str) -> Result<(), PackageError> {
    check(
        number(&new[key])?
            == number(&old[key])?
                .checked_add(1)
                .ok_or(PackageError::Limit)?,
    )
}
/// Validates one exact immutable-snapshot CAS. Native lock, current authority,
/// independent Saved/open proofs, capacity and all barriers remain prerequisites.
/// # Errors
/// Refuses stale scope/CAS, replaced originals, wrong generations, invalid stage
/// edges, premature activation, receipt ambiguity or evidence loss.
#[expect(
    clippy::too_many_lines,
    reason = "Single atomic snapshot transition checked in protocol order"
)]
pub fn validate_transition(
    old: &Snapshot,
    new: &Snapshot,
    expected_old_sha: &str,
    scope: &str,
) -> Result<Transition, PackageError> {
    check(
        old.sha256() == expected_old_sha
            && old.body()["authority_scope_sha256"] == scope
            && new.body()["authority_scope_sha256"] == scope,
    )?;
    if old.bytes() == new.bytes() {
        return Ok(Transition::Retry);
    }
    let before = old.body();
    let after = new.body();
    for key in ["device_id", "workspace_slot_id"] {
        check(before[key] == after[key])?;
    }
    increment(before, after, "revision")?;
    for (id, record) in &old.records {
        check(new.records.get(id) == Some(record))?;
    }
    let fresh = new
        .records
        .iter()
        .filter(|(id, _)| !old.records.contains_key(*id))
        .map(|(_, record)| record)
        .collect::<Vec<_>>();
    let progress_count = fresh
        .iter()
        .filter(|r| r["kind"] == "ActivationProgress")
        .count();
    let active_count = fresh
        .iter()
        .filter(|r| r["kind"] == "ActiveSession")
        .count();
    let receipts = fresh
        .iter()
        .filter(|record| record["kind"] == "ActivationReceipt")
        .collect::<Vec<_>>();
    match (old.pending(), new.pending()) {
        (None, Some(pending)) => {
            check(
                progress_count == 1
                    && active_count == 0
                    && !old
                        .records
                        .contains_key(pending["id"].as_str().ok_or(PackageError::Record)?),
            )?;
            check(
                receipts.is_empty()
                    && before["active"] == after["active"]
                    && before["committed_generation"] == after["committed_generation"],
            )?;
            increment(before, after, "request_generation")?;
            let progress = new.kind(pending, "ActivationProgress")?;
            check(progress["stage"] == "Preparing")?;
            let intent = new.kind(&progress["intent"], "ActivationIntent")?;
            check(
                !old.records.contains_key(
                    progress["intent"]["id"]
                        .as_str()
                        .ok_or(PackageError::Record)?,
                ) && intent["request_generation"] == after["request_generation"]
                    && intent["expected_committed_generation"] == before["committed_generation"]
                    && intent["source_active"] == before["active"],
            )?;
            check(
                fresh
                    .iter()
                    .filter(|r| r["kind"] == "ActivationIntent")
                    .count()
                    == 1,
            )?;
            Ok(Transition::Prepared)
        }
        (Some(prior), Some(pending)) => {
            check(
                progress_count == 1
                    && active_count == 0
                    && !old
                        .records
                        .contains_key(pending["id"].as_str().ok_or(PackageError::Record)?),
            )?;
            check(
                receipts.is_empty()
                    && before["active"] == after["active"]
                    && before["request_generation"] == after["request_generation"]
                    && before["committed_generation"] == after["committed_generation"]
                    && prior != pending,
            )?;
            let prior = old.kind(prior, "ActivationProgress")?;
            let next = new.kind(pending, "ActivationProgress")?;
            check(prior["intent"] == next["intent"])?;
            let intent = new.kind(&next["intent"], "ActivationIntent")?;
            let edge = (prior["stage"].as_str(), next["stage"].as_str());
            check(
                matches!(
                    edge,
                    (Some("Preparing"), Some("Confirmed"))
                        | (Some("Confirmed"), Some("Frozen"))
                        | (Some("Frozen"), Some("SourceSaved"))
                        | (Some("SourceSaved"), Some("TargetReady"))
                ) || (next["stage"] == "Refused" && prior["stage"] != "Refused")
                    || (intent["source_active"].is_null()
                        && prior["stage"] == "Preparing"
                        && next["stage"] == "TargetReady"),
            )?;
            for key in ["source_saved", "capsule", "target_open"] {
                if !prior[key].is_null() {
                    check(prior[key] == next[key])?;
                }
            }
            check(
                fresh
                    .iter()
                    .all(|r| r["kind"] != "ActivationIntent" && r["kind"] != "ActiveSession"),
            )?;
            Ok(Transition::Progressed)
        }
        (Some(prior), None) => {
            check(progress_count == 0)?;
            check(
                before["request_generation"] == after["request_generation"] && receipts.len() == 1,
            )?;
            let progress = old.kind(prior, "ActivationProgress")?;
            let receipt = &receipts[0]["body"];
            check(receipt["intent"] == progress["intent"])?;
            for key in ["source_saved", "capsule", "target_open"] {
                check(receipt[key] == progress[key])?;
            }
            if receipt["outcome"] == "Activated" {
                check(
                    active_count == 1
                        && !old.records.contains_key(
                            receipt["active"]["id"]
                                .as_str()
                                .ok_or(PackageError::Record)?,
                        ),
                )?;
                check(progress["stage"] == "TargetReady" && after["active"] == receipt["active"])?;
                increment(before, after, "committed_generation")?;
                records::valid_selected_view(new, new.kind(&after["active"], "ActiveSession")?)?;
                Ok(Transition::Activated)
            } else {
                check(active_count == 0)?;
                check(
                    progress["stage"] == "Refused"
                        && after["active"] == before["active"]
                        && after["committed_generation"] == before["committed_generation"],
                )?;
                Ok(Transition::Retained)
            }
        }
        (None, None) => {
            // Optional view/capsule records may be durably prepared before their
            // first visible use; no standalone pointer/generation mutation.
            check(
                receipts.is_empty()
                    && before["active"] == after["active"]
                    && before["request_generation"] == after["request_generation"]
                    && before["committed_generation"] == after["committed_generation"]
                    && fresh
                        .iter()
                        .all(|r| matches!(r["kind"].as_str(), Some("SessionView" | "SavedProof"))),
            )?;
            Ok(Transition::Progressed)
        }
    }
}
