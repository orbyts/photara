//! v3 writer uses the same typed overlay audit as the independent reader.
use super::{Builder, Result, Value, ensure, get, incremental, json, key, reference};
use crate::package::{
    PackageUuid,
    v1_3::{
        TreeLimits,
        overlay::{self, Family, Resolved},
    },
};
use std::collections::{BTreeMap, BTreeSet};
fn limits(b: &Builder, physical: bool) -> TreeLimits {
    TreeLimits {
        max_depth: b.limits.semantic.json.max_depth,
        max_pages: b.limits.max_objects,
        max_entries: b.limits.max_entries,
        max_leaf_entries: if physical {
            b.geometry.physical_leaf
        } else {
            b.geometry.semantic_leaf
        } as usize,
        max_branch_children: b.geometry.branch as usize,
    }
}
fn read(b: &Builder, r: &Value, allocation: Option<&str>) -> Result<Value> {
    if allocation.is_some() {
        b.physical_value(r)
    } else {
        get(&b.pool, r)
    }
}
pub(in super::super) fn resolve(
    b: &Builder,
    root: &Value,
    purpose: &str,
    role: Option<(&str, &str)>,
) -> Result<Resolved> {
    let family = if let Some((allocation, role)) = role {
        Family::Locator {
            role: Some(role),
            allocation,
        }
    } else {
        Family::Inventory {
            purpose: Some(purpose),
        }
    };
    overlay::resolve(
        |r| read(b, r, role.map(|r| r.0)).map_err(|_| crate::package::PackageError::Integrity),
        root,
        PackageUuid::parse(&b.project).map_err(|_| "overlay project")?,
        family,
        limits(b, role.is_some()),
    )
    .map_err(|_| "overlay typed audit")
}
fn write(b: &mut Builder, body: Value, allocation: Option<&str>) -> Result<Value> {
    crate::package::parse_canonical_json(&super::encode(&body), b.limits.semantic.json)
        .map_err(|_| "generated overlay JSON bound")?;
    if let Some(allocation) = allocation {
        b.pack(allocation, "metadata", vec![(body.clone(), 3)])?;
        Ok(b.locations[allocation][&key(&reference(&body))?].clone())
    } else {
        let r = b.add(body)?;
        let rank = b.changed_order.len();
        b.changed_order.entry(key(&r)?).or_insert(rank);
        Ok(r)
    }
}
pub(in super::super) fn build(
    b: &mut Builder,
    seed: &Value,
    target: Vec<Value>,
    purpose: &str,
    role: Option<(&str, &str)>,
) -> Result<(Value, Vec<Value>)> {
    let prior = resolve(b, seed, purpose, role)?;
    let physical = role.map(|r| r.0);
    let prefix = if physical.is_some() {
        "photara.storage.locator-overlay"
    } else {
        "photara.package.inventory-overlay"
    };
    let k = |v: &Value| key(if physical.is_some() { &v["object"] } else { v });
    let old = prior
        .entries
        .iter()
        .map(|v| Ok((k(v)?, v.clone())))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut next = BTreeMap::new();
    for v in target {
        ensure(next.insert(k(&v)?, v).is_none(), "overlay target duplicate")?;
    }
    let keys = old
        .keys()
        .chain(next.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut entries = Vec::new();
    for key in keys {
        if old.get(&key) == next.get(&key) {
            continue;
        }
        let value = next
            .get(&key)
            .or_else(|| old.get(&key))
            .ok_or("overlay diff")?;
        let object = if physical.is_some() {
            value["object"].clone()
        } else {
            value.clone()
        };
        let change = if physical.is_some() {
            if next.contains_key(&key) {
                json!({"kind":"put","membership":value["membership"],"physical":value["physical"]})
            } else {
                json!({"kind":"remove"})
            }
        } else {
            json!(if next.contains_key(&key) {
                "insert"
            } else {
                "remove"
            })
        };
        entries.push(json!({"object":object,"change":change}));
    }
    if entries.is_empty() {
        return Ok((
            seed.clone(),
            prior.nodes.into_iter().map(|(_, v)| v).collect(),
        ));
    }
    let sequence = prior
        .sequence
        .checked_add(1)
        .ok_or("overlay sequence overflow")?;
    let batch=b.object(&format!("{prefix}-batch"),json!({"sequence":sequence.to_string(),"before_count":old.len().to_string(),"after_count":next.len().to_string(),"entries":entries}));
    let batch_ref = write(b, batch, physical)?;
    let mut rows = prior.batches;
    rows.push(json!({"sequence":sequence.to_string(),"batch":batch_ref}));
    let capacity = if physical.is_some() {
        b.geometry.physical_leaf
    } else {
        b.geometry.semantic_leaf
    } as usize;
    let (index, _) = incremental::tree(
        b,
        prior.index.as_ref(),
        rows,
        incremental::Spec {
            name: &format!("{prefix}-index"),
            field: "sequence",
            capacity,
            allocation: physical,
        },
    )?;
    let mut body = json!({"base":prior.base,"base_count":prior.base_count.to_string(),"batches":index,"through_sequence":sequence.to_string(),"count":next.len().to_string()});
    if let Some((allocation, role)) = role {
        body["allocation_id"] = json!(allocation);
        body["role"] = json!(role);
    } else {
        body["purpose"] = json!(purpose);
    }
    let mut body = b.object(prefix, body);
    if physical.is_none() {
        body["schema"]["version"] = json!(2);
    }
    let result = write(b, body, physical)?;
    let checked = resolve(b, &result, purpose, role)?;
    ensure(
        checked.entries == next.into_values().collect::<Vec<_>>(),
        "overlay exact target",
    )?;
    Ok((result, checked.nodes.into_iter().map(|(_, v)| v).collect()))
}
