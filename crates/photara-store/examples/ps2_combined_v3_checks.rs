//! Independent adversarial checks of the actual disposable v3 implementation.
//! Included only by placement_v3's cfg(test) child hook; not a production codec.
use super::*;

#[test]
fn independent_current_append_pack_never_retires() -> Result<()> {
    let (_temp, _source, mut f) = setup(32, Kind::Btree)?;
    for data in [true, false] {
        let pack = if data { f.data.pack } else { f.meta.pack };
        let before = f.head.clone();
        let snapshot = f.snapshot()?;
        assert!(f.compact_combined(data, pack, Fault::None).is_err());
        assert!(
            if data {
                f.compact_data(pack, Cut::None, false)
            } else {
                f.compact_meta(pack, Cut::None, false)
            }
            .is_err()
        );
        assert_eq!(before, f.head);
        assert_eq!(snapshot, f.snapshot()?);
    }
    f.audit()?;
    Ok(())
}

#[test]
fn independent_append_hold_cannot_authorize_compaction() -> Result<()> {
    let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
    f.data.rotate(&mut f.c)?;
    let old_end = source.size();
    logical_group(&mut source, 3)?;
    assert!(
        f.checkpoint(&mut source, old_end, Fault::EnospcBefore)
            .is_err()
    );
    let before = f.head.clone();
    let snapshot = f.snapshot()?;
    assert!(f.compact_combined(true, 0, Fault::None).is_err());
    assert!(
        f.compact_data(0, Cut::None, false).is_err(),
        "append hold must not authorize an unrelated raw compaction"
    );
    assert_eq!(before, f.head);
    assert_eq!(snapshot, f.snapshot()?);
    assert!(f.data.path(0).exists());
    Ok(())
}

#[test]
fn independent_reopened_hold_requires_original_request() -> Result<()> {
    let (_temp, mut source, mut f) = setup(32, Kind::Radix)?;
    f.data.rotate(&mut f.c)?;
    let end = source.size();
    logical_group(&mut source, 3)?;
    assert!(f.checkpoint(&mut source, end, Fault::EnospcBefore).is_err());
    let exact = f.head.gate.holds.values().next().unwrap().clone();
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    let before = reopened.head.clone();
    assert!(reopened.compact_data(0, Cut::None, false).is_err());
    assert!(reopened.append(&mut source, end).is_err());
    assert!(reopened.checkpoint(&mut source, end, Fault::None).is_err());
    assert_eq!(before, reopened.head);
    reopened.resume_checkpoint(&mut source, end, &exact)?;
    assert!(reopened.head.gate.holds.is_empty());
    reopened.audit()?;
    Ok(())
}

#[test]
fn independent_stale_or_unknown_reader_release_cannot_undercount() -> Result<()> {
    let (_temp, _source, mut f) = setup(32, Kind::Btree)?;
    f.data.rotate(&mut f.c)?;
    let pin = f.acquire(PinClass::ReaderLease, true)?;
    let mut stale = pin.clone();
    stale.epoch += 1;
    for (exact, finished, reconciled) in [
        (&stale, true, true),
        (&pin, false, true),
        (&pin, true, false),
    ] {
        let before = f.head.clone();
        assert!(f.release(exact, true, finished, reconciled).is_err());
        assert_eq!(before, f.head);
    }
    assert!(f.compact_combined(true, 0, Fault::None).is_err());
    assert!(f.head.gate.holds.is_empty());
    f.release(&pin, true, true, true)?;
    let newer = f.acquire(PinClass::ReaderLease, false)?;
    assert_ne!(pin.token, newer.token);
    let before = f.head.clone();
    assert!(f.release(&pin, true, true, true).is_err());
    assert_eq!(before, f.head);
    assert!(f.compact_combined(true, 0, Fault::None).is_err());
    f.release(&newer, true, true, true)?;
    f.compact_combined(true, 0, Fault::None)?;
    f.audit()?;
    Ok(())
}

#[test]
fn independent_shared_domain_sum_and_overflow_refuse_atomically() -> Result<()> {
    let (_temp, _source, mut f) = setup(32, Kind::Radix)?;
    let mut gate = f.head.gate.clone();
    let charged = gate.domains[&0].charged;
    gate.domains.get_mut(&0).unwrap().limit = charged + CONTROL * 3 / 2;
    f.select_gate(gate)?;
    let before = f.head.clone();
    let snapshot = f.snapshot()?;
    for requests in [
        vec![(0, CONTROL), (0, CONTROL)],
        vec![(0, u64::MAX), (0, 1)],
        vec![(99, 1)],
    ] {
        assert!(f.reserve(f.head.semantic.clone(), None, &requests).is_err());
        assert_eq!(before, f.head);
        assert_eq!(snapshot, f.snapshot()?);
    }
    Ok(())
}

#[test]
fn independent_first_selection_and_unknown_selector_preserve_evidence() -> Result<()> {
    for fault in [Fault::EnospcBeforeHead, Fault::EnospcAfterHead] {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        f.data.rotate(&mut f.c)?;
        let original_recovery = f.head.recovery.clone();
        assert!(f.compact_combined(true, 0, fault).is_err());
        let hold = f.head.gate.holds.values().next().unwrap().clone();
        assert!(f.data.path(0).exists());
        assert_eq!(f.head.recovery, original_recovery);
        let selected_bytes = fs::read(f.dir.join("HEAD")).map_err(error)?;
        let mut unknown = f.head.clone();
        unknown.epoch += 900;
        fs::write(
            f.dir.join("HEAD"),
            serde_json::to_vec(&unknown).map_err(error)?,
        )
        .map_err(error)?;
        assert!(Fixture::reopen_combined(&f.dir).is_err());
        assert!(f.reconcile().is_err());
        assert!(f.dir.join("intent").exists());
        assert!(f.dir.join("candidate").exists());
        assert!(f.data.path(0).exists());
        fs::write(f.dir.join("HEAD"), selected_bytes).map_err(error)?;
        let mut reopened = Fixture::reopen_combined(&f.dir)?;
        assert!(reopened.complete_liability(&hold, &[]).is_err());
        reopened.reconcile()?;
        assert!(reopened.complete_liability(&hold, &[]).is_err());
        assert!(reopened.data.path(0).exists());
        let h = reopened.head.clone();
        reopened.audit_root(&h.recovery, &h.semantic.recovery)?;
        reopened.resume_compaction(&hold)?;
        assert!(!reopened.data.path(0).exists());
        reopened.audit_combined()?;
    }
    Ok(())
}

#[test]
fn independent_group_original_receipts_survive_every_checkpoint_cut() -> Result<()> {
    for fault in [
        Fault::EnospcBefore,
        Fault::EnospcBeforeHead,
        Fault::EnospcAfterHead,
    ] {
        let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
        let end = source.size();
        logical_group(&mut source, 8)?;
        assert!(f.checkpoint(&mut source, end, fault).is_err());
        let hold = f.head.gate.holds.values().next().unwrap().clone();
        let mut reopened = Fixture::reopen_combined(&f.dir)?;
        reopened.resume_checkpoint(&mut source, end, &hold)?;
        let before = reopened.head.clone();
        let snapshot = reopened.snapshot()?;
        for ordinal in 33..=40 {
            let operation = id(ordinal);
            let digest = request(&operation);
            let original = reopened.retry(&operation, &digest)?;
            assert!(
                matches!(source.get(&original)?, Node::Receipt { id: got, request: req, ordinal: ord } if got == operation && req == digest && ord == ordinal)
            );
            assert_eq!(original, reopened.retry(&operation, &digest)?);
            assert!(reopened.retry(&operation, "different-request").is_err());
        }
        assert_eq!(before, reopened.head);
        assert_eq!(snapshot, reopened.snapshot()?);
        reopened.audit_combined()?;
    }
    Ok(())
}

#[test]
fn independent_all_pin_classes_outlive_native_recovery_turnover() -> Result<()> {
    let (_temp, mut source, mut f) = setup(64, Kind::Btree)?;
    f.data.rotate(&mut f.c)?;
    let mut pins = Vec::new();
    for class in EXTRA_CLASSES {
        pins.push(f.acquire(class, class == PinClass::UnresolvedIntent)?);
    }
    for _ in 0..2 {
        let end = source.size();
        logical_group(&mut source, 8)?;
        f.checkpoint(&mut source, end, Fault::None)?;
    }
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    assert_ne!(reopened.head.semantic.recovery, pins[0].semantic.active);
    reopened.audit_combined()?;
    for pin in &pins {
        reopened.audit_root(&pin.active, &pin.semantic.active)?;
        reopened.audit_root(&pin.recovery, &pin.semantic.recovery)?;
        let before = reopened.head.clone();
        assert!(reopened.compact_combined(true, 0, Fault::None).is_err());
        assert_eq!(before, reopened.head);
        reopened.release(pin, true, true, true)?;
    }
    reopened.compact_combined(true, 0, Fault::None)?;
    reopened.audit_combined()?;
    Ok(())
}

#[test]
fn independent_rollover_large_selector_hold_covers_fault_resume() -> Result<()> {
    for fault in [
        Fault::EnospcBefore,
        Fault::EnospcBeforeHead,
        Fault::EnospcAfterHead,
    ] {
        let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
        // Fill the current data pack exactly; the next changed object must roll.
        let padding = PACK - f.data.end;
        assert!(padding > 12);
        f.data.append(
            &vec![0; (padding - 12) as usize],
            Some(u64::MAX - 1),
            &mut f.c,
        )?;
        // Valid unselected metadata pages drive the locator arena near rollover.
        let page = f.meta.read(&f.head.active)?;
        while f.meta.end + page.len() as u64 + 4 <= PACK {
            f.meta.append(&page, None, &mut f.c)?;
        }
        let mut gate = f.head.gate.clone();
        gate.domains.get_mut(&0).unwrap().charged = f.snapshot()?.1;
        f.select_gate(gate)?;
        for _ in 0..MAX_PINS {
            f.acquire(PinClass::ReaderLease, false)?;
        }
        let before = f.head.clone();
        assert!(f.acquire(PinClass::ReaderLease, false).is_err());
        assert_eq!(before, f.head);
        let data_pack = f.data.pack;
        let meta_pack = f.meta.pack;
        let end = source.size();
        logical_group(&mut source, 32)?;
        let allocated = f.snapshot()?.1;
        assert!(f.checkpoint(&mut source, end, fault).is_err());
        let exact = f.head.gate.holds.values().next().unwrap().clone();
        let held = exact.by_domain[&0];
        let mut reopened = Fixture::reopen_combined(&f.dir)?;
        reopened.resume_checkpoint(&mut source, end, &exact)?;
        assert!(reopened.data.pack > data_pack);
        assert!(reopened.meta.pack > meta_pack);
        // Reserve selection is a separate CONTROL charge; the persisted hold
        // covers work, uncertain replay/orphans, and its own completion selection.
        assert!(reopened.snapshot()?.1.saturating_sub(allocated) <= held + CONTROL);
        assert_eq!(reopened.head.gate.pins.len(), MAX_PINS);
        reopened.audit_combined()?;
    }
    Ok(())
}

#[test]
fn independent_concurrent_holds_cannot_start_unbudgeted_work() -> Result<()> {
    let (_temp, mut source, mut f) = setup(32, Kind::Radix)?;
    f.data.rotate(&mut f.c)?;
    let target = f.head.semantic.clone();
    f.reserve(target.clone(), None, &[(0, CONTROL)])?;
    f.reserve(target, Some((true, 0)), &[(0, CONTROL)])?;
    let before = f.head.clone();
    let allocated = f.snapshot()?;
    let end = source.size();
    logical_group(&mut source, 3)?;
    assert!(f.checkpoint(&mut source, end, Fault::None).is_err());
    assert!(f.compact_combined(true, 0, Fault::None).is_err());
    f.authorized_hold = f.head.gate.holds.keys().next().copied();
    assert!(f.append(&mut source, end).is_err());
    assert!(f.compact_data(0, Cut::None, false).is_err());
    assert_eq!(before, f.head);
    assert_eq!(allocated, f.snapshot()?);
    Ok(())
}

#[test]
fn independent_checkpoint_preflight_is_no_effect_and_binds_encoded_sizes() -> Result<()> {
    let (_temp, mut source, mut f) = setup(64, Kind::Radix)?;
    let end = source.size();
    logical_group(&mut source, 32)?;
    let head = f.head.clone();
    let snapshot = f.snapshot()?;
    let before = f.c.clone();
    let plan = f.checkpoint_plan(&mut source, end)?;
    let estimate = physical_bound(Some(&plan.data), &plan.meta)?;
    assert_eq!(head, f.head);
    assert_eq!(snapshot, f.snapshot()?);
    assert_eq!(before.files_created, f.c.files_created);
    assert_eq!(before.files_deleted, f.c.files_deleted);
    assert_eq!(before.data_write_bytes, f.c.data_write_bytes);
    assert_eq!(before.meta_write_bytes, f.c.meta_write_bytes);
    assert_eq!(before.envelope_write_bytes, f.c.envelope_write_bytes);
    let data_bytes = plan.data.bytes;
    let meta_bytes = plan.meta.bytes;
    let continuation = Continuation::append(&plan);
    let hold = f.reserve_with(
        plan.semantic.clone(),
        None,
        &[(0, estimate["allocation_bound"].as_u64().unwrap() + CONTROL)],
        Some(continuation),
    )?;
    f.authorized_hold = Some(hold.token);
    let writes = f.c.clone();
    f.apply_append(plan, Cut::None)?;
    assert_eq!(f.c.data_write_bytes - writes.data_write_bytes, data_bytes);
    assert_eq!(f.c.meta_write_bytes - writes.meta_write_bytes, meta_bytes);
    let consumed = f.c.allocation_charge - writes.allocation_charge
        + (f.c.files_created - writes.files_created) * 16384;
    f.complete_liability(&hold, &[(0, consumed)])?;
    f.audit_combined()?;
    Ok(())
}

#[test]
fn independent_tight_checkpoint_capacity_boundary_is_pre_effect() -> Result<()> {
    let (_temp, mut source, mut probe) = setup(32, Kind::Btree)?;
    let end = source.size();
    logical_group(&mut source, 32)?;
    let charged = probe.head.gate.domains[&0].charged;
    assert!(
        probe
            .checkpoint(&mut source, end, Fault::EnospcBefore)
            .is_err()
    );
    let hold = probe.head.gate.holds.values().next().unwrap();
    let required = hold.by_domain[&0] + probe.head.gate.domains[&0].charged - charged;
    for one_byte_short in [true, false] {
        let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
        let mut gate = f.head.gate.clone();
        let selection_charge = f.control_bound(&gate)?;
        gate.domains.get_mut(&0).unwrap().limit =
            gate.domains[&0].charged + selection_charge + required - u64::from(one_byte_short);
        f.select_gate(gate)?;
        let before = f.head.clone();
        let allocated = f.snapshot()?;
        let end = source.size();
        logical_group(&mut source, 32)?;
        assert!(f.checkpoint(&mut source, end, Fault::EnospcBefore).is_err());
        if one_byte_short {
            assert_eq!(before, f.head);
            assert_eq!(allocated, f.snapshot()?);
            assert!(f.head.gate.holds.is_empty());
        } else {
            let exact = f.head.gate.holds.values().next().unwrap().clone();
            f.resume_checkpoint(&mut source, end, &exact)?;
            f.audit_combined()?;
        }
    }
    Ok(())
}

#[test]
fn independent_repeated_continuation_cuts_do_not_duplicate_pack_output() -> Result<()> {
    for compaction in [false, true] {
        let (_temp, mut source, mut f) = setup(64, Kind::Radix)?;
        let end = source.size();
        if compaction {
            f.data.rotate(&mut f.c)?;
            assert!(
                f.compact_combined(true, 0, Fault::EnospcBeforeHead)
                    .is_err()
            );
        } else {
            logical_group(&mut source, 32)?;
            assert!(
                f.checkpoint(&mut source, end, Fault::EnospcBeforeHead)
                    .is_err()
            );
        }
        let hold = f.head.gate.holds.values().next().unwrap().clone();
        let mut f = Fixture::reopen_combined(&f.dir)?;
        let data_bytes = f.c.data_write_bytes;
        let meta_bytes = f.c.meta_write_bytes;
        let extents = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
        for _ in 0..5 {
            assert!(f.continue_original(&hold, Cut::BeforeHead).is_err());
            assert_eq!(data_bytes, f.c.data_write_bytes);
            assert_eq!(meta_bytes, f.c.meta_write_bytes);
            assert_eq!(extents, (f.data.pack, f.data.end, f.meta.pack, f.meta.end));
            assert_eq!(f.head.gate.holds.get(&hold.token), Some(&hold));
            if compaction {
                assert!(f.data.path(0).exists());
            }
            f = Fixture::reopen_combined(&f.dir)?;
        }
        if compaction {
            f.resume_compaction(&hold)?;
        } else {
            f.resume_checkpoint(&mut source, end, &hold)?;
        }
        f.audit_combined()?;
    }
    Ok(())
}

#[test]
fn independent_planned_or_corrupt_continuation_is_not_barrier_evidence() -> Result<()> {
    let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
    let end = source.size();
    logical_group(&mut source, 8)?;
    assert!(f.checkpoint(&mut source, end, Fault::EnospcBefore).is_err());
    let exact = f.head.gate.holds.values().next().unwrap().clone();
    let before = f.head.clone();
    let snapshot = f.snapshot()?;
    assert!(f.continue_original(&exact, Cut::None).is_err());
    assert_eq!(before, f.head);
    assert_eq!(snapshot, f.snapshot()?);
    // Execute once but stop before selector; corrupt only new unselected page.
    assert!(f.append_cut(&mut source, end, Cut::BeforeHead).is_err());
    let continuation = exact.continuation.as_ref().unwrap();
    let path = f.meta.path(continuation.active.pack);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(error)?;
    let mut original = vec![0; continuation.active.len as usize];
    file.seek(SeekFrom::Start(continuation.active.offset))
        .and_then(|_| file.read_exact(&mut original))
        .map_err(error)?;
    file.seek(SeekFrom::Start(continuation.active.offset))
        .and_then(|_| file.write_all(&[0]))
        .map_err(error)?;
    let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    assert!(reopened.continue_original(&exact, Cut::None).is_err());
    assert_eq!(head, fs::read(f.dir.join("HEAD")).map_err(error)?);
    assert!(f.dir.join("intent").exists());
    assert!(f.dir.join("candidate").exists());
    assert_eq!(reopened.head.gate.holds.get(&exact.token), Some(&exact));
    file.seek(SeekFrom::Start(continuation.active.offset))
        .and_then(|_| file.write_all(&original))
        .map_err(error)?;
    reopened.resume_checkpoint(&mut source, end, &exact)?;
    reopened.audit_combined()?;
    Ok(())
}

#[test]
fn independent_reconcile_then_restart_keeps_original_continuation_extents() -> Result<()> {
    let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
    let padding = PACK - f.data.end;
    f.data.append(
        &vec![0; (padding - 12) as usize],
        Some(u64::MAX - 1),
        &mut f.c,
    )?;
    let mut gate = f.head.gate.clone();
    gate.domains.get_mut(&0).unwrap().charged = f.snapshot()?.1;
    f.select_gate(gate)?;
    let end = source.size();
    logical_group(&mut source, 32)?;
    assert!(
        f.checkpoint(&mut source, end, Fault::EnospcBeforeHead)
            .is_err()
    );
    let exact = f.head.gate.holds.values().next().unwrap().clone();
    let continuation = exact.continuation.clone().unwrap();
    assert!(continuation.data_pack > exact.origin.data_pack);
    assert_eq!(f.reconcile()?, "exact-old");
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    reopened.resume_checkpoint(&mut source, end, &exact)?;
    assert_eq!(reopened.head.data_pack, continuation.data_pack);
    assert_eq!(reopened.head.data_end, continuation.data_end);
    let mut final_open = Fixture::reopen_combined(&f.dir)?;
    final_open.audit_combined()?;
    Ok(())
}
