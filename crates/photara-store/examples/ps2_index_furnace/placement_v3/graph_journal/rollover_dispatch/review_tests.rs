//! Independent corruption and generic-API regression checks for original admission.
#[allow(clippy::wildcard_imports, reason = "Read-only review regression child")]
use super::*;

fn files(root: &Path) -> Result<BTreeMap<String, (u64, u64, Vec<u8>)>> {
    fn visit(
        root: &Path,
        at: &Path,
        out: &mut BTreeMap<String, (u64, u64, Vec<u8>)>,
    ) -> Result<()> {
        for entry in fs::read_dir(at).map_err(error)? {
            let entry = entry.map_err(error)?;
            let path = entry.path();
            let m = fs::symlink_metadata(&path).map_err(error)?;
            if m.is_dir() {
                visit(root, &path, out)?;
            } else {
                let key = path
                    .strip_prefix(root)
                    .map_err(error)?
                    .to_string_lossy()
                    .into_owned();
                let bytes = if m.file_type().is_symlink() {
                    fs::read_link(&path)
                        .map_err(error)?
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec()
                } else {
                    fs::read(&path).map_err(error)?
                };
                out.insert(key, (m.dev(), m.ino(), bytes));
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out)?;
    Ok(out)
}

#[test]
fn review_rollover_original_hold_fences_unrelated_gate_apis() -> Result<()> {
    for payload in [false, true] {
        let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
        let pin = f.acquire(PinClass::ReaderLease, false)?;
        let group = plan(&mut source, 8)?;
        admit_dedicated(&mut source, &mut f, &journal, &group)?;
        if payload {
            ensure(
                finish(&mut f, &mut source, &journal, rollover::Cut::PayloadBefore).is_err(),
                "payload cut",
            )?;
        }
        let before = files(temp.path())?;
        let selected = f.head.clone();
        ensure(
            f.acquire(PinClass::Export, false).is_err(),
            "pending original admitted unrelated pin",
        )?;
        ensure(
            f.release(&pin, true, true, true).is_err(),
            "pending original released admitted pin",
        )?;
        ensure(
            f.reserve(f.head.semantic.clone(), None, &[(0, 1)]).is_err(),
            "pending original admitted second reserve",
        )?;
        ensure(
            f.head == selected && files(temp.path())? == before,
            "generic API refusal changed evidence",
        )?;
    }
    Ok(())
}

#[test]
fn review_rollover_changed_no_dispatch_selector_refuses_before_journal_repair() -> Result<()> {
    for mode in 0..3 {
        let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
        let pin = f.acquire(PinClass::ReaderLease, false)?;
        let group = plan(&mut source, 8)?;
        admit_dedicated(&mut source, &mut f, &journal, &group)?;
        ensure(
            finish(&mut f, &mut source, &journal, rollover::Cut::PayloadBefore).is_err(),
            "payload cut",
        )?;
        ensure(
            !f.dir.join("intent").exists(),
            "test requires selected phase without dispatch",
        )?;
        let mut altered = f.head.clone();
        match mode {
            0 => altered.epoch += 1,
            1 => {
                altered.gate.pins.remove(&pin.token);
            }
            _ => {
                altered.gate.domains.get_mut(&0).ok_or("domain")?.limit += 1;
            }
        }
        fs::write(
            f.dir.join("HEAD"),
            serde_json::to_vec(&altered).map_err(error)?,
        )
        .map_err(error)?;
        // An exact admitted journal prefix could normally be repaired. The
        // unrelated selected-state corruption must be rejected before that write.
        let group_bytes = fs::read(journal.join("group")).map_err(error)?;
        fs::write(journal.join("group"), &group_bytes[..group_bytes.len() / 2]).map_err(error)?;
        let before = files(temp.path())?;
        let physical = f.dir.clone();
        let logical = source.dir.clone();
        drop(f);
        drop(source);
        if let Ok(mut reopened) = Fixture::reopen_combined(&physical) {
            let mut source = Store::open(&logical)?;
            ensure(
                finish(&mut reopened, &mut source, &journal, rollover::Cut::None).is_err(),
                "changed selector accepted",
            )?;
        }
        ensure(
            files(temp.path())? == before,
            "changed selector allowed journal/source effects",
        )?;
    }
    Ok(())
}

#[test]
fn review_rollover_unknown_journal_controls_fence_all_effects() -> Result<()> {
    for role in ["candidate", "marker", "unexpected"] {
        let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
        let group = plan(&mut source, 8)?;
        admit_dedicated(&mut source, &mut f, &journal, &group)?;
        ensure(
            finish_cut(
                &mut f,
                &mut source,
                &journal,
                rollover::Cut::None,
                GraphCut::SourceCandidate,
            )
            .is_err(),
            "source candidate cut",
        )?;
        fs::write(journal.join(role), b"unknown recovery evidence").map_err(error)?;
        let before = files(temp.path())?;
        let physical = f.dir.clone();
        let logical = source.dir.clone();
        drop(f);
        drop(source);
        let mut f = Fixture::reopen_combined(&physical)?;
        let mut source = Store::open(&logical)?;
        ensure(
            finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err(),
            "unknown journal control accepted",
        )?;
        ensure(
            files(temp.path())? == before,
            "unknown journal evidence allowed publication/effects",
        )?;
    }
    Ok(())
}

#[test]
fn review_rollover_replaced_retained_source_refuses_before_any_growth() -> Result<()> {
    let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
    let group = plan(&mut source, 8)?;
    admit_dedicated(&mut source, &mut f, &journal, &group)?;
    let physical = f.dir.clone();
    let logical = source.dir.clone();
    drop(f);
    drop(source);
    let path = logical.join("objects.pack");
    let bytes = fs::read(&path).map_err(error)?;
    let replacement = logical.join("replacement");
    fs::write(&replacement, &bytes).map_err(error)?;
    fs::rename(&replacement, &path).map_err(error)?;
    let before = files(temp.path())?;
    let mut f = Fixture::reopen_combined(&physical)?;
    let mut source = Store::open(&logical)?;
    ensure(
        finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err(),
        "replaced retained source accepted",
    )?;
    ensure(
        files(temp.path())? == before,
        "replaced retained source received unowned growth",
    )?;
    Ok(())
}

#[test]
fn review_rollover_current_source_selection_drift_refuses_before_birth() -> Result<()> {
    let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
    let first = plan(&mut source, 8)?;
    admit_dedicated(&mut source, &mut f, &journal, &first)?;
    finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
    let group = plan(&mut source, 8)?;
    // Both references are real roots and the retained source extent is unchanged.
    // Re-select an older valid source HEAD while the physical HEAD stays current.
    let historical = Head {
        active: group.old.recovery.clone(),
        recovery: group.old.recovery.clone(),
    };
    ensure(
        historical != group.old,
        "distinct valid source selection required",
    )?;
    source.publish(&historical)?;
    let before = files(temp.path())?;
    let selected = f.head.clone();
    ensure(
        persist(&mut source, &mut f, &journal, &group).is_err(),
        "source selection drift admitted",
    )?;
    ensure(
        f.head == selected && files(temp.path())? == before,
        "source drift created original hold or generations",
    )?;
    Ok(())
}

#[test]
fn review_rollover_pending_binding_checks_inode_before_dispatch_cleanup() -> Result<()> {
    for cut in [
        rollover::Cut::BindingBeforeHead,
        rollover::Cut::BindingStagedHead,
    ] {
        let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
        let group = plan(&mut source, 8)?;
        let hold = admit_dedicated(&mut source, &mut f, &journal, &group)?;
        ensure(
            finish(&mut f, &mut source, &journal, cut).is_err(),
            "binding cut",
        )?;
        let (data, pack, _, _) =
            fresh_generation::promoted_generations(hold.birth.as_ref().ok_or("birth")?)?[0];
        let path = if data {
            f.data.path(pack)
        } else {
            f.meta.path(pack)
        };
        let bytes = fs::read(&path).map_err(error)?;
        fs::rename(&path, temp.path().join("original-held-generation")).map_err(error)?;
        fs::write(&path, &bytes).map_err(error)?;
        let before = files(temp.path())?;
        let physical = f.dir.clone();
        let logical = source.dir.clone();
        drop(f);
        drop(source);
        let mut f = Fixture::reopen_combined(&physical)?;
        let mut source = Store::open(&logical)?;
        ensure(
            finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err(),
            "substituted binding generation accepted",
        )?;
        ensure(
            files(temp.path())? == before,
            "substituted generation destroyed pending dispatch evidence",
        )?;
    }
    Ok(())
}

#[test]
fn review_rollover_pending_binding_checks_source_suffix_before_dispatch_cleanup() -> Result<()> {
    for cut in [
        rollover::Cut::BindingBeforeHead,
        rollover::Cut::BindingStagedHead,
    ] {
        let (temp, mut source, mut f, journal) = setup_fixture(Kind::Btree, &[true])?;
        let group = plan(&mut source, 8)?;
        admit_dedicated(&mut source, &mut f, &journal, &group)?;
        ensure(
            finish(&mut f, &mut source, &journal, cut).is_err(),
            "binding cut",
        )?;
        let path = source.dir.join("objects.pack");
        let mut bytes = fs::read(&path).map_err(error)?;
        ensure(bytes.len() as u64 > group.old_end, "present source suffix")?;
        *bytes.last_mut().ok_or("nonempty source")? ^= 1;
        fs::write(&path, &bytes).map_err(error)?;
        let before = files(temp.path())?;
        let physical = f.dir.clone();
        let logical = source.dir.clone();
        drop(f);
        drop(source);
        let mut f = Fixture::reopen_combined(&physical)?;
        let mut source = Store::open(&logical)?;
        ensure(
            finish(&mut f, &mut source, &journal, rollover::Cut::None).is_err(),
            "altered present source suffix accepted",
        )?;
        ensure(
            files(temp.path())? == before,
            "altered source suffix destroyed pending dispatch evidence",
        )?;
    }
    Ok(())
}
