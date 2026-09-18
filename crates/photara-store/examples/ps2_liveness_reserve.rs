//! Disposable conservative liveness/reserve model. No production wire or GC.
//! Run: `cargo run --release -p photara-store --example ps2_liveness_reserve`
//! Test: `cargo test -p photara-store --example ps2_liveness_reserve`
//! WAL fsync/replay exercises process-visible persistence, not APFS qualification.
//! Paged checkpoint reopening reads a bounded selector and pending transition;
//! the separate full WAL replay is an audit oracle only. No physical pack deletion.

#![allow(
    dead_code,
    reason = "Disposable model operations are exercised by the example tests"
)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Instant;

type Result<T> = std::result::Result<T, String>;
const MAX_EDGES: usize = 16;
const MAX_DRAIN: usize = 32;
const MAX_FRAME: usize = 65_536;

fn ensure(value: bool, message: &str) -> Result<()> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or_else(|| "overflow".into())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Pin {
    class: PinClass,
    object: u64,
    epoch: u64,
    // Unknown outcomes may only be released by exact original-ID reconciliation.
    unknown: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Object {
    edges: Vec<u64>,
    incoming: u64,
    allocated: u64,
    domain: u64,
    retired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Delta {
    sequence: u64,
    operation: u64,
    // Persist touched postimages together: a torn frame never applies half a count.
    objects: BTreeMap<u64, Object>,
    pins: BTreeMap<u64, Option<Pin>>,
    queue_pop: usize,
    queue_push: Vec<u64>,
    accepted_operations: Vec<(u64, String)>,
}

#[derive(Default)]
struct Live {
    objects: BTreeMap<u64, Object>,
    pins: BTreeMap<u64, Pin>,
    queue: VecDeque<u64>,
    // Audit oracle only. Normal disk reopening uses Paged's point-addressed map.
    applied: BTreeMap<u64, Vec<u8>>,
    sequence: u64,
    touched: u64,
}

impl Live {
    fn delta(&self, operation: u64) -> Result<Delta> {
        Ok(Delta {
            sequence: add(self.sequence, 1)?,
            operation,
            objects: BTreeMap::new(),
            pins: BTreeMap::new(),
            queue_pop: 0,
            queue_push: Vec::new(),
            accepted_operations: Vec::new(),
        })
    }

    fn object(&self, id: u64) -> Result<&Object> {
        let object = self.objects.get(&id).ok_or("missing object")?;
        ensure(!object.retired, "retired object cannot acquire a reference")?;
        Ok(object)
    }

    fn change_count(&self, delta: &mut Delta, id: u64, increment: bool) -> Result<()> {
        let object = delta.objects.entry(id).or_insert(self.object(id)?.clone());
        object.incoming = if increment {
            add(object.incoming, 1)?
        } else {
            object.incoming.checked_sub(1).ok_or("undercount")?
        };
        if object.incoming == 0 {
            delta.queue_push.push(id);
        }
        Ok(())
    }

    fn create(&self, operation: u64, id: u64, edges: Vec<u64>, domain: u64) -> Result<Delta> {
        ensure(!self.objects.contains_key(&id), "object identity collision")?;
        ensure(edges.len() <= MAX_EDGES, "edge admission limit")?;
        ensure(
            edges.iter().copied().collect::<BTreeSet<_>>().len() == edges.len(),
            "duplicate edge",
        )?;
        // Existing-only dependencies make this fixture graph acyclic.
        let mut delta = self.delta(operation)?;
        for child in &edges {
            self.change_count(&mut delta, *child, true)?;
        }
        delta.objects.insert(
            id,
            Object {
                edges,
                incoming: 0,
                allocated: 4096,
                domain,
                retired: false,
            },
        );
        // Unselected new objects are conservative queue debt until explicitly drained.
        delta.queue_push.push(id);
        Ok(delta)
    }

    fn pin(&self, operation: u64, token: u64, pin: Pin) -> Result<Delta> {
        ensure(!self.pins.contains_key(&token), "pin identity collision")?;
        let mut delta = self.delta(operation)?;
        self.change_count(&mut delta, pin.object, true)?;
        delta.pins.insert(token, Some(pin));
        Ok(delta)
    }

    fn release(
        &self,
        operation: u64,
        token: u64,
        epoch: u64,
        proof: ReleaseProof,
    ) -> Result<Delta> {
        let pin = self.pins.get(&token).ok_or("missing pin")?;
        ensure(pin.epoch == epoch, "stale epoch")?;
        ensure(proof.original_token == token, "wrong release identity")?;
        ensure(proof.completed, "release before durable completion")?;
        ensure(
            !pin.unknown || proof.reconciled,
            "unknown outcome retains pin",
        )?;
        ensure(
            pin.class != PinClass::ReaderLease || proof.reader_finished,
            "reader remains active",
        )?;
        let mut delta = self.delta(operation)?;
        self.change_count(&mut delta, pin.object, false)?;
        delta.pins.insert(token, None);
        Ok(delta)
    }

    // One bounded transaction per item. A large cascade becomes charged queue debt.
    fn drain_one(&self, operation: u64) -> Result<Delta> {
        let mut delta = self.delta(operation)?;
        let Some(id) = self.queue.front() else {
            return Ok(delta);
        };
        delta.queue_pop = 1;
        let object = self
            .objects
            .get(id)
            .ok_or("queue references unknown object")?;
        if object.incoming == 0 && !object.retired {
            let mut retired = object.clone();
            retired.retired = true;
            delta.objects.insert(*id, retired);
            for child in &object.edges {
                self.change_count(&mut delta, *child, false)?;
            }
        }
        Ok(delta)
    }

    fn apply(&mut self, delta: &Delta) -> Result<bool> {
        let encoded = serde_json::to_vec(delta).map_err(|e| e.to_string())?;
        if let Some(original) = self.applied.get(&delta.operation) {
            ensure(original == &encoded, "operation replay differs")?;
            return Ok(false);
        }
        ensure(
            delta.sequence == add(self.sequence, 1)?,
            "noncontiguous ledger",
        )?;
        ensure(delta.queue_pop <= self.queue.len(), "queue underflow")?;
        self.touched = add(
            self.touched,
            u64::try_from(delta.objects.len() + delta.pins.len() + delta.queue_pop).unwrap(),
        )?;
        self.objects.extend(delta.objects.clone());
        for (id, pin) in &delta.pins {
            if let Some(pin) = pin {
                self.pins.insert(*id, pin.clone());
            } else {
                self.pins.remove(id);
            }
        }
        for _ in 0..delta.queue_pop {
            self.queue.pop_front();
        }
        self.queue.extend(&delta.queue_push);
        self.sequence = delta.sequence;
        self.applied.insert(delta.operation, encoded);
        Ok(true)
    }

    // Explicit audit only. Foreground mutation and candidate lookup never call it.
    fn audit(&self) -> Result<()> {
        let mut actual = BTreeMap::<u64, u64>::new();
        for object in self.objects.values().filter(|o| !o.retired) {
            for child in &object.edges {
                *actual.entry(*child).or_default() += 1;
            }
        }
        for pin in self.pins.values() {
            self.object(pin.object)?;
            *actual.entry(pin.object).or_default() += 1;
        }
        for (id, object) in &self.objects {
            ensure(
                object.incoming == actual.get(id).copied().unwrap_or(0),
                "count audit mismatch",
            )?;
            ensure(
                !object.retired || object.incoming == 0,
                "retired live object",
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ReleaseProof {
    original_token: u64,
    completed: bool,
    reconciled: bool,
    reader_finished: bool,
}

#[derive(Clone, Copy)]
enum Cut {
    None,
    Before,
    Torn,
    After,
}

struct Journal {
    file: File,
}
impl Journal {
    fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        Ok(Self { file })
    }

    fn persist(&mut self, live: &mut Live, delta: &Delta, cut: Cut) -> Result<()> {
        if matches!(cut, Cut::Before) {
            return Err("injected before-effect".into());
        }
        let payload = serde_json::to_vec(delta).map_err(|e| e.to_string())?;
        ensure(payload.len() <= MAX_FRAME, "frame admission limit")?;
        let mut bytes = Vec::with_capacity(payload.len() + 40);
        bytes.extend(u64::try_from(payload.len()).unwrap().to_le_bytes());
        bytes.extend(Sha256::digest(&payload));
        bytes.extend(payload);
        if matches!(cut, Cut::Torn) {
            self.file
                .write_all(&bytes[..bytes.len() / 2])
                .map_err(|e| e.to_string())?;
            self.file.sync_all().map_err(|e| e.to_string())?;
            return Err("injected torn frame: reopen read-only".into());
        }
        self.file.write_all(&bytes).map_err(|e| e.to_string())?;
        self.file.sync_all().map_err(|e| e.to_string())?;
        if matches!(cut, Cut::After) {
            return Err("injected unknown completion".into());
        }
        live.apply(delta)?;
        Ok(())
    }

    fn replay(path: &Path) -> Result<Live> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|e| e.to_string())?
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let mut live = Live::default();
        let mut cursor = 0;
        while cursor < bytes.len() {
            ensure(
                bytes.len() - cursor >= 40,
                "incomplete frame: refuse writable replay",
            )?;
            let length = usize::try_from(u64::from_le_bytes(
                bytes[cursor..cursor + 8].try_into().unwrap(),
            ))
            .map_err(|e| e.to_string())?;
            ensure(
                length <= MAX_FRAME && length <= bytes.len() - cursor - 40,
                "incomplete/oversized frame",
            )?;
            let payload = &bytes[cursor + 40..cursor + 40 + length];
            ensure(
                Sha256::digest(payload).as_slice() == &bytes[cursor + 8..cursor + 40],
                "frame digest mismatch",
            )?;
            let delta = serde_json::from_slice(payload).map_err(|e| e.to_string())?;
            live.apply(&delta)?;
            cursor += length + 40;
        }
        live.audit()?;
        Ok(live)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
enum Category {
    JournalFrames,
    OriginalReceipts,
    DataPack,
    MetadataPack,
    IndexPaths,
    InventoryPaths,
    LocatorBootstrap,
    Namespace,
    Head,
    Intent,
    SavedReceipt,
    CheckpointLiability,
    CompactionCopy,
    CompactionMetadata,
    SafetyMargin,
    FilePreallocation,
    LivenessMetadata,
    ReserveLedger,
    BarrierEvidence,
    PackRolloverSlack,
}

// A separate disk-backed incremental checkpoint. Fixed-depth, copy-on-write radix
// pages persist touched count/pin/queue postimages. Reopen reads one bounded HEAD
// and at most one pending selection; it does not hydrate the lifetime object map.
// Append-only pages are conservative physical debt: reclamation remains unproved.
const PAGE: usize = 4096;
const DEPTH: usize = 18; // one namespace byte plus one u64 identity
const STAGED: u64 = 1 << 63;

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Route {
    namespace: u8,
    id: u64,
    depth: u8,
}

#[derive(Default, Serialize, Deserialize)]
struct Page {
    route: Route,
    children: BTreeMap<u8, u64>,
    value: Option<serde_json::Value>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Cursor {
    root: Option<u64>,
    sequence: u64,
    queue_head: u64,
    queue_tail: u64,
}
#[derive(Serialize, Deserialize)]
struct Pending {
    before: Cursor,
    after: Cursor,
}
struct Paged {
    file: File,
    directory: std::path::PathBuf,
    cursor: Cursor,
    reads: u64,
    writes: u64,
    frozen: bool,
    staged: BTreeMap<u64, Page>,
    next_staged: u64,
    pack_writes: u64,
}

impl Paged {
    fn new(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(directory.join("pages"))
            .map_err(|e| e.to_string())?;
        let cursor = Cursor {
            root: None,
            sequence: 0,
            queue_head: 0,
            queue_tail: 0,
        };
        let mut this = Self {
            file,
            directory: directory.into(),
            cursor,
            reads: 0,
            writes: 0,
            frozen: false,
            staged: BTreeMap::new(),
            next_staged: STAGED,
            pack_writes: 0,
        };
        this.select()?;
        Ok(this)
    }

    fn small_json(path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        ensure(
            file.metadata().map_err(|e| e.to_string())?.len() <= u64::try_from(MAX_FRAME).unwrap(),
            "unbounded bootstrap refused",
        )?;
        let mut bytes = Vec::new();
        file.take(u64::try_from(MAX_FRAME + 1).unwrap())
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        ensure(bytes.len() <= MAX_FRAME, "bootstrap grew while reading")?;
        Ok(bytes)
    }

    fn reopen(directory: &Path) -> Result<Self> {
        let cursor: Cursor =
            serde_json::from_slice(&Self::small_json(&directory.join("HEAD.fixture"))?)
                .map_err(|e| e.to_string())?;
        let mut reads = 1;
        let pending_path = directory.join("pending.fixture");
        let frozen = if pending_path.exists() {
            reads += 1;
            let pending: Pending = serde_json::from_slice(&Self::small_json(&pending_path)?)
                .map_err(|e| e.to_string())?;
            ensure(
                cursor == pending.before || cursor == pending.after,
                "unrelated selector: freeze",
            )?;
            // Exact original transition remains unresolved; never infer release.
            true
        } else {
            false
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(directory.join("pages"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            file,
            directory: directory.into(),
            cursor,
            reads,
            writes: 0,
            frozen,
            staged: BTreeMap::new(),
            next_staged: STAGED,
            pack_writes: 0,
        })
    }

    fn write_small(&self, name: &str, value: &impl Serialize) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        ensure(bytes.len() <= MAX_FRAME, "bounded selector limit")?;
        let mut file = File::create(self.directory.join(name)).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    }

    fn select(&mut self) -> Result<()> {
        self.write_small("HEAD.next", &self.cursor)?;
        std::fs::rename(
            self.directory.join("HEAD.next"),
            self.directory.join("HEAD.fixture"),
        )
        .map_err(|e| e.to_string())?;
        File::open(&self.directory)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())
    }

    fn read_page(&mut self, offset: u64) -> Result<Page> {
        ensure(offset < STAGED, "invalid persistent page offset")?;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0; 36];
        self.file
            .read_exact(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.reads += 1;
        let length = usize::try_from(u32::from_le_bytes(bytes[..4].try_into().unwrap())).unwrap();
        ensure(length <= PAGE - 36, "page length refused")?;
        let mut payload = vec![0; length];
        self.file
            .read_exact(&mut payload)
            .map_err(|e| e.to_string())?;
        ensure(
            Sha256::digest(&payload).as_slice() == &bytes[4..36],
            "page digest mismatch",
        )?;
        serde_json::from_slice(&payload).map_err(|e| e.to_string())
    }

    fn pack_page(&mut self, reference: u64, base: u64, packed: &mut Vec<u8>) -> Result<u64> {
        if reference < STAGED {
            return Ok(reference);
        }
        let mut page = self
            .staged
            .remove(&reference)
            .ok_or("missing staged page")?;
        for child in page.children.values_mut() {
            *child = self.pack_page(*child, base, packed)?;
        }
        let payload = serde_json::to_vec(&page).map_err(|e| e.to_string())?;
        ensure(payload.len() <= PAGE - 36, "page limit refused")?;
        let offset = add(base, u64::try_from(packed.len()).unwrap())?;
        ensure(offset < STAGED, "pack offset limit")?;
        packed.extend(u32::try_from(payload.len()).unwrap().to_le_bytes());
        packed.extend(Sha256::digest(&payload));
        packed.extend(payload);
        self.writes += 1;
        Ok(offset)
    }

    fn nibble(namespace: u8, id: u64, depth: usize) -> u8 {
        let key = (u128::from(namespace) << 64) | u128::from(id);
        u8::try_from((key >> ((DEPTH - depth - 1) * 4)) & 15).unwrap()
    }

    fn put(
        &mut self,
        root: Option<u64>,
        namespace: u8,
        id: u64,
        depth: usize,
        value: &serde_json::Value,
    ) -> Result<u64> {
        let reference = if root.is_some_and(|reference| reference >= STAGED) {
            root.unwrap()
        } else {
            let reference = self.next_staged;
            self.next_staged = add(reference, 1)?;
            reference
        };
        let mut page = if let Some(offset) = root {
            if offset >= STAGED {
                self.staged
                    .remove(&offset)
                    .ok_or("missing coalesced path")?
            } else {
                self.read_page(offset)?
            }
        } else {
            Page::default()
        };
        if depth == DEPTH {
            page.value = Some(value.clone());
        } else {
            let nibble = Self::nibble(namespace, id, depth);
            let child = self.put(
                page.children.get(&nibble).copied(),
                namespace,
                id,
                depth + 1,
                value,
            )?;
            page.children.insert(nibble, child);
        }
        page.route = Route {
            namespace,
            id,
            depth: u8::try_from(depth).unwrap(),
        };
        self.staged.insert(reference, page);
        Ok(reference)
    }

    fn get(&mut self, namespace: u8, id: u64) -> Result<Option<serde_json::Value>> {
        let mut selected = self.cursor.root;
        for depth in 0..=DEPTH {
            let Some(offset) = selected else {
                return Ok(None);
            };
            let page = self.read_page(offset)?;
            if depth == DEPTH {
                return Ok(page.value);
            }
            selected = page
                .children
                .get(&Self::nibble(namespace, id, depth))
                .copied();
        }
        unreachable!()
    }

    fn checkpoint(&mut self, delta: &Delta, cut: Cut) -> Result<()> {
        ensure(!self.frozen, "unresolved selection freezes admission")?;
        let digest = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(delta).map_err(|e| e.to_string())?)
        ));
        if let Some(original) = self.get(4, delta.operation)? {
            ensure(original == digest, "original ledger operation differs")?;
            return Ok(());
        }
        ensure(
            delta.sequence == add(self.cursor.sequence, 1)?,
            "checkpoint sequence mismatch",
        )?;
        let before = self.cursor.clone();
        let mut after = before.clone();
        ensure(
            self.staged.is_empty(),
            "unfinished staging freezes admission",
        )?;
        for (id, object) in &delta.objects {
            after.root = Some(self.put(
                after.root,
                1,
                *id,
                0,
                &serde_json::to_value(object).map_err(|e| e.to_string())?,
            )?);
        }
        for (id, pin) in &delta.pins {
            after.root = Some(self.put(
                after.root,
                2,
                *id,
                0,
                &serde_json::to_value(pin).map_err(|e| e.to_string())?,
            )?);
        }
        for id in &delta.queue_push {
            after.root = Some(self.put(after.root, 3, after.queue_tail, 0, &json!(id))?);
            after.queue_tail = add(after.queue_tail, 1)?;
        }
        for (id, digest) in &delta.accepted_operations {
            after.root = Some(self.put(after.root, 5, *id, 0, &json!(digest))?);
        }
        after.queue_head = add(after.queue_head, u64::try_from(delta.queue_pop).unwrap())?;
        ensure(after.queue_head <= after.queue_tail, "queue underflow")?;
        // Original operation evidence survives root turnover in its own map.
        after.root = Some(self.put(after.root, 4, delta.operation, 0, &digest)?);
        after.sequence = delta.sequence;
        let base = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|e| e.to_string())?;
        let mut packed = Vec::new();
        after.root = Some(self.pack_page(after.root.unwrap(), base, &mut packed)?);
        // One append for all coalesced pages; physical allocation rounds the pack,
        // not each tiny page. Admission must reserve its worst-case page bound.
        ensure(
            packed.len() <= 4 * 1024 * 1024,
            "bounded metadata transaction",
        )?;
        self.file.write_all(&packed).map_err(|e| e.to_string())?;
        self.pack_writes += 1;
        self.file.sync_all().map_err(|e| e.to_string())?;
        self.write_small(
            "pending.fixture",
            &Pending {
                before,
                after: after.clone(),
            },
        )?;
        File::open(&self.directory)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        self.frozen = true;
        if matches!(cut, Cut::Before | Cut::Torn) {
            return Err("pending old selection".into());
        }
        self.cursor = after;
        self.select()?;
        if matches!(cut, Cut::After) {
            return Err("pending candidate selection".into());
        }
        // Removal affects only this fixture's exact completed pending marker.
        std::fs::remove_file(self.directory.join("pending.fixture")).map_err(|e| e.to_string())?;
        File::open(&self.directory)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        self.frozen = false;
        Ok(())
    }

    fn load_object(&mut self, live: &mut Live, id: u64) -> Result<()> {
        if let Some(value) = self.get(1, id)? {
            live.objects.insert(
                id,
                serde_json::from_value(value).map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    // Bounded normal mutation path: materialize only the requested object and
    // its <=16 direct dependencies, never a historical Live/applied map.
    fn create_object(&mut self, operation: u64, id: u64, edges: Vec<u64>) -> Result<()> {
        ensure(edges.len() <= MAX_EDGES, "edge admission limit")?;
        let mut local = Live {
            sequence: self.cursor.sequence,
            ..Live::default()
        };
        self.load_object(&mut local, id)?;
        for child in &edges {
            self.load_object(&mut local, *child)?;
        }
        let delta = local.create(operation, id, edges, 1)?;
        self.checkpoint(&delta, Cut::None)
    }

    fn drain_bounded(&mut self, steps: usize) -> Result<()> {
        ensure(steps <= MAX_DRAIN, "drain budget exceeded")?;
        let mut local = Live {
            sequence: self.cursor.sequence,
            ..Live::default()
        };
        let available = usize::try_from(
            (self.cursor.queue_tail - self.cursor.queue_head).min(u64::try_from(steps).unwrap()),
        )
        .unwrap();
        for offset in 0..available {
            let value = self
                .get(3, self.cursor.queue_head + u64::try_from(offset).unwrap())?
                .ok_or("missing queue entry")?;
            local
                .queue
                .push_back(value.as_u64().ok_or("invalid queue entry")?);
        }
        if available == 0 {
            return Ok(());
        }
        let mut combined = local.delta(add(self.cursor.sequence, 1)?)?;
        let mut processed = 0;
        for _ in 0..steps {
            let Some(id) = local.queue.front().copied() else {
                break;
            };
            if !local.objects.contains_key(&id) {
                self.load_object(&mut local, id)?;
            }
            let children = local
                .object(id)
                .map(|object| object.edges.clone())
                .unwrap_or_default();
            for child in children {
                if !local.objects.contains_key(&child) {
                    self.load_object(&mut local, child)?;
                }
            }
            let delta = local.drain_one(add(local.sequence, 1)?)?;
            combined.objects.extend(delta.objects.clone());
            local.apply(&delta)?;
            processed += 1;
        }
        combined.queue_pop = processed.min(available);
        combined.queue_push = local
            .queue
            .iter()
            .skip(available - combined.queue_pop)
            .copied()
            .collect();
        self.checkpoint(&combined, Cut::None)
    }

    fn create_pinned_group(&mut self, first: u64, count: u64) -> Result<()> {
        ensure(count > 0 && count <= 64, "bounded creation group")?;
        let mut local = Live {
            sequence: self.cursor.sequence,
            ..Live::default()
        };
        if first > 1 {
            self.load_object(&mut local, first - 1)?;
        }
        let mut combined = local.delta(add(self.cursor.sequence, 1)?)?;
        for id in first..add(first, count)? {
            ensure(
                self.get(1, id)?.is_none()
                    && self.get(2, id)?.is_none()
                    && self.get(5, id)?.is_none(),
                "group identity collision",
            )?;
            let create = local.create(
                add(local.sequence, 1)?,
                id,
                if id > 1 { vec![id - 1] } else { vec![] },
                1,
            )?;
            local.apply(&create)?;
            let classes = [
                PinClass::Active,
                PinClass::Recovery,
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
            let pin = local.pin(
                add(local.sequence, 1)?,
                id,
                Pin {
                    class: classes[usize::try_from(id % 12).unwrap()],
                    object: id,
                    epoch: 1,
                    unknown: true,
                },
            )?;
            local.apply(&pin)?;
            // Fixture-only original operation commitment; semantic operation index
            // integration is separate. Lookup does not load lifetime IDs.
            combined.accepted_operations.push((
                id,
                format!(
                    "{:x}",
                    Sha256::digest(format!("fixture-create-and-pin:{id}"))
                ),
            ));
        }
        combined.objects = local.objects;
        combined.pins = local
            .pins
            .into_iter()
            .map(|(id, pin)| (id, Some(pin)))
            .collect();
        // Every created object gained a pin in this atomic group; no false zero
        // count became visible and no stale creation queue entry is necessary.
        self.checkpoint(&combined, Cut::None)
    }

    fn release_pinned_group(&mut self, first: u64, count: u64) -> Result<()> {
        ensure(count > 0 && count <= 64, "bounded release group")?;
        let mut local = Live {
            sequence: self.cursor.sequence,
            ..Live::default()
        };
        let mut combined = local.delta(add(self.cursor.sequence, 1)?)?;
        for id in first..add(first, count)? {
            self.load_object(&mut local, id)?;
            let pin: Pin = serde_json::from_value(self.get(2, id)?.ok_or("missing selected pin")?)
                .map_err(|e| e.to_string())?;
            local.pins.insert(id, pin);
            let release = local.release(
                add(local.sequence, 1)?,
                id,
                1,
                ReleaseProof {
                    original_token: id,
                    completed: true,
                    reconciled: true,
                    reader_finished: true,
                },
            )?;
            combined.objects.extend(release.objects.clone());
            combined.pins.extend(release.pins.clone());
            local.apply(&release)?;
        }
        combined.queue_push = local.queue.into();
        self.checkpoint(&combined, Cut::None)
    }

    /// Adapter boundary for `placement_v2::compact_meta`, not a GC implementation.
    /// Each count/pin/queue/operation page carries its namespace/key/depth route.
    /// A compactor may enumerate ONE sealed pack, probe that route in EACH exact
    /// selected root, then rewrite only matching paths into a reserved new pack.
    /// Two physical selections must replace active and recovery references before
    /// checking the candidate again. All reader/lease/unresolved roots must also
    /// leave the candidate; unknown epochs or incomplete root enumeration refuse.
    /// No physical erase may follow this fixture's certificate alone: real pack
    /// identifiers, qualified barriers and shared allocator integration are absent.
    fn candidate_routes(&mut self, start: u64, end: u64) -> Result<Vec<(u64, Route)>> {
        ensure(
            end >= start && end - start <= 256 * 1024,
            "candidate pack budget",
        )?;
        let mut cursor = start;
        let mut records = Vec::new();
        while cursor < end {
            ensure(records.len() < 4096, "candidate record budget")?;
            let page = self.read_page(cursor)?;
            let next = self.file.stream_position().map_err(|e| e.to_string())?;
            ensure(next > cursor && next <= end, "candidate framing mismatch")?;
            ensure(
                usize::from(page.route.depth) <= DEPTH,
                "invalid route depth",
            )?;
            records.push((cursor, page.route));
            cursor = next;
        }
        Ok(records)
    }

    fn route_contains(&mut self, root: Option<u64>, target: u64, route: Route) -> Result<bool> {
        let mut current = root;
        for depth in 0..=usize::from(route.depth) {
            let Some(offset) = current else {
                return Ok(false);
            };
            if offset == target {
                return Ok(true);
            }
            let page = self.read_page(offset)?;
            ensure(
                usize::from(page.route.depth) == depth,
                "route depth mismatch",
            )?;
            if depth == usize::from(route.depth) {
                return Ok(false);
            }
            current = page
                .children
                .get(&Self::nibble(route.namespace, route.id, depth))
                .copied();
        }
        Ok(false)
    }

    fn candidate_is_unreferenced(
        &mut self,
        records: &[(u64, Route)],
        roots: &[Option<u64>],
        all_epochs_resolved: bool,
    ) -> Result<bool> {
        ensure(
            !self.frozen && all_epochs_resolved,
            "unknown selection/lease retains metadata pack",
        )?;
        ensure(
            roots.len() >= 2 && roots.len() <= 32,
            "complete bounded root set required",
        )?;
        for (offset, route) in records {
            for root in roots {
                if self.route_contains(*root, *offset, *route)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Budget(BTreeMap<Category, u64>);
impl Budget {
    fn charge(&mut self, category: Category, units: u64, max_bytes: u64) -> Result<()> {
        let bytes = units.checked_mul(max_bytes).ok_or("overflow")?;
        let rounded = bytes.checked_add(4095).ok_or("overflow")? / 4096 * 4096;
        let previous = self.0.get(&category).copied().unwrap_or(0);
        self.0.insert(category, add(previous, rounded)?);
        Ok(())
    }
    fn total(&self) -> Result<u64> {
        self.0
            .values()
            .try_fold(0, |total, bytes| add(total, *bytes))
    }
    fn worst_case(batch: u64, path_pages: u64, compaction_bytes: u64) -> Result<Self> {
        ensure(
            batch > 0 && batch <= 64 && path_pages <= 256,
            "request bound refused",
        )?;
        let mut budget = Self::default();
        // Synthetic checked maxima, deliberately not measured average file sizes.
        for (category, count, bytes) in [
            (Category::JournalFrames, batch, 65_576),
            (Category::OriginalReceipts, batch, 8192),
            (Category::DataPack, batch, 262_144),
            (Category::MetadataPack, batch, 65_536),
            (Category::IndexPaths, path_pages, 16_384),
            (Category::InventoryPaths, path_pages, 16_384),
            (Category::LocatorBootstrap, path_pages, 4096),
            (
                Category::Namespace,
                batch.checked_add(path_pages).ok_or("overflow")?,
                16_384,
            ),
            (Category::Head, 2, 16_384),
            (Category::Intent, 1, 65_536),
            (Category::SavedReceipt, 1, 8192),
            (Category::CheckpointLiability, batch, 131_072),
            (Category::CompactionCopy, 1, compaction_bytes),
            (Category::CompactionMetadata, path_pages, 16_384),
            (Category::SafetyMargin, 1, 1_048_576),
        ] {
            budget.charge(category, count, bytes)?;
        }
        Ok(budget)
    }

    fn integrated(batch: u64, compaction: bool) -> Result<Self> {
        let selectors = if compaction { 2 } else { 1 };
        let mut budget = Self::worst_case(batch, 256, if compaction { 262_144 } else { 0 })?;
        // Additional explicit categories missing from the original standalone
        // ledger. These are checked synthetic maxima, not APFS allocation bounds.
        budget.charge(Category::LivenessMetadata, 1, 4 * 1024 * 1024)?;
        budget.charge(Category::ReserveLedger, 1, 65_536)?;
        budget.charge(Category::BarrierEvidence, selectors, 65_536)?;
        budget.charge(Category::PackRolloverSlack, selectors * 2, 262_144)?;
        budget.charge(Category::Namespace, selectors * 8, 16_384)?;
        if compaction {
            budget.charge(Category::Head, 2, 16_384)?;
            budget.charge(Category::Intent, 1, 65_536)?;
            budget.charge(Category::SavedReceipt, 1, 8192)?;
        }
        Ok(budget)
    }

    fn sum_categories(&self, categories: &[Category]) -> Result<u64> {
        categories.iter().try_fold(0, |total, category| {
            add(
                total,
                self.0
                    .get(category)
                    .copied()
                    .ok_or("missing charged category")?,
            )
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Domain {
    limit: u64,
    allocated: u64,
    held: u64,
    peak: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reservation {
    by_domain: BTreeMap<u64, u64>,
    preallocated_file_blocks: BTreeMap<u64, u64>,
    accepted: bool,
    unknown: bool,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct Ledger {
    domains: BTreeMap<u64, Domain>,
    reservations: BTreeMap<u64, Reservation>,
}

impl Ledger {
    fn admit(&mut self, operation: u64, requests: &[(u64, Budget)]) -> Result<()> {
        ensure(
            self.domains.len() <= 16 && self.reservations.len() < 64,
            "finite reserve ledger admission cap",
        )?;
        ensure(
            !self.reservations.contains_key(&operation),
            "reserve operation collision",
        )?;
        let mut by_domain = BTreeMap::<u64, u64>::new();
        for (domain, budget) in requests {
            let current = by_domain.get(domain).copied().unwrap_or(0);
            by_domain.insert(*domain, add(current, budget.total()?)?);
        }
        // Validate all domains before mutating any; shared-volume requests sum.
        for (id, bytes) in &by_domain {
            let domain = self.domains.get(id).ok_or("unqualified capacity domain")?;
            ensure(
                add(add(domain.allocated, domain.held)?, *bytes)? <= domain.limit,
                "capacity refusal",
            )?;
        }
        for (id, bytes) in &by_domain {
            let domain = self.domains.get_mut(id).unwrap();
            domain.held += bytes;
            domain.peak = domain.peak.max(domain.allocated + domain.held);
        }
        self.reservations.insert(
            operation,
            Reservation {
                by_domain,
                preallocated_file_blocks: BTreeMap::new(),
                accepted: false,
                unknown: false,
            },
        );
        Ok(())
    }

    fn persist(&self, directory: &Path, cut: Cut) -> Result<()> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        ensure(bytes.len() <= MAX_FRAME, "reserve checkpoint admission cap")?;
        if matches!(cut, Cut::Before | Cut::Torn) {
            return Err("reserve not persisted: do not accept".into());
        }
        let mut file = File::create(directory.join("reserve.next")).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(
            directory.join("reserve.next"),
            directory.join("reserve.fixture"),
        )
        .map_err(|e| e.to_string())?;
        File::open(directory)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        if matches!(cut, Cut::After) {
            return Err("reserve completion unknown: reconcile exact persisted state".into());
        }
        Ok(())
    }

    fn reopen(directory: &Path) -> Result<Self> {
        let bytes = Paged::small_json(&directory.join("reserve.fixture"))?;
        let ledger: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        ensure(
            ledger.domains.len() <= 16 && ledger.reservations.len() <= 64,
            "oversized reserve state",
        )?;
        let mut held = BTreeMap::<u64, u64>::new();
        for reservation in ledger.reservations.values() {
            for (domain, charge) in &reservation.by_domain {
                ensure(
                    ledger.domains.contains_key(domain),
                    "unknown reserved capacity domain",
                )?;
                held.insert(
                    *domain,
                    add(held.get(domain).copied().unwrap_or(0), *charge)?,
                );
            }
        }
        for (id, domain) in &ledger.domains {
            ensure(
                domain.held == held.get(id).copied().unwrap_or(0),
                "reserve charge mismatch",
            )?;
            ensure(
                add(domain.allocated, domain.held)? <= domain.limit,
                "reserve budget exceeded",
            )?;
        }
        Ok(ledger)
    }

    fn accepted(&mut self, operation: u64) -> Result<()> {
        self.reservations
            .get_mut(&operation)
            .ok_or("missing reservation")?
            .accepted = true;
        Ok(())
    }

    fn preallocate_file_blocks(&mut self, operation: u64, id: u64, bytes: u64) -> Result<()> {
        // Models successful F_PREALLOCATE/F_ALLOCATEALL data allocation only.
        // Namespace, metadata COW, quota races and future allocations remain liabilities.
        let existing = self
            .reservations
            .get(&operation)
            .ok_or("missing reservation")?
            .preallocated_file_blocks
            .get(&id)
            .copied()
            .unwrap_or(0);
        let total = add(existing, bytes)?;
        self.allocate(operation, id, bytes, Cut::None)?;
        self.reservations
            .get_mut(&operation)
            .unwrap()
            .preallocated_file_blocks
            .insert(id, total);
        Ok(())
    }

    // Actual allocation consumes held capacity, never credits predicted reclamation.
    // A post-effect error conservatively retains the full original held charge.
    fn allocate(&mut self, operation: u64, id: u64, bytes: u64, cut: Cut) -> Result<()> {
        let reservation = self
            .reservations
            .get_mut(&operation)
            .ok_or("missing reservation")?;
        let held = reservation
            .by_domain
            .get_mut(&id)
            .ok_or("wrong capacity domain")?;
        ensure(bytes <= *held, "allocation exceeds admitted maximum")?;
        let domain = self.domains.get_mut(&id).ok_or("missing domain")?;
        if matches!(cut, Cut::Before | Cut::Torn) {
            return Err("ENOSPC before allocation; checkpoint liability remains".into());
        }
        if matches!(cut, Cut::After) {
            reservation.unknown = true;
            return Err("unknown allocation; full charge retained until reconciliation".into());
        }
        domain.allocated = add(domain.allocated, bytes)?;
        domain.held -= bytes;
        *held -= bytes;
        Ok(())
    }

    fn finish(&mut self, operation: u64, saved: bool) -> Result<()> {
        let reservation = self
            .reservations
            .get(&operation)
            .ok_or("missing reservation")?;
        ensure(!reservation.unknown, "unknown outcome retains reserve")?;
        ensure(
            !reservation.accepted || saved,
            "accepted checkpoint liability cannot be cancelled",
        )?;
        let reservation = self.reservations.remove(&operation).unwrap();
        for (id, held) in reservation.by_domain {
            self.domains.get_mut(&id).unwrap().held -= held;
        }
        Ok(())
    }

    fn reservation_digest(&self, operation: u64) -> Result<String> {
        let reservation = self
            .reservations
            .get(&operation)
            .ok_or("missing original reservation")?;
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(reservation).map_err(|e| e.to_string())?)
        ))
    }

    fn reconcile_allocation(
        &mut self,
        operation: u64,
        domain: u64,
        actual_bytes: u64,
        exact_pending_digest: &str,
    ) -> Result<()> {
        ensure(
            self.reservation_digest(operation)? == exact_pending_digest,
            "reconciliation does not match original pending evidence",
        )?;
        ensure(
            self.reservations[&operation].unknown,
            "no unresolved allocation",
        )?;
        // A bounded clone makes model reconciliation atomic on validation error.
        // Real exact allocation/identity evidence is supplied by the qualified
        // publisher; matching a model digest does not itself prove OS durability.
        let mut candidate = self.clone();
        candidate.allocate(operation, domain, actual_bytes, Cut::None)?;
        candidate.reservations.get_mut(&operation).unwrap().unknown = false;
        *self = candidate;
        Ok(())
    }
}

fn measured_number(value: &serde_json::Value, field: &str) -> Result<u64> {
    value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("missing measured counter: {field}"))
}

fn compare_observation(
    phase: &str,
    sample: &serde_json::Value,
    old_allocated: u64,
) -> Result<serde_json::Value> {
    let counters = &sample["counters"];
    let compaction = phase.contains("compaction");
    let budget = Budget::integrated(1, compaction)?;
    let data = measured_number(counters, "data_write_bytes")?;
    let metadata = measured_number(counters, "meta_write_bytes")?;
    let envelope = measured_number(counters, "envelope_write_bytes")?;
    let names = measured_number(counters, "files_created")?;
    let barriers = measured_number(counters, "sync_calls")?;
    let reported_charge = measured_number(counters, "allocation_charge")?;
    let positive_growth = sample
        .get("after")
        .and_then(|v| v.get(1))
        .and_then(serde_json::Value::as_u64)
        .map(|after| after.saturating_sub(old_allocated));
    // Neither deleted files nor observed negative st_blocks growth is credit.
    let observed_allocation_lower_bound = reported_charge.max(positive_growth.unwrap_or(0));
    let data_bound = budget.sum_categories(&[if compaction {
        Category::CompactionCopy
    } else {
        Category::DataPack
    }])?;
    let metadata_bound = budget.sum_categories(&[
        Category::MetadataPack,
        Category::IndexPaths,
        Category::InventoryPaths,
        Category::LocatorBootstrap,
        Category::CompactionMetadata,
    ])?;
    let envelope_bound = budget.sum_categories(&[Category::Head, Category::Intent])?;
    let namespace_bound = budget.sum_categories(&[Category::Namespace])? / 16_384;
    ensure(
        data <= data_bound,
        "data copy exceeds declared admission bound",
    )?;
    ensure(
        metadata <= metadata_bound,
        "metadata exceeds declared admission bound",
    )?;
    ensure(
        envelope <= envelope_bound,
        "envelope exceeds declared admission bound",
    )?;
    ensure(
        names <= namespace_bound,
        "namespace count exceeds declared admission bound",
    )?;
    ensure(
        observed_allocation_lower_bound <= budget.total()?,
        "observed allocation exceeds reserve",
    )?;
    let deferred = sample["result"]["deferred_nonpositive_standalone"]
        .as_bool()
        .unwrap_or(false);
    if deferred {
        ensure(
            data == 0 && metadata == 0 && envelope == 0 && names == 0 && barriers == 0,
            "deferred maintenance mutated state",
        )?;
    }
    let retired = sample["result"]["retired_bytes"].as_u64().unwrap_or(0);
    Ok(
        json!({"phase":phase,"data_bytes":data,"metadata_bytes":metadata,"envelope_bytes":envelope,
        "new_names":names,"sync_calls":barriers,"reported_allocation_charge":reported_charge,
        "positive_allocated_growth":positive_growth,"observed_allocation_lower_bound":observed_allocation_lower_bound,
        "observed_old_placement_allocation":old_allocated,"reserved_additional_bytes":budget.total()?,
        "modeled_peak_over_observed_old_placement":add(old_allocated,budget.total()?)?,"retired_bytes_not_credited":retired,
        "full_integrated_old_keep_set_measured":false,
        "deferred_without_writes":deferred,"within_declared_synthetic_bounds":true,
        "barrier_kind_mapping_measured":false,"exclusive_os_reservation":false}),
    )
}

fn source_observations(row: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
    let mut observations = Vec::new();
    let mut old = row["baseline"][1].as_u64().unwrap_or(0);
    if let Some(appends) = row["appends"].as_array() {
        for append in appends {
            old = append["before"][1]
                .as_u64()
                .ok_or("missing old append allocation")?;
            observations.push(compare_observation("publication", append, old)?);
            old = append["after"][1]
                .as_u64()
                .ok_or("missing new append allocation")?;
        }
        observations.push(compare_observation(
            "root-turnover",
            &row["root_turnover"],
            old,
        )?);
        for phase in ["data_compaction", "metadata_compaction"] {
            let sample = &row[phase];
            observations.push(compare_observation(
                phase,
                sample,
                sample["before"][1]
                    .as_u64()
                    .ok_or("missing compaction old allocation")?,
            )?);
        }
    }
    if let Some(cases) = row["cases"].as_array() {
        for sample in cases {
            observations.push(compare_observation(
                "density-compaction",
                sample,
                sample["before"][1]
                    .as_u64()
                    .ok_or("missing density old allocation")?,
            )?);
        }
    }
    Ok(observations)
}

fn cross_check_file(path: &Path) -> Result<serde_json::Value> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut observations = Vec::new();
    let mut examples = Vec::new();
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let row: serde_json::Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
        let batch = source_observations(&row)?;
        if row["n"].as_u64() == Some(100_000) {
            examples.extend(
                batch
                    .iter()
                    .filter(|sample| sample["phase"] != "publication")
                    .cloned(),
            );
        }
        observations.extend(batch);
    }
    ensure(!observations.is_empty(), "no observed publication counters")?;
    let maximum = |field: &str| {
        observations
            .iter()
            .filter_map(|row| row[field].as_u64())
            .max()
            .unwrap_or(0)
    };
    Ok(
        json!({"fixture":"capacity-observed-lower-bound-cross-check","source":path,
        "source_sha256":format!("{:x}",Sha256::digest(&bytes)),"observations_checked":observations.len(),
        "all_within_declared_synthetic_bounds":true,"max_data_bytes":maximum("data_bytes"),
        "max_metadata_bytes":maximum("metadata_bytes"),"max_envelope_bytes":maximum("envelope_bytes"),
        "max_reported_allocation_charge":maximum("reported_allocation_charge"),
        "max_positive_allocated_growth":maximum("positive_allocated_growth"),
        "allocation_growth_exceeded_reported_charge_cases":observations.iter().filter(|row|row["positive_allocated_growth"].as_u64()>row["reported_allocation_charge"].as_u64()).count(),
        "max_old_placement_allocation":maximum("observed_old_placement_allocation"),"max_modeled_peak_over_observed_old_placement":maximum("modeled_peak_over_observed_old_placement"),
        "deferred_cases":observations.iter().filter(|row|row["deferred_without_writes"]==true).count(),
        "new_explicit_charges":["liveness metadata","reserve snapshot","barrier evidence","pack rollover slack","second compaction selector HEAD/intent/receipt"],
        "reserved_but_not_separately_measured":["journal frames","original operation receipts","Saved receipts","semantic source index/inventory","liveness ledger","reserve checkpoint","filesystem namespace/COW allocation","barrier kind and ordering"],
        "inference_limit":"old placement bytes exclude independently stored semantic source/liveness/journal state; complete old keep-set is an admission input, not proved by these counters; lower bounds do not prove future maxima or qualified persistence; no deletion credit",
        "examples_100k":examples}),
    )
}

fn integrated_liability_trace() -> Result<serde_json::Value> {
    let old = 150_000_000; // Includes active/recovery, pinned packs and old locator generations.
    let mut ledger = Ledger {
        domains: [(
            1,
            Domain {
                limit: 500_000_000,
                allocated: old,
                held: 0,
                peak: old,
            },
        )]
        .into(),
        ..Ledger::default()
    };
    let mut trace = Vec::new();
    for operation in 1..=3 {
        ledger.admit(operation, &[(1, Budget::integrated(8, false)?)])?;
        ledger.accepted(operation)?;
        trace.push(json!({"stage":"accepted-unsaved","operation":operation,"allocated":ledger.domains[&1].allocated,"held":ledger.domains[&1].held}));
    }
    ledger.admit(99, &[(1, Budget::integrated(1, true)?)])?;
    let peak = ledger.domains[&1].peak;
    let original_held = ledger.domains[&1].held;
    let before = ledger.allocate(99, 1, 262_144, Cut::Before).unwrap_err();
    ensure(
        ledger.domains[&1].held == original_held,
        "pre-effect error dropped liability",
    )?;
    let after = ledger.allocate(99, 1, 262_144, Cut::After).unwrap_err();
    ensure(
        ledger.finish(99, true).is_err(),
        "unknown compaction finished",
    )?;
    let digest = ledger.reservation_digest(99)?;
    ledger.reconcile_allocation(99, 1, 262_144, &digest)?;
    // The candidate may later be reclaimed, but no credit is taken here.
    ledger.finish(99, true)?;
    ledger.finish(1, true)?;
    ensure(
        ledger.reservations.contains_key(&2) && ledger.reservations.contains_key(&3),
        "later accepted liability erased",
    )?;
    ensure(
        ledger.domains[&1].allocated == old + 262_144,
        "old-generation deletion credit",
    )?;
    Ok(
        json!({"fixture":"integrated-cumulative-liability","events":trace,"compaction_peak":peak,
        "pre_effect_enospc":before,"post_effect_enospc":after,"original_id_reconciled":99,
        "after_one_saved_and_compaction":ledger,"no_deletion_credit":true,"exclusive_os_reservation":false}),
    )
}

fn barrier_fault_case(cut_at: usize, cut: Cut) -> Result<serde_json::Value> {
    let mut stages = vec![
        "reserve-ledger-intent".to_owned(),
        "accepted-journal-original-receipts".to_owned(),
    ];
    for selector in 1..=2 {
        for phase in [
            "data-pack-files",
            "metadata-index-inventory-locator-liveness-files",
            "namespace-rollover",
            "candidate-intent-head-temp",
            "head-selection",
            "original-saved-receipt",
        ] {
            stages.push(format!("selector-{selector}:{phase}"));
        }
    }
    ensure(cut_at < stages.len(), "unknown barrier stage")?;
    let old = 20_000_000;
    let mut ledger = Ledger {
        domains: [(
            1,
            Domain {
                limit: 500_000_000,
                allocated: old,
                held: 0,
                peak: old,
            },
        )]
        .into(),
        ..Ledger::default()
    };
    ledger.admit(1, &[(1, Budget::integrated(8, true)?)])?;
    for index in 0..cut_at {
        ledger.allocate(1, 1, 4096, Cut::None)?;
        if index == 1 {
            ledger.accepted(1)?;
        }
    }
    let before_held = ledger.domains[&1].held;
    let accepted_at_fault = ledger.reservations[&1].accepted;
    ledger
        .allocate(1, 1, 4096, cut)
        .expect_err("fault requested");
    ensure(
        ledger.domains[&1].held == before_held,
        "barrier failure released reserve",
    )?;
    if matches!(cut, Cut::After) {
        ensure(
            ledger.finish(1, true).is_err(),
            "unknown barrier acknowledged Saved",
        )?;
        let digest = ledger.reservation_digest(1)?;
        ledger.reconcile_allocation(1, 1, 4096, &digest)?;
    } else {
        ledger.allocate(1, 1, 4096, Cut::None)?;
    }
    if cut_at == 1 {
        ledger.accepted(1)?;
    }
    for index in cut_at + 1..stages.len() {
        ledger.allocate(1, 1, 4096, Cut::None)?;
        if index == 1 {
            ledger.accepted(1)?;
        }
    }
    ledger.finish(1, true)?;
    ensure(
        ledger.domains[&1].allocated == old + u64::try_from(stages.len()).unwrap() * 4096,
        "reconciliation duplicated allocation or credited deletion",
    )?;
    Ok(
        json!({"barrier":stages[cut_at],"fault":if matches!(cut,Cut::After){"post-effect-unknown"}else{"pre-effect-enospc"},
        "accepted_at_fault":accepted_at_fault,"saved_at_fault":false,"held_at_fault":before_held,
        "same_operation_reconciled":1,"old_allocated_retained":old,"final_allocated":ledger.domains[&1].allocated,
        "qualification":"model fault ordering only, not a syscall or durability qualification"}),
    )
}

fn barrier_fault_trace() -> Result<serde_json::Value> {
    let mut cases = Vec::new();
    for stage in 0..14 {
        for cut in [Cut::Before, Cut::After] {
            cases.push(barrier_fault_case(stage, cut)?);
        }
    }
    Ok(json!({"fixture":"integrated-capacity-barrier-faults","cases":cases,"count":28}))
}

fn benchmark(count: u64) -> Result<serde_json::Value> {
    let mut live = Live::default();
    let start = Instant::now();
    for id in 1..=count {
        let delta = live.create(id, id, if id > 1 { vec![id - 1] } else { vec![] }, 1)?;
        live.apply(&delta)?;
    }
    let build_us = start.elapsed().as_micros();
    let pin = Pin {
        class: PinClass::Active,
        object: count,
        epoch: 1,
        unknown: false,
    };
    let delta = live.pin(count + 1, 1, pin)?;
    live.apply(&delta)?;
    // Setup/audit phase clears stale creation candidates while the root is live.
    // The timed phase below processes a genuine zero-count descendant cascade.
    while !live.queue.is_empty() {
        let delta = live.drain_one(add(live.sequence, 1)?)?;
        live.apply(&delta)?;
    }
    let delta = live.release(
        add(live.sequence, 1)?,
        1,
        1,
        ReleaseProof {
            original_token: 1,
            completed: true,
            reconciled: true,
            reader_finished: true,
        },
    )?;
    live.apply(&delta)?;
    let before = live.touched;
    let start = Instant::now();
    for _ in 0..MAX_DRAIN {
        let delta = live.drain_one(add(live.sequence, 1)?)?;
        live.apply(&delta)?;
    }
    let foreground_us = start.elapsed().as_micros();
    let touched = live.touched - before;
    live.audit()?;
    Ok(
        json!({"fixture":"incremental-liveness-reserve","objects":count,"build_us":build_us,
        "bounded_queue_steps":MAX_DRAIN,"foreground_touched_records":touched,
        "foreground_us":foreground_us,"remaining_queue_debt":live.queue.len(),
        "retired_records":live.objects.values().filter(|object| object.retired).count(),
        "foreground_lifetime_scans":0,"explicit_audit_objects":count,
        "physical_pack_deletions":0,"durability_qualified":false}),
    )
}

fn compact_benchmark(count: u64) -> Result<serde_json::Value> {
    ensure(count >= 32, "benchmark needs 32 descendants")?;
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut pages = Paged::new(directory.path())?;
    let start = Instant::now();
    let mut max_group_pages = 0;
    let mut max_group_bytes = 0;
    let mut first = 1;
    while first <= count {
        let group = 64.min(count - first + 1);
        let old_pages = pages.writes;
        let old_bytes = pages.file.metadata().map_err(|e| e.to_string())?.len();
        pages.create_pinned_group(first, group)?;
        max_group_pages = max_group_pages.max(pages.writes - old_pages);
        max_group_bytes = max_group_bytes
            .max(pages.file.metadata().map_err(|e| e.to_string())?.len() - old_bytes);
        first += group;
    }
    let build_us = start.elapsed().as_micros();
    let metadata = pages.file.metadata().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut pages = Paged::reopen(directory.path())?;
    let reopen_us = start.elapsed().as_micros();
    let reopen_reads = pages.reads;
    let before = pages.reads;
    ensure(pages.get(5, count)?.is_some(), "missing original operation")?;
    let operation_lookup_reads = pages.reads - before;
    pages.release_pinned_group(count - 31, 32)?;
    let before_pages = pages.writes;
    let before_bytes = pages.file.metadata().map_err(|e| e.to_string())?.len();
    let before_packs = pages.pack_writes;
    let before_reads = pages.reads;
    let start = Instant::now();
    pages.drain_bounded(MAX_DRAIN)?;
    let cascade_us = start.elapsed().as_micros();
    let cascade_bytes = pages.file.metadata().map_err(|e| e.to_string())?.len() - before_bytes;
    let cascade_reads = pages.reads - before_reads;
    let object: Object = serde_json::from_value(
        pages
            .get(1, count - 31)?
            .ok_or("missing final descendant")?,
    )
    .map_err(|e| e.to_string())?;
    ensure(object.retired, "cascade did not reach 32 descendants")?;
    let retained: Object =
        serde_json::from_value(pages.get(1, count - 32)?.ok_or("missing pinned boundary")?)
            .map_err(|e| e.to_string())?;
    ensure(
        !retained.retired && retained.incoming == 1,
        "remaining pin was lost",
    )?;
    let budget = Budget::worst_case(64, 256, 0)?;
    Ok(
        json!({"fixture":"packed-coalesced-liveness","objects":count,"pins":count,"original_operation_commitments":count,
        "build_us":build_us,"group_limit":64,"max_group_pages":max_group_pages,"max_group_appended_bytes":max_group_bytes,
        "arena_logical_bytes":metadata.len(),"arena_allocated_bytes":metadata.blocks()*512,
        "reopen_reads":reopen_reads,"reopen_us":reopen_us,"operation_lookup_reads":operation_lookup_reads,
        "cascade_records":32,"cascade_pack_appends":pages.pack_writes-before_packs,"cascade_pages":pages.writes-before_pages,
        "cascade_appended_bytes":cascade_bytes,"cascade_reads":cascade_reads,"cascade_us":cascade_us,
        "modeled_checkpoint_liability":budget.total()?,"modeled_peak_allocated_plus_liability":add(metadata.blocks()*512,budget.total()?)?,
        "foreground_lifetime_scan":false,"metadata_reclamation_proven":false,"exclusive_os_reservation":false,
        "physical_pack_deletions":0,"note":"pack compaction/reclamation and original semantic index integration remain separate"}),
    )
}

fn reserve_trace() -> Result<()> {
    let mut ledger = Ledger {
        domains: [(
            1,
            Domain {
                limit: 100_000_000,
                allocated: 20_000_000,
                held: 0,
                peak: 20_000_000,
            },
        )]
        .into(),
        ..Ledger::default()
    };
    println!("{}", json!({"stage":"initial","ledger":ledger}));
    ledger.admit(1, &[(1, Budget::worst_case(8, 64, 1_048_576)?)])?;
    println!(
        "{}",
        json!({"stage":"reserve-before-admission","ledger":ledger})
    );
    ledger.accepted(1)?;
    println!(
        "{}",
        json!({"stage":"journal-accepted-checkpoint-liability-retained","ledger":ledger})
    );
    ledger.preallocate_file_blocks(1, 1, 262_144)?;
    println!(
        "{}",
        json!({"stage":"file-data-preallocation-only","ledger":ledger})
    );
    let error = ledger.allocate(1, 1, 4096, Cut::Before).unwrap_err();
    println!(
        "{}",
        json!({"stage":"enospc-before-effect","outcome":error,"ledger":ledger})
    );
    let error = ledger.allocate(1, 1, 8192, Cut::After).unwrap_err();
    println!(
        "{}",
        json!({"stage":"unknown-allocation-completion","outcome":error,"ledger":ledger})
    );
    let error = ledger.finish(1, true).unwrap_err();
    println!(
        "{}",
        json!({"stage":"premature-saved-refused","outcome":error,"ledger":ledger})
    );
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "cross-check")
    {
        for path in &arguments[1..] {
            println!("{}", cross_check_file(Path::new(path))?);
        }
        println!("{}", integrated_liability_trace()?);
        println!("{}", barrier_fault_trace()?);
        return Ok(());
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == "trace")
    {
        return reserve_trace();
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == "compact")
    {
        for argument in &arguments[1..] {
            println!(
                "{}",
                compact_benchmark(argument.parse::<u64>().map_err(|e| e.to_string())?)?
            );
        }
        return Ok(());
    }
    for count in [1000, 10_000, 100_000] {
        println!("{}", benchmark(count)?);
    }
    let budget = Budget::worst_case(8, 64, 1_048_576)?;
    println!(
        "{}",
        json!({"fixture":"reserve-bound","categories":budget,"total":budget.total()?,
        "exclusive_os_reservation":false,"guarantee":"bounded internal admission; later ENOSPC preserves old state and liability",
        "limits":"synthetic maxima; paged normal reopen bounded, full replay audit only; no production wire/GC or APFS qualification"})
    );
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut pages = Paged::new(directory.path())?;
    for id in 1..=128 {
        pages.create_object(id, id, if id > 1 { vec![id - 1] } else { vec![] })?;
    }
    let allocated = pages.file.metadata().map_err(|e| e.to_string())?.len();
    let start = Instant::now();
    let mut reopened = Paged::reopen(directory.path())?;
    let reopen_us = start.elapsed().as_micros();
    let reopen_reads = reopened.reads;
    let before = reopened.reads;
    ensure(reopened.get(1, 128)?.is_some(), "missing checkpoint object")?;
    let point_reads = reopened.reads - before;
    // Clear the original candidate queue; its last item starts the real cascade.
    for _ in 0..4 {
        reopened.drain_bounded(MAX_DRAIN)?;
    }
    let before_reads = reopened.reads;
    let before_writes = reopened.writes;
    let before_packs = reopened.pack_writes;
    let before_bytes = reopened.file.metadata().map_err(|e| e.to_string())?.len();
    let start = Instant::now();
    reopened.drain_bounded(MAX_DRAIN)?;
    println!(
        "{}",
        json!({"fixture":"paged-liveness-reopen","objects":128,"reopen_reads":reopen_reads,
        "reopen_us":reopen_us,"point_reads":point_reads,"queue_steps":MAX_DRAIN,
        "queue_reads":reopened.reads-before_reads,"queue_page_writes":reopened.writes-before_writes,
        "queue_pack_appends":reopened.pack_writes-before_packs,
        "queue_case":"genuine-descendant-cascade",
        "queue_appended_bytes":reopened.file.metadata().map_err(|e| e.to_string())?.len()-before_bytes,
        "queue_us":start.elapsed().as_micros(),"initial_arena_bytes":allocated,
        "metadata_reclamation_proven":false,"lifetime_map_loaded":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(live: &mut Live, delta: Delta) {
        live.apply(&delta).unwrap();
    }
    fn create(live: &mut Live, id: u64, edges: Vec<u64>) {
        let delta = live.create(live.sequence + 1, id, edges, 1).unwrap();
        apply(live, delta);
    }
    fn proof(token: u64) -> ReleaseProof {
        ReleaseProof {
            original_token: token,
            completed: true,
            reconciled: true,
            reader_finished: true,
        }
    }
    fn ledger() -> Ledger {
        Ledger {
            domains: [
                (
                    1,
                    Domain {
                        limit: 100_000_000,
                        allocated: 20_000_000,
                        held: 0,
                        peak: 20_000_000,
                    },
                ),
                (
                    2,
                    Domain {
                        limit: 100_000_000,
                        allocated: 10_000_000,
                        held: 0,
                        peak: 10_000_000,
                    },
                ),
            ]
            .into(),
            ..Ledger::default()
        }
    }

    #[test]
    fn every_pin_class_survives_other_root_release_and_queue() {
        let classes = [
            PinClass::Active,
            PinClass::Recovery,
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
        for class in classes {
            let mut live = Live::default();
            create(&mut live, 1, vec![]);
            create(&mut live, 2, vec![1]);
            create(&mut live, 3, vec![1]);
            let delta = live
                .pin(
                    4,
                    40,
                    Pin {
                        class,
                        object: 2,
                        epoch: 7,
                        unknown: true,
                    },
                )
                .unwrap();
            apply(&mut live, delta);
            let delta = live
                .pin(
                    5,
                    50,
                    Pin {
                        class: PinClass::Active,
                        object: 3,
                        epoch: 8,
                        unknown: false,
                    },
                )
                .unwrap();
            apply(&mut live, delta);
            assert!(live.release(6, 40, 6, proof(40)).is_err());
            let mut incomplete = proof(40);
            incomplete.reconciled = false;
            assert!(live.release(6, 40, 7, incomplete).is_err());
            let delta = live.release(6, 50, 8, proof(50)).unwrap();
            apply(&mut live, delta);
            for _ in 0..MAX_DRAIN {
                let delta = live.drain_one(live.sequence + 1).unwrap();
                apply(&mut live, delta);
            }
            assert!(!live.objects[&1].retired && !live.objects[&2].retired);
            assert!(live.objects[&3].retired);
            live.audit().unwrap();
        }
    }

    #[test]
    fn reader_lease_requires_completion_and_cannot_resurrect_retired_data() {
        let mut live = Live::default();
        create(&mut live, 1, vec![]);
        let delta = live
            .pin(
                2,
                2,
                Pin {
                    class: PinClass::ReaderLease,
                    object: 1,
                    epoch: 3,
                    unknown: false,
                },
            )
            .unwrap();
        apply(&mut live, delta);
        let mut unfinished = proof(2);
        unfinished.reader_finished = false;
        assert!(live.release(3, 2, 3, unfinished).is_err());
        let mut no_barrier = proof(2);
        no_barrier.completed = false;
        assert!(live.release(3, 2, 3, no_barrier).is_err());
        let delta = live.release(3, 2, 3, proof(2)).unwrap();
        apply(&mut live, delta);
        let delta = live.drain_one(4).unwrap();
        apply(&mut live, delta);
        assert!(
            live.pin(
                5,
                5,
                Pin {
                    class: PinClass::Active,
                    object: 1,
                    epoch: 4,
                    unknown: false
                }
            )
            .is_err()
        );
    }

    #[test]
    fn durable_postimages_replay_exactly_once_and_refuse_torn_or_conflicting_frames() {
        for cut in [Cut::None, Cut::Before, Cut::After, Cut::Torn] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("ledger.fixture");
            let mut journal = Journal::open(&path).unwrap();
            let mut live = Live::default();
            let first = live.create(1, 1, vec![], 1).unwrap();
            journal.persist(&mut live, &first, Cut::None).unwrap();
            let second = live.create(2, 2, vec![1], 1).unwrap();
            let result = journal.persist(&mut live, &second, cut);
            if matches!(cut, Cut::Torn) {
                assert!(Journal::replay(&path).is_err());
                continue;
            }
            let mut recovered = Journal::replay(&path).unwrap();
            assert_eq!(
                recovered.objects.len(),
                if matches!(cut, Cut::Before) { 1 } else { 2 }
            );
            if !matches!(cut, Cut::Before) {
                assert!(!recovered.apply(&second).unwrap());
                let mut conflict = second.clone();
                conflict.objects.get_mut(&2).unwrap().allocated += 1;
                assert!(recovered.apply(&conflict).is_err());
            }
            assert_eq!(result.is_ok(), matches!(cut, Cut::None));
            recovered.audit().unwrap();
        }
    }

    #[test]
    fn persisted_pin_release_and_cascade_survive_lost_completion() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.fixture");
        let mut journal = Journal::open(&path).unwrap();
        let mut live = Live::default();
        for id in 1..=3 {
            let delta = live
                .create(id, id, if id > 1 { vec![id - 1] } else { vec![] }, 1)
                .unwrap();
            journal.persist(&mut live, &delta, Cut::None).unwrap();
        }
        let delta = live
            .pin(
                4,
                4,
                Pin {
                    class: PinClass::Relocation,
                    object: 3,
                    epoch: 9,
                    unknown: true,
                },
            )
            .unwrap();
        journal.persist(&mut live, &delta, Cut::After).unwrap_err();
        live = Journal::replay(&path).unwrap();
        assert_eq!(live.pins.len(), 1);
        let delta = live.release(5, 4, 9, proof(4)).unwrap();
        journal.persist(&mut live, &delta, Cut::After).unwrap_err();
        live = Journal::replay(&path).unwrap();
        for _ in 0..MAX_DRAIN {
            let delta = live.drain_one(live.sequence + 1).unwrap();
            journal.persist(&mut live, &delta, Cut::None).unwrap();
        }
        let recovered = Journal::replay(&path).unwrap();
        assert!(recovered.objects.values().all(|object| object.retired));
        assert_eq!(
            recovered
                .objects
                .values()
                .map(|object| object.allocated)
                .sum::<u64>(),
            12_288
        );
    }

    #[test]
    fn budget_overflow_and_shared_domain_sum_refuse_atomically() {
        assert!(Budget::worst_case(65, 1, 0).is_err());
        assert!(Budget::worst_case(1, 257, 0).is_err());
        assert!(Budget::worst_case(1, 1, u64::MAX).is_err());
        let mut budget = Budget::default();
        assert!(budget.charge(Category::DataPack, u64::MAX, 2).is_err());
        let mut ledger = ledger();
        let budget = Budget::worst_case(32, 128, 0).unwrap();
        ledger.domains.get_mut(&1).unwrap().limit = 20_000_000 + budget.total().unwrap();
        let original = serde_json::to_vec(&ledger).unwrap();
        assert!(
            ledger
                .admit(1, &[(1, budget.clone()), (1, budget.clone())])
                .is_err()
        );
        assert_eq!(serde_json::to_vec(&ledger).unwrap(), original);
        ledger
            .admit(1, &[(1, budget.clone()), (2, budget)])
            .unwrap();
        assert!(ledger.domains.values().all(|domain| domain.held > 0));
    }

    #[test]
    fn accepted_checkpoint_liability_and_unknown_compaction_are_never_deleted_for_space() {
        let mut ledger = ledger();
        let budget = Budget::worst_case(8, 64, 1_048_576).unwrap();
        ledger.admit(1, &[(1, budget)]).unwrap();
        ledger.accepted(1).unwrap();
        let held = ledger.domains[&1].held;
        assert!(ledger.finish(1, false).is_err());
        assert!(ledger.allocate(1, 1, 4096, Cut::Before).is_err());
        assert_eq!(ledger.domains[&1].held, held);
        ledger.allocate(1, 1, 4096, Cut::None).unwrap();
        assert_eq!(ledger.domains[&1].allocated, 20_004_096);
        assert!(ledger.allocate(1, 1, 8192, Cut::After).is_err());
        assert!(ledger.finish(1, true).is_err());
        let recovered: Ledger =
            serde_json::from_slice(&serde_json::to_vec(&ledger).unwrap()).unwrap();
        assert_eq!(recovered.domains[&1].held, held - 4096);
        assert!(recovered.reservations[&1].unknown);
        assert_eq!(
            recovered.domains[&1].allocated + recovered.domains[&1].held,
            20_000_000 + held
        );
    }

    #[test]
    fn successful_save_releases_only_unused_reserve_not_old_physical_bytes() {
        let mut ledger = ledger();
        let budget = Budget::worst_case(1, 16, 1_048_576).unwrap();
        ledger.admit(1, &[(1, budget)]).unwrap();
        ledger.accepted(1).unwrap();
        ledger.allocate(1, 1, 1_048_576, Cut::None).unwrap();
        ledger.finish(1, true).unwrap();
        assert_eq!(ledger.domains[&1].held, 0);
        assert_eq!(ledger.domains[&1].allocated, 21_048_576);
        assert!(ledger.domains[&1].peak >= ledger.domains[&1].allocated);
    }

    #[test]
    fn foreground_work_stays_bounded_as_lifetime_grows() {
        for count in [1000, 10_000, 100_000] {
            let row = benchmark(count).unwrap();
            assert_eq!(row["foreground_lifetime_scans"], 0);
            assert!(
                row["foreground_touched_records"].as_u64().unwrap()
                    <= u64::try_from(MAX_DRAIN * (MAX_EDGES + 2)).unwrap()
            );
        }
    }

    #[test]
    fn selected_checkpoint_reopen_and_new_mutation_do_not_replay_lifetime() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = Paged::new(directory.path()).unwrap();
        for id in 1..=64 {
            pages
                .create_object(id, id, if id > 1 { vec![id - 1] } else { vec![] })
                .unwrap();
        }
        let mut reopened = Paged::reopen(directory.path()).unwrap();
        assert_eq!(reopened.reads, 1);
        let before = reopened.reads;
        reopened.create_object(65, 65, vec![64]).unwrap();
        assert!(reopened.reads - before <= 10 * u64::try_from(DEPTH + 1).unwrap());
        reopened.drain_bounded(MAX_DRAIN).unwrap();
        let mut fresh = Paged::reopen(directory.path()).unwrap();
        assert_eq!(fresh.reads, 1);
        let object: Object = serde_json::from_value(fresh.get(1, 64).unwrap().unwrap()).unwrap();
        assert_eq!(object.incoming, 1);
        assert!(!object.retired);
        assert!(fresh.drain_bounded(MAX_DRAIN + 1).is_err());
    }

    #[test]
    fn bounded_pending_transition_reopen_keeps_exact_old_or_candidate_and_freezes() {
        for cut in [Cut::Before, Cut::After] {
            let directory = tempfile::tempdir().unwrap();
            let mut pages = Paged::new(directory.path()).unwrap();
            let local = Live::default();
            let delta = local.create(1, 1, vec![], 1).unwrap();
            assert!(pages.checkpoint(&delta, cut).is_err());
            let mut reopened = Paged::reopen(directory.path()).unwrap();
            assert_eq!(reopened.reads, 2);
            assert!(reopened.frozen);
            assert_eq!(
                reopened.get(1, 1).unwrap().is_some(),
                matches!(cut, Cut::After)
            );
            assert!(reopened.create_object(2, 2, vec![]).is_err());
        }
    }

    #[test]
    fn paged_exact_replay_and_corrupt_count_page_refuse_without_lifetime_audit() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = Paged::new(directory.path()).unwrap();
        let delta = Live::default().create(1, 1, vec![], 1).unwrap();
        pages.checkpoint(&delta, Cut::None).unwrap();
        let before = pages.writes;
        pages.checkpoint(&delta, Cut::None).unwrap();
        assert_eq!(pages.writes, before);
        let root = pages.cursor.root.unwrap();
        pages.file.seek(SeekFrom::Start(root + 40)).unwrap();
        pages.file.write_all(&[255]).unwrap();
        pages.file.sync_all().unwrap();
        let mut reopened = Paged::reopen(directory.path()).unwrap();
        assert!(reopened.get(1, 1).is_err());
    }

    #[test]
    fn file_preallocation_never_claims_namespace_or_future_cow_reservation() {
        let mut ledger = ledger();
        let budget = Budget::worst_case(1, 16, 0).unwrap();
        ledger.admit(1, &[(1, budget)]).unwrap();
        ledger.accepted(1).unwrap();
        let total = ledger.domains[&1].allocated + ledger.domains[&1].held;
        ledger.preallocate_file_blocks(1, 1, 262_144).unwrap();
        assert_eq!(
            ledger.reservations[&1].preallocated_file_blocks[&1],
            262_144
        );
        assert!(ledger.domains[&1].held > 0);
        assert_eq!(
            ledger.domains[&1].allocated + ledger.domains[&1].held,
            total
        );
        assert!(ledger.allocate(1, 1, 4096, Cut::Before).is_err());
        assert!(ledger.finish(1, false).is_err());
    }

    #[test]
    fn finite_reserve_snapshot_survives_unknown_completion_without_erasing_liability() {
        let directory = tempfile::tempdir().unwrap();
        let mut ledger = ledger();
        ledger.persist(directory.path(), Cut::None).unwrap();
        ledger
            .admit(1, &[(1, Budget::worst_case(8, 64, 1_048_576).unwrap())])
            .unwrap();
        assert!(ledger.persist(directory.path(), Cut::Before).is_err());
        assert!(
            Ledger::reopen(directory.path())
                .unwrap()
                .reservations
                .is_empty()
        );
        assert!(ledger.persist(directory.path(), Cut::After).is_err());
        let mut recovered = Ledger::reopen(directory.path()).unwrap();
        assert_eq!(recovered.domains[&1].held, ledger.domains[&1].held);
        recovered.accepted(1).unwrap();
        recovered.persist(directory.path(), Cut::None).unwrap();
        let mut after_crash = Ledger::reopen(directory.path()).unwrap();
        assert!(after_crash.finish(1, false).is_err());
        after_crash.domains.get_mut(&1).unwrap().held -= 1;
        after_crash.persist(directory.path(), Cut::None).unwrap();
        assert!(Ledger::reopen(directory.path()).is_err());
    }

    #[test]
    fn all_pin_classes_and_epochs_are_in_the_selected_persistent_checkpoint() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = Paged::new(directory.path()).unwrap();
        let mut oracle = Live::default();
        let delta = oracle.create(1, 1, vec![], 1).unwrap();
        pages.checkpoint(&delta, Cut::None).unwrap();
        apply(&mut oracle, delta);
        let classes = [
            PinClass::Active,
            PinClass::Recovery,
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
        for (index, class) in classes.into_iter().enumerate() {
            let token = u64::try_from(index).unwrap() + 2;
            let delta = oracle
                .pin(
                    token,
                    token,
                    Pin {
                        class,
                        object: 1,
                        epoch: 99,
                        unknown: true,
                    },
                )
                .unwrap();
            pages.checkpoint(&delta, Cut::None).unwrap();
            apply(&mut oracle, delta);
        }
        let mut reopened = Paged::reopen(directory.path()).unwrap();
        assert_eq!(reopened.reads, 1);
        let object: Object = serde_json::from_value(reopened.get(1, 1).unwrap().unwrap()).unwrap();
        assert_eq!(object.incoming, 12);
        for (index, class) in classes.into_iter().enumerate() {
            let token = u64::try_from(index).unwrap() + 2;
            let pin: Pin =
                serde_json::from_value(reopened.get(2, token).unwrap().unwrap()).unwrap();
            assert_eq!(pin.class, class);
            assert_eq!(pin.epoch, 99);
            assert!(pin.unknown);
        }
    }

    #[test]
    fn coalesced_cascade_persists_counts_once_per_bounded_group() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = Paged::new(directory.path()).unwrap();
        for id in 1..=96 {
            pages
                .create_object(id, id, if id > 1 { vec![id - 1] } else { vec![] })
                .unwrap();
        }
        for _ in 0..8 {
            if pages.cursor.queue_head == pages.cursor.queue_tail {
                break;
            }
            let before = pages.pack_writes;
            pages.drain_bounded(MAX_DRAIN).unwrap();
            assert_eq!(pages.pack_writes - before, 1);
            pages = Paged::reopen(directory.path()).unwrap();
            assert_eq!(pages.reads, 1);
        }
        assert_eq!(pages.cursor.queue_head, pages.cursor.queue_tail);
        // Separate explicit full audit; no normal reopening invokes this loop.
        for id in 1..=96 {
            let object: Object =
                serde_json::from_value(pages.get(1, id).unwrap().unwrap()).unwrap();
            assert!(object.retired);
            assert_eq!(object.incoming, 0);
            assert_eq!(object.allocated, 4096);
        }
    }

    #[test]
    fn candidate_local_route_adapter_keeps_recovery_reader_and_unresolved_roots() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = Paged::new(directory.path()).unwrap();
        pages.create_pinned_group(1, 16).unwrap();
        let first_root = pages.cursor.root;
        let first_end = pages.file.metadata().unwrap().len();
        let candidate = pages.candidate_routes(0, first_end).unwrap();
        pages.create_pinned_group(17, 16).unwrap();
        let second_root = pages.cursor.root;
        assert!(
            !pages
                .candidate_is_unreferenced(&candidate, &[second_root, first_root], true)
                .unwrap()
        );
        let root_candidate: Vec<_> = candidate
            .iter()
            .copied()
            .filter(|(offset, _)| Some(*offset) == first_root)
            .collect();
        assert_eq!(root_candidate.len(), 1);
        // Once both dispatch roots have moved, an outstanding reader can still
        // retain the former root page. Release needs its exact epoch evidence.
        assert!(
            !pages
                .candidate_is_unreferenced(
                    &root_candidate,
                    &[second_root, second_root, first_root],
                    true
                )
                .unwrap()
        );
        assert!(
            pages
                .candidate_is_unreferenced(&root_candidate, &[second_root, second_root], false)
                .is_err()
        );
        assert!(
            pages
                .candidate_is_unreferenced(&root_candidate, &[second_root, second_root], true)
                .unwrap()
        );
        // Shared descendants in the same candidate remain live: no pack-level
        // retirement follows merely because the former root page became dead.
        assert!(
            !pages
                .candidate_is_unreferenced(&candidate, &[second_root, second_root], true)
                .unwrap()
        );
        assert!(pages.candidate_routes(0, 256 * 1024 + 1).is_err());
        assert!(pages.directory.join("pages").exists());
    }

    #[test]
    fn integrated_budget_charges_every_explicit_missing_category_and_two_selectors() {
        let one = Budget::integrated(1, false).unwrap();
        let two = Budget::integrated(1, true).unwrap();
        for category in [
            Category::LivenessMetadata,
            Category::ReserveLedger,
            Category::BarrierEvidence,
            Category::PackRolloverSlack,
        ] {
            assert!(one.0[&category] > 0);
        }
        assert_eq!(two.0[&Category::Head], 2 * one.0[&Category::Head]);
        assert_eq!(two.0[&Category::Intent], 2 * one.0[&Category::Intent]);
        assert_eq!(
            two.0[&Category::SavedReceipt],
            2 * one.0[&Category::SavedReceipt]
        );
        assert!(two.total().unwrap() > one.total().unwrap());
    }

    #[test]
    fn observed_allocated_growth_is_not_hidden_by_smaller_charge_or_deleted_files() {
        let sample = json!({"before":[0,1_000_000,3],"after":[0,1_200_000,2],
            "counters":{"data_write_bytes":4096,"meta_write_bytes":4096,"envelope_write_bytes":1024,
            "files_created":3,"files_deleted":4,"sync_calls":10,"allocation_charge":49_152},
            "result":{"retired_bytes":262_144}});
        let observed = compare_observation("data_compaction", &sample, 1_000_000).unwrap();
        assert_eq!(observed["observed_allocation_lower_bound"], 200_000);
        assert_eq!(observed["observed_old_placement_allocation"], 1_000_000);
        assert!(
            observed["modeled_peak_over_observed_old_placement"]
                .as_u64()
                .unwrap()
                > 1_200_000
        );
        let mut unmeasured = sample;
        unmeasured["counters"]
            .as_object_mut()
            .unwrap()
            .remove("meta_write_bytes");
        assert!(compare_observation("data_compaction", &unmeasured, 1_000_000).is_err());
    }

    #[test]
    fn multiple_accepted_unsaved_groups_and_barrier_faults_keep_full_liability() {
        let trace = integrated_liability_trace().unwrap();
        let events = trace["events"].as_array().unwrap();
        let first = events[0]["held"].as_u64().unwrap();
        assert_eq!(events[1]["held"], first * 2);
        assert_eq!(events[2]["held"], first * 3);
        let faults = barrier_fault_trace().unwrap();
        assert_eq!(faults["count"], 28);
        assert!(
            faults["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|case| case["saved_at_fault"] == false)
        );
    }

    #[test]
    fn reconciliation_requires_exact_pending_evidence_and_cannot_spend_other_domain() {
        let mut ledger = ledger();
        ledger
            .admit(7, &[(1, Budget::integrated(1, true).unwrap())])
            .unwrap();
        ledger.allocate(7, 1, 4096, Cut::After).unwrap_err();
        let before = serde_json::to_vec(&ledger).unwrap();
        assert!(ledger.reconcile_allocation(7, 1, 4096, "wrong").is_err());
        let digest = ledger.reservation_digest(7).unwrap();
        assert!(ledger.reconcile_allocation(7, 2, 4096, &digest).is_err());
        assert_eq!(serde_json::to_vec(&ledger).unwrap(), before);
        ledger.reconcile_allocation(7, 1, 4096, &digest).unwrap();
        assert!(!ledger.reservations[&7].unknown);
    }
}
