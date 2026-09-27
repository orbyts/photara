//! Genuine Graph journal composition with witnessed physical rollover.
use super::super::typed_inventory::{fresh_generation, rollover};
#[allow(
    clippy::wildcard_imports,
    reason = "Child shares original Graph journal"
)]
use super::*;
use std::os::unix::fs::MetadataExt;

#[cfg(test)]
#[path = "rollover_dispatch/review_tests.rs"]
mod review_tests;
#[cfg(test)]
#[path = "rollover_dispatch/turnover.rs"]
mod turnover;

fn prepare(
    source: &mut Store,
    f: &mut Fixture,
    group: &Group,
    hold: Option<&Liability>,
    dedicated: bool,
) -> Result<(crate::graph_preview::Preview, rollover::Draft, Binding)> {
    let previous = f.head.clone();
    let tips = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    if let Some(hold) = hold {
        f.head = hold
            .graph
            .as_ref()
            .ok_or("rollover original Graph")?
            .control_base
            .clone();
        f.data.pack = hold.origin.data_pack;
        f.data.end = hold.origin.data_end;
        f.meta.pack = hold.origin.meta_pack;
        f.meta.end = hold.origin.meta_end;
    }
    if let Err(e) = source.begin_graph_preview(group.old.clone(), group.old_end) {
        f.head = previous;
        (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
        f.staged = None;
        f.planned_metadata = None;
        f.cache.clear();
        return Err(e);
    }
    let result: Result<(rollover::Draft, Binding)> = (|| {
        let ownership = if let Some(h) = hold {
            h.graph
                .as_ref()
                .and_then(|b| b.ownership.clone())
                .ok_or("original ownership")?
        } else {
            ownership_graph::Original::capture(f)?
        };
        let original_source = retirement_ledger::source_charge(f)?;
        let control_base = f.head.clone();
        let (data_base, meta_base) = if let Some(h) = hold {
            let b = h.graph.as_ref().ok_or("original binding")?;
            (b.data_base.clone(), b.meta_base.clone())
        } else {
            (
                recipe::Prefix::capture(&f.data)?,
                recipe::Prefix::capture(&f.meta)?,
            )
        };
        let candidate = build_candidate(source, group)?;
        source.publish(&candidate)?;
        let draft = rollover::prepare(
            f,
            source,
            group.old_end,
            hold.and_then(|h| h.rollover.as_ref()),
            dedicated,
        )?;
        Ok((
            draft,
            Binding {
                group_json: String::from_utf8(canonical(group)?).map_err(error)?,
                control_base,
                source_tail_sha: String::new(),
                source_tail_len: 0,
                data_base,
                meta_base,
                ownership: Some(ownership),
                source_charge: Some(original_source),
            },
        ))
    })();
    f.head = previous;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
    f.staged = None;
    f.planned_metadata = None;
    f.cache.clear();
    let preview = source.take_graph_preview()?;
    let (draft, mut binding) = result?;
    binding.source_tail_sha = hash(&preview.tail());
    binding.source_tail_len = preview.tail().len() as u64;
    if let Some(hold) = hold {
        // Prefix capture above cannot reinterpret a grown physical tail. Resume
        // reconstruction uses the admitted prefix evidence and exact original
        // planning coordinates, with reads supplied by the source preview.
        let original = hold.graph.as_ref().ok_or("rollover original Graph")?;
        binding.data_base = original.data_base.clone();
        binding.meta_base = original.meta_base.clone();
        ensure(
            binding == *original,
            "rollover rebuilt Graph binding changed",
        )?;
        ensure(
            draft.initial().initial_admission_sha()?
                == hold
                    .rollover
                    .as_ref()
                    .ok_or("rollover original")?
                    .initial_admission_sha()?,
            "rollover admission reconstruction changed",
        )?;
    }
    Ok((preview, draft, binding))
}
fn persist(source: &mut Store, f: &mut Fixture, dir: &Path, group: &Group) -> Result<u64> {
    persist_cut(source, f, dir, group, GraphCut::None)
}
fn persist_cut(
    source: &mut Store,
    f: &mut Fixture,
    dir: &Path,
    group: &Group,
    cut: GraphCut,
) -> Result<u64> {
    ensure(
        f.head.gate.holds.is_empty() && fs::read_dir(dir).map_err(error)?.next().is_none(),
        "rollover requires empty journal and idle gate",
    )?;
    ensure(
        source.head()? == group.old && f.head.semantic == group.old,
        "rollover requires exact current Graph source and packed selection",
    )?;
    ensure(
        canonical(group)?.len() <= JOURNAL_MAX / 2,
        "rollover canonical group admission bound",
    )?;
    validate(source, group)?;
    retirement_ledger::registered_source(f, source)?;
    let (_preview, draft, binding) = prepare(source, f, group, None, false)?;
    let original = fresh_generation::reserve_joint(f, binding, draft.initial())?;
    let promoted = fresh_generation::resume_joint(f, &original)?;
    journal_preflight(
        dir,
        promoted.graph.as_ref().ok_or("binding")?,
        &promoted.target,
        group,
    )?;
    persist_original_journal(
        dir,
        promoted.graph.as_ref().ok_or("rollover Graph binding")?,
        cut,
    )?;
    Ok(promoted.graph.as_ref().ok_or("binding")?.group_json.len() as u64)
}
fn recover_hold(f: &Fixture) -> Result<Option<Liability>> {
    if let Some(h) = f.head.gate.holds.values().find(|h| h.rollover.is_some()) {
        return Ok(Some(h.clone()));
    }
    if f.dir.join("intent").exists() {
        let old = fresh_generation::control(f, "intent")?.ok_or("rollover intent")?;
        return Ok(old
            .gate
            .holds
            .values()
            .find(|h| h.rollover.is_some())
            .cloned());
    }
    Ok(None)
}
fn finish(f: &mut Fixture, source: &mut Store, dir: &Path, cut: rollover::Cut) -> Result<()> {
    finish_cut(f, source, dir, cut, GraphCut::None)
}
fn finish_cut(
    f: &mut Fixture,
    source: &mut Store,
    dir: &Path,
    cut: rollover::Cut,
    journal_cut: GraphCut,
) -> Result<()> {
    let Some(original_hold) = recover_hold(f)? else {
        ensure(
            !f.dir.join("candidate").exists()
                && !f.dir.join("HEAD.next").exists()
                && fs::read_dir(dir).map_err(error)?.next().is_none(),
            "unknown completed rollover controls",
        )?;
        return Ok(());
    };
    let binding = original_hold
        .graph
        .as_ref()
        .ok_or("rollover original Graph")?;
    binding
        .source_charge
        .as_ref()
        .ok_or("original retained source charge")?
        .observe_growth(source)?;
    let group: Group = serde_json::from_str(&binding.group_json).map_err(error)?;
    ensure(
        canonical(&group)? == binding.group_json.as_bytes(),
        "rollover canonical Graph group",
    )?;
    let selected_source = source.head()?;
    ensure(
        selected_source == group.old || selected_source == original_hold.target,
        "unknown rollover source selector before effects",
    )?;
    validate(source, &group)?;
    journal_preflight(dir, binding, &original_hold.target, &group)?;
    let (preview, draft, _) = prepare(source, f, &group, Some(&original_hold), false)?;
    preview.verify_present(source)?;
    let payload = draft.payload();
    rollover::reconcile(f, source, &payload, cut)?;
    if let Some(held) = f.head.gate.holds.values().next().cloned()
        && held
            .rollover
            .as_ref()
            .is_some_and(rollover::Rollover::is_birth)
    {
        fresh_generation::resume_joint(f, &held)?;
    }
    if f.head.gate.holds.is_empty() {
        return Ok(());
    }
    // Journal may have been removed after verified publication. The original
    // binding remains through cleanup and therefore needs no replacement token.
    let completing = f.head.semantic == original_hold.target && !dir.join("group").exists();
    if !completing {
        persist_original_journal(dir, binding, GraphCut::None)?;
    }
    preview.persist(source, journal_cut == GraphCut::SourcePartial)?;
    if !completing && !dir.join("candidate").exists() {
        write_new(dir, "candidate", &original_hold.target)?;
    }
    if journal_cut == GraphCut::SourceCandidate {
        return Err("cut after rollover source candidate".into());
    }
    verify_source(source, &original_hold.target, &group)?;
    if source.head()? != original_hold.target {
        source.publish(&original_hold.target)?;
    }
    rollover::advance(f, source, &payload, cut)?;
    ensure(
        f.head.semantic == original_hold.target,
        "rollover selected Graph target",
    )?;
    let h = f.head.clone();
    ensure(
        physical_state(f, &h.active, &h.semantic.active)? == group.after,
        "rollover final Graph state",
    )?;
    for record in &group.records {
        original(f, record)?;
    }
    if !completing {
        let marker = checkpoint_marker(&original_hold.target, &group.after)?;
        if !dir.join("marker").exists() {
            write_new(dir, "marker", &marker)?;
        }
        if journal_cut == GraphCut::Marker {
            return Err("cut after rollover marker".into());
        }
        fs::remove_file(dir.join("group")).map_err(error)?;
        if journal_cut == GraphCut::JournalUnlink {
            return Err("cut after rollover journal unlink".into());
        }
        File::open(dir)
            .and_then(|file| file.sync_all())
            .map_err(error)?;
        if journal_cut == GraphCut::JournalCleanupBarrier {
            return Err("cut after rollover journal cleanup barrier".into());
        }
    }
    cleanup_completed(source, f, dir, journal_cut)?;
    rollover::cleanup(f, cut)
}

fn setup_measurement(
    kind: Kind,
    pads: &[bool],
) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
    let temp = tempfile::tempdir().map_err(error)?;
    let mut source = Store::open(&temp.path().join("source"))?;
    build_graph(&mut source, 8, kind)?;
    let mut f = Fixture::create(&temp.path().join("physical"), &mut source)?;
    for data in pads {
        let arena = if *data { &mut f.data } else { &mut f.meta };
        let frame = if *data { 12 } else { 4 };
        let target = PACK - 8 * 1024;
        while arena.end + frame < target {
            let len = usize::try_from((target - arena.end - frame).min(4092)).map_err(error)?;
            arena.append(
                &vec![0; len],
                if *data { Some(u64::MAX - 1) } else { None },
                &mut f.c,
            )?;
        }
    }
    let h = f.head.clone();
    f.publish(h.active, h.recovery, h.semantic, super::super::Cut::None)?;
    retirement_ledger::initialize(&mut f)?;
    retirement_ledger::enroll_source(&mut f, &source)?;
    let journal = temp.path().join("journal");
    fs::create_dir(&journal).map_err(error)?;
    Ok((temp, source, f, journal))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn setup(
        kind: Kind,
        pads: &[bool],
    ) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
        setup_measurement(kind, pads)
    }
    pub(super) fn dedicated(
        source: &mut Store,
        f: &mut Fixture,
        journal: &Path,
        group: &Group,
    ) -> Result<Liability> {
        let (_, draft, binding) = prepare(source, f, group, None, true)?;
        let original = fresh_generation::reserve_joint(f, binding, draft.initial())?;
        let promoted = fresh_generation::resume_joint(f, &original)?;
        persist_original_journal(
            journal,
            promoted.graph.as_ref().ok_or("binding")?,
            GraphCut::None,
        )?;
        Ok(promoted)
    }
    #[test]
    fn real_graph_rollover_journal_source_and_cleanup_cuts() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                GraphCut::JournalPartial,
                GraphCut::JournalBytes,
                GraphCut::JournalFile,
                GraphCut::SourcePartial,
                GraphCut::SourceCandidate,
                GraphCut::Marker,
                GraphCut::JournalUnlink,
                GraphCut::JournalCleanupBarrier,
                GraphCut::CandidateUnlink,
                GraphCut::MarkerUnlink,
            ] {
                let (_temp, mut source, mut f, journal) = setup(kind, &[true, false])?;
                let group = plan(&mut source, 8)?;
                if matches!(
                    cut,
                    GraphCut::JournalPartial | GraphCut::JournalBytes | GraphCut::JournalFile
                ) {
                    assert!(persist_cut(&mut source, &mut f, &journal, &group, cut).is_err());
                } else {
                    persist(&mut source, &mut f, &journal, &group)?;
                    assert!(
                        finish_cut(&mut f, &mut source, &journal, rollover::Cut::None, cut)
                            .is_err()
                    );
                }
                let token = f.head.gate.last_token;
                let physical = f.dir.clone();
                let logical = source.dir.clone();
                drop(f);
                drop(source);
                let mut f = Fixture::reopen_combined(&physical)?;
                let mut source = Store::open(&logical)?;
                finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
                assert_eq!(f.head.gate.last_token, token);
                for record in &group.records {
                    original(&mut f, record)?;
                }
                f.audit_combined()?;
                retirement_ledger::audit_attribution(&mut f)?;
            }
        }
        Ok(())
    }
    #[test]
    fn real_graph_rollover_partial_unbound_birth_never_creates_journal() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup(Kind::Btree, &[true, false])?;
        let group = plan(&mut source, 8)?;
        let (_, draft, binding) = prepare(&mut source, &mut f, &group, None, true)?;
        let original =
            fresh_generation::reserve_joint_partial_marker(&mut f, binding, draft.initial())?;
        let physical = f.dir.clone();
        let logical = source.dir.clone();
        drop(f);
        drop(source);
        let mut f = Fixture::reopen_combined(&physical)?;
        let mut source = Store::open(&logical)?;
        let old = f.head.clone();
        let snapshot = f.snapshot()?;
        let size = source.size();
        assert!(finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err());
        assert_eq!(f.head, old);
        assert_eq!(f.snapshot()?, snapshot);
        assert_eq!(source.size(), size);
        assert_eq!(f.head.gate.holds.get(&original.token), Some(&original));
        assert_eq!(fs::read_dir(journal).map_err(error)?.count(), 0);
        Ok(())
    }
    fn write_control(path: &Path, value: serde_json::Value) -> Result<()> {
        let typed: Selection = serde_json::from_value(value).map_err(error)?;
        fs::write(path, serde_json::to_vec(&typed).map_err(error)?).map_err(error)
    }
    #[test]
    fn real_graph_rollover_changed_transfer_witness_and_unknown_controls_refuse() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup(Kind::Btree, &[true, false])?;
        let group = plan(&mut source, 8)?;
        let held = dedicated(&mut source, &mut f, &journal, &group)?;
        assert!(
            finish(
                &mut f,
                &mut source,
                &journal,
                rollover::Cut::PublicationBeforeHead
            )
            .is_err()
        );
        let candidate = fresh_generation::control(&f, "candidate")?.ok_or("candidate")?;
        let mut value = serde_json::to_value(&candidate).map_err(error)?;
        let key = held.token.to_string();
        for item in
            [&mut value["gate"]["holds"][&key]["rollover"]["phase"]["Published"]["consumed"]["0"]]
        {
            *item = json!(item.as_u64().ok_or("consumed")? + 1);
        }
        let n = value["gate"]["ledger"]["source"]["charged_high_water"]
            .as_u64()
            .ok_or("source charge")?
            + 1;
        value["gate"]["ledger"]["source"]["charged_high_water"] = json!(n);
        value["gate"]["holds"][&key]["rollover"]["phase"]["Published"]["ledger"]["source"]["charged_high_water"] =
            json!(n);
        let n = value["gate"]["domains"]["0"]["charged"]
            .as_u64()
            .ok_or("charge")?
            + 1;
        value["gate"]["domains"]["0"]["charged"] = json!(n);
        let altered: Selection = serde_json::from_value(value.clone()).map_err(error)?;
        let old = f.head.clone();
        assert!(f.select_gate(altered.gate).is_err());
        assert_eq!(f.head, old);
        write_control(&f.dir.join("candidate"), value)?;
        assert!(finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err());
        assert_eq!(f.head, old);
        assert!(journal.join("group").exists());
        fs::write(
            f.dir.join("candidate"),
            serde_json::to_vec(&candidate).map_err(error)?,
        )
        .map_err(error)?;
        finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
        // Unknown orphan controls are preserved, including private valid JSON,
        // symlink controls and noncanonical/unknown-field selector encodings.
        for kind in 0..3 {
            let target = f.dir.join("candidate");
            match kind {
                0 => {
                    fs::write(&target, serde_json::to_vec(&f.head).map_err(error)?)
                        .map_err(error)?;
                }
                1 => {
                    std::os::unix::fs::symlink(f.dir.join("HEAD"), &target).map_err(error)?;
                }
                _ => {
                    let mut value = serde_json::to_value(&f.head).map_err(error)?;
                    value["unknown"] = json!(true);
                    fs::write(&target, serde_json::to_vec(&value).map_err(error)?)
                        .map_err(error)?;
                }
            }
            assert!(finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err());
            assert!(fs::symlink_metadata(&target).is_ok());
            fs::remove_file(target).map_err(error)?;
        }
        let (_temp, mut source, mut f, journal) = setup(Kind::Radix, &[true, false])?;
        let group = plan(&mut source, 8)?;
        dedicated(&mut source, &mut f, &journal, &group)?;
        let path = f.data.path(1);
        let original = f.dir.join("held-original-generation");
        fs::rename(&path, &original).map_err(error)?;
        fs::copy(&original, &path).map_err(error)?;
        let before = f.head.clone();
        assert!(finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err());
        assert_eq!(f.head, before);
        assert!(original.exists() && journal.join("group").exists());
        Ok(())
    }
    #[test]
    fn real_graph_rollover_staged_cleanup_and_low_capacity_keep_original_evidence() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup(Kind::Radix, &[true, false])?;
        let group = plan(&mut source, 8)?;
        dedicated(&mut source, &mut f, &journal, &group)?;
        assert!(
            finish(
                &mut f,
                &mut source,
                &journal,
                rollover::Cut::PublicationStagedHead
            )
            .is_err()
        );
        assert!(
            finish(
                &mut f,
                &mut source,
                &journal,
                rollover::Cut::ReconcileStagedUnlink
            )
            .is_err()
        );
        assert!(!f.dir.join("HEAD.next").exists() && f.dir.join("intent").exists());
        finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
        let (_temp, mut source, mut f, journal) = setup(Kind::Btree, &[true, false])?;
        let mut gate = f.head.gate.clone();
        let d = gate.domains.get_mut(&0).ok_or("domain")?;
        d.limit = d.charged;
        f.select_gate(gate)?;
        let group = plan(&mut source, 8)?;
        let old = f.head.clone();
        let snapshot = f.snapshot()?;
        let size = source.size();
        assert!(persist(&mut source, &mut f, &journal, &group).is_err());
        assert_eq!(f.head, old);
        assert_eq!(f.snapshot()?, snapshot);
        assert_eq!(source.size(), size);
        assert!(f.staged.is_none() && f.planned_metadata.is_none());
        assert_eq!(fs::read_dir(journal).map_err(error)?.count(), 0);
        Ok(())
    }
    #[test]
    fn real_graph_rollover_data_metadata_simultaneous_and_created_sealed() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for pads in [vec![true], vec![false], vec![true, false]] {
                let (_temp, mut source, mut f, journal) = setup(kind, &pads)?;
                let pin = f.acquire(PinClass::ReaderLease, false)?;
                let old = f.head.clone();
                let group = plan(&mut source, 8)?;
                let held = dedicated(&mut source, &mut f, &journal, &group)?;
                finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
                assert_eq!(f.head.gate.last_token, held.token);
                assert!(f.head.data_pack > old.data_pack && f.head.meta_pack > old.meta_pack);
                assert!(
                    f.head.data_pack >= old.data_pack + 2 || f.head.meta_pack >= old.meta_pack + 2
                );
                f.audit_combined()?;
                retirement_ledger::audit_attribution(&mut f)?;
                for data in [true, false] {
                    let pack = if data { old.data_pack } else { old.meta_pack };
                    rollover::assert_sealed_owned(&mut f, data, pack)?;
                    assert!(f.inventory_retirement_guard(pack, data).is_err());
                }
                assert_eq!(f.head.gate.pins.get(&pin.token), Some(&pin));
                for record in &group.records {
                    original(&mut f, record)?;
                }
                let settled = f.head.clone();
                finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
                assert_eq!(f.head, settled);
            }
        }
        Ok(())
    }
    #[test]
    fn real_graph_rollover_original_token_all_physical_phase_cuts() -> Result<()> {
        use rollover::Cut as C;
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                C::BindingBeforeHead,
                C::BindingStagedHead,
                C::BindingAfterHead,
                C::PayloadBefore,
                C::PayloadPartial(1),
                C::PayloadPartial(13),
                C::PayloadAfterData,
                C::PayloadBarrier,
                C::FinalizationBeforeHead,
                C::FinalizationStagedHead,
                C::FinalizationAfterHead,
                C::FinalizerPartial(1),
                C::FinalizerPartial(13),
                C::FinalizerAfterData,
                C::FinalizerBarrier,
                C::PublicationBeforeHead,
                C::PublicationStagedHead,
                C::PublicationAfterHead,
                C::CleanupBeforeHead,
                C::CleanupStagedHead,
                C::CleanupAfterHead,
            ] {
                let (_temp, mut source, mut f, journal) = setup(kind, &[true, false])?;
                let group = plan(&mut source, 8)?;
                let held = dedicated(&mut source, &mut f, &journal, &group)?;
                assert!(
                    finish(&mut f, &mut source, &journal, cut).is_err(),
                    "missing cut {cut:?}"
                );
                let physical = f.dir.clone();
                let logical = source.dir.clone();
                drop(f);
                drop(source);
                let mut f = Fixture::reopen_combined(&physical)?;
                let mut source = Store::open(&logical)?;
                finish(&mut f, &mut source, &journal, C::None)
                    .map_err(|e| format!("{kind:?} {cut:?}: {e}"))?;
                assert_eq!(f.head.gate.last_token, held.token);
                assert!(f.head.gate.holds.is_empty());
                for record in &group.records {
                    original(&mut f, record)?;
                }
                f.audit_combined()?;
                retirement_ledger::audit_attribution(&mut f)?;
                let settled = f.head.clone();
                drop(f);
                drop(source);
                let mut f = Fixture::reopen_combined(&physical)?;
                let mut source = Store::open(&logical)?;
                finish(&mut f, &mut source, &journal, C::None)?;
                assert_eq!(f.head, settled);
            }
        }
        Ok(())
    }
    #[test]
    fn real_graph_rollover_joint_birth_uses_original_graph_admission() -> Result<()> {
        let (_temp, mut source, mut f, journal) = setup(Kind::Btree, &[true])?;
        let group = plan(&mut source, 8)?;
        let (_, draft, binding) = prepare(&mut source, &mut f, &group, None, true)?;
        let held = fresh_generation::exercise_joint_birth(&mut f, binding, draft.initial())?;
        finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
        assert_eq!(f.head.gate.last_token, held.token);
        f.audit_combined()?;
        retirement_ledger::audit_attribution(&mut f)?;
        Ok(())
    }
    #[test]
    fn real_graph_rollover_pipeline_smoke_both_maps() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut source, mut f, journal) = setup_joint_padded(8, kind, Some(true))?;
            let group = plan(&mut source, 8)?;
            persist(&mut source, &mut f, &journal, &group)?;
            finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
            assert!(f.head.gate.holds.is_empty());
            f.audit_combined()?;
            retirement_ledger::audit_attribution(&mut f)?;
            audit_graph(&mut f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn setup_fixture(
    kind: Kind,
    pads: &[bool],
) -> Result<(tempfile::TempDir, Store, Fixture, PathBuf)> {
    tests::setup(kind, pads)
}
#[cfg(test)]
pub(super) fn admit_dedicated(
    source: &mut Store,
    f: &mut Fixture,
    journal: &Path,
    group: &Group,
) -> Result<Liability> {
    tests::dedicated(source, f, journal, group)
}

fn journal_preflight(dir: &Path, binding: &Binding, target: &Head, group: &Group) -> Result<()> {
    let marker = canonical(&checkpoint_marker(target, &group.after)?)?;
    let candidate = canonical(target)?;
    for entry in fs::read_dir(dir).map_err(error)? {
        let entry = entry.map_err(error)?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("unknown journal name")?;
        let expected = match name {
            "group" => binding.group_json.as_bytes(),
            "candidate" => &candidate,
            "marker" => &marker,
            _ => return Err("unknown rollover journal artifact".into()),
        };
        let path = entry.path();
        let named = fs::symlink_metadata(&path).map_err(error)?;
        ensure(
            named.is_file() && named.nlink() == 1 && named.len() <= JOURNAL_MAX as u64,
            "rollover private bounded journal artifact",
        )?;
        let fd = rustix::fs::open(
            &path,
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(error)?;
        let mut file = File::from(fd);
        let opened = file.metadata().map_err(error)?;
        ensure(
            opened.dev() == named.dev()
                && opened.ino() == named.ino()
                && opened.nlink() == 1
                && opened.len() == named.len(),
            "rollover journal artifact changed",
        )?;
        let mut bytes = vec![0; usize::try_from(opened.len()).map_err(error)?];
        file.read_exact(&mut bytes).map_err(error)?;
        ensure(
            file.metadata().map_err(error)?.len() == opened.len(),
            "rollover journal changed while reading",
        )?;
        ensure(
            if name == "group" {
                expected.starts_with(&bytes)
            } else {
                bytes == expected
            },
            "unknown original rollover journal bytes",
        )?;
    }
    Ok(())
}

pub(super) fn run_cli(args: &[String]) -> Result<()> {
    let sizes = if args.is_empty() {
        vec![8]
    } else {
        args.iter()
            .map(|v| v.parse().map_err(error))
            .collect::<Result<Vec<u64>>>()?
    };
    println!(
        "{}",
        json!({"fixture":"actual-v3-witnessed-graph-rollover","qualification":"disposable sync_all observations only",
        "scope":"bounded original-token births, payload, measured sealed attribution, finalizer corridor, atomic R-to-charge transfer and cleanup",
        "caveats":["bulk baseline with admitted framed padding; initialization excluded","both maps and natural/dedicated corridor modes",
            "source retained with separate standing scratch; no source retirement or filesystem availability credit",
            "original token and total admitted capacity never expanded","all source/journal/packed writes counted, including birth markers",
            "full closure audits after measured completion; original recipe/receipt checks inside completion",
            "controlled return cuts, no kill/power-loss qualification or permanent wire"]})
    );
    for size in sizes {
        ensure((1..=32).contains(&size), "finite rollover CLI group")?;
        for kind in [Kind::Radix, Kind::Btree] {
            for pads in [vec![true], vec![false], vec![true, false]] {
                for dedicated in [false, true] {
                    let (_temp, mut source, mut f, journal) = setup_measurement(kind, &pads)?;
                    let ledger_before = serde_json::to_value(&f.head.gate.ledger).map_err(error)?;
                    let charged_before = f.head.gate.domains[&0].charged;
                    source.stats = Stats::default();
                    f.c = Counters::default();
                    let start = Instant::now();
                    let group = plan(&mut source, size)?;
                    let planning_us = start.elapsed().as_micros();
                    let start = Instant::now();
                    let (_, draft, binding) =
                        prepare(&mut source, &mut f, &group, None, dedicated)?;
                    let original_hold =
                        fresh_generation::reserve_joint(&mut f, binding, draft.initial())?;
                    let promoted = fresh_generation::resume_joint(&mut f, &original_hold)?;
                    let marker_bytes = fresh_generation::promoted_generations(
                        promoted.birth.as_ref().ok_or("birth")?,
                    )?
                    .len();
                    persist_original_journal(
                        &journal,
                        promoted.graph.as_ref().ok_or("binding")?,
                        GraphCut::None,
                    )?;
                    let admission_us = start.elapsed().as_micros();
                    let admitted_counters = f.c.clone();
                    let start = Instant::now();
                    finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
                    let completion_us = start.elapsed().as_micros();
                    let journal_writes = canonical(&group)?.len() as u64
                        + canonical(&promoted.target)?.len() as u64
                        + canonical(&checkpoint_marker(&promoted.target, &group.after)?)?.len()
                            as u64;
                    let source_writes =
                        source.stats.object_write_bytes + source.stats.envelope_write_bytes;
                    let packed_writes =
                        f.c.data_write_bytes + f.c.meta_write_bytes + f.c.envelope_write_bytes;
                    let measured = f.c.clone();
                    let source_measured = source.stats.clone();
                    let selected = f.head.clone();
                    let snapshot = f.snapshot()?;
                    for record in &group.records {
                        original(&mut f, record)?;
                    }
                    let graph_audit = audit_graph(&mut f)?;
                    f.audit_combined()?;
                    retirement_ledger::audit_attribution(&mut f)?;
                    println!(
                        "{}",
                        json!({"kind":format!("{kind:?}"),"baseline":8,"group_size":size,
                        "padded_arenas":pads,"force_dedicated_corridors":dedicated,"born_generation_count":marker_bytes,
                        "planning_us":planning_us,"admission_birth_journal_us":admission_us,"checkpoint_cleanup_us":completion_us,
                        "original_token":original_hold.token,"immutable_admission_R":original_hold.by_domain,
                        "committed_growth_C":selected.gate.domains[&0].charged-charged_before,
                        "ledger_before":ledger_before,"ledger_after":selected.gate.ledger,
                        "selected_data_pack":selected.data_pack,"selected_meta_pack":selected.meta_pack,
                        "source_process_write_bytes":source_writes,"journal_file_write_bytes":journal_writes,
                        "packed_process_write_bytes_including_birth_markers":packed_writes,
                        "total_successful_process_write_bytes":source_writes+journal_writes+packed_writes,
                        "source_counters":source_measured,"packed_counters":measured,"admission_packed_counters":admitted_counters,
                        "graph_replay_audit":graph_audit,"physical_snapshot":snapshot,"qualified_accepted":false,"qualified_saved":false})
                    );
                }
            }
        }
    }
    Ok(())
}
