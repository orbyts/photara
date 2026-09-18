//! Bounded original-token write recipe for actual typed-v3 payload recovery.
//! HEAD-held bytes are a disposable control-amplification experiment, not wire.
#[allow(clippy::wildcard_imports, reason = "Actual typed fixture child")]
use super::*;
const MAX_RECIPE: usize = 48 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Prefix {
    pack: u64,
    end: u64,
    dev: u64,
    ino: u64,
    sha: String,
}
impl Prefix {
    fn capture(arena: &Arena) -> Result<Self> {
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
            sha: hash(&fs::read(arena.path(arena.pack)).map_err(error)?),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct Recipe {
    digest: String,
    claim: Claim,
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
}
type Segment = (u64, u64, Vec<u8>);
fn segments(base: &Prefix, plan: &WritePlan, data: bool) -> Result<Vec<Segment>> {
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
        Ok(())
    }
}
fn prepare(f: &mut Fixture, claim: &Claim) -> Result<(Liability, serde_json::Value)> {
    let p = plan(f, claim)?;
    let mut recipe = Recipe {
        digest: String::new(),
        claim: claim.clone(),
        data_base: Prefix::capture(&f.data)?,
        meta_base: Prefix::capture(&f.meta)?,
        data: p.data,
        meta: p.meta,
    };
    recipe.digest = recipe.digest()?;
    f.c.recipe_prefix_read_bytes += claim.extent + recipe.data_base.end + recipe.meta_base.end;
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
fn replay(
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
    let generated = plan(f, &recipe.claim);
    f.head = selected;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
    f.cache.clear();
    f.staged = None;
    let generated = generated?;
    f.c.recipe_prefix_read_bytes += recipe.claim.extent;
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
    let consumed = exact
        .by_domain
        .iter()
        .map(|(d, n)| (*d, if *d == 0 { n - exact.control } else { *n }))
        .collect::<Vec<_>>();
    f.complete_liability(exact, &consumed)
}

pub(in super::super) fn run_cli(args: &[String]) -> Result<()> {
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
            let claim = Claim::capture(&f, true, 0)?;
            let before = f.snapshot()?;
            f.c = Counters::default();
            let p = Instant::now();
            let (hold, preflight) = prepare(&mut f, &claim)?;
            let prepare_us = p.elapsed().as_micros();
            let p = Instant::now();
            resume(&mut f, &hold, Fault::None)?;
            let publication = json!({"prepare_us":prepare_us,"resume_us":p.elapsed().as_micros(),"preflight":preflight,"held":hold.by_domain,"counters":f.c,"before":before,"after":f.snapshot()?});
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
                json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"selected_claims":1,"publication":publication,"checkpoints":samples,"structural_open":open,"old_retry":retry,"elapsed_ms":t.elapsed().as_millis(),"complete_refundable_integration":false,"qualified_saved":false})
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let claim = Claim::capture(&f, true, 0)?;
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
        let claim = Claim::capture(&f, true, 0)?;
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
        let claim = Claim::capture(&f, true, 0)?;
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
        let claim = Claim::capture(&f, true, 0)?;
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
            claim: claim.clone(),
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
        let claim = Claim::capture(&f, true, 0)?;
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
        let claim = Claim::capture(&f, true, 0)?;
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
        let claim = Claim::capture(&f, true, 0)?;
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
