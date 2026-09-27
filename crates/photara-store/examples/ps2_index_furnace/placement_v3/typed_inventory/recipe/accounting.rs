//! Original-token observations for the actual-v3 recipe, not an OS reservation.
//! Pack deltas settle admission; control generations remain conservatively held.
#[allow(clippy::wildcard_imports, reason = "Bounded actual-v3 fixture child")]
use super::*;

pub(super) fn charge(m: &fs::Metadata) -> Result<u64> {
    Ok(m.blocks()
        .checked_mul(512)
        .ok_or("allocation overflow")?
        .max(m.len()))
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct Allocation {
    data: bool,
    pack: u64,
    domain: u64,
    dev: u64,
    ino: u64,
    extent: u64,
    original_charge: u64,
    current_observed_charge: u64,
    incremental_charge: u64,
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct Control {
    role: String,
    token: u64,
    domain: u64,
    dev: u64,
    ino: u64,
    extent: u64,
    observed_charge: u64,
    content_sha: String,
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct Settlement {
    original_token: u64,
    allocations: Vec<Allocation>,
    controls: Vec<Control>,
    pub(super) incremental_pack_charge: u64,
    current_control_charge: u64,
    conservative_completion_reserve: u64,
    project_retirement_credit: u64,
    filesystem_availability_credit: u64,
}
// Derive both permitted selectors from the pre-effect selected state and the
// exact original hold. This includes domains, pins, token sequencing and charges,
// not merely the reachable roots or a matching token embedded in arbitrary JSON.
pub(in super::super::super) fn selectors_from_base(
    exact: &Liability,
    base: &Selection,
) -> Result<(Selection, Selection)> {
    ensure(
        base.gate.holds.is_empty()
            && exact.origin == Continuation::from_selection(base)
            && exact.epoch == base.epoch
            && exact.token == checked(base.gate.last_token, 1)?,
        "accounting original selector/hold identity",
    )?;
    let c = exact
        .continuation
        .as_ref()
        .ok_or("accounting continuation")?;
    let mut old = base.clone();
    old.gate.last_token = exact.token;
    old.gate.holds.insert(exact.token, exact.clone());
    // Same finite bound used by Fixture::select_gate for the admission selector.
    let mut bounded = old.clone();
    bounded.epoch = u64::MAX;
    for d in bounded.gate.domains.values_mut() {
        d.charged = u64::MAX;
        d.limit = u64::MAX;
    }
    let len = serde_json::to_vec(&bounded).map_err(error)?.len() as u64;
    ensure(len <= GATE_MAX as u64, "accounting selector envelope cap")?;
    let control = 4 * round(len + 4096) + 8 * 4096 + 65536;
    if old.gate.ledger.is_none() {
        let d = old
            .gate
            .domains
            .get_mut(&0)
            .ok_or("accounting project domain")?;
        d.charged = checked(d.charged, control)?;
    }
    old.epoch = checked(old.epoch, 1)?;
    let mut candidate = old.clone();
    candidate.epoch = checked(candidate.epoch, 1)?;
    candidate.semantic = exact.target.clone();
    candidate.active = c.active.clone();
    candidate.recovery = c.recovery.clone();
    candidate.inventory = c.inventory.clone();
    candidate.data_pack = c.data_pack;
    candidate.data_end = c.data_end;
    candidate.meta_pack = c.meta_pack;
    candidate.meta_end = c.meta_end;
    Ok((old, candidate))
}
pub(super) fn controls(
    f: &mut Fixture,
    exact: &Liability,
    recipe: &Recipe,
) -> Result<Vec<Control>> {
    controls_from_base(f, exact, &recipe.control_base)
}
pub(super) fn controls_from_base(
    f: &mut Fixture,
    exact: &Liability,
    base: &Selection,
) -> Result<Vec<Control>> {
    let (old, candidate) = selectors_from_base(exact, base)?;
    let mut observations = vec![];
    for role in ["HEAD", "intent", "candidate", "HEAD.next"] {
        let path = f.dir.join(role);
        let m = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && role != "HEAD" => continue,
            Err(e) => return Err(error(e)),
        };
        ensure(
            m.is_file() && m.nlink() == 1 && m.len() <= GATE_MAX as u64,
            "accounting private bounded control generation",
        )?;
        let mut file = File::open(&path).map_err(error)?;
        let opened = file.metadata().map_err(error)?;
        ensure(
            opened.dev() == m.dev() && opened.ino() == m.ino(),
            "accounting control generation changed during open",
        )?;
        let mut bytes = vec![0; usize::try_from(m.len()).map_err(error)?];
        file.read_exact(&mut bytes).map_err(error)?;
        f.c.head_read_bytes += bytes.len() as u64;
        let observed: Selection = serde_json::from_slice(&bytes).map_err(error)?;
        let matches = match role {
            "HEAD" => observed == old || observed == candidate,
            "intent" => observed == old,
            _ => observed == candidate,
        };
        ensure(
            matches,
            "unknown control generation retains original accounting hold",
        )?;
        let after = file.metadata().map_err(error)?;
        ensure(
            after.len() == m.len() && charge(&after)? == charge(&m)?,
            "control observation changed",
        )?;
        observations.push(Control {
            role: role.into(),
            token: exact.token,
            domain: 0,
            dev: m.dev(),
            ino: m.ino(),
            extent: m.len(),
            observed_charge: charge(&after)?,
            content_sha: hash(&bytes),
        });
    }
    let has = |name| observations.iter().any(|o| o.role == name);
    ensure(
        has("intent") == has("candidate"),
        "incomplete control dispatch retains original hold",
    )?;
    Ok(observations)
}
pub(super) fn observe(f: &mut Fixture, exact: &Liability, recipe: &Recipe) -> Result<Settlement> {
    ensure(
        f.head.gate.holds.get(&exact.token) == Some(exact),
        "accounting requires exact live original token",
    )?;
    let controls = controls(f, exact, recipe)?;
    let mut allocations = vec![];
    let mut incremental_pack_charge = 0;
    for (data, base, plan) in [
        (true, &recipe.data_base, &recipe.data),
        (false, &recipe.meta_base, &recipe.meta),
    ] {
        let arena = if data { &f.data } else { &f.meta };
        for (pack, start, expected) in segments(base, plan, data)? {
            let path = arena.path(pack);
            let m = fs::symlink_metadata(&path).map_err(error)?;
            ensure(
                m.is_file() && m.nlink() == 1 && m.len() == start + expected.len() as u64,
                "accounting exact private allocation extent",
            )?;
            let mut file = File::open(&path).map_err(error)?;
            let opened = file.metadata().map_err(error)?;
            ensure(
                m.dev() == opened.dev() && m.ino() == opened.ino(),
                "accounting allocation replaced before open",
            )?;
            let mut bytes = vec![0; usize::try_from(m.len()).map_err(error)?];
            file.read_exact(&mut bytes).map_err(error)?;
            f.c.recipe_prefix_read_bytes += start;
            f.c.recipe_suffix_read_bytes += expected.len() as u64;
            let original_charge = if pack == base.pack {
                ensure(
                    m.dev() == base.dev
                        && m.ino() == base.ino
                        && start == base.end
                        && hash(&bytes[..usize::try_from(start).map_err(error)?]) == base.sha,
                    "accounting original allocation generation/prefix",
                )?;
                base.observed_charge
            } else {
                0
            };
            ensure(
                bytes[usize::try_from(start).map_err(error)?..] == expected,
                "accounting suffix differs from original recipe",
            )?;
            let after = file.metadata().map_err(error)?;
            ensure(
                after.len() == m.len() && charge(&after)? == charge(&m)?,
                "allocation observation changed",
            )?;
            let current_observed_charge = charge(&after)?;
            // Shrink/compression cannot yield credit without proven retirement.
            let incremental_charge = current_observed_charge.saturating_sub(original_charge);
            incremental_pack_charge = checked(incremental_pack_charge, incremental_charge)?;
            allocations.push(Allocation {
                data,
                pack,
                domain: 0,
                dev: m.dev(),
                ino: m.ino(),
                extent: m.len(),
                original_charge,
                current_observed_charge,
                incremental_charge,
            });
        }
    }
    let current_control_charge = controls
        .iter()
        .try_fold(0, |n, c| checked(n, c.observed_charge))?;
    let held = exact
        .by_domain
        .get(&0)
        .copied()
        .ok_or("accounting original project hold")?;
    ensure(
        checked(
            checked(incremental_pack_charge, current_control_charge)?,
            exact.control,
        )? <= held,
        "observed allocation exceeds original admission",
    )?;
    Ok(Settlement {
        original_token: exact.token,
        allocations,
        controls,
        incremental_pack_charge,
        current_control_charge,
        conservative_completion_reserve: exact.control,
        project_retirement_credit: 0,
        filesystem_availability_credit: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accounting_original_token_cuts_settle_growth_once_and_keep_control_reserve() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                Fault::Partial(7),
                Fault::AfterData,
                Fault::BeforeHead,
                Fault::AfterHead,
                Fault::BeforeSettlement,
            ] {
                let (_temp, _source, mut f) = setup(64, kind)?;
                let (hold, _) = prepare_self_covered(&mut f)?;
                let charged = f.head.gate.domains[&0].charged;
                ensure(
                    resume(&mut f, &hold, cut).is_err(),
                    "accounting cut required",
                )?;
                f = Fixture::reopen_combined(&f.dir)?;
                let evidence = resume_accounted(&mut f, &hold, Fault::None)?;
                let growth = evidence["incremental_pack_charge"].as_u64().unwrap();
                ensure(
                    f.head.gate.domains[&0].charged == charged + growth + hold.control,
                    "original delta charged more than once",
                )?;
                ensure(
                    evidence["project_retirement_credit"] == 0
                        && evidence["filesystem_availability_credit"] == 0,
                    "speculative credit",
                )?;
                let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
                let before = f.snapshot()?;
                ensure(
                    resume(&mut f, &hold, Fault::None).is_err(),
                    "settled token replayed",
                )?;
                ensure(
                    f.snapshot()? == before && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
                    "duplicate settlement effects",
                )?;
                f.audit_combined()?;
            }
        }
        Ok(())
    }
    #[test]
    fn accounting_unknown_candidate_gate_retains_evidence_without_effects() -> Result<()> {
        for attack in 0..3 {
            let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
            let pin = f.acquire(PinClass::ReaderLease, false)?;
            let mut gate = f.head.gate.clone();
            gate.domains.insert(
                9,
                CapacityDomain {
                    limit: u64::MAX / 2,
                    charged: 0,
                },
            );
            f.select_gate(gate)?;
            let (hold, _) = prepare_self_covered(&mut f)?;
            ensure(
                resume(&mut f, &hold, Fault::BeforeHead).is_err(),
                "selector cut",
            )?;
            let path = f.dir.join("candidate");
            let mut candidate: Selection = Fixture::bounded_read(&path)?;
            match attack {
                0 => candidate.gate.domains.get_mut(&9).unwrap().limit += 1,
                1 => candidate.gate.pins.get_mut(&pin.token).unwrap().unknown = true,
                _ => candidate.gate.domains.get_mut(&0).unwrap().charged += 1,
            }
            fs::write(&path, serde_json::to_vec(&candidate).map_err(error)?).map_err(error)?;
            let before = f.snapshot()?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let retained = fs::read(&path).map_err(error)?;
            ensure(
                resume(&mut f, &hold, Fault::None)
                    .unwrap_err()
                    .contains("unknown control generation"),
                "unknown control accepted",
            )?;
            ensure(
                f.snapshot()? == before
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && fs::read(path).map_err(error)? == retained
                    && f.head.gate.holds.get(&hold.token) == Some(&hold),
                "unknown accounting evidence lost",
            )?;
        }
        Ok(())
    }
    #[test]
    fn accounting_replaced_pack_generation_cannot_claim_original_delta() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let (hold, _) = prepare_self_covered(&mut f)?;
        ensure(
            resume(&mut f, &hold, Fault::BeforeSettlement).is_err(),
            "settlement cut",
        )?;
        let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
        let path = f.data.path(recipe.data_base.pack);
        let original = f.dir.join("replaced-original");
        fs::rename(&path, &original).map_err(error)?;
        fs::copy(&original, &path).map_err(error)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            observe(&mut f, &hold, recipe)
                .unwrap_err()
                .contains("original allocation generation"),
            "wrong inode accounted",
        )?;
        ensure(
            f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && f.head.gate.holds.get(&hold.token) == Some(&hold),
            "wrong-inode accounting effects",
        )
    }
    #[test]
    fn accounting_late_suffix_corruption_keeps_original_hold_and_charge() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let (hold, _) = prepare_self_covered(&mut f)?;
        ensure(
            resume(&mut f, &hold, Fault::BeforeSettlement).is_err(),
            "settlement cut",
        )?;
        let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
        let r = &recipe.meta.writes.last().unwrap().1;
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.meta.path(r.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(r.offset)).map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            observe(&mut f, &hold, recipe)
                .unwrap_err()
                .contains("suffix differs"),
            "late corrupt suffix accounted",
        )?;
        ensure(
            f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && f.head.gate.holds.get(&hold.token) == Some(&hold),
            "late accounting evidence lost",
        )
    }
}
