//! Sizing-only refusal for the proposed newborn fields, not a selected newborn proof.
use super::wire::{encode, hash, reference};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn obj(name: &str, mut v: Value) -> Value {
    v["schema"] = json!({"id":format!("photara.storage.{name}"),"version":1});
    v["project_id"] = json!(super::wire::PROJECT);
    v["extensions"] = json!({});
    v
}
fn aid(n: u64) -> String {
    format!("96000000-0000-4000-8000-{:012}", 1000 + n)
}
fn round(n: usize) -> u64 {
    (n as u64).div_ceil(4096) * 4096
}
fn controls(snapshot: &Value) -> Vec<Value> {
    snapshot["loose"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| serde_json::from_str(v.as_str().unwrap()).unwrap())
        .collect()
}
fn named<'a>(values: &'a [Value], name: &str) -> &'a Value {
    values
        .iter()
        .find(|v| v["schema"]["id"] == format!("photara.storage.{name}"))
        .unwrap()
}
fn coexistence(a: &Value, b: &Value) -> (u64, usize) {
    let mut files = BTreeMap::new();
    for snapshot in [a, b] {
        for (key, value) in snapshot["loose"].as_object().unwrap() {
            files.insert(key.clone(), value.as_str().unwrap().len());
        }
        for name in ["manifest", "head", "commit"] {
            let bytes = snapshot["bootstrap"][name].as_str().unwrap().as_bytes();
            files.insert(hash(bytes), bytes.len());
        }
    }
    // Exact same selected-phase model: intent, staged HEAD and directory allowance.
    (
        files.values().map(|n| round(*n)).sum::<u64>() + 12_288,
        files.len() + 3,
    )
}

fn proposed_plan(o: &Value) -> Value {
    let slots = [9, 10].map(|n| json!({"ordinal":(n-8).to_string(),"allocation_id":aid(n),"arena":"data","stage_path":["birth",format!("slot-{n}")],"final_path":["packs",format!("data-{n}")],"maximum_extent":"524288","marker_allocation_bound":"4096"}));
    let corridor = [3, 4, 5, 6, 7, 8, 10].map(|n| json!({"allocation_id":aid(n),"arena":if n%2==1 || n==8 {"metadata"} else {"data"},"start":"262144","maximum_end":"524288"}));
    let mut bounds = o["control_bounds"].clone();
    bounds["marker_bytes"] = json!("4096");
    bounds["newborn_binding_bytes"] = json!("4096");
    let shape = json!({"old_head_sha256":hash(&encode(&o["old"]["head"])),"payload_recipe_sha256":hash(&encode(&o["payload_recipe"])),"semantic_target_sha256":hash(&encode(&o["semantic_target"])),"slots":slots,"corridor":corridor,"finalizer_codec":o["payload_recipe"]["codec"]});
    obj(
        "generation-plan",
        json!({"token":o["token"],"nonce":o["nonce"],"scope":o["scope"],"request_sha256":o["request"]["request_sha256"],"old_head_sha256":shape["old_head_sha256"],"reserve":o["reserve"],"cleanup_bound":o["cleanup_bound"],"project_limit":o["project_limit"],"payload_codec":o["payload_recipe"]["codec"],"payload_recipe_sha256":shape["payload_recipe_sha256"],"semantic_target_sha256":shape["semantic_target_sha256"],"slots":slots,"finalizer_codec":o["payload_recipe"]["codec"],"corridor":corridor,"shape_sha256":hash(&encode(&shape)),"control_bounds":bounds}),
    )
}

// Values below are sizing sentinels, never observations, accepted recipes or authority.
// Extent/count widths are bounded by 524288-byte corridors, not unbounded u64 ends.
fn finalizer_size(f: &Value, ledger: &Value, plan: &Value, binding: &Value) -> Value {
    let mut f = f.clone();
    f["generation_plan"] = reference(plan);
    f["newborn_binding"] = reference(binding);
    let sealed = [2, 9].map(|n| {
        let mut observation = ledger["tips"][0]["observation"].clone();
        observation["subject"]["allocation_id"] = json!(aid(n));
        observation["observation_id"] = json!(format!("96000000-0000-4000-8000-{:012}",3000+n));
        observation["physical"] = json!({"device":u64::MAX.to_string(),"inode":u64::MAX.to_string()});
        observation["measured_extent"] = json!("524288");
        observation["charged_high_water"] = json!("524288");
        let charge = obj("sealed-charge",json!({"allocation_id":aid(n),"arena":"data","domain_incarnation":observation["incarnation"],"measured_extent":"524288","charged_high_water":"524288","observation":reference(&observation)}));
        json!({"allocation_id":aid(n),"observation":observation,"charge":charge})
    });
    f["sealed_allocations"] = json!(sealed);
    f["payload_completion"][0]["allocation_id"] = json!(aid(9));
    let mut extra = f["payload_completion"][0].clone();
    extra["allocation_id"] = json!(aid(10));
    f["payload_completion"].as_array_mut().unwrap().push(extra);
    f["writes"][0]["allocation_id"] = json!(aid(10));
    for kind in ["payload_completion", "writes"] {
        for range in f[kind].as_array_mut().unwrap() {
            if range["allocation_id"] == aid(9) || range["allocation_id"] == aid(10) {
                range["witness"] =
                    json!({"device":u64::MAX.to_string(),"inode":u64::MAX.to_string()});
            }
            range["original_end"] = json!("524288");
            range["final_end"] = json!("524288");
            range["frame_count"] = json!("32768");
            range["original_sha256"] = json!("f".repeat(64));
            if kind == "payload_completion" {
                range["framed_bytes"] = json!("524288");
            } else {
                range["recipe_sha256"] = json!("f".repeat(64));
            }
        }
    }
    f
}

#[test]
fn newborn_exported_fields_exceed_original_control_pool_before_effects() {
    let corpus = super::corpus();
    let snapshots = &corpus["snapshots"];
    let values = controls(&snapshots["published"]);
    let original = named(&values, "original-admission");
    let plan = proposed_plan(original);
    let markers = [9, 10].map(|n| obj("generation-marker",json!({"token":original["token"],"generation_plan":reference(&plan),"nonce":original["nonce"],"ordinal":(n-8).to_string(),"allocation_id":aid(n),"arena":"data"})));
    let binding = obj(
        "newborn-binding",
        json!({"token":original["token"],"generation_plan":reference(&plan),"slots":markers.iter().enumerate().map(|(i,m)|json!({"ordinal":(i+1).to_string(),"state":"promoted","witness":{"device":u64::MAX.to_string(),"inode":u64::MAX.to_string(),"marker_byte_length":encode(m).len().to_string(),"marker_sha256":hash(&encode(m)),"marker_observed_charge":"4096"}})).collect::<Vec<_>>()}),
    );
    let f = finalizer_size(
        named(&values, "finalization-control"),
        named(&values, "ledger"),
        &plan,
        &binding,
    );
    let (base_bytes, base_roles) =
        coexistence(&snapshots["finalizer-durable"], &snapshots["published"]);
    assert_eq!((base_bytes, base_roles), (118_784, 22));
    assert!(encode(&plan).len() <= 4096 && encode(&binding).len() <= 4096);
    assert!(markers.iter().all(|m| encode(m).len() <= 4096));
    assert!(encode(&f).len() > 12_288 && encode(&f).len() <= 16_384);
    // Necessary lower bound only: unchanged-role rounding is retained; larger O/P,
    // future ledger widths and marker coexistence can only worsen this estimate.
    let required = base_bytes
        + round(encode(&plan).len())
        + round(encode(&binding).len())
        + round(encode(&f).len())
        - round(encode(named(&values, "finalization-control")).len());
    assert_eq!((required, base_roles + 2), (135_168, 24));
    assert!(required > 131_072);
    let mut compact = f.clone();
    // Smallest measured proposal: derive the seven recipe hashes through the
    // authenticated original codec; retain every prefix/raw-suffix hash and length.
    for range in compact["writes"].as_array_mut().unwrap() {
        range.as_object_mut().unwrap().remove("recipe_sha256");
    }
    assert_eq!(encode(&compact).len(), 12_270);
    assert_eq!(
        required - round(encode(&f).len()) + round(encode(&compact).len()),
        131_072
    );
    println!(
        "newborn sizing refusal: plan={} binding={} markers={:?} F={} compact-F={} lower-bound={required}/{} cap=131072/24; marker allocation peak={} per live marker (not a control-cap rebate)",
        encode(&plan).len(),
        encode(&binding).len(),
        markers.iter().map(|m| encode(m).len()).collect::<Vec<_>>(),
        encode(&f).len(),
        encode(&compact).len(),
        base_roles + 2,
        4096
    );
}

#[test]
fn one_born_sealed_pack_keeps_original_corridor_without_field_changes() {
    let corpus = super::corpus();
    let snapshots = &corpus["snapshots"];
    let values = controls(&snapshots["published"]);
    let original = named(&values, "original-admission");
    let mut plan = proposed_plan(original);
    plan["slots"].as_array_mut().unwrap().truncate(1);
    plan["corridor"][6]["allocation_id"] = json!(aid(2));
    plan["corridor"].as_array_mut().unwrap().sort_by_key(|v| {
        (
            v["arena"].as_str().unwrap().to_owned(),
            v["allocation_id"].as_str().unwrap().to_owned(),
        )
    });
    let shape = json!({"old_head_sha256":plan["old_head_sha256"],"payload_recipe_sha256":plan["payload_recipe_sha256"],"semantic_target_sha256":plan["semantic_target_sha256"],"slots":plan["slots"],"corridor":plan["corridor"],"finalizer_codec":plan["finalizer_codec"]});
    plan["shape_sha256"] = json!(hash(&encode(&shape)));
    let marker = obj(
        "generation-marker",
        json!({"token":original["token"],"generation_plan":reference(&plan),"nonce":original["nonce"],"ordinal":"1","allocation_id":aid(9),"arena":"data"}),
    );
    let binding = obj(
        "newborn-binding",
        json!({"token":original["token"],"generation_plan":reference(&plan),"slots":[{"ordinal":"1","state":"promoted","witness":{"device":u64::MAX.to_string(),"inode":u64::MAX.to_string(),"marker_byte_length":encode(&marker).len().to_string(),"marker_sha256":hash(&encode(&marker)),"marker_observed_charge":"4096"}}]}),
    );
    let mut f = finalizer_size(
        named(&values, "finalization-control"),
        named(&values, "ledger"),
        &plan,
        &binding,
    );
    f["sealed_allocations"].as_array_mut().unwrap().remove(0);
    f["payload_completion"][0]["allocation_id"] = json!(aid(2));
    f["payload_completion"][0]["witness"] =
        named(&values, "finalization-control")["payload_completion"][0]["witness"].clone();
    f["payload_completion"][7]["allocation_id"] = json!(aid(9));
    f["writes"][0]["allocation_id"] = json!(aid(2));
    f["writes"][0]["witness"] =
        named(&values, "finalization-control")["writes"][0]["witness"].clone();
    let (base_bytes, base_roles) =
        coexistence(&snapshots["finalizer-durable"], &snapshots["published"]);
    let bound = base_bytes
        + round(encode(&plan).len())
        + round(encode(&binding).len())
        + round(encode(&f).len())
        - round(encode(named(&values, "finalization-control")).len());
    assert!(encode(&f).len() <= 12_288);
    assert_eq!((bound, base_roles + 2), (131_072, 24));
    println!(
        "one born sealed sizing: plan={} binding={} marker={} F={} publication-bound={bound}/{}; all proposed fields retained; not yet full admission preflight",
        encode(&plan).len(),
        encode(&binding).len(),
        encode(&marker).len(),
        encode(&f).len(),
        base_roles + 2
    );
}
