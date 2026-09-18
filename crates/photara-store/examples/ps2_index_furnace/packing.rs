//! Disposable segment, batch, and block-relocation extension. No permanent wire.
#[allow(
    clippy::wildcard_imports,
    reason = "Private companion module shares the disposable fixture types and algorithms"
)]
use super::*;
use std::collections::HashMap;

const SEGMENT: u64 = 1024 * 1024;
const BLOCK: u64 = 4096;
const SLOTS: usize = (SEGMENT / BLOCK) as usize;
const TEMP: u64 = 1 << 63;
const OPERATIONS: usize = 64;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) enum Mode {
    Sealed,
    Relocatable,
}
#[derive(Serialize, Deserialize)]
struct State {
    mode: Mode,
    len: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Placement {
    generation: u64,
    table_sha: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct MoveIntent {
    segment: u64,
    old: Placement,
    new: Placement,
    reserve: u64,
}
struct LivePlan {
    blocks: BTreeMap<u64, Vec<bool>>,
    refs: BTreeMap<u64, HashSet<Ref>>,
    candidates: Vec<u64>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveCut {
    None,
    PartialCopy,
    BeforePointer,
    AfterPointer,
}

pub(super) struct Backend {
    dir: PathBuf,
    mode: Mode,
    len: u64,
    dirty: HashSet<PathBuf>,
}
fn sync_file(file: &File, stats: &mut Stats) -> Result<()> {
    let t = Instant::now();
    file.sync_all().map_err(|e| e.to_string())?;
    stats.sync_calls += 1;
    stats.sync_micros += t.elapsed().as_micros();
    Ok(())
}
fn sync_directory(dir: &Path, stats: &mut Stats) -> Result<()> {
    sync_file(&File::open(dir).map_err(|e| e.to_string())?, stats)
}
fn atomic<T: Serialize>(dir: &Path, name: &str, value: &T, stats: &mut Stats) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.pending"));
    let mut f = File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    stats.placement_write_bytes += bytes.len() as u64;
    sync_file(&f, stats)?;
    fs::rename(path, dir.join(name)).map_err(|e| e.to_string())?;
    sync_directory(dir, stats)
}
fn read_file(path: &Path, stats: &mut Stats) -> Result<Vec<u8>> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    stats.placement_reads += 1;
    stats.placement_read_bytes += bytes.len() as u64;
    Ok(bytes)
}
fn table_bytes(slots: &[u64]) -> Vec<u8> {
    slots.iter().flat_map(|v| v.to_le_bytes()).collect()
}
impl Backend {
    fn open(dir: &Path, mode: Mode, stats: &mut Stats) -> Result<Self> {
        let path = dir.join("placement-state");
        let len = if path.exists() {
            let persisted: State =
                serde_json::from_slice(&read_file(&path, stats)?).map_err(|e| e.to_string())?;
            ensure(persisted.mode == mode, "placement mode mismatch")?;
            persisted.len
        } else {
            0
        };
        Ok(Self {
            dir: dir.to_owned(),
            mode,
            len,
            dirty: HashSet::new(),
        })
    }
    pub(super) fn logical_len(&self) -> u64 {
        self.len
    }
    fn data_path(&self, segment: u64, generation: u64) -> PathBuf {
        self.dir
            .join(format!("segment-{segment}-{generation}.data"))
    }
    fn table_path(&self, segment: u64, generation: u64) -> PathBuf {
        self.dir
            .join(format!("segment-{segment}-{generation}.table"))
    }
    fn pointer_name(segment: u64) -> String {
        format!("segment-{segment}.placement")
    }
    fn create_segment(&mut self, segment: u64, stats: &mut Stats) -> Result<()> {
        let path = self.data_path(segment, 0);
        if path.exists() {
            return Ok(());
        }
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        self.dirty.insert(path);
        if self.mode == Mode::Relocatable {
            let bytes = table_bytes(&(0..SLOTS as u64).map(|s| s * BLOCK).collect::<Vec<_>>());
            let table = self.table_path(segment, 0);
            fs::write(&table, &bytes).map_err(|e| e.to_string())?;
            stats.placement_write_bytes += bytes.len() as u64;
            self.dirty.insert(table);
            let pointer = self.dir.join(Self::pointer_name(segment));
            let encoded = serde_json::to_vec(&Placement {
                generation: 0,
                table_sha: hash(&bytes),
            })
            .map_err(|e| e.to_string())?;
            fs::write(&pointer, &encoded).map_err(|e| e.to_string())?;
            stats.placement_write_bytes += encoded.len() as u64;
            self.dirty.insert(pointer);
        }
        Ok(())
    }
    pub(super) fn append(&mut self, bytes: &[u8], stats: &mut Stats) -> Result<u64> {
        let original = self.len;
        let mut remaining = bytes;
        while !remaining.is_empty() {
            let segment = self.len / SEGMENT;
            let within = self.len % SEGMENT;
            self.create_segment(segment, stats)?;
            let path = self.data_path(segment, 0);
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            ensure(
                file.metadata().map_err(|e| e.to_string())?.len() == within,
                "unreconciled segment tail",
            )?;
            let count = remaining
                .len()
                .min(usize::try_from(SEGMENT - within).map_err(|e| e.to_string())?);
            file.seek(SeekFrom::End(0))
                .and_then(|_| file.write_all(&remaining[..count]))
                .map_err(|e| e.to_string())?;
            self.dirty.insert(path);
            self.len += count as u64;
            remaining = &remaining[count..];
        }
        Ok(original)
    }
    fn placement(&self, segment: u64, stats: &mut Stats) -> Result<(Placement, Vec<u64>)> {
        let p: Placement = serde_json::from_slice(&read_file(
            &self.dir.join(Self::pointer_name(segment)),
            stats,
        )?)
        .map_err(|e| e.to_string())?;
        let bytes = read_file(&self.table_path(segment, p.generation), stats)?;
        ensure(
            bytes.len() == SLOTS * 8 && hash(&bytes) == p.table_sha,
            "placement table corruption",
        )?;
        let table = bytes
            .chunks_exact(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        Ok((p, table))
    }
    pub(super) fn read(&self, r: &Ref, stats: &mut Stats) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(usize::try_from(r.len).map_err(|e| e.to_string())?);
        let mut address = r.offset;
        let end = r.offset + r.len;
        while address < end {
            let segment = address / SEGMENT;
            let segment_end = end.min((segment + 1) * SEGMENT);
            let (generation, table) = if self.mode == Mode::Relocatable {
                let (p, t) = self.placement(segment, stats)?;
                (p.generation, Some(t))
            } else {
                (0, None)
            };
            let mut file =
                File::open(self.data_path(segment, generation)).map_err(|e| e.to_string())?;
            while address < segment_end {
                let local = address % SEGMENT;
                let slot = (local / BLOCK) as usize;
                let physical = if let Some(t) = &table {
                    ensure(t[slot] != u64::MAX, "required logical block retired")?;
                    t[slot] + local % BLOCK
                } else {
                    local
                };
                let count = usize::try_from((segment_end - address).min(BLOCK - local % BLOCK))
                    .map_err(|e| e.to_string())?;
                let mut bytes = vec![0; count];
                file.seek(SeekFrom::Start(physical))
                    .and_then(|_| file.read_exact(&mut bytes))
                    .map_err(|e| e.to_string())?;
                stats.physical_data_read_calls += 1;
                stats.physical_data_read_bytes += count as u64;
                out.extend(bytes);
                address += count as u64;
            }
        }
        Ok(out)
    }
    pub(super) fn sync(&mut self, stats: &mut Stats) -> Result<()> {
        for path in self.dirty.drain() {
            sync_file(&File::open(path).map_err(|e| e.to_string())?, stats)?;
        }
        atomic(
            &self.dir,
            "placement-state",
            &State {
                mode: self.mode,
                len: self.len,
            },
            stats,
        )
    }
    fn live_blocks(refs: &HashSet<Ref>) -> BTreeMap<u64, Vec<bool>> {
        let mut groups = BTreeMap::<u64, Vec<bool>>::new();
        for r in refs {
            let mut block = r.offset / BLOCK;
            let last = (r.offset + r.len - 1) / BLOCK;
            while block <= last {
                let segment = block / (SEGMENT / BLOCK);
                let slot = (block % (SEGMENT / BLOCK)) as usize;
                groups.entry(segment).or_insert_with(|| vec![false; SLOTS])[slot] = true;
                block += 1;
            }
        }
        groups
    }
    fn plan(&self, refs: &HashSet<Ref>) -> LivePlan {
        let blocks = Self::live_blocks(refs);
        let mut by_segment = BTreeMap::<u64, HashSet<Ref>>::new();
        for r in refs {
            for s in r.offset / SEGMENT..=(r.offset + r.len - 1) / SEGMENT {
                by_segment.entry(s).or_default().insert(r.clone());
            }
        }
        let mut candidates: Vec<_> = (0..self.len / SEGMENT)
            .filter(|s| blocks.get(s).is_none_or(|b| b.iter().any(|x| !x)))
            .collect();
        candidates.sort_by_key(|s| {
            blocks
                .get(s)
                .map_or(0, |b| b.iter().filter(|x| **x).count())
        });
        candidates.truncate(2);
        LivePlan {
            blocks,
            refs: by_segment,
            candidates,
        }
    }
    fn physical_bytes(&self) -> u64 {
        fs::read_dir(&self.dir)
            .unwrap()
            .map(|e| e.unwrap())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".data"))
            .map(|e| e.metadata().unwrap().len())
            .sum()
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep bounded relocation, barriers, and explicit cut points together"
    )]
    fn compact(
        &self,
        plan: &LivePlan,
        budget: u64,
        cut: MoveCut,
        stats: &mut Stats,
    ) -> Result<serde_json::Value> {
        ensure(
            !self.dir.join("accepted-group").exists(),
            "pending journal pins staging",
        )?;
        ensure(
            !self.dir.join("compaction-intent").exists(),
            "pending placement transition requires recovery",
        )?;
        let mut copied = 0;
        let mut retired = 0;
        let mut max_reserve = 0;
        let mut moved = 0;
        for &segment in &plan.candidates {
            let blocks = plan
                .blocks
                .get(&segment)
                .cloned()
                .unwrap_or_else(|| vec![false; SLOTS]);
            let refs = plan.refs.get(&segment).cloned().unwrap_or_default();
            let count = blocks.iter().filter(|b| **b).count();
            if self.mode == Mode::Sealed {
                if count == 0 {
                    let path = self.data_path(segment, 0);
                    retired += path.metadata().map_err(|e| e.to_string())?.len();
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                    sync_directory(&self.dir, stats)?;
                }
                continue;
            }
            let (old, old_table) = self.placement(segment, stats)?;
            let old_bytes = self
                .data_path(segment, old.generation)
                .metadata()
                .map_err(|e| e.to_string())?
                .len();
            let extra = count as u64 * BLOCK + (SLOTS * 8) as u64 + 8192;
            // Existing physical storage stays charged; budget covers a second
            // copy plus table/intent/pointer slack. No expected retirement credit.
            ensure(budget >= extra, "compaction reserve exhausted")?;
            max_reserve = max_reserve.max(extra);
            let generation = old.generation + 1;
            let mut table = vec![u64::MAX; SLOTS];
            let mut offset = 0;
            for (i, keep) in blocks.iter().enumerate() {
                if *keep {
                    table[i] = offset;
                    offset += BLOCK;
                }
            }
            let table_data = table_bytes(&table);
            let new = Placement {
                generation,
                table_sha: hash(&table_data),
            };
            let intent = MoveIntent {
                segment,
                old: old.clone(),
                new: new.clone(),
                reserve: extra,
            };
            atomic(&self.dir, "compaction-intent", &intent, stats)?;
            let mut input =
                File::open(self.data_path(segment, old.generation)).map_err(|e| e.to_string())?;
            let output = self.data_path(segment, generation);
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&output)
                .map_err(|e| e.to_string())?;
            for (i, keep) in blocks.iter().enumerate() {
                if *keep {
                    ensure(
                        old_table[i] != u64::MAX,
                        "cannot relocate missing live block",
                    )?;
                    let mut bytes = vec![0; usize::try_from(BLOCK).unwrap()];
                    input
                        .seek(SeekFrom::Start(old_table[i]))
                        .and_then(|_| input.read_exact(&mut bytes))
                        .map_err(|e| e.to_string())?;
                    stats.physical_data_read_calls += 1;
                    stats.physical_data_read_bytes += BLOCK;
                    if cut == MoveCut::PartialCopy {
                        file.write_all(&bytes[..bytes.len() / 2])
                            .map_err(|e| e.to_string())?;
                        return Err("injected partial relocation copy".to_owned());
                    }
                    file.write_all(&bytes).map_err(|e| e.to_string())?;
                    copied += BLOCK;
                }
            }
            stats.relocation_write_bytes += offset;
            sync_file(&file, stats)?;
            let target_table = self.table_path(segment, generation);
            let mut tf = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target_table)
                .map_err(|e| e.to_string())?;
            tf.write_all(&table_data).map_err(|e| e.to_string())?;
            stats.placement_write_bytes += table_data.len() as u64;
            sync_file(&tf, stats)?;
            sync_directory(&self.dir, stats)?;
            // Verify each retained object touching this segment from the staged
            // bytes before the placement-only pointer can select them.
            self.verify_generation(segment, &new, &blocks, stats)?;
            if cut == MoveCut::BeforePointer {
                return Err("injected before placement switch".to_owned());
            }
            atomic(&self.dir, &Self::pointer_name(segment), &new, stats)?;
            if cut == MoveCut::AfterPointer {
                return Err("injected after placement switch".to_owned());
            }
            self.verify_selected(&refs, stats)?;
            fs::remove_file(self.data_path(segment, old.generation)).map_err(|e| e.to_string())?;
            fs::remove_file(self.table_path(segment, old.generation)).map_err(|e| e.to_string())?;
            fs::remove_file(self.dir.join("compaction-intent")).map_err(|e| e.to_string())?;
            sync_directory(&self.dir, stats)?;
            retired += old_bytes;
            moved += 1;
        }
        Ok(json!({"copied_bytes":copied,
            "retired_old_data_bytes":retired,"max_extra_reserve_bytes":max_reserve,"segments_relocated":moved,
            "max_segments_per_pass":2,"scope":"full-audit plan; frozen selected roots; no incremental liveness proof"}))
    }
    fn verify_selected(&self, refs: &HashSet<Ref>, stats: &mut Stats) -> Result<()> {
        for r in refs {
            ensure(
                hash(&self.read(r, stats)?) == r.sha,
                "relocation changed a selected object",
            )?;
        }
        Ok(())
    }
    fn verify_generation(
        &self,
        segment: u64,
        new: &Placement,
        blocks: &[bool],
        stats: &mut Stats,
    ) -> Result<()> {
        let bytes = read_file(&self.table_path(segment, new.generation), stats)?;
        ensure(
            hash(&bytes) == new.table_sha && bytes.len() == SLOTS * 8,
            "staged table mismatch",
        )?;
        let table: Vec<_> = bytes
            .chunks_exact(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        // Compare each copied live block against its old generation byte-for-byte.
        let (old, old_table) = self.placement(segment, stats)?;
        let mut input =
            File::open(self.data_path(segment, old.generation)).map_err(|e| e.to_string())?;
        let mut output =
            File::open(self.data_path(segment, new.generation)).map_err(|e| e.to_string())?;
        for (i, keep) in blocks.iter().enumerate() {
            if *keep {
                ensure(
                    table[i] != u64::MAX && old_table[i] != u64::MAX,
                    "missing retained block",
                )?;
                let mut a = vec![0; usize::try_from(BLOCK).unwrap()];
                let mut b = a.clone();
                input
                    .seek(SeekFrom::Start(old_table[i]))
                    .and_then(|_| input.read_exact(&mut a))
                    .map_err(|e| e.to_string())?;
                output
                    .seek(SeekFrom::Start(table[i]))
                    .and_then(|_| output.read_exact(&mut b))
                    .map_err(|e| e.to_string())?;
                stats.physical_data_read_calls += 2;
                stats.physical_data_read_bytes += 2 * BLOCK;
                ensure(a == b, "relocation byte mismatch")?;
            }
        }
        Ok(())
    }
    fn recover_move(&self, plan: &LivePlan, stats: &mut Stats) -> Result<()> {
        let path = self.dir.join("compaction-intent");
        let intent: MoveIntent =
            serde_json::from_slice(&read_file(&path, stats)?).map_err(|e| e.to_string())?;
        let (selected, _) = self.placement(intent.segment, stats)?;
        ensure(
            selected == intent.old || selected == intent.new,
            "unknown placement outcome",
        )?;
        if selected == intent.old {
            // Cancel only the known unselected fixture copy; originals remain.
            for p in [
                self.data_path(intent.segment, intent.new.generation),
                self.table_path(intent.segment, intent.new.generation),
            ] {
                if p.exists() {
                    fs::remove_file(p).map_err(|e| e.to_string())?;
                }
            }
        } else {
            self.verify_selected(
                &plan.refs.get(&intent.segment).cloned().unwrap_or_default(),
                stats,
            )?;
            // Unknown pointer-barrier outcomes must be re-established before
            // retiring an old generation that a crash could otherwise reselect.
            sync_file(
                &File::open(self.data_path(intent.segment, intent.new.generation))
                    .map_err(|e| e.to_string())?,
                stats,
            )?;
            sync_file(
                &File::open(self.table_path(intent.segment, intent.new.generation))
                    .map_err(|e| e.to_string())?,
                stats,
            )?;
            atomic(
                &self.dir,
                &Self::pointer_name(intent.segment),
                &intent.new,
                stats,
            )?;
            for p in [
                self.data_path(intent.segment, intent.old.generation),
                self.table_path(intent.segment, intent.old.generation),
            ] {
                if p.exists() {
                    fs::remove_file(p).map_err(|e| e.to_string())?;
                }
            }
        }
        fs::remove_file(path).map_err(|e| e.to_string())?;
        sync_directory(&self.dir, stats)
    }
}

pub(super) struct Staging {
    nodes: Vec<(Ref, Node)>,
}
pub(super) fn is_staged(r: &Ref) -> bool {
    r.offset >= TEMP
}
impl Staging {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }
    pub(super) fn put(&mut self, node: &Node, stats: &mut Stats) -> Result<Ref> {
        let bytes = serde_json::to_vec(node).map_err(|e| e.to_string())?;
        ensure(bytes.len() as u64 <= MAX_NODE, "oversized staged node")?;
        let r = Ref {
            offset: TEMP + self.nodes.len() as u64,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        stats.staging_encoded_bytes += bytes.len() as u64;
        self.nodes.push((r.clone(), node.clone()));
        Ok(r)
    }
    pub(super) fn get(&self, r: &Ref, stats: &mut Stats) -> Result<Node> {
        let (stored, node) = self
            .nodes
            .get(usize::try_from(r.offset - TEMP).map_err(|e| e.to_string())?)
            .ok_or_else(|| "unknown staged ref".to_owned())?;
        ensure(stored == r, "staged ref mismatch")?;
        stats.staging_reads += 1;
        Ok(node.clone())
    }
    fn materialize(&self, st: &mut Store, r: &Ref, memo: &mut HashMap<u64, Ref>) -> Result<Ref> {
        if !is_staged(r) {
            return Ok(r.clone());
        }
        if let Some(r) = memo.get(&r.offset) {
            return Ok(r.clone());
        }
        let mut node = self.get(r, &mut st.stats)?;
        match &mut node {
            Node::Leaf { entries } => {
                for e in entries {
                    e.receipt = self.materialize(st, &e.receipt, memo)?;
                }
            }
            Node::Radix { left, right, .. } => {
                *left = self.materialize(st, left, memo)?;
                *right = self.materialize(st, right, memo)?;
            }
            Node::Btree { children, .. } | Node::Sequence { children, .. } => {
                for child in children {
                    *child = self.materialize(st, child, memo)?;
                }
            }
            Node::Manifest {
                authored,
                map,
                sequence,
            } => {
                *sequence = self.materialize(st, sequence, memo)?;
                *map = self.materialize(st, map, memo)?;
                *authored = self.materialize(st, authored, memo)?;
            }
            Node::Root {
                map,
                sequence,
                manifest,
                ..
            } => {
                *sequence = self.materialize(st, sequence, memo)?;
                *map = self.materialize(st, map, memo)?;
                *manifest = self.materialize(st, manifest, memo)?;
            }
            Node::Receipt { .. } | Node::Authored { .. } => {}
        }
        let result = st.put(&node)?;
        memo.insert(r.offset, result.clone());
        Ok(result)
    }
}
fn open(dir: &Path, mode: Mode) -> Result<Store> {
    let mut st = Store::open(dir)?;
    st.packed = Some(Backend::open(dir, mode, &mut st.stats)?);
    Ok(st)
}
#[derive(Serialize, Deserialize)]
struct AcceptedGroup {
    old: Head,
    ids: Vec<String>,
    requests: Vec<String>,
    first_ordinal: u64,
    reserved_bytes: u64,
    original_pack_len: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchCut {
    None,
    AfterJournal,
    BeforeHead,
    AfterHead,
}
fn batch(st: &mut Store, session: &Session, count: usize) -> Result<(u128, u128, usize)> {
    batch_inner(
        st,
        session,
        count,
        BatchCut::None,
        (count as u64 + 1) * RESERVE,
        false,
    )
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep finite accepted-group replay and publication ordering visible together"
)]
fn batch_inner(
    st: &mut Store,
    session: &Session,
    count: usize,
    cut: BatchCut,
    budget: u64,
    recover: bool,
) -> Result<(u128, u128, usize)> {
    ensure(count > 0 && count <= 32, "fixture batch count bound")?;
    let head = st.head()?;
    ensure(
        session.trusted_head.borrow().as_ref() == Some(&head),
        "batch base changed",
    )?;
    let pending = st.dir.join("accepted-group");
    let original: Option<AcceptedGroup> = if pending.exists() {
        ensure(
            recover,
            "pending accepted group requires explicit reconciliation",
        )?;
        Some(
            serde_json::from_slice(&fs::read(&pending).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    if let Some(group) = &original {
        ensure(
            group.ids.len() == count && group.requests.len() == count,
            "recovery group mismatch",
        )?;
        if head != group.old {
            let candidate: Head = serde_json::from_slice(
                &fs::read(st.dir.join("batch-candidate")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            ensure(head == candidate, "unrelated batch HEAD")?;
            let r = root(st, &head.active)?;
            for (i, (id, req)) in group.ids.iter().zip(&group.requests).enumerate() {
                let entry = lookup(st, &r.map, id, r.kind)?
                    .ok_or_else(|| "published batch operation missing".to_owned())?;
                ensure(
                    entry.request == *req && entry.ordinal == group.first_ordinal + i as u64,
                    "published batch mismatch",
                )?;
                match st.get(&entry.receipt)? {
                    Node::Receipt {
                        id: actual,
                        request,
                        ordinal,
                    } => ensure(
                        actual == *id && request == *req && ordinal == entry.ordinal,
                        "original receipt mismatch",
                    )?,
                    _ => return Err("receipt kind".to_owned()),
                }
            }
            st.sync()?;
            structural(st)?;
            st.envelope("batch-ack", &head)?;
            fs::remove_file(pending).map_err(|e| e.to_string())?;
            fs::remove_file(st.dir.join("batch-candidate")).map_err(|e| e.to_string())?;
            st.sync_dir()?;
            st.reserve = None;
            return Ok((0, 0, 0));
        }
    }
    let old = root(st, &head.active)?;
    let authored = authored(st, &old)?;
    let group = original.unwrap_or_else(|| {
        let ids: Vec<_> = (1..=count).map(|i| id(old.count + i as u64)).collect();
        let requests = ids.iter().map(|id| request(id)).collect();
        AcceptedGroup {
            old: head.clone(),
            ids,
            requests,
            first_ordinal: old.count + 1,
            reserved_bytes: budget,
            original_pack_len: st.size(),
        }
    });
    // Finite fixture cap: each synthetic operation reserves the original 2 MiB
    // cap plus one group allowance. It is not a qualified free-space allocator.
    let required = (count as u64 + 1) * RESERVE;
    ensure(
        group.reserved_bytes >= required && budget >= required,
        "batch reserve preflight refused",
    )?;
    let consumed = st
        .size()
        .checked_sub(group.original_pack_len)
        .ok_or_else(|| "batch storage continuity".to_owned())?;
    ensure(
        consumed < group.reserved_bytes,
        "unresolved batch exhausted original reserve",
    )?;
    st.reserve = Some(group.reserved_bytes - consumed);
    let ids = group.ids.clone();
    let requests = group.requests.clone();
    // No receipt is advertised as accepted before this finite group's barrier.
    let start = Instant::now();
    st.envelope("accepted-group", &group)?;
    st.sync_dir()?;
    let accepted_us = start.elapsed().as_micros();
    if cut == BatchCut::AfterJournal {
        return Err("injected after journal acceptance".to_owned());
    }
    let checkpoint = Instant::now();
    st.staging = Some(Staging::new());
    let mut map = old.map.clone();
    let mut sequence = old.sequence.clone();
    for (i, (id, req)) in ids.iter().zip(&requests).enumerate() {
        ensure(
            lookup(st, &map, id, old.kind)?.is_none(),
            "batch duplicate ID",
        )?;
        let ordinal = old.count + i as u64 + 1;
        let receipt = st.put(&Node::Receipt {
            id: id.clone(),
            request: req.clone(),
            ordinal,
        })?;
        let entry = Entry {
            key: key(id),
            id: id.clone(),
            request: req.clone(),
            ordinal,
            receipt: receipt.clone(),
        };
        map = match old.kind {
            Kind::Radix => radix_insert(st, &map, entry)?,
            Kind::Btree => btree_insert(st, &map, entry)?,
        };
        sequence = seq_append(st, &sequence, receipt)?;
    }
    // Temporary pointers are replaced while materializing final reachable nodes;
    // every original receipt survives through the dense ordinal sequence.
    let candidate = make_root(
        st,
        old.kind,
        old.count + count as u64,
        map,
        sequence,
        authored,
    )?;
    let staged = st.staging.take().unwrap();
    let staged_nodes = staged.nodes.len();
    st.changed = Some(Vec::new());
    let active = staged.materialize(st, &candidate, &mut HashMap::new())?;
    let new = root(st, &active)?;
    prefix(st, &old.sequence, &new.sequence)?;
    st.sync()?;
    for r in st.changed.take().unwrap_or_default() {
        st.get(&r)?;
    }
    ensure(st.head()? == head, "batch HEAD changed")?;
    let next = Head {
        active,
        recovery: head.active,
    };
    st.envelope("batch-candidate", &next)?;
    st.sync_dir()?;
    if cut == BatchCut::BeforeHead {
        return Err("injected after pack before HEAD".to_owned());
    }
    st.publish(&next)?;
    *session.trusted_head.borrow_mut() = Some(next.clone());
    if cut == BatchCut::AfterHead {
        return Err("injected after HEAD before batch ack".to_owned());
    }
    structural(st)?;
    st.envelope("batch-ack", &next)?;
    fs::remove_file(st.dir.join("accepted-group")).map_err(|e| e.to_string())?;
    fs::remove_file(st.dir.join("batch-candidate")).map_err(|e| e.to_string())?;
    st.sync_dir()?;
    st.reserve = None;
    Ok((accepted_us, checkpoint.elapsed().as_micros(), staged_nodes))
}
#[allow(
    clippy::cast_precision_loss,
    reason = "Reporting ratios of tiny bounded fixture counters"
)]
fn policy(st: &mut Store, session: &Session, size: usize) -> Result<serde_json::Value> {
    st.stats = Stats::default();
    let before = st.size();
    let before_allocated = st.allocated();
    let mut accept = Vec::new();
    let mut checkpoint = Vec::new();
    let mut total = Vec::new();
    let mut staged = 0;
    for _ in 0..OPERATIONS / size {
        let t = Instant::now();
        let (a, c, s) = batch(st, session, size)?;
        accept.push(a);
        checkpoint.push(c);
        total.push(t.elapsed().as_micros());
        staged += s;
    }
    Ok(
        json!({"max_batch_operations":size,"operations":OPERATIONS,"groups":OPERATIONS/size,
        "modeled_max_wait_ms":50,"queue_wait_measured":false,
        "fixture_reserved_bytes_per_group":(size as u64+1)*RESERVE,
        "filesystem_allocated_delta":i128::from(st.allocated())-i128::from(before_allocated),
        "total_process_write_bytes":st.stats.object_write_bytes+st.stats.envelope_write_bytes+st.stats.placement_write_bytes+st.stats.relocation_write_bytes,
        "sync_calls_per_operation":st.stats.sync_calls as f64/OPERATIONS as f64,
        "journal_group_acceptance":quantiles(accept),"checkpoint_after_journal":quantiles(checkpoint),"whole_group":quantiles(total),
        "virtual_pack_bytes_appended":st.size()-before,"staged_nodes":staged,"counters":st.stats}),
    )
}
fn fresh_open(dir: &Path, mode: Mode) -> Result<serde_json::Value> {
    let mut times = Vec::new();
    let mut counters = Vec::new();
    for _ in 0..8 {
        let t = Instant::now();
        let mut st = open(dir, mode)?;
        structural(&mut st)?;
        times.push(t.elapsed().as_micros());
        counters.push(st.stats);
    }
    Ok(json!({"latency":quantiles(times),"counters":counters,"os_cache_evicted":false}))
}
fn run(n: u64, kind: Kind, mode: Mode) -> Result<serde_json::Value> {
    let temp = tempfile::Builder::new()
        .prefix("photara-ps2-packing-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let mut st = open(temp.path(), mode)?;
    let t = Instant::now();
    build(&mut st, n, kind)?;
    let build_ms = t.elapsed().as_millis();
    let mut session = Session::imported();
    let t = Instant::now();
    session.audit(&mut st)?;
    let baseline_audit_ms = t.elapsed().as_millis();
    let baseline = st.size();
    let baseline_allocated = st.allocated();
    let mut policies = Vec::new();
    for size in [1, 8, 32] {
        policies.push(policy(&mut st, &session, size)?);
    }
    let before_move_open = fresh_open(temp.path(), mode)?;
    let retry = measure(&mut st, 32, |st, i| {
        let op = id(1 + (i as u64 * 7919) % n);
        session.accept(st, &op, &request(&op), Cut::None, RESERVE)?;
        Ok(())
    })?;
    st.stats = Stats::default();
    st.seen.clear();
    let t = Instant::now();
    let selected = structural(&mut st)?;
    st.seen.clear();
    audit_root(&mut st, &selected.active)?;
    let active_bytes: u64 = st.seen.iter().map(|r| r.len).sum();
    let before_recovery = st.stats.clone();
    let rt = Instant::now();
    audit_root(&mut st, &selected.recovery)?;
    let recovery_audit_ms = rt.elapsed().as_millis();
    let recovery_audit_stats = json!({"object_reads":st.stats.object_reads-before_recovery.object_reads,
        "object_read_bytes":st.stats.object_read_bytes-before_recovery.object_read_bytes,
        "placement_reads":st.stats.placement_reads-before_recovery.placement_reads,
        "placement_read_bytes":st.stats.placement_read_bytes-before_recovery.placement_read_bytes});
    let union_bytes: u64 = st.seen.iter().map(|r| r.len).sum();
    let live = st.seen.clone();
    let backend = st.packed.take().unwrap();
    let plan = backend.plan(&live);
    let planning_ms = t.elapsed().as_millis();
    let planning_stats = st.stats.clone();
    let before_bytes = backend.physical_bytes();
    st.stats = Stats::default();
    let t = Instant::now();
    let mut compaction = backend.compact(&plan, 2 * SEGMENT, MoveCut::None, &mut st.stats)?;
    let compact_ms = t.elapsed().as_millis();
    compaction["data_bytes_before"] = json!(before_bytes);
    compaction["data_bytes_after"] = json!(backend.physical_bytes());
    let compact_stats = st.stats.clone();
    st.packed = Some(backend);
    let after_move_open = fresh_open(temp.path(), mode)?;
    st.seen.clear();
    let t = Instant::now();
    full_audit(&mut st)?;
    let final_audit_ms = t.elapsed().as_millis();
    let turnover = measure(&mut st, 8, |st, _| session.turnover(st))?;
    let shared_open = fresh_open(temp.path(), mode)?;
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"packing":format!("{mode:?}"),"fully_materialized":true,
        "build_ms":build_ms,"baseline_audit_ms":baseline_audit_ms,"baseline_pack_bytes":baseline,"baseline_file_allocation":baseline_allocated,
        "policies":policies,"retry":retry,"root_turnover":turnover,"before_compaction_open":before_move_open,
        "compaction_planning_ms":planning_ms,"compaction_planning_counters":planning_stats,"compaction":compaction,
        "compaction_ms":compact_ms,"compaction_counters":compact_stats,"after_compaction_open":after_move_open,
        "final_audit_ms":final_audit_ms,"independent_recovery_audit_ms":recovery_audit_ms,"independent_recovery_audit_counters":recovery_audit_stats,
        "active_unique_object_bytes":active_bytes,"active_recovery_union_object_bytes":union_bytes,
        "independent_recovery_extra_object_bytes":union_bytes-active_bytes,"shared_index_open_after_turnover":shared_open,
        "final_physical_data_bytes":st.packed.as_ref().unwrap().physical_bytes(),"final_file_allocation":st.allocated()}),
    )
}
#[allow(
    clippy::too_many_lines,
    reason = "Explicit disposable interruption and recovery matrix"
)]
fn faults() -> Result<Vec<String>> {
    let mut results = Vec::new();
    for mode in [Mode::Sealed, Mode::Relocatable] {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                BatchCut::AfterJournal,
                BatchCut::BeforeHead,
                BatchCut::AfterHead,
            ] {
                let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
                let mut st = open(temp.path(), mode)?;
                build(&mut st, 32, kind)?;
                let mut session = Session::imported();
                session.audit(&mut st)?;
                let initial = st.head()?;
                let original_size = st.size();
                expect_failure(
                    batch_inner(&mut st, &session, 8, BatchCut::None, 0, false),
                    "batch reserve preflight",
                )?;
                ensure(
                    st.head()? == initial
                        && st.size() == original_size
                        && !st.dir.join("accepted-group").exists(),
                    "reserve refusal accepted work",
                )?;
                expect_failure(
                    batch_inner(&mut st, &session, 8, cut, 9 * RESERVE, false),
                    "batch cut",
                )?;
                drop(st);
                let mut st = open(temp.path(), mode)?;
                let mut session = Session::imported();
                session.audit(&mut st)?;
                let h = st.head()?;
                let count = root(&mut st, &h.active)?.count;
                ensure(
                    count == if cut == BatchCut::AfterHead { 40 } else { 32 },
                    "batch cut selected wrong count",
                )?;
                expect_failure(
                    batch(&mut st, &session, 8),
                    "fresh batch while accepted group unresolved",
                )?;
                batch_inner(&mut st, &session, 8, BatchCut::None, 9 * RESERVE, true)?;
                let h = st.head()?;
                let r = root(&mut st, &h.active)?;
                ensure(r.count == 40, "batch recovery duplicated or lost ordinals")?;
                for ordinal in 33..=40 {
                    let op = id(ordinal);
                    let entry = lookup(&mut st, &r.map, &op, kind)?
                        .ok_or_else(|| "recovered ID missing".to_owned())?;
                    ensure(
                        entry.ordinal == ordinal && entry.request == request(&op),
                        "recovered original identity changed",
                    )?;
                }
                full_audit(&mut st)?;
                ensure(
                    !st.dir.join("accepted-group").exists(),
                    "batch recovery incomplete",
                )?;
            }
        }
    }
    results.push(
        "both_maps_and_storage_modes_batch_reserve_and_three_original_id_recovery_cuts".to_owned(),
    );
    for cut in [
        MoveCut::PartialCopy,
        MoveCut::BeforePointer,
        MoveCut::AfterPointer,
    ] {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut st = open(temp.path(), Mode::Relocatable)?;
        build(&mut st, 32, Kind::Radix)?;
        let mut session = Session::imported();
        session.audit(&mut st)?;
        // Registered unselected fixture objects create one mostly dead sealed segment.
        for _ in 0..40 {
            st.put(&Node::Authored {
                value: "x".repeat(32768),
            })?;
        }
        st.sync()?;
        st.seen.clear();
        full_audit(&mut st)?;
        let live = st.seen.clone();
        let head = st.head()?;
        let backend = st.packed.take().unwrap();
        let plan = backend.plan(&live);
        expect_failure(
            backend.compact(&plan, 0, MoveCut::None, &mut st.stats),
            "relocation reserve",
        )?;
        expect_failure(
            backend.compact(&plan, 2 * SEGMENT, cut, &mut st.stats),
            "relocation cut",
        )?;
        st.packed = Some(backend);
        drop(st);
        let mut st = open(temp.path(), Mode::Relocatable)?;
        structural(&mut st)?;
        ensure(st.head()? == head, "placement changed semantic HEAD")?;
        let backend = st.packed.take().unwrap();
        if cut == MoveCut::AfterPointer {
            let intent: MoveIntent = serde_json::from_slice(
                &fs::read(backend.dir.join("compaction-intent")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            fs::write(
                backend.dir.join(Backend::pointer_name(intent.segment)),
                b"torn placement pointer",
            )
            .map_err(|e| e.to_string())?;
            expect_failure(
                backend.recover_move(&plan, &mut st.stats),
                "torn pointer must refuse",
            )?;
            ensure(
                backend
                    .data_path(intent.segment, intent.old.generation)
                    .exists()
                    && backend
                        .data_path(intent.segment, intent.new.generation)
                        .exists(),
                "unknown placement discarded a generation",
            )?;
            atomic(
                &backend.dir,
                &Backend::pointer_name(intent.segment),
                &intent.new,
                &mut st.stats,
            )?;
        }
        backend.recover_move(&plan, &mut st.stats)?;
        st.packed = Some(backend);
        full_audit(&mut st)?;
        audit_root(&mut st, &head.recovery)?;
        results.push(
            match cut {
                MoveCut::PartialCopy => "partial_copy_old_generation_retained",
                MoveCut::BeforePointer => "before_pointer_originals_retained",
                MoveCut::AfterPointer => "after_pointer_new_generation_reconciled",
                MoveCut::None => unreachable!(),
            }
            .to_owned(),
        );
    }
    results.push("compaction_capacity_refused_without_retirement_credit".to_owned());
    Ok(results)
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    let counts = if args.is_empty() {
        vec![1_000, 10_000, 100_000]
    } else {
        args.iter()
            .map(|s| s.parse::<u64>().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>>>()?
    };
    println!(
        "{}",
        json!({"fixture":"bounded packed batch comparison","segment_bytes":SEGMENT,"relocation_block_bytes":BLOCK,
        "locator_entries_per_segment":SLOTS,"batches":[1,8,32],"operations_per_policy":OPERATIONS,
        "caveats":["placement pointer is a fixture-only second physical dispatch requiring architecture review",
            "stable virtual offset plus SHA/length, not hash-only CAS or byte deduplication",
            "full-audit frozen-root liveness planning; no bounded persistent incremental GC",
            "relocation copies at most two segments; 4 KiB blocks can retain dead bytes",
            "50 ms batch deadline is modeled, not a scheduler or interaction latency benchmark",
            "batch journal accepted before checkpoint; no production Saved claim",
            "journal replays synthetic index/dedupe operations only; no authored Graph mutation payload",
            "persisted batch reserve is a fixture cap, not a qualified physical free-space allocator",
            "cap debits object/envelope bytes; placement metadata and filesystem allocation need a separate production reserve proof",
            "allocation snapshots enumerate fixture files outside timed operations; data read counters are file requests, not device I/O telemetry",
            "small synthetic receipts, OS-cache-retaining open, no power-loss qualification"]})
    );
    println!("{}", json!({"packing_faults_passed":faults()?}));
    for n in counts {
        for mode in [Mode::Sealed, Mode::Relocatable] {
            for kind in [Kind::Radix, Kind::Btree] {
                println!("{}", run(n, kind, mode)?);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn batch_and_placement_fault_matrix() -> super::Result<()> {
        let passed = super::faults()?;
        super::ensure(passed.len() == 5, "packing fault matrix incomplete")
    }
}
