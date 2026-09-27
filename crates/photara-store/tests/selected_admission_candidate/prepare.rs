//! Actual Core/PS1 preparation oracle, not an accepted journal result.
use super::wire::{Result, ensure, key, parse};
use photara_core::{NodeDefinitionRegistry, ValueTypeRegistry, creation::PackageExtension};
use photara_store::package::{MemoryPackage, PackageLimits, planning::*};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/route/operation-four.json"
    ))
    .unwrap()
}
fn apply(base: &VerifiedClosure, intent: &Value, suffix: u64) -> Result<PlanOutcome> {
    ensure(
        serde_json::to_value(base.coordinate()).unwrap() == intent["expected"],
        "prepared original authored coordinate",
    )?;
    let mutation = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone())
            .map_err(|_| "operation id")?,
        expected: base.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(
                serde_json::from_value(intent["command"]["envelope"].clone())
                    .map_err(|_| "Graph envelope")?,
            ),
        },
        updated_at: intent["updated_at"].as_str().ok_or("updated_at")?.into(),
    };
    let naming = PackageNamingPolicy {
        write_extension: PackageExtension::try_from("jprtest".to_owned()).unwrap(),
        legacy_read_extensions: vec![PackageExtension::legacy_creation_alias()],
    };
    plan(
        base,
        PlanRequest {
            mutation: &mutation,
            expected_head: base.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse(&format!("50000000-0000-4000-8000-{suffix:012}")).unwrap(),
                commit_id: serde_json::from_value(json!(format!(
                    "50000000-0000-4000-8000-{:012}",
                    suffix + 1
                )))
                .unwrap(),
            },
            naming: &naming,
            observed_extension: &naming.write_extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .map_err(|_| "Core preparation")
}
pub(super) struct Prepared {
    pub(super) authored: BTreeMap<super::wire::Key, Value>,
    pub(super) intent: Value,
    pub(super) receipt: Value,
}
pub(super) fn verify(old: &super::wire::Proof) -> Result<Prepared> {
    let c = corpus();
    let files = c["base_files"]
        .as_object()
        .ok_or("base files")?
        .iter()
        .map(|(k, v)| {
            Ok((
                k.clone(),
                v.as_str().ok_or("base bytes")?.as_bytes().to_vec(),
            ))
        })
        .collect::<Result<_>>()?;
    let base = VerifiedClosure::verify(
        MemoryPackage::new(files, PackageLimits::default()).map_err(|_| "base package")?,
        IncarnationId::parse("50000000-0000-4000-8000-000000000090").unwrap(),
    )
    .map_err(|_| "original PS1 closure")?;
    let PlanOutcome::Checkpoint(first) = apply(&base, &c["records"]["intent-1"]["input"], 100)?
    else {
        return Err("first checkpoint");
    };
    ensure(
        matches!(
            apply(first.candidate(), &c["records"]["intent-2"]["input"], 200)?,
            PlanOutcome::Unchanged(_)
        ),
        "original noop",
    )?;
    let PlanOutcome::Checkpoint(third) =
        apply(first.candidate(), &c["records"]["intent-3"]["input"], 300)?
    else {
        return Err("third checkpoint");
    };
    let before = serde_json::to_value(third.candidate().coordinate()).unwrap();
    let active = &old.roles["active"].state;
    ensure(
        active["authored_revision"] == before["revision"]
            && active["authored"]["sha256"] == before["authored_digest"],
        "actual selected active matches prepared old coordinate",
    )?;
    let intent = c["records"]["intent-4"]["input"].clone();
    let PlanOutcome::Checkpoint(fourth) = apply(third.candidate(), &intent, 400)? else {
        return Err("fourth checkpoint");
    };
    let receipt = c["records"]["receipt-4"]["input"].clone();
    ensure(
        serde_json::to_value(fourth.receipt().operation_id()).unwrap() == receipt["operation_id"]
            && super::wire::hash(&super::wire::encode(&intent)) == receipt["request_sha256"],
        "actual Core prepared operation and request identity",
    )?;
    for (coordinate, field) in [
        (fourth.receipt().before(), "before"),
        (fourth.receipt().after(), "after"),
    ] {
        let v = serde_json::to_value(coordinate).unwrap();
        ensure(
            receipt[field] == json!({"revision":v["revision"],"digest":v["authored_digest"]}),
            "prepared receipt actual result",
        )?;
    }
    ensure(
        old.roles["active"].receipts.len() == 3
            && !old.roles["active"]
                .receipts
                .iter()
                .any(|r| r["operation_id"] == receipt["operation_id"]),
        "prepared operation not accepted",
    )?;
    let mut authored = BTreeMap::new();
    for (path, bytes) in fourth.candidate().files().files() {
        if path.starts_with("objects/json/sha256/") {
            let v = parse(bytes.as_ref())?;
            authored.insert(key(&super::wire::reference(&v))?, v);
        }
    }
    for name in ["graph-3", "authored-3"] {
        let v = &c["records"][name]["input"];
        ensure(
            authored.get(&key(&super::wire::reference(v))?) == Some(v),
            "actual prepared canonical bytes",
        )?;
    }
    Ok(Prepared {
        authored,
        intent,
        receipt,
    })
}
#[test]
fn selected_admission_prepares_real_unaccepted_fourth_operation() -> Result<()> {
    let c: Value = serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/integrated/linked.json"
    ))
    .unwrap();
    let w = super::wire::World::load(&c)?;
    let p = super::wire::verify(&w)?;
    verify(&p)?;
    Ok(())
}
