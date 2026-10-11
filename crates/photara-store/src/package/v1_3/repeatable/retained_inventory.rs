//! Typed, append-only retained authority inventory for admission v2.
use super::{Builder, Result, Value, ensure, get, incremental, json, key};
use crate::package::{ObjectRef, v1_3::Membership};
use std::collections::BTreeMap;
const NAME: &str = "photara.storage.retained-control-inventory";
fn member(value: &Value) -> Result<Membership> {
    match value.as_str() {
        Some("semantic") => Ok(Membership::Semantic),
        Some("ownership") => Ok(Membership::Ownership),
        _ => Err("retained membership"),
    }
}
fn label(m: Membership) -> Result<&'static str> {
    match m {
        Membership::Semantic => Ok("semantic"),
        Membership::Ownership => Ok("ownership"),
        Membership::Blob => Err("retained Blob unsupported"),
    }
}
fn entries(values: &[Value]) -> Result<BTreeMap<ObjectRef, Membership>> {
    let mut result = BTreeMap::new();
    let mut previous = None;
    for v in values {
        let object = v.as_object().ok_or("retained entry")?;
        ensure(
            object.len() == 2 && object.contains_key("record") && object.contains_key("membership"),
            "retained fields",
        )?;
        let k = key(&v["record"])?;
        ensure(previous.as_ref().is_none_or(|p| p < &k), "retained order")?;
        previous = Some(k);
        let r: ObjectRef =
            serde_json::from_value(v["record"].clone()).map_err(|_| "retained ref")?;
        ensure(
            result.insert(r, member(&v["membership"])?).is_none(),
            "retained duplicate",
        )?;
    }
    Ok(result)
}
pub(super) fn build(
    builder: &mut Builder,
    old: &super::OriginalPackage,
    expected: &BTreeMap<ObjectRef, Membership>,
) -> Result<(Value, Vec<Value>)> {
    let root = &old.commit["root_set"];
    let envelope = old.loose(&root["placement"]["accounting"])?;
    let hold = old
        .loose(&envelope["holds"])
        .or_else(|_| get(&builder.pool, &envelope["holds"]))?;
    let hold_entries = hold["entries"].as_array().ok_or("original holds")?;
    ensure(hold_entries.len() <= 1, "single predecessor")?;
    let mut seed = None;
    let mut inherited = BTreeMap::new();
    let mut additions = Vec::new();
    let base = if let Some(entry) = hold_entries.first() {
        let original = old.loose(&entry["original"])?;
        let phase = old.loose(&entry["phase"])?;
        ensure(
            phase["stage"] == "clean"
                && phase["remaining"] == "0"
                && hold["remaining"] == "0"
                && phase["original"] == entry["original"],
            "completed retained predecessor",
        )?;
        let base = match original["original_codec"].as_str() {
            Some("photara.codec.ps2-repeatable-admission-v1") => {
                inherited = entries(
                    original["payload_recipe"]["retained_controls"]
                        .as_array()
                        .ok_or("legacy retained list")?,
                )?;
                json!({"kind":"legacy","original":entry["original"]})
            }
            Some(
                "photara.codec.ps2-repeatable-admission-v2"
                | "photara.codec.ps2-repeatable-admission-v3"
                | "photara.codec.ps2-repeatable-admission-v4",
            ) => {
                let r = original["payload_recipe"]["retained_control_inventory"]["result"].clone();
                let values = incremental::audit(
                    builder,
                    &r,
                    incremental::Spec {
                        name: NAME,
                        field: "record",
                        capacity: builder.geometry.semantic_leaf as usize,
                        allocation: None,
                    },
                )?;
                inherited = entries(&values)?;
                seed = Some(r.clone());
                json!({"kind":"tree","root":r})
            }
            _ => return Err("retained predecessor codec"),
        };
        let overlay = old.loose(&root["inventory"])?;
        for r in overlay["controls"]
            .as_array()
            .ok_or("predecessor controls")?
        {
            let body = old.loose(r)?;
            let membership = if body["schema"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("photara.storage."))
            {
                Membership::Ownership
            } else {
                Membership::Semantic
            };
            let reference: ObjectRef =
                serde_json::from_value(r.clone()).map_err(|_| "control ref")?;
            if let Some(existing) = inherited.insert(reference, membership) {
                ensure(existing == membership, "retained membership conflict")?;
            }
            additions.push(json!({"record":r,"membership":label(membership)?}));
        }
        base
    } else {
        json!({"kind":"empty"})
    };
    additions.sort_by_key(|e| key(&e["record"]).expect("checked retained ref"));
    ensure(inherited == *expected, "exact inherited retained controls")?;
    let values = expected
        .iter()
        .map(|(r, m)| Ok(json!({"record":r,"membership":label(*m)?})))
        .collect::<Result<Vec<_>>>()?;
    let (result, nodes) = incremental::tree(
        builder,
        seed.as_ref(),
        values,
        incremental::Spec {
            name: NAME,
            field: "record",
            capacity: builder.geometry.semantic_leaf as usize,
            allocation: None,
        },
    )?;
    // Result is a pre-publication commitment; only actual packed nodes resolve it.
    Ok((
        json!({"base":base,"additions":additions,"result":result}),
        nodes,
    ))
}
