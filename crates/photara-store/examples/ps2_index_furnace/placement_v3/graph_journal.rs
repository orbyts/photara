//! Genuine Core commands on the existing disposable actual-v3 physical path.
//! Journal barriers are measured `sync_all` calls, never qualified acknowledgements.
use super::typed_inventory::{graph as ownership_graph, recipe, retirement_ledger};
#[allow(
    clippy::wildcard_imports,
    reason = "Companion shares the disposable packed fixture"
)]
use super::*;
use crate::graph_model::{self as graph, Record, State};

const JOURNAL_MAX: usize = 256 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Group {
    old: Head,
    old_end: u64,
    before: State,
    after: State,
    records: Vec<Record>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Binding {
    group_json: String,
    control_base: Selection,
    source_tail_sha: String,
    source_tail_len: u64,
    data_base: recipe::Prefix,
    meta_base: recipe::Prefix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ownership: Option<ownership_graph::Original>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) source_charge: Option<ownership_graph::SourceCharge>,
}
impl Binding {
    pub(super) fn is_joint(&self) -> bool {
        self.ownership.is_some() && self.source_charge.is_some()
    }
}
struct PreparedGraph {
    append: PreparedAppend,
    continuation: Continuation,
    ownership: Option<ownership_graph::Original>,
}
#[derive(Clone, Copy, PartialEq)]
enum GraphCut {
    None,
    Journal,
    Hold,
    JournalPartial,
    JournalBytes,
    JournalFile,
    SourcePartial,
    PackedPartial(usize),
    PackedAfterData,
    SourceCandidate,
    PackedBefore,
    PackedBeforeHead,
    PackedAfterHead,
    Marker,
    JournalUnlink,
    JournalCleanupBarrier,
    CandidateUnlink,
    MarkerUnlink,
    SettlementBeforeHead,
    SettlementAfterHead,
}
fn authored_state(source: &mut Store, head: &Head) -> Result<State> {
    let selected = root(source, &head.active)?;
    let reference = authored(source, &selected)?;
    let Node::Authored {
        state: Some(state), ..
    } = source.get(&reference)?
    else {
        return Err("typed Graph authored state missing".into());
    };
    ensure(state.count == selected.count, "Graph state/index count")?;
    Ok(*state)
}
fn receipt(source: &mut Store, record: &Record) -> Result<Ref> {
    source.put(&Node::Receipt {
        id: record.id.clone(),
        request: record.request.clone(),
        ordinal: record.ordinal,
        record: Some(Box::new(record.clone())),
    })
}
fn entry(record: &Record, receipt: Ref) -> Entry {
    Entry {
        key: key(&record.id),
        id: record.id.clone(),
        request: record.request.clone(),
        ordinal: record.ordinal,
        receipt,
    }
}
fn build_graph(source: &mut Store, n: u64, kind: Kind) -> Result<()> {
    ensure(n > 0, "positive Graph fixture baseline")?;
    let mut state = State::initial(16);
    let mut entries = Vec::new();
    let mut receipts = Vec::new();
    for n in 1..=n {
        let record = graph::workload(&mut state, n)?;
        let r = receipt(source, &record)?;
        receipts.push(r.clone());
        entries.push(entry(&record, r));
    }
    let sequence = seq_build(source, &receipts)?;
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    let map = match kind {
        Kind::Radix => radix_build(source, &entries)?,
        Kind::Btree => btree_build(source, &entries)?,
    };
    let authored = source.put(&Node::Authored {
        value: "genuine Graph state".into(),
        state: Some(Box::new(state)),
    })?;
    let active = make_root(source, kind, n, map, sequence, authored)?;
    source.publish(&Head {
        recovery: active.clone(),
        active,
    })
}
fn plan(source: &mut Store, size: u64) -> Result<Group> {
    ensure(size > 0 && size <= 32, "finite Graph journal group")?;
    let old = source.head()?;
    let before = authored_state(source, &old)?;
    let mut after = before.clone();
    let mut records = Vec::new();
    for n in before.count + 1..=before.count + size {
        records.push(graph::workload(&mut after, n)?);
    }
    Ok(Group {
        old,
        old_end: source.size(),
        before,
        after,
        records,
    })
}
fn validate(source: &mut Store, group: &Group) -> Result<()> {
    ensure(
        !group.records.is_empty() && group.records.len() <= 32,
        "finite original group",
    )?;
    ensure(group.old_end <= source.size(), "original source extent")?;
    ensure(
        authored_state(source, &group.old)? == group.before,
        "original authored state",
    )?;
    let old = root(source, &group.old.active)?;
    let mut ids = HashSet::new();
    let mut state = group.before.clone();
    for r in &group.records {
        ensure(ids.insert(r.id.clone()), "duplicate journal OperationId")?;
        ensure(
            lookup(source, &old.map, &r.id, old.kind)?.is_none(),
            "already indexed journal OperationId",
        )?;
        graph::replay(&mut state, r)?;
    }
    ensure(state == group.after, "journal final state")
}
fn read<T: serde::de::DeserializeOwned>(dir: &Path, name: &str) -> Result<T> {
    let file = File::open(dir.join(name)).map_err(error)?;
    ensure(
        file.metadata().map_err(error)?.len() <= JOURNAL_MAX as u64,
        "finite journal envelope bound",
    )?;
    let mut bytes = Vec::new();
    file.take(JOURNAL_MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    ensure(bytes.len() <= JOURNAL_MAX, "finite journal envelope bound")?;
    serde_json::from_slice(&bytes).map_err(error)
}
fn write_new<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<u64> {
    let bytes = photara_core::canonical_json(&serde_json::to_value(value).map_err(error)?)
        .map_err(error)?;
    ensure(bytes.len() <= JOURNAL_MAX, "finite journal envelope bound")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name))
        .map_err(error)?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(error)?;
    File::open(dir).and_then(|f| f.sync_all()).map_err(error)?;
    Ok(bytes.len() as u64)
}
fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    photara_core::canonical_json(&serde_json::to_value(value).map_err(error)?).map_err(error)
}
fn persist_pending(source: &mut Store, f: &mut Fixture, dir: &Path, group: &Group) -> Result<u64> {
    admit_pending(source, f, dir, group, GraphCut::None)
}
fn admit_pending(
    source: &mut Store,
    f: &mut Fixture,
    dir: &Path,
    group: &Group,
    cut: GraphCut,
) -> Result<u64> {
    ensure(
        !dir.join("group").exists()
            && !dir.join("candidate").exists()
            && !dir.join("marker").exists(),
        "pending Graph journal",
    )?;
    ensure(
        source.head()? == group.old && f.head.semantic == group.old,
        "Graph admission selection",
    )?;
    ensure(f.head.gate.holds.is_empty(), "pending physical liability")?;
    validate(source, group)?;
    let (preview, prepared) = prepare(source, f, group, None)?;
    let bytes = canonical(group)?;
    ensure(
        bytes.len() <= JOURNAL_MAX / 2,
        "original-token journal byte bound",
    )?;
    let tail = preview.tail();
    let binding = Binding {
        group_json: String::from_utf8(bytes.clone()).map_err(error)?,
        control_base: f.head.clone(),
        source_tail_sha: hash(&tail),
        source_tail_len: tail.len() as u64,
        data_base: recipe::Prefix::capture(&f.data)?,
        meta_base: recipe::Prefix::capture(&f.meta)?,
        source_charge: if prepared.ownership.is_some() {
            Some(retirement_ledger::registered_source(f, source)?)
        } else {
            None
        },
        ownership: prepared.ownership.clone(),
    };
    f.c.recipe_prefix_read_bytes += checked(f.data.end, f.meta.end)?;
    recipe::segments(&binding.data_base, &prepared.append.data, true)?;
    recipe::segments(&binding.meta_base, &prepared.append.meta, false)?;
    let physical = physical_bound(Some(&prepared.append.data), &prepared.append.meta)?;
    // Finite fixture caps include both journal and source staging on the SAME modeled domain.
    // Full source allowance is deliberately conservative; it is not an OS reservation.
    let budget = checked(
        physical["allocation_bound"]
            .as_u64()
            .ok_or("Graph physical bound")?,
        16 * 1024 * 1024 + 4 * JOURNAL_MAX as u64 + f.control_bound(&f.head.gate)? * 5,
    )?;
    let continuation = prepared.continuation;
    f.reserve_with_graph(
        prepared.append.semantic,
        None,
        &[(0, budget)],
        Some(continuation),
        Some(binding.clone()),
    )?;
    if cut == GraphCut::Hold {
        return Err("cut after original Graph hold before journal".into());
    }
    persist_original_journal(dir, &binding, cut)?;
    Ok(bytes.len() as u64)
}
fn persist_original_journal(dir: &Path, binding: &Binding, cut: GraphCut) -> Result<()> {
    let expected = binding.group_json.as_bytes();
    let path = dir.join("group");
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if !path.exists() {
        options.create_new(true);
    }
    let mut file = options.open(path).map_err(error)?;
    let len = usize::try_from(file.metadata().map_err(error)?.len()).map_err(error)?;
    ensure(len <= expected.len(), "journal exceeds original bound")?;
    let mut prefix = vec![0; len];
    file.read_exact(&mut prefix).map_err(error)?;
    ensure(
        prefix == expected[..len],
        "journal differs from original admitted bytes",
    )?;
    let remaining = &expected[len..];
    let write = if cut == GraphCut::JournalPartial {
        &remaining[..remaining.len() / 2]
    } else {
        remaining
    };
    file.write_all(write).map_err(error)?;
    if matches!(cut, GraphCut::JournalPartial | GraphCut::JournalBytes) {
        return Err("cut during original journal write".into());
    }
    file.sync_all().map_err(error)?;
    if cut == GraphCut::JournalFile {
        return Err("cut after journal file barrier".into());
    }
    File::open(dir).and_then(|f| f.sync_all()).map_err(error)
}
fn prepare(
    source: &mut Store,
    f: &mut Fixture,
    group: &Group,
    hold: Option<&Liability>,
) -> Result<(crate::graph_preview::Preview, PreparedGraph)> {
    let ownership = if let Some(hold) = hold {
        hold.graph
            .as_ref()
            .ok_or("original Graph binding")?
            .ownership
            .clone()
    } else if f.head.gate.ledger.is_some() {
        Some(ownership_graph::Original::capture(f)?)
    } else {
        None
    };
    source.begin_graph_preview(group.old.clone(), group.old_end)?;
    let previous = f.head.clone();
    let ends = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    if let Some(hold) = hold {
        if ownership.is_some() {
            f.head = hold
                .graph
                .as_ref()
                .ok_or("joint original binding")?
                .control_base
                .clone();
        }
        f.head.semantic = group.old.clone();
        f.head.active = hold.origin.active.clone();
        f.head.recovery = hold.origin.recovery.clone();
        f.head.inventory = hold.origin.inventory.clone();
        f.data.pack = hold.origin.data_pack;
        f.data.end = hold.origin.data_end;
        f.meta.pack = hold.origin.meta_pack;
        f.meta.end = hold.origin.meta_end;
    }
    let result: Result<(PreparedAppend, Continuation)> = (|| {
        let candidate = build_candidate(source, group)?;
        source.publish(&candidate)?;
        if let Some(original) = &ownership {
            let joint = ownership_graph::plan(f, source, group.old_end, original)?;
            Ok((joint.append, joint.continuation))
        } else {
            let append = f.checkpoint_plan(source, group.old_end)?;
            let mut continuation = Continuation::append(&append);
            continuation.inventory = Inventory {
                active: f.head.inventory.active.clone(),
                recovery: f.head.inventory.active.clone(),
                next: f.head.inventory.next,
            };
            Ok((append, continuation))
        }
    })();
    f.head = previous;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = ends;
    f.staged = None;
    f.cache.clear();
    let preview = source.take_graph_preview()?;
    let (append, continuation) = result?;
    Ok((
        preview,
        PreparedGraph {
            append,
            continuation,
            ownership,
        },
    ))
}
fn build_candidate(source: &mut Store, group: &Group) -> Result<Head> {
    let old = root(source, &group.old.active)?;
    let mut map = old.map;
    let mut sequence = old.sequence;
    for record in &group.records {
        let r = receipt(source, record)?;
        let e = entry(record, r.clone());
        map = match old.kind {
            Kind::Radix => radix_insert(source, &map, e)?,
            Kind::Btree => btree_insert(source, &map, e)?,
        };
        sequence = seq_append(source, &sequence, r)?;
    }
    let state = source.put(&Node::Authored {
        value: "genuine Graph state".into(),
        state: Some(Box::new(group.after.clone())),
    })?;
    let active = make_root(source, old.kind, group.after.count, map, sequence, state)?;
    let candidate = Head {
        active,
        recovery: group.old.active.clone(),
    };
    verify_source(source, &candidate, group)?;
    Ok(candidate)
}
fn verify_source(source: &mut Store, candidate: &Head, group: &Group) -> Result<()> {
    ensure(
        candidate.recovery == group.old.active && authored_state(source, candidate)? == group.after,
        "original Graph candidate",
    )?;
    let old = root(source, &group.old.active)?;
    let new = root(source, &candidate.active)?;
    ensure(old.kind == new.kind, "Graph index kind continuity")?;
    prefix(source, &old.sequence, &new.sequence)?;
    for record in &group.records {
        let e = lookup(source, &new.map, &record.id, new.kind)?
            .ok_or("source original receipt absent")?;
        ensure(
            e == entry(record, e.receipt.clone()),
            "source original receipt index",
        )?;
        ensure(
            matches!(source.get(&e.receipt)?, Node::Receipt { record: Some(got), .. } if *got == *record),
            "source original record changed",
        )?;
    }
    Ok(())
}
fn physical_state(f: &mut Fixture, locator: &PRef, semantic: &Ref) -> Result<State> {
    let Node::Root {
        count, manifest, ..
    } = f.object(locator, semantic)?
    else {
        return Err("physical Graph root".into());
    };
    let Node::Manifest { authored, .. } = f.object(locator, &manifest)? else {
        return Err("physical Graph manifest".into());
    };
    let Node::Authored {
        state: Some(state), ..
    } = f.object(locator, &authored)?
    else {
        return Err("physical Graph state".into());
    };
    ensure(state.count == count, "physical Graph count")?;
    Ok(*state)
}
fn original(f: &mut Fixture, record: &Record) -> Result<Ref> {
    let r = f.retry(&record.id, &record.request)?;
    let locator = f.head.active.clone();
    ensure(
        matches!(f.object(&locator, &r)?, Node::Receipt { record: Some(got), .. } if *got == *record),
        "packed original record mismatch",
    )?;
    Ok(r)
}
fn cleanup_completed(source: &mut Store, f: &mut Fixture, dir: &Path, cut: GraphCut) -> Result<()> {
    ensure(
        !dir.join("group").exists(),
        "pending Graph group requires reconciliation",
    )?;
    if !dir.join("marker").exists() {
        return ensure(
            !dir.join("candidate").exists(),
            "candidate without original completion marker",
        );
    }
    let marker: serde_json::Value = read(dir, "marker")?;
    let head = f.head.clone();
    ensure(
        source.head()? == head.semantic,
        "unknown completed source selection",
    )?;
    let state = physical_state(f, &head.active, &head.semantic.active)?;
    ensure(
        marker == checkpoint_marker(&head.semantic, &state)?,
        "unknown completed marker preserves evidence",
    )?;
    if dir.join("candidate").exists() {
        ensure(
            read::<Head>(dir, "candidate")? == head.semantic,
            "unknown completed candidate",
        )?;
        fs::remove_file(dir.join("candidate")).map_err(error)?;
        if cut == GraphCut::CandidateUnlink {
            return Err("cut after candidate unlink".into());
        }
        File::open(dir).and_then(|f| f.sync_all()).map_err(error)?;
    }
    fs::remove_file(dir.join("marker")).map_err(error)?;
    if cut == GraphCut::MarkerUnlink {
        return Err("cut after marker unlink".into());
    }
    File::open(dir).and_then(|f| f.sync_all()).map_err(error)
}
fn checkpoint_marker(candidate: &Head, state: &State) -> Result<serde_json::Value> {
    Ok(
        json!({"semantic":candidate,"prefix":state.prefix,"through":state.count,"authored":state.coordinate()?,"qualified_accepted":false,"qualified_saved":false}),
    )
}
// Completion consumes the hold. Its interrupted selectors therefore require
// separate exact reconstruction from the original hold in the old intent.
fn reconcile_settlement(f: &mut Fixture, source: &mut Store, dir: &Path) -> Result<()> {
    if !f.dir.join("intent").exists() {
        return Ok(());
    }
    let old: Selection = Fixture::bounded_read(&f.dir.join("intent"))?;
    let candidate: Selection = Fixture::bounded_read(&f.dir.join("candidate"))?;
    if !candidate.gate.holds.is_empty() {
        return Ok(());
    }
    let hold = old
        .gate
        .holds
        .values()
        .next()
        .ok_or("settlement original hold")?
        .clone();
    let binding = hold
        .graph
        .as_ref()
        .ok_or("settlement original Graph binding")?;
    ensure(
        binding.is_joint() && old.gate.holds.len() == 1,
        "joint original settlement",
    )?;
    let (_, published) = recipe::accounting::selectors_from_base(&hold, &binding.control_base)?;
    ensure(
        old == published,
        "settlement old selector differs from original publication",
    )?;
    let selected = f.head.clone();
    f.head = old.clone();
    let derive: Result<()> = (|| {
        let gate = retirement_ledger::graph_completion_gate(f, &hold, source)?;
        let mut completion = old.clone();
        completion.gate = gate;
        completion.epoch = checked(completion.epoch, 1)?;
        ensure(candidate == completion, "unknown Graph completion selector")?;
        if f.dir.join("HEAD.next").exists() {
            let next: Selection = Fixture::bounded_read(&f.dir.join("HEAD.next"))?;
            ensure(
                next == completion,
                "unknown staged Graph completion selector",
            )?;
        }
        let disk: Selection = Fixture::bounded_read(&f.dir.join("HEAD"))?;
        ensure(
            disk == selected && (disk == old || disk == completion),
            "unknown selected settlement",
        )?;
        ensure(
            fs::read_dir(dir).map_err(error)?.next().is_none(),
            "settlement before journal cleanup",
        )?;
        let group: Group = serde_json::from_str(&binding.group_json).map_err(error)?;
        ensure(
            canonical(&group)? == binding.group_json.as_bytes(),
            "settlement original canonical group",
        )?;
        validate(source, &group)?;
        let (preview, prepared) = prepare(source, f, &group, Some(&hold))?;
        ensure(
            hold.continuation.as_ref() == Some(&prepared.continuation)
                && hash(&preview.tail()) == binding.source_tail_sha
                && preview.tail().len() as u64 == binding.source_tail_len,
            "settlement original plan changed",
        )?;
        verify_source(source, &hold.target, &group)?;
        binding
            .ownership
            .as_ref()
            .ok_or("joint ownership commitment")?
            .verify_selected(f, &prepared.continuation)?;
        ensure(
            physical_state(f, &old.active, &old.semantic.active)? == group.after,
            "settlement Graph state",
        )?;
        for record in &group.records {
            original(f, record)?;
        }
        Ok(())
    })();
    f.head = selected;
    derive?;
    f.reconcile()?;
    Ok(())
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep original-token journal/recipe/selection/cleanup order visible together"
)]
fn finish(f: &mut Fixture, source: &mut Store, dir: &Path, cut: GraphCut) -> Result<()> {
    reconcile_settlement(f, source, dir)?;
    let Some(hold) = f.head.gate.holds.values().next().cloned() else {
        ensure(
            !dir.join("group").exists(),
            "journal without original liability",
        )?;
        return cleanup_completed(source, f, dir, cut);
    };
    ensure(f.head.gate.holds.len() == 1, "one original Graph liability")?;
    let binding = hold.graph.as_ref().ok_or("original Graph binding absent")?;
    recipe::verify_controls(f, &hold, &binding.control_base)?;
    let group: Group = serde_json::from_str(&binding.group_json).map_err(error)?;
    ensure(
        canonical(&group)? == binding.group_json.as_bytes(),
        "original canonical Graph binding",
    )?;
    validate(source, &group)?;
    let (preview, prepared) = prepare(source, f, &group, Some(&hold))?;
    let tail = preview.tail();
    ensure(
        hash(&tail) == binding.source_tail_sha
            && tail.len() as u64 == binding.source_tail_len
            && prepared.append.semantic == hold.target,
        "original Graph preview changed",
    )?;
    let expected = prepared.continuation;
    ensure(
        hold.continuation.as_ref() == Some(&expected),
        "Graph rebuilt placement differs from original continuation",
    )?;
    let disk: Selection = Fixture::bounded_read(&f.dir.join("HEAD"))?;
    ensure(disk == f.head, "unknown selected Graph HEAD")?;
    if f.dir.join("intent").exists() {
        let old: Selection = Fixture::bounded_read(&f.dir.join("intent"))?;
        let candidate: Selection = Fixture::bounded_read(&f.dir.join("candidate"))?;
        ensure(
            (disk == old || disk == candidate)
                && old.gate.holds.get(&hold.token) == Some(&hold)
                && candidate.gate.holds.get(&hold.token) == Some(&hold),
            "unknown Graph publication original token",
        )?;
    }
    let candidate = hold.target.clone();
    let selected = source.head()?;
    ensure(
        selected == group.old || selected == candidate,
        "unknown source HEAD preserves journal",
    )?;
    ensure(
        f.head.semantic == group.old || f.head.semantic == candidate,
        "unknown packed HEAD preserves journal",
    )?;
    let completing = f.head.semantic == candidate && !dir.join("group").exists();
    if !completing {
        persist_original_journal(dir, binding, GraphCut::None)?;
    }
    preview.persist(source, cut == GraphCut::SourcePartial)?;
    verify_source(source, &candidate, &group)?;
    if dir.join("candidate").exists() {
        ensure(
            read::<Head>(dir, "candidate")? == candidate,
            "original Graph candidate envelope",
        )?;
    } else if !completing {
        write_new(dir, "candidate", &candidate)?;
    }
    if cut == GraphCut::SourceCandidate {
        return Err("cut after source candidate barrier".into());
    }
    if selected == group.old {
        source.publish(&candidate)?;
    }
    f.authorized_hold = Some(hold.token);
    if cut == GraphCut::PackedBefore {
        return Err("cut before packed effect retains Graph hold".into());
    }
    let mut remaining = if let GraphCut::PackedPartial(bytes) = cut {
        Some(bytes)
    } else {
        None
    };
    recipe::replay(
        f,
        true,
        &binding.data_base,
        &prepared.append.data,
        &mut remaining,
    )?;
    if cut == GraphCut::PackedAfterData {
        return Err("cut after Graph data recipe".into());
    }
    recipe::replay(
        f,
        false,
        &binding.meta_base,
        &prepared.append.meta,
        &mut remaining,
    )?;
    if f.dir.join("intent").exists() {
        f.reconcile()?;
    }
    if f.head.semantic != candidate
        || f.head.active != expected.active
        || f.head.recovery != expected.recovery
    {
        let packed_cut = match cut {
            GraphCut::PackedBeforeHead => Cut::BeforeHead,
            GraphCut::PackedAfterHead => Cut::AfterHead,
            _ => Cut::None,
        };
        f.publish(
            expected.active.clone(),
            expected.recovery.clone(),
            candidate.clone(),
            packed_cut,
        )?;
    }
    ensure(f.head.semantic == candidate, "Graph packed selection")?;
    if let Some(ownership) = &binding.ownership {
        ownership.verify_selected(f, &expected)?;
    }
    let h = f.head.clone();
    ensure(
        physical_state(f, &h.active, &h.semantic.active)? == group.after,
        "packed checkpoint final state",
    )?;
    for record in &group.records {
        original(f, record)?;
    }
    if !completing {
        let marker = checkpoint_marker(&candidate, &group.after)?;
        if dir.join("marker").exists() {
            ensure(
                read::<serde_json::Value>(dir, "marker")? == marker,
                "original checkpoint marker",
            )?;
        } else {
            write_new(dir, "marker", &marker)?;
        }
        if cut == GraphCut::Marker {
            return Err("cut after Graph checkpoint marker".into());
        }
        fs::remove_file(dir.join("group")).map_err(error)?;
        if cut == GraphCut::JournalUnlink {
            return Err("cut after journal unlink".into());
        }
        File::open(dir).and_then(|f| f.sync_all()).map_err(error)?;
        if cut == GraphCut::JournalCleanupBarrier {
            return Err("cut after journal cleanup barrier".into());
        }
    }
    cleanup_completed(source, f, dir, cut)?;
    // Conservative high-water consumption on both clean and interrupted paths. No deletion credit.
    let used = hold
        .by_domain
        .iter()
        .map(|(domain, bytes)| {
            (
                *domain,
                if *domain == 0 {
                    bytes - hold.control
                } else {
                    *bytes
                },
            )
        })
        .collect::<Vec<_>>();
    if binding.is_joint() {
        let gate = retirement_ledger::graph_completion_gate(f, &hold, source)?;
        let cut = match cut {
            GraphCut::SettlementBeforeHead => Cut::BeforeHead,
            GraphCut::SettlementAfterHead => Cut::AfterHead,
            _ => Cut::None,
        };
        f.select_gate_cut(gate, cut)
    } else {
        f.complete_liability(&hold, &used)
    }
}

fn sequence_records(
    f: &mut Fixture,
    locator: &PRef,
    r: &Ref,
    start: u64,
    depth: usize,
) -> Result<Vec<(Ref, Record)>> {
    ensure(depth <= 32, "Graph sequence audit depth")?;
    let Node::Sequence {
        level,
        count,
        children,
    } = f.object(locator, r)?
    else {
        return Err("Graph sequence kind".into());
    };
    let mut out = Vec::new();
    for child in children {
        if level == 0 {
            let Node::Receipt {
                id,
                request,
                ordinal,
                record: Some(record),
            } = f.object(locator, &child)?
            else {
                return Err("typed original Graph receipt".into());
            };
            ensure(
                id == record.id
                    && request == record.request
                    && ordinal == record.ordinal
                    && ordinal == start + out.len() as u64,
                "Graph original receipt/ordinal identity",
            )?;
            out.push((child, *record));
        } else {
            let Node::Sequence {
                level: child_level, ..
            } = f.object(locator, &child)?
            else {
                return Err("Graph sequence child kind".into());
            };
            ensure(child_level + 1 == level, "Graph sequence level")?;
            out.extend(sequence_records(
                f,
                locator,
                &child,
                start + out.len() as u64,
                depth + 1,
            )?);
        }
    }
    ensure(out.len() as u64 == count, "Graph sequence count")?;
    Ok(out)
}
fn audit_graph(f: &mut Fixture) -> Result<serde_json::Value> {
    let head = f.head.clone();
    let mut counts = Vec::new();
    for (locator, semantic) in [
        (&head.active, &head.semantic.active),
        (&head.recovery, &head.semantic.recovery),
    ] {
        let final_state = physical_state(f, locator, semantic)?;
        let Node::Root { sequence, map, .. } = f.object(locator, semantic)? else {
            return Err("Graph audit root".into());
        };
        let records = sequence_records(f, locator, &sequence, 1, 0)?;
        let mut state = State::initial(final_state.graph.nodes.len());
        let mut entries = BTreeMap::new();
        let mut todo = vec![map];
        let mut visited = HashSet::new();
        while let Some(r) = todo.pop() {
            ensure(visited.insert(r.clone()), "Graph audit map cycle/alias")?;
            match f.object(locator, &r)? {
                Node::Leaf { entries: leaf } => {
                    checked_entries(&leaf)?;
                    for e in leaf {
                        ensure(
                            entries.insert(e.id.clone(), e).is_none(),
                            "Graph audit duplicate ID",
                        )?;
                    }
                }
                Node::Radix { left, right, .. } => todo.extend([left, right]),
                Node::Btree { children, .. } => todo.extend(children),
                _ => return Err("Graph audit map kind".into()),
            }
        }
        let mut noops = 0;
        let mut inverses = 0;
        for (r, record) in records {
            let saved = f.head.clone();
            f.head.active = locator.clone();
            f.head.semantic.active = semantic.clone();
            let found = original(f, &record);
            f.head = saved;
            ensure(found? == r, "Graph routed original receipt")?;
            ensure(
                entries.remove(&record.id) == Some(entry(&record, r)),
                "Graph map/ordinal original identity",
            )?;
            graph::replay(&mut state, &record)?;
            noops += usize::from(record.unchanged);
            inverses += usize::from(record.fixture_inverse_of.is_some());
        }
        ensure(
            entries.is_empty() && state == final_state,
            "Graph complete replay/extra map entries",
        )?;
        counts.push(json!({"operations":state.count,"authored_revision":state.authored_revision,"noops":noops,"inverses":inverses,"prefix":state.prefix}));
    }
    Ok(json!(counts))
}
fn setup_graph(n: u64, kind: Kind) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
    let temp = tempfile::tempdir().map_err(error)?;
    let mut source = Store::open(&temp.path().join("source"))?;
    build_graph(&mut source, n, kind)?;
    full_audit(&mut source)?;
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    f.initialize_gate()?;
    let journal = temp.path().join("journal");
    fs::create_dir(&journal).map_err(error)?;
    let mut gate = f.head.gate.clone();
    let domain = gate.domains.get_mut(&0).ok_or("Graph bootstrap domain")?;
    domain.charged = checked(domain.charged, source.allocated() + 32768)?;
    f.select_gate(gate)?;
    Ok((temp, source, f, journal))
}
fn setup_joint(n: u64, kind: Kind) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
    setup_joint_padded(n, kind, None)
}
fn setup_joint_padded(
    n: u64,
    kind: Kind,
    pad: Option<bool>,
) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
    let temp = tempfile::tempdir().map_err(error)?;
    let mut source = Store::open(&temp.path().join("source"))?;
    build_graph(&mut source, n, kind)?;
    full_audit(&mut source)?;
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    if let Some(data) = pad {
        // Bulk-bootstrap framed unreachable bytes force a near-full existing
        // generation. Ownership enrollment below includes this allocation.
        let arena = if data { &mut f.data } else { &mut f.meta };
        let target = PACK - 16 * 1024;
        let frame = if data { 12 } else { 4 };
        while arena.end + frame < target {
            let len = usize::try_from((target - arena.end - frame).min(4092)).map_err(error)?;
            arena.append(
                &vec![0; len],
                if data { Some(u64::MAX - 1) } else { None },
                &mut f.c,
            )?;
        }
        let h = f.head.clone();
        f.publish(h.active, h.recovery, h.semantic, Cut::None)?;
    }
    retirement_ledger::initialize(&mut f)?;
    let journal = temp.path().join("journal");
    fs::create_dir(&journal).map_err(error)?;
    retirement_ledger::enroll_source(&mut f, &source)?;
    Ok((temp, source, f, journal))
}
fn run(n: u64, kind: Kind, joint: bool) -> Result<serde_json::Value> {
    let (_temp, mut source, mut f, journal) = if joint {
        setup_joint(n, kind)?
    } else {
        setup_graph(n, kind)?
    };
    let mut samples = Vec::new();
    for size in [1, 8, 32] {
        let ledger_before = serde_json::to_value(&f.head.gate.ledger).map_err(error)?;
        let charged_before = f
            .head
            .gate
            .domains
            .get(&0)
            .ok_or("Graph capacity domain")?
            .charged;
        source.stats = Stats::default();
        f.c = Counters::default();
        let t = Instant::now();
        let group = plan(&mut source, size)?;
        let command_planning_us = t.elapsed().as_micros();
        let t = Instant::now();
        let journal_bytes = persist_pending(&mut source, &mut f, &journal, &group)?;
        let admission_journal_us = t.elapsed().as_micros();
        let admitted = f
            .head
            .gate
            .holds
            .values()
            .next()
            .cloned()
            .ok_or("Graph hold")?;
        let admitted_controls = f.c.clone();
        let t = Instant::now();
        finish(&mut f, &mut source, &journal, GraphCut::None)?;
        let checkpoint_cleanup_us = t.elapsed().as_micros();
        let journal_file_write_bytes = journal_bytes
            + canonical(&admitted.target)?.len() as u64
            + canonical(&checkpoint_marker(&admitted.target, &group.after)?)?.len() as u64;
        let source_process_write_bytes =
            source.stats.object_write_bytes + source.stats.envelope_write_bytes;
        let packed_process_write_bytes =
            f.c.data_write_bytes + f.c.meta_write_bytes + f.c.envelope_write_bytes;
        samples.push(json!({"group_size":size,"command_planning_us":command_planning_us,"admission_including_preview_reserve_journal_us":admission_journal_us,"checkpoint_cleanup_us":checkpoint_cleanup_us,"journal_group_bytes":journal_bytes,"journal_file_write_bytes":journal_file_write_bytes,"journal_sync_calls_clean_path":11,"source_process_write_bytes":source_process_write_bytes,"source_counters":source.stats,"packed_process_write_bytes":packed_process_write_bytes,"packed_counters_including_admission":f.c,"admission_packed_counters":admitted_controls,"total_successful_process_write_bytes":journal_file_write_bytes + source_process_write_bytes + packed_process_write_bytes,"conservative_original_hold_bytes":admitted.by_domain,"ledger_before":ledger_before,"ledger_after":f.head.gate.ledger,"charged_increment":f.head.gate.domains.get(&0).ok_or("Graph capacity domain")?.charged - charged_before,"settlement":if joint { "observed source and tip highwater growth once; standing controls unchanged; no project or filesystem credit" } else { "full admitted high-water; no refund/OS guarantee" }}));
    }
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    reopened.structural_probe()?;
    let mut genesis = State::initial(16);
    let oldest = graph::workload(&mut genesis, 1)?;
    original(&mut reopened, &oldest)?;
    let replay = audit_graph(&mut reopened)?;
    reopened.audit_combined()?;
    full_audit(&mut source)?;
    Ok(
        json!({"kind":format!("{kind:?}"),"baseline":n,"joint_owned":joint,"samples":samples,"full_closure_audit":"independent after measured completion; completion checks regenerated exact plan, original receipts and both selected tip claims","physical_replay_audit":replay,"oldest_original_retry":true,"qualified_accepted":false,"qualified_saved":false,"physical_snapshot":reopened.snapshot()?}),
    )
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    let joint = args.first().is_some_and(|s| s == "joint");
    let args = if joint { &args[1..] } else { args };
    println!(
        "{}",
        json!({"fixture":if joint {"actual-v3-joint-owned-graph-journal"} else {"actual-v3-genuine-graph-journal"},"qualification":"disposable sync_all observations only","caveats":["baseline bulk-built, not individually durable acceptance","fixed16node position/noop/inverse/Batch workload","original hold commits canonical finite journal bytes before journal creation","matching source/journal/data/meta partial-prefix recipe replay; conflicting bytes retain original hold",if joint {"Graph and both exact data/meta ownership closures share one original token; fresh-generation rollover refuses before effects"} else {"legacy Graph mode: automatic ownership self-coverage remains a separate publication"},if joint {"observed source/tip highwater settlement plus separate standing controls; no source retirement credit or OS reservation"} else {"legacy Graph mode: full conservative original hold charged"},"controlled return cuts, not kill/power-loss/qualified storage","no permanent wire or production Accepted/Saved"]})
    );
    let counts = if args.is_empty() {
        vec![if joint { 8 } else { 64 }]
    } else {
        args.iter()
            .map(|s| s.parse().map_err(error))
            .collect::<Result<Vec<u64>>>()?
    };
    for n in counts {
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run(n, kind, joint)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_graph_groups_recover_on_actual_v3_at_twenty_one_cuts() -> Result<()> {
        recovery_matrix(false)
    }
    #[test]
    fn joint_owned_graph_recovery_and_ledger_at_twenty_one_cuts() -> Result<()> {
        recovery_matrix(true)
    }
    fn recovery_matrix(joint: bool) -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                GraphCut::Hold,
                GraphCut::JournalPartial,
                GraphCut::JournalBytes,
                GraphCut::JournalFile,
                GraphCut::Journal,
                GraphCut::SourcePartial,
                GraphCut::SourceCandidate,
                GraphCut::PackedBefore,
                GraphCut::PackedPartial(1),
                GraphCut::PackedPartial(12),
                GraphCut::PackedPartial(13),
                GraphCut::PackedPartial(4096),
                GraphCut::PackedAfterData,
                GraphCut::PackedPartial(usize::MAX),
                GraphCut::PackedBeforeHead,
                GraphCut::PackedAfterHead,
                GraphCut::Marker,
                GraphCut::JournalUnlink,
                GraphCut::JournalCleanupBarrier,
                GraphCut::CandidateUnlink,
                GraphCut::MarkerUnlink,
            ] {
                let (_temp, mut source, mut f, journal) = if joint {
                    setup_joint(8, kind)?
                } else {
                    setup_graph(8, kind)?
                };
                let group = plan(&mut source, 8)?;
                let source_size = source.size();
                let selection = f.head.clone();
                if matches!(
                    cut,
                    GraphCut::Hold
                        | GraphCut::JournalPartial
                        | GraphCut::JournalBytes
                        | GraphCut::JournalFile
                ) {
                    assert!(admit_pending(&mut source, &mut f, &journal, &group, cut).is_err());
                } else {
                    persist_pending(&mut source, &mut f, &journal, &group)?;
                    assert_eq!(source.size(), source_size);
                    assert_eq!(f.head.semantic, selection.semantic);
                    assert_eq!(f.head.active, selection.active);
                    if cut != GraphCut::Journal {
                        let cut = if cut == GraphCut::PackedPartial(usize::MAX) {
                            let hold = f
                                .head
                                .gate
                                .holds
                                .values()
                                .next()
                                .cloned()
                                .ok_or("original hold")?;
                            let (_, plan) = prepare(&mut source, &mut f, &group, Some(&hold))?;
                            GraphCut::PackedPartial(
                                usize::try_from(plan.append.data.bytes).map_err(error)? + 1,
                            )
                        } else {
                            cut
                        };
                        assert!(finish(&mut f, &mut source, &journal, cut).is_err());
                    }
                }
                let hold = f.head.gate.holds.values().next().cloned();
                let token = f.head.gate.last_token;
                let source_dir = source.dir.clone();
                let physical_dir = f.dir.clone();
                drop(source);
                drop(f);
                let mut source = Store::open(&source_dir)?;
                let mut f = Fixture::reopen_combined(&physical_dir)?;
                if let Some(hold) = &hold {
                    assert_eq!(f.head.gate.holds.get(&hold.token), Some(hold));
                }
                finish(&mut f, &mut source, &journal, GraphCut::None)?;
                if hold.is_some() {
                    assert_eq!(f.head.gate.last_token, token);
                }
                for r in &group.records {
                    original(&mut f, r)?;
                }
                assert!(group.records.iter().any(|r| r.unchanged));
                assert!(group.records.iter().any(|r| r.fixture_inverse_of.is_some()));
                assert!(f.head.gate.holds.is_empty());
                assert!(!journal.join("group").exists());
                if joint {
                    let selected = f.head.clone();
                    finish(&mut f, &mut source, &journal, GraphCut::None)?;
                    assert_eq!(f.head, selected);
                    assert!(f.head.gate.ledger.is_some());
                    assert!(
                        f.head.inventory.active.is_some() && f.head.inventory.recovery.is_some()
                    );
                }
                audit_graph(&mut f)?;
                f.audit_combined()?;
                full_audit(&mut source)?;
            }
        }
        Ok(())
    }
    #[test]
    fn malformed_graph_group_refuses_before_journal_or_checkpoint_bytes() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut source, mut f, journal) = setup_graph(8, kind)?;
            let group = plan(&mut source, 8)?;
            let size = source.size();
            for mutation in 0..4 {
                let mut bad = group.clone();
                match mutation {
                    0 => bad.records[0].request = hash(b"different request"),
                    1 => bad.records[1] = bad.records[0].clone(),
                    2 => bad.records[0].unchanged = !bad.records[0].unchanged,
                    _ => bad.after.authored_revision += 1,
                }
                assert!(persist_pending(&mut source, &mut f, &journal, &bad).is_err());
                assert!(!journal.join("group").exists());
                assert_eq!(source.size(), size);
            }
        }
        Ok(())
    }
    #[test]
    fn pending_original_retry_conflict_and_torn_journal_preserve_evidence() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup_graph(8, Kind::Btree)?;
        let group = plan(&mut source, 8)?;
        persist_pending(&mut source, &mut f, &journal, &group)?;
        let bytes = fs::read(journal.join("group")).map_err(error)?;
        assert!(persist_pending(&mut source, &mut f, &journal, &group).is_err());
        let replacement = plan(&mut source, 1)?;
        fs::write(journal.join("group"), canonical(&replacement)?).map_err(error)?;
        let physical = f.head.clone();
        assert!(finish(&mut f, &mut source, &journal, GraphCut::None).is_err());
        assert_eq!(f.head, physical);
        assert!(journal.join("group").exists());
        fs::write(journal.join("group"), &bytes[..bytes.len() / 2]).map_err(error)?;
        finish(&mut f, &mut source, &journal, GraphCut::None)?;
        let before = f.head.clone();
        assert!(f.retry(&group.records[0].id, &hash(b"conflict")).is_err());
        original(&mut f, &group.records[0])?;
        assert_eq!(f.head, before);
        Ok(())
    }
    #[test]
    fn low_capacity_graph_admission_has_no_journal_source_or_pack_effect() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut source, mut f, journal) = setup_graph(8, kind)?;
            let mut gate = f.head.gate.clone();
            let domain = gate.domains.get_mut(&0).ok_or("domain")?;
            domain.limit = domain.charged + 4 * CONTROL;
            f.select_gate(gate)?;
            let group = plan(&mut source, 8)?;
            let old = f.head.clone();
            let snapshot = f.snapshot()?;
            let source_size = source.size();
            f.c = Counters::default();
            assert!(persist_pending(&mut source, &mut f, &journal, &group).is_err());
            assert_eq!(f.head, old);
            assert_eq!(f.snapshot()?, snapshot);
            assert_eq!(source.size(), source_size);
            assert_eq!(fs::read_dir(&journal).map_err(error)?.count(), 0);
            assert!(f.head.gate.holds.is_empty());
        }
        Ok(())
    }
    #[test]
    fn altered_graph_control_domain_with_same_token_retains_all_evidence() -> Result<()> {
        altered_control(false)
    }
    #[test]
    fn joint_graph_altered_ledger_candidate_retains_original_hold_and_evidence() -> Result<()> {
        altered_control(true)
    }
    fn altered_control(joint: bool) -> Result<()> {
        let (_temp, mut source, mut f, journal) = if joint {
            setup_joint(8, Kind::Btree)?
        } else {
            setup_graph(8, Kind::Btree)?
        };
        let group = plan(&mut source, 8)?;
        persist_pending(&mut source, &mut f, &journal, &group)?;
        assert!(finish(&mut f, &mut source, &journal, GraphCut::PackedBeforeHead).is_err());
        let original = fs::read(f.dir.join("candidate")).map_err(error)?;
        let mut candidate: serde_json::Value = serde_json::from_slice(&original).map_err(error)?;
        let changed = if joint {
            &mut candidate["gate"]["ledger"]["source"]["charged_high_water"]
        } else {
            &mut candidate["gate"]["domains"]["0"]["limit"]
        };
        *changed = json!(changed.as_u64().ok_or("candidate integer")? + 1);
        fs::write(
            f.dir.join("candidate"),
            serde_json::to_vec(&candidate).map_err(error)?,
        )
        .map_err(error)?;
        let before = f.head.clone();
        assert!(finish(&mut f, &mut source, &journal, GraphCut::None).is_err());
        assert_eq!(f.head, before);
        assert!(journal.join("group").exists() && f.dir.join("intent").exists());
        fs::write(f.dir.join("candidate"), original).map_err(error)?;
        finish(&mut f, &mut source, &journal, GraphCut::None)?;
        Ok(())
    }
    #[test]
    fn conflicting_partial_graph_payload_preserves_original_hold_and_journal() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup_graph(8, Kind::Btree)?;
        let group = plan(&mut source, 8)?;
        persist_pending(&mut source, &mut f, &journal, &group)?;
        let hold = f.head.gate.holds.values().next().cloned().ok_or("hold")?;
        assert!(finish(&mut f, &mut source, &journal, GraphCut::PackedPartial(13)).is_err());
        let path = f.data.path(hold.origin.data_pack);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(error)?;
        file.seek(SeekFrom::Start(hold.origin.data_end + 12))
            .map_err(error)?;
        let mut byte = [0];
        file.read_exact(&mut byte).map_err(error)?;
        file.seek(SeekFrom::Start(hold.origin.data_end + 12))
            .map_err(error)?;
        file.write_all(&[byte[0] ^ 1]).map_err(error)?;
        let source_dir = source.dir.clone();
        let physical_dir = f.dir.clone();
        drop(source);
        drop(f);
        let mut source = Store::open(&source_dir)?;
        let mut f = Fixture::reopen_combined(&physical_dir)?;
        assert!(finish(&mut f, &mut source, &journal, GraphCut::None).is_err());
        assert_eq!(f.head.gate.holds.get(&hold.token), Some(&hold));
        assert!(journal.join("group").exists());
        assert_eq!(f.head.semantic, group.old);
        Ok(())
    }
    #[test]
    fn joint_graph_final_settlement_two_cuts_reopen_and_charge_once() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                GraphCut::SettlementBeforeHead,
                GraphCut::SettlementAfterHead,
            ] {
                let (_temp, mut source, mut f, journal) = setup_joint(8, kind)?;
                let group = plan(&mut source, 8)?;
                persist_pending(&mut source, &mut f, &journal, &group)?;
                let token = f.head.gate.last_token;
                assert!(finish(&mut f, &mut source, &journal, cut).is_err());
                assert!(f.dir.join("intent").exists());
                let candidate_bytes = fs::read(f.dir.join("candidate")).map_err(error)?;
                let mut changed: Selection =
                    serde_json::from_slice(&candidate_bytes).map_err(error)?;
                changed.gate.domains.get_mut(&0).ok_or("domain")?.charged += 1;
                fs::write(
                    f.dir.join("candidate"),
                    serde_json::to_vec(&changed).map_err(error)?,
                )
                .map_err(error)?;
                let interrupted = f.head.clone();
                assert!(finish(&mut f, &mut source, &journal, GraphCut::None).is_err());
                assert_eq!(f.head, interrupted);
                assert!(f.dir.join("intent").exists());
                fs::write(f.dir.join("candidate"), candidate_bytes).map_err(error)?;
                let source_dir = source.dir.clone();
                let physical_dir = f.dir.clone();
                drop(source);
                drop(f);
                let mut source = Store::open(&source_dir)?;
                let mut f = Fixture::reopen_combined(&physical_dir)?;
                finish(&mut f, &mut source, &journal, GraphCut::None)?;
                assert_eq!(f.head.gate.last_token, token);
                assert!(f.head.gate.holds.is_empty() && !f.dir.join("intent").exists());
                for record in &group.records {
                    original(&mut f, record)?;
                }
                let settled = f.head.clone();
                drop(source);
                drop(f);
                let mut source = Store::open(&source_dir)?;
                let mut f = Fixture::reopen_combined(&physical_dir)?;
                finish(&mut f, &mut source, &journal, GraphCut::None)?;
                assert_eq!(f.head, settled);
                audit_graph(&mut f)?;
                f.audit_combined()?;
            }
        }
        Ok(())
    }
    #[test]
    fn joint_graph_rollover_refuses_before_original_hold_or_any_write() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for data in [true, false] {
                let (_temp, mut source, mut f, journal) = setup_joint_padded(8, kind, Some(data))?;
                let group = plan(&mut source, 32)?;
                let head = f.head.clone();
                let snapshot = f.snapshot()?;
                let size = source.size();
                let refusal = persist_pending(&mut source, &mut f, &journal, &group).unwrap_err();
                assert!(refusal.contains("rollover"), "{refusal}");
                assert_eq!(f.head, head);
                assert_eq!(f.snapshot()?, snapshot);
                assert_eq!(source.size(), size);
                assert!(f.staged.is_none() && source.preview.is_none());
                assert_eq!(fs::read_dir(journal).map_err(error)?.count(), 0);
            }
        }
        Ok(())
    }
    #[test]
    fn joint_graph_cannot_bless_a_corrupt_old_owned_prefix() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup_joint(8, Kind::Btree)?;
        let group = plan(&mut source, 8)?;
        let head = f.head.clone();
        let size = source.size();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(f.data.path(f.data.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(0)).map_err(error)?;
        let mut byte = [0];
        file.read_exact(&mut byte).map_err(error)?;
        file.seek(SeekFrom::Start(0)).map_err(error)?;
        file.write_all(&[byte[0] ^ 1]).map_err(error)?;
        assert!(persist_pending(&mut source, &mut f, &journal, &group).is_err());
        assert_eq!(f.head, head);
        assert_eq!(source.size(), size);
        assert!(f.staged.is_none() && source.preview.is_none());
        assert_eq!(fs::read_dir(journal).map_err(error)?.count(), 0);
        Ok(())
    }
    #[test]
    fn joint_graph_source_and_tip_growth_settle_once_without_refund() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup_joint(1, Kind::Radix)?;
        for size in [1, 8, 8] {
            let before = f.head.gate.domains[&0].charged;
            let ledger_before = serde_json::to_value(&f.head.gate.ledger).map_err(error)?;
            let group = plan(&mut source, size)?;
            persist_pending(&mut source, &mut f, &journal, &group)?;
            finish(&mut f, &mut source, &journal, GraphCut::None)?;
            let ledger_after = serde_json::to_value(&f.head.gate.ledger).map_err(error)?;
            assert_eq!(ledger_before["control"], ledger_after["control"]);
            assert_eq!(
                ledger_before["source"]["standing_scratch"],
                ledger_after["source"]["standing_scratch"]
            );
            let tips = |v: &serde_json::Value| {
                v["tips"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t["charged_high_water"].as_u64().unwrap())
                    .sum::<u64>()
            };
            let source_charge =
                |v: &serde_json::Value| v["source"]["charged_high_water"].as_u64().unwrap();
            let growth = tips(&ledger_after) - tips(&ledger_before) + source_charge(&ledger_after)
                - source_charge(&ledger_before);
            assert_eq!(f.head.gate.domains[&0].charged, before + growth);
            let head = f.head.clone();
            finish(&mut f, &mut source, &journal, GraphCut::None)?;
            assert_eq!(f.head, head);
            audit_graph(&mut f)?;
            f.audit_combined()?;
        }
        Ok(())
    }
    #[test]
    fn graph_original_noop_advances_prefix_without_authored_revision() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup_graph(1, Kind::Radix)?;
        let group = plan(&mut source, 1)?;
        assert!(group.records[0].unchanged);
        assert_eq!(group.before.coordinate()?, group.after.coordinate()?);
        assert_ne!(group.before.prefix, group.after.prefix);
        let before = f.head.clone();
        persist_pending(&mut source, &mut f, &journal, &group)?;
        assert_eq!(
            physical_state(&mut f, &before.active, &before.semantic.active)?,
            group.before
        );
        assert!(
            f.retry(&group.records[0].id, &group.records[0].request)
                .is_err()
        );
        finish(&mut f, &mut source, &journal, GraphCut::None)?;
        original(&mut f, &group.records[0])?;
        audit_graph(&mut f)?;
        Ok(())
    }
}
