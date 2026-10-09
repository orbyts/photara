use super::super::repeatable::journal::{
    self,
    session::{coordinate, link, owner},
};
use super::{
    BTreeSet, PackageError, Snapshot, Value, check, digest, encode, fields, hash, id, json, number,
    reference,
};
pub(super) fn binding(value: &Value) -> Result<(), PackageError> {
    fields(
        value,
        &["project_id", "incarnation_id", "owner_epoch", "owner"],
    )?;
    for key in ["project_id", "incarnation_id", "owner_epoch"] {
        id(&value[key])?;
    }
    owner(&value["owner"])
}
pub(super) fn accepted(value: &Value) -> Result<(), PackageError> {
    fields(value, &["coordinate", "mutation", "accepted_frame"])?;
    coordinate(&value["coordinate"])?;
    check(value["mutation"].is_null() == value["accepted_frame"].is_null())?;
    if !value["mutation"].is_null() {
        link(&value["mutation"])?;
        let frame = &value["accepted_frame"];
        fields(
            frame,
            &[
                "schema",
                "journal_id",
                "sequence",
                "operation_id",
                "request_sha256",
                "receipt_sha256",
            ],
        )?;
        check(
            frame["schema"] == json!({"id":"photara.package.accepted-journal-frame","version":1}),
        )?;
        id(&frame["journal_id"])?;
        id(&frame["operation_id"])?;
        number(&frame["sequence"])?;
        digest(&frame["request_sha256"])?;
        digest(&frame["receipt_sha256"])?;
    }
    Ok(())
}
pub(super) fn target(value: &Value) -> Result<(), PackageError> {
    fields(
        value,
        &[
            "library_id",
            "project_id",
            "incarnation_id",
            "graph_id",
            "registration_sha256",
        ],
    )?;
    for key in ["library_id", "project_id", "incarnation_id", "graph_id"] {
        id(&value[key])?;
    }
    digest(&value["registration_sha256"])
}
fn selected(head: &Value, commit: &Value, project: &Value) -> Result<(), PackageError> {
    fields(
        head,
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    check(
        head["schema"] == json!({"id":"photara.package.head","version":1})
            && &head["project_id"] == project
            && commit["project_id"] == *project
            && commit["commit_id"] == head["commit_id"]
            && head["commit_sha256"] == hash(&encode(commit)?),
    )?;
    id(&head["commit_id"])?;
    digest(&head["commit_sha256"])?;
    // The selected v1.3 commit remains the existing exact outer schema. Complete
    // root/physical closure and qualification are independently live-verified.
    fields(
        commit,
        &[
            "schema",
            "project_id",
            "commit_id",
            "write_id",
            "package_revision",
            "parent",
            "created_at",
            "bootstrap_sha256",
            "minimum_reader",
            "required_features",
            "authored",
            "history",
            "inventory",
            "root_set",
            "extensions",
        ],
    )?;
    check(commit["schema"] == json!({"id":"photara.package.commit","version":1}))?;
    id(&commit["write_id"])?;
    number(&commit["package_revision"])?;
    digest(&commit["bootstrap_sha256"])?;
    Ok(())
}
fn checkpoint(value: &Value, kind: &str) -> Result<(), PackageError> {
    fields(value, &["value", "record_checksum"])?;
    digest(&value["record_checksum"])?;
    check(value["value"]["kind"] == kind)?;
    journal::validate_detached_checkpoint(&value["value"])
}
fn common_slot(body: &Value) -> Result<(), PackageError> {
    id(&body["device_id"])?;
    id(&body["workspace_slot_id"])?;
    digest(&body["authority_scope_sha256"])
}
fn nullable_ref(value: &Value) -> Result<(), PackageError> {
    if !value.is_null() {
        reference(value)?;
    }
    Ok(())
}
pub(super) fn view_shape(body: &Value) -> Result<(), PackageError> {
    fields(
        body,
        &[
            "device_id",
            "library_id",
            "project_id",
            "graph_id",
            "pan_x",
            "pan_y",
            "zoom_ppm",
            "selected_node_ids",
            "visible_panels",
        ],
    )?;
    for key in ["device_id", "library_id", "project_id", "graph_id"] {
        id(&body[key])?;
    }
    check(
        body["pan_x"].as_i64().is_some()
            && body["pan_y"].as_i64().is_some()
            && number(&body["zoom_ppm"])? > 0,
    )?;
    let mut last = None;
    for value in body["selected_node_ids"]
        .as_array()
        .ok_or(PackageError::Record)?
    {
        let next = id(value)?;
        check(last.is_none_or(|p| p < next))?;
        last = Some(next);
    }
    let mut panels = BTreeSet::new();
    for value in body["visible_panels"]
        .as_array()
        .ok_or(PackageError::Record)?
    {
        let panel = value.as_str().ok_or(PackageError::Record)?;
        check(
            matches!(
                panel,
                "assetGallery"
                    | "graph"
                    | "layoutAuthoring"
                    | "inspector"
                    | "diagnostics"
                    | "people"
                    | "locations"
                    | "scenes"
                    | "projectInfo"
                    | "account"
            ) && panels.insert(panel),
        )?;
    }
    Ok(())
}
/// Validates optional view content against the exact authenticated Core Graph.
/// False requires a fresh default-view record; it cannot select a Project.
#[must_use]
pub fn validate_view(record: &Value, device: &Value, target: &Value, graph: &Value) -> bool {
    let body = &record["body"];
    record["kind"] == "SessionView"
        && record["version"] == 1
        && view_shape(body).is_ok()
        && body["device_id"] == *device
        && ["library_id", "project_id", "graph_id"]
            .iter()
            .all(|key| body[*key] == target[*key])
        && graph["id"] == target["graph_id"]
        && body["selected_node_ids"]
            .as_array()
            .is_some_and(|selected| {
                selected.iter().all(|id| {
                    graph["nodes"]
                        .as_array()
                        .is_some_and(|nodes| nodes.iter().any(|node| node["id"] == *id))
                })
            })
}
/// Deterministic optional view body; caller persists it under a fresh record ID.
#[must_use]
pub fn default_view(device: &Value, target: &Value) -> Value {
    json!({"device_id":device,"library_id":target["library_id"],"project_id":target["project_id"],"graph_id":target["graph_id"],"pan_x":0,"pan_y":0,"zoom_ppm":"1000000","selected_node_ids":[],"visible_panels":["graph"]})
}
#[expect(
    clippy::too_many_lines,
    reason = "Exact reviewed authority record dispatch"
)]
pub(super) fn shape(record: &Value) -> Result<(), PackageError> {
    let body = &record["body"];
    check(body.is_object())?;
    if record["kind"] == "SessionView" {
        return Ok(());
    } // Optional content/version may need replacement; its raw immutable hash remains checked.
    if record["version"] != 1 {
        return Err(PackageError::UnsupportedVersion);
    }
    match record["kind"].as_str() {
        Some("SavedProof") => {
            fields(
                body,
                &[
                    "binding",
                    "accepted",
                    "checkpoint_intent",
                    "checkpoint_receipt",
                    "registration_sha256",
                ],
            )?;
            binding(&body["binding"])?;
            accepted(&body["accepted"])?;
            digest(&body["registration_sha256"])?;
            checkpoint(&body["checkpoint_intent"], "CheckpointIntent")?;
            checkpoint(&body["checkpoint_receipt"], "CheckpointReceipt")?;
            let intent = &body["checkpoint_intent"]["value"];
            let receipt = &body["checkpoint_receipt"]["value"];
            check(
                receipt["body"]["intent_record_id"] == intent["record_id"]
                    && receipt["body"]["intent_record_checksum"]
                        == body["checkpoint_intent"]["record_checksum"]
                    && receipt["body"]["original_sha256"]
                        == hash(&encode(&intent["body"]["original"])?)
                    && receipt["body"]["operation_receipt"] == intent["body"]["operation_receipt"]
                    && number(&receipt["sequence"])? > number(&intent["sequence"])?,
            )?;
            for value in [intent, receipt] {
                check(
                    value["project_id"] == body["binding"]["project_id"]
                        && value["incarnation_id"] == body["binding"]["incarnation_id"],
                )?;
            }
            selected(
                &receipt["body"]["selected_head"],
                &receipt["body"]["selected_commit"],
                &body["binding"]["project_id"],
            )?;
            check(
                body["accepted"]["coordinate"]["revision"]
                    == receipt["body"]["operation_receipt"]["after"]["revision"]
                    && body["accepted"]["coordinate"]["authored_digest"]
                        == receipt["body"]["operation_receipt"]["after"]["digest"],
            )?;
            if !body["accepted"]["accepted_frame"].is_null() {
                check(body["accepted"]["accepted_frame"] == intent["body"]["accepted_frame"])?;
            }
            Ok(())
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
            for key in ["source_active", "source_saved", "view"] {
                reference(&body[key])?;
            }
            digest(&body["registration_sha256"])?;
            core_graph(&body["graph_snapshot"])
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
                    "authority_scope_sha256",
                    "source_active",
                    "source_binding",
                    "accepted",
                    "prior_saved",
                    "target",
                ],
            )?;
            id(&body["activation_id"])?;
            common_slot(body)?;
            for key in ["request_generation", "expected_committed_generation"] {
                number(&body[key])?;
            }
            reference(&body["source_active"])?;
            binding(&body["source_binding"])?;
            accepted(&body["accepted"])?;
            reference(&body["prior_saved"])?;
            target(&body["target"])
        }
        Some("ActivationIntent") => {
            fields(
                body,
                &[
                    "device_id",
                    "workspace_slot_id",
                    "request_generation",
                    "expected_committed_generation",
                    "authority_scope_sha256",
                    "source_active",
                    "target",
                    "source_attachment",
                    "confirmation_basis",
                ],
            )?;
            common_slot(body)?;
            check(number(&body["request_generation"])? > 0)?;
            number(&body["expected_committed_generation"])?;
            nullable_ref(&body["source_active"])?;
            nullable_ref(&body["confirmation_basis"])?;
            check(
                body["source_active"].is_null() == body["source_attachment"].is_null()
                    && body["source_active"].is_null() == body["confirmation_basis"].is_null(),
            )?;
            if !body["source_attachment"].is_null() {
                binding(&body["source_attachment"])?;
            }
            target(&body["target"])
        }
        Some("ActivationProgress") => {
            fields(
                body,
                &["intent", "stage", "source_saved", "capsule", "target_open"],
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
            for key in ["source_saved", "capsule", "target_open"] {
                nullable_ref(&body[key])?;
            }
            Ok(())
        }
        Some("TargetOpenProof") => {
            fields(
                body,
                &[
                    "binding",
                    "registration_sha256",
                    "selected_head",
                    "selected_commit",
                    "accepted",
                    "graph_id",
                    "view",
                ],
            )?;
            binding(&body["binding"])?;
            digest(&body["registration_sha256"])?;
            selected(
                &body["selected_head"],
                &body["selected_commit"],
                &body["binding"]["project_id"],
            )?;
            accepted(&body["accepted"])?;
            id(&body["graph_id"])?;
            reference(&body["view"])
        }
        Some("ActiveSession") => {
            fields(
                body,
                &[
                    "device_id",
                    "workspace_slot_id",
                    "committed_generation",
                    "authority_scope_sha256",
                    "library_id",
                    "project_id",
                    "incarnation_id",
                    "graph_id",
                    "activation_id",
                    "view",
                    "target_open",
                ],
            )?;
            common_slot(body)?;
            check(number(&body["committed_generation"])? > 0)?;
            for key in [
                "library_id",
                "project_id",
                "incarnation_id",
                "graph_id",
                "activation_id",
            ] {
                id(&body[key])?;
            }
            reference(&body["view"])?;
            reference(&body["target_open"])
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
                    "target_open",
                    "active",
                ],
            )?;
            reference(&body["intent"])?;
            check(matches!(
                body["outcome"].as_str(),
                Some("Activated" | "RetainedCurrent" | "RetainedReadOnlyRecovery")
            ))?;
            for key in ["old_committed_generation", "new_committed_generation"] {
                number(&body[key])?;
            }
            for key in ["source_saved", "capsule", "target_open", "active"] {
                nullable_ref(&body[key])?;
            }
            Ok(())
        }
        _ => Err(PackageError::UnsupportedVersion),
    }
}
pub(super) fn core_graph(value: &Value) -> Result<(), PackageError> {
    id(&value["id"])?;
    for key in ["nodes", "connections"] {
        for item in value[key].as_array().ok_or(PackageError::Record)? {
            id(&item["id"])?;
        }
    }
    let graph: photara_core::GraphDocument =
        serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)?;
    let packages = graph
        .nodes
        .iter()
        .map(|node| photara_core::PackageRequirement {
            package_id: node.definition.package_id.clone(),
            package_version: node.definition.package_version.clone(),
        })
        .collect::<BTreeSet<_>>();
    photara_core::NodeGraphDocument {
        schema_version: photara_core::SchemaVersion::first(),
        metadata: photara_core::NodeGraphMetadata {
            name: "Activation recovery".into(),
            description: None,
        },
        required_packages: packages.into_iter().collect(),
        graph,
        extensions: std::collections::BTreeMap::default(),
    }
    .validate()
    .map_err(|_| PackageError::Record)
}
fn same_project(binding: &Value, active: &Value) -> Result<(), PackageError> {
    check(
        binding["project_id"] == active["project_id"]
            && binding["incarnation_id"] == active["incarnation_id"],
    )
}
fn saved_source(snapshot: &Snapshot, proof: &Value, active: &Value) -> Result<(), PackageError> {
    same_project(&proof["binding"], active)?;
    let open = snapshot.kind(&active["target_open"], "TargetOpenProof")?;
    check(
        proof["registration_sha256"] == open["registration_sha256"]
            && proof["checkpoint_receipt"]["value"]["body"]["operation_receipt"]["library_id"]
                == active["library_id"],
    )
}
fn target_open(snapshot: &Snapshot, reference: &Value, target: &Value) -> Result<(), PackageError> {
    let open = snapshot.kind(reference, "TargetOpenProof")?;
    check(
        open["binding"]["project_id"] == target["project_id"]
            && open["binding"]["incarnation_id"] == target["incarnation_id"]
            && open["graph_id"] == target["graph_id"]
            && open["registration_sha256"] == target["registration_sha256"]
            && open["selected_commit"]["root_set"]["library_id"] == target["library_id"],
    )
}
fn source_evidence(
    snapshot: &Snapshot,
    intent: &Value,
    saved: &Value,
    capsule: &Value,
) -> Result<(), PackageError> {
    if intent["source_active"].is_null() {
        return check(saved.is_null() && capsule.is_null());
    }
    let source = snapshot.kind(&intent["source_active"], "ActiveSession")?;
    if !saved.is_null() {
        let proof = snapshot.kind(saved, "SavedProof")?;
        saved_source(snapshot, proof, source)?;
        let confirmation = snapshot.kind(&intent["confirmation_basis"], "ConfirmationEvidence")?;
        check(
            proof["binding"] == intent["source_attachment"]
                && proof["accepted"] == confirmation["accepted"],
        )?;
    }
    if !capsule.is_null() {
        let capsule = snapshot.kind(capsule, "RollbackCapsule")?;
        check(
            capsule["source_active"] == intent["source_active"]
                && capsule["source_saved"] == *saved,
        )?;
    }
    Ok(())
}
#[expect(
    clippy::too_many_lines,
    reason = "Linked authority identity constraints kept together"
)]
pub(super) fn links(snapshot: &Snapshot, record: &Value) -> Result<(), PackageError> {
    let body = &record["body"];
    match record["kind"].as_str() {
        Some("SessionView" | "SavedProof") => Ok(()),
        Some("ConfirmationEvidence") => {
            let active = snapshot.kind(&body["source_active"], "ActiveSession")?;
            same_project(&body["source_binding"], active)?;
            let proof = snapshot.kind(&body["prior_saved"], "SavedProof")?;
            saved_source(snapshot, proof, active)?;
            check(
                proof["binding"] == body["source_binding"]
                    && proof["accepted"] == body["accepted"]
                    && active["device_id"] == body["device_id"]
                    && active["workspace_slot_id"] == body["workspace_slot_id"]
                    && active["committed_generation"] == body["expected_committed_generation"]
                    && active["authority_scope_sha256"] == body["authority_scope_sha256"]
                    && active["library_id"] == body["target"]["library_id"],
            )
        }
        Some("ActivationIntent") => {
            if body["source_active"].is_null() {
                return check(number(&body["expected_committed_generation"])? == 0);
            }
            let source = snapshot.kind(&body["source_active"], "ActiveSession")?;
            same_project(&body["source_attachment"], source)?;
            check(
                source["library_id"] == body["target"]["library_id"]
                    && source["project_id"] != body["target"]["project_id"],
            )?;
            let confirmation =
                snapshot.kind(&body["confirmation_basis"], "ConfirmationEvidence")?;
            check(
                confirmation["activation_id"] == record["id"]
                    && confirmation["source_binding"] == body["source_attachment"],
            )?;
            for key in [
                "device_id",
                "workspace_slot_id",
                "request_generation",
                "expected_committed_generation",
                "authority_scope_sha256",
                "source_active",
                "target",
            ] {
                check(confirmation[key] == body[key])?;
            }
            Ok(())
        }
        Some("RollbackCapsule") => {
            let active = snapshot.kind(&body["source_active"], "ActiveSession")?;
            let saved = snapshot.kind(&body["source_saved"], "SavedProof")?;
            snapshot.kind(&body["view"], "SessionView")?;
            saved_source(snapshot, saved, active)?;
            let graph = active["graph_id"].as_str().ok_or(PackageError::Record)?;
            check(
                body["registration_sha256"] == saved["registration_sha256"]
                    && body["graph_snapshot"]["id"] == active["graph_id"]
                    && hash(&encode(&body["graph_snapshot"])?)
                        == saved["accepted"]["coordinate"]["graphs"][graph]["payload_digest"],
            )
        }
        Some("ActivationProgress") => {
            let intent = snapshot.kind(&body["intent"], "ActivationIntent")?;
            snapshot.optional(&body["source_saved"], "SavedProof")?;
            snapshot.optional(&body["capsule"], "RollbackCapsule")?;
            snapshot.optional(&body["target_open"], "TargetOpenProof")?;
            source_evidence(snapshot, intent, &body["source_saved"], &body["capsule"])?;
            if matches!(
                body["stage"].as_str(),
                Some("Preparing" | "Confirmed" | "Frozen")
            ) {
                check(
                    body["source_saved"].is_null()
                        && body["capsule"].is_null()
                        && body["target_open"].is_null(),
                )?;
            }
            if matches!(body["stage"].as_str(), Some("SourceSaved" | "TargetReady"))
                && !intent["source_active"].is_null()
            {
                check(!body["source_saved"].is_null() && !body["capsule"].is_null())?;
            }
            if body["stage"] == "TargetReady" {
                target_open(snapshot, &body["target_open"], &intent["target"])?;
            } else if body["stage"] != "Refused" {
                check(body["target_open"].is_null())?;
            }
            Ok(())
        }
        Some("TargetOpenProof") => {
            snapshot.kind(&body["view"], "SessionView")?;
            let graph = body["graph_id"].as_str().ok_or(PackageError::Record)?;
            check(
                body["accepted"]["coordinate"]["graphs"]
                    .get(graph)
                    .is_some(),
            )
        }
        Some("ActiveSession") => {
            let intent = snapshot
                .records
                .get(body["activation_id"].as_str().ok_or(PackageError::Record)?)
                .ok_or(PackageError::Integrity)?;
            check(intent["kind"] == "ActivationIntent")?;
            let intent = &intent["body"];
            target_open(snapshot, &body["target_open"], &intent["target"])?;
            let open = snapshot.kind(&body["target_open"], "TargetOpenProof")?;
            check(body["view"] == open["view"])?;
            snapshot.kind(&body["view"], "SessionView")?;
            for key in ["device_id", "workspace_slot_id", "authority_scope_sha256"] {
                check(body[key] == intent[key])?;
            }
            for key in ["library_id", "project_id", "incarnation_id", "graph_id"] {
                check(body[key] == intent["target"][key])?;
            }
            check(
                number(&body["committed_generation"])?
                    == number(&intent["expected_committed_generation"])?
                        .checked_add(1)
                        .ok_or(PackageError::Limit)?,
            )
        }
        Some("ActivationReceipt") => {
            let intent = snapshot.kind(&body["intent"], "ActivationIntent")?;
            check(body["old_committed_generation"] == intent["expected_committed_generation"])?;
            source_evidence(snapshot, intent, &body["source_saved"], &body["capsule"])?;
            snapshot.optional(&body["target_open"], "TargetOpenProof")?;
            if body["outcome"] == "Activated" {
                check(
                    number(&body["new_committed_generation"])?
                        == number(&body["old_committed_generation"])?
                            .checked_add(1)
                            .ok_or(PackageError::Limit)?,
                )?;
                let active = snapshot.kind(&body["active"], "ActiveSession")?;
                check(
                    active["activation_id"] == body["intent"]["id"]
                        && active["committed_generation"] == body["new_committed_generation"]
                        && active["target_open"] == body["target_open"],
                )?;
                target_open(snapshot, &body["target_open"], &intent["target"])?;
                if !intent["source_active"].is_null() {
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
            Ok(())
        }
        _ => Err(PackageError::UnsupportedVersion),
    }
}
pub(super) fn valid_selected_view(snapshot: &Snapshot, active: &Value) -> Result<(), PackageError> {
    let view = snapshot.record(&active["view"])?;
    check(view["version"] == 1)?;
    view_shape(&view["body"])?;
    check(view["body"]["device_id"] == active["device_id"])?;
    for key in ["library_id", "project_id", "graph_id"] {
        check(view["body"][key] == active[key])?;
    }
    Ok(())
}
