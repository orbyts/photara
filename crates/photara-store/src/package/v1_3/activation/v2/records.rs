use super::super::records as v1;
pub use super::super::{default_view, validate_view};
use super::{
    PackageError, Snapshot, Value, check, digest, encode, fields, hash, id, number, reference,
};
fn nullable(view: &Value) -> Result<(), PackageError> {
    if view.is_null() {
        Ok(())
    } else {
        reference(view)
    }
}
fn slot(body: &Value) -> Result<(), PackageError> {
    id(&body["device_id"])?;
    id(&body["workspace_slot_id"])?;
    digest(&body["slot_scope_sha256"])
}
pub(super) fn context(body: &Value) -> Result<(), PackageError> {
    fields(body, &["authority", "principal", "library_id"])?;
    id(&body["library_id"])?;
    fields(&body["authority"], &["kind", "database_id", "epoch"])?;
    check(body["authority"]["kind"] == "local")?;
    id(&body["authority"]["database_id"])?;
    id(&body["authority"]["epoch"])?;
    fields(&body["principal"], &["kind", "id"])?;
    check(body["principal"]["kind"] == "local")?;
    id(&body["principal"]["id"])?;
    Ok(())
}
fn target(body: &Value) -> Result<(), PackageError> {
    match body["kind"].as_str() {
        Some("library") => fields(body, &["kind", "context"])?,
        Some("project") => {
            fields(body, &["kind", "context", "project"])?;
            v1::target(&body["project"])?;
            check(body["project"]["library_id"] == body["context"]["library_id"])?;
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    context(&body["context"])
}
#[expect(
    clippy::too_many_lines,
    reason = "Exact approved v2 record shape dispatch"
)]
pub(super) fn shape(record: &Value) -> Result<(), PackageError> {
    let body = &record["body"];
    check(body.is_object())?;
    if matches!(
        record["kind"].as_str(),
        Some("SessionView" | "SavedProof" | "TargetOpenProof")
    ) {
        return v1::shape(record);
    }
    check(record["version"] == 2)?;
    match record["kind"].as_str() {
        Some("LocalContextProof") => {
            fields(
                body,
                &[
                    "context",
                    "library_revision",
                    "contract_revision",
                    "authorization_generation",
                ],
            )?;
            context(&body["context"])?;
            for k in [
                "library_revision",
                "contract_revision",
                "authorization_generation",
            ] {
                check(number(&body[k])? > 0)?;
            }
        }
        Some("ConfirmationEvidence") => {
            fields(
                body,
                &[
                    "activation_id",
                    "device_id",
                    "workspace_slot_id",
                    "request_generation",
                    "expected_committed_generation",
                    "slot_scope_sha256",
                    "source_active",
                    "source_binding",
                    "accepted",
                    "prior_saved",
                    "target",
                ],
            )?;
            id(&body["activation_id"])?;
            slot(body)?;
            number(&body["request_generation"])?;
            number(&body["expected_committed_generation"])?;
            reference(&body["source_active"])?;
            reference(&body["prior_saved"])?;
            v1::binding(&body["source_binding"])?;
            v1::accepted(&body["accepted"])?;
            target(&body["target"])?;
        }
        Some("ActivationIntent") => {
            fields(
                body,
                &[
                    "device_id",
                    "workspace_slot_id",
                    "request_generation",
                    "expected_committed_generation",
                    "slot_scope_sha256",
                    "source_active",
                    "target",
                    "source_attachment",
                    "confirmation_basis",
                ],
            )?;
            slot(body)?;
            check(number(&body["request_generation"])? > 0)?;
            number(&body["expected_committed_generation"])?;
            nullable(&body["source_active"])?;
            nullable(&body["confirmation_basis"])?;
            if !body["source_attachment"].is_null() {
                v1::binding(&body["source_attachment"])?;
            }
            target(&body["target"])?;
        }
        Some("RollbackCapsule") => {
            fields(
                body,
                &[
                    "source_active",
                    "source_saved",
                    "view",
                    "graph_snapshot",
                    "registration_sha256",
                ],
            )?;
            for k in ["source_active", "source_saved", "view"] {
                reference(&body[k])?;
            }
            digest(&body["registration_sha256"])?;
            v1::core_graph(&body["graph_snapshot"])?;
        }
        Some("SelectionReady") => {
            fields(body, &["intent", "context_proof", "project_open"])?;
            reference(&body["intent"])?;
            reference(&body["context_proof"])?;
            nullable(&body["project_open"])?;
        }
        Some("ActivationProgress") => {
            fields(
                body,
                &["intent", "stage", "source_saved", "capsule", "target_ready"],
            )?;
            reference(&body["intent"])?;
            check(matches!(
                body["stage"].as_str(),
                Some(
                    "Preparing"
                        | "Confirmed"
                        | "Frozen"
                        | "SourceSaved"
                        | "TargetReady"
                        | "Refused"
                )
            ))?;
            for k in ["source_saved", "capsule", "target_ready"] {
                nullable(&body[k])?;
            }
        }
        Some("ActiveSelection") => {
            fields(
                body,
                &[
                    "device_id",
                    "workspace_slot_id",
                    "committed_generation",
                    "slot_scope_sha256",
                    "activation_id",
                    "target",
                    "ready",
                ],
            )?;
            slot(body)?;
            check(number(&body["committed_generation"])? > 0)?;
            id(&body["activation_id"])?;
            target(&body["target"])?;
            reference(&body["ready"])?;
        }
        Some("ActivationReceipt") => {
            fields(
                body,
                &[
                    "intent",
                    "outcome",
                    "old_committed_generation",
                    "new_committed_generation",
                    "source_saved",
                    "capsule",
                    "target_ready",
                    "active",
                ],
            )?;
            reference(&body["intent"])?;
            check(matches!(
                body["outcome"].as_str(),
                Some("Activated" | "RetainedCurrent" | "RetainedReadOnlyRecovery")
            ))?;
            number(&body["old_committed_generation"])?;
            number(&body["new_committed_generation"])?;
            for k in ["source_saved", "capsule", "target_ready", "active"] {
                nullable(&body[k])?;
            }
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(())
}
fn source<'active>(
    snapshot: &'active Snapshot,
    intent: &Value,
) -> Result<Option<&'active Value>, PackageError> {
    if intent["source_active"].is_null() {
        return Ok(None);
    }
    let active = snapshot.kind(&intent["source_active"], "ActiveSelection")?;
    Ok((active["target"]["kind"] == "project").then_some(active))
}
fn owner_context(binding: &Value, context: &Value) -> Result<(), PackageError> {
    check(
        binding["owner"]["principal"]["kind"] == "local-controller"
            && binding["owner"]["principal"]["principal_id"] == context["principal"]["id"],
    )
}
fn saved(proof: &Value, target: &Value) -> Result<(), PackageError> {
    check(
        proof["binding"]["project_id"] == target["project_id"]
            && proof["binding"]["incarnation_id"] == target["incarnation_id"]
            && proof["registration_sha256"] == target["registration_sha256"]
            && proof["checkpoint_receipt"]["value"]["body"]["operation_receipt"]["library_id"]
                == target["library_id"],
    )
}
fn evidence(
    snapshot: &Snapshot,
    intent: &Value,
    proof: &Value,
    capsule: &Value,
) -> Result<(), PackageError> {
    let Some(active) = source(snapshot, intent)? else {
        return check(proof.is_null() && capsule.is_null());
    };
    if !proof.is_null() {
        let proof = snapshot.kind(proof, "SavedProof")?;
        saved(proof, &active["target"]["project"])?;
        check(proof["binding"] == intent["source_attachment"])?;
        if !intent["confirmation_basis"].is_null() {
            check(
                proof["accepted"]
                    == snapshot.kind(&intent["confirmation_basis"], "ConfirmationEvidence")?["accepted"],
            )?;
        }
    }
    if !capsule.is_null() {
        let capsule = snapshot.kind(capsule, "RollbackCapsule")?;
        check(
            capsule["source_active"] == intent["source_active"]
                && capsule["source_saved"] == *proof,
        )?;
    }
    Ok(())
}
fn ready(
    snapshot: &Snapshot,
    record: &Value,
    intent: &Value,
    intent_ref: &Value,
) -> Result<(), PackageError> {
    let record = snapshot.kind(record, "SelectionReady")?;
    check(record["intent"] == *intent_ref)?;
    let proof = snapshot.kind(&record["context_proof"], "LocalContextProof")?;
    check(proof["context"] == intent["target"]["context"])?;
    if intent["target"]["kind"] == "library" {
        return check(record["project_open"].is_null());
    }
    let proof = snapshot.kind(&record["project_open"], "TargetOpenProof")?;
    let target = &intent["target"]["project"];
    owner_context(&proof["binding"], &intent["target"]["context"])?;
    check(
        proof["binding"]["project_id"] == target["project_id"]
            && proof["binding"]["incarnation_id"] == target["incarnation_id"]
            && proof["registration_sha256"] == target["registration_sha256"]
            && proof["graph_id"] == target["graph_id"]
            && proof["selected_commit"]["root_set"]["library_id"] == target["library_id"],
    )
}
#[expect(clippy::too_many_lines, reason = "Exact linked v2 authority contracts")]
pub(super) fn links(snapshot: &Snapshot, record: &Value) -> Result<(), PackageError> {
    let body = &record["body"];
    match record["kind"].as_str() {
        Some("SessionView" | "SavedProof" | "LocalContextProof") => {}
        Some("TargetOpenProof") => {
            snapshot.kind(&body["view"], "SessionView")?;
            let graph = body["graph_id"].as_str().ok_or(PackageError::Record)?;
            check(
                body["accepted"]["coordinate"]["graphs"]
                    .get(graph)
                    .is_some(),
            )?;
        }
        Some("ConfirmationEvidence") => {
            let active = snapshot.kind(&body["source_active"], "ActiveSelection")?;
            check(active["target"]["kind"] == "project" && body["target"]["kind"] == "project")?;
            let proof = snapshot.kind(&body["prior_saved"], "SavedProof")?;
            saved(proof, &active["target"]["project"])?;
            check(
                proof["binding"] == body["source_binding"]
                    && proof["accepted"] == body["accepted"]
                    && active["committed_generation"] == body["expected_committed_generation"],
            )?;
            for k in ["device_id", "workspace_slot_id", "slot_scope_sha256"] {
                check(active[k] == body[k])?;
            }
        }
        Some("ActivationIntent") => {
            if body["source_active"].is_null() {
                check(number(&body["expected_committed_generation"])? == 0)?;
            } else {
                let active = snapshot.kind(&body["source_active"], "ActiveSelection")?;
                for k in ["device_id", "workspace_slot_id", "slot_scope_sha256"] {
                    check(active[k] == body[k])?;
                }
                check(
                    active["committed_generation"] == body["expected_committed_generation"]
                        && active["target"] != body["target"],
                )?;
                check(
                    active["target"]["context"]["authority"]
                        == body["target"]["context"]["authority"]
                        && active["target"]["context"]["principal"]
                            == body["target"]["context"]["principal"],
                )?;
            }
            if let Some(active) = source(snapshot, body)? {
                let target = &active["target"]["project"];
                owner_context(&body["source_attachment"], &active["target"]["context"])?;
                check(
                    body["source_attachment"]["project_id"] == target["project_id"]
                        && body["source_attachment"]["incarnation_id"] == target["incarnation_id"],
                )?;
                if body["target"]["kind"] == "project" {
                    let capsule =
                        snapshot.kind(&body["confirmation_basis"], "ConfirmationEvidence")?;
                    check(
                        capsule["activation_id"] == record["id"]
                            && capsule["source_binding"] == body["source_attachment"],
                    )?;
                    for k in [
                        "device_id",
                        "workspace_slot_id",
                        "request_generation",
                        "expected_committed_generation",
                        "slot_scope_sha256",
                        "source_active",
                        "target",
                    ] {
                        check(capsule[k] == body[k])?;
                    }
                } else {
                    check(body["confirmation_basis"].is_null())?;
                }
            } else {
                check(body["source_attachment"].is_null() && body["confirmation_basis"].is_null())?;
            }
        }
        Some("RollbackCapsule") => {
            let active = snapshot.kind(&body["source_active"], "ActiveSelection")?;
            check(active["target"]["kind"] == "project")?;
            let target = &active["target"]["project"];
            let proof = snapshot.kind(&body["source_saved"], "SavedProof")?;
            saved(proof, target)?;
            let view = snapshot.record(&body["view"])?;
            check(validate_view(
                view,
                &active["device_id"],
                target,
                &body["graph_snapshot"],
            ))?;
            let graph = target["graph_id"].as_str().ok_or(PackageError::Record)?;
            check(
                body["registration_sha256"] == proof["registration_sha256"]
                    && hash(&encode(&body["graph_snapshot"])?)
                        == proof["accepted"]["coordinate"]["graphs"][graph]["payload_digest"],
            )?;
        }
        Some("SelectionReady") => {
            let intent = snapshot.kind(&body["intent"], "ActivationIntent")?;
            ready(
                snapshot,
                &super::record_ref(record)?,
                intent,
                &body["intent"],
            )?;
        }
        Some("ActivationProgress") => {
            let intent = snapshot.kind(&body["intent"], "ActivationIntent")?;
            snapshot.optional(&body["source_saved"], "SavedProof")?;
            snapshot.optional(&body["capsule"], "RollbackCapsule")?;
            snapshot.optional(&body["target_ready"], "SelectionReady")?;
            evidence(snapshot, intent, &body["source_saved"], &body["capsule"])?;
            if matches!(
                body["stage"].as_str(),
                Some("Preparing" | "Confirmed" | "Frozen")
            ) {
                check(
                    body["source_saved"].is_null()
                        && body["capsule"].is_null()
                        && body["target_ready"].is_null(),
                )?;
            }
            if matches!(body["stage"].as_str(), Some("SourceSaved" | "TargetReady"))
                && source(snapshot, intent)?.is_some()
            {
                check(!body["source_saved"].is_null() && !body["capsule"].is_null())?;
            }
            if body["stage"] == "Confirmed" {
                check(!intent["confirmation_basis"].is_null())?;
            }
            if body["stage"] == "TargetReady" {
                ready(snapshot, &body["target_ready"], intent, &body["intent"])?;
            } else if body["stage"] != "Refused" {
                check(body["target_ready"].is_null())?;
            }
        }
        Some("ActiveSelection") => {
            let record = snapshot
                .records
                .get(body["activation_id"].as_str().ok_or(PackageError::Record)?)
                .ok_or(PackageError::Integrity)?;
            check(record["kind"] == "ActivationIntent")?;
            let intent = &record["body"];
            ready(
                snapshot,
                &body["ready"],
                intent,
                &super::record_ref(record)?,
            )?;
            for k in [
                "device_id",
                "workspace_slot_id",
                "slot_scope_sha256",
                "target",
            ] {
                check(body[k] == intent[k])?;
            }
            check(
                number(&body["committed_generation"])?
                    == number(&intent["expected_committed_generation"])?
                        .checked_add(1)
                        .ok_or(PackageError::Limit)?,
            )?;
        }
        Some("ActivationReceipt") => {
            let intent = snapshot.kind(&body["intent"], "ActivationIntent")?;
            check(body["old_committed_generation"] == intent["expected_committed_generation"])?;
            evidence(snapshot, intent, &body["source_saved"], &body["capsule"])?;
            snapshot.optional(&body["target_ready"], "SelectionReady")?;
            if body["outcome"] == "Activated" {
                let active = snapshot.kind(&body["active"], "ActiveSelection")?;
                check(
                    number(&body["new_committed_generation"])?
                        == number(&body["old_committed_generation"])?
                            .checked_add(1)
                            .ok_or(PackageError::Limit)?
                        && active["activation_id"] == body["intent"]["id"]
                        && active["committed_generation"] == body["new_committed_generation"]
                        && active["ready"] == body["target_ready"],
                )?;
                ready(snapshot, &body["target_ready"], intent, &body["intent"])?;
                if source(snapshot, intent)?.is_some() {
                    check(!body["source_saved"].is_null() && !body["capsule"].is_null())?;
                }
            } else {
                check(
                    body["new_committed_generation"] == body["old_committed_generation"]
                        && body["active"] == intent["source_active"],
                )?;
                if body["outcome"] == "RetainedReadOnlyRecovery" {
                    check(!body["capsule"].is_null())?;
                }
            }
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(())
}
pub(super) fn valid_selected_view(snapshot: &Snapshot, active: &Value) -> Result<(), PackageError> {
    if active["target"]["kind"] == "library" {
        return Ok(());
    }
    let record = snapshot.kind(&active["ready"], "SelectionReady")?;
    let proof = snapshot.kind(&record["project_open"], "TargetOpenProof")?;
    let view = snapshot.record(&proof["view"])?;
    check(view["version"] == 1)?;
    v1::view_shape(&view["body"])?;
    check(view["body"]["device_id"] == active["device_id"])?;
    for k in ["library_id", "project_id", "graph_id"] {
        check(view["body"][k] == active["target"]["project"][k])?;
    }
    Ok(())
}
