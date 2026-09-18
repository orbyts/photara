//! Disposable packed-prefix and bounded physical-reclamation experiment.
//! All schemas here are fixture-only, not proposed permanent wire.
#[allow(
    clippy::wildcard_imports,
    reason = "Disposable companion shares synthetic logical indexes"
)]
use super::*;
use std::collections::{HashMap, VecDeque};

const PACK: u64 = 256 * 1024;
const TX_OBJECTS: usize = 4096;
const CACHE: usize = 128;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct PRef {
    pack: u64,
    offset: u64,
    len: u64,
    sha: String,
}
type PackRecord = (Option<u64>, PRef, Vec<u8>);
struct WritePlan {
    pack: u64,
    end: u64,
    bytes: u64,
    writes: Vec<PackRecord>,
}
impl WritePlan {
    fn from(arena: &Arena) -> Self {
        Self {
            pack: arena.pack,
            end: arena.end,
            bytes: 0,
            writes: Vec::new(),
        }
    }
    fn push(&mut self, bytes: Vec<u8>, key: Option<u64>) -> Result<PRef> {
        let frame = if key.is_some() { 12 } else { 4 };
        let len = bytes.len() as u64;
        ensure(len + frame <= PACK, "planned record bound")?;
        if self.end + len + frame > PACK {
            self.pack += 1;
            self.end = 0;
        }
        let r = PRef {
            pack: self.pack,
            offset: self.end + frame,
            len,
            sha: hash(&bytes),
        };
        self.end += len + frame;
        self.bytes += len + frame;
        self.writes.push((key, r.clone(), bytes));
        Ok(r)
    }
    fn staged(&mut self, r: &PRef, pages: &[Page], memo: &mut HashMap<PRef, PRef>) -> Result<PRef> {
        if r.pack != u64::MAX {
            return Ok(r.clone());
        }
        if let Some(done) = memo.get(r) {
            return Ok(done.clone());
        }
        let mut page = pages
            .get(usize::try_from(r.offset).map_err(error)?)
            .cloned()
            .ok_or("planned staged reference")?;
        if let Page::Branch { left, right, .. } = &mut page {
            *left = self.staged(left, pages, memo)?;
            *right = self.staged(right, pages, memo)?;
        }
        let done = self.push(serde_json::to_vec(&page).map_err(error)?, None)?;
        memo.insert(r.clone(), done.clone());
        Ok(done)
    }
    fn write(self, arena: &mut Arena, c: &mut Counters) -> Result<()> {
        for (key, expected, bytes) in self.writes {
            let got = arena.append(&bytes, key, c)?;
            ensure(got == expected, "physical allocation departed from preview")?;
            arena.read(&got)?;
            if key.is_some() {
                c.data_reads += 1;
                c.data_read_bytes += got.len;
            } else {
                c.meta_reads += 1;
                c.meta_read_bytes += got.len;
            }
        }
        ensure(
            arena.pack == self.pack && arena.end == self.end,
            "preview end mismatch",
        )?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Location {
    object: Ref,
    data: PRef,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Page {
    Leaf {
        key: u64,
        value: Location,
    },
    Branch {
        bit: u8,
        anchor: u64,
        left: PRef,
        right: PRef,
    },
}
impl Page {
    fn anchor(&self) -> u64 {
        match self {
            Self::Leaf { key, .. } => *key,
            Self::Branch { anchor, .. } => *anchor,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Selection {
    semantic: Head,
    active: PRef,
    recovery: PRef,
    data_pack: u64,
    data_end: u64,
    meta_pack: u64,
    meta_end: u64,
    epoch: u64,
    data_cursor: u64,
    meta_cursor: u64,
}
#[cfg(test)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtraPin {
    owner: String,
    class: String,
    selection: Selection,
}
#[derive(Clone, Debug, Default, Serialize)]
struct Counters {
    meta_reads: u64,
    meta_read_bytes: u64,
    meta_writes: u64,
    meta_write_bytes: u64,
    data_reads: u64,
    data_read_bytes: u64,
    data_write_bytes: u64,
    envelope_write_bytes: u64,
    cache_hits: u64,
    sync_calls: u64,
    sync_us: u128,
    files_created: u64,
    files_deleted: u64,
    allocation_charge: u64,
    candidate_read_bytes: u64,
    candidate_records: u64,
    staged_pages: u64,
    head_read_bytes: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cut {
    None,
    BeforeHead,
    AfterHead,
}
struct Arena {
    dir: PathBuf,
    prefix: &'static str,
    pack: u64,
    end: u64,
    file: File,
    dirty: HashSet<u64>,
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn round(n: u64) -> u64 {
    n.div_ceil(4096) * 4096
}
fn sync_file(file: &File, c: &mut Counters) -> Result<()> {
    let t = Instant::now();
    file.sync_all().map_err(error)?;
    c.sync_calls += 1;
    c.sync_us += t.elapsed().as_micros();
    Ok(())
}
fn sync_dir(dir: &Path, c: &mut Counters) -> Result<()> {
    sync_file(&File::open(dir).map_err(error)?, c)
}
impl Arena {
    fn rotate(&mut self, c: &mut Counters) -> Result<()> {
        self.pack += 1;
        self.end = 0;
        self.file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(self.path(self.pack))
            .map_err(error)?;
        self.dirty.insert(self.pack);
        c.files_created += 1;
        Ok(())
    }
    fn create(dir: &Path, prefix: &'static str, c: &mut Counters) -> Result<Self> {
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(dir.join(format!("{prefix}-0")))
            .map_err(error)?;
        c.files_created += 1;
        Ok(Self {
            dir: dir.to_owned(),
            prefix,
            pack: 0,
            end: 0,
            file,
            dirty: HashSet::from([0]),
        })
    }
    fn path(&self, p: u64) -> PathBuf {
        self.dir.join(format!("{}-{p}", self.prefix))
    }
    fn append(&mut self, bytes: &[u8], key: Option<u64>, c: &mut Counters) -> Result<PRef> {
        let framing = if key.is_some() { 12 } else { 4 };
        let required = bytes.len() as u64 + framing;
        ensure(required <= PACK, "record exceeds bounded pack")?;
        if self.end + required > PACK {
            self.pack += 1;
            self.end = 0;
            self.file = OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(self.path(self.pack))
                .map_err(error)?;
            self.dirty.insert(self.pack);
            c.files_created += 1;
        }
        let before = round(self.end);
        self.file.seek(SeekFrom::Start(self.end)).map_err(error)?;
        self.file
            .write_all(&u32::try_from(bytes.len()).map_err(error)?.to_le_bytes())
            .map_err(error)?;
        if let Some(key) = key {
            self.file.write_all(&key.to_le_bytes()).map_err(error)?;
        }
        self.file.write_all(bytes).map_err(error)?;
        let r = PRef {
            pack: self.pack,
            offset: self.end + framing,
            len: bytes.len() as u64,
            sha: hash(bytes),
        };
        self.end += required;
        self.dirty.insert(self.pack);
        c.allocation_charge += round(self.end) - before;
        if key.is_some() {
            c.data_write_bytes += required;
        } else {
            c.meta_write_bytes += required;
            c.meta_writes += 1;
        }
        Ok(r)
    }
    fn read(&self, r: &PRef) -> Result<Vec<u8>> {
        ensure(
            r.len > 0 && r.len <= PACK && r.offset.checked_add(r.len).is_some_and(|n| n <= PACK),
            "physical reference bound",
        )?;
        let mut f = File::open(self.path(r.pack)).map_err(error)?;
        let mut bytes = vec![0; usize::try_from(r.len).map_err(error)?];
        f.seek(SeekFrom::Start(r.offset))
            .and_then(|_| f.read_exact(&mut bytes))
            .map_err(error)?;
        ensure(hash(&bytes) == r.sha, "physical digest mismatch")?;
        Ok(bytes)
    }
    fn sync(&mut self, c: &mut Counters) -> Result<()> {
        for p in self.dirty.drain().collect::<Vec<_>>() {
            sync_file(&File::open(self.path(p)).map_err(error)?, c)?;
        }
        Ok(())
    }
    fn records(&self, pack: u64, data: bool) -> Result<Vec<PackRecord>> {
        ensure(pack < self.pack, "only sealed pack candidates")?;
        let bytes = fs::read(self.path(pack)).map_err(error)?;
        ensure(bytes.len() as u64 <= PACK, "oversized pack")?;
        let mut out = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            ensure(at + 4 <= bytes.len(), "partial pack framing")?;
            let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            at += 4;
            let key = if data {
                ensure(at + 8 <= bytes.len(), "partial key")?;
                let key = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
                at += 8;
                Some(key)
            } else {
                None
            };
            ensure(
                len > 0 && at.checked_add(len).is_some_and(|end| end <= bytes.len()),
                "partial record",
            )?;
            let body = bytes[at..at + len].to_vec();
            let r = PRef {
                pack,
                offset: at as u64,
                len: len as u64,
                sha: hash(&body),
            };
            out.push((key, r, body));
            at += len;
        }
        Ok(out)
    }
}
struct Fixture {
    dir: PathBuf,
    data: Arena,
    meta: Arena,
    head: Selection,
    cache: VecDeque<(PRef, Page)>,
    c: Counters,
    staged: Option<Vec<Page>>,
}
fn direction(key: u64, bit: u8) -> bool {
    key & (1_u64 << (63 - bit)) != 0
}
fn difference(a: u64, b: u64) -> u8 {
    u8::try_from((a ^ b).leading_zeros()).unwrap()
}
impl Fixture {
    fn put(&mut self, page: &Page) -> Result<PRef> {
        if let Some(pages) = &mut self.staged {
            ensure(pages.len() < TX_OBJECTS * 130, "staged metadata budget")?;
            self.c.staged_pages += 1;
            let id = pages.len();
            pages.push(page.clone());
            return Ok(PRef {
                pack: u64::MAX,
                offset: id as u64,
                len: 0,
                sha: "temporary".into(),
            });
        }
        let r = self
            .meta
            .append(&serde_json::to_vec(page).map_err(error)?, None, &mut self.c)?;
        self.meta.read(&r)?;
        self.c.meta_reads += 1;
        self.c.meta_read_bytes += r.len;
        Ok(r)
    }
    fn get(&mut self, r: &PRef) -> Result<Page> {
        if r.pack == u64::MAX {
            return self
                .staged
                .as_ref()
                .and_then(|v| {
                    usize::try_from(r.offset)
                        .ok()
                        .and_then(|offset| v.get(offset))
                })
                .cloned()
                .ok_or("unresolved temporary metadata reference".into());
        }
        ensure(
            r.pack < self.head.meta_pack
                || (r.pack == self.head.meta_pack
                    && r.offset
                        .checked_add(r.len)
                        .is_some_and(|end| end <= self.head.meta_end)),
            "metadata outside selected prefix",
        )?;
        if let Some(i) = self.cache.iter().position(|(key, _)| key == r) {
            let item = self.cache.remove(i).unwrap();
            let page = item.1.clone();
            self.cache.push_back(item);
            self.c.cache_hits += 1;
            return Ok(page);
        }
        let bytes = self.meta.read(r)?;
        self.c.meta_reads += 1;
        self.c.meta_read_bytes += bytes.len() as u64;
        let page: Page = serde_json::from_slice(&bytes).map_err(error)?;
        if let Page::Branch {
            bit, left, right, ..
        } = &page
        {
            ensure(*bit < 64 && left != right, "malformed branch")?;
        }
        if self.cache.len() == CACHE {
            self.cache.pop_front();
        }
        self.cache.push_back((r.clone(), page.clone()));
        Ok(page)
    }
    fn lookup(&mut self, root: &PRef, key: u64) -> Result<Option<Location>> {
        let mut at = root.clone();
        let mut previous = None;
        for _ in 0..65 {
            match self.get(&at)? {
                Page::Leaf { key: k, value } => {
                    ensure(k == value.object.offset, "locator logical identity")?;
                    return Ok((k == key).then_some(value));
                }
                Page::Branch {
                    bit,
                    anchor,
                    left,
                    right,
                } => {
                    ensure(previous.is_none_or(|p| p < bit), "locator branch order")?;
                    if difference(anchor, key) < bit {
                        return Ok(None);
                    }
                    previous = Some(bit);
                    at = if direction(key, bit) { right } else { left };
                }
            }
        }
        Err("locator depth bound".into())
    }
    fn update(
        &mut self,
        root: Option<PRef>,
        key: u64,
        value: Option<Location>,
    ) -> Result<Option<PRef>> {
        let Some(r) = root else {
            return value
                .map(|value| self.put(&Page::Leaf { key, value }))
                .transpose();
        };
        let page = self.get(&r)?;
        let d = difference(page.anchor(), key);
        let branch_bit = match &page {
            Page::Leaf { .. } => 64,
            Page::Branch { bit, .. } => *bit,
        };
        if d < branch_bit {
            let Some(value) = value else {
                return Ok(Some(r));
            };
            let leaf = self.put(&Page::Leaf { key, value })?;
            let (left, right) = if direction(key, d) {
                (r, leaf)
            } else {
                (leaf, r)
            };
            return self
                .put(&Page::Branch {
                    bit: d,
                    anchor: key,
                    left,
                    right,
                })
                .map(Some);
        }
        match page {
            Page::Leaf { key: k, value: old } => {
                ensure(k == key, "leaf routing")?;
                if value.as_ref() == Some(&old) {
                    return Ok(Some(r));
                }
                value
                    .map(|value| self.put(&Page::Leaf { key, value }))
                    .transpose()
            }
            Page::Branch {
                bit,
                anchor,
                left,
                right,
            } => {
                let (a, b) = if direction(key, bit) {
                    (
                        Some(left.clone()),
                        self.update(Some(right.clone()), key, value)?,
                    )
                } else {
                    (
                        self.update(Some(left.clone()), key, value)?,
                        Some(right.clone()),
                    )
                };
                match (a, b) {
                    (Some(a), Some(b)) if a == left && b == right => Ok(Some(r)),
                    (Some(left), Some(right)) => self
                        .put(&Page::Branch {
                            bit,
                            anchor,
                            left,
                            right,
                        })
                        .map(Some),
                    (one, None) | (None, one) => Ok(one),
                }
            }
        }
    }
    fn bulk(&mut self, items: &[(u64, Location)]) -> Result<PRef> {
        ensure(!items.is_empty(), "empty locator")?;
        if items.len() == 1 {
            return self.put(&Page::Leaf {
                key: items[0].0,
                value: items[0].1.clone(),
            });
        }
        let bit = difference(items[0].0, items.last().unwrap().0);
        ensure(bit < 64, "duplicate object identity")?;
        let split = items.partition_point(|(k, _)| !direction(*k, bit));
        ensure(split > 0 && split < items.len(), "bulk branch split")?;
        let left = self.bulk(&items[..split])?;
        let right = self.bulk(&items[split..])?;
        self.put(&Page::Branch {
            bit,
            anchor: items[0].0,
            left,
            right,
        })
    }
    fn multi(
        &mut self,
        root: Option<PRef>,
        changes: &[(u64, Option<Location>)],
    ) -> Result<Option<PRef>> {
        if changes.is_empty() {
            return Ok(root);
        }
        let Some(r) = root else {
            let items: Vec<_> = changes
                .iter()
                .filter_map(|(k, v)| v.clone().map(|v| (*k, v)))
                .collect();
            return if items.is_empty() {
                Ok(None)
            } else {
                self.bulk(&items).map(Some)
            };
        };
        let page = self.get(&r)?;
        match page {
            Page::Leaf { key, value } => {
                let mut items = BTreeMap::from([(key, value.clone())]);
                for (key, value) in changes {
                    if let Some(value) = value {
                        items.insert(*key, value.clone());
                    } else {
                        items.remove(key);
                    }
                }
                if items.is_empty() {
                    return Ok(None);
                }
                if items.len() == 1 && items.get(&key) == Some(&value) {
                    return Ok(Some(r));
                }
                self.bulk(&items.into_iter().collect::<Vec<_>>()).map(Some)
            }
            Page::Branch {
                bit,
                anchor,
                left,
                right,
            } => {
                let divergence = changes
                    .iter()
                    .filter(|(_, v)| v.is_some())
                    .map(|(key, _)| difference(*key, anchor))
                    .min()
                    .unwrap_or(64);
                let split = bit.min(divergence);
                let (old_left, old_right) = if split < bit {
                    if direction(anchor, split) {
                        (None, Some(r.clone()))
                    } else {
                        (Some(r.clone()), None)
                    }
                } else {
                    (Some(left.clone()), Some(right.clone()))
                };
                let l: Vec<_> = changes
                    .iter()
                    .filter(|(k, _)| !direction(*k, split))
                    .cloned()
                    .collect();
                let rr: Vec<_> = changes
                    .iter()
                    .filter(|(k, _)| direction(*k, split))
                    .cloned()
                    .collect();
                let a = self.multi(old_left, &l)?;
                let b = self.multi(old_right, &rr)?;
                match (a, b) {
                    (Some(a), Some(b)) if split == bit && a == left && b == right => Ok(Some(r)),
                    (Some(left), Some(right)) => self
                        .put(&Page::Branch {
                            bit: split,
                            anchor,
                            left,
                            right,
                        })
                        .map(Some),
                    (a, None) | (None, a) => Ok(a),
                }
            }
        }
    }
    fn object(&mut self, root: &PRef, r: &Ref) -> Result<Node> {
        let loc = self
            .lookup(root, r.offset)?
            .ok_or("missing selected object")?;
        ensure(loc.object == *r, "object identity conflict")?;
        ensure(
            loc.data.pack < self.head.data_pack
                || (loc.data.pack == self.head.data_pack
                    && loc
                        .data
                        .offset
                        .checked_add(loc.data.len)
                        .is_some_and(|end| end <= self.head.data_end)),
            "data outside selected prefix",
        )?;
        let bytes = self.data.read(&loc.data)?;
        self.c.data_reads += 1;
        self.c.data_read_bytes += bytes.len() as u64;
        ensure(hash(&bytes) == r.sha, "semantic data hash")?;
        serde_json::from_slice(&bytes).map_err(error)
    }
    fn copy_object(&mut self, source: &mut Store, r: &Ref) -> Result<Location> {
        let mut bytes = vec![0; usize::try_from(r.len).map_err(error)?];
        source
            .file
            .seek(SeekFrom::Start(r.offset))
            .and_then(|_| source.file.read_exact(&mut bytes))
            .map_err(error)?;
        ensure(hash(&bytes) == r.sha, "source capture hash")?;
        let data = self.data.append(&bytes, Some(r.offset), &mut self.c)?;
        self.data.read(&data)?;
        self.c.data_reads += 1;
        self.c.data_read_bytes += data.len;
        Ok(Location {
            object: r.clone(),
            data,
        })
    }
    fn snapshot(&self) -> Result<(u64, u64, u64)> {
        let mut bytes = 0;
        let mut allocated = 0;
        let mut files = 0;
        for e in fs::read_dir(&self.dir).map_err(error)? {
            let m = e.map_err(error)?.metadata().map_err(error)?;
            bytes += m.len();
            allocated += m.blocks() * 512;
            files += 1;
        }
        Ok((bytes, allocated, files))
    }
    fn envelope<T: Serialize>(&mut self, name: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(error)?;
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.dir.join(name))
            .map_err(error)?;
        f.write_all(&bytes).map_err(error)?;
        self.c.envelope_write_bytes += bytes.len() as u64;
        self.c.files_created += 1;
        self.c.allocation_charge += round(bytes.len() as u64) + 4096;
        sync_file(&f, &mut self.c)
    }
    fn publish(&mut self, active: PRef, recovery: PRef, semantic: Head, cut: Cut) -> Result<()> {
        ensure(
            !self.dir.join("intent").exists(),
            "unresolved fixture transaction",
        )?;
        let next = Selection {
            semantic,
            active,
            recovery,
            data_pack: self.data.pack,
            data_end: self.data.end,
            meta_pack: self.meta.pack,
            meta_end: self.meta.end,
            epoch: self.head.epoch + 1,
            data_cursor: self.head.data_cursor,
            meta_cursor: self.head.meta_cursor,
        };
        let old = self.head.clone();
        self.envelope("intent", &old)?;
        sync_dir(&self.dir, &mut self.c)?;
        self.data.sync(&mut self.c)?;
        self.meta.sync(&mut self.c)?;
        sync_dir(&self.dir, &mut self.c)?;
        self.envelope("candidate", &next)?;
        sync_dir(&self.dir, &mut self.c)?;
        if cut == Cut::BeforeHead {
            return Err("injected before HEAD".into());
        }
        if self.dir.join("HEAD").exists() {
            let bytes = fs::read(self.dir.join("HEAD")).map_err(error)?;
            let observed: Selection = serde_json::from_slice(&bytes).map_err(error)?;
            ensure(observed == self.head, "HEAD substituted")?;
        }
        self.envelope("HEAD.next", &next)?;
        fs::rename(self.dir.join("HEAD.next"), self.dir.join("HEAD")).map_err(error)?;
        self.head = next;
        if cut == Cut::AfterHead {
            return Err("injected after HEAD".into());
        }
        sync_dir(&self.dir, &mut self.c)?;
        fs::remove_file(self.dir.join("intent"))
            .and_then(|()| fs::remove_file(self.dir.join("candidate")))
            .map_err(error)?;
        self.c.files_deleted += 2;
        sync_dir(&self.dir, &mut self.c)?;
        Ok(())
    }
}

fn edges(node: Node) -> Vec<Ref> {
    match node {
        Node::Receipt { .. } | Node::Authored { .. } => vec![],
        Node::Leaf { entries } => entries.into_iter().map(|e| e.receipt).collect(),
        Node::Radix { left, right, .. } => vec![left, right],
        Node::Btree { children, .. } | Node::Sequence { children, .. } => children,
        Node::Manifest {
            authored,
            map,
            sequence,
        } => vec![authored, map, sequence],
        Node::Root {
            map,
            sequence,
            manifest,
            ..
        } => vec![map, sequence, manifest],
    }
}
fn reachable(source: &mut Store, root: &Ref) -> Result<BTreeMap<u64, Ref>> {
    let mut seen = BTreeMap::new();
    let mut todo = vec![root.clone()];
    while let Some(r) = todo.pop() {
        if let Some(old) = seen.insert(r.offset, r.clone()) {
            ensure(old == r, "source identity alias")?;
        } else {
            todo.extend(edges(source.get(&r)?));
        }
    }
    Ok(seen)
}
fn changes(source: &mut Store, old: &Ref, new: &Ref, old_end: u64) -> Result<(Vec<Ref>, Vec<Ref>)> {
    let mut added = BTreeMap::new();
    let mut boundary = HashSet::new();
    let mut todo = vec![new.clone()];
    while let Some(r) = todo.pop() {
        if r.offset < old_end {
            boundary.insert(r);
            continue;
        }
        if added.insert(r.offset, r.clone()).is_none() {
            ensure(added.len() <= TX_OBJECTS, "new object budget")?;
            todo.extend(edges(source.get(&r)?));
        }
    }
    let mut removed = BTreeMap::new();
    let mut todo = vec![old.clone()];
    while let Some(r) = todo.pop() {
        if boundary.contains(&r) {
            continue;
        }
        if removed.insert(r.offset, r.clone()).is_none() {
            ensure(removed.len() <= TX_OBJECTS, "removed object budget")?;
            todo.extend(edges(source.get(&r)?));
        }
    }
    Ok((
        added.into_values().collect(),
        removed.into_values().collect(),
    ))
}

impl Fixture {
    fn create(dir: &Path, source: &mut Store) -> Result<Self> {
        fs::create_dir(dir).map_err(error)?;
        let mut c = Counters::default();
        let data = Arena::create(dir, "data", &mut c)?;
        let meta = Arena::create(dir, "meta", &mut c)?;
        let semantic = source.head()?;
        let placeholder = PRef {
            pack: 0,
            offset: 0,
            len: 0,
            sha: String::new(),
        };
        let head = Selection {
            semantic: semantic.clone(),
            active: placeholder.clone(),
            recovery: placeholder,
            data_pack: 0,
            data_end: 0,
            meta_pack: 0,
            meta_end: 0,
            epoch: 0,
            data_cursor: 0,
            meta_cursor: 0,
        };
        let mut f = Self {
            dir: dir.to_owned(),
            data,
            meta,
            head,
            cache: VecDeque::new(),
            c,
            staged: None,
        };
        let objects = reachable(source, &semantic.active)?;
        let mut entries = Vec::new();
        for (key, r) in objects {
            entries.push((key, f.copy_object(source, &r)?));
        }
        let root = f.bulk(&entries)?;
        f.publish(root.clone(), root, semantic, Cut::None)?;
        Ok(f)
    }
    fn append(&mut self, source: &mut Store, old_end: u64) -> Result<serde_json::Value> {
        self.append_cut(source, old_end, Cut::None)
    }
    fn append_cut(
        &mut self,
        source: &mut Store,
        old_end: u64,
        cut: Cut,
    ) -> Result<serde_json::Value> {
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction before new append",
        )?;
        let old = self.head.clone();
        let semantic = source.head()?;
        ensure(
            semantic.recovery == old.semantic.active,
            "logical selected prefix discontinuity",
        )?;
        let (added, removed) = changes(source, &old.semantic.active, &semantic.active, old_end)?;
        let mut updates = BTreeMap::new();
        for r in &removed {
            ensure(
                self.lookup(&old.active, r.offset)?
                    .is_some_and(|l| l.object == *r),
                "removed object absent",
            )?;
            updates.insert(r.offset, None);
        }
        for r in &added {
            let loc = self.copy_object(source, r)?;
            updates.insert(r.offset, Some(loc));
        }
        let root = self
            .multi(
                Some(old.active.clone()),
                &updates.into_iter().collect::<Vec<_>>(),
            )?
            .ok_or("empty current locator")?;
        self.publish(root, old.active, semantic, cut)?;
        Ok(json!({"added":added.len(),"removed":removed.len()}))
    }
    fn audit_root(&mut self, root: &PRef, semantic: &Ref) -> Result<usize> {
        let mut todo = vec![semantic.clone()];
        let mut seen = HashSet::new();
        while let Some(r) = todo.pop() {
            if seen.insert(r.clone()) {
                todo.extend(edges(self.object(root, &r)?));
            }
        }
        let mut pages = vec![(root.clone(), Vec::<(u8, u64, bool)>::new())];
        let mut locations = HashSet::new();
        while let Some((at, path)) = pages.pop() {
            ensure(path.len() <= 64, "audit locator depth")?;
            match self.get(&at)? {
                Page::Leaf { key, value } => {
                    ensure(
                        key == value.object.offset
                            && seen.contains(&value.object)
                            && locations.insert(value.object),
                        "extraneous or duplicate locator leaf",
                    )?;
                    ensure(
                        path.iter().all(|(bit, anchor, right)| {
                            difference(key, *anchor) >= *bit && direction(key, *bit) == *right
                        }),
                        "audit locator routing",
                    )?;
                }
                Page::Branch {
                    bit,
                    anchor,
                    left,
                    right,
                } => {
                    ensure(
                        path.last().is_none_or(|(parent, _, _)| *parent < bit),
                        "audit branch order",
                    )?;
                    let mut lp = path.clone();
                    lp.push((bit, anchor, false));
                    let mut rp = path;
                    rp.push((bit, anchor, true));
                    pages.push((left, lp));
                    pages.push((right, rp));
                }
            }
        }
        ensure(locations.len() == seen.len(), "locator coverage mismatch")?;
        Ok(seen.len())
    }
    fn audit(&mut self) -> Result<usize> {
        self.cache.clear();
        let head = self.head.clone();
        let a = self.audit_root(&head.active, &head.semantic.active)?;
        let b = self.audit_root(&head.recovery, &head.semantic.recovery)?;
        Ok(a + b)
    }
    fn structural_probe(&mut self) -> Result<()> {
        let bytes = fs::read(self.dir.join("HEAD")).map_err(error)?;
        self.c.head_read_bytes += bytes.len() as u64;
        let h: Selection = serde_json::from_slice(&bytes).map_err(error)?;
        for (locator, semantic) in [
            (&h.active, &h.semantic.active),
            (&h.recovery, &h.semantic.recovery),
        ] {
            let Node::Root {
                kind,
                count,
                map,
                sequence,
                manifest,
            } = self.object(locator, semantic)?
            else {
                return Err("selected root kind".into());
            };
            let Node::Manifest {
                authored,
                map: manifest_map,
                sequence: manifest_sequence,
            } = self.object(locator, &manifest)?
            else {
                return Err("manifest kind".into());
            };
            ensure(
                map == manifest_map && sequence == manifest_sequence,
                "manifest identity",
            )?;
            ensure(
                matches!(self.object(locator, &authored)?, Node::Authored { .. }),
                "authored state kind",
            )?;
            ensure(
                matches!(self.object(locator,&sequence)?,Node::Sequence{count:n,..} if n==count),
                "selected sequence header count",
            )?;
            match self.object(locator, &map)? {
                Node::Leaf { entries } => checked_entries(&entries)?,
                Node::Radix { bit, anchor, .. } if kind == Kind::Radix => {
                    ensure(bit < 256 && is_digest(&anchor), "radix header")?;
                }
                Node::Btree {
                    level,
                    keys,
                    children,
                } if kind == Kind::Btree => b_shape(level, &keys, &children)?,
                _ => return Err("selected map header".into()),
            }
        }
        Ok(())
    }
    fn retry(&mut self, operation: &str, digest: &str) -> Result<Ref> {
        let h = self.head.clone();
        let Node::Root { kind, map, .. } = self.object(&h.active, &h.semantic.active)? else {
            return Err("root kind".into());
        };
        let k = key(operation);
        let mut next = map;
        let mut previous = None;
        for _ in 0..258 {
            match self.object(&h.active, &next)? {
                Node::Leaf { entries } => {
                    checked_entries(&entries)?;
                    let entry = entries
                        .into_iter()
                        .find(|e| e.key == k)
                        .ok_or("operation absent")?;
                    ensure(
                        entry.id == operation && entry.request == digest,
                        "operation digest conflict",
                    )?;
                    ensure(
                        matches!(self.object(&h.active,&entry.receipt)?,Node::Receipt{id,request,ordinal} if id==operation&&request==digest&&ordinal==entry.ordinal),
                        "original receipt mismatch",
                    )?;
                    return Ok(entry.receipt);
                }
                Node::Radix {
                    bit: b,
                    anchor,
                    left,
                    right,
                } if kind == Kind::Radix => {
                    ensure(
                        b < 256 && is_digest(&anchor) && previous.is_none_or(|p| p < b),
                        "radix retry header",
                    )?;
                    ensure(first_diff(&k, &anchor) >= b, "operation absent")?;
                    previous = Some(b);
                    next = if bit(&k, b) == 0 { left } else { right };
                }
                Node::Btree {
                    level,
                    keys,
                    children,
                } if kind == Kind::Btree => {
                    b_shape(level, &keys, &children)?;
                    ensure(k >= keys[0], "operation absent")?;
                    next = children[keys.partition_point(|v| v <= &k) - 1].clone();
                }
                _ => return Err("retry map kind".into()),
            }
        }
        Err("retry depth bound".into())
    }
    fn path_contains(&mut self, root: &PRef, target: &PRef, anchor: u64) -> Result<bool> {
        let mut at = root.clone();
        let mut prev = None;
        for _ in 0..65 {
            if &at == target {
                return Ok(true);
            }
            match self.get(&at)? {
                Page::Leaf { .. } => return Ok(false),
                Page::Branch {
                    bit,
                    anchor: a,
                    left,
                    right,
                } => {
                    ensure(prev.is_none_or(|p| p < bit), "metadata route cycle")?;
                    if difference(a, anchor) < bit {
                        return Ok(false);
                    }
                    prev = Some(bit);
                    at = if direction(anchor, bit) { right } else { left };
                }
            }
        }
        Err("metadata reachability budget".into())
    }
    fn rewrite_metadata(
        &mut self,
        root: &PRef,
        targets: &[(PRef, u64)],
        memo: &mut HashMap<PRef, PRef>,
    ) -> Result<PRef> {
        if targets.is_empty() {
            return Ok(root.clone());
        }
        if let Some(done) = memo.get(root) {
            return Ok(done.clone());
        }
        let page = self.get(root)?;
        let selected = targets.iter().any(|(r, _)| r == root);
        let updated = match page {
            Page::Leaf { .. } => {
                if selected {
                    self.put(&page)?
                } else {
                    root.clone()
                }
            }
            Page::Branch {
                bit,
                anchor,
                left,
                right,
            } => {
                let l: Vec<_> = targets
                    .iter()
                    .filter(|(_, k)| difference(*k, anchor) >= bit && !direction(*k, bit))
                    .cloned()
                    .collect();
                let r: Vec<_> = targets
                    .iter()
                    .filter(|(_, k)| difference(*k, anchor) >= bit && direction(*k, bit))
                    .cloned()
                    .collect();
                let a = self.rewrite_metadata(&left, &l, memo)?;
                let b = self.rewrite_metadata(&right, &r, memo)?;
                if selected || a != left || b != right {
                    self.put(&Page::Branch {
                        bit,
                        anchor,
                        left: a,
                        right: b,
                    })?
                } else {
                    root.clone()
                }
            }
        };
        memo.insert(root.clone(), updated.clone());
        Ok(updated)
    }
    fn compact_data(
        &mut self,
        pack: u64,
        cut: Cut,
        defer_nonpositive: bool,
    ) -> Result<serde_json::Value> {
        ensure(
            !self.dir.join("extra-pin").exists(),
            "extra pinned root blocks retirement",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction pins old bytes",
        )?;
        let before = fs::metadata(self.data.path(pack)).map_err(error)?.len();
        let records = self.data.records(pack, true)?;
        ensure(records.len() <= TX_OBJECTS, "candidate record budget")?;
        self.c.candidate_read_bytes += before;
        self.c.candidate_records += records.len() as u64;
        let head = self.head.clone();
        let mut active = Some(head.active.clone());
        let mut recovery = Some(head.recovery.clone());
        self.staged = Some(Vec::new());
        let mut copied = 0;
        let mut live = 0;
        let mut data_plan = WritePlan::from(&self.data);
        for (key, r, bytes) in &records {
            let key = key.unwrap();
            let a = self.lookup(&head.active, key)?.filter(|l| l.data == *r);
            let b = self.lookup(&head.recovery, key)?.filter(|l| l.data == *r);
            if a.is_none() && b.is_none() {
                continue;
            }
            live += 1;
            let physical = data_plan.push(bytes.clone(), Some(key))?;
            copied += bytes.len() as u64;
            if let Some(mut value) = a {
                value.data = physical.clone();
                active = self.update(active, key, Some(value))?;
            }
            if let Some(mut value) = b {
                value.data = physical;
                recovery = self.update(recovery, key, Some(value))?;
            }
        }
        let pages = self.staged.take().ok_or("missing preview staging")?;
        let mut meta_plan = WritePlan::from(&self.meta);
        let mut memo = HashMap::new();
        let roots = [
            meta_plan.staged(&active.unwrap(), &pages, &mut memo)?,
            meta_plan.staged(&recovery.unwrap(), &pages, &mut memo)?,
        ];
        let mut final_head = head.clone();
        final_head.active = roots[0].clone();
        final_head.recovery = roots[1].clone();
        final_head.epoch += 2;
        final_head.data_pack = data_plan.pack;
        final_head.data_end = data_plan.end;
        final_head.meta_pack = meta_plan.pack;
        final_head.meta_end = meta_plan.end;
        let mut middle_head = final_head.clone();
        middle_head.recovery = head.recovery.clone();
        middle_head.epoch -= 1;
        let old_len = serde_json::to_vec(&head).map_err(error)?.len() as u64;
        let middle_len = serde_json::to_vec(&middle_head).map_err(error)?.len() as u64;
        let final_len = serde_json::to_vec(&final_head).map_err(error)?.len() as u64;
        let net = i128::from(before)
            - i128::from(data_plan.bytes)
            - i128::from(meta_plan.bytes)
            - i128::from(final_len)
            + i128::from(old_len);
        let preview = json!({"standalone_net_file_bytes":net,"data_encoded_bytes":data_plan.bytes,"metadata_encoded_bytes":meta_plan.bytes,"envelope_process_bytes":old_len+3*middle_len+2*final_len,"extra_file_bytes_upper_estimate":data_plan.bytes+meta_plan.bytes+4*old_len.max(middle_len).max(final_len),"coupled_followup_not_planned":true,"filesystem_allocation_not_predicted":true});
        if defer_nonpositive && net <= 0 {
            return Ok(
                json!({"candidate_pack_bytes":before,"candidate_records":records.len(),"live_records":live,"deferred_nonpositive_standalone":true,"retired_bytes":0,"preview":preview}),
            );
        }
        data_plan.write(&mut self.data, &mut self.c)?;
        meta_plan.write(&mut self.meta, &mut self.c)?;
        // First transition keeps the independently selected recovery placement.
        self.publish(roots[0].clone(), head.recovery, head.semantic.clone(), cut)?;
        ensure(
            self.data.path(pack).exists(),
            "old pack vanished before recovery transition",
        )?;
        self.publish(roots[0].clone(), roots[1].clone(), head.semantic, Cut::None)?;
        for (key, r, _) in &records {
            for root in [&roots[0], &roots[1]] {
                ensure(
                    self.lookup(root, key.unwrap())?
                        .is_none_or(|l| l.data != *r),
                    "data retirement still pinned",
                )?;
            }
        }
        fs::remove_file(self.data.path(pack)).map_err(error)?;
        self.c.files_deleted += 1;
        sync_dir(&self.dir, &mut self.c)?;
        Ok(
            json!({"candidate_pack_bytes":before,"candidate_records":records.len(),"live_records":live,"copied_payload_bytes":copied,"retired_bytes":before,"two_selections":true,"deferred_nonpositive_standalone":false,"preview":preview}),
        )
    }
    fn compact_meta(
        &mut self,
        pack: u64,
        cut: Cut,
        defer_nonpositive: bool,
    ) -> Result<serde_json::Value> {
        ensure(
            !self.dir.join("extra-pin").exists(),
            "extra pinned root blocks metadata retirement",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction pins metadata",
        )?;
        let records = self.meta.records(pack, false)?;
        let before = fs::metadata(self.meta.path(pack)).map_err(error)?.len();
        ensure(records.len() <= TX_OBJECTS, "candidate record budget")?;
        self.c.candidate_read_bytes += before;
        self.c.candidate_records += records.len() as u64;
        let head = self.head.clone();
        let mut targets = Vec::new();
        for (_, r, bytes) in &records {
            let page: Page = serde_json::from_slice(bytes).map_err(error)?;
            if self.path_contains(&head.active, r, page.anchor())?
                || self.path_contains(&head.recovery, r, page.anchor())?
            {
                targets.push((r.clone(), page.anchor()));
            }
        }
        self.staged = Some(Vec::new());
        let mut memo = HashMap::new();
        let a = self.rewrite_metadata(&head.active, &targets, &mut memo)?;
        let b = self.rewrite_metadata(&head.recovery, &targets, &mut memo)?;
        let pages = self.staged.take().ok_or("missing metadata preview")?;
        let mut plan = WritePlan::from(&self.meta);
        let mut memo = HashMap::new();
        let roots = [
            plan.staged(&a, &pages, &mut memo)?,
            plan.staged(&b, &pages, &mut memo)?,
        ];
        let mut final_head = head.clone();
        final_head.active = roots[0].clone();
        final_head.recovery = roots[1].clone();
        final_head.epoch += 2;
        final_head.meta_pack = plan.pack;
        final_head.meta_end = plan.end;
        final_head.data_pack = self.data.pack;
        final_head.data_end = self.data.end;
        let mut middle = final_head.clone();
        middle.epoch -= 1;
        middle.recovery = head.recovery.clone();
        let old_len = serde_json::to_vec(&head).map_err(error)?.len() as u64;
        let middle_len = serde_json::to_vec(&middle).map_err(error)?.len() as u64;
        let final_len = serde_json::to_vec(&final_head).map_err(error)?.len() as u64;
        let net = i128::from(before) - i128::from(plan.bytes) - i128::from(final_len)
            + i128::from(old_len);
        let preview = json!({"standalone_net_file_bytes":net,"metadata_encoded_bytes":plan.bytes,"envelope_process_bytes":old_len+3*middle_len+2*final_len,"extra_file_bytes_upper_estimate":plan.bytes+4*old_len.max(middle_len).max(final_len),"coupled_followup_not_planned":true,"filesystem_allocation_not_predicted":true});
        if defer_nonpositive && net <= 0 {
            return Ok(
                json!({"candidate_pack_bytes":before,"candidate_records":records.len(),"live_records":targets.len(),"deferred_nonpositive_standalone":true,"retired_bytes":0,"preview":preview}),
            );
        }
        plan.write(&mut self.meta, &mut self.c)?;
        self.publish(roots[0].clone(), head.recovery, head.semantic.clone(), cut)?;
        ensure(
            self.meta.path(pack).exists(),
            "metadata retired before recovery transition",
        )?;
        self.publish(roots[0].clone(), roots[1].clone(), head.semantic, Cut::None)?;
        for (_, r, bytes) in &records {
            let page: Page = serde_json::from_slice(bytes).map_err(error)?;
            ensure(
                !self.path_contains(&roots[0], r, page.anchor())?
                    && !self.path_contains(&roots[1], r, page.anchor())?,
                "metadata retirement still pinned",
            )?;
        }
        fs::remove_file(self.meta.path(pack)).map_err(error)?;
        self.c.files_deleted += 1;
        sync_dir(&self.dir, &mut self.c)?;
        Ok(
            json!({"candidate_pack_bytes":before,"candidate_records":records.len(),"live_records":targets.len(),"retired_bytes":before,"two_selections":true,"deferred_nonpositive_standalone":false,"preview":preview}),
        )
    }
    #[cfg(test)]
    fn reconcile(&mut self) -> Result<&'static str> {
        let old: Selection =
            serde_json::from_slice(&fs::read(self.dir.join("intent")).map_err(error)?)
                .map_err(error)?;
        let candidate: Selection =
            serde_json::from_slice(&fs::read(self.dir.join("candidate")).map_err(error)?)
                .map_err(error)?;
        let selected: Selection =
            serde_json::from_slice(&fs::read(self.dir.join("HEAD")).map_err(error)?)
                .map_err(error)?;
        ensure(
            selected == old || selected == candidate,
            "unknown physical selector; preserve all evidence",
        )?;
        let outcome = if selected == old {
            "exact-old"
        } else {
            "exact-candidate"
        };
        self.head = selected;
        self.cache.clear();
        // Exhaustive fixture reconciliation is separate from bounded open.
        // Production must integrate changed-path evidence and owner identity.
        self.audit()?;
        self.data.sync(&mut self.c)?;
        self.meta.sync(&mut self.c)?;
        sync_file(
            &File::open(self.dir.join("HEAD")).map_err(error)?,
            &mut self.c,
        )?;
        sync_dir(&self.dir, &mut self.c)?;
        fs::remove_file(self.dir.join("intent"))
            .and_then(|()| fs::remove_file(self.dir.join("candidate")))
            .map_err(error)?;
        self.c.files_deleted += 2;
        sync_dir(&self.dir, &mut self.c)?;
        Ok(outcome)
    }
    #[cfg(test)]
    fn pin(&mut self, owner: &str, class: &str) -> Result<ExtraPin> {
        let pin = ExtraPin {
            owner: owner.into(),
            class: class.into(),
            selection: self.head.clone(),
        };
        self.envelope("extra-pin", &pin)?;
        sync_dir(&self.dir, &mut self.c)?;
        Ok(pin)
    }
    #[cfg(test)]
    fn release_pin(&mut self, expected: &ExtraPin) -> Result<()> {
        let observed: ExtraPin =
            serde_json::from_slice(&fs::read(self.dir.join("extra-pin")).map_err(error)?)
                .map_err(error)?;
        ensure(&observed == expected, "pin owner/selection mismatch")?;
        fs::remove_file(self.dir.join("extra-pin")).map_err(error)?;
        sync_dir(&self.dir, &mut self.c)
    }
}

fn run(n: u64, kind: Kind) -> Result<serde_json::Value> {
    ensure(n > 0, "positive operation count")?;
    let temp = tempfile::tempdir().map_err(error)?;
    let sd = temp.path().join("source");
    fs::create_dir(&sd).map_err(error)?;
    let mut source = Store::open(&sd)?;
    build(&mut source, n, kind)?;
    let mut session = Session::imported();
    session.audit(&mut source)?;
    let t = Instant::now();
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    let bootstrap_ms = t.elapsed().as_millis();
    let baseline = f.snapshot()?;
    let mut appends = Vec::new();
    for i in n + 1..=n + 32 {
        let old_end = source.size();
        session.accept(
            &mut source,
            &id(i),
            &request(&id(i)),
            super::Cut::None,
            RESERVE,
        )?;
        f.c = Counters::default();
        let before = f.snapshot()?;
        let t = Instant::now();
        let delta = f.append(&mut source, old_end)?;
        appends.push(json!({"us":t.elapsed().as_micros(),"logical_delta":delta,"source_append_bytes":source.size()-old_end,"counters":f.c,"before":before,"after":f.snapshot()?}));
    }
    let h = f.head.clone();
    f.cache.clear();
    f.c = Counters::default();
    let t = Instant::now();
    f.structural_probe()?;
    let cold = json!({"us":t.elapsed().as_micros(),"counters":f.c});
    f.c = Counters::default();
    let t = Instant::now();
    for _ in 0..32 {
        f.object(&h.active, &h.semantic.active)?;
    }
    let warm = json!({"us":t.elapsed().as_micros(),"counters":f.c});
    f.cache.clear();
    f.c = Counters::default();
    let t = Instant::now();
    let old = id(1);
    f.retry(&old, &request(&old))?;
    let cold_retry = json!({"us":t.elapsed().as_micros(),"counters":f.c});
    f.c = Counters::default();
    let t = Instant::now();
    for _ in 0..32 {
        f.retry(&old, &request(&old))?;
    }
    let warm_retry = json!({"us":t.elapsed().as_micros(),"counters":f.c});
    f.c = Counters::default();
    let t = Instant::now();
    f.publish(h.active.clone(), h.recovery.clone(), h.semantic, Cut::None)?;
    let turnover = json!({"us":t.elapsed().as_micros(),"counters":f.c});
    f.audit()?;
    // Explicit bounded recent-pack candidates, not an asserted complete GC
    // scheduling policy. Each pass scans at most one 256KiB physical unit.
    let data_pack = f
        .data
        .pack
        .checked_sub(1)
        .ok_or("fixture needs a sealed data pack")?;
    f.c = Counters::default();
    let before = f.snapshot()?;
    let t = Instant::now();
    let data_gc = f.compact_data(data_pack, Cut::None, true)?;
    let data_gc = json!({"result":data_gc,"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?});
    f.audit()?;
    let meta_pack = f
        .meta
        .pack
        .checked_sub(1)
        .ok_or("fixture needs a sealed metadata pack")?;
    f.c = Counters::default();
    let before = f.snapshot()?;
    let t = Instant::now();
    let meta_gc = f.compact_meta(meta_pack, Cut::None, true)?;
    let meta_gc = json!({"result":meta_gc,"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?});
    let t = Instant::now();
    let audited = f.audit()?;
    let audit_ms = t.elapsed().as_millis();
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"bootstrap_ms":bootstrap_ms,"baseline":baseline,"appends":appends,"cold_root_pair":cold,"warm32_root_reads":warm,"cold_oldest_retry":cold_retry,"warm32_oldest_retry":warm_retry,"root_turnover":turnover,"data_compaction":data_gc,"metadata_compaction":meta_gc,"final_audit_ms":audit_ms,"audited_objects_sum_both_roots":audited,"final_snapshot":f.snapshot()?}),
    )
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    if args.first().is_some_and(|arg| arg == "groups") {
        let n = args
            .get(1)
            .ok_or("groups requires operation count")?
            .parse::<u64>()
            .map_err(error)?;
        println!(
            "{}",
            json!({"fixture":"grouped packed prefix placement","n":n,"groups_per_policy":8,"policies":[8,32],"caveats":["one logical and physical selected HEAD per group; every original receipt retained","synthetic logical source builder and its I/O excluded from physical timings and bytes","no group acceptance journal or Saved claim; prior approved semantics unchanged","policies sequential per fixture; matched operation IDs across both maps","eight samples per policy, OS cache retained, no scheduler wait measured","recovery locator pins previous group boundary; full recovery audit measured separately"]})
        );
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run_groups(n, kind)?);
        }
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "density") {
        let n = args
            .get(1)
            .ok_or("density requires operation count")?
            .parse::<u64>()
            .map_err(error)?;
        println!(
            "{}",
            json!({"fixture":"controlled bounded pack density","n":n,"pack_limit":PACK,"caveats":["fully materialized operation baseline; construction outside maintenance timing","same original receipts remapped before each candidate measurement","synthetic unselected garbage, not a production churn distribution","all-class pin and physical reserve integration remain separate gates"]})
        );
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run_density(n, kind)?);
        }
        return Ok(());
    }
    println!(
        "{}",
        json!({"fixture":"packed prefix placement v2b","pack_limit":PACK,"metadata_cache_entries":CACHE,"changes_budget":TX_OBJECTS,"append_samples":32,"append_update":"recursive multi-key ancestor coalescing","caveats":["disposable wire and physical layout only; stable synthetic virtual IDs, not hash-only CAS","source semantic operation committed separately before physical publication","pack-prefix content is immutable; files append only until sealed","bounded candidate scans plus logarithmic locator paths; full audit separate","physical-only transitions preserve semantic active and recovery root IDs","extra reader/undo pin conservatively refuses all compaction; selective all-class liveness not integrated","no OS-exclusive reservation or qualified Saved/power-loss claim","standalone no-write preview defers nonpositive file-byte reclaim; no coupled cycle or filesystem allocation prediction","explicit recent sealed-pack candidates; not a complete GC scheduling policy","cold means empty fixture metadata cache, not cleared OS cache","structural probe checks roots/manifests/authored/sequence/map headers; not full history audit or a new prefix proof"]})
    );
    for arg in args {
        let n = arg.parse::<u64>().map_err(error)?;
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run(n, kind)?);
        }
    }
    Ok(())
}

fn density_case(
    f: &mut Fixture,
    source: &mut Store,
    n: u64,
    garbage: bool,
) -> Result<serde_json::Value> {
    f.c = Counters::default();
    let construction = Instant::now();
    f.data.rotate(&mut f.c)?;
    let candidate = f.data.pack;
    let old = f.head.clone();
    let mut changes = Vec::new();
    let mut originals = Vec::new();
    let mut live_bytes = 0;
    for i in 1..=n.min(2000) {
        if garbage && i > 16 {
            break;
        }
        let operation = id(i);
        let r = f.retry(&operation, &request(&operation))?;
        if f.data.end + r.len + 12 > PACK {
            break;
        }
        let value = f.copy_object(source, &r)?;
        live_bytes += r.len + 12;
        originals.push(r);
        changes.push((value.object.offset, Some(value)));
    }
    ensure(
        !changes.is_empty(),
        "density fixture needs original receipts",
    )?;
    if garbage {
        let bytes = vec![0xA5; 1024];
        let mut key = u64::MAX;
        while f.data.end + bytes.len() as u64 + 12 <= PACK {
            f.data.append(&bytes, Some(key), &mut f.c)?;
            key -= 1;
        }
    }
    let candidate_bytes = f.data.end;
    f.data.rotate(&mut f.c)?;
    changes.sort_by_key(|(key, _)| *key);
    let active = f
        .multi(Some(old.active), &changes)?
        .ok_or("density locator")?;
    // Both initial semantic roots are the same baseline, so publish one shared
    // placement only after retaining the prior recovery placement in phase one.
    ensure(
        old.semantic.active == old.semantic.recovery,
        "density requires equal semantic roots",
    )?;
    f.publish(
        active.clone(),
        old.recovery,
        old.semantic.clone(),
        Cut::None,
    )?;
    f.publish(active.clone(), active, old.semantic, Cut::None)?;
    let construction_us = construction.elapsed().as_micros();
    let construction_counters = f.c.clone();
    let before = f.snapshot()?;
    let selected = f.head.clone();
    f.c = Counters::default();
    let t = Instant::now();
    let result = f.compact_data(candidate, Cut::None, true)?;
    let us = t.elapsed().as_micros();
    let counters = f.c.clone();
    let after = f.snapshot()?;
    if garbage {
        ensure(
            result["deferred_nonpositive_standalone"] == false && after.0 < before.0,
            "high garbage candidate did not yield net reclaim",
        )?;
    } else {
        ensure(
            result["deferred_nonpositive_standalone"] == true
                && after == before
                && f.head == selected
                && counters.meta_write_bytes == 0
                && counters.data_write_bytes == 0
                && counters.envelope_write_bytes == 0,
            "all-live candidate not deferred without writes",
        )?;
    }
    for original in originals.iter().step_by(127) {
        let Node::Receipt { id: operation, .. } = source.get(original)? else {
            return Err("density non-receipt".into());
        };
        ensure(
            f.retry(&operation, &request(&operation))? == *original,
            "density relocation changed original receipt",
        )?;
    }
    Ok(
        json!({"case":if garbage{"high-garbage"}else{"all-live-low-yield"},"candidate_pack":candidate,"candidate_bytes":candidate_bytes,"live_encoded_bytes":live_bytes,"selected_original_receipts":originals.len(),"construction_us":construction_us,"construction_counters":construction_counters,"compaction_us":us,"result":result,"counters":counters,"before":before,"after":after}),
    )
}
fn run_density(n: u64, kind: Kind) -> Result<serde_json::Value> {
    ensure(n > 0, "positive density operation count")?;
    let temp = tempfile::tempdir().map_err(error)?;
    let sd = temp.path().join("source");
    fs::create_dir(&sd).map_err(error)?;
    let mut source = Store::open(&sd)?;
    build(&mut source, n, kind)?;
    let mut session = Session::imported();
    session.audit(&mut source)?;
    let t = Instant::now();
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    let bootstrap_ms = t.elapsed().as_millis();
    let high = density_case(&mut f, &mut source, n, true)?;
    let low = density_case(&mut f, &mut source, n, false)?;
    let t = Instant::now();
    let audited = f.audit()?;
    let audit_ms = t.elapsed().as_millis();
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"bootstrap_ms":bootstrap_ms,"cases":[high,low],"final_audit_ms":audit_ms,"audited_objects_sum_both_roots":audited,"final_snapshot":f.snapshot()?}),
    )
}

fn logical_group(source: &mut Store, size: u64) -> Result<Head> {
    ensure(size > 0 && size <= 32, "bounded group size")?;
    let old_head = source.head()?;
    let old = root(source, &old_head.active)?;
    let authored = authored(source, &old)?;
    let mut map = old.map.clone();
    let mut sequence = old.sequence.clone();
    for ordinal in old.count + 1..=old.count + size {
        let operation = id(ordinal);
        let request = request(&operation);
        ensure(
            lookup(source, &map, &operation, old.kind)?.is_none(),
            "group duplicate operation",
        )?;
        let receipt = source.put(&Node::Receipt {
            id: operation.clone(),
            request: request.clone(),
            ordinal,
        })?;
        let entry = Entry {
            key: key(&operation),
            id: operation,
            request,
            ordinal,
            receipt: receipt.clone(),
        };
        map = match old.kind {
            Kind::Radix => radix_insert(source, &map, entry)?,
            Kind::Btree => btree_insert(source, &map, entry)?,
        };
        sequence = seq_append(source, &sequence, receipt)?;
    }
    prefix(source, &old.sequence, &sequence)?;
    let active = make_root(source, old.kind, old.count + size, map, sequence, authored)?;
    let next = Head {
        active,
        recovery: old_head.active,
    };
    source.publish(&next)?;
    Ok(next)
}
fn run_groups(n: u64, kind: Kind) -> Result<serde_json::Value> {
    ensure(n > 0, "positive operation count")?;
    let temp = tempfile::tempdir().map_err(error)?;
    let sd = temp.path().join("source");
    fs::create_dir(&sd).map_err(error)?;
    let mut source = Store::open(&sd)?;
    build(&mut source, n, kind)?;
    let mut session = Session::imported();
    session.audit(&mut source)?;
    let t = Instant::now();
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    let bootstrap_ms = t.elapsed().as_millis();
    let mut policies = Vec::new();
    for size in [8_u64, 32] {
        let start_count = root(&mut source, &f.head.semantic.active)?.count;
        let mut groups = Vec::new();
        for _ in 0..8 {
            let old_end = source.size();
            logical_group(&mut source, size)?;
            f.c = Counters::default();
            let before = f.snapshot()?;
            let t = Instant::now();
            let delta = f.append(&mut source, old_end)?;
            groups.push(json!({"us":t.elapsed().as_micros(),"logical_delta":delta,"counters":f.c,"before":before,"after":f.snapshot()?}));
        }
        let h = f.head.clone();
        let recovery_count = root(&mut source, &h.semantic.recovery)?.count;
        let count = root(&mut source, &h.semantic.active)?.count;
        ensure(
            count == start_count + size * 8 && recovery_count == count - size,
            "group ordinal continuity",
        )?;
        // Verify every original group receipt by its exact original ID/digest.
        for ordinal in start_count + 1..=count {
            let operation = id(ordinal);
            let receipt = f.retry(&operation, &request(&operation))?;
            ensure(
                matches!(source.get(&receipt)?,Node::Receipt{ordinal:n,..} if n==ordinal),
                "group receipt ordinal changed",
            )?;
        }
        f.cache.clear();
        f.c = Counters::default();
        let t = Instant::now();
        let operation = id(1);
        f.retry(&operation, &request(&operation))?;
        let retry = json!({"us":t.elapsed().as_micros(),"counters":f.c});
        f.cache.clear();
        f.c = Counters::default();
        let t = Instant::now();
        f.structural_probe()?;
        let open = json!({"us":t.elapsed().as_micros(),"counters":f.c});
        f.c = Counters::default();
        let t = Instant::now();
        f.publish(
            h.active.clone(),
            h.recovery.clone(),
            h.semantic.clone(),
            Cut::None,
        )?;
        let turnover = json!({"us":t.elapsed().as_micros(),"counters":f.c});
        f.cache.clear();
        f.c = Counters::default();
        let t = Instant::now();
        let recovery_objects = f.audit_root(&h.recovery, &h.semantic.recovery)?;
        let recovery =
            json!({"us":t.elapsed().as_micros(),"objects":recovery_objects,"counters":f.c});
        policies.push(json!({"batch":size,"operations":size*8,"groups":groups,"count":count,"recovery_count":recovery_count,"cold_oldest_retry":retry,"structural_probe":open,"root_turnover":turnover,"independent_recovery_audit":recovery}));
    }
    full_audit(&mut source)?;
    let t = Instant::now();
    let objects = f.audit()?;
    let audit_ms = t.elapsed().as_millis();
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"bootstrap_ms":bootstrap_ms,"policies":policies,"final_audit_ms":audit_ms,"audited_objects_sum_both_roots":objects,"final_snapshot":f.snapshot()?}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controlled_density_admits_reclaim_and_defers_growth() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            run_density(64, kind)?;
        }
        Ok(())
    }
    #[test]
    fn grouped_original_receipts_reconcile_without_new_ordinals() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for size in [8_u64, 32] {
                for cut in [Cut::BeforeHead, Cut::AfterHead] {
                    let temp = tempfile::tempdir().map_err(error)?;
                    let sd = temp.path().join("source");
                    fs::create_dir(&sd).map_err(error)?;
                    let mut source = Store::open(&sd)?;
                    build(&mut source, 32, kind)?;
                    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
                    let end = source.size();
                    logical_group(&mut source, size)?;
                    ensure(
                        f.append_cut(&mut source, end, cut).is_err(),
                        "group cut missing",
                    )?;
                    let expected = if cut == Cut::BeforeHead {
                        "exact-old"
                    } else {
                        "exact-candidate"
                    };
                    ensure(f.reconcile()? == expected, "group outcome changed")?;
                    if cut == Cut::BeforeHead {
                        f.append(&mut source, end)?;
                    }
                    ensure(
                        root(&mut source, &f.head.semantic.active)?.count == 32 + size
                            && root(&mut source, &f.head.semantic.recovery)?.count == 32,
                        "group ordinal changed on recovery",
                    )?;
                    for ordinal in 1..=32 + size {
                        let operation = id(ordinal);
                        let receipt = f.retry(&operation, &request(&operation))?;
                        ensure(
                            matches!(source.get(&receipt)?,Node::Receipt{ordinal:n,..} if n==ordinal),
                            "original group receipt mismatch",
                        )?;
                    }
                    full_audit(&mut source)?;
                    f.audit()?;
                }
            }
        }
        Ok(())
    }
    #[test]
    fn extra_reader_and_undo_pins_require_exact_release() -> Result<()> {
        for class in ["reader", "undo"] {
            let temp = tempfile::tempdir().map_err(error)?;
            let sd = temp.path().join("source");
            fs::create_dir(&sd).map_err(error)?;
            let mut source = Store::open(&sd)?;
            build(&mut source, 1000, Kind::Radix)?;
            let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
            let pin = f.pin("fixture-owner", class)?;
            let before = f.snapshot()?;
            ensure(
                f.compact_data(0, Cut::None, false).is_err()
                    && f.compact_meta(0, Cut::None, false).is_err(),
                "extra pin did not refuse compaction",
            )?;
            ensure(f.snapshot()? == before, "pinned refusal wrote data")?;
            let mut wrong = pin.clone();
            wrong.owner = "other-owner".into();
            ensure(
                f.release_pin(&wrong).is_err() && f.dir.join("extra-pin").exists(),
                "wrong owner released pin",
            )?;
            f.audit_root(&pin.selection.active, &pin.selection.semantic.active)?;
            f.release_pin(&pin)?;
            f.compact_data(0, Cut::None, false)?;
            f.audit()?;
        }
        Ok(())
    }
    #[test]
    fn coalesced_updates_and_no_write_defer() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let temp = tempfile::tempdir().map_err(error)?;
            let sd = temp.path().join("source");
            fs::create_dir(&sd).map_err(error)?;
            let mut source = Store::open(&sd)?;
            build(&mut source, 1000, kind)?;
            let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
            let before = f.snapshot()?;
            let old = f.head.clone();
            f.c = Counters::default();
            let deferred = f.compact_data(0, Cut::None, true)?;
            ensure(
                deferred["deferred_nonpositive_standalone"] == true,
                "all-live pack did not defer",
            )?;
            ensure(
                f.snapshot()? == before
                    && f.head == old
                    && f.c.meta_write_bytes == 0
                    && f.c.data_write_bytes == 0
                    && f.c.envelope_write_bytes == 0,
                "deferred preview wrote bytes",
            )?;
            let mut session = Session::imported();
            session.audit(&mut source)?;
            for i in 1001..=1032 {
                let end = source.size();
                session.accept(
                    &mut source,
                    &id(i),
                    &request(&id(i)),
                    super::super::Cut::None,
                    RESERVE,
                )?;
                f.append(&mut source, end)?;
                if i % 8 == 0 {
                    f.audit()?;
                }
            }
            f.audit()?;
        }
        Ok(())
    }
    #[test]
    fn exact_reconciliation_and_bounded_pack_retirement() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for metadata in [false, true] {
                for cut in [Cut::BeforeHead, Cut::AfterHead] {
                    let temp = tempfile::tempdir().map_err(error)?;
                    let sd = temp.path().join("source");
                    fs::create_dir(&sd).map_err(error)?;
                    let mut source = Store::open(&sd)?;
                    build(&mut source, 1000, kind)?;
                    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
                    let before = f.head.clone();
                    let op = id(1);
                    let original_receipt = f.retry(&op, &request(&op))?;
                    ensure(
                        f.retry(&op, "conflicting request digest").is_err(),
                        "conflicting retry accepted",
                    )?;
                    let old_path = if metadata {
                        f.meta.path(0)
                    } else {
                        f.data.path(0)
                    };
                    let result = if metadata {
                        f.compact_meta(0, cut, false)
                    } else {
                        f.compact_data(0, cut, false)
                    };
                    ensure(result.is_err(), "fault not injected")?;
                    ensure(
                        old_path.exists() && f.dir.join("intent").exists(),
                        "interruption lost old bytes or evidence",
                    )?;
                    let physical_before = f.snapshot()?;
                    let end = source.size();
                    ensure(
                        f.append(&mut source, end).is_err(),
                        "pending transaction permitted new work",
                    )?;
                    ensure(
                        f.snapshot()? == physical_before,
                        "pending refusal wrote physical bytes",
                    )?;
                    ensure(
                        f.head.semantic == before.semantic,
                        "physical-only transition altered authored roots",
                    )?;
                    let expected = if cut == Cut::BeforeHead {
                        "exact-old"
                    } else {
                        "exact-candidate"
                    };
                    ensure(f.reconcile()? == expected, "reconciliation changed outcome")?;
                    if metadata {
                        f.compact_meta(0, Cut::None, false)?;
                    } else {
                        f.compact_data(0, Cut::None, false)?;
                    }
                    ensure(!old_path.exists(), "released candidate pack retained")?;
                    f.audit()?;
                    ensure(
                        f.retry(&op, &request(&op))? == original_receipt,
                        "relocation changed original receipt identity",
                    )?;
                }
            }
        }
        Ok(())
    }
    #[test]
    fn unknown_selector_and_missing_metadata_refuse() -> Result<()> {
        let temp = tempfile::tempdir().map_err(error)?;
        let sd = temp.path().join("source");
        fs::create_dir(&sd).map_err(error)?;
        let mut source = Store::open(&sd)?;
        build(&mut source, 1000, Kind::Radix)?;
        let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
        ensure(
            f.compact_meta(0, Cut::AfterHead, false).is_err(),
            "fault not injected",
        )?;
        let original = fs::read(f.dir.join("HEAD")).map_err(error)?;
        let mut unknown = f.head.clone();
        unknown.epoch += 9;
        fs::write(
            f.dir.join("HEAD"),
            serde_json::to_vec(&unknown).map_err(error)?,
        )
        .map_err(error)?;
        ensure(
            f.reconcile().is_err() && f.dir.join("intent").exists() && f.meta.path(0).exists(),
            "unknown selector discarded evidence",
        )?;
        fs::write(f.dir.join("HEAD"), original).map_err(error)?;
        f.reconcile()?;
        let root = f.head.active.clone();
        f.cache.clear();
        let file = OpenOptions::new()
            .write(true)
            .open(f.meta.path(root.pack))
            .map_err(error)?;
        file.set_len(root.offset).map_err(error)?;
        ensure(f.get(&root).is_err(), "missing selected metadata accepted")?;
        Ok(())
    }
}
