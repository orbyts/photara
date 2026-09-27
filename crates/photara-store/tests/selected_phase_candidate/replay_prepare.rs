//! Preparation-only PS1 projection from actual authenticated prefix objects.
//! No historical corpus, prospective package, journal acceptance or native filesystem access.
use super::{prepare, wire};
use photara_core::{NodeDefinitionRegistry, ValueTypeRegistry, creation::PackageExtension};
use photara_store::package::{MemoryPackage, PackageLimits, planning::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, fields, hash, key, number, reference, text};

fn objects(old: &wire::World) -> Result<BTreeMap<wire::Key, Value>> {
    let mut values = BTreeMap::new();
    let mut total = 0usize;
    for allocation in old.allocations.values() {
        total = total
            .checked_add(allocation.bytes.len())
            .ok_or("replay prefix size overflow")?;
        ensure(total <= 16 * 1024 * 1024, "replay prefix bound")?;
        let bytes = &allocation.bytes;
        let mut offset = 0usize;
        while offset < bytes.len() {
            ensure(
                bytes.len() - offset >= 16 && &bytes[offset..offset + 8] == b"PS2PKD01",
                "replay original frame header",
            )?;
            ensure(
                bytes[offset + 8] <= 3 && bytes[offset + 9..offset + 12] == [0, 0, 0],
                "replay original frame dispatch",
            )?;
            let length = usize::try_from(u32::from_le_bytes(
                bytes[offset + 12..offset + 16].try_into().unwrap(),
            ))
            .map_err(|_| "replay frame length")?;
            let end = offset
                .checked_add(16)
                .and_then(|p| p.checked_add(length))
                .ok_or("replay original frame overflow")?;
            ensure(end <= bytes.len(), "replay complete original frame")?;
            if bytes[offset + 8] == 0 {
                ensure(
                    bytes[offset + 16..end].iter().all(|b| *b == 0),
                    "replay canonical original padding",
                )?;
            } else {
                let value = wire::parse(&bytes[offset + 16..end])?;
                let identity = key(&reference(&value))?;
                if let Some(prior) = values.insert(identity, value.clone()) {
                    ensure(prior == value, "replay exact shared original bytes")?;
                }
                ensure(values.len() <= 4096, "replay original object bound")?;
            }
            offset = end;
        }
    }
    Ok(values)
}
#[allow(
    clippy::too_many_lines,
    reason = "Bounded typed legacy closure supplies the independent PS1 validator"
)]
fn legacy_closure(
    pool: &BTreeMap<wire::Key, Value>,
    state: &Value,
    authenticated: &BTreeSet<wire::Key>,
) -> Result<BTreeMap<wire::Key, Value>> {
    fn visit(
        pool: &BTreeMap<wire::Key, Value>,
        r: &Value,
        allowed: &BTreeSet<wire::Key>,
        out: &mut BTreeMap<wire::Key, Value>,
        depth: usize,
    ) -> Result<()> {
        ensure(depth <= 64, "replay legacy closure depth")?;
        let identity = key(r)?;
        ensure(
            allowed.contains(&identity),
            "replay object outside authenticated active semantic closure",
        )?;
        if out.contains_key(&identity) {
            return Ok(());
        }
        let value = pool
            .get(&identity)
            .ok_or("replay actual original semantic object missing")?
            .clone();
        ensure(
            reference(&value) == *r,
            "replay exact original semantic reference",
        )?;
        out.insert(identity, value.clone());
        let mut edges = Vec::new();
        match text(&value["schema"]["id"])? {
            "photara.project.authored" => {
                for f in [
                    "party_assignments",
                    "location_assignments",
                    "asset_ledger",
                    "resource_ledger",
                    "context",
                ] {
                    edges.push(value[f].clone());
                }
                for g in value["graphs"].as_array().ok_or("replay authored graphs")? {
                    edges.push(g["document"].clone());
                }
            }
            "photara.project.saved-graph" => {
                edges.push(value["context"].clone());
                for f in ["required_packages", "node_contracts"] {
                    for item in value[f].as_array().ok_or("replay graph dependencies")? {
                        edges.push(item["manifest"].clone());
                    }
                }
            }
            "photara.context.authored" => {
                for f in [
                    "variables",
                    "expressions",
                    "captures",
                    "metadata_selections",
                ] {
                    ensure(
                        value[f].as_array().is_some_and(Vec::is_empty),
                        "replay supported empty context codec",
                    )?;
                }
                for item in value["node_contexts"]
                    .as_array()
                    .ok_or("replay node contexts")?
                {
                    edges.push(item["context"].clone());
                }
            }
            "photara.project.party-assignments" | "photara.project.location-assignments" => ensure(
                value["assignments"].as_array().is_some_and(Vec::is_empty),
                "replay supported empty assignment codec",
            )?,
            "photara.project.asset-ledger" => ensure(
                value["assets"].as_array().is_some_and(Vec::is_empty),
                "replay supported empty assets codec",
            )?,
            "photara.project.resource-ledger" => {
                for f in ["managed_resources", "external_resources", "artifacts"] {
                    ensure(
                        value[f].as_array().is_some_and(Vec::is_empty),
                        "replay supported empty resource codec",
                    )?;
                }
            }
            "photara.project.history" => {
                for f in [
                    "runs",
                    "operations",
                    "evidence",
                    "snapshots",
                    "metadata_observations",
                    "proposals",
                    "apply_receipts",
                    "artifacts",
                ] {
                    ensure(
                        value[f].as_array().is_some_and(Vec::is_empty),
                        "replay supported empty history codec",
                    )?;
                }
            }
            "photara.node.manifest" => {}
            _ => return Err("replay unsupported legacy dependency codec"),
        }
        for edge in edges {
            visit(pool, &edge, allowed, out, depth + 1)?;
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    for field in ["authored", "history"] {
        visit(pool, &state[field], authenticated, &mut out, 0)?;
    }
    Ok(out)
}
/// `old` is the bounded original-prefix view authenticated by the route. `proof` is
/// its actual full selected proof. The caller separately covers all currently live
/// suffixes and authenticates intent/receipt references from the selected original.
#[allow(
    clippy::too_many_lines,
    reason = "One explicit preparation-only projection and actual Core result check"
)]
pub(super) fn regenerate(
    old: &wire::World,
    proof: &wire::Proof,
    intent: &Value,
    receipt: &Value,
) -> Result<prepare::Prepared> {
    wire::schema(
        receipt,
        "photara.package.operation-receipt",
        1,
        &[
            "library_id",
            "bootstrap_sha256",
            "operation_id",
            "request_sha256",
            "acceptance_ordinal",
            "journal_id",
            "journal_sequence",
            "outcome",
            "before",
            "after",
            "undo_group_id",
            "provenance",
        ],
    )?;
    let active = proof
        .roles
        .get("active")
        .ok_or("replay original active proof")?;
    ensure(
        reference(&active.state) == old.commit["root_set"]["active"],
        "replay original selected active identity",
    )?;
    fields(
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
    )?;
    ensure(
        intent["domain"] == "photara.package.operation-intent.v1"
            && intent["version"] == 1
            && intent["boundary"] == "single"
            && intent["undo_group_id"].is_null(),
        "replay explicit original intent codec",
    )?;
    ensure(
        intent["project_id"] == old.manifest["project_id"]
            && intent["library_id"] == old.commit["root_set"]["library_id"]
            && intent["bootstrap_sha256"] == hash(&encode(&old.manifest)),
        "replay original package binding",
    )?;
    let pool = objects(old)?;
    let closure = legacy_closure(&pool, &active.state, &active.semantic)?;
    let mut files = BTreeMap::new();
    for (identity, value) in &closure {
        files.insert(
            format!("objects/json/sha256/{}.json", identity.0),
            encode(value),
        );
    }
    let inventory = json!({"schema":{"id":"photara.package.inventory","version":1},"project_id":old.manifest["project_id"],"objects":closure.values().map(reference).collect::<Vec<_>>()});
    let inventory_ref = reference(&inventory);
    files.insert(
        format!(
            "objects/json/sha256/{}.json",
            text(&inventory_ref["sha256"])?
        ),
        encode(&inventory),
    );
    // These fresh wrappers describe only a temporary logical preparation input.
    // Their IDs/hash are never presented as original HEAD or selected Saved evidence.
    let commit = json!({"schema":{"id":"photara.package.commit","version":1},"project_id":old.manifest["project_id"],
        "commit_id":"97000000-0000-4000-8000-000000000001","write_id":"97000000-0000-4000-8000-000000000002",
        "package_revision":"1","parent":null,"created_at":old.manifest["created_at"],"bootstrap_sha256":hash(&encode(&old.manifest)),
        "minimum_reader":old.manifest["format_version"],"required_features":old.manifest["required_features"],
        "authored":active.state["authored"],"history":active.state["history"],"inventory":inventory_ref});
    let head = json!({"schema":{"id":"photara.package.head","version":1},"project_id":old.manifest["project_id"],"commit_id":commit["commit_id"],"commit_sha256":hash(&encode(&commit))});
    files.insert("manifest.json".to_owned(), encode(&old.manifest));
    files.insert("HEAD.json".to_owned(), encode(&head));
    files.insert(
        "commits/97000000-0000-4000-8000-000000000001.json".to_owned(),
        encode(&commit),
    );
    let logical = VerifiedClosure::verify(
        MemoryPackage::new(files, PackageLimits::default())
            .map_err(|_| "replay temporary logical package")?,
        IncarnationId::parse("97000000-0000-4000-8000-000000000003").unwrap(),
    )
    .map_err(|_| "replay actual legacy closure verification")?;
    let coordinate =
        serde_json::to_value(logical.coordinate()).map_err(|_| "replay coordinate encoding")?;
    ensure(
        coordinate == intent["expected"]
            && coordinate["authored_digest"] == active.state["authored"]["sha256"]
            && coordinate["revision"] == active.state["authored_revision"],
        "replay actual original authored coordinate",
    )?;
    fields(&intent["command"], &["kind", "envelope"])?;
    ensure(intent["command"]["kind"] == "graph", "replay Graph command")?;
    let mutation = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone())
            .map_err(|_| "replay operation identity")?,
        expected: logical.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(
                serde_json::from_value(intent["command"]["envelope"].clone())
                    .map_err(|_| "replay actual Graph envelope")?,
            ),
        },
        updated_at: text(&intent["updated_at"])?.to_owned(),
    };
    let naming = PackageNamingPolicy {
        write_extension: PackageExtension::try_from("jprtest".to_owned()).unwrap(),
        legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
    };
    let result = plan(
        &logical,
        PlanRequest {
            mutation: &mutation,
            expected_head: logical.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse("97000000-0000-4000-8000-000000000004").unwrap(),
                commit_id: serde_json::from_value(json!("97000000-0000-4000-8000-000000000005"))
                    .unwrap(),
            },
            naming: &naming,
            observed_extension: &naming.write_extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .map_err(|_| "replay actual Core preparation")?;
    let PlanOutcome::Checkpoint(result) = result else {
        return Err("replay supported changed Graph result");
    };
    ensure(
        receipt["schema"] == json!({"id":"photara.package.operation-receipt","version":1})
            && receipt["operation_id"]
                == serde_json::to_value(result.receipt().operation_id()).unwrap()
            && receipt["request_sha256"] == hash(&encode(intent))
            && receipt["project_id"] == intent["project_id"]
            && receipt["library_id"] == intent["library_id"]
            && receipt["bootstrap_sha256"] == intent["bootstrap_sha256"]
            && receipt["outcome"] == "accepted"
            && receipt["undo_group_id"] == intent["undo_group_id"],
        "replay original receipt template identity",
    )?;
    for (coordinate, field) in [
        (result.receipt().before(), "before"),
        (result.receipt().after(), "after"),
    ] {
        let value = serde_json::to_value(coordinate).unwrap();
        ensure(
            receipt[field]
                == json!({"revision":value["revision"],"digest":value["authored_digest"]}),
            "replay receipt derived from actual Core result",
        )?;
    }
    ensure(
        number(&receipt["acceptance_ordinal"])?
            == number(&active.state["accepted"]["through_ordinal"])?
                .checked_add(1)
                .ok_or("replay ordinal overflow")?
            && receipt["journal_id"] == active.state["journal_inclusion"]["journal_id"]
            && number(&receipt["journal_sequence"])?
                > number(&active.state["journal_inclusion"]["through_sequence"])?,
        "replay original next receipt coordinate",
    )?;
    let mut authored = BTreeMap::new();
    for (path, bytes) in result.candidate().files().files() {
        if path.starts_with("objects/json/sha256/") {
            let value = wire::parse(bytes.as_ref())?;
            authored.insert(key(&reference(&value))?, value);
        }
    }
    ensure(
        authored.keys().any(|k| k.0 == receipt["after"]["digest"]),
        "replay actual new authored bytes",
    )?;
    Ok(prepare::Prepared {
        authored,
        intent: intent.clone(),
        receipt: receipt.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> Result<(wire::World, wire::Proof, Value, Value)> {
        let selected = super::super::world("admitted");
        let (original, _) = super::super::phases::selected_original(&selected)?;
        let old = super::super::phases::original_view(&selected, &original)?;
        let proof = wire::verify(&old)?;
        let intent = selected.loose(&original["request"]["intent"])?;
        let receipt = selected.loose(&original["request"]["receipt"])?;
        Ok((old, proof, intent, receipt))
    }
    #[test]
    fn replay_preparation_needs_only_actual_original_semantics() -> Result<()> {
        let (mut old, proof, intent, receipt) = inputs()?;
        // The full original proof was established above. The preparation helper
        // has no current accounting authority and must not use these as fallback.
        old.source.clear();
        old.source_witnesses = Value::Null;
        old.loose.clear();
        old.journal.clear();
        let prepared = regenerate(&old, &proof, &intent, &receipt)?;
        ensure(
            prepared.intent == intent
                && prepared.receipt == receipt
                && prepared
                    .authored
                    .keys()
                    .any(|k| k.0 == receipt["after"]["digest"]),
            "original-prefix-only actual Core result",
        )
    }
    #[test]
    fn replay_preparation_missing_or_corrupt_actual_semantics_refuses() -> Result<()> {
        let (old, proof, intent, receipt) = inputs()?;
        let pool = objects(&old)?;
        let body = encode(&pool[&key(&proof.roles["active"].state["authored"])?]);
        for corrupt in [false, true] {
            let mut changed = old.clone();
            let mut found = false;
            if corrupt {
                for allocation in changed.allocations.values_mut() {
                    if let Some(offset) =
                        allocation.bytes.windows(body.len()).position(|b| b == body)
                    {
                        allocation.bytes[offset + body.len() - 1] = b']';
                        found = true;
                    }
                }
            } else {
                changed.allocations.retain(|_, a| {
                    let contains = a.bytes.windows(body.len()).any(|b| b == body);
                    found |= contains;
                    !contains
                });
            }
            ensure(found, "actual original body present in test")?;
            ensure(
                regenerate(&changed, &proof, &intent, &receipt).is_err(),
                "missing/corrupt original silently fell back",
            )?;
        }
        Ok(())
    }
    #[test]
    fn replay_preparation_original_intent_and_result_cannot_substitute() -> Result<()> {
        let (old, proof, intent, receipt) = inputs()?;
        for mode in ["coordinate", "request", "result", "ordinal", "unknown"] {
            let mut i = intent.clone();
            let mut r = receipt.clone();
            match mode {
                "coordinate" => i["expected"]["authored_digest"] = json!("0".repeat(64)),
                "request" => r["request_sha256"] = json!("0".repeat(64)),
                "result" => r["after"]["digest"] = json!("0".repeat(64)),
                "ordinal" => r["acceptance_ordinal"] = json!("5"),
                _ => r["unknown_required"] = json!(true),
            }
            ensure(
                regenerate(&old, &proof, &i, &r).is_err(),
                "substituted original preparation accepted",
            )?;
        }
        Ok(())
    }
}
