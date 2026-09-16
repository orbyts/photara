//! I/O-free, single-RenameGraph replay evidence. This is a disposable test
//! contract, not a journal format, durable receipt or production recovery API.
use super::*;
use photara_core::{
    GraphId,
    contracts::ids::{CommitId, OperationId},
};
use serde::{Deserialize, Serialize};

#[path = "replay_journal.rs"]
mod framed;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    kind: String,
    graph_id: GraphId,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    operation_id: OperationId,
    expected: Value,
    command: Rename,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    version: u32,
    mutation: Request,
    request_digest: String,
    before: Value,
    after: Value,
    incarnation: String,
    manifest_digest: String,
    base_head_digest: String,
    write_id: String,
    commit_id: CommitId,
    candidate_head_digest: String,
    candidate_files: BTreeMap<String, String>,
}

fn file_digests(base: &VerifiedClosure) -> BTreeMap<String, String> {
    base.files()
        .files()
        .iter()
        .map(|(name, bytes)| (name.clone(), hash(bytes)))
        .collect()
}

fn prepare(base: &VerifiedClosure, mutation: &MutationRequest, n: u32) -> Vec<u8> {
    let plan = checkpoint(run(base, mutation, n).unwrap());
    canonical_json(&Evidence {
        version: 1,
        mutation: decode(serde_json::to_value(mutation).unwrap()),
        request_digest: plan.receipt().request_digest().as_str().into(),
        before: serde_json::to_value(plan.receipt().before()).unwrap(),
        after: serde_json::to_value(plan.receipt().after()).unwrap(),
        incarnation: incarnation().uuid().to_string(),
        manifest_digest: base.token().manifest_digest().as_str().into(),
        base_head_digest: base.token().head_digest().as_str().into(),
        write_id: plan.ids().write_id.uuid().to_string(),
        commit_id: plan.ids().commit_id,
        candidate_head_digest: plan.candidate().token().head_digest().as_str().into(),
        candidate_files: file_digests(plan.candidate()),
    })
    .unwrap()
}

fn replay(base: &VerifiedClosure, bytes: &[u8]) -> Result<Box<CheckpointPlan>, PlanError> {
    let value = package::parse_canonical_json(bytes, package::JsonLimits::default())?;
    let evidence: Evidence = serde_json::from_value(value).map_err(|_| PlanError::Validation)?;
    if canonical_json(&evidence).map_err(|_| PlanError::Validation)? != bytes
        || evidence.version != 1
        || evidence.mutation.version != 1
        || evidence.mutation.command.kind != "rename-graph"
    {
        return Err(PlanError::UnsupportedCommand);
    }
    base.token().compare(
        IncarnationId::parse(&evidence.incarnation)?,
        &base.files().files()["manifest.json"],
        base.token().head_bytes(),
    )?;
    if evidence.manifest_digest != base.token().manifest_digest().as_str()
        || evidence.base_head_digest != base.token().head_digest().as_str()
    {
        return Err(PlanError::ExternalChange);
    }
    let before = serde_json::to_value(base.coordinate()).map_err(|_| PlanError::Validation)?;
    if evidence.before != before || evidence.mutation.expected != before {
        return Err(PlanError::RevisionConflict);
    }
    let request = MutationRequest {
        version: evidence.mutation.version,
        operation_id: evidence.mutation.operation_id,
        expected: base.coordinate().clone(),
        command: AuthoredCommand::RenameGraph {
            graph_id: evidence.mutation.command.graph_id,
            name: evidence.mutation.command.name.clone(),
        },
        updated_at: evidence.mutation.updated_at.clone(),
    };
    let request_bytes = canonical_json(&request).map_err(|_| PlanError::Validation)?;
    if request_bytes != canonical_json(&evidence.mutation).map_err(|_| PlanError::Validation)?
        || hash(&request_bytes) != evidence.request_digest
    {
        return Err(PlanError::Validation);
    }
    let policy = naming();
    let outcome = plan(
        base,
        PlanRequest {
            mutation: &request,
            expected_head: base.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse(&evidence.write_id)?,
                commit_id: evidence.commit_id,
            },
            naming: &policy,
            observed_extension: &policy.write_extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )?;
    let PlanOutcome::Checkpoint(plan) = outcome else {
        return Err(PlanError::Validation);
    };
    if evidence.after
        != serde_json::to_value(plan.receipt().after()).map_err(|_| PlanError::Validation)?
        || evidence.candidate_head_digest != plan.candidate().token().head_digest().as_str()
        || evidence.candidate_files != file_digests(plan.candidate())
    {
        return Err(PlanError::Validation);
    }
    Ok(plan)
}

#[test]
fn single_rename_replay_matches_exact_bytes_and_refuses_tampering_or_wrong_base() {
    let base = verified(build(|o| {
        o.get_mut("authored").unwrap()["future_optional"] = json!({"unknown":[1,null,"é"]});
        o.get_mut("graph").unwrap()["future_optional"] = json!({"preserved":true});
    }));
    let original = owned(base.files());
    let mutation = rename(&base, "Recovered name");
    let expected = checkpoint(run(&base, &mutation, 33000).unwrap());
    let bytes = prepare(&base, &mutation, 33000);
    let recovered = replay(&base, &bytes).unwrap();
    assert_eq!(
        owned(expected.candidate().files()),
        owned(recovered.candidate().files())
    );
    assert_eq!(
        expected.receipt().request_digest(),
        recovered.receipt().request_digest()
    );
    assert_eq!(expected.receipt().after(), recovered.receipt().after());
    let mut changed: Value = serde_json::from_slice(&bytes).unwrap();
    changed["mutation"]["command"]["name"] = json!("Tampered command");
    assert!(matches!(
        replay(&base, &canon(&changed)),
        Err(PlanError::Validation)
    ));
    // Matching a recomputed request hash still cannot bypass semantic/result
    // agreement: the recorded result and exact checkpoint must match the command.
    changed["request_digest"] = json!(hash(&canon(&changed["mutation"])));
    assert!(matches!(
        replay(&base, &canon(&changed)),
        Err(PlanError::Validation)
    ));
    assert!(matches!(
        replay(expected.candidate(), &bytes),
        Err(PlanError::ExternalChange)
    ));
    assert_eq!(owned(base.files()), original);
}
