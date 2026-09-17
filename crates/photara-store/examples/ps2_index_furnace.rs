//! Disposable PS2 architecture furnace; NOT a package codec or production writer.
//! `cargo run --release -p photara-store --example ps2_index_furnace -- 1000 10000 100000`
//! Fully materializes every receipt and both indexes. The pack's offset/length/SHA
//! references are fixture-only; immutable ranges permit direct reads without a
//! lifetime locator scan. `sync_all` measurements are not power-loss qualification.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

type Result<T> = std::result::Result<T, String>;
const FAN: usize = 16;
const MAX_NODE: u64 = 65_536;
const RESERVE: u64 = 2 * 1024 * 1024;

#[allow(
    clippy::format_collect,
    reason = "Preserve the benchmarked formatter; hash throughput optimization is outside this fixture"
)]
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn id(i: u64) -> String {
    format!("operation-{i:020}")
}
fn key(id: &str) -> String {
    hash(format!("fixture.operation-id:{id}").as_bytes())
}
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn request(id: &str) -> String {
    hash(format!("fixture.request:{id}").as_bytes())
}
fn bit(k: &str, n: u16) -> u8 {
    let b = k.as_bytes()[usize::from(n / 4)];
    let v = if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
    (v >> (3 - n % 4)) & 1
}
fn first_diff(a: &str, b: &str) -> u16 {
    (0..256).find(|&i| bit(a, i) != bit(b, i)).unwrap_or(256)
}
fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.to_owned()) }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
struct Ref {
    offset: u64,
    len: u64,
    sha: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
struct Entry {
    key: String,
    id: String,
    request: String,
    ordinal: u64,
    receipt: Ref,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum Kind {
    Radix,
    Btree,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "fixture_kind")]
enum Node {
    Receipt {
        id: String,
        request: String,
        ordinal: u64,
    },
    Leaf {
        entries: Vec<Entry>,
    },
    Radix {
        bit: u16,
        anchor: String,
        left: Ref,
        right: Ref,
    },
    Btree {
        level: u8,
        keys: Vec<String>,
        children: Vec<Ref>,
    },
    Sequence {
        level: u8,
        count: u64,
        children: Vec<Ref>,
    },
    Authored {
        value: String,
    },
    Manifest {
        authored: Ref,
        map: Ref,
        sequence: Ref,
    },
    Root {
        kind: Kind,
        count: u64,
        map: Ref,
        sequence: Ref,
        manifest: Ref,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
struct Head {
    active: Ref,
    recovery: Ref,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Intent {
    id: String,
    request: String,
    old: Head,
    candidate: Option<Head>,
}
#[derive(Clone, Default, Debug, Serialize)]
struct Stats {
    object_reads: u64,
    object_read_bytes: u64,
    object_writes: u64,
    object_encoded_bytes: u64,
    object_write_bytes: u64,
    envelope_write_bytes: u64,
    head_reads: u64,
    head_read_bytes: u64,
    sync_calls: u64,
    sync_micros: u128,
}
struct Store {
    dir: PathBuf,
    file: File,
    stats: Stats,
    reserve: Option<u64>,
    seen: HashSet<Ref>,
    partial_next: bool,
    partial_envelope: bool,
    changed: Option<Vec<Ref>>,
}
impl Store {
    fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("objects.pack"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            dir: dir.to_owned(),
            file,
            stats: Stats::default(),
            reserve: None,
            seen: HashSet::new(),
            partial_next: false,
            partial_envelope: false,
            changed: None,
        })
    }
    fn size(&self) -> u64 {
        self.file.metadata().unwrap().len()
    }
    fn allocated(&self) -> u64 {
        fs::read_dir(&self.dir)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().blocks() * 512)
            .sum()
    }
    fn debit(&mut self, bytes: u64) -> Result<()> {
        if let Some(left) = &mut self.reserve {
            ensure(*left >= bytes, "reserve exhausted")?;
            *left -= bytes;
        }
        Ok(())
    }
    fn put(&mut self, node: &Node) -> Result<Ref> {
        let bytes = serde_json::to_vec(node).map_err(|e| e.to_string())?;
        ensure(bytes.len() as u64 <= MAX_NODE, "oversized fixture node")?;
        self.debit(bytes.len() as u64)?;
        let offset = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|e| e.to_string())?;
        if self.partial_next {
            self.partial_next = false;
            self.file
                .write_all(&bytes[..bytes.len() / 2])
                .map_err(|e| e.to_string())?;
            return Err("injected partial object write".to_owned());
        }
        self.file.write_all(&bytes).map_err(|e| e.to_string())?;
        self.stats.object_writes += 1;
        self.stats.object_encoded_bytes += bytes.len() as u64;
        self.stats.object_write_bytes += bytes.len() as u64;
        let reference = Ref {
            offset,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        if let Some(changed) = &mut self.changed {
            changed.push(reference.clone());
        }
        Ok(reference)
    }
    fn get(&mut self, r: &Ref) -> Result<Node> {
        ensure(r.len > 0 && r.len <= MAX_NODE, "invalid object length")?;
        ensure(
            r.offset
                .checked_add(r.len)
                .is_some_and(|end| end <= self.size()),
            "missing object range",
        )?;
        self.file
            .seek(SeekFrom::Start(r.offset))
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0; usize::try_from(r.len).map_err(|e| e.to_string())?];
        self.file
            .read_exact(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.stats.object_reads += 1;
        self.stats.object_read_bytes += r.len;
        ensure(hash(&bytes) == r.sha, "object digest mismatch")?;
        self.seen.insert(r.clone());
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    fn sync(&mut self) -> Result<()> {
        let now = Instant::now();
        self.file.sync_all().map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        self.stats.sync_micros += now.elapsed().as_micros();
        Ok(())
    }
    fn sync_dir(&mut self) -> Result<()> {
        let now = Instant::now();
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        self.stats.sync_micros += now.elapsed().as_micros();
        Ok(())
    }
    fn envelope<T: Serialize>(&mut self, name: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        self.debit(bytes.len() as u64)?;
        // Replace metadata atomically; a failed candidate-intent rewrite leaves
        // the original persisted intent intact. Caller supplies directory sync.
        let temporary = self.dir.join(format!("{name}.temporary"));
        let mut f = File::create(&temporary).map_err(|e| e.to_string())?;
        if self.partial_envelope {
            self.partial_envelope = false;
            f.write_all(&bytes[..bytes.len() / 2])
                .map_err(|e| e.to_string())?;
            return Err("injected partial candidate-intent envelope".to_owned());
        }
        f.write_all(&bytes).map_err(|e| e.to_string())?;
        self.stats.envelope_write_bytes += bytes.len() as u64;
        let now = Instant::now();
        f.sync_all().map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        self.stats.sync_micros += now.elapsed().as_micros();
        fs::rename(temporary, self.dir.join(name)).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn publish(&mut self, head: &Head) -> Result<()> {
        self.sync()?;
        self.envelope("HEAD.next", head)?;
        fs::rename(self.dir.join("HEAD.next"), self.dir.join("HEAD")).map_err(|e| e.to_string())?;
        self.sync_dir()
    }
    fn head(&mut self) -> Result<Head> {
        let bytes = fs::read(self.dir.join("HEAD")).map_err(|e| e.to_string())?;
        self.stats.head_reads += 1;
        self.stats.head_read_bytes += bytes.len() as u64;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}

fn checked_entries(entries: &[Entry]) -> Result<()> {
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
fn b_shape(level: u8, keys: &[String], children: &[Ref]) -> Result<()> {
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
fn lookup(st: &mut Store, r: &Ref, target: &str, kind: Kind) -> Result<Option<Entry>> {
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
fn radix_build(st: &mut Store, entries: &[Entry]) -> Result<Ref> {
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
fn radix_insert(st: &mut Store, root: &Ref, entry: Entry) -> Result<Ref> {
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
fn btree_build(st: &mut Store, entries: &[Entry]) -> Result<Ref> {
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
fn btree_insert_inner(st: &mut Store, r: &Ref, entry: Entry) -> Result<Vec<(String, Ref)>> {
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
fn btree_insert(st: &mut Store, r: &Ref, entry: Entry) -> Result<Ref> {
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
fn capacity(level: u8) -> Result<u64> {
    16_u64
        .checked_pow(u32::from(level) + 1)
        .ok_or_else(|| "sequence capacity overflow".to_owned())
}
fn seq_node(st: &mut Store, r: &Ref) -> Result<(u8, u64, Vec<Ref>)> {
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
fn seq_build(st: &mut Store, receipts: &[Ref]) -> Result<Ref> {
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
fn seq_single(st: &mut Store, level: u8, receipt: Ref) -> Result<Ref> {
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
fn seq_append(st: &mut Store, r: &Ref, receipt: Ref) -> Result<Ref> {
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
fn prefix(st: &mut Store, old: &Ref, new: &Ref) -> Result<()> {
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
struct Root {
    kind: Kind,
    count: u64,
    map: Ref,
    sequence: Ref,
    manifest: Ref,
}
fn root(st: &mut Store, r: &Ref) -> Result<Root> {
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
fn make_root(
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
fn authored(st: &mut Store, r: &Root) -> Result<Ref> {
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
fn structural(st: &mut Store) -> Result<Head> {
    let h = st.head()?;
    for rr in [&h.active, &h.recovery] {
        let r = root(st, rr)?;
        authored(st, &r)?;
        let (_, n, _) = seq_node(st, &r.sequence)?;
        ensure(n == r.count, "root sequence count")?;
        match st.get(&r.map)? {
            Node::Leaf { entries } => checked_entries(&entries)?,
            Node::Radix { bit, anchor, .. } if r.kind == Kind::Radix => {
                ensure(bit < 256 && is_digest(&anchor), "radix header")?;
            }
            Node::Btree {
                level,
                keys,
                children,
            } if r.kind == Kind::Btree => b_shape(level, &keys, &children)?,
            _ => return Err("map header".to_owned()),
        }
    }
    let a = root(st, &h.active)?;
    let b = root(st, &h.recovery)?;
    prefix(st, &b.sequence, &a.sequence)?;
    Ok(h)
}
fn audit_map(st: &mut Store, r: &Ref, kind: Kind, depth: usize) -> Result<Vec<Entry>> {
    ensure(depth <= 257, "audit depth budget")?;
    match st.get(r)? {
        Node::Leaf { entries } => {
            checked_entries(&entries)?;
            Ok(entries)
        }
        Node::Radix {
            bit: b,
            anchor,
            left,
            right,
        } if kind == Kind::Radix => {
            ensure(b < 256 && is_digest(&anchor), "radix shape")?;
            for child in [&left, &right] {
                if let Node::Radix { bit: child_bit, .. } = st.get(child)? {
                    ensure(child_bit > b, "radix branch bits must increase")?;
                }
            }
            let l = audit_map(st, &left, kind, depth + 1)?;
            let rr = audit_map(st, &right, kind, depth + 1)?;
            ensure(
                l.iter()
                    .all(|e| first_diff(&e.key, &anchor) >= b && bit(&e.key, b) == 0)
                    && rr
                        .iter()
                        .all(|e| first_diff(&e.key, &anchor) >= b && bit(&e.key, b) == 1),
                "radix routing mismatch",
            )?;
            let mut out = l;
            out.extend(rr);
            Ok(out)
        }
        Node::Btree {
            level,
            keys,
            children,
        } if kind == Kind::Btree => {
            b_shape(level, &keys, &children)?;
            let mut out = Vec::new();
            for (i, ch) in children.iter().enumerate() {
                let child_level = match st.get(ch)? {
                    Node::Leaf { .. } => 0,
                    Node::Btree { level, .. } => level,
                    _ => return Err("B-tree child kind".to_owned()),
                };
                ensure(child_level + 1 == level, "B-tree child level")?;
                let entries = audit_map(st, ch, kind, depth + 1)?;
                ensure(
                    entries[0].key == keys[i]
                        && (i + 1 == keys.len() || entries.last().unwrap().key < keys[i + 1]),
                    "B-tree range",
                )?;
                out.extend(entries);
            }
            Ok(out)
        }
        _ => Err("audit map kind".to_owned()),
    }
}
fn audit_seq(st: &mut Store, r: &Ref, start: u64) -> Result<Vec<Entry>> {
    let (level, count, children) = seq_node(st, r)?;
    let mut out = Vec::new();
    for (i, child) in children.into_iter().enumerate() {
        let ordinal = start + out.len() as u64;
        if level == 0 {
            if let Node::Receipt {
                id,
                request,
                ordinal: o,
            } = st.get(&child)?
            {
                ensure(o == ordinal, "ordinal gap or duplicate")?;
                out.push(Entry {
                    key: key(&id),
                    id,
                    request,
                    ordinal: o,
                    receipt: child,
                });
            } else {
                return Err("receipt kind".to_owned());
            }
        } else {
            let (l, child_count, _) = seq_node(st, &child)?;
            let per = capacity(level - 1)?;
            ensure(
                l + 1 == level && child_count == (count - i as u64 * per).min(per),
                "sequence child height/count",
            )?;
            out.extend(audit_seq(st, &child, ordinal)?);
        }
    }
    ensure(out.len() as u64 == count, "sequence child count")?;
    Ok(out)
}
fn audit_root(st: &mut Store, rr: &Ref) -> Result<()> {
    let r = root(st, rr)?;
    authored(st, &r)?;
    let entries = audit_map(st, &r.map, r.kind, 0)?;
    let receipts = audit_seq(st, &r.sequence, 1)?;
    ensure(
        entries.len() as u64 == r.count && receipts.len() as u64 == r.count,
        "index count disagreement",
    )?;
    let mut by_id = BTreeMap::new();
    for e in entries {
        ensure(
            by_id.insert(e.id.clone(), e).is_none(),
            "duplicate OperationId",
        )?;
    }
    for e in receipts {
        ensure(
            by_id.remove(&e.id).as_ref() == Some(&e),
            "map/sequence disagreement",
        )?;
    }
    ensure(by_id.is_empty(), "extra map entries")
}
fn full_audit(st: &mut Store) -> Result<()> {
    let h = structural(st)?;
    audit_root(st, &h.active)?;
    audit_root(st, &h.recovery)
}
fn build(st: &mut Store, n: u64, kind: Kind) -> Result<()> {
    ensure(n > 0, "fixture requires positive N")?;
    let count = usize::try_from(n).map_err(|e| e.to_string())?;
    let mut entries = Vec::with_capacity(count);
    let mut receipts = Vec::with_capacity(count);
    for i in 1..=n {
        let id = id(i);
        let req = request(&id);
        let receipt = st.put(&Node::Receipt {
            id: id.clone(),
            request: req.clone(),
            ordinal: i,
        })?;
        receipts.push(receipt.clone());
        entries.push(Entry {
            key: key(&id),
            id,
            request: req,
            ordinal: i,
            receipt,
        });
    }
    let sequence = seq_build(st, &receipts)?;
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    let map = match kind {
        Kind::Radix => radix_build(st, &entries)?,
        Kind::Btree => btree_build(st, &entries)?,
    };
    let authored = st.put(&Node::Authored {
        value: "fixed tiny current state".to_owned(),
    })?;
    let root = make_root(st, kind, n, map, sequence, authored)?;
    st.publish(&Head {
        active: root.clone(),
        recovery: root,
    })
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Cut {
    None,
    Partial,
    CandidateIntent,
    BeforeHead,
    AfterHead,
}
struct Session {
    trusted: bool,
    trusted_head: RefCell<Option<Head>>,
}
impl Session {
    fn imported() -> Self {
        Self {
            trusted: false,
            trusted_head: RefCell::new(None),
        }
    }
    fn audit(&mut self, st: &mut Store) -> Result<()> {
        full_audit(st)?;
        self.trusted = true;
        *self.trusted_head.borrow_mut() = Some(st.head()?);
        Ok(())
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep the complete fixture publication and fault-cut order visible together"
    )]
    fn accept(&self, st: &mut Store, id: &str, req: &str, cut: Cut, budget: u64) -> Result<Ref> {
        ensure(self.trusted, "imported root is read-only until audit")?;
        let head = st.head()?;
        ensure(
            self.trusted_head.borrow().as_ref() == Some(&head),
            "HEAD differs from audited/published session root",
        )?;
        let old = root(st, &head.active)?;
        let pending = st.dir.join("intent");
        let mut original_intent = None;
        if pending.exists() {
            let intent: Intent =
                serde_json::from_slice(&fs::read(&pending).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            ensure(
                intent.id == id && intent.request == req,
                "pending original ID requires reconciliation",
            )?;
            ensure(
                intent.old == head || intent.candidate.as_ref() == Some(&head),
                "unrelated HEAD while reconciling",
            )?;
            original_intent = Some(intent);
        }
        if let Some(e) = lookup(st, &old.map, id, old.kind)? {
            ensure(e.request == req, "conflicting request digest")?;
            match st.get(&e.receipt)? {
                Node::Receipt {
                    id: ri,
                    request: rq,
                    ordinal,
                } => ensure(
                    ri == id && rq == req && ordinal == e.ordinal,
                    "retry receipt mismatch",
                )?,
                _ => return Err("retry receipt kind".to_owned()),
            }
            if let Some(intent) = original_intent {
                ensure(
                    intent.candidate.as_ref() == Some(&head),
                    "accepted retry does not match intended HEAD",
                )?;
                st.sync()?;
                st.envelope("ack", &e.receipt)?;
                fs::remove_file(&pending).map_err(|e| e.to_string())?;
                st.sync_dir()?;
                st.reserve = None;
            }
            return Ok(e.receipt);
        }
        // This fixed finite bound is a fixture admission cap, not a production reserve formula.
        ensure(budget >= RESERVE, "reserve exhausted before admission")?;
        st.reserve = Some(budget);
        st.changed = Some(Vec::new());
        st.envelope(
            "intent",
            &Intent {
                id: id.to_owned(),
                request: req.to_owned(),
                old: head.clone(),
                candidate: None,
            },
        )?;
        st.sync_dir()?;
        if cut == Cut::Partial {
            st.partial_next = true;
        }
        let n = old
            .count
            .checked_add(1)
            .ok_or_else(|| "ordinal overflow".to_owned())?;
        let receipt = st.put(&Node::Receipt {
            id: id.to_owned(),
            request: req.to_owned(),
            ordinal: n,
        })?;
        let entry = Entry {
            key: key(id),
            id: id.to_owned(),
            request: req.to_owned(),
            ordinal: n,
            receipt: receipt.clone(),
        };
        let map = match old.kind {
            Kind::Radix => radix_insert(st, &old.map, entry)?,
            Kind::Btree => btree_insert(st, &old.map, entry)?,
        };
        let sequence = seq_append(st, &old.sequence, receipt.clone())?;
        prefix(st, &old.sequence, &sequence)?;
        let authored_ref = authored(st, &old)?;
        let active = make_root(st, old.kind, n, map, sequence, authored_ref)?;
        let candidate = Head {
            active,
            recovery: head.active.clone(),
        };
        if cut == Cut::CandidateIntent {
            st.partial_envelope = true;
        }
        st.envelope(
            "intent",
            &Intent {
                id: id.to_owned(),
                request: req.to_owned(),
                old: head.clone(),
                candidate: Some(candidate.clone()),
            },
        )?;
        st.sync_dir()?;
        st.sync()?;
        // Read back every newly written object, including split siblings that
        // would not be visited by the new operation's own search path.
        for r in st.changed.take().unwrap_or_default() {
            st.get(&r)?;
        }
        let new_root = root(st, &candidate.active)?;
        authored(st, &new_root)?;
        let new_entry = lookup(st, &new_root.map, id, new_root.kind)?
            .ok_or_else(|| "new entry missing on candidate readback".to_owned())?;
        ensure(
            new_entry.receipt == receipt && new_entry.request == req && new_entry.ordinal == n,
            "new index receipt mismatch",
        )?;
        if cut == Cut::BeforeHead {
            return Err("injected before HEAD".to_owned());
        }
        ensure(st.head()? == head, "HEAD changed")?;
        st.publish(&candidate)?;
        *self.trusted_head.borrow_mut() = Some(candidate);
        if cut == Cut::AfterHead {
            return Err("injected after HEAD".to_owned());
        }
        st.envelope("ack", &receipt)?;
        fs::remove_file(&pending).map_err(|e| e.to_string())?;
        st.sync_dir()?;
        st.reserve = None;
        Ok(receipt)
    }
    fn turnover(&self, st: &mut Store) -> Result<()> {
        ensure(self.trusted, "untrusted turnover")?;
        let head = st.head()?;
        ensure(
            self.trusted_head.borrow().as_ref() == Some(&head),
            "HEAD differs from audited/published session root",
        )?;
        let old = root(st, &head.active)?;
        let a = authored(st, &old)?;
        // A new authored/manifest/root object; both historical indexes are reused.
        let a = st.put(&Node::Authored {
            value: format!("turnover-after-offset-{}-{}", a.offset, st.size()),
        })?;
        let active = make_root(st, old.kind, old.count, old.map, old.sequence, a)?;
        let candidate = Head {
            active,
            recovery: head.active,
        };
        st.publish(&candidate)?;
        *self.trusted_head.borrow_mut() = Some(candidate);
        Ok(())
    }
}
fn quantiles(mut values: Vec<u128>) -> serde_json::Value {
    values.sort_unstable();
    let n = values.len();
    json!({"samples":n,"p50_us":values[n/2],"p95_us":values[(n*95/100).min(n-1)],"max_us":values[n-1]})
}
fn measure<F>(st: &mut Store, count: usize, mut f: F) -> Result<serde_json::Value>
where
    F: FnMut(&mut Store, usize) -> Result<()>,
{
    st.stats = Stats::default();
    let before_len = st.size();
    let before_alloc = st.allocated();
    let mut times = Vec::new();
    for i in 0..count {
        let now = Instant::now();
        f(st, i)?;
        times.push(now.elapsed().as_micros());
    }
    Ok(json!({"latency":quantiles(times),"counters":st.stats,
        "pack_file_length_delta":st.size()-before_len,
        "filesystem_allocated_delta":i128::from(st.allocated())-i128::from(before_alloc)}))
}
fn measure_open(path: &Path) -> Result<serde_json::Value> {
    // Fresh file handles/application state; the operating-system cache remains.
    let mut times = Vec::new();
    let mut reads = Vec::new();
    let mut bytes = Vec::new();
    let mut heads = Vec::new();
    for _ in 0..16 {
        let now = Instant::now();
        let mut st = Store::open(path)?;
        structural(&mut st)?;
        times.push(now.elapsed().as_micros());
        reads.push(st.stats.object_reads);
        bytes.push(st.stats.object_read_bytes);
        heads.push(st.stats.head_read_bytes);
    }
    Ok(
        json!({"latency":quantiles(times),"object_reads":reads,"object_read_bytes":bytes,
        "head_read_bytes":heads,"head_reads_per_open":1,"os_cache_evicted":false}),
    )
}
fn run(n: u64, kind: Kind) -> Result<serde_json::Value> {
    let temp = tempfile::Builder::new()
        .prefix("photara-ps2-index-furnace-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let mut st = Store::open(temp.path())?;
    let now = Instant::now();
    build(&mut st, n, kind)?;
    let build_ms = now.elapsed().as_millis();
    let baseline_bytes = st.size();
    let baseline_allocated = st.allocated();
    let mut session = Session::imported();
    let now = Instant::now();
    session.audit(&mut st)?;
    let initial_audit_ms = now.elapsed().as_millis();
    let append = measure(&mut st, 32, |st, i| {
        let id = id(n + 1 + i as u64);
        session.accept(st, &id, &request(&id), Cut::None, RESERVE)?;
        Ok(())
    })?;
    let old_retry = measure(&mut st, 64, |st, i| {
        let ord = match i % 4 {
            0 => 1,
            1 => n / 2 + 1,
            2 => n,
            _ => 1 + (i as u64 * 7919) % n,
        };
        let id = id(ord);
        session.accept(st, &id, &request(&id), Cut::None, RESERVE)?;
        Ok(())
    })?;
    let absent = measure(&mut st, 32, |st, i| {
        let h = st.head()?;
        let r = root(st, &h.active)?;
        ensure(
            lookup(st, &r.map, &id(n + 1000 + i as u64), kind)?.is_none(),
            "absent lookup",
        )
    })?;
    let checkpoint_open = measure_open(temp.path())?;
    let turnover = measure(&mut st, 16, |st, _| session.turnover(st))?;
    let shared_open = measure_open(temp.path())?;
    st.seen.clear();
    let now = Instant::now();
    full_audit(&mut st)?;
    let audit_ms = now.elapsed().as_millis();
    let live_object_bytes: u64 = st.seen.iter().map(|r| r.len).sum();
    Ok(
        json!({"kind":format!("{kind:?}"),"n":n,"fully_materialized":true,
        "build_ms":build_ms,"baseline_pack_bytes":baseline_bytes,"baseline_filesystem_allocated_bytes":baseline_allocated,
        "initial_full_audit_ms":initial_audit_ms,"append_checkpoint":append,"old_retry":old_retry,"absent_lookup":absent,
        "root_turnover":turnover,"checkpoint_fresh_handle_open":checkpoint_open,"fresh_handle_open":shared_open,
        "final_full_audit_ms":audit_ms,"final_pack_bytes":st.size(),"live_selected_object_bytes":live_object_bytes,
        "stale_pack_bytes_without_collection":st.size()-live_object_bytes}),
    )
}

fn expect_failure<T>(result: Result<T>, name: &str) -> Result<()> {
    match result {
        Ok(_value) => Err(format!("fault failed to refuse: {name}")),
        Err(_error) => Ok(()),
    }
}
#[allow(
    clippy::too_many_lines,
    reason = "Fault cases are kept in one explicit disposable acceptance matrix"
)]
fn fault_suite(kind: Kind) -> Result<Vec<String>> {
    let mut passed = Vec::new();
    for cut in [
        Cut::Partial,
        Cut::CandidateIntent,
        Cut::BeforeHead,
        Cut::AfterHead,
    ] {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = Store::open(temp.path())?;
        build(&mut st, 32, kind)?;
        let mut session = Session::imported();
        session.audit(&mut st)?;
        let op = id(33);
        let req = request(&op);
        expect_failure(
            session.accept(&mut st, &op, &req, cut, RESERVE),
            "interruption",
        )?;
        drop(st);
        let mut st = Store::open(temp.path())?;
        full_audit(&mut st)?;
        let head = st.head()?;
        let r = root(&mut st, &head.active)?;
        ensure(
            r.count == if cut == Cut::AfterHead { 33 } else { 32 },
            "interruption selection",
        )?;
        expect_failure(
            session.accept(&mut st, &id(34), &request(&id(34)), Cut::None, RESERVE),
            "fresh ID while unresolved",
        )?;
        session.accept(&mut st, &op, &req, Cut::None, RESERVE)?;
        full_audit(&mut st)?;
        ensure(
            !st.dir.join("intent").exists() && st.dir.join("ack").exists(),
            "reconciliation did not finalize acknowledgement",
        )?;
        passed.push(
            match cut {
                Cut::Partial => "partial_object_reopen_original_id",
                Cut::BeforeHead => "before_head_reopen_original_id",
                Cut::AfterHead => "after_head_reopen_original_receipt",
                Cut::CandidateIntent => "partial_candidate_intent_original_preserved",
                Cut::None => unreachable!(),
            }
            .to_owned(),
        );
    }
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut st = Store::open(temp.path())?;
    build(&mut st, 32, kind)?;
    let mut session = Session::imported();
    structural(&mut st)?;
    expect_failure(
        session.accept(&mut st, &id(33), &request(&id(33)), Cut::None, RESERVE),
        "unaudited imported write",
    )?;
    passed.push("imported_structural_open_read_only".to_owned());
    session.audit(&mut st)?;
    let old = st.head()?;
    let bytes = st.size();
    expect_failure(
        session.accept(&mut st, &id(33), &request(&id(33)), Cut::None, 0),
        "capacity",
    )?;
    ensure(
        st.head()? == old && st.size() == bytes,
        "capacity mutated package",
    )?;
    passed.push("reserve_exhaustion_before_admission".to_owned());
    expect_failure(
        session.accept(&mut st, &id(1), &"f".repeat(64), Cut::None, RESERVE),
        "digest conflict",
    )?;
    ensure(
        st.head()? == old && st.size() == bytes,
        "conflict mutated package",
    )?;
    passed.push("conflicting_request_digest".to_owned());
    let r = root(&mut st, &old.active)?;
    let original = lookup(&mut st, &r.map, &id(1), kind)?.unwrap();
    ensure(
        session.accept(&mut st, &id(1), &request(&id(1)), Cut::None, RESERVE)? == original.receipt,
        "retry original receipt",
    )?;
    ensure(st.size() == bytes, "retry appended bytes")?;
    passed.push("same_id_original_receipt_zero_write".to_owned());
    let missing = Ref {
        offset: st.size() + 1,
        len: 100,
        sha: "0".repeat(64),
    };
    expect_failure(
        lookup(&mut st, &missing, &id(1), kind),
        "missing page is not absence",
    )?;
    passed.push("missing_page".to_owned());
    let corrupt = st.put(&Node::Receipt {
        id: id(99),
        request: request(&id(99)),
        ordinal: 99,
    })?;
    st.file
        .seek(SeekFrom::Start(corrupt.offset))
        .map_err(|e| e.to_string())?;
    st.file.write_all(b"!").map_err(|e| e.to_string())?;
    expect_failure(st.get(&corrupt), "corrupt page")?;
    passed.push("corrupt_page".to_owned());
    let malformed = st.put(&Node::Sequence {
        level: 0,
        count: 2,
        children: vec![original.receipt.clone()],
    })?;
    expect_failure(seq_node(&mut st, &malformed), "malformed node")?;
    passed.push("authenticated_malformed_node".to_owned());
    let gap = st.put(&Node::Receipt {
        id: id(2),
        request: request(&id(2)),
        ordinal: 3,
    })?;
    let gap_sequence = seq_build(&mut st, &[original.receipt.clone(), gap])?;
    expect_failure(audit_seq(&mut st, &gap_sequence, 1), "ordinal gap")?;
    passed.push("ordinal_gap".to_owned());
    let duplicate = st.put(&Node::Leaf {
        entries: vec![original.clone(), original.clone()],
    })?;
    expect_failure(audit_map(&mut st, &duplicate, kind, 0), "duplicate ID")?;
    passed.push("duplicate_id".to_owned());
    let wrong = st.put(&Node::Receipt {
        id: id(1),
        request: "f".repeat(64),
        ordinal: 1,
    })?;
    let wrongseq = seq_build(&mut st, &[wrong])?;
    expect_failure(prefix(&mut st, &wrongseq, &r.sequence), "prefix mismatch")?;
    passed.push("prefix_mismatch".to_owned());
    let duplicate_receipt = st.put(&Node::Receipt {
        id: id(1),
        request: request(&id(1)),
        ordinal: 2,
    })?;
    let seq = seq_build(&mut st, &[original.receipt.clone(), duplicate_receipt])?;
    let leaf = st.put(&Node::Leaf {
        entries: vec![original.clone()],
    })?;
    let auth = authored(&mut st, &r)?;
    let badroot = make_root(&mut st, kind, 2, leaf, seq, auth)?;
    expect_failure(audit_root(&mut st, &badroot), "cross index duplicate")?;
    passed.push("cross_index_duplicate_and_count".to_owned());
    // Independent recovery does not require the active root or previous dispatch.
    audit_root(&mut st, &old.recovery)?;
    passed.push("independent_recovery_audit".to_owned());
    // Damage outside the top-level/open paths is not magically detected on open.
    st.file
        .seek(SeekFrom::Start(original.receipt.offset))
        .map_err(|e| e.to_string())?;
    st.file.write_all(b"!").map_err(|e| e.to_string())?;
    structural(&mut st)?;
    expect_failure(
        session.accept(&mut st, &id(1), &request(&id(1)), Cut::None, RESERVE),
        "untouched historical receipt on retry",
    )?;
    expect_failure(full_audit(&mut st), "untouched historical receipt on audit")?;
    passed.push("unread_history_corruption_open_then_retry_audit_refuse".to_owned());
    for n in [15, 16, 255, 256, 4095, 4096] {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = Store::open(temp.path())?;
        build(&mut st, n, kind)?;
        let mut session = Session::imported();
        session.audit(&mut st)?;
        let op = id(n + 1);
        session.accept(&mut st, &op, &request(&op), Cut::None, RESERVE)?;
        full_audit(&mut st)?;
    }
    passed.push("16_256_4096_ordinal_boundary_append_and_prefix_audit".to_owned());
    Ok(passed)
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let counts: Vec<u64> = if args.is_empty() {
        vec![1_000, 10_000, 100_000]
    } else {
        args.iter()
            .map(|s| {
                s.parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())
            })
            .collect::<Result<_>>()?
    };
    println!(
        "{}",
        json!({"fixture":"PS2 disposable immutable-pack index furnace","fanout":FAN,
        "max_node_bytes":MAX_NODE,"reservation_cap_bytes":RESERVE,
        "caveats":["fixture JSON, not permanent canonical wire","pack offsets are fixture-only locators",
            "allocation is st_blocks, not device-write telemetry","fresh-handle open retains OS cache",
            "sync_all is not APFS power-loss qualification","faults are injected function cuts plus reopen, not process-kill tests",
            "fixed tiny authored state; no explicit pins, external-media reads or collection",
            "turnover is timing-only; no turnover intent/fault protocol",
            "2 MiB reserve is a fixture admission cap, not a proved production worst-case formula",
            "ack is latest-only fixture metadata; original outcomes remain in immutable receipts",
            "metadata uses temporary-file sync then rename; injected partial candidate-intent rewrite is tested"]})
    );
    for kind in [Kind::Radix, Kind::Btree] {
        println!(
            "{}",
            json!({"kind":format!("{kind:?}"),"faults_passed":fault_suite(kind)?})
        );
    }
    for n in counts {
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run(n, kind)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_hash_prefixes_preserve_map_routing() -> Result<()> {
        // Genuine hashed IDs sharing eight routing bits exercise compressed
        // prefixes; this is a correctness workload, not a latency benchmark.
        let ids: Vec<_> = (1..)
            .map(id)
            .filter(|id| key(id).starts_with("00"))
            .take(48)
            .collect();
        for kind in [Kind::Radix, Kind::Btree] {
            let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
            let mut st = Store::open(temp.path())?;
            let mut entries = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                let receipt = st.put(&Node::Receipt {
                    id: id.clone(),
                    request: request(id),
                    ordinal: i as u64 + 1,
                })?;
                entries.push(Entry {
                    key: key(id),
                    id: id.clone(),
                    request: request(id),
                    ordinal: i as u64 + 1,
                    receipt,
                });
            }
            let mut initial = entries[..16].to_vec();
            initial.sort_by(|a, b| a.key.cmp(&b.key));
            let mut map = match kind {
                Kind::Radix => radix_build(&mut st, &initial)?,
                Kind::Btree => btree_build(&mut st, &initial)?,
            };
            for e in &entries[16..] {
                map = match kind {
                    Kind::Radix => radix_insert(&mut st, &map, e.clone())?,
                    Kind::Btree => btree_insert(&mut st, &map, e.clone())?,
                };
            }
            ensure(
                audit_map(&mut st, &map, kind, 0)?.len() == 48,
                "shared-prefix audit count",
            )?;
            for e in entries {
                ensure(
                    lookup(&mut st, &map, &e.id, kind)?.as_ref() == Some(&e),
                    "shared-prefix lookup",
                )?;
            }
        }
        Ok(())
    }

    #[test]
    fn mid_staging_reserve_exhaustion_preserves_selected_root() -> Result<()> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = Store::open(temp.path())?;
        build(&mut st, 32, Kind::Radix)?;
        let head = st.head()?;
        st.envelope(
            "intent",
            &Intent {
                id: id(33),
                request: request(&id(33)),
                old: head.clone(),
                candidate: None,
            },
        )?;
        st.sync_dir()?;
        st.reserve = Some(MAX_NODE);
        st.put(&Node::Receipt {
            id: id(33),
            request: request(&id(33)),
            ordinal: 33,
        })?;
        let partial_bytes = st.size();
        // Inject exhaustion after staging one object, before a complete map path.
        st.reserve = Some(0);
        expect_failure(
            st.put(&Node::Authored {
                value: "unpublished".to_owned(),
            }),
            "mid-staging reserve",
        )?;
        ensure(
            st.size() == partial_bytes && st.head()? == head && st.dir.join("intent").exists(),
            "exhaustion lost original evidence",
        )?;
        drop(st);
        let mut st = Store::open(temp.path())?;
        full_audit(&mut st)?;
        Ok(())
    }

    #[test]
    fn audited_session_refuses_substituted_head() -> Result<()> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = Store::open(temp.path())?;
        build(&mut st, 32, Kind::Radix)?;
        let mut session = Session::imported();
        session.audit(&mut st)?;
        let old = st.head()?;
        let r = root(&mut st, &old.active)?;
        let a = st.put(&Node::Authored {
            value: "independently substituted state".to_owned(),
        })?;
        let replacement = make_root(&mut st, r.kind, r.count, r.map, r.sequence, a)?;
        st.publish(&Head {
            active: replacement,
            recovery: old.recovery,
        })?;
        let selected = st.head()?;
        let bytes = st.size();
        expect_failure(
            session.accept(&mut st, &id(33), &request(&id(33)), Cut::None, RESERVE),
            "HEAD substitution",
        )?;
        ensure(
            st.head()? == selected && st.size() == bytes,
            "refusal changed substituted state",
        )?;
        Ok(())
    }

    #[test]
    fn recovery_audit_does_not_read_corrupt_active_root() -> Result<()> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = Store::open(temp.path())?;
        build(&mut st, 32, Kind::Radix)?;
        let mut session = Session::imported();
        session.audit(&mut st)?;
        session.accept(&mut st, &id(33), &request(&id(33)), Cut::None, RESERVE)?;
        let h = st.head()?;
        ensure(h.active != h.recovery, "fixture roots should differ")?;
        st.file
            .seek(SeekFrom::Start(h.active.offset))
            .map_err(|e| e.to_string())?;
        st.file.write_all(b"!").map_err(|e| e.to_string())?;
        st.seen.clear();
        audit_root(&mut st, &h.recovery)?;
        ensure(!st.seen.contains(&h.active), "recovery touched active root")?;
        expect_failure(
            structural(&mut st),
            "selected corrupt active root must not silently fall back",
        )?;
        Ok(())
    }
}
