// Disposable tree algorithms copied unchanged from ps2_index_furnace at 853781a.
use super::{Entry, FAN, Kind, Node, Ref, Result, Store, bit, ensure, first_diff, is_digest, key};
pub(super) fn checked_entries(entries: &[Entry]) -> Result<()> {
    ensure(
        !entries.is_empty() && entries.len() <= FAN,
        "leaf occupancy",
    )?;
    for e in entries {
        ensure(
            e.key == key(&e.id) && e.ordinal > 0 && is_digest(&e.request),
            "entry shape",
        )?;
    }
    ensure(
        entries.windows(2).all(|w| w[0].key < w[1].key),
        "duplicate or unordered key",
    )
}
pub(super) fn b_shape(level: u8, keys: &[String], children: &[Ref]) -> Result<()> {
    ensure(
        level > 0
            && level <= 32
            && !children.is_empty()
            && children.len() <= FAN
            && keys.len() == children.len()
            && keys.iter().all(|k| is_digest(k))
            && keys.windows(2).all(|w| w[0] < w[1]),
        "malformed B-tree node",
    )
}
pub(super) fn lookup(st: &mut Store, r: &Ref, target: &str, kind: Kind) -> Result<Option<Entry>> {
    let k = key(target);
    let mut next = r.clone();
    let mut previous_bit = None;
    for _ in 0..258 {
        match st.get(&next)? {
            Node::Leaf { entries } => {
                checked_entries(&entries)?;
                let hit = entries.into_iter().find(|e| e.key == k);
                if let Some(e) = &hit {
                    ensure(e.id == target, "operation key collision")?;
                }
                return Ok(hit);
            }
            Node::Radix {
                bit: b,
                anchor,
                left,
                right,
            } if kind == Kind::Radix => {
                ensure(
                    b < 256 && is_digest(&anchor) && previous_bit.is_none_or(|p| b > p),
                    "malformed radix node",
                )?;
                if first_diff(&k, &anchor) < b {
                    return Ok(None);
                }
                previous_bit = Some(b);
                next = if bit(&k, b) == 0 { left } else { right };
            }
            Node::Btree {
                level,
                keys,
                children,
            } if kind == Kind::Btree => {
                b_shape(level, &keys, &children)?;
                if k < keys[0] {
                    return Ok(None);
                }
                let i = keys.partition_point(|v| v <= &k) - 1;
                next = children[i].clone();
            }
            _ => return Err("map node kind".to_owned()),
        }
    }
    Err("map depth budget".to_owned())
}
pub(super) fn radix_build(st: &mut Store, entries: &[Entry]) -> Result<Ref> {
    if entries.len() <= FAN {
        return st.put(&Node::Leaf {
            entries: entries.to_vec(),
        });
    }
    let b = first_diff(&entries[0].key, &entries[entries.len() - 1].key);
    ensure(b < 256, "operation key collision")?;
    let mid = entries.partition_point(|e| bit(&e.key, b) == 0);
    let left = radix_build(st, &entries[..mid])?;
    let right = radix_build(st, &entries[mid..])?;
    st.put(&Node::Radix {
        bit: b,
        anchor: entries[0].key.clone(),
        left,
        right,
    })
}
#[allow(
    clippy::many_single_char_names,
    reason = "Local bit and child notation in this disposable radix algorithm"
)]
pub(super) fn radix_insert(st: &mut Store, root: &Ref, entry: Entry) -> Result<Ref> {
    match st.get(root)? {
        Node::Leaf { mut entries } => {
            checked_entries(&entries)?;
            ensure(
                entries.iter().all(|e| e.key != entry.key),
                "duplicate key insert",
            )?;
            entries.push(entry);
            entries.sort_by(|a, b| a.key.cmp(&b.key));
            radix_build(st, &entries)
        }
        Node::Radix {
            bit: b,
            anchor,
            mut left,
            mut right,
        } => {
            ensure(b < 256 && is_digest(&anchor), "malformed radix node")?;
            let d = first_diff(&entry.key, &anchor);
            if d < b {
                let k = entry.key.clone();
                let leaf = st.put(&Node::Leaf {
                    entries: vec![entry],
                })?;
                let (l, r) = if bit(&k, d) == 0 {
                    (leaf, root.clone())
                } else {
                    (root.clone(), leaf)
                };
                st.put(&Node::Radix {
                    bit: d,
                    anchor,
                    left: l,
                    right: r,
                })
            } else {
                if bit(&entry.key, b) == 0 {
                    left = radix_insert(st, &left, entry)?;
                } else {
                    right = radix_insert(st, &right, entry)?;
                }
                st.put(&Node::Radix {
                    bit: b,
                    anchor,
                    left,
                    right,
                })
            }
        }
        _ => Err("radix insertion node".to_owned()),
    }
}
pub(super) fn btree_build(st: &mut Store, entries: &[Entry]) -> Result<Ref> {
    let mut nodes = Vec::new();
    for chunk in entries.chunks(FAN) {
        nodes.push((
            chunk[0].key.clone(),
            st.put(&Node::Leaf {
                entries: chunk.to_vec(),
            })?,
        ));
    }
    let mut level = 0;
    while nodes.len() > 1 {
        level += 1;
        let mut next = Vec::new();
        for chunk in nodes.chunks(FAN) {
            next.push((
                chunk[0].0.clone(),
                st.put(&Node::Btree {
                    level,
                    keys: chunk.iter().map(|x| x.0.clone()).collect(),
                    children: chunk.iter().map(|x| x.1.clone()).collect(),
                })?,
            ));
        }
        nodes = next;
    }
    Ok(nodes.remove(0).1)
}
pub(super) fn btree_insert_inner(
    st: &mut Store,
    r: &Ref,
    entry: Entry,
) -> Result<Vec<(String, Ref)>> {
    let mut result = Vec::new();
    match st.get(r)? {
        Node::Leaf { mut entries } => {
            checked_entries(&entries)?;
            ensure(
                entries.iter().all(|e| e.key != entry.key),
                "duplicate key insert",
            )?;
            entries.push(entry);
            entries.sort_by(|a, b| a.key.cmp(&b.key));
            let chunk = if entries.len() > FAN {
                entries.len().div_ceil(2)
            } else {
                entries.len()
            };
            for es in entries.chunks(chunk) {
                result.push((
                    es[0].key.clone(),
                    st.put(&Node::Leaf {
                        entries: es.to_vec(),
                    })?,
                ));
            }
        }
        Node::Btree {
            level,
            keys,
            children,
        } => {
            b_shape(level, &keys, &children)?;
            let i = keys.partition_point(|k| k <= &entry.key).saturating_sub(1);
            let inserted = btree_insert_inner(st, &children[i], entry)?;
            let mut pairs: Vec<_> = keys.into_iter().zip(children).collect();
            pairs.splice(i..=i, inserted);
            let chunk = if pairs.len() > FAN {
                pairs.len().div_ceil(2)
            } else {
                pairs.len()
            };
            for ps in pairs.chunks(chunk) {
                result.push((
                    ps[0].0.clone(),
                    st.put(&Node::Btree {
                        level,
                        keys: ps.iter().map(|x| x.0.clone()).collect(),
                        children: ps.iter().map(|x| x.1.clone()).collect(),
                    })?,
                ));
            }
        }
        _ => return Err("B-tree insertion node".to_owned()),
    }
    Ok(result)
}
pub(super) fn btree_insert(st: &mut Store, r: &Ref, entry: Entry) -> Result<Ref> {
    let level = match st.get(r)? {
        Node::Leaf { .. } => 0,
        Node::Btree { level, .. } => level,
        _ => return Err("B-tree root".to_owned()),
    };
    let pairs = btree_insert_inner(st, r, entry)?;
    if pairs.len() == 1 {
        return Ok(pairs[0].1.clone());
    }
    st.put(&Node::Btree {
        level: level + 1,
        keys: pairs.iter().map(|x| x.0.clone()).collect(),
        children: pairs.iter().map(|x| x.1.clone()).collect(),
    })
}
pub(super) fn capacity(level: u8) -> Result<u64> {
    16_u64
        .checked_pow(u32::from(level) + 1)
        .ok_or_else(|| "sequence capacity overflow".to_owned())
}
pub(super) fn seq_node(st: &mut Store, r: &Ref) -> Result<(u8, u64, Vec<Ref>)> {
    if let Node::Sequence {
        level,
        count,
        children,
    } = st.get(r)?
    {
        let cap = capacity(level)?;
        let per = if level == 0 { 1 } else { capacity(level - 1)? };
        ensure(
            count > 0
                && count <= cap
                && children.len() as u64 == count.div_ceil(per)
                && children.len() <= FAN,
            "malformed sequence node",
        )?;
        Ok((level, count, children))
    } else {
        Err("sequence node kind".to_owned())
    }
}
pub(super) fn seq_build(st: &mut Store, receipts: &[Ref]) -> Result<Ref> {
    let mut nodes: Vec<(u64, Ref)> = receipts.iter().map(|r| (1, r.clone())).collect();
    let mut level = 0;
    loop {
        let mut next = Vec::new();
        for chunk in nodes.chunks(FAN) {
            let count = chunk.iter().map(|p| p.0).sum();
            next.push((
                count,
                st.put(&Node::Sequence {
                    level,
                    count,
                    children: chunk.iter().map(|p| p.1.clone()).collect(),
                })?,
            ));
        }
        if next.len() == 1 {
            return Ok(next.remove(0).1);
        }
        nodes = next;
        level += 1;
    }
}
pub(super) fn seq_single(st: &mut Store, level: u8, receipt: Ref) -> Result<Ref> {
    let child = if level == 0 {
        receipt
    } else {
        seq_single(st, level - 1, receipt)?
    };
    st.put(&Node::Sequence {
        level,
        count: 1,
        children: vec![child],
    })
}
pub(super) fn seq_append(st: &mut Store, r: &Ref, receipt: Ref) -> Result<Ref> {
    let (level, count, mut children) = seq_node(st, r)?;
    if count == capacity(level)? {
        let right = seq_single(st, level, receipt)?;
        return st.put(&Node::Sequence {
            level: level + 1,
            count: count + 1,
            children: vec![r.clone(), right],
        });
    }
    if level == 0 {
        children.push(receipt);
    } else {
        let last = children.len() - 1;
        if count % capacity(level - 1)? == 0 {
            children.push(seq_single(st, level - 1, receipt)?);
        } else {
            children[last] = seq_append(st, &children[last], receipt)?;
        }
    }
    st.put(&Node::Sequence {
        level,
        count: count + 1,
        children,
    })
}
// Prefix comparison touches shared frontier nodes, not every original receipt.
pub(super) fn prefix(st: &mut Store, old: &Ref, new: &Ref) -> Result<()> {
    if old == new {
        return Ok(());
    }
    let (ol, on, oc) = seq_node(st, old)?;
    let (nl, nn, nc) = seq_node(st, new)?;
    ensure(on <= nn && ol <= nl, "prefix count/height")?;
    if ol < nl {
        return prefix(st, old, &nc[0]);
    }
    if ol == 0 {
        return ensure(oc == nc[..oc.len()], "receipt prefix mismatch");
    }
    for (i, o) in oc.iter().enumerate() {
        if i + 1 < oc.len() {
            ensure(o == &nc[i], "historical subtree changed")?;
        } else {
            prefix(st, o, &nc[i])?;
        }
    }
    Ok(())
}
#[derive(Clone)]
pub(super) struct Root {
    pub(super) kind: Kind,
    pub(super) count: u64,
    pub(super) map: Ref,
    pub(super) sequence: Ref,
    pub(super) manifest: Ref,
}
pub(super) fn root(st: &mut Store, r: &Ref) -> Result<Root> {
    if let Node::Root {
        kind,
        count,
        map,
        sequence,
        manifest,
    } = st.get(r)?
    {
        ensure(count > 0, "empty root unsupported in fixture")?;
        Ok(Root {
            kind,
            count,
            map,
            sequence,
            manifest,
        })
    } else {
        Err("root node kind".to_owned())
    }
}
pub(super) fn make_root(
    st: &mut Store,
    kind: Kind,
    count: u64,
    map: Ref,
    sequence: Ref,
    authored: Ref,
) -> Result<Ref> {
    let manifest = st.put(&Node::Manifest {
        authored,
        map: map.clone(),
        sequence: sequence.clone(),
    })?;
    st.put(&Node::Root {
        kind,
        count,
        map,
        sequence,
        manifest,
    })
}
pub(super) fn authored(st: &mut Store, r: &Root) -> Result<Ref> {
    if let Node::Manifest {
        authored,
        map,
        sequence,
    } = st.get(&r.manifest)?
    {
        ensure(map == r.map && sequence == r.sequence, "manifest mismatch")?;
        ensure(
            matches!(st.get(&authored)?, Node::Authored { .. }),
            "authored kind",
        )?;
        Ok(authored)
    } else {
        Err("manifest kind".to_owned())
    }
}
