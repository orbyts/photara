//! Direct v2 resource closure adapter for the new settled package reader.
//! Caller supplies a verified selected `StateRoot` identity and authenticated resolver.
//! Authored origins only; support is the frozen synthetic profile, not media access.
#[allow(
    dead_code,
    reason = "Reuse direct v2 validation; projection-only entry points remain unused"
)]
#[path = "../resource_factored_candidate/wire.rs"]
mod factored;
#[allow(
    dead_code,
    reason = "Reuse frozen typed resource helpers without changing their independent target"
)]
#[path = "../resource_conversion_candidate/wire.rs"]
mod legacy;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) struct Proof {
    pub(super) closure: BTreeSet<(String, u64)>,
    pub(super) supported: bool,
    pub(super) associations: BTreeMap<String, Value>,
}

/// Enumerate declared edges only. Unknown extension-shaped refs stay nonedges.
/// The frozen validator subsequently checks exact schema fields, versions, tree
/// structure and semantics; this collector confers no independent authority.
fn edges(value: &Value) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    match value["schema"]["id"].as_str().ok_or("resource schema")? {
        "photara.resource.state" => {
            for name in [
                "identities",
                "working_bindings",
                "versions",
                "backings",
                "requirements",
                "retention_sources",
            ] {
                result.push(value[name].clone());
            }
        }
        "photara.resource.selection-leaf" | "photara.resource.selection-branch" => {
            let leaf = value["schema"]["id"] == "photara.resource.selection-leaf";
            let entries = value[if leaf { "entries" } else { "children" }]
                .as_array()
                .ok_or("resource tree entries")?;
            legacy::ensure(entries.len() <= 256, "resource tree collection bound")?;
            for entry in entries {
                result.push(entry[if leaf { "record" } else { "child" }].clone());
            }
        }
        "photara.resource.retention-source" => result.push(value["requirements"].clone()),
        "photara.resource.captured-version" => result.push(value["capture_evidence"].clone()),
        "photara.resource.backing" => result.push(value["publication_evidence"].clone()),
        "photara.resource.identity"
        | "photara.resource.working-binding"
        | "photara.resource.retention-requirement"
        | "photara.resource.publication-evidence" => {}
        _ => return Err("unsupported selected resource schema"),
    }
    Ok(result)
}

/// Actual commit dispatch must independently require resource-backings.v1 and
/// resource-state-trees.v1. The context below projects those validated capabilities
/// solely into the frozen helper; it does not grant missing package capabilities.
pub(super) fn verify(
    state_ref: &Value,
    root_id: &str,
    resolve: impl FnMut(&Value) -> Result<Value>,
) -> Result<Proof> {
    verify_with_origin(state_ref, root_id, resolve, |source, context| {
        legacy::ensure(
            source["origin"] == json!({"kind":"authored","source_id":context["root_id"]}),
            "resolved selected source association",
        )
    })
}
/// The selected package policy must resolve non-authored authority before returning
/// a package proof. Collection, schema, subset and retention checks are unchanged.
pub(super) fn verify_with_origin(
    state_ref: &Value,
    root_id: &str,
    mut resolve: impl FnMut(&Value) -> Result<Value>,
    origin: impl FnMut(&Value, &Value) -> Result<()>,
) -> Result<Proof> {
    let mut records = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![state_ref.clone()];
    let mut total_bytes = 0usize;
    while let Some(reference) = pending.pop() {
        let key = legacy::reference(&reference)?;
        if !seen.insert(key.clone()) {
            continue;
        }
        legacy::ensure(seen.len() <= 4096, "resource collection count bound")?;
        // The top resolver parses bounded original canonical bytes and proves
        // selected locator membership. Rechecking their canonical value binds
        // this adapter to exactly the requested reference, not a lookalike value.
        let value = resolve(&reference)?;
        let bytes = photara_core::canonical_json(&value).map_err(|_| "resource canonical")?;
        legacy::ensure(
            bytes.len() as u64 == key.1 && legacy::hash(&bytes) == key.0,
            "resource resolved reference",
        )?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or("resource aggregate overflow")?;
        legacy::ensure(
            total_bytes <= 16 * 1024 * 1024,
            "resource aggregate byte bound",
        )?;
        let value = legacy::parse(&bytes)?;
        pending.extend(edges(&value)?);
        legacy::ensure(pending.len() <= 4096, "resource pending edge bound")?;
        records.insert(key.0, bytes);
    }
    let store = factored::Store {
        records,
        roots: json!({"selected": {
            "root_id":root_id,"resource_state":state_ref,
            "required_features":["photara.resource-backings.v1",factored::FEATURE]
        }}),
        profile: json!({
            "profile_id":"71000000-0000-4000-8000-000000000004",
            "profile_revision":"1",
            "failure_model_sha256":"ed13cf36565d400d39a1e89878c95c5a220be15aa78156630f7892fbb4e2d428"
        }),
        representation: Value::Null,
    };
    let proof = factored::verify_with_origin(&store, "selected", origin)?;
    legacy::ensure(proof.closure == seen, "resource exact collected closure")?;
    Ok(Proof {
        closure: proof.closure,
        supported: proof.supported,
        associations: proof.associations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/architecture/proposals/ps2/resource-factored/linked.json"
        ))
        .unwrap()
    }
    fn run(scenario: &Value, role: &str, root: &str) -> Result<Proof> {
        verify(
            &scenario["roots"][role]["resource_state"],
            root,
            |reference| {
                let sha = reference["sha256"].as_str().ok_or("reference hash")?;
                scenario["records"]
                    .get(sha)
                    .map(|r| r["input"].clone())
                    .ok_or("missing selected resource")
            },
        )
    }
    #[test]
    fn actual_context_closure_and_wrong_root() {
        let corpus = corpus();
        let scenario = &corpus["scales"]["16"];
        let a = run(
            scenario,
            "a",
            scenario["roots"]["a"]["root_id"].as_str().unwrap(),
        )
        .unwrap();
        let b = run(
            scenario,
            "b",
            scenario["roots"]["b"]["root_id"].as_str().unwrap(),
        )
        .unwrap();
        assert!(a.supported && b.supported);
        assert!(!a.closure.is_empty());
        assert!(!a.associations.is_empty());
        assert_eq!(
            run(
                scenario,
                "a",
                scenario["roots"]["b"]["root_id"].as_str().unwrap()
            )
            .err(),
            Some("resolved selected source association")
        );
    }
    #[test]
    fn structural_support_is_not_retention_or_media_proof() {
        let corpus = corpus();
        for name in ["two-copies", "wrong-qualification"] {
            let scenario = &corpus["scenarios"][name];
            let proof = run(
                scenario,
                "a",
                scenario["roots"]["a"]["root_id"].as_str().unwrap(),
            )
            .unwrap();
            assert!(!proof.supported, "{name}");
        }
        let scenario = &corpus["scenarios"]["unresolved-origin"];
        assert_eq!(
            run(
                scenario,
                "b",
                scenario["roots"]["b"]["root_id"].as_str().unwrap()
            )
            .err(),
            Some("resolved selected source association")
        );
    }
    #[test]
    fn resolver_cannot_substitute_canonical_object() {
        let corpus = corpus();
        let scenario = &corpus["scales"]["1"];
        assert_eq!(
            verify(
                &scenario["roots"]["a"]["resource_state"],
                scenario["roots"]["a"]["root_id"].as_str().unwrap(),
                |_| Ok(json!({}))
            )
            .err(),
            Some("resource resolved reference")
        );
    }
}
