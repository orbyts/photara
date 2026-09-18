//! Disposable combined bounded placement, all-class pins and capacity-liability gate.
//! Forked v2 experiment; original v2 source/evidence remain unchanged.
#![allow(
    dead_code,
    reason = "Disposable inherited primitives retained for comparison"
)]
//! All schemas here are fixture-only, not proposed permanent wire.
#[allow(
    clippy::wildcard_imports,
    reason = "Disposable companion shares synthetic logical indexes"
)]
use super::*;
use std::collections::{HashMap, VecDeque};

#[path = "placement_v3/ownership_probe.rs"]
mod ownership_probe;
#[path = "placement_v3/typed_inventory.rs"]
mod typed_inventory;
use typed_inventory::{Inventory, Membership};

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
struct PreparedAppend {
    data: WritePlan,
    meta: WritePlan,
    active: PRef,
    recovery: PRef,
    semantic: Head,
    added: usize,
    removed: usize,
}
fn physical_bound(data: Option<&WritePlan>, meta: &WritePlan) -> Result<serde_json::Value> {
    fn stats(plan: &WritePlan) -> (u64, u64, u64) {
        let packs = plan
            .writes
            .iter()
            .map(|(_, r, _)| r.pack)
            .collect::<HashSet<_>>()
            .len() as u64;
        (plan.bytes, plan.writes.len() as u64, packs)
    }
    let d = data.map_or((0, 0, 0), stats);
    let m = stats(meta);
    let packs = checked(d.2, m.2)?;
    // A touched pack may allocate up to its complete bounded capacity even
    // when only a short prefix is written. Namespace and COW allowance remain
    // explicit fixture assumptions, not a universal APFS allocation theorem.
    let bound = packs
        .checked_mul(PACK + 16384)
        .and_then(|v| v.checked_add(65536))
        .ok_or("physical budget overflow")?;
    Ok(
        json!({"data_bytes":d.0,"metadata_bytes":m.0,"data_records":d.1,"metadata_pages":m.1,"data_touched_packs":d.2,"metadata_touched_packs":m.2,"allocation_bound":bound,"allowance":"full256KiB per touched pack +16KiB namespace per touched pack +64KiB COW uncertainty; modeled, not OS reservation"}),
    )
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
        ensure(key.is_some() || bytes.len() <= 4092, "metadata page budget")?;
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
    membership: Membership,
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
    inventory: Inventory,
    semantic: Head,
    gate: GateState,
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
    imported_read_only: bool,
    dir: PathBuf,
    data: Arena,
    meta: Arena,
    head: Selection,
    cache: VecDeque<(PRef, Page)>,
    c: Counters,
    staged: Option<Vec<Page>>,
    desired_gate: Option<GateState>,
    authorized_hold: Option<u64>,
    planning: bool,
}
fn direction(key: u64, bit: u8) -> bool {
    key & (1_u64 << (63 - bit)) != 0
}
fn difference(a: u64, b: u64) -> u8 {
    u8::try_from((a ^ b).leading_zeros()).unwrap()
}
impl Fixture {
    fn control_bound(&self, gate: &GateState) -> Result<u64> {
        let mut h = self.head.clone();
        h.gate = gate.clone();
        h.epoch = u64::MAX;
        for domain in h.gate.domains.values_mut() {
            domain.charged = u64::MAX;
            domain.limit = u64::MAX;
        }
        let len = serde_json::to_vec(&h).map_err(error)?.len() as u64;
        ensure(len <= GATE_MAX as u64, "control envelope bound")?;
        Ok(4 * round(len + 4096) + 8 * 4096 + 65536)
    }
    fn check_continuation(&self, actual: &Continuation) -> Result<()> {
        let hold = self
            .authorized_hold
            .and_then(|t| self.head.gate.holds.get(&t))
            .ok_or("original prepared hold")?;
        ensure(
            hold.continuation.as_ref() == Some(actual),
            "prepared placement departed from original liability; do not duplicate uncertain effects",
        )
    }
    fn compaction_plan(&mut self, data: bool, pack: u64) -> Result<serde_json::Value> {
        ensure(
            self.head.gate.holds.is_empty() && !self.dir.join("intent").exists(),
            "pending original operation blocks planning",
        )?;
        self.planning = true;
        let result = if data {
            self.compact_data(pack, Cut::None, false)
        } else {
            self.compact_meta(pack, Cut::None, false)
        };
        self.planning = false;
        self.staged = None;
        result
    }
    fn compact_economic(&mut self, data: bool, pack: u64) -> Result<serde_json::Value> {
        let plan = self.compaction_plan(data, pack)?;
        let net = plan["preview"]["standalone_net_file_bytes"]
            .as_i64()
            .ok_or("profitability plan")?;
        if net <= 0 {
            return Ok(json!({"deferred":true,"plan":plan}));
        }
        Ok(
            json!({"deferred":false,"plan":plan,"execution":self.compact_combined(data,pack,Fault::None)?}),
        )
    }
    fn put(&mut self, page: &Page) -> Result<PRef> {
        ensure(
            serde_json::to_vec(page).map_err(error)?.len() <= 4092,
            "bounded metadata page encoding",
        )?;
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
            loc.membership == Membership::Semantic && r.offset < typed_inventory::OWNER_KEY,
            "semantic membership/namespace",
        )?;
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
            membership: Membership::Semantic,
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
        ensure(bytes.len() <= GATE_MAX, "bounded envelope")?;
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
            !self.imported_read_only,
            "imported inventory requires explicit full audit",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "unresolved fixture transaction",
        )?;
        let next = Selection {
            inventory: self.selected_inventory(&active, &recovery)?,
            semantic,
            gate: self
                .desired_gate
                .clone()
                .unwrap_or_else(|| self.head.gate.clone()),
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
        next.gate.validate()?;
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
        self.desired_gate = None;
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
            inventory: Inventory::default(),
            semantic: semantic.clone(),
            gate: GateState::default(),
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
            imported_read_only: false,
            dir: dir.to_owned(),
            data,
            meta,
            head,
            cache: VecDeque::new(),
            c,
            staged: None,
            desired_gate: None,
            authorized_hold: None,
            planning: false,
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
        self.authorize_pending()?;
        let original_hold = self
            .authorized_hold
            .and_then(|t| self.head.gate.holds.get(&t))
            .ok_or("append requires original admitted liability")?;
        ensure(
            original_hold.retirement.is_none() && original_hold.target == source.head()?,
            "append liability work identity mismatch",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction before new append",
        )?;
        let plan = self.checkpoint_plan(source, old_end)?;
        self.apply_append(plan, cut)
    }
    fn checkpoint_plan(&mut self, source: &mut Store, old_end: u64) -> Result<PreparedAppend> {
        let old = self.head.clone();
        let semantic = source.head()?;
        ensure(
            semantic.recovery == old.semantic.active,
            "logical selected prefix discontinuity",
        )?;
        let (added, removed) = changes(source, &old.semantic.active, &semantic.active, old_end)?;
        let mut updates = BTreeMap::new();
        let mut data = WritePlan::from(&self.data);
        for r in &removed {
            ensure(
                self.lookup(&old.active, r.offset)?
                    .is_some_and(|l| l.object == *r),
                "removed object absent",
            )?;
            updates.insert(r.offset, None);
        }
        for r in &added {
            let mut bytes = vec![0; usize::try_from(r.len).map_err(error)?];
            source
                .file
                .seek(SeekFrom::Start(r.offset))
                .and_then(|_| source.file.read_exact(&mut bytes))
                .map_err(error)?;
            ensure(hash(&bytes) == r.sha, "planned source hash")?;
            let loc = Location {
                membership: Membership::Semantic,
                object: r.clone(),
                data: data.push(bytes, Some(r.offset))?,
            };
            updates.insert(r.offset, Some(loc));
        }
        self.staged = Some(Vec::new());
        let root = self
            .multi(
                Some(old.active.clone()),
                &updates.into_iter().collect::<Vec<_>>(),
            )?
            .ok_or("empty current locator")?;
        let pages = self.staged.take().ok_or("missing append planning pages")?;
        let mut meta = WritePlan::from(&self.meta);
        let active = meta.staged(&root, &pages, &mut HashMap::new())?;
        Ok(PreparedAppend {
            data,
            meta,
            active,
            recovery: old.active,
            semantic,
            added: added.len(),
            removed: removed.len(),
        })
    }
    fn apply_append(&mut self, plan: PreparedAppend, cut: Cut) -> Result<serde_json::Value> {
        self.authorize_pending()?;
        let hold = self
            .authorized_hold
            .and_then(|t| self.head.gate.holds.get(&t))
            .ok_or("original append hold")?;
        ensure(
            hold.retirement.is_none()
                && hold.target == plan.semantic
                && plan.recovery == self.head.active,
            "prepared append identity",
        )?;
        let mut actual = Continuation::append(&plan);
        actual.inventory = Inventory {
            active: self.head.inventory.active.clone(),
            recovery: self.head.inventory.active.clone(),
            next: self.head.inventory.next,
        };
        self.check_continuation(&actual)?;
        let delta = json!({"added":plan.added,"removed":plan.removed});
        plan.data.write(&mut self.data, &mut self.c)?;
        plan.meta.write(&mut self.meta, &mut self.c)?;
        self.publish(plan.active, plan.recovery, plan.semantic, cut)?;
        Ok(delta)
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
        let owned = self.audit_inventory(root)?;
        let mut locations = HashSet::new();
        while let Some((at, path)) = pages.pop() {
            ensure(path.len() <= 64, "audit locator depth")?;
            match self.get(&at)? {
                Page::Leaf { key, value } => {
                    ensure(
                        key == value.object.offset
                            && match value.membership {
                                Membership::Semantic => {
                                    key < typed_inventory::OWNER_KEY && seen.contains(&value.object)
                                }
                                Membership::Ownership => {
                                    key >= typed_inventory::OWNER_KEY
                                        && owned.contains(&value.object)
                                }
                            }
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
        ensure(
            locations.len() == seen.len() + owned.len(),
            "locator coverage mismatch",
        )?;
        Ok(seen.len() + owned.len())
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
            self.inventory_probe(locator)?;
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
    #[allow(
        clippy::too_many_lines,
        reason = "Keep disposable two-selection plan and write ordering adjacent"
    )]
    fn compact_data(
        &mut self,
        pack: u64,
        cut: Cut,
        defer_nonpositive: bool,
    ) -> Result<serde_json::Value> {
        ensure(pack < self.data.pack, "only sealed data pack may retire")?;
        self.authorize_retirement(true, pack)?;
        ensure(
            !self.dir.join("extra-pin").exists(),
            "extra pinned root blocks retirement",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction pins old bytes",
        )?;
        self.gate_candidate(pack, true)?;
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
        let preview = json!({"standalone_net_file_bytes":net,"data_encoded_bytes":data_plan.bytes,"metadata_encoded_bytes":meta_plan.bytes,"envelope_process_bytes":old_len+3*middle_len+2*final_len,"extra_file_bytes_upper_estimate":data_plan.bytes+meta_plan.bytes+4*old_len.max(middle_len).max(final_len),"coupled_followup_not_planned":true,"filesystem_allocation_not_predicted":true,"physical_plan":physical_bound(Some(&data_plan),&meta_plan)?});
        let continuation = Continuation::from_selection(&final_head);
        if self.planning {
            return Ok(json!({"preview":preview,"continuation":continuation}));
        }
        self.check_continuation(&continuation)?;
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
            pack < self.meta.pack,
            "only sealed metadata pack may retire",
        )?;
        self.authorize_retirement(false, pack)?;
        ensure(
            !self.dir.join("extra-pin").exists(),
            "extra pinned root blocks metadata retirement",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending transaction pins metadata",
        )?;
        self.gate_candidate(pack, false)?;
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
        let preview = json!({"standalone_net_file_bytes":net,"metadata_encoded_bytes":plan.bytes,"envelope_process_bytes":old_len+3*middle_len+2*final_len,"extra_file_bytes_upper_estimate":plan.bytes+4*old_len.max(middle_len).max(final_len),"coupled_followup_not_planned":true,"filesystem_allocation_not_predicted":true,"physical_plan":physical_bound(None,&plan)?});
        let continuation = Continuation::from_selection(&final_head);
        if self.planning {
            return Ok(json!({"preview":preview,"continuation":continuation}));
        }
        self.check_continuation(&continuation)?;
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

const GATE_MAX: usize = 256 * 1024;
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
const MAX_PINS: usize = 64;
#[cfg(test)]
#[path = "../ps2_combined_v3_checks.rs"]
mod independent_checks;
const MAX_HOLDS: usize = 16;
const CONTROL: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum PinClass {
    Active,
    Recovery,
    AcceptedJournal,
    Undo,
    History,
    ConversionSource,
    Export,
    Backup,
    ReaderLease,
    UnresolvedIntent,
    Relocation,
    ResourceObligation,
}
const EXTRA_CLASSES: [PinClass; 10] = [
    PinClass::AcceptedJournal,
    PinClass::Undo,
    PinClass::History,
    PinClass::ConversionSource,
    PinClass::Export,
    PinClass::Backup,
    PinClass::ReaderLease,
    PinClass::UnresolvedIntent,
    PinClass::Relocation,
    PinClass::ResourceObligation,
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Obligation {
    inventory: Inventory,
    token: u64,
    epoch: u64,
    class: PinClass,
    semantic: Head,
    active: PRef,
    recovery: PRef,
    unknown: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CapacityDomain {
    limit: u64,
    charged: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Liability {
    token: u64,
    epoch: u64,
    target: Head,
    // None is append/checkpoint; Some identifies the exact old pack to retire.
    retirement: Option<(bool, u64)>,
    by_domain: BTreeMap<u64, u64>,
    continuation: Option<Continuation>,
    control: u64,
    origin: Continuation,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Continuation {
    inventory: Inventory,
    active: PRef,
    recovery: PRef,
    data_pack: u64,
    data_end: u64,
    meta_pack: u64,
    meta_end: u64,
}
impl Continuation {
    fn from_selection(h: &Selection) -> Self {
        Self {
            inventory: h.inventory.clone(),
            active: h.active.clone(),
            recovery: h.recovery.clone(),
            data_pack: h.data_pack,
            data_end: h.data_end,
            meta_pack: h.meta_pack,
            meta_end: h.meta_end,
        }
    }
    fn append(p: &PreparedAppend) -> Self {
        Self {
            inventory: Inventory::default(),
            active: p.active.clone(),
            recovery: p.recovery.clone(),
            data_pack: p.data.pack,
            data_end: p.data.end,
            meta_pack: p.meta.pack,
            meta_end: p.meta.end,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct GateState {
    pins: BTreeMap<u64, Obligation>,
    holds: BTreeMap<u64, Liability>,
    domains: BTreeMap<u64, CapacityDomain>,
    last_token: u64,
}
fn checked(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("capacity overflow".into())
}
impl GateState {
    fn held(&self, domain: u64) -> Result<u64> {
        self.holds.values().try_fold(0, |a, h| {
            checked(a, h.by_domain.get(&domain).copied().unwrap_or(0))
        })
    }
    fn validate(&self) -> Result<()> {
        ensure(
            self.pins.len() <= MAX_PINS
                && self.holds.len() <= MAX_HOLDS
                && self.domains.len() <= 16,
            "concurrent gate admission bound",
        )?;
        for (token, pin) in &self.pins {
            ensure(
                *token == pin.token && *token <= self.last_token && !self.holds.contains_key(token),
                "pin token structure",
            )?;
        }
        for (token, hold) in &self.holds {
            ensure(
                *token == hold.token && *token <= self.last_token,
                "hold token structure",
            )?;
            for domain in hold.by_domain.keys() {
                ensure(self.domains.contains_key(domain), "unknown capacity domain")?;
            }
        }
        for (id, domain) in &self.domains {
            ensure(
                checked(domain.charged, self.held(*id)?)? <= domain.limit,
                "capacity refusal",
            )?;
        }
        Ok(())
    }
    fn token(&mut self) -> Result<u64> {
        self.last_token = checked(self.last_token, 1)?;
        Ok(self.last_token)
    }
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    EnospcBefore,
    EnospcBeforeHead,
    EnospcAfterHead,
}

impl Fixture {
    fn authorize_pending(&self) -> Result<()> {
        ensure(
            !self.imported_read_only,
            "imported inventory requires explicit full audit",
        )?;
        ensure(
            self.head.gate.holds.is_empty()
                || (self.head.gate.holds.len() == 1
                    && self
                        .authorized_hold
                        .is_some_and(|t| self.head.gate.holds.contains_key(&t))),
            "pending original liability blocks unrelated physical work",
        )
    }
    fn authorize_retirement(&self, data: bool, pack: u64) -> Result<()> {
        if self.planning {
            return ensure(
                self.head.gate.holds.is_empty(),
                "pending hold blocks new plan",
            );
        }
        self.authorize_pending()?;
        let hold = self
            .authorized_hold
            .and_then(|t| self.head.gate.holds.get(&t))
            .ok_or("retirement requires original admitted liability")?;
        ensure(
            hold.retirement == Some((data, pack)) && hold.target == self.head.semantic,
            "retirement liability work identity mismatch",
        )
    }
    fn initialize_gate(&mut self) -> Result<()> {
        let mut gate = self.head.gate.clone();
        ensure(gate.domains.is_empty(), "already initialized")?;
        // Bootstrap-only exhaustive allocation observation; never a checkpoint scan.
        gate.domains.insert(
            0,
            CapacityDomain {
                limit: 64 * 1024 * 1024 * 1024,
                charged: self.snapshot()?.1,
            },
        );
        self.select_gate(gate)
    }
    fn select_gate(&mut self, mut gate: GateState) -> Result<()> {
        ensure(
            !self.imported_read_only,
            "imported inventory requires explicit full audit",
        )?;
        ensure(
            !self.dir.join("intent").exists(),
            "pending selection retains all gate state",
        )?;
        // Bounded HEAD/intent/candidate staging and namespace allowance. These
        // are modeled caps, not a claim about exclusive OS/APFS reservation.
        let control = self.control_bound(&gate)?;
        if let Some(domain) = gate.domains.get_mut(&0) {
            domain.charged = checked(domain.charged, control)?;
        }
        gate.validate()?;
        self.desired_gate = Some(gate);
        let h = self.head.clone();
        let result = self.publish(h.active, h.recovery, h.semantic, Cut::None);
        if result.is_err() {
            self.desired_gate = None;
        }
        result
    }
    fn acquire(&mut self, class: PinClass, unknown: bool) -> Result<Obligation> {
        ensure(
            !matches!(class, PinClass::Active | PinClass::Recovery),
            "native root owns active/recovery pin",
        )?;
        let mut gate = self.head.gate.clone();
        ensure(gate.pins.len() < MAX_PINS, "pin admission cap")?;
        let token = gate.token()?;
        let pin = Obligation {
            inventory: self.head.inventory.clone(),
            token,
            epoch: self.head.epoch,
            class,
            semantic: self.head.semantic.clone(),
            active: self.head.active.clone(),
            recovery: self.head.recovery.clone(),
            unknown,
        };
        gate.pins.insert(token, pin.clone());
        self.select_gate(gate)?;
        Ok(pin)
    }
    fn release(
        &mut self,
        exact: &Obligation,
        completed: bool,
        reader_finished: bool,
        reconciled: bool,
    ) -> Result<()> {
        ensure(
            completed
                && (exact.class != PinClass::ReaderLease || reader_finished)
                && (!exact.unknown || reconciled),
            "release proof incomplete",
        )?;
        let mut gate = self.head.gate.clone();
        ensure(
            gate.pins.get(&exact.token) == Some(exact),
            "exact pin token/epoch/snapshot required",
        )?;
        gate.pins.remove(&exact.token);
        self.select_gate(gate)
    }
    fn reserve(
        &mut self,
        target: Head,
        retirement: Option<(bool, u64)>,
        requests: &[(u64, u64)],
    ) -> Result<Liability> {
        self.reserve_with(target, retirement, requests, None)
    }
    fn reserve_with(
        &mut self,
        target: Head,
        retirement: Option<(bool, u64)>,
        requests: &[(u64, u64)],
        continuation: Option<Continuation>,
    ) -> Result<Liability> {
        let mut gate = self.head.gate.clone();
        ensure(gate.holds.len() < MAX_HOLDS, "liability admission cap")?;
        let token = gate.token()?;
        let mut by_domain = BTreeMap::<u64, u64>::new();
        // Sum first, validate all domains atomically: two paths on one volume
        // never receive independent copies of the same available capacity.
        for (id, bytes) in requests {
            by_domain.insert(
                *id,
                checked(by_domain.get(id).copied().unwrap_or(0), *bytes)?,
            );
        }
        // The final durable release is itself reserved; never rely on capacity
        // becoming available by deleting the old placement or clearing the hold.
        let control = self.control_bound(&gate)? + 65536;
        by_domain.insert(
            0,
            checked(by_domain.get(&0).copied().unwrap_or(0), control)?,
        );
        let hold = Liability {
            token,
            epoch: self.head.epoch,
            target,
            retirement,
            by_domain,
            continuation,
            control,
            origin: Continuation {
                inventory: self.head.inventory.clone(),
                active: self.head.active.clone(),
                recovery: self.head.recovery.clone(),
                data_pack: self.data.pack,
                data_end: self.data.end,
                meta_pack: self.meta.pack,
                meta_end: self.meta.end,
            },
        };
        gate.holds.insert(token, hold.clone());
        gate.validate()?;
        self.select_gate(gate)?;
        Ok(hold)
    }
    fn complete_liability(&mut self, exact: &Liability, used: &[(u64, u64)]) -> Result<()> {
        ensure(
            !self.dir.join("intent").exists(),
            "unknown outcome retains original liability",
        )?;
        let mut gate = self.head.gate.clone();
        ensure(
            gate.holds.get(&exact.token) == Some(exact),
            "exact original liability required",
        )?;
        ensure(
            self.head.semantic == exact.target,
            "checkpoint target not selected",
        )?;
        if let Some((data, pack)) = exact.retirement {
            ensure(
                !(if data {
                    self.data.path(pack)
                } else {
                    self.meta.path(pack)
                })
                .exists(),
                "old placement remains charged/pinned",
            )?;
        }
        let mut totals = BTreeMap::<u64, u64>::new();
        for (id, bytes) in used {
            totals.insert(*id, checked(totals.get(id).copied().unwrap_or(0), *bytes)?);
        }
        gate.holds.remove(&exact.token);
        let control = self.control_bound(&gate)?;
        ensure(
            control <= exact.control,
            "selector growth exceeds admitted completion allowance",
        )?;
        totals.insert(
            0,
            checked(totals.get(&0).copied().unwrap_or(0), exact.control)?,
        );
        for (id, bytes) in totals {
            ensure(
                bytes <= exact.by_domain.get(&id).copied().unwrap_or(0),
                "allocation exceeded admitted fixture bound",
            )?;
            let domain = gate
                .domains
                .get_mut(&id)
                .ok_or("wrong consumption domain")?;
            // select_gate below charges CONTROL once; it is included above for
            // admission checking but must not be charged twice.
            domain.charged = checked(
                domain.charged,
                if id == 0 { bytes - control } else { bytes },
            )?;
        }
        // Only unused hold is released; old physical bytes are never subtracted.
        self.select_gate(gate)
    }
    fn gate_candidate(&mut self, pack: u64, data: bool) -> Result<()> {
        self.authorize_pending()?;
        self.inventory_retirement_guard(pack, data)?;
        self.head.gate.validate()?;
        let pins = self.head.gate.pins.values().cloned().collect::<Vec<_>>();
        if pins.is_empty() {
            return Ok(());
        }
        let records = if data {
            self.data.records(pack, true)?
        } else {
            self.meta.records(pack, false)?
        };
        ensure(records.len() <= TX_OBJECTS, "bounded candidate pins")?;
        self.c.candidate_records += records.len() as u64;
        self.c.candidate_read_bytes += if data {
            fs::metadata(self.data.path(pack))
        } else {
            fs::metadata(self.meta.path(pack))
        }
        .map_err(error)?
        .len();
        for pin in &pins {
            for (key, physical, bytes) in &records {
                for root in [&pin.active, &pin.recovery] {
                    let referenced = if data {
                        self.lookup(root, key.ok_or("data framing")?)?
                            .is_some_and(|l| l.data == *physical)
                    } else {
                        let page: Page = serde_json::from_slice(bytes).map_err(error)?;
                        self.path_contains(root, physical, page.anchor())?
                    };
                    ensure(
                        !referenced,
                        "candidate retained by exact all-class snapshot pin",
                    )?;
                }
            }
        }
        Ok(())
    }
    fn bounded_read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
        ensure(
            fs::metadata(path).map_err(error)?.len() <= GATE_MAX as u64,
            "envelope size bound",
        )?;
        serde_json::from_slice(&fs::read(path).map_err(error)?).map_err(error)
    }
    fn reopen_combined(dir: &Path) -> Result<Self> {
        let mut envelope_read_bytes = fs::metadata(dir.join("HEAD")).map_err(error)?.len();
        let head: Selection = Self::bounded_read(&dir.join("HEAD"))?;
        head.gate.validate()?;
        let mut extent = head.clone();
        if dir.join("intent").exists() {
            let old: Selection = Self::bounded_read(&dir.join("intent"))?;
            let candidate: Selection = Self::bounded_read(&dir.join("candidate"))?;
            ensure(
                head == old || head == candidate,
                "unknown HEAD preserves pins, liability and physical evidence",
            )?;
            old.gate.validate()?;
            candidate.gate.validate()?;
            envelope_read_bytes += fs::metadata(dir.join("intent")).map_err(error)?.len()
                + fs::metadata(dir.join("candidate")).map_err(error)?.len();
            extent = candidate;
        }
        // A prior exact-old reconciliation may have removed intent/candidate
        // before restart. Inspect only each pending original transaction's
        // bounded planned range, never the lifetime directory inventory.
        for hold in head.gate.holds.values() {
            if let Some(cont) = &hold.continuation {
                for (prefix, start, end) in [
                    ("data", hold.origin.data_pack, cont.data_pack),
                    ("meta", hold.origin.meta_pack, cont.meta_pack),
                ] {
                    ensure(
                        end >= start && end - start <= 8194,
                        "pending extent admission bound",
                    )?;
                    for pack in start..=end {
                        match fs::metadata(dir.join(format!("{prefix}-{pack}"))) {
                            Ok(m) => {
                                ensure(m.len() <= PACK, "pending pack extent bound")?;
                                let (current, current_end) = if prefix == "data" {
                                    (&mut extent.data_pack, &mut extent.data_end)
                                } else {
                                    (&mut extent.meta_pack, &mut extent.meta_end)
                                };
                                if pack >= *current {
                                    *current = pack;
                                    *current_end = m.len();
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                            Err(e) => return Err(error(e)),
                        }
                    }
                }
            }
        }
        let data = Self::reopen_arena(dir, "data", extent.data_pack, extent.data_end)?;
        let meta = Self::reopen_arena(dir, "meta", extent.meta_pack, extent.meta_end)?;
        Ok(Self {
            imported_read_only: false,
            dir: dir.into(),
            data,
            meta,
            head,
            cache: VecDeque::new(),
            c: Counters {
                head_read_bytes: envelope_read_bytes,
                ..Counters::default()
            },
            staged: None,
            desired_gate: None,
            authorized_hold: None,
            planning: false,
        })
    }
    fn reopen_arena(
        dir: &Path,
        prefix: &'static str,
        pack: u64,
        selected_end: u64,
    ) -> Result<Arena> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.join(format!("{prefix}-{pack}")))
            .map_err(error)?;
        let end = file.metadata().map_err(error)?.len();
        ensure(end >= selected_end && end <= PACK, "append-prefix extent")?;
        Ok(Arena {
            dir: dir.into(),
            prefix,
            pack,
            end,
            file,
            dirty: HashSet::new(),
        })
    }
    fn audit_combined(&mut self) -> Result<usize> {
        self.head.gate.validate()?;
        let mut count = self.audit()?;
        for pin in self.head.gate.pins.values().cloned().collect::<Vec<_>>() {
            count += self.audit_root(&pin.active, &pin.semantic.active)?;
            count += self.audit_root(&pin.recovery, &pin.semantic.recovery)?;
        }
        if let Some(domain) = self.head.gate.domains.get(&0) {
            ensure(
                checked(domain.charged, self.head.gate.held(0)?)? >= self.snapshot()?.1,
                "audit observed allocation exceeds charged+held model",
            )?;
        }
        self.imported_read_only = false;
        Ok(count)
    }
    fn checkpoint(
        &mut self,
        source: &mut Store,
        old_end: u64,
        fault: Fault,
    ) -> Result<serde_json::Value> {
        ensure(
            self.head.gate.holds.is_empty(),
            "resume original liability before new checkpoint",
        )?;
        let plan = self.checkpoint_plan(source, old_end)?;
        let preflight = physical_bound(Some(&plan.data), &plan.meta)?;
        let budget = checked(
            preflight["allocation_bound"].as_u64().ok_or("plan bound")?,
            self.control_bound(&self.head.gate)? * 3,
        )?;
        let mut continuation = Continuation::append(&plan);
        continuation.inventory = Inventory {
            active: self.head.inventory.active.clone(),
            recovery: self.head.inventory.active.clone(),
            next: self.head.inventory.next,
        };
        let hold = self.reserve_with(
            plan.semantic.clone(),
            None,
            &[(0, budget)],
            Some(continuation),
        )?;
        self.authorized_hold = Some(hold.token);
        if matches!(fault, Fault::EnospcBefore) {
            return Err("injected ENOSPC before physical effect; liability retained".into());
        }
        let before = self.c.clone();
        let cut = match fault {
            Fault::EnospcBeforeHead => Cut::BeforeHead,
            Fault::EnospcAfterHead => Cut::AfterHead,
            _ => Cut::None,
        };
        let delta = self.apply_append(plan, cut)?;
        let used = self.c.allocation_charge - before.allocation_charge
            + (self.c.files_created - before.files_created) * 16384;
        self.complete_liability(&hold, &[(0, used)])?;
        Ok(
            json!({"delta":delta,"preflight":preflight,"held_before_effect":hold.by_domain,"modeled_consumed":used}),
        )
    }
    fn compact_combined(
        &mut self,
        data: bool,
        pack: u64,
        fault: Fault,
    ) -> Result<serde_json::Value> {
        ensure(
            self.head.gate.holds.is_empty(),
            "resume original liability before new compaction",
        )?;
        ensure(
            pack < if data { self.data.pack } else { self.meta.pack },
            "candidate must be sealed before reservation",
        )?;
        let preflight = self.compaction_plan(data, pack)?;
        let budget = checked(
            preflight["preview"]["physical_plan"]["allocation_bound"]
                .as_u64()
                .ok_or("compaction allocation plan")?,
            self.control_bound(&self.head.gate)? * 4,
        )?;
        let continuation: Continuation =
            serde_json::from_value(preflight["continuation"].clone()).map_err(error)?;
        let hold = self.reserve_with(
            self.head.semantic.clone(),
            Some((data, pack)),
            &[(0, budget)],
            Some(continuation),
        )?;
        self.authorized_hold = Some(hold.token);
        if matches!(fault, Fault::EnospcBefore) {
            return Err("injected ENOSPC before compaction; both placements remain charged".into());
        }
        let before = self.c.clone();
        let cut = match fault {
            Fault::EnospcBeforeHead => Cut::BeforeHead,
            Fault::EnospcAfterHead => Cut::AfterHead,
            _ => Cut::None,
        };
        let result = if data {
            self.compact_data(pack, cut, false)?
        } else {
            self.compact_meta(pack, cut, false)?
        };
        let used = self.c.allocation_charge - before.allocation_charge
            + (self.c.files_created - before.files_created) * 16384;
        self.complete_liability(&hold, &[(0, used)])?;
        Ok(
            json!({"result":result,"preflight":preflight,"held_before_effect":hold.by_domain,"modeled_consumed":used}),
        )
    }
    fn resume_checkpoint(
        &mut self,
        source: &mut Store,
        old_end: u64,
        exact: &Liability,
    ) -> Result<()> {
        ensure(
            exact.retirement.is_none() && source.head()? == exact.target,
            "wrong original checkpoint",
        )?;
        ensure(
            self.head.gate.holds.get(&exact.token) == Some(exact),
            "original pending liability absent",
        )?;
        self.authorized_hold = Some(exact.token);
        let untouched = self.data.pack == exact.origin.data_pack
            && self.data.end == exact.origin.data_end
            && self.meta.pack == exact.origin.meta_pack
            && self.meta.end == exact.origin.meta_end;
        if untouched && !self.dir.join("intent").exists() {
            self.append(source, old_end)?;
        } else {
            self.continue_original(exact, Cut::None)?;
        }
        // Uncertain physical effects are conservatively charged to the full
        // admitted maximum. No orphan scan or guessed deletion credit.
        let consumed = exact
            .by_domain
            .iter()
            .map(|(a, b)| (*a, if *a == 0 { b - exact.control } else { *b }))
            .collect::<Vec<_>>();
        self.complete_liability(exact, &consumed)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Disposable original-token recovery keeps proof and selection ordering explicit"
    )]
    fn continue_original(&mut self, exact: &Liability, cut: Cut) -> Result<()> {
        ensure(
            self.head.gate.holds.get(&exact.token) == Some(exact),
            "original continuation liability required",
        )?;
        self.authorized_hold = Some(exact.token);
        let continuation = exact
            .continuation
            .clone()
            .ok_or("no prepared continuation")?;
        ensure(
            self.data.pack == continuation.data_pack
                && self.data.end == continuation.data_end
                && self.meta.pack == continuation.meta_pack
                && self.meta.end == continuation.meta_end,
            "planned output is incomplete or has unknown trailing effects",
        )?;
        let selected: Selection = Self::bounded_read(&self.dir.join("HEAD"))?;
        ensure(selected == self.head, "HEAD changed before continuation")?;
        if self.dir.join("intent").exists() {
            let old: Selection = Self::bounded_read(&self.dir.join("intent"))?;
            let candidate: Selection = Self::bounded_read(&self.dir.join("candidate"))?;
            ensure(
                selected == old || selected == candidate,
                "unknown HEAD retains all continuation evidence",
            )?;
            ensure(
                old.gate.holds.get(&exact.token) == Some(exact)
                    && candidate.gate.holds.get(&exact.token) == Some(exact),
                "continuation transaction identity",
            )?;
        }
        // Planned hashes alone are not completion evidence. Verify every root
        // against the original target before any selection, and barrier every
        // possibly newly written pack (bounded by this original transaction).
        ensure(
            continuation.data_pack >= exact.origin.data_pack
                && continuation.meta_pack >= exact.origin.meta_pack
                && continuation.data_pack - exact.origin.data_pack <= 8194
                && continuation.meta_pack - exact.origin.meta_pack <= 8194,
            "bounded continuation extents",
        )?;
        let saved = self.head.clone();
        self.head.data_pack = continuation.data_pack;
        self.head.data_end = continuation.data_end;
        self.head.meta_pack = continuation.meta_pack;
        self.head.meta_end = continuation.meta_end;
        self.cache.clear();
        let verification = (|| -> Result<()> {
            self.audit_root(&continuation.active, &exact.target.active)?;
            self.audit_root(&continuation.recovery, &exact.target.recovery)?;
            for (arena, start, end) in [
                (&self.data, exact.origin.data_pack, continuation.data_pack),
                (&self.meta, exact.origin.meta_pack, continuation.meta_pack),
            ] {
                for pack in start..=end {
                    sync_file(&File::open(arena.path(pack)).map_err(error)?, &mut self.c)?;
                }
            }
            sync_dir(&self.dir, &mut self.c)
        })();
        self.head = saved;
        verification?;
        if self.dir.join("intent").exists() {
            self.reconcile()?;
        }
        if let Some((data, pack)) = exact.retirement {
            if self.head.active != continuation.active {
                self.publish(
                    continuation.active.clone(),
                    self.head.recovery.clone(),
                    exact.target.clone(),
                    cut,
                )?;
            }
            if self.head.recovery != continuation.recovery {
                self.publish(
                    continuation.active.clone(),
                    continuation.recovery.clone(),
                    exact.target.clone(),
                    cut,
                )?;
            }
            self.gate_candidate(pack, data)?;
            let records = if data {
                self.data.records(pack, true)?
            } else {
                self.meta.records(pack, false)?
            };
            for (key, r, bytes) in records {
                for root in [&continuation.active, &continuation.recovery] {
                    let live = if data {
                        self.lookup(root, key.ok_or("retirement data key")?)?
                            .is_some_and(|l| l.data == r)
                    } else {
                        let page: Page = serde_json::from_slice(&bytes).map_err(error)?;
                        self.path_contains(root, &r, page.anchor())?
                    };
                    ensure(!live, "continuation pack still selected")?;
                }
            }
            fs::remove_file(if data {
                self.data.path(pack)
            } else {
                self.meta.path(pack)
            })
            .map_err(error)?;
            self.c.files_deleted += 1;
            sync_dir(&self.dir, &mut self.c)?;
        } else if self.head.semantic != exact.target
            || self.head.active != continuation.active
            || self.head.recovery != continuation.recovery
        {
            self.publish(
                continuation.active,
                continuation.recovery,
                exact.target.clone(),
                cut,
            )?;
        }
        Ok(())
    }
    fn resume_compaction(&mut self, exact: &Liability) -> Result<()> {
        ensure(exact.retirement.is_some(), "compaction original identity")?;
        self.authorized_hold = Some(exact.token);
        let untouched = self.data.pack == exact.origin.data_pack
            && self.data.end == exact.origin.data_end
            && self.meta.pack == exact.origin.meta_pack
            && self.meta.end == exact.origin.meta_end;
        if untouched && !self.dir.join("intent").exists() {
            let (data, pack) = exact.retirement.unwrap();
            if data {
                self.compact_data(pack, Cut::None, false)?;
            } else {
                self.compact_meta(pack, Cut::None, false)?;
            }
        } else {
            self.continue_original(exact, Cut::None)?;
        }
        let consumed = exact
            .by_domain
            .iter()
            .map(|(a, b)| (*a, if *a == 0 { b - exact.control } else { *b }))
            .collect::<Vec<_>>();
        self.complete_liability(exact, &consumed)
    }
}

fn setup(n: u64, kind: Kind) -> Result<(tempfile::TempDir, Store, Fixture)> {
    let temp = tempfile::tempdir().map_err(error)?;
    let sd = temp.path().join("source");
    fs::create_dir(&sd).map_err(error)?;
    let mut source = Store::open(&sd)?;
    build(&mut source, n, kind)?;
    Session::imported().audit(&mut source)?;
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    f.initialize_gate()?;
    Ok((temp, source, f))
}
fn combined_run(n: u64, kind: Kind) -> Result<serde_json::Value> {
    let t = Instant::now();
    let (_temp, mut source, mut f) = setup(n, kind)?;
    let bootstrap = t.elapsed().as_millis();
    let mut samples = Vec::new();
    for batch in [1, 8, 32] {
        for _ in 0..4 {
            let end = source.size();
            logical_group(&mut source, batch)?;
            f.c = Counters::default();
            let before = f.snapshot()?;
            let t = Instant::now();
            let result = f.checkpoint(&mut source, end, Fault::None)?;
            samples.push(json!({"batch":batch,"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":result}));
        }
    }
    let t = Instant::now();
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    reopened.structural_probe()?;
    let open = json!({"us":t.elapsed().as_micros(),"counters":reopened.c,"gate_head_bytes":fs::metadata(f.dir.join("HEAD")).map_err(error)?.len()});
    reopened.c = Counters::default();
    let t = Instant::now();
    reopened.retry(&id(1), &request(&id(1)))?;
    let retry = json!({"us":t.elapsed().as_micros(),"counters":reopened.c});
    for ordinal in n + 1..=n + 164 {
        let got = reopened.retry(&id(ordinal), &request(&id(ordinal)))?;
        ensure(
            matches!(source.get(&got)?,Node::Receipt{ordinal:o,..} if o==ordinal),
            "original receipt/ordinal continuity",
        )?;
    }
    // One selected pin from every non-native class; no lifetime pin enumeration.
    let mut pins = Vec::new();
    for class in EXTRA_CLASSES {
        pins.push(f.acquire(class, class == PinClass::UnresolvedIntent)?);
    }
    let candidate = 0;
    f.c = Counters::default();
    let t = Instant::now();
    ensure(
        f.gate_candidate(candidate, true).is_err(),
        "pinned data candidate admitted",
    )?;
    let refusal = json!({"us":t.elapsed().as_micros(),"counters":f.c,"pins":pins.len()});
    for pin in &pins {
        f.release(pin, true, true, true)?;
    }
    f.c = Counters::default();
    let before = f.snapshot()?;
    let t = Instant::now();
    let compact = f.compact_combined(true, candidate, Fault::None)?;
    let compaction = json!({"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":compact});
    f.c = Counters::default();
    let before = f.snapshot()?;
    let t = Instant::now();
    let compact = f.compact_combined(false, 0, Fault::None)?;
    let metadata_compaction = json!({"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":compact});
    let t = Instant::now();
    let audit = f.audit_combined()?;
    Session::imported().audit(&mut source)?;
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"bootstrap_ms":bootstrap,"samples":samples,"structural_open":open,"oldest_retry":retry,"all_class_candidate_refusal":refusal,"data_compaction":compaction,"metadata_compaction":metadata_compaction,"audit_ms":t.elapsed().as_millis(),"audited_objects":audit,"final_snapshot":f.snapshot()?,"capacity":f.head.gate}),
    )
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    if args.first().is_some_and(|s| s == "typed-inventory") {
        return typed_inventory::run_cli(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "ownership-probe") {
        return ownership_probe::run_cli(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "turnover") {
        return turnover_cli(&args[1..]);
    }
    println!(
        "{}",
        json!({"fixture":"combined-placement-liveness-capacity-v3","pack_bytes":PACK,"max_concurrent_pins":MAX_PINS,"max_concurrent_liabilities":MAX_HOLDS,"head_limit":GATE_MAX,"caveats":["fixture only; no production wire or Accepted/Saved guarantee","HEAD-selected finite bounded concurrent gate state, not a lifetime flat index","all-class snapshot pin lookup is candidate-local; concurrency cap64 and two locator paths per pin","capacity is conservative modeled admission, not exclusive APFS space or actual quota guarantee","no deletion credit; charged high-water remains after physical retirement","source semantic builder I/O excluded; all original receipts and ordinals materialized","cache cold is not disk cold; full audit is separately exhaustive","pending cut recovery may use exhaustive audit; ordinary structural reopen does not","no arbitrary mid-record/power-loss recovery or all-owner lease protocol"]})
    );
    for n in if args.is_empty() {
        vec![1000]
    } else {
        args.iter()
            .map(|s| s.parse().map_err(error))
            .collect::<Result<Vec<u64>>>()?
    } {
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", combined_run(n, kind)?);
        }
    }
    Ok(())
}
fn turnover_cli(args: &[String]) -> Result<()> {
    println!(
        "{}",
        json!({"fixture":"combined-v3 unchanged-root turnover supplement","samples":8,"caveats":["fully materialized baselines; bootstrap excluded","forced physical-only gate/HEAD selection with unchanged semantic and locator roots","no-op ordinary Save need not publish; this is turnover cost only","size-derived control admission included; no payload/index rewrite","fsync, not qualified power-loss or Saved guarantee; OS caches retained"]})
    );
    for n in args
        .iter()
        .map(|s| s.parse().map_err(error))
        .collect::<Result<Vec<u64>>>()?
    {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, _source, mut f) = setup(n, kind)?;
            let h = f.head.clone();
            let mut samples = Vec::new();
            for _ in 0..8 {
                f.c = Counters::default();
                let before = f.snapshot()?;
                let t = Instant::now();
                f.select_gate(f.head.gate.clone())?;
                let us = t.elapsed().as_micros();
                ensure(
                    f.head.semantic == h.semantic
                        && f.head.active == h.active
                        && f.head.recovery == h.recovery,
                    "turnover mutated authored or locator roots",
                )?;
                samples.push(json!({"us":us,"counters":f.c,"before":before,"after":f.snapshot()?}));
            }
            let mut reopened = Fixture::reopen_combined(&f.dir)?;
            reopened.structural_probe()?;
            println!(
                "{}",
                json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"samples":samples,"post_turnover_structural_read":reopened.c})
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod combined_tests {
    use super::*;
    #[test]
    fn all_class_economic_compaction_refuses_growth_and_reclaims_garbage() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut source, mut f) = setup(2000, kind)?;
            for garbage in [true, false] {
                f.data.rotate(&mut f.c)?;
                let candidate = f.data.pack;
                let old = f.head.clone();
                let mut updates = Vec::new();
                for ordinal in 1..=2000 {
                    if garbage && ordinal > 16 {
                        break;
                    }
                    let r = f.retry(&id(ordinal), &request(&id(ordinal)))?;
                    if f.data.end + r.len + 12 > PACK {
                        break;
                    }
                    let loc = f.copy_object(&mut source, &r)?;
                    updates.push((r.offset, Some(loc)));
                }
                if garbage {
                    let bytes = vec![0xA5; 1024];
                    let mut key = u64::MAX;
                    while f.data.end + 1036 <= PACK {
                        f.data.append(&bytes, Some(key), &mut f.c)?;
                        key -= 1;
                    }
                }
                f.data.rotate(&mut f.c)?;
                updates.sort_by_key(|(key, _)| *key);
                let active = f
                    .multi(Some(old.active), &updates)?
                    .ok_or("density current root")?;
                f.publish(
                    active.clone(),
                    old.recovery,
                    old.semantic.clone(),
                    Cut::None,
                )?;
                f.publish(active.clone(), active, old.semantic, Cut::None)?;
                // Fixture input construction is outside admission measurement;
                // charge its observed allocation once, without deletion credit.
                let mut gate = f.head.gate.clone();
                let observed = f.snapshot()?.1;
                gate.domains.get_mut(&0).unwrap().charged = gate.domains[&0].charged.max(observed);
                f.select_gate(gate)?;
                let mut pins = Vec::new();
                for class in EXTRA_CLASSES {
                    pins.push(f.acquire(class, class == PinClass::UnresolvedIntent)?);
                }
                let before = f.snapshot()?;
                assert!(f.compact_economic(true, candidate).is_err());
                assert_eq!(before, f.snapshot()?);
                for pin in pins {
                    f.release(&pin, true, true, true)?;
                }
                let before = f.snapshot()?;
                let selected = f.head.clone();
                f.c = Counters::default();
                let result = f.compact_economic(true, candidate)?;
                println!(
                    "{}",
                    json!({"fixture":"combined-all-class-profitability","kind":format!("{kind:?}"),"n":2000,"case":if garbage{"high-garbage"}else{"all-live"},"all_extra_pin_classes":EXTRA_CLASSES.len(),"before":before,"after":f.snapshot()?,"counters":f.c,"result":result})
                );
                if garbage {
                    assert_eq!(result["deferred"], false);
                    assert!(f.snapshot()?.0 < before.0);
                    assert!(!f.data.path(candidate).exists());
                } else {
                    assert_eq!(result["deferred"], true);
                    assert_eq!(before, f.snapshot()?);
                    assert_eq!(selected, f.head);
                    assert_eq!(
                        f.c.data_write_bytes + f.c.meta_write_bytes + f.c.envelope_write_bytes,
                        0
                    );
                    assert!(f.head.gate.holds.is_empty());
                }
                f.audit_combined()?;
            }
        }
        Ok(())
    }
    #[test]
    fn every_class_exact_release_and_reader_aba() -> Result<()> {
        let (_temp, mut source, mut f) = setup(256, Kind::Radix)?;
        f.data.rotate(&mut f.c)?;
        f.meta.rotate(&mut f.c)?;
        for class in EXTRA_CLASSES {
            let pin = f.acquire(class, class == PinClass::UnresolvedIntent)?;
            let mut reopened = Fixture::reopen_combined(&f.dir)?;
            let before = reopened.snapshot()?;
            assert!(reopened.compact_combined(true, 0, Fault::None).is_err());
            assert!(reopened.compact_combined(false, 0, Fault::None).is_err());
            assert_eq!(before, reopened.snapshot()?);
            let mut wrong = pin.clone();
            wrong.epoch += 1;
            assert!(f.release(&wrong, true, true, true).is_err());
            if class == PinClass::ReaderLease {
                assert!(f.release(&pin, true, false, true).is_err());
            }
            if pin.unknown {
                assert!(f.release(&pin, true, true, false).is_err());
            }
            f.release(&pin, true, true, true)?;
            let next = f.acquire(class, false)?;
            assert!(f.release(&pin, true, true, true).is_err());
            assert!(next.token > pin.token);
            f.release(&next, true, true, true)?;
        }
        let old = f.head.semantic.clone();
        f.compact_combined(true, 0, Fault::None)?;
        f.compact_combined(false, 0, Fault::None)?;
        assert_eq!(old, f.head.semantic);
        f.audit()?;
        Session::imported().audit(&mut source)?;
        Ok(())
    }
    #[test]
    fn enospc_original_group_reconciles_and_unknown_head_refuses() -> Result<()> {
        for fault in [
            Fault::EnospcBefore,
            Fault::EnospcBeforeHead,
            Fault::EnospcAfterHead,
        ] {
            let (_temp, mut source, mut f) = setup(64, Kind::Btree)?;
            let end = source.size();
            logical_group(&mut source, 8)?;
            assert!(f.checkpoint(&mut source, end, fault).is_err());
            let hold = f.head.gate.holds.values().next().unwrap().clone();
            let mut reopened = Fixture::reopen_combined(&f.dir)?;
            assert!(reopened.complete_liability(&hold, &[(0, 0)]).is_err());
            if reopened.dir.join("intent").exists() {
                let original = fs::read(reopened.dir.join("HEAD")).map_err(error)?;
                let mut unknown = reopened.head.clone();
                unknown.epoch += 17;
                fs::write(
                    reopened.dir.join("HEAD"),
                    serde_json::to_vec(&unknown).map_err(error)?,
                )
                .map_err(error)?;
                assert!(Fixture::reopen_combined(&reopened.dir).is_err());
                assert!(reopened.reconcile().is_err());
                assert!(reopened.dir.join("intent").exists());
                fs::write(reopened.dir.join("HEAD"), original).map_err(error)?;
            }
            reopened.resume_checkpoint(&mut source, end, &hold)?;
            assert!(reopened.head.gate.holds.is_empty());
            for ordinal in 65..=72 {
                let r = reopened.retry(&id(ordinal), &request(&id(ordinal)))?;
                assert!(matches!(source.get(&r)?,Node::Receipt{ordinal:o,..} if o==ordinal));
            }
            let unchanged = reopened.head.clone();
            assert!(reopened.resume_checkpoint(&mut source, end, &hold).is_err());
            assert_eq!(unchanged, reopened.head);
            reopened.audit()?;
        }
        Ok(())
    }
    #[test]
    fn compaction_first_selection_retains_recovery_and_liability() -> Result<()> {
        for fault in [
            Fault::EnospcBefore,
            Fault::EnospcBeforeHead,
            Fault::EnospcAfterHead,
        ] {
            let (_temp, _source, mut f) = setup(256, Kind::Radix)?;
            f.data.rotate(&mut f.c)?;
            assert!(f.compact_combined(true, 0, fault).is_err());
            let hold = f.head.gate.holds.values().next().unwrap().clone();
            let mut reopened = Fixture::reopen_combined(&f.dir)?;
            assert!(reopened.data.path(0).exists());
            assert!(reopened.complete_liability(&hold, &[(0, 0)]).is_err());
            if reopened.dir.join("intent").exists() {
                reopened.reconcile()?;
            }
            let h = reopened.head.clone();
            reopened.audit_root(&h.recovery, &h.semantic.recovery)?;
            reopened.resume_compaction(&hold)?;
            assert!(!reopened.data.path(0).exists());
            reopened.audit()?;
        }
        Ok(())
    }
    #[test]
    fn shared_domain_reserve_atomic_and_no_deletion_credit() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let before = f.head.clone();
        let charged = before.gate.domains[&0].charged;
        let mut gate = f.head.gate.clone();
        gate.domains.get_mut(&0).unwrap().limit = charged + CONTROL * 2;
        f.select_gate(gate)?;
        let before = f.head.clone();
        let snapshot = f.snapshot()?;
        assert!(
            f.reserve(f.head.semantic.clone(), None, &[(0, CONTROL), (0, CONTROL)])
                .is_err()
        );
        assert_eq!(f.head, before);
        assert_eq!(snapshot, f.snapshot()?);
        assert!(
            f.reserve(f.head.semantic.clone(), None, &[(99, 1)])
                .is_err()
        );
        assert_eq!(f.head, before);
        Ok(())
    }
}
