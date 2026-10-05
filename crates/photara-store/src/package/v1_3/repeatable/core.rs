//! Actual Core/PS1 replay from authenticated original packed semantic bytes.
use super::{
    Limits, PlannerInputs,
    input::{OriginalPackage, OriginalProof, PreparedChange},
    layout,
    wire::{self, Result, encode, ensure, hash, key, number, reference, text},
};
use crate::package::{
    MemoryPackage,
    planning::{
        AuthoredCommand, CheckpointIds, IncarnationId, MutationRequest, PackageNamingPolicy,
        PlanOutcome, PlanRequest, VerifiedClosure, WriteId, plan,
    },
};
use photara_core::{NodeDefinitionRegistry, ValueTypeRegistry, creation::PackageExtension};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[expect(
    clippy::too_many_lines,
    reason = "Keep actual Core replay and receipt binding reviewable together"
)]
pub(super) fn replay(
    old: &OriginalPackage,
    proof: &OriginalProof,
    intent: &Value,
    receipt: &Value,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<PreparedChange> {
    let active = proof.roles.get("active").ok_or("active role")?;
    ensure(
        reference(&active.state) == old.commit["root_set"]["active"],
        "original active root",
    )?;
    crate::package::v1_3::tree::fields(
        intent,
        &[
            "bootstrap_sha256",
            "boundary",
            "command",
            "domain",
            "expected",
            "library_id",
            "operation_id",
            "project_id",
            "undo_group_id",
            "updated_at",
            "version",
        ],
    )
    .map_err(|_| "intent fields")?;
    ensure(
        intent["domain"] == "photara.package.operation-intent.v1"
            && intent["version"] == 1
            && intent["boundary"] == "single"
            && intent["undo_group_id"].is_null(),
        "supported intent codec",
    )?;
    ensure(
        intent["project_id"] == old.manifest["project_id"]
            && intent["library_id"] == old.commit["root_set"]["library_id"]
            && intent["bootstrap_sha256"] == hash(&encode(&old.manifest)),
        "original intent binding",
    )?;
    let (logical, closure) = original_closure(old, proof, inputs, limits)?;
    ensure(
        serde_json::to_value(logical.coordinate()).map_err(|_| "coordinate")? == intent["expected"],
        "actual expected coordinate",
    )?;
    crate::package::v1_3::tree::fields(&intent["command"], &["kind", "envelope"])
        .map_err(|_| "command fields")?;
    ensure(
        intent["command"]["kind"] == "graph",
        "repeatable supported Graph command",
    )?;
    let request = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone())
            .map_err(|_| "operation UUID")?,
        expected: logical.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(
                serde_json::from_value(intent["command"]["envelope"].clone())
                    .map_err(|_| "Graph command")?,
            ),
        },
        updated_at: text(&intent["updated_at"])?.into(),
    };
    let extension = PackageExtension::try_from("ps2preparation".to_owned())
        .map_err(|_| "temporary extension")?;
    let naming = PackageNamingPolicy {
        write_extension: extension.clone(),
        legacy_read_extensions: vec![],
    };
    let outcome = plan(
        &logical,
        PlanRequest {
            mutation: &request,
            expected_head: logical.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse(&inputs.active_root_id).map_err(|_| "temporary ID")?,
                commit_id: serde_json::from_value(json!(inputs.active_root_id))
                    .map_err(|_| "temporary commit")?,
            },
            naming: &naming,
            observed_extension: &extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .map_err(|_| "Core replay")?;
    let (authored, prepared_receipt) = match outcome {
        PlanOutcome::Checkpoint(plan) => {
            let mut objects = BTreeMap::new();
            for (path, bytes) in plan.candidate().files().files() {
                if path.starts_with("objects/json/sha256/") {
                    let v = wire::parse(bytes)?;
                    objects.insert(key(&reference(&v))?, v);
                }
            }
            (objects, plan.receipt().clone())
        }
        PlanOutcome::Unchanged(receipt) => (closure, receipt),
    };
    let identity = crate::package::v1_3::SelectionIdentity {
        project: crate::package::PackageUuid::parse(text(&old.manifest["project_id"])?)
            .map_err(|_| "project")?,
        library: crate::package::PackageUuid::parse(text(&old.commit["root_set"]["library_id"])?)
            .map_err(|_| "library")?,
        bootstrap_sha256: crate::package::Sha256Hex::parse(&hash(&encode(&old.manifest)))
            .map_err(|_| "bootstrap")?,
    };
    crate::package::v1_3::operations::validate_receipt(receipt, &identity)
        .map_err(|_| "frozen receipt")?;
    ensure(
        receipt["operation_id"] == intent["operation_id"]
            && receipt["request_sha256"] == hash(&encode(intent))
            && receipt["undo_group_id"] == intent["undo_group_id"],
        "original request receipt identity",
    )?;
    for (coordinate, field) in [
        (prepared_receipt.before(), "before"),
        (prepared_receipt.after(), "after"),
    ] {
        let v = serde_json::to_value(coordinate).map_err(|_| "Core receipt coordinate")?;
        ensure(
            receipt[field] == json!({"revision":v["revision"],"digest":v["authored_digest"]}),
            "actual Core result",
        )?;
    }
    ensure(
        number(&receipt["acceptance_ordinal"])?
            == number(&active.state["accepted"]["through_ordinal"])?
                .checked_add(1)
                .ok_or("ordinal overflow")?
            && receipt["journal_id"] == active.state["journal_inclusion"]["journal_id"]
            && number(&receipt["journal_sequence"])?
                > number(&active.state["journal_inclusion"]["through_sequence"])?,
        "next receipt order",
    )?;
    ensure(
        !active
            .receipts
            .iter()
            .any(|r| r["operation_id"] == receipt["operation_id"]),
        "existing operation identity",
    )?;
    Ok(PreparedChange {
        authored,
        receipt: receipt.clone(),
    })
}

pub(super) fn original_closure(
    old: &OriginalPackage,
    proof: &OriginalProof,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<(VerifiedClosure, BTreeMap<wire::Key, Value>)> {
    let active = proof.roles.get("active").ok_or("active role")?;
    let pool = layout::original_objects(old, inputs, limits)?;
    let mut closure = BTreeMap::new();
    let mut pending = vec![
        active.state["authored"].clone(),
        active.state["history"].clone(),
    ];
    let mut seen = BTreeSet::new();
    while let Some(r) = pending.pop() {
        let k = key(&r)?;
        if !seen.insert(k.clone()) {
            continue;
        }
        ensure(
            active.semantic.contains(&k) && seen.len() <= limits.max_objects,
            "authenticated semantic closure",
        )?;
        let value = pool.get(&k).ok_or("original semantic bytes")?;
        let mut edges = Vec::new();
        crate::package::v1_1::references(value, &mut edges).map_err(|_| "legacy edges")?;
        pending.extend(
            edges
                .into_iter()
                .map(|r| serde_json::to_value(r).expect("reference")),
        );
        closure.insert(k, value.clone());
    }
    let mut files = BTreeMap::new();
    for (k, v) in &closure {
        files.insert(format!("objects/json/sha256/{}.json", k.0), encode(v));
    }
    let inventory = json!({"schema":{"id":"photara.package.inventory","version":1},"project_id":old.manifest["project_id"],"objects":closure.values().map(reference).collect::<Vec<_>>()});
    let inventory_ref = reference(&inventory);
    files.insert(
        format!(
            "objects/json/sha256/{}.json",
            inventory_ref["sha256"].as_str().unwrap()
        ),
        encode(&inventory),
    );
    // Temporary PS1 wrappers are never published or presented as a package selector.
    let commit = json!({"schema":{"id":"photara.package.commit","version":1},"project_id":old.manifest["project_id"],"commit_id":old.commit["commit_id"],"write_id":old.commit["write_id"],"package_revision":"1","parent":null,"created_at":old.manifest["created_at"],"bootstrap_sha256":hash(&encode(&old.manifest)),"minimum_reader":old.manifest["format_version"],"required_features":old.manifest["required_features"],"authored":active.state["authored"],"history":active.state["history"],"inventory":inventory_ref});
    let head = json!({"schema":{"id":"photara.package.head","version":1},"project_id":old.manifest["project_id"],"commit_id":commit["commit_id"],"commit_sha256":hash(&encode(&commit))});
    files.insert("manifest.json".into(), encode(&old.manifest));
    files.insert("HEAD.json".into(), encode(&head));
    files.insert(
        format!("commits/{}.json", text(&commit["commit_id"])?),
        encode(&commit),
    );
    let logical = VerifiedClosure::verify(
        MemoryPackage::new(files, limits.semantic).map_err(|_| "temporary Core input")?,
        IncarnationId::parse(&inputs.active_root_id).map_err(|_| "temporary identity")?,
    )
    .map_err(|_| "actual original Core closure")?;

    Ok((logical, closure))
}
#[cfg(test)]
pub(crate) fn original_coordinate(
    original: &super::VerifiedOriginal,
    inputs: &PlannerInputs,
    limits: Limits,
) -> Result<Value> {
    let (logical, _) = original_closure(&original.package, &original.proof, inputs, limits)?;
    serde_json::to_value(logical.coordinate()).map_err(|_| "coordinate")
}
