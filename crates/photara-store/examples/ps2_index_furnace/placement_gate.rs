//! Disposable HEAD-selected placement experiment, not a package format.
//! Direct physical metadata refs deliberately expose the bootstrap-arena gate.
#![allow(
    clippy::ref_option,
    reason = "Disposable persistent-trie recursion shares owned optional snapshot roots"
)]
#[allow(clippy::wildcard_imports, reason = "Private disposable companion")]
use super::*;
use std::collections::VecDeque;

const SEGMENT: u64 = 1024 * 1024;
const BLOCK: u64 = 4096;
const CACHE: usize = 32;
const WORK: usize = 32;
const NEW_OBJECT_LIMIT: usize = 2048;

#[derive(Clone, Debug, Default, Serialize)]
struct Metrics {
    metadata_reads: u64,
    metadata_read_bytes: u64,
    metadata_writes: u64,
    metadata_write_bytes: u64,
    data_write_bytes: u64,
    envelope_write_bytes: u64,
    data_read_bytes: u64,
    cache_hits: u64,
    sync_calls: u64,
    reserve_charged: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Page {
    depth: u8,
    prefix: u64,
    children: Vec<Option<Ref>>,
    value: Option<serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Life {
    object: Ref,
    incoming: u64,
    children: Vec<Ref>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Locator {
    segment: u64,
    generation: u64,
    slots: Vec<Option<u64>>,
    physical_len: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Selection {
    semantic: Head,
    active_locator: Option<Ref>,
    recovery_locator: Option<Ref>,
    life: Option<Ref>,
    blocks: Option<Ref>,
    queue: Option<Ref>,
    queue_first: u64,
    queue_last: u64,
    candidate_cursor: u64,
    segments: u64,
    epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    None,
    BeforeHead,
    AfterHead,
}

struct Gate {
    dir: PathBuf,
    meta: File,
    selected: Selection,
    cache: VecDeque<(String, u64, Locator)>,
    stats: Metrics,
    remaining: Option<u64>,
    trusted: bool,
}

fn rounded(n: u64) -> Result<u64> {
    n.checked_add(BLOCK - 1)
        .map(|v| v / BLOCK * BLOCK)
        .ok_or("allocation overflow".into())
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
impl Gate {
    fn create(dir: &Path, semantic: Head) -> Result<Self> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let meta = OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(dir.join("bootstrap-arena"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            dir: dir.to_owned(),
            meta,
            selected: Selection {
                semantic,
                active_locator: None,
                recovery_locator: None,
                life: None,
                blocks: None,
                queue: None,
                queue_first: 0,
                queue_last: 0,
                candidate_cursor: 0,
                segments: 0,
                epoch: 0,
            },
            cache: VecDeque::new(),
            stats: Metrics::default(),
            remaining: None,
            trusted: true,
        })
    }
    fn reopen(dir: &Path) -> Result<Self> {
        let selected: Selection =
            serde_json::from_slice(&fs::read(dir.join("HEAD")).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let meta = OpenOptions::new()
            .read(true)
            .append(true)
            .open(dir.join("bootstrap-arena"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            dir: dir.to_owned(),
            meta,
            selected,
            cache: VecDeque::new(),
            stats: Metrics::default(),
            remaining: None,
            trusted: false,
        })
    }
    fn charge(&mut self, n: u64) -> Result<()> {
        // Deliberately conservative: each write charged an entire allocation-unit
        // rounding, even when packed into already charged arena blocks.
        let n = rounded(n)?;
        if let Some(left) = &mut self.remaining {
            ensure(*left >= n, "fixture physical reserve exhausted")?;
            *left -= n;
        }
        self.stats.reserve_charged += n;
        Ok(())
    }
    fn sync(&mut self) -> Result<()> {
        self.meta.sync_all().map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        Ok(())
    }
    fn put<T: Serialize>(&mut self, value: &T) -> Result<Ref> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        ensure(bytes.len() <= 16_384, "metadata page size")?;
        self.charge(bytes.len() as u64)?;
        let offset = self
            .meta
            .seek(SeekFrom::End(0))
            .map_err(|e| e.to_string())?;
        self.meta.write_all(&bytes).map_err(|e| e.to_string())?;
        self.stats.metadata_writes += 1;
        self.stats.metadata_write_bytes += bytes.len() as u64;
        Ok(Ref {
            offset,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        })
    }
    fn get<T: serde::de::DeserializeOwned>(&mut self, r: &Ref) -> Result<T> {
        ensure(r.len > 0 && r.len <= 16_384, "metadata ref bound")?;
        ensure(
            r.offset
                .checked_add(r.len)
                .is_some_and(|n| n <= self.meta.metadata().unwrap().len()),
            "missing metadata range",
        )?;
        let mut bytes = vec![0; usize::try_from(r.len).unwrap()];
        self.meta
            .seek(SeekFrom::Start(r.offset))
            .and_then(|_| self.meta.read_exact(&mut bytes))
            .map_err(|e| e.to_string())?;
        self.stats.metadata_reads += 1;
        self.stats.metadata_read_bytes += r.len;
        ensure(hash(&bytes) == r.sha, "metadata digest mismatch")?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    fn page(&mut self, r: &Ref, depth: u8, prefix: u64) -> Result<Page> {
        let p: Page = self.get(r)?;
        ensure(
            p.depth == depth && p.prefix == prefix,
            "metadata trie route",
        )?;
        ensure(
            if depth == 16 {
                p.children.is_empty() && p.value.is_some()
            } else {
                p.children.len() == 16 && p.value.is_none()
            },
            "metadata trie shape",
        )?;
        Ok(p)
    }
    fn lookup<T: serde::de::DeserializeOwned>(
        &mut self,
        root: &Option<Ref>,
        key: u64,
    ) -> Result<Option<T>> {
        let mut at = root.clone();
        let mut prefix = 0;
        for depth in 0..=16 {
            let Some(r) = &at else { return Ok(None) };
            let p = self.page(r, depth, prefix)?;
            if depth == 16 {
                return p
                    .value
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|e| e.to_string());
            }
            let digit = ((key >> (60 - depth * 4)) & 15) as usize;
            prefix = (prefix << 4) | digit as u64;
            at.clone_from(&p.children[digit]);
        }
        Err("metadata trie depth".into())
    }
    fn update(
        &mut self,
        root: &Option<Ref>,
        depth: u8,
        prefix: u64,
        changes: &[(u64, Option<serde_json::Value>)],
    ) -> Result<Option<Ref>> {
        if changes.is_empty() {
            return Ok(root.clone());
        }
        if depth == 16 {
            return changes[0]
                .1
                .as_ref()
                .map(|value| {
                    self.put(&Page {
                        depth,
                        prefix,
                        children: vec![],
                        value: Some(value.clone()),
                    })
                })
                .transpose();
        }
        let mut children = if let Some(r) = root {
            self.page(r, depth, prefix)?.children
        } else {
            vec![None; 16]
        };
        for (digit, child) in children.iter_mut().enumerate() {
            let selected: Vec<_> = changes
                .iter()
                .filter(|(key, _)| ((key >> (60 - depth * 4)) & 15) == digit as u64)
                .cloned()
                .collect();
            *child = self.update(child, depth + 1, (prefix << 4) | digit as u64, &selected)?;
        }
        if children.iter().all(Option::is_none) {
            return Ok(None);
        }
        self.put(&Page {
            depth,
            prefix,
            children,
            value: None,
        })
        .map(Some)
    }
    fn apply<T: Serialize>(
        &mut self,
        root: &Option<Ref>,
        changes: BTreeMap<u64, Option<T>>,
    ) -> Result<Option<Ref>> {
        let changes = changes
            .into_iter()
            .map(|(k, v)| {
                Ok((
                    k,
                    v.map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| e.to_string())?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.update(root, 0, 0, &changes)
    }
    fn collect(
        &mut self,
        root: &Option<Ref>,
        depth: u8,
        prefix: u64,
        values: &mut BTreeMap<u64, serde_json::Value>,
    ) -> Result<()> {
        let Some(r) = root else { return Ok(()) };
        let page = self.page(r, depth, prefix)?;
        if depth == 16 {
            ensure(
                values.insert(prefix, page.value.unwrap()).is_none(),
                "duplicate metadata key",
            )?;
            return Ok(());
        }
        for (digit, child) in page.children.iter().enumerate() {
            self.collect(child, depth + 1, (prefix << 4) | digit as u64, values)?;
        }
        Ok(())
    }
    fn audit_counts(&mut self, source: &mut Store) -> Result<()> {
        let selected = self.selected.clone();
        let mut values = BTreeMap::new();
        self.collect(&selected.life, 0, 0, &mut values)?;
        let lives = values
            .into_iter()
            .map(|(k, v)| {
                serde_json::from_value::<Life>(v)
                    .map(|v| (k, v))
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut incoming = BTreeMap::<u64, u64>::new();
        let mut blocks = BTreeMap::<u64, Vec<u64>>::new();
        for root in [&selected.semantic.active, &selected.semantic.recovery] {
            *incoming.entry(root.offset).or_default() += 1;
        }
        for (key, life) in &lives {
            ensure(*key == life.object.offset, "liveness key mismatch")?;
            ensure(
                edges(source.get(&life.object)?) == life.children,
                "liveness edge mismatch",
            )?;
            for child in &life.children {
                ensure(
                    lives.get(&child.offset).is_some_and(|l| l.object == *child),
                    "liveness child identity",
                )?;
                *incoming.entry(child.offset).or_default() += 1;
            }
            for block in
                life.object.offset / BLOCK..=(life.object.offset + life.object.len - 1) / BLOCK
            {
                blocks.entry(block / 256).or_insert_with(|| vec![0; 256])
                    [(block % 256) as usize] += 1;
            }
        }
        let mut queue_values = BTreeMap::new();
        self.collect(&selected.queue, 0, 0, &mut queue_values)?;
        ensure(
            queue_values.len() as u64 == selected.queue_last - selected.queue_first,
            "retirement queue gap",
        )?;
        let mut queued = HashSet::new();
        for (key, value) in queue_values {
            ensure(
                key >= selected.queue_first && key < selected.queue_last,
                "queue ordinal range",
            )?;
            let r: Ref = serde_json::from_value(value).map_err(|e| e.to_string())?;
            queued.insert(r);
        }
        for (key, life) in &lives {
            ensure(
                life.incoming == incoming.get(key).copied().unwrap_or_default(),
                "liveness incoming mismatch",
            )?;
            if life.incoming == 0 {
                ensure(queued.contains(&life.object), "untracked zero-count object")?;
            }
        }
        let mut stored = BTreeMap::new();
        self.collect(&selected.blocks, 0, 0, &mut stored)?;
        for (segment, value) in stored {
            let actual: Vec<u64> = serde_json::from_value(value).map_err(|e| e.to_string())?;
            ensure(
                actual == blocks.remove(&segment).unwrap_or(vec![0; 256]),
                "live-block mismatch",
            )?;
        }
        ensure(blocks.is_empty(), "missing live-block page")?;
        audit_placement(self, source)?;
        self.trusted = true;
        Ok(())
    }
    fn publish(&mut self, next: Selection, cut: Cut) -> Result<()> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        ensure(next.epoch == self.selected.epoch + 1, "selection epoch")?;
        if self.dir.join("HEAD").exists() {
            let observed: Selection = serde_json::from_slice(
                &fs::read(self.dir.join("HEAD")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            ensure(
                serde_json::to_vec(&observed).unwrap()
                    == serde_json::to_vec(&self.selected).unwrap(),
                "HEAD changed under physical transaction",
            )?;
        }
        let bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
        self.charge(bytes.len() as u64)?;
        self.charge(BLOCK)?; // explicit directory-entry/rename fixture allowance
        let mut f = File::create(self.dir.join("HEAD.pending")).map_err(|e| e.to_string())?;
        f.write_all(&bytes).map_err(|e| e.to_string())?;
        self.stats.envelope_write_bytes += bytes.len() as u64;
        self.sync()?;
        f.sync_all().map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        if cut == Cut::BeforeHead {
            return Err("injected before HEAD".into());
        }
        fs::rename(self.dir.join("HEAD.pending"), self.dir.join("HEAD"))
            .map_err(|e| e.to_string())?;
        self.selected = next;
        if cut == Cut::AfterHead {
            return Err("injected after HEAD before directory barrier".into());
        }
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        Ok(())
    }
    fn locator(&mut self, root: &Option<Ref>, segment: u64, cached: bool) -> Result<Locator> {
        let epoch = root.as_ref().ok_or("missing locator root")?.sha.clone();
        if cached
            && let Some(index) = self
                .cache
                .iter()
                .position(|(e, s, _)| e == &epoch && *s == segment)
        {
            let hit = self.cache.remove(index).unwrap();
            let value = hit.2.clone();
            self.cache.push_back(hit);
            self.stats.cache_hits += 1;
            return Ok(value);
        }
        let loc: Locator = self
            .lookup(root, segment)?
            .ok_or("missing locator segment")?;
        ensure(
            loc.segment == segment && loc.slots.len() == 256,
            "locator shape",
        )?;
        let mut used = HashSet::new();
        for slot in loc.slots.iter().flatten() {
            ensure(
                slot % BLOCK == 0
                    && slot
                        .checked_add(BLOCK)
                        .is_some_and(|n| n <= loc.physical_len)
                    && used.insert(slot),
                "locator range or alias",
            )?;
        }
        if cached {
            if self.cache.len() == CACHE {
                self.cache.pop_front();
            }
            self.cache.push_back((epoch, segment, loc.clone()));
        }
        Ok(loc)
    }
    fn data_path(&self, segment: u64, generation: u64) -> PathBuf {
        self.dir.join(format!("data-{segment}-{generation}"))
    }
    fn read_object(&mut self, root: &Option<Ref>, r: &Ref, cached: bool) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut at = r.offset;
        let end = at.checked_add(r.len).ok_or("object overflow")?;
        while at < end {
            let segment = at / SEGMENT;
            let loc = self.locator(root, segment, cached)?;
            let mut file =
                File::open(self.data_path(segment, loc.generation)).map_err(|e| e.to_string())?;
            let limit = end.min((segment + 1) * SEGMENT);
            while at < limit {
                let local = at % SEGMENT;
                let physical =
                    loc.slots[(local / BLOCK) as usize].ok_or("live block absent")? + local % BLOCK;
                let count = (limit - at).min(BLOCK - local % BLOCK);
                let mut bytes = vec![0; usize::try_from(count).unwrap()];
                file.seek(SeekFrom::Start(physical))
                    .and_then(|_| file.read_exact(&mut bytes))
                    .map_err(|e| e.to_string())?;
                self.stats.data_read_bytes += count;
                result.extend(bytes);
                at += count;
            }
        }
        ensure(
            hash(&result) == r.sha,
            "semantic object digest after placement",
        )?;
        Ok(result)
    }
    fn write_segment(
        &mut self,
        segment: u64,
        generation: u64,
        bytes: &[u8],
        slots: Vec<Option<u64>>,
    ) -> Result<Locator> {
        self.charge(bytes.len() as u64)?;
        self.charge(BLOCK)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.data_path(segment, generation))
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        self.stats.data_write_bytes += bytes.len() as u64;
        Ok(Locator {
            segment,
            generation,
            slots,
            physical_len: bytes.len() as u64,
        })
    }
    fn begin_reserve(&mut self, bytes: u64) -> Result<()> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        ensure(
            bytes >= 2 * BLOCK,
            "reserve refused before durable liability",
        )?;
        ensure(
            !self.dir.join("liability").exists(),
            "unresolved physical reserve liability",
        )?;
        self.remaining = Some(bytes);
        let value=serde_json::to_vec(&json!({"epoch":self.selected.epoch,"head_sha":hash(&serde_json::to_vec(&self.selected).unwrap()),"reserved_bytes":bytes,"exclusive_os_reservation":false})).unwrap();
        self.charge(value.len() as u64)?;
        self.charge(BLOCK)?;
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.dir.join("liability"))
            .map_err(|e| e.to_string())?;
        f.write_all(&value)
            .and_then(|()| f.sync_all())
            .map_err(|e| e.to_string())?;
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.envelope_write_bytes += value.len() as u64;
        self.stats.sync_calls += 2;
        Ok(())
    }
    fn finish_reserve(&mut self) -> Result<()> {
        self.charge(BLOCK)?;
        fs::remove_file(self.dir.join("liability")).map_err(|e| e.to_string())?;
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        self.remaining = None;
        Ok(())
    }
}

fn block_delta(
    changes: &mut BTreeMap<u64, Option<Vec<u64>>>,
    gate: &mut Gate,
    root: &Option<Ref>,
    r: &Ref,
    add: bool,
) -> Result<()> {
    for block in r.offset / BLOCK..=(r.offset + r.len - 1) / BLOCK {
        let segment = block / 256;
        if let std::collections::btree_map::Entry::Vacant(entry) = changes.entry(segment) {
            entry.insert(Some(gate.lookup(root, segment)?.unwrap_or(vec![0; 256])));
        }
        let values = changes.get_mut(&segment).unwrap().as_mut().unwrap();
        let count = &mut values[(block % 256) as usize];
        *count = if add {
            count.checked_add(1)
        } else {
            count.checked_sub(1)
        }
        .ok_or("live block counter overflow/underflow")?;
    }
    Ok(())
}
fn life_value(
    gate: &mut Gate,
    root: &Option<Ref>,
    changes: &BTreeMap<u64, Option<Life>>,
    r: &Ref,
) -> Result<Option<Life>> {
    let life = if let Some(value) = changes.get(&r.offset) {
        value.clone()
    } else {
        gate.lookup::<Life>(root, r.offset)?
    };
    if let Some(value) = &life {
        ensure(value.object == *r, "logical identity alias")?;
    }
    Ok(life)
}
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit isolated fixture transaction"
)]
fn register(
    gate: &mut Gate,
    source: &mut Store,
    r: &Ref,
    life_root: &Option<Ref>,
    blocks_root: &Option<Ref>,
    lives: &mut BTreeMap<u64, Option<Life>>,
    blocks: &mut BTreeMap<u64, Option<Vec<u64>>>,
    budget: &mut usize,
) -> Result<()> {
    if life_value(gate, life_root, lives, r)?.is_some() {
        return Ok(());
    }
    ensure(*budget > 0, "new-object transaction budget")?;
    *budget -= 1;
    let children = edges(source.get(r)?);
    lives.insert(
        r.offset,
        Some(Life {
            object: r.clone(),
            incoming: 0,
            children: children.clone(),
        }),
    );
    block_delta(blocks, gate, blocks_root, r, true)?;
    for child in children {
        register(
            gate,
            source,
            &child,
            life_root,
            blocks_root,
            lives,
            blocks,
            budget,
        )?;
        let mut value =
            life_value(gate, life_root, lives, &child)?.ok_or("missing registered child")?;
        value.incoming = value.incoming.checked_add(1).ok_or("incoming overflow")?;
        lives.insert(child.offset, Some(value));
    }
    Ok(())
}
impl Gate {
    fn bootstrap(&mut self, source: &mut Store) -> Result<()> {
        let mut lives = BTreeMap::new();
        let mut blocks = BTreeMap::new();
        // Exhaustive bootstrap is intentionally separate from bounded updates.
        let mut budget = usize::MAX;
        for r in [
            self.selected.semantic.active.clone(),
            self.selected.semantic.recovery.clone(),
        ] {
            register(
                self,
                source,
                &r,
                &None,
                &None,
                &mut lives,
                &mut blocks,
                &mut budget,
            )?;
            lives.get_mut(&r.offset).unwrap().as_mut().unwrap().incoming += 1;
        }
        let life = self.apply(&None, lives)?;
        let block_root = self.apply(&None, blocks)?;
        let segments = source.size().div_ceil(SEGMENT);
        let mut placements = BTreeMap::new();
        for segment in 0..segments {
            let used = (source.size() - segment * SEGMENT).min(SEGMENT);
            let mut bytes = vec![0; usize::try_from(rounded(used)?).unwrap()];
            source
                .file
                .seek(SeekFrom::Start(segment * SEGMENT))
                .and_then(|_| source.file.read_exact(&mut bytes[..used as usize]))
                .map_err(|e| e.to_string())?;
            let slots = (0_u64..256)
                .map(|i| (i * BLOCK < bytes.len() as u64).then_some(i * BLOCK))
                .collect();
            placements.insert(
                segment,
                Some(self.write_segment(segment, 0, &bytes, slots)?),
            );
        }
        let locator = self.apply(&None, placements)?;
        let mut next = self.selected.clone();
        next.epoch += 1;
        next.life = life;
        next.blocks = block_root;
        next.segments = segments;
        next.active_locator.clone_from(&locator);
        next.recovery_locator = locator;
        self.publish(next, Cut::None)
    }
    fn select_semantic(&mut self, source: &mut Store, new: Head) -> Result<usize> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        let old = self.selected.clone();
        let mut next = old.clone();
        let mut lives = BTreeMap::new();
        let mut blocks = BTreeMap::new();
        let mut queue = BTreeMap::new();
        let mut budget = NEW_OBJECT_LIMIT;
        for r in [&new.active, &new.recovery] {
            register(
                self,
                source,
                r,
                &old.life,
                &old.blocks,
                &mut lives,
                &mut blocks,
                &mut budget,
            )?;
        }
        for (r, add) in [
            (&new.active, true),
            (&new.recovery, true),
            (&old.semantic.active, false),
            (&old.semantic.recovery, false),
        ] {
            let mut value =
                life_value(self, &old.life, &lives, r)?.ok_or("selected root unregistered")?;
            value.incoming = if add {
                value.incoming.checked_add(1)
            } else {
                value.incoming.checked_sub(1)
            }
            .ok_or("root count underflow/overflow")?;
            lives.insert(r.offset, Some(value));
        }
        for value in lives.values().flatten().filter(|l| l.incoming == 0) {
            queue.insert(next.queue_last, Some(value.object.clone()));
            next.queue_last += 1;
        }
        next.life = self.apply(&old.life, lives)?;
        next.blocks = self.apply(&old.blocks, blocks)?;
        next.queue = self.apply(&old.queue, queue)?;
        // Copy only segments containing newly appended virtual bytes; the tail
        // may cost a full MiB. Count that cost instead of calling it free packing.
        let first = old.segments.saturating_sub(1);
        let end = source.size().div_ceil(SEGMENT);
        ensure(end - first <= 4, "bounded data publication segment budget")?;
        let mut placements = BTreeMap::new();
        for segment in first..end {
            let used = (source.size() - segment * SEGMENT).min(SEGMENT);
            let mut bytes = vec![0; usize::try_from(rounded(used)?).unwrap()];
            source
                .file
                .seek(SeekFrom::Start(segment * SEGMENT))
                .and_then(|_| source.file.read_exact(&mut bytes[..used as usize]))
                .map_err(|e| e.to_string())?;
            let slots = (0_u64..256)
                .map(|i| (i * BLOCK < bytes.len() as u64).then_some(i * BLOCK))
                .collect();
            placements.insert(
                segment,
                Some(self.write_segment(segment, next.epoch + 1, &bytes, slots)?),
            );
        }
        next.active_locator = self.apply(&old.active_locator, placements)?;
        next.recovery_locator = old.active_locator;
        next.semantic = new;
        next.segments = end;
        next.epoch += 1;
        self.publish(next, Cut::None)?;
        Ok(NEW_OBJECT_LIMIT - budget)
    }
    fn retire_step(&mut self) -> Result<usize> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        let old = self.selected.clone();
        let mut next = old.clone();
        let mut lives = BTreeMap::new();
        let mut blocks = BTreeMap::new();
        let mut queue = BTreeMap::new();
        let mut processed = 0;
        while processed < WORK && next.queue_first < old.queue_last {
            let r: Ref = self
                .lookup(&old.queue, next.queue_first)?
                .ok_or("missing retirement work")?;
            queue.insert(next.queue_first, None);
            next.queue_first += 1;
            processed += 1;
            let Some(value) = life_value(self, &old.life, &lives, &r)? else {
                continue;
            };
            if value.incoming != 0 {
                continue;
            }
            for child in &value.children {
                let mut c =
                    life_value(self, &old.life, &lives, child)?.ok_or("retirement child absent")?;
                c.incoming = c.incoming.checked_sub(1).ok_or("retirement undercount")?;
                if c.incoming == 0 {
                    queue.insert(next.queue_last, Some(child.clone()));
                    next.queue_last += 1;
                }
                lives.insert(child.offset, Some(c));
            }
            block_delta(&mut blocks, self, &old.blocks, &r, false)?;
            lives.insert(r.offset, None);
        }
        next.life = self.apply(&old.life, lives)?;
        next.blocks = self.apply(&old.blocks, blocks)?;
        next.queue = self.apply(&old.queue, queue)?;
        next.epoch += 1;
        self.publish(next, Cut::None)?;
        Ok(processed)
    }
    fn candidate(&mut self) -> Result<(u64, Vec<u64>)> {
        let segment = self.selected.candidate_cursor % self.selected.segments;
        let root = self.selected.blocks.clone();
        let blocks: Vec<u64> = self.lookup(&root, segment)?.unwrap_or(vec![0; 256]);
        ensure(blocks.len() == 256, "block count shape")?;
        Ok((segment, blocks))
    }
    fn compact_one(&mut self, cut: Cut) -> Result<serde_json::Value> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        let (segment, live) = self.candidate()?;
        let old = self.selected.clone();
        let source = self.locator(&old.active_locator, segment, true)?;
        let mut input =
            File::open(self.data_path(segment, source.generation)).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        let mut slots = vec![None; 256];
        for (i, count) in live.iter().enumerate() {
            if *count > 0 {
                let offset = source.slots[i].ok_or("liveness references absent block")?;
                let mut block = vec![0; usize::try_from(BLOCK).unwrap()];
                input
                    .seek(SeekFrom::Start(offset))
                    .and_then(|_| input.read_exact(&mut block))
                    .map_err(|e| e.to_string())?;
                self.stats.data_read_bytes += BLOCK;
                slots[i] = Some(bytes.len() as u64);
                bytes.extend(block);
            }
        }
        let loc = self.write_segment(segment, old.epoch + 1, &bytes, slots)?;
        let mut next = old.clone();
        next.active_locator = self.apply(
            &old.active_locator,
            BTreeMap::from([(segment, Some(loc.clone()))]),
        )?;
        next.epoch += 1;
        next.candidate_cursor += 1;
        // Re-reading exact captured blocks before HEAD publication establishes
        // their equality, independently of trusting write success.
        ensure(
            fs::read(self.data_path(segment, loc.generation)).map_err(|e| e.to_string())? == bytes,
            "relocation verify",
        )?;
        self.publish(next, cut)?;
        ensure(
            self.data_path(segment, source.generation).exists(),
            "recovery placement retired too early",
        )?;
        Ok(
            json!({"segment":segment,"copied_bytes":bytes.len(),"old_generation":source.generation,"new_generation":loc.generation,"old_copy_retained":true}),
        )
    }
    fn release_old_placement(&mut self, segment: u64, old_generation: u64) -> Result<()> {
        ensure(self.trusted, "unaudited physical state is read-only")?;
        let mut next = self.selected.clone();
        let root = next.active_locator.clone();
        let loc = self.locator(&root, segment, true)?;
        next.recovery_locator = self.apply(
            &next.recovery_locator,
            BTreeMap::from([(segment, Some(loc))]),
        )?;
        next.epoch += 1;
        self.publish(next, Cut::None)?;
        for root in [
            self.selected.active_locator.clone(),
            self.selected.recovery_locator.clone(),
        ] {
            ensure(
                self.locator(&root, segment, false)?.generation != old_generation,
                "placement still selected",
            )?;
        }
        fs::remove_file(self.data_path(segment, old_generation)).map_err(|e| e.to_string())?;
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.stats.sync_calls += 1;
        Ok(())
    }
}

fn snapshot(dir: &Path) -> Result<(u64, u64)> {
    let mut bytes = 0;
    let mut allocated = 0;
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let m = entry
            .map_err(|e| e.to_string())?
            .metadata()
            .map_err(|e| e.to_string())?;
        bytes += m.len();
        allocated += m.blocks() * 512;
    }
    Ok((bytes, allocated))
}
fn run(n: u64, kind: Kind) -> Result<serde_json::Value> {
    ensure(n > 0, "positive count")?;
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let source_dir = temp.path().join("source");
    fs::create_dir(&source_dir).map_err(|e| e.to_string())?;
    let mut source = Store::open(&source_dir)?;
    build(&mut source, n, kind)?;
    let mut session = Session::imported();
    session.audit(&mut source)?;
    let mut gate = Gate::create(&temp.path().join("placement"), source.head()?)?;
    let t = Instant::now();
    gate.bootstrap(&mut source)?;
    let bootstrap_ms = t.elapsed().as_millis();
    let baseline = snapshot(&gate.dir)?;
    let mut updates = Vec::new();
    for i in n + 1..=n + 4 {
        let source_before = source.size();
        session.accept(
            &mut source,
            &id(i),
            &request(&id(i)),
            CutAlias::None,
            RESERVE,
        )?;
        let source_append_bytes = source.size() - source_before;
        gate.stats = Metrics::default();
        gate.begin_reserve(64 * 1024 * 1024)?;
        let before = snapshot(&gate.dir)?;
        let t = Instant::now();
        let new = source.head()?;
        let registered = gate.select_semantic(&mut source, new)?;
        gate.finish_reserve()?;
        updates.push(json!({"us":t.elapsed().as_micros(),"new_objects":registered,"source_pack_append_bytes":source_append_bytes,"counters":gate.stats,"snapshot_before":before,"snapshot_after":snapshot(&gate.dir)?}));
    }
    let mut steps = Vec::new();
    for _ in 0..8 {
        gate.stats = Metrics::default();
        gate.begin_reserve(8 * 1024 * 1024)?;
        let t = Instant::now();
        let count = gate.retire_step()?;
        gate.finish_reserve()?;
        steps.push(json!({"processed":count,"us":t.elapsed().as_micros(),"counters":gate.stats}));
    }
    gate.stats = Metrics::default();
    let t = Instant::now();
    let candidate = gate.candidate()?;
    let candidate_us = t.elapsed().as_micros();
    let candidate_stats = gate.stats.clone();
    let selected = gate.selected.clone();
    let root = selected.semantic.active.clone();
    let mut fresh = Gate::reopen(&gate.dir)?;
    let t = Instant::now();
    fresh.read_object(&selected.active_locator, &root, false)?;
    fresh.read_object(
        &selected.recovery_locator,
        &selected.semantic.recovery,
        false,
    )?;
    let fresh_open = json!({"us":t.elapsed().as_micros(),"counters":fresh.stats,"read_only_until_full_audit":!fresh.trusted,"os_cache_evicted":false});
    gate.cache.clear();
    gate.stats = Metrics::default();
    let t = Instant::now();
    gate.read_object(&selected.active_locator, &root, true)?;
    let cold = json!({"us":t.elapsed().as_micros(),"counters":gate.stats});
    gate.stats = Metrics::default();
    let t = Instant::now();
    for _ in 0..32 {
        gate.read_object(&selected.active_locator, &root, true)?;
    }
    let warm = json!({"us":t.elapsed().as_micros(),"counters":gate.stats});
    gate.stats = Metrics::default();
    gate.begin_reserve(8 * 1024 * 1024)?;
    let t = Instant::now();
    let compact = gate.compact_one(Cut::None)?;
    gate.finish_reserve()?;
    let compact_us = t.elapsed().as_micros();
    let compact_stats = gate.stats.clone();
    // Every selected original receipt remains traversable through an independent
    // old-root audit in the source fixture; placement checks below explicitly
    // traverse every reachable semantic object, outside bounded measurements.
    audit_placement(&mut gate, &mut source)?;
    let old_generation = compact["old_generation"].as_u64().unwrap();
    let segment = compact["segment"].as_u64().unwrap();
    gate.stats = Metrics::default();
    gate.begin_reserve(8 * 1024 * 1024)?;
    gate.release_old_placement(segment, old_generation)?;
    gate.finish_reserve()?;
    let release_stats = gate.stats.clone();
    gate.audit_counts(&mut source)?;
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"bootstrap_ms":bootstrap_ms,"baseline":baseline,"updates":updates,"retirement_steps":steps,"queue_remaining":gate.selected.queue_last-gate.selected.queue_first,"candidate_segment":candidate.0,"candidate_us":candidate_us,"candidate_counters":candidate_stats,"fresh_root_pair_open":fresh_open,"cold_root_object_lookup":cold,"warm_32_root_object_lookups":warm,"compaction":compact,"compaction_us":compact_us,"compaction_counters":compact_stats,"release_counters":release_stats,"final_snapshot":snapshot(&gate.dir)?}),
    )
}
use super::Cut as CutAlias;
fn audit_placement(gate: &mut Gate, source: &mut Store) -> Result<()> {
    let selected = gate.selected.clone();
    for (root, locator) in [
        (selected.semantic.active, selected.active_locator),
        (selected.semantic.recovery, selected.recovery_locator),
    ] {
        let mut seen = HashSet::new();
        let mut todo = vec![root];
        while let Some(r) = todo.pop() {
            if seen.insert(r.clone()) {
                gate.read_object(&locator, &r, true)?;
                todo.extend(edges(source.get(&r)?));
            }
        }
    }
    Ok(())
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    println!(
        "{}",
        json!({"fixture":"HEAD-selected placement gate","cache_entries":CACHE,"retirement_work_bound":WORK,"metadata_trie_depth":16,"caveats":["fixture comparison only; direct-ref bootstrap arena and superseded tail generations have no bounded reclamation","cache bound to immutable locator root digest; no mutable pointer cache or reader leases","allocation-unit ledger and persisted liability are not OS reservation or qualified APFS metadata allowance","HEAD.pending is overwritten; not a production staging-ownership or cleanup protocol","four physical publication samples per map and count; source semantic commit is outside physical publication timing","source semantic commits measured separately in prior furnace; this gate copies tail segments","fresh root-pair probe validates placement and object hashes only, not complete project structural validation","active/recovery plus queued edge pins only; no undo or export pin classes","no qualified macOS power-loss or Saved claim"]})
    );
    for arg in args {
        let n = arg.parse::<u64>().map_err(|e| e.to_string())?;
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
    fn corrupt_metadata_and_forged_counts_refuse() -> Result<()> {
        for fault in ["count", "alias", "missing", "head"] {
            let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
            let mut source = Store::open(temp.path())?;
            build(&mut source, 32, Kind::Radix)?;
            let mut gate = Gate::create(&temp.path().join("physical"), source.head()?)?;
            gate.bootstrap(&mut source)?;
            let mut next = gate.selected.clone();
            match fault {
                "count" => {
                    let key = next.semantic.active.offset;
                    let mut life: Life = gate.lookup(&next.life, key)?.unwrap();
                    life.incoming -= 1;
                    next.life = gate.apply(&next.life, BTreeMap::from([(key, Some(life))]))?;
                    next.epoch += 1;
                    gate.publish(next, Cut::None)?;
                    let mut imported = Gate::reopen(&gate.dir)?;
                    ensure(
                        imported.audit_counts(&mut source).is_err(),
                        "forged count accepted",
                    )?;
                    ensure(!imported.trusted, "failed audit granted write trust")?;
                }
                "alias" => {
                    let mut loc = gate.locator(&next.active_locator, 0, false)?;
                    loc.slots[1] = loc.slots[0];
                    next.active_locator =
                        gate.apply(&next.active_locator, BTreeMap::from([(0, Some(loc))]))?;
                    next.epoch += 1;
                    gate.publish(next, Cut::None)?;
                    let mut imported = Gate::reopen(&gate.dir)?;
                    let locator = imported.selected.active_locator.clone();
                    ensure(
                        imported.locator(&locator, 0, false).is_err(),
                        "aliased locator accepted",
                    )?;
                }
                "missing" => {
                    gate.meta.set_len(0).map_err(|e| e.to_string())?;
                    let locator = gate.selected.active_locator.clone();
                    ensure(
                        gate.locator(&locator, 0, false).is_err(),
                        "missing metadata accepted",
                    )?;
                }
                "head" => {
                    next.epoch += 100;
                    fs::write(gate.dir.join("HEAD"), serde_json::to_vec(&next).unwrap())
                        .map_err(|e| e.to_string())?;
                    ensure(gate.retire_step().is_err(), "substituted HEAD accepted")?;
                    ensure(
                        gate.data_path(0, 0).exists(),
                        "substitution deleted selected bytes",
                    )?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
    #[test]
    fn persistent_counts_cache_and_recovery_placement() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
            let mut source = Store::open(temp.path())?;
            build(&mut source, 32, kind)?;
            let mut gate = Gate::create(&temp.path().join("physical"), source.head()?)?;
            gate.bootstrap(&mut source)?;
            let old = gate.selected.clone();
            gate.read_object(&old.active_locator, &old.semantic.active, true)?;
            let compact = gate.compact_one(Cut::None)?;
            let segment = compact["segment"].as_u64().unwrap();
            let generation = compact["old_generation"].as_u64().unwrap();
            ensure(
                gate.selected.semantic == old.semantic,
                "physical move changed semantic roots",
            )?;
            ensure(
                gate.data_path(segment, generation).exists(),
                "recovery generation prematurely deleted",
            )?;
            let new = gate.selected.clone();
            gate.stats = Metrics::default();
            gate.read_object(&new.active_locator, &new.semantic.active, true)?;
            ensure(
                gate.stats.metadata_reads > 0,
                "stale cached locator used under new root",
            )?;
            gate.read_object(&new.recovery_locator, &new.semantic.recovery, true)?;
            gate.release_old_placement(segment, generation)?;
            ensure(
                !gate.data_path(segment, generation).exists(),
                "old unpinned placement not retired",
            )?;
            let mut reopened = Gate::reopen(&gate.dir)?;
            ensure(
                reopened.retire_step().is_err(),
                "unaudited import allowed mutation",
            )?;
            reopened.audit_counts(&mut source)?;
            let mut session = Session::imported();
            session.audit(&mut source)?;
            for i in 33..=36 {
                session.accept(
                    &mut source,
                    &id(i),
                    &request(&id(i)),
                    CutAlias::None,
                    RESERVE,
                )?;
                let head = source.head()?;
                reopened.select_semantic(&mut source, head)?;
            }
            for _ in 0..8 {
                ensure(
                    reopened.retire_step()? <= WORK,
                    "retirement budget exceeded",
                )?;
            }
            reopened.audit_counts(&mut source)?;
        }
        Ok(())
    }
    #[test]
    fn unknown_selection_and_reserve_exhaustion_keep_both_copies() -> Result<()> {
        for cut in [Cut::BeforeHead, Cut::AfterHead] {
            let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
            let mut source = Store::open(temp.path())?;
            build(&mut source, 32, Kind::Radix)?;
            let mut gate = Gate::create(&temp.path().join("physical"), source.head()?)?;
            gate.bootstrap(&mut source)?;
            let old = gate.selected.clone();
            ensure(
                gate.begin_reserve(0).is_err() && !gate.dir.join("liability").exists(),
                "preflight refusal wrote liability",
            )?;
            gate.begin_reserve(8 * 1024 * 1024)?;
            ensure(gate.compact_one(cut).is_err(), "cut did not interrupt")?;
            ensure(
                gate.dir.join("liability").exists(),
                "unknown outcome released liability",
            )?;
            ensure(
                gate.data_path(0, 0).exists() && gate.data_path(0, old.epoch + 1).exists(),
                "unknown outcome retired generation",
            )?;
            let mut reopened = Gate::reopen(&gate.dir)?;
            audit_placement(&mut reopened, &mut source)?;
            ensure(
                reopened.selected.semantic == old.semantic,
                "unknown placement changed semantics",
            )?;
            // Exhausted external capacity after selection cannot yield a success
            // or discard either generation: the unresolved liability persists.
            reopened.trusted = true;
            reopened.remaining = Some(0);
            ensure(
                reopened.finish_reserve().is_err() && reopened.dir.join("liability").exists(),
                "post-selection exhaustion lost liability",
            )?;
        }
        Ok(())
    }
}
