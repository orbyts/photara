//! Bounded original-token write recipe for actual typed-v3 payload recovery.
//! HEAD-held bytes are a disposable control-amplification experiment, not wire.
#[allow(clippy::wildcard_imports, reason = "Actual typed fixture child")]
use super::*;
#[path = "recipe/accounting.rs"]
pub(in super::super) mod accounting;
const MAX_RECIPE: usize = 48 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct Prefix {
    pack: u64,
    end: u64,
    dev: u64,
    ino: u64,
    observed_charge: u64,
    sha: String,
}
impl Prefix {
    pub(in super::super) fn capture(arena: &Arena) -> Result<Self> {
        let m = fs::symlink_metadata(arena.path(arena.pack)).map_err(error)?;
        ensure(
            m.is_file() && m.nlink() == 1 && m.len() == arena.end && m.len() <= PACK,
            "original recipe prefix",
        )?;
        Ok(Self {
            pack: arena.pack,
            end: arena.end,
            dev: m.dev(),
            ino: m.ino(),
            observed_charge: accounting::charge(&m)?,
            sha: hash(&fs::read(arena.path(arena.pack)).map_err(error)?),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct Recipe {
    digest: String,
    claims: Vec<Claim>,
    self_covered: bool,
    control_base: Selection,
    data_base: Prefix,
    meta_base: Prefix,
    data: WritePlan,
    meta: WritePlan,
}
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum Fault {
    None,
    BeforeWrite,
    Partial(usize),
    AfterData,
    BeforeHead,
    AfterHead,
    BeforeSettlement,
}
type Segment = (u64, u64, Vec<u8>);
pub(in super::super) fn segments(
    base: &Prefix,
    plan: &WritePlan,
    data: bool,
) -> Result<Vec<Segment>> {
    ensure(
        plan.pack >= base.pack && plan.pack - base.pack <= 4,
        "recipe arena span",
    )?;
    let mut pack = base.pack;
    let mut end = base.end;
    let mut total = 0;
    let mut out = vec![];
    let mut bytes = vec![];
    let mut start = end;
    for (key, r, body) in &plan.writes {
        ensure(
            key.is_some() == data && hash(body) == r.sha && body.len() as u64 == r.len,
            "recipe record commitment",
        )?;
        let frame = if data { 12 } else { 4 };
        let len = r.len + frame;
        ensure(
            len <= PACK && (data || r.len <= 4092),
            "bounded recipe record",
        )?;
        if end + len > PACK {
            out.push((pack, start, bytes));
            bytes = vec![];
            pack = checked(pack, 1)?;
            end = 0;
            start = 0;
        }
        ensure(
            r.pack == pack && r.offset == end + frame,
            "recipe physical continuity",
        )?;
        bytes.extend(u32::try_from(body.len()).map_err(error)?.to_le_bytes());
        if let Some(k) = key {
            bytes.extend(k.to_le_bytes());
        }
        bytes.extend(body);
        end += len;
        total += len;
    }
    out.push((pack, start, bytes));
    ensure(
        pack == plan.pack && end == plan.end && total == plan.bytes,
        "recipe final extent/bytes",
    )?;
    Ok(out)
}
impl Recipe {
    fn digest(&self) -> Result<String> {
        let mut payload = self.clone();
        payload.digest.clear();
        Ok(hash(&serde_json::to_vec(&payload).map_err(error)?))
    }
    fn validate(&self) -> Result<()> {
        ensure(
            serde_json::to_vec(self).map_err(error)?.len() <= MAX_RECIPE,
            "recipe byte admission cap",
        )?;
        ensure(
            self.digest == self.digest()?,
            "original recipe digest mismatch",
        )?;
        segments(&self.data_base, &self.data, true)?;
        segments(&self.meta_base, &self.meta, false)?;
        verify_self_coverage(self)?;
        Ok(())
    }
}
fn prepare(f: &mut Fixture, claim: &Claim) -> Result<(Liability, serde_json::Value)> {
    prepare_claims(f, std::slice::from_ref(claim))
}
fn prepare_claims(f: &mut Fixture, claims: &[Claim]) -> Result<(Liability, serde_json::Value)> {
    let p = plan_claims(f, claims)?;
    reserve_plan(f, claims, p, false)
}
fn reserve_plan(
    f: &mut Fixture,
    claims: &[Claim],
    p: TypedPlan,
    self_covered: bool,
) -> Result<(Liability, serde_json::Value)> {
    let mut recipe = Recipe {
        digest: String::new(),
        claims: claims.to_vec(),
        self_covered,
        control_base: f.head.clone(),
        data_base: Prefix::capture(&f.data)?,
        meta_base: Prefix::capture(&f.meta)?,
        data: p.data,
        meta: p.meta,
    };
    recipe.digest = recipe.digest()?;
    f.c.recipe_prefix_read_bytes += recipe.data_base.end + recipe.meta_base.end;
    recipe.validate()?;
    let bytes = serde_json::to_vec(&recipe).map_err(error)?.len() as u64;
    let physical = physical_bound(Some(&recipe.data), &recipe.meta)?;
    let mut continuation = p.continuation;
    continuation.recipe = Some(recipe);
    // Reserve all envelope copies conservatively. Fixed recipe cap keeps this
    // independent of lifetime N; this is not a filesystem allocation guarantee.
    let budget = checked(
        physical["allocation_bound"]
            .as_u64()
            .ok_or("recipe physical reserve")?,
        checked(
            CONTROL,
            bytes.checked_mul(16).ok_or("recipe envelope overflow")?,
        )?,
    )?;
    let idle_head_bytes = fs::metadata(f.dir.join("HEAD")).map_err(error)?.len();
    let hold = f.reserve_with(
        f.head.semantic.clone(),
        None,
        &[(0, budget)],
        Some(continuation),
    )?;
    Ok((
        hold,
        json!({"recipe_bytes":bytes,"recipe_cap":MAX_RECIPE,"physical":physical,"modeled_budget":budget,"idle_head_bytes":idle_head_bytes,"pending_head_bytes":fs::metadata(f.dir.join("HEAD")).map_err(error)?.len(),"head_frame_cap":GATE_MAX}),
    ))
}
// Bootstrap-only bounded coverage of existing data and locator packs. Later
// growth uses explicit touched units; neither path scans lifetime files during open.
fn capture_prefixes(f: &mut Fixture, units: &[(bool, u64)]) -> Result<Vec<Claim>> {
    ensure(f.head.gate.holds.is_empty(), "pending original operation")?;
    ensure(
        !units.is_empty() && units.len() <= 64,
        "bounded capture units",
    )?;
    let mut keys = HashSet::new();
    units
        .iter()
        .map(|&(data, pack)| {
            let arena = if data { &f.data } else { &f.meta };
            let (tip, end) = (arena.pack, arena.end);
            ensure(pack <= tip, "unselected capture pack")?;
            let claim = Claim::capture(f, data, pack)?;
            ensure(keys.insert(claim.key()?), "duplicate capture unit")?;
            ensure(
                pack != tip || claim.extent == end,
                "unselected capture suffix",
            )?;
            Ok(claim)
        })
        .collect()
}
fn bootstrap_prefixes(f: &mut Fixture) -> Result<Vec<Claim>> {
    let count = checked(checked(f.data.pack, f.meta.pack)?, 2)?;
    ensure(count <= 64, "bootstrap ownership budget")?;
    let units = (0..=f.data.pack)
        .map(|pack| (true, pack))
        .chain((0..=f.meta.pack).map(|pack| (false, pack)))
        .collect::<Vec<_>>();
    capture_prefixes(f, &units)
}
// This manifest owns allocation extents, not every byte's content. Existing
// prefix hashes remain intact; exact typed locator closure authenticates selected
// suffix objects/pages. Transient unreachable frames are owned physical garbage.
// Refuse rollover until original-token fresh-generation binding is integrated.
fn self_covered_plan(f: &mut Fixture) -> Result<(Vec<Claim>, TypedPlan)> {
    let mut claims = bootstrap_prefixes(f)?;
    for _ in 0..16 {
        let p = plan_claim_extents(f, &claims, true)?;
        ensure(
            p.data.pack == f.data.pack && p.meta.pack == f.meta.pack,
            "self coverage requires prebound pack generations; rollover refused",
        )?;
        let mut changed = false;
        for claim in &mut claims {
            let (pack, end) = if claim.data {
                (p.data.pack, p.data.end)
            } else {
                (p.meta.pack, p.meta.end)
            };
            if claim.pack == pack && claim.extent != end {
                claim.extent = end;
                changed = true;
            }
        }
        if !changed {
            return Ok((claims, p));
        }
    }
    Err("self coverage extent fixed point exceeded bounded planning".into())
}
pub(in super::super) fn prepare_self_covered(
    f: &mut Fixture,
) -> Result<(Liability, serde_json::Value)> {
    let (claims, p) = self_covered_plan(f)?;
    reserve_plan(f, &claims, p, true)
}
fn verify_self_coverage(recipe: &Recipe) -> Result<()> {
    if !recipe.self_covered {
        return Ok(());
    }
    ensure(
        checked(checked(recipe.data_base.pack, recipe.meta_base.pack)?, 2)? <= 64
            && recipe.claims.len() <= 64,
        "self coverage bounded allocation manifest",
    )?;
    for (data, base, plan) in [
        (true, &recipe.data_base, &recipe.data),
        (false, &recipe.meta_base, &recipe.meta),
    ] {
        ensure(plan.pack == base.pack, "self coverage rollover")?;
        for pack in 0..=plan.pack {
            let claim = recipe
                .claims
                .iter()
                .find(|c| c.data == data && c.pack == pack)
                .ok_or("self coverage omitted allocation")?;
            if pack == plan.pack {
                ensure(
                    claim.dev == base.dev
                        && claim.ino == base.ino
                        && claim.content_end == base.end
                        && claim.sha == base.sha
                        && claim.extent == plan.end
                        && !claim.sealed,
                    "self coverage exact generation/extent",
                )?;
            }
        }
    }
    Ok(())
}
pub(in super::super) fn replay(
    f: &mut Fixture,
    data: bool,
    base: &Prefix,
    plan: &WritePlan,
    remaining: &mut Option<usize>,
) -> Result<()> {
    let arena = if data { &f.data } else { &f.meta };
    let path_prefix = arena.prefix;
    let dir = arena.dir.clone();
    for (pack, start, expected) in segments(base, plan, data)? {
        let path = dir.join(format!("{path_prefix}-{pack}"));
        let namespace = match fs::symlink_metadata(&path) {
            Ok(m) => {
                ensure(
                    m.file_type().is_file() && m.nlink() == 1,
                    "recipe destination is not a private regular file",
                )?;
                Some((m.dev(), m.ino()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(error(e)),
        };
        let mut file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && pack > base.pack => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(error)?;
                f.c.files_created += 1;
                file
            }
            Err(e) => return Err(error(e)),
        };
        let m = file.metadata().map_err(error)?;
        ensure(
            namespace.is_none_or(|(dev, ino)| m.dev() == dev && m.ino() == ino),
            "recipe destination changed before open",
        )?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && m.len() >= start
                && m.len() <= start + expected.len() as u64,
            "unknown/truncated recipe extent",
        )?;
        if pack == base.pack {
            ensure(
                m.dev() == base.dev && m.ino() == base.ino && start == base.end,
                "original recipe generation",
            )?;
            let mut prefix = vec![0; usize::try_from(base.end).map_err(error)?];
            file.read_exact(&mut prefix).map_err(error)?;
            f.c.recipe_prefix_read_bytes += base.end;
            ensure(
                hash(&prefix) == base.sha,
                "original recipe prefix corrupted",
            )?;
        }
        let present = usize::try_from(m.len() - start).map_err(error)?;
        file.seek(SeekFrom::Start(start)).map_err(error)?;
        let mut existing = vec![0; present];
        file.read_exact(&mut existing).map_err(error)?;
        f.c.recipe_suffix_read_bytes += present as u64;
        ensure(
            existing == expected[..present],
            "partial recipe suffix corruption",
        )?;
        let left = &expected[present..];
        let count = remaining.map_or(left.len(), |limit| limit.min(left.len()));
        file.seek(SeekFrom::Start(m.len())).map_err(error)?;
        file.write_all(&left[..count]).map_err(error)?;
        if data {
            f.c.data_write_bytes += count as u64;
        } else {
            f.c.meta_write_bytes += count as u64;
        }
        if let Some(limit) = remaining {
            *limit -= count;
            if *limit == 0 {
                return Err("injected partial record/ENOSPC; original recipe retained".into());
            }
        }
        sync_file(&file, &mut f.c)?;
    }
    sync_dir(&dir, &mut f.c)?;
    let arena = Fixture::reopen_arena(&dir, path_prefix, plan.pack, plan.end)?;
    if data {
        f.data = arena;
    } else {
        f.meta = arena;
    }
    Ok(())
}
fn verify_original_plan(f: &mut Fixture, exact: &Liability, recipe: &Recipe) -> Result<()> {
    let selected = f.head.clone();
    let tips = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    f.head.active = exact.origin.active.clone();
    f.head.recovery = exact.origin.recovery.clone();
    f.head.inventory = exact.origin.inventory.clone();
    f.head.semantic = exact.target.clone();
    f.head.data_pack = exact.origin.data_pack;
    f.head.data_end = exact.origin.data_end;
    f.head.meta_pack = exact.origin.meta_pack;
    f.head.meta_end = exact.origin.meta_end;
    f.head.gate.holds.clear();
    f.data.pack = exact.origin.data_pack;
    f.data.end = exact.origin.data_end;
    f.meta.pack = exact.origin.meta_pack;
    f.meta.end = exact.origin.meta_end;
    f.cache.clear();
    let generated = plan_claim_extents(f, &recipe.claims, recipe.self_covered);
    f.head = selected;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
    f.cache.clear();
    f.staged = None;
    let generated = generated?;
    let mut expected = exact
        .continuation
        .clone()
        .ok_or("original recipe continuation")?;
    expected.recipe = None;
    ensure(
        generated.data == recipe.data
            && generated.meta == recipe.meta
            && generated.continuation == expected,
        "recipe differs from original typed mutation plan",
    )
}
pub(in super::super) fn resume(f: &mut Fixture, exact: &Liability, fault: Fault) -> Result<()> {
    resume_accounted(f, exact, fault).map(|_| ())
}
pub(in super::super) fn resume_accounted(
    f: &mut Fixture,
    exact: &Liability,
    fault: Fault,
) -> Result<serde_json::Value> {
    ensure(
        !f.imported_read_only && f.head.gate.holds.get(&exact.token) == Some(exact),
        "exact trusted original recipe hold",
    )?;
    let c = exact.continuation.as_ref().ok_or("recipe continuation")?;
    let matches = |h: &Selection, state: &Continuation| {
        h.semantic == exact.target
            && h.active == state.active
            && h.recovery == state.recovery
            && h.inventory.active == state.inventory.active
            && h.inventory.recovery == state.inventory.recovery
            && h.inventory.next >= state.inventory.next
            && h.inventory.next <= c.inventory.next
            && h.data_pack == state.data_pack
            && h.data_end == state.data_end
            && h.meta_pack == state.meta_pack
            && h.meta_end == state.meta_end
    };
    let selected: Selection = Fixture::bounded_read(&f.dir.join("HEAD"))?;
    ensure(selected == f.head, "unknown HEAD before recipe effects")?;
    ensure(
        matches(&selected, &exact.origin) || matches(&selected, c),
        "selected state differs from original recipe states",
    )?;
    if f.dir.join("intent").exists() {
        let old: Selection = Fixture::bounded_read(&f.dir.join("intent"))?;
        let candidate: Selection = Fixture::bounded_read(&f.dir.join("candidate"))?;
        ensure(
            (selected == old || selected == candidate)
                && old.gate.holds.get(&exact.token) == Some(exact)
                && candidate.gate.holds.get(&exact.token) == Some(exact)
                && matches(&old, &exact.origin)
                && matches(&candidate, c),
            "unknown recipe selection/identity",
        )?;
    }
    let recipe = c.recipe.as_ref().ok_or("original recipe absent")?;
    recipe.validate()?;
    accounting::controls(f, exact, recipe)?;
    ensure(
        recipe.data_base.pack == exact.origin.data_pack
            && recipe.data_base.end == exact.origin.data_end
            && recipe.meta_base.pack == exact.origin.meta_pack
            && recipe.meta_base.end == exact.origin.meta_end
            && recipe.data.pack == c.data_pack
            && recipe.data.end == c.data_end
            && recipe.meta.pack == c.meta_pack
            && recipe.meta.end == c.meta_end,
        "recipe/original continuation mismatch",
    )?;
    // Trusted local selected state: reconstruct the bounded typed change from
    // its original snapshot, rather than traversing all historical objects.
    verify_original_plan(f, exact, recipe)?;
    if matches!(fault, Fault::BeforeWrite) {
        return Err("injected pre-effect ENOSPC; original hold retained".into());
    }
    let mut remaining = if let Fault::Partial(n) = fault {
        ensure(n > 0, "positive cut")?;
        Some(n)
    } else {
        None
    };
    replay(f, true, &recipe.data_base, &recipe.data, &mut remaining)?;
    if matches!(fault, Fault::AfterData) {
        return Err("injected after data; original hold retained".into());
    }
    replay(f, false, &recipe.meta_base, &recipe.meta, &mut remaining)?;
    for claim in &recipe.claims {
        claim.verify(f)?;
    }
    f.authorized_hold = Some(exact.token);
    if f.dir.join("intent").exists() {
        f.reconcile()?;
    }
    if f.head.active != c.active || f.head.recovery != c.recovery || f.head.inventory != c.inventory
    {
        f.publish(
            c.active.clone(),
            c.recovery.clone(),
            exact.target.clone(),
            match fault {
                Fault::BeforeHead => Cut::BeforeHead,
                Fault::AfterHead => Cut::AfterHead,
                _ => Cut::None,
            },
        )?;
    }
    if matches!(fault, Fault::BeforeSettlement) {
        return Err("injected before original-token allocation settlement".into());
    }
    let settlement = accounting::observe(f, exact, recipe)?;
    // Allocation observations settle only this original operation's nonnegative
    // pack growth. Control completion stays conservatively reserved; no old
    // allocation or namespace cost is refunded by this bounded observation.
    f.complete_liability(exact, &[(0, settlement.incremental_pack_charge)])?;
    serde_json::to_value(settlement).map_err(error)
}

pub(in super::super) fn run_cli(args: &[String]) -> Result<()> {
    let self_covered = args.first().is_some_and(|s| s == "self-covered");
    let automatic = self_covered || args.first().is_some_and(|s| s == "automatic");
    let args = if automatic { &args[1..] } else { args };
    for n in if args.is_empty() {
        vec![1000]
    } else {
        args.iter()
            .map(|s| s.parse::<u64>().map_err(error))
            .collect::<Result<Vec<_>>>()?
    } {
        ensure((1..=10000).contains(&n), "bounded recipe fixture")?;
        for kind in [Kind::Radix, Kind::Btree] {
            let t = Instant::now();
            let (_temp, mut source, mut f) = setup(n, kind)?;
            let claims = if automatic {
                bootstrap_prefixes(&mut f)?
            } else {
                vec![Claim::capture(&mut f, true, 0)?]
            };
            let observed_pack_charge = claims
                .iter()
                .try_fold(0u64, |n, c| checked(n, c.observed_charge))?;
            let before = f.snapshot()?;
            f.c = Counters::default();
            let p = Instant::now();
            let (hold, preflight) = if self_covered {
                prepare_self_covered(&mut f)?
            } else {
                prepare_claims(&mut f, &claims)?
            };
            let prepare_us = p.elapsed().as_micros();
            let p = Instant::now();
            let settlement = resume_accounted(&mut f, &hold, Fault::None)?;
            let publication = json!({"prepare_us":prepare_us,"resume_us":p.elapsed().as_micros(),"allocation_settlement":settlement,"preflight":preflight,"held":hold.by_domain,"counters":f.c,"before":before,"after":f.snapshot()?});
            let mut samples = vec![];
            for group in [1, 8, 32] {
                let end = source.size();
                logical_group(&mut source, group)?;
                f.c = Counters::default();
                let before = f.snapshot()?;
                let p = Instant::now();
                let result = f.checkpoint(&mut source, end, super::super::Fault::None)?;
                samples.push(json!({"group":group,"us":p.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":result}));
            }
            let p = Instant::now();
            let mut opened = Fixture::reopen_combined(&f.dir)?;
            opened.structural_probe()?;
            let open = json!({"us":p.elapsed().as_micros(),"counters":opened.c});
            opened.c = Counters::default();
            let p = Instant::now();
            opened.retry(&id(1), &request(&id(1)))?;
            let retry = json!({"us":p.elapsed().as_micros(),"counters":opened.c});
            f.audit_combined()?;
            println!(
                "{}",
                json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"selected_claims":claims.len(),"automatic_base_prefixes":automatic,"observed_base_pack_charge":observed_pack_charge,"candidate_suffix_allocation_coverage":self_covered,"control_generation_ownership":false,"candidate_suffix_and_control_coverage":false,"publication":publication,"checkpoints":samples,"structural_open":open,"old_retry":retry,"elapsed_ms":t.elapsed().as_millis(),"complete_refundable_integration":false,"qualified_saved":false})
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn self_covered_suffix_survives_original_token_cuts_and_independent_audit() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                Fault::Partial(7),
                Fault::AfterData,
                Fault::BeforeHead,
                Fault::AfterHead,
            ] {
                let (_temp, _source, mut f) = setup(128, kind)?;
                let (hold, _) = prepare_self_covered(&mut f)?;
                let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
                verify_self_coverage(recipe)?;
                let claims = recipe.claims.clone();
                ensure(
                    claims.iter().all(|c| c.extent > c.content_end),
                    "suffix fixture",
                )?;
                ensure(resume(&mut f, &hold, cut).is_err(), "self covered cut")?;
                f = Fixture::reopen_combined(&f.dir)?;
                resume(&mut f, &hold, Fault::None)?;
                f = Fixture::reopen_combined(&f.dir)?;
                ensure(f.head.gate.holds.is_empty(), "recipe must have completed")?;
                f.audit_combined()?;
                for root in [f.head.active.clone(), f.head.recovery.clone()] {
                    for claim in &claims {
                        ensure(
                            f.ownership_lookup(&root, claim.key()?)? == Some(claim.clone()),
                            "selected suffix claim",
                        )?;
                        let arena = if claim.data { &f.data } else { &f.meta };
                        ensure(claim.extent == arena.end, "self extent exact selected end")?;
                    }
                }
            }
        }
        Ok(())
    }
    #[test]
    fn self_covered_prefix_and_selected_suffix_corruption_still_fail_audit() -> Result<()> {
        for suffix in [false, true] {
            let (_temp, _source, mut f) = setup(128, Kind::Btree)?;
            let (hold, _) = prepare_self_covered(&mut f)?;
            let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
            let (pack, offset) = if suffix {
                let r = &recipe.data.writes.last().unwrap().1;
                (r.pack, r.offset)
            } else {
                (recipe.data_base.pack, 0)
            };
            resume(&mut f, &hold, Fault::None)?;
            let mut file = OpenOptions::new()
                .write(true)
                .open(f.data.path(pack))
                .map_err(error)?;
            file.seek(SeekFrom::Start(offset)).map_err(error)?;
            file.write_all(b"!").map_err(error)?;
            f = Fixture::reopen_combined(&f.dir)?;
            ensure(
                f.audit_combined().is_err(),
                "prefix or selected suffix corruption accepted",
            )?;
        }
        Ok(())
    }
    #[test]
    fn self_covered_transient_frames_are_owned_allocations_not_content_edges() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let (first, _) = prepare_self_covered(&mut f)?;
        resume(&mut f, &first, Fault::None)?;
        let (hold, _) = prepare_self_covered(&mut f)?;
        let recipe = hold
            .continuation
            .as_ref()
            .unwrap()
            .recipe
            .as_ref()
            .unwrap()
            .clone();
        resume(&mut f, &hold, Fault::None)?;
        let mut transient = None;
        for (key, location, _) in &recipe.data.writes {
            if f.lookup(&f.head.active.clone(), key.unwrap())?.is_none()
                && f.lookup(&f.head.recovery.clone(), key.unwrap())?.is_none()
            {
                transient = Some(location.clone());
                break;
            }
        }
        let location = transient.ok_or("fixture needs staged unreachable frame")?;
        let claim = recipe
            .claims
            .iter()
            .find(|c| c.data && c.pack == location.pack)
            .unwrap();
        ensure(
            location.offset >= claim.content_end && location.offset + location.len <= claim.extent,
            "transient allocated suffix",
        )?;
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.data.path(location.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(location.offset)).map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        f = Fixture::reopen_combined(&f.dir)?;
        // No selected content edge promises the bytes of this dead frame.
        // Its extent remains owned, charged, and ineligible for retirement.
        f.audit_combined()?;
        ensure(
            f.gate_candidate(location.pack, true).is_err(),
            "transient allocation retired",
        )
    }
    #[test]
    fn self_coverage_rejects_missing_extent_and_rollover_without_effects() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let (claims, p) = self_covered_plan(&mut f)?;
        let mut recipe = Recipe {
            digest: String::new(),
            claims,
            self_covered: true,
            control_base: f.head.clone(),
            data_base: Prefix::capture(&f.data)?,
            meta_base: Prefix::capture(&f.meta)?,
            data: p.data,
            meta: p.meta,
        };
        recipe.claims.pop();
        recipe.digest = recipe.digest()?;
        ensure(recipe.validate().is_err(), "missing arena accepted")?;
        let fill = serde_json::to_vec(&f.get(&f.head.active.clone())?).map_err(error)?;
        while PACK - f.meta.end >= fill.len() as u64 + 4 {
            f.meta.append(&fill, None, &mut f.c)?;
        }
        f.meta.sync(&mut f.c)?;
        f.select_gate(f.head.gate.clone())?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            prepare_self_covered(&mut f)
                .unwrap_err()
                .contains("rollover refused"),
            "rollover admitted",
        )?;
        ensure(
            f.snapshot()? == before && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
            "rollover refusal effects",
        )
    }
    #[test]
    fn automatic_multi_pack_batch_keeps_exact_locator_closure() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, _source, mut f) = setup(512, kind)?;
            let claims = bootstrap_prefixes(&mut f)?;
            ensure(claims.len() > 2, "multiple allocation units required")?;
            let (hold, _) = prepare_claims(&mut f, &claims)?;
            resume(&mut f, &hold, Fault::None)?;
            f.audit_combined()?;
        }
        Ok(())
    }
    #[test]
    fn automatic_real_large_batch_recipe_cap_refuses_without_effects() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, _source, mut f) = setup(1000, kind)?;
            let claims = bootstrap_prefixes(&mut f)?;
            let before = f.snapshot()?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            ensure(
                prepare_claims(&mut f, &claims)
                    .unwrap_err()
                    .contains("recipe byte admission cap"),
                "real recipe cap",
            )?;
            ensure(
                f.snapshot()? == before
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && f.head.gate.holds.is_empty(),
                "large refusal effects",
            )?;
        }
        Ok(())
    }
    #[test]
    fn automatic_prefix_batch_preserves_both_exact_closures_and_original_token() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for cut in [
                Fault::Partial(7),
                Fault::AfterData,
                Fault::BeforeHead,
                Fault::AfterHead,
            ] {
                let (_temp, _source, mut f) = setup(128, kind)?;
                let semantic = f.head.semantic.clone();
                let claims = bootstrap_prefixes(&mut f)?;
                ensure(
                    claims.iter().any(|c| c.data) && claims.iter().any(|c| !c.data),
                    "both arenas",
                )?;
                let (hold, _) = prepare_claims(&mut f, &claims)?;
                let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
                let expected = recipe.data.bytes + recipe.meta.bytes;
                f.c = Counters::default();
                ensure(resume(&mut f, &hold, cut).is_err(), "automatic batch cut")?;
                let wrote = f.c.data_write_bytes + f.c.meta_write_bytes;
                f = Fixture::reopen_combined(&f.dir)?;
                ensure(
                    f.head.gate.holds.get(&hold.token) == Some(&hold),
                    "batch original token",
                )?;
                resume(&mut f, &hold, Fault::None)?;
                ensure(
                    wrote + f.c.data_write_bytes + f.c.meta_write_bytes == expected,
                    "batch duplicated payload",
                )?;
                f.audit_combined()?;
                for root in [f.head.active.clone(), f.head.recovery.clone()] {
                    for claim in &claims {
                        ensure(
                            f.ownership_lookup(&root, claim.key()?)? == Some(claim.clone()),
                            "automatic claim coverage",
                        )?;
                    }
                }
                ensure(
                    f.head.semantic == semantic && f.head.gate.holds.is_empty(),
                    "batch semantic identity",
                )?;
            }
        }
        Ok(())
    }
    #[test]
    fn automatic_touched_prefix_growth_replaces_without_extraneous_locator_leaves() -> Result<()> {
        let (_temp, mut source, mut f) = setup(128, Kind::Btree)?;
        let claims = bootstrap_prefixes(&mut f)?;
        let (hold, _) = prepare_claims(&mut f, &claims)?;
        resume(&mut f, &hold, Fault::None)?;
        let pin = f.acquire(PinClass::ReaderLease, false)?;
        let end = source.size();
        logical_group(&mut source, 8)?;
        f.checkpoint(&mut source, end, super::super::Fault::None)?;
        let units = [(true, f.data.pack), (false, f.meta.pack)];
        let touched = capture_prefixes(&mut f, &units)?;
        let (hold, _) = prepare_claims(&mut f, &touched)?;
        resume(&mut f, &hold, Fault::None)?;
        f.audit_combined()?;
        for claim in &touched {
            ensure(
                f.ownership_lookup(&f.head.active.clone(), claim.key()?)? == Some(claim.clone()),
                "grown prefix absent",
            )?;
        }
        for claim in &claims {
            ensure(
                f.ownership_lookup(&pin.active, claim.key()?)? == Some(claim.clone()),
                "reader prefix mutated",
            )?;
            ensure(
                f.gate_candidate(claim.pack, claim.data).is_err(),
                "owned pack retirement admitted",
            )?;
        }
        Ok(())
    }
    #[test]
    fn automatic_prefix_caps_duplicates_and_unknown_suffix_have_no_effects() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            capture_prefixes(&mut f, &[(true, 0), (true, 0)]).is_err(),
            "duplicate captured",
        )?;
        ensure(
            capture_prefixes(&mut f, &[(true, 0); 65]).is_err(),
            "capture cap",
        )?;
        let claim = Claim::capture(&mut f, true, 0)?;
        ensure(
            prepare_claims(&mut f, &[claim.clone(), claim]).is_err(),
            "duplicate planned",
        )?;
        ensure(
            f.snapshot()? == before && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
            "refusal effects",
        )?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(f.data.path(f.data.pack))
            .map_err(error)?;
        file.write_all(b"?").map_err(error)?;
        let after = f.snapshot()?;
        let pack = f.data.pack;
        ensure(
            capture_prefixes(&mut f, &[(true, pack)]).is_err(),
            "unknown suffix captured",
        )?;
        ensure(f.snapshot()? == after, "capture changed unknown suffix")
    }
    #[test]
    fn automatic_metadata_prefix_corruption_preserves_original_batch_evidence() -> Result<()> {
        let (_temp, _source, mut f) = setup(128, Kind::Radix)?;
        let claims = bootstrap_prefixes(&mut f)?;
        let (hold, _) = prepare_claims(&mut f, &claims)?;
        ensure(
            resume(&mut f, &hold, Fault::BeforeWrite).is_err(),
            "before write cut",
        )?;
        let claim = claims.iter().find(|c| !c.data).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.meta.path(claim.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(claim.extent - 1))
            .map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            resume(&mut f, &hold, Fault::None)
                .unwrap_err()
                .contains("owned prefix corruption"),
            "metadata corruption accepted",
        )?;
        ensure(
            f.snapshot()? == before && f.head.gate.holds.get(&hold.token) == Some(&hold),
            "corruption lost batch evidence",
        )
    }
    #[test]
    fn every_recipe_cut_reopens_and_completes_original_token() -> Result<()> {
        for fault in [
            Fault::BeforeWrite,
            Fault::Partial(1),
            Fault::Partial(7),
            Fault::Partial(100),
            Fault::AfterData,
            Fault::BeforeHead,
            Fault::AfterHead,
        ] {
            let (_temp, mut source, mut f) = setup(64, Kind::Radix)?;
            let semantic = f.head.semantic.clone();
            let claim = Claim::capture(&mut f, true, 0)?;
            let (hold, _) = prepare(&mut f, &claim)?;
            ensure(
                resume(&mut f, &hold, fault).is_err(),
                "missing recipe fault",
            )?;
            f = Fixture::reopen_combined(&f.dir)?;
            ensure(
                f.head.gate.holds.get(&hold.token) == Some(&hold),
                "original liability lost",
            )?;
            f.resume_checkpoint(&mut source, 0, &hold)?;
            f.audit_combined()?;
            ensure(
                f.head.semantic == semantic && f.head.gate.holds.is_empty(),
                "recipe completion identity",
            )?;
        }
        Ok(())
    }
    #[test]
    fn repeated_partial_and_pre_head_retries_never_duplicate_payload() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let (hold, _) = prepare(&mut f, &claim)?;
        let r = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
        let expected = r.data.bytes + r.meta.bytes;
        let mut written = 0;
        for fault in [
            Fault::Partial(1),
            Fault::Partial(2),
            Fault::Partial(7),
            Fault::BeforeHead,
            Fault::BeforeHead,
            Fault::BeforeHead,
        ] {
            f.c = Counters::default();
            ensure(resume(&mut f, &hold, fault).is_err(), "fault")?;
            written += f.c.data_write_bytes + f.c.meta_write_bytes;
            f = Fixture::reopen_combined(&f.dir)?;
            ensure(
                f.head.gate.holds.get(&hold.token) == Some(&hold),
                "original recipe bytes/digest changed",
            )?;
        }
        f.c = Counters::default();
        resume(&mut f, &hold, Fault::None)?;
        written += f.c.data_write_bytes + f.c.meta_write_bytes;
        ensure(
            written == expected,
            "recipe payload was rewritten or duplicated",
        )?;
        f.audit_combined()?;
        Ok(())
    }
    #[test]
    fn mismatched_partial_frame_refuses_without_deletion_or_fresh_hold() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let (hold, _) = prepare(&mut f, &claim)?;
        ensure(
            resume(&mut f, &hold, Fault::Partial(1)).is_err(),
            "partial cut",
        )?;
        let base = &hold
            .continuation
            .as_ref()
            .unwrap()
            .recipe
            .as_ref()
            .unwrap()
            .data_base;
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.data.path(base.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(base.end)).map_err(error)?;
        file.write_all(&[255]).map_err(error)?;
        file.sync_all().map_err(error)?;
        f = Fixture::reopen_combined(&f.dir)?;
        let before = f.snapshot()?;
        ensure(
            resume(&mut f, &hold, Fault::None)
                .unwrap_err()
                .contains("partial recipe suffix corruption"),
            "corrupt occupied frame accepted",
        )?;
        ensure(
            f.snapshot()? == before && f.head.gate.holds.get(&hold.token) == Some(&hold),
            "corruption caused effect or changed liability",
        )
    }
    #[test]
    fn admission_and_recipe_cap_refuse_before_pack_effects() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let mut gate = f.head.gate.clone();
        let allowance = f.control_bound(&gate)?;
        gate.domains.get_mut(&0).unwrap().limit = gate.domains[&0].charged + allowance + 8192;
        f.select_gate(gate)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        let claim = Claim::capture(&mut f, true, 0)?;
        ensure(
            prepare(&mut f, &claim).is_err(),
            "exhausted project budget admitted",
        )?;
        ensure(
            f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && !f.dir.join("intent").exists(),
            "admission refusal wrote state",
        )?;
        let p = plan(&mut f, &claim)?;
        let recipe = Recipe {
            digest: String::new(),
            claims: vec![claim.clone()],
            self_covered: false,
            control_base: f.head.clone(),
            data_base: Prefix::capture(&f.data)?,
            meta_base: Prefix::capture(&f.meta)?,
            data: p.data,
            meta: p.meta,
        };
        let mut over = recipe;
        over.digest = "x".repeat(MAX_RECIPE);
        ensure(
            over.validate().unwrap_err() == "recipe byte admission cap",
            "oversized recipe accepted",
        )?;
        ensure(f.snapshot()? == before, "recipe cap wrote files")
    }
    #[test]
    fn partial_new_metadata_pack_reopens_original_rollover_recipe() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let page = f.get(&f.head.active.clone())?;
        let fill = serde_json::to_vec(&page).map_err(error)?;
        while PACK - f.meta.end >= fill.len() as u64 + 4 {
            f.meta.append(&fill, None, &mut f.c)?;
        }
        f.meta.sync(&mut f.c)?;
        sync_dir(&f.dir, &mut f.c)?;
        f.select_gate(f.head.gate.clone())?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let (hold, _) = prepare(&mut f, &claim)?;
        let recipe = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
        ensure(
            recipe.meta.pack > recipe.meta_base.pack,
            "fixture must rotate metadata",
        )?;
        let cut = usize::try_from(recipe.data.bytes + 5).map_err(error)?;
        ensure(
            resume(&mut f, &hold, Fault::Partial(cut)).is_err(),
            "new-pack partial cut",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        resume(&mut f, &hold, Fault::None)?;
        f.audit_combined()?;
        Ok(())
    }
    #[test]
    fn unknown_head_keeps_original_recipe_and_both_selector_evidences() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let (hold, _) = prepare(&mut f, &claim)?;
        ensure(
            resume(&mut f, &hold, Fault::BeforeHead).is_err(),
            "selector cut",
        )?;
        let intent = fs::read(f.dir.join("intent")).map_err(error)?;
        let candidate = fs::read(f.dir.join("candidate")).map_err(error)?;
        let mut unknown = f.head.clone();
        unknown.epoch += 100;
        fs::write(
            f.dir.join("HEAD"),
            serde_json::to_vec(&unknown).map_err(error)?,
        )
        .map_err(error)?;
        ensure(
            Fixture::reopen_combined(&f.dir).is_err(),
            "unknown selector reopened",
        )?;
        ensure(
            resume(&mut f, &hold, Fault::None).is_err(),
            "unknown selector replayed",
        )?;
        ensure(
            fs::read(f.dir.join("intent")).map_err(error)? == intent
                && fs::read(f.dir.join("candidate")).map_err(error)? == candidate,
            "unknown result destroyed original recipe evidence",
        )
    }
    #[test]
    fn symlink_new_pack_refuses_without_writing_target() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
        let fill = serde_json::to_vec(&f.get(&f.head.active.clone())?).map_err(error)?;
        while PACK - f.meta.end >= fill.len() as u64 + 4 {
            f.meta.append(&fill, None, &mut f.c)?;
        }
        f.meta.sync(&mut f.c)?;
        sync_dir(&f.dir, &mut f.c)?;
        f.select_gate(f.head.gate.clone())?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let (hold, _) = prepare(&mut f, &claim)?;
        let r = hold.continuation.as_ref().unwrap().recipe.as_ref().unwrap();
        ensure(
            r.meta.pack > r.meta_base.pack,
            "new metadata destination required",
        )?;
        let surrogate = tempfile::NamedTempFile::new().map_err(error)?;
        std::os::unix::fs::symlink(surrogate.path(), f.meta.path(r.meta.pack)).map_err(error)?;
        ensure(
            resume(&mut f, &hold, Fault::None)
                .unwrap_err()
                .contains("private regular file"),
            "symlink destination accepted",
        )?;
        ensure(
            fs::metadata(surrogate.path()).map_err(error)?.len() == 0
                && f.head.gate.holds.get(&hold.token) == Some(&hold),
            "symlink target written or liability lost",
        )
    }
}

pub(in super::super) fn verify_controls(
    f: &mut Fixture,
    exact: &Liability,
    base: &Selection,
) -> Result<()> {
    accounting::controls_from_base(f, exact, base).map(|_| ())
}
