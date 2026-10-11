//! v2 immutable tree updates; v1 never enters this module.
use super::{Builder, Result, Value, ensure, get, json, key, reference, text, wire};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Key {
    Ref(wire::Key),
    Ordinal(u64),
    Text(String),
}
fn entry_key(entry: &Value, field: &str) -> Result<Key> {
    let value = if field == "ref" { entry } else { &entry[field] };
    Ok(match field {
        "ref" | "object" | "root" | "record" => Key::Ref(key(value)?),
        "acceptance_ordinal" | "sequence" => Key::Ordinal(wire::number(value)?),
        _ => Key::Text(text(value)?.to_owned()),
    })
}
fn bound_key(value: &Value, field: &str) -> Result<Key> {
    if field == "ref" {
        entry_key(value, field)
    } else {
        {
            let mut entry = json!({});
            entry[field] = value.clone();
            entry_key(&entry, field)
        }
    }
}
fn bound(entry: &Value, field: &str) -> Value {
    if field == "ref" {
        entry.clone()
    } else {
        entry[field].clone()
    }
}
#[derive(Clone)]
struct Node {
    original: Option<Value>,
    entries: Vec<Value>,
    children: Vec<Arc<Node>>,
    first: Value,
    last: Value,
    count: u64,
}
impl Node {
    fn leaf(entries: Vec<Value>, field: &str) -> Arc<Self> {
        Arc::new(Self {
            original: None,
            first: entries.first().map_or(Value::Null, |e| bound(e, field)),
            last: entries.last().map_or(Value::Null, |e| bound(e, field)),
            count: entries.len() as u64,
            entries,
            children: Vec::new(),
        })
    }
    fn branch(children: Vec<Arc<Self>>) -> Result<Arc<Self>> {
        ensure(children.len() >= 2, "incremental branch arity")?;
        Ok(Arc::new(Self {
            original: None,
            entries: Vec::new(),
            first: children[0].first.clone(),
            last: children.last().ok_or("branch")?.last.clone(),
            count: children.iter().try_fold(0u64, |n, c| {
                n.checked_add(c.count).ok_or("tree count overflow")
            })?,
            children,
        }))
    }
}
#[derive(Clone, Copy)]
pub(in super::super) struct Spec<'a> {
    pub name: &'a str,
    pub field: &'a str,
    pub capacity: usize,
    pub allocation: Option<&'a str>,
}
fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or("tree object")?;
    ensure(
        object.len() == expected.len() && expected.iter().all(|k| object.contains_key(*k)),
        "exact incremental node fields",
    )
}
fn read(builder: &Builder, r: &Value, spec: &Spec<'_>) -> Result<Value> {
    if spec.allocation.is_none() {
        return get(&builder.pool, r);
    }
    ensure(
        r["allocation_id"] == spec.allocation.unwrap_or_default(),
        "seed destination allocation",
    )?;
    builder.physical_value(r)
}
fn load(
    builder: &Builder,
    r: &Value,
    spec: &Spec<'_>,
    depth: usize,
    seen: &mut BTreeSet<String>,
    work: &mut usize,
) -> Result<Arc<Node>> {
    *work = work.checked_add(1).ok_or("tree work")?;
    ensure(
        *work <= builder.limits.max_objects && depth <= builder.limits.semantic.json.max_depth,
        "incremental tree budget",
    )?;
    ensure(
        seen.insert(wire::hash(&wire::encode(r))),
        "incremental repeated node",
    )?;
    let body = read(builder, r, spec)?;
    let leaf = body["schema"] == json!({"id":format!("{}-leaf",spec.name),"version":1});
    ensure(
        leaf || body["schema"] == json!({"id":format!("{}-branch",spec.name),"version":1}),
        "incremental schema",
    )?;
    fields(
        &body,
        &[
            "schema",
            "project_id",
            "count",
            if leaf { "entries" } else { "children" },
            "extensions",
        ],
    )?;
    ensure(
        body["project_id"] == builder.project && body["extensions"].as_object().is_some(),
        "incremental identity",
    )?;
    if spec.name == "photara.storage.retained-control-inventory" {
        ensure(body["extensions"] == json!({}), "retained extensions")?;
    }
    let node = if leaf {
        let entries = body["entries"].as_array().ok_or("tree entries")?;
        ensure(
            entries.len() <= spec.capacity && (depth == 0 || !entries.is_empty()),
            "seed leaf capacity",
        )?;
        let keys = entries
            .iter()
            .map(|e| entry_key(e, spec.field))
            .collect::<Result<Vec<_>>>()?;
        ensure(keys.windows(2).all(|w| w[0] < w[1]), "seed entry order")?;
        Node::leaf(entries.clone(), spec.field)
    } else {
        let entries = body["children"].as_array().ok_or("tree children")?;
        ensure(
            entries.len() >= 2 && entries.len() <= builder.geometry.branch as usize,
            "seed branch capacity",
        )?;
        let mut children = Vec::new();
        for e in entries {
            fields(e, &["first", "last", "count", "child"])?;
            let child = load(builder, &e["child"], spec, depth + 1, seen, work)?;
            ensure(
                e["first"] == child.first
                    && e["last"] == child.last
                    && wire::number(&e["count"])? == child.count,
                "seed child summary",
            )?;
            if let Some(previous) = children.last() {
                let previous: &Arc<Node> = previous;
                ensure(
                    bound_key(&previous.last, spec.field)? < bound_key(&child.first, spec.field)?,
                    "seed child order",
                )?;
            }
            children.push(child);
        }
        Node::branch(children)?
    };
    ensure(
        wire::number(&body["count"])? == node.count
            && node.count <= builder.limits.max_entries as u64,
        "incremental count budget",
    )?;
    let mut owned = (*node).clone();
    owned.original = Some(r.clone());
    Ok(Arc::new(owned))
}
fn entries(node: &Node, out: &mut BTreeMap<Key, Value>, field: &str) -> Result<()> {
    if node.children.is_empty() {
        for e in &node.entries {
            ensure(
                out.insert(entry_key(e, field)?, e.clone()).is_none(),
                "duplicate tree key",
            )?;
        }
    } else {
        for child in &node.children {
            entries(child, out, field)?;
        }
    }
    Ok(())
}
fn update(
    node: &Arc<Node>,
    k: &Key,
    value: Option<&Value>,
    spec: &Spec<'_>,
    branch: usize,
) -> Result<Vec<Arc<Node>>> {
    if node.children.is_empty() {
        let mut values = node.entries.clone();
        let keys = values
            .iter()
            .map(|v| entry_key(v, spec.field))
            .collect::<Result<Vec<_>>>()?;
        match keys.binary_search(k) {
            Ok(i) => {
                if let Some(v) = value {
                    values[i] = v.clone();
                } else {
                    values.remove(i);
                }
            }
            Err(i) => {
                if let Some(v) = value {
                    values.insert(i, v.clone());
                }
            }
        }
        if values == node.entries {
            return Ok(vec![node.clone()]);
        }
        if values.is_empty() {
            return Ok(Vec::new());
        }
        if values.len() > spec.capacity {
            let right = values.split_off(values.len() / 2);
            return Ok(vec![
                Node::leaf(values, spec.field),
                Node::leaf(right, spec.field),
            ]);
        }
        return Ok(vec![Node::leaf(values, spec.field)]);
    }
    let mut position = node.children.len() - 1;
    for (i, child) in node.children.iter().enumerate() {
        if bound_key(&child.last, spec.field)? >= *k {
            position = i;
            break;
        }
    }
    let changed = update(&node.children[position], k, value, spec, branch)?;
    if changed.len() == 1 && Arc::ptr_eq(&changed[0], &node.children[position]) {
        return Ok(vec![node.clone()]);
    }
    let mut children = node.children.clone();
    children.splice(position..=position, changed);
    if children.is_empty() {
        return Ok(Vec::new());
    }
    if children.len() == 1 {
        return Ok(children);
    }
    if children.len() > branch {
        let right = children.split_off(children.len() / 2);
        return Ok(vec![collapse(children)?, collapse(right)?]);
    }
    Ok(vec![Node::branch(children)?])
}
fn collapse(mut children: Vec<Arc<Node>>) -> Result<Arc<Node>> {
    if children.len() == 1 {
        Ok(children.remove(0))
    } else {
        Node::branch(children)
    }
}
fn emit(
    builder: &mut Builder,
    node: &Node,
    spec: &Spec<'_>,
    all: &mut Vec<Value>,
) -> Result<Value> {
    let mut children = Vec::new();
    for child in &node.children {
        let r = emit(builder, child, spec, all)?;
        children.push(json!({"first":child.first,"last":child.last,"count":child.count.to_string(),"child":r}));
    }
    if let Some(r) = &node.original {
        all.push(read(builder, r, spec)?);
        return Ok(r.clone());
    }
    let value = builder.object(
        &format!(
            "{}-{}",
            spec.name,
            if node.children.is_empty() {
                "leaf"
            } else {
                "branch"
            }
        ),
        if node.children.is_empty() {
            json!({"count":node.count.to_string(),"entries":node.entries})
        } else {
            json!({"count":node.count.to_string(),"children":children})
        },
    );
    let r = if let Some(allocation) = spec.allocation {
        builder.pack(allocation, "metadata", vec![(value.clone(), 3)])?;
        builder.locations[allocation][&key(&reference(&value))?].clone()
    } else {
        let r = builder.add(value.clone())?;
        let rank = builder.changed_order.len();
        builder.changed_order.entry(key(&r)?).or_insert(rank);
        r
    };
    all.push(value);
    Ok(r)
}
pub(in super::super) fn tree(
    builder: &mut Builder,
    seed: Option<&Value>,
    target: Vec<Value>,
    spec: Spec<'_>,
) -> Result<(Value, Vec<Value>)> {
    ensure(
        spec.capacity > 0 && target.len() <= builder.limits.max_entries,
        "incremental input budget",
    )?;
    let mut root = if let Some(seed) = seed {
        load(builder, seed, &spec, 0, &mut BTreeSet::new(), &mut 0)?
    } else {
        Node::leaf(Vec::new(), spec.field)
    };
    let mut old = BTreeMap::new();
    entries(&root, &mut old, spec.field)?;
    let mut next = BTreeMap::new();
    for e in target {
        ensure(
            next.insert(entry_key(&e, spec.field)?, e).is_none(),
            "target duplicate tree key",
        )?;
    }
    let keys = old
        .keys()
        .chain(next.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for k in keys {
        if old.get(&k) != next.get(&k) {
            let mut roots = update(
                &root,
                &k,
                next.get(&k),
                &spec,
                builder.geometry.branch as usize,
            )?;
            root = match roots.len() {
                0 => Node::leaf(Vec::new(), spec.field),
                1 => roots.remove(0),
                _ => Node::branch(roots)?,
            };
        }
    }
    #[cfg(test)]
    if std::env::var_os("PHOTARA_COMPILER_SIZE_IMPORT").is_some() {
        fn counts(node: &Node) -> (usize, usize) {
            node.children.iter().fold(
                (1, usize::from(node.original.is_none())),
                |(total, changed), child| {
                    let (t, c) = counts(child);
                    (total + t, changed + c)
                },
            )
        }
        let (total, changed) = counts(&root);
        eprintln!(
            "v2 tree {} destination={:?} total={total} changed={changed} entries={}",
            spec.name, spec.allocation, root.count
        );
    }
    let mut nodes = Vec::new();
    let r = emit(builder, &root, &spec, &mut nodes)?;
    Ok((r, nodes))
}
pub(in super::super) fn audit(
    builder: &Builder,
    seed: &Value,
    spec: Spec<'_>,
) -> Result<Vec<Value>> {
    let root = load(builder, seed, &spec, 0, &mut BTreeSet::new(), &mut 0)?;
    let mut values = BTreeMap::new();
    entries(&root, &mut values, spec.field)?;
    Ok(values.into_values().collect())
}
