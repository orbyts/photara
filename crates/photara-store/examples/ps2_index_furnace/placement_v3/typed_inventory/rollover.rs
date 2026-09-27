//! Disposable witnessed Graph rollover and post-barrier ownership finalization.
//!
//! Birth, payload, finalization and publication share one immutable admission.
//! A phase may consume that admission only while selecting exact allocation
//! attribution with the final Graph roots. No sizing claim is ever published.
#[allow(
    clippy::wildcard_imports,
    reason = "Actual-v3 bounded rollover fixture"
)]
use super::*;
use super::{fresh_generation, graph, recipe, retirement_ledger};

const MAX_GENERATIONS: u64 = 4;
const CLEANUP_RESERVE: u64 = 128 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Generation {
    data: bool,
    pack: u64,
    dev: u64,
    ino: u64,
}
impl Generation {
    fn open(&self, f: &Fixture) -> Result<File> {
        let arena = if self.data { &f.data } else { &f.meta };
        let path = arena.path(self.pack);
        let named = fs::symlink_metadata(&path).map_err(error)?;
        ensure(
            named.is_file() && named.nlink() == 1,
            "rollover private generation",
        )?;
        let fd = rustix::fs::open(
            &path,
            rustix::fs::OFlags::RDWR | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(error)?;
        let file = File::from(fd);
        let opened = file.metadata().map_err(error)?;
        ensure(
            opened.dev() == self.dev
                && opened.ino() == self.ino
                && named.dev() == self.dev
                && named.ino() == self.ino
                && opened.nlink() == 1
                && opened.len() <= PACK,
            "rollover original witnessed generation changed",
        )?;
        Ok(file)
    }
}

#[derive(Clone, Debug)]
pub(in super::super) struct Payload {
    data: WritePlan,
    meta: WritePlan,
    active: PRef,
    semantic: Head,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PayloadCommitment {
    data_sha: String,
    meta_sha: String,
    active: PRef,
    data_pack: u64,
    data_end: u64,
    meta_pack: u64,
    meta_end: u64,
}
impl Payload {
    fn commitment(&self) -> Result<PayloadCommitment> {
        Ok(PayloadCommitment {
            data_sha: hash(&serde_json::to_vec(&self.data).map_err(error)?),
            meta_sha: hash(&serde_json::to_vec(&self.meta).map_err(error)?),
            active: self.active.clone(),
            data_pack: self.data.pack,
            data_end: self.data.end,
            meta_pack: self.meta.pack,
            meta_end: self.meta.end,
        })
    }
}
impl From<PreparedAppend> for Payload {
    fn from(p: PreparedAppend) -> Self {
        Self {
            data: p.data,
            meta: p.meta,
            active: p.active,
            semantic: p.semantic,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Corridor {
    data_pack: u64,
    data_start: u64,
    data_bound: u64,
    meta_pack: u64,
    meta_start: u64,
    meta_bound: u64,
    // Shape commitment includes exact allocation keys, immutable base and
    // payload. Maximum-width sizing numbers never become observed attribution.
    shape_sha: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Admission {
    base: Selection,
    target: Head,
    payload: PayloadCommitment,
    data_base: recipe::Prefix,
    meta_base: recipe::Prefix,
    corridor: Corridor,
    source: graph::SourceCharge,
    admitted: BTreeMap<u64, u64>,
    cleanup: u64,
    dedicated_corridors: bool,
    control: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Finalizer {
    claims: Vec<Claim>,
    data_base: recipe::Prefix,
    meta_base: recipe::Prefix,
    data_sha: String,
    meta_sha: String,
    continuation: Continuation,
}
struct FinalizationPlan {
    recipe: Finalizer,
    data: WritePlan,
    meta: WritePlan,
}
pub(in super::super) struct Draft {
    admission: Admission,
    payload: Payload,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
enum Phase {
    Birth,
    Payload {
        generations: Vec<Generation>,
    },
    Finalization {
        generations: Vec<Generation>,
        recipe: Box<Finalizer>,
    },
    Published {
        generations: Vec<Generation>,
        recipe: Box<Finalizer>,
        consumed: BTreeMap<u64, u64>,
        // Exact transfer selector is independently derived from Admission and
        // final observed tips/source; it is not an arbitrary supplied balance.
        ledger: Box<retirement_ledger::Ledger>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Rollover {
    admission: Admission,
    phase: Phase,
}
impl Rollover {
    pub(in super::super) fn remaining(&self, domain: u64) -> Result<u64> {
        let admitted = self.admission.admitted.get(&domain).copied().unwrap_or(0);
        let consumed = match &self.phase {
            Phase::Published { consumed, .. } => consumed.get(&domain).copied().unwrap_or(0),
            _ => 0,
        };
        let remaining = admitted
            .checked_sub(consumed)
            .ok_or("rollover expanded consumption")?;
        ensure(
            domain != 0 || remaining >= self.admission.cleanup,
            "rollover original cleanup corridor exhausted",
        )?;
        Ok(remaining)
    }
    fn generations(&self) -> Result<&[Generation]> {
        match &self.phase {
            Phase::Birth => Err("rollover births not completely witnessed".into()),
            Phase::Payload { generations }
            | Phase::Finalization { generations, .. }
            | Phase::Published { generations, .. } => Ok(generations),
        }
    }
    fn generation(&self, data: bool, pack: u64) -> Result<&Generation> {
        self.generations()?
            .iter()
            .find(|g| g.data == data && g.pack == pack)
            .ok_or("rollover generation absent from original witnesses".into())
    }
}

// Finalization uses the same typed builder and locator implementation as the
// same-tip path. It writes no semantic objects a second time.
fn finalizer_once(
    f: &mut Fixture,
    a: &Admission,
    claims: &[Claim],
    sizing: bool,
) -> Result<FinalizationPlan> {
    let old = &a.base;
    let mut builder = Builder {
        data: WritePlan {
            pack: a.corridor.data_pack,
            end: a.corridor.data_start,
            bytes: 0,
            writes: vec![],
        },
        next: old.inventory.next,
        added: vec![],
        removed: vec![],
    };
    let mut root = old.inventory.active.clone();
    for claim in claims {
        if !sizing {
            claim.verify_extent(f, true)?;
        }
        root = Some(builder.insert(f, &old.active, root, claim)?);
    }
    let root = root.ok_or("rollover ownership root")?;
    let updates = builder.updates();
    f.staged = Some(Vec::new());
    let active = f
        .multi(Some(a.payload.active.clone()), &updates)?
        .ok_or("rollover active root")?;
    let recovery = f
        .multi(Some(old.active.clone()), &updates)?
        .ok_or("rollover recovery root")?;
    let pages = f.staged.take().ok_or("rollover staged pages")?;
    let mut meta = WritePlan {
        pack: a.corridor.meta_pack,
        end: a.corridor.meta_start,
        bytes: 0,
        writes: vec![],
    };
    let mut memo = HashMap::new();
    let active = meta.staged(&active, &pages, &mut memo)?;
    let recovery = meta.staged(&recovery, &pages, &mut memo)?;
    ensure(
        builder.data.pack == a.corridor.data_pack && meta.pack == a.corridor.meta_pack,
        "rollover finalizer requires another unadmitted generation",
    )?;
    let continuation = Continuation {
        recipe: None,
        inventory: Inventory {
            active: Some(root.clone()),
            recovery: Some(root),
            next: builder.next,
        },
        active,
        recovery,
        data_pack: builder.data.pack,
        data_end: builder.data.end,
        meta_pack: meta.pack,
        meta_end: meta.end,
    };
    let recipe = Finalizer {
        claims: claims.to_vec(),
        data_base: a.data_base.clone(),
        meta_base: a.meta_base.clone(),
        data_sha: hash(&serde_json::to_vec(&builder.data).map_err(error)?),
        meta_sha: hash(&serde_json::to_vec(&meta).map_err(error)?),
        continuation,
    };
    Ok(FinalizationPlan {
        recipe,
        data: builder.data,
        meta,
    })
}

// Overlay the finite unselected payload metadata for NO-EFFECT corridor planning.
// No fake physical file is created and original ownership verification remains
// enabled inside Builder::insert. The caller restores all planning coordinates.
fn overlay(f: &mut Fixture, a: &Admission, payload: &Payload) -> Result<()> {
    f.head = a.base.clone();
    f.data.pack = a.corridor.data_pack;
    f.data.end = a.corridor.data_start;
    f.meta.pack = a.corridor.meta_pack;
    f.meta.end = a.corridor.meta_start;
    f.head.meta_pack = a.corridor.meta_pack;
    f.head.meta_end = if a.corridor.meta_pack == a.payload.meta_pack {
        a.payload.meta_end
    } else {
        0
    };
    f.cache.clear();
    ensure(
        payload.meta.writes.len() <= TX_OBJECTS * 130,
        "rollover metadata overlay bound",
    )?;
    let mut pages = HashMap::new();
    for (_, r, bytes) in &payload.meta.writes {
        let page: Page = serde_json::from_slice(bytes).map_err(error)?;
        ensure(
            pages.insert(r.clone(), page).is_none(),
            "duplicate rollover planned metadata",
        )?;
    }
    f.planned_metadata = Some(pages);
    Ok(())
}
fn with_overlay<T>(
    f: &mut Fixture,
    a: &Admission,
    payload: &Payload,
    action: impl FnOnce(&mut Fixture) -> Result<T>,
) -> Result<T> {
    let selected = f.head.clone();
    let tips = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    let result = (|| {
        overlay(f, a, payload)?;
        action(f)
    })();
    f.head = selected;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
    f.staged = None;
    f.planned_metadata = None;
    f.cache.clear();
    result
}

fn payload_extents(base: &recipe::Prefix, p: &WritePlan, data: bool) -> Result<BTreeMap<u64, u64>> {
    recipe::segments(base, p, data)?
        .into_iter()
        .map(|(pack, start, bytes)| Ok((pack, checked(start, bytes.len() as u64)?)))
        .collect()
}
fn claim_coordinates(a: &Admission, payload: &Payload) -> Result<Vec<(bool, u64, u64, bool)>> {
    let mut out = vec![];
    for (data, base, p, final_pack) in [
        (true, &a.data_base, &payload.data, a.corridor.data_pack),
        (false, &a.meta_base, &payload.meta, a.corridor.meta_pack),
    ] {
        ensure(
            final_pack >= base.pack && final_pack - base.pack <= MAX_GENERATIONS,
            "rollover generation admission bound",
        )?;
        let extents = payload_extents(base, p, data)?;
        for pack in base.pack..=final_pack {
            let end = extents.get(&pack).copied().unwrap_or(0);
            ensure(
                pack <= p.pack || pack == final_pack && pack == p.pack + 1,
                "rollover unplanned physical gap",
            )?;
            out.push((data, pack, end, pack < final_pack));
        }
    }
    Ok(out)
}
fn sizing_claims(a: &Admission, payload: &Payload) -> Result<Vec<Claim>> {
    claim_coordinates(a, payload)?
        .into_iter()
        .map(|(data, pack, end, sealed)| {
            let base = if data { &a.data_base } else { &a.meta_base };
            let (dev, ino) = if pack == base.pack {
                (base.dev, base.ino)
            } else {
                (u64::MAX, u64::MAX)
            };
            let claim = Claim {
                data,
                pack,
                dev,
                ino,
                extent: if sealed { end } else { PACK },
                content_end: end,
                sha: if pack == base.pack && end == base.end {
                    base.sha.clone()
                } else {
                    hash(b"sizing-only; never selected")
                },
                sealed,
                observed_charge: u64::MAX,
                charge: if sealed {
                    Some(retirement_ledger::AllocationCharge::sizing(
                        data, pack, dev, ino, end,
                    ))
                } else {
                    None
                },
            };
            Ok(claim)
        })
        .collect()
}
fn shape_digest(a: &Admission, payload: &Payload) -> Result<String> {
    Ok(hash(
        &serde_json::to_vec(&(
            &a.base,
            &a.payload,
            a.corridor.data_pack,
            a.corridor.data_start,
            a.corridor.meta_pack,
            a.corridor.meta_start,
            claim_coordinates(a, payload)?,
        ))
        .map_err(error)?,
    ))
}

/// Derive a NO-EFFECT admission. Caller adds canonical Graph journal/source
/// reservation and one birth plan before publishing the immutable original hold.
pub(in super::super) fn prepare(
    f: &mut Fixture,
    source: &mut Store,
    old_end: u64,
    original: Option<&Rollover>,
    dedicated: bool,
) -> Result<Draft> {
    let head = f.head.clone();
    let tips = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    let result = prepare_inner(f, source, old_end, original, dedicated);
    f.head = head;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = tips;
    f.staged = None;
    f.planned_metadata = None;
    f.cache.clear();
    result
}
fn prepare_inner(
    f: &mut Fixture,
    source: &mut Store,
    old_end: u64,
    original: Option<&Rollover>,
    dedicated: bool,
) -> Result<Draft> {
    ensure(
        f.head.gate.holds.is_empty() && f.head.gate.ledger.is_some(),
        "rollover idle attributed base",
    )?;
    let base = f.head.clone();
    let (data_base, meta_base) = if let Some(r) = original {
        (r.admission.data_base.clone(), r.admission.meta_base.clone())
    } else {
        (
            recipe::Prefix::capture(&f.data)?,
            recipe::Prefix::capture(&f.meta)?,
        )
    };
    let payload: Payload = f.checkpoint_plan(source, old_end)?.into();
    let registered = retirement_ledger::source_charge(f)?;
    let mut refusal = "rollover corridor does not fit bounded final tips".to_string();
    // Reuse natural payload tips when possible. At most one additional enrolled
    // corridor-only generation per arena is considered, entirely before effects.
    let dedicated = original.map_or(dedicated, |r| r.admission.dedicated_corridors);
    let choices = if dedicated {
        vec![(true, true)]
    } else {
        vec![(false, false), (false, true), (true, false), (true, true)]
    };
    for (extra_data, extra_meta) in choices {
        let mut a = Admission {
            base: base.clone(),
            target: payload.semantic.clone(),
            payload: payload.commitment()?,
            data_base: data_base.clone(),
            meta_base: meta_base.clone(),
            source: registered.clone(),
            corridor: Corridor {
                data_pack: checked(payload.data.pack, u64::from(extra_data))?,
                data_start: if extra_data { 0 } else { payload.data.end },
                data_bound: 0,
                meta_pack: checked(payload.meta.pack, u64::from(extra_meta))?,
                meta_start: if extra_meta { 0 } else { payload.meta.end },
                meta_bound: 0,
                shape_sha: String::new(),
            },
            admitted: BTreeMap::new(),
            cleanup: CLEANUP_RESERVE,
            dedicated_corridors: dedicated,
            control: 4 * (GATE_MAX as u64 + 4096) + 8 * 4096 + 2 * 65536,
        };
        let planned = (|| {
            let claims = sizing_claims(&a, &payload)?;
            with_overlay(f, &a, &payload, |f| finalizer_once(f, &a, &claims, true))
        })();
        match planned {
            Ok(proof) => {
                a.corridor.data_bound = proof.data.bytes;
                a.corridor.meta_bound = proof.meta.bytes;
                a.corridor.shape_sha = shape_digest(&a, &payload)?;
                let units = (a.corridor.data_pack - a.base.data_pack + 1)
                    + (a.corridor.meta_pack - a.base.meta_pack + 1);
                let physical = units
                    .checked_mul(PACK + 32768)
                    .ok_or("rollover admission overflow")?;
                a.admitted.insert(
                    0,
                    checked(
                        checked(physical, a.control)?,
                        16 * 1024 * 1024 + 4 * 256 * 1024,
                    )?,
                );
                return Ok(Draft {
                    admission: a,
                    payload,
                });
            }
            Err(e) => refusal = e,
        }
    }
    Err(refusal)
}

/// Exact witnessed replay: no mkdir/create/adopt fallback is allowed. Birth
/// witnesses are checked even for byte-identical replacements and empty files.
fn replay(
    f: &mut Fixture,
    r: &Rollover,
    data: bool,
    base: &recipe::Prefix,
    plan: &WritePlan,
    remaining: &mut Option<usize>,
) -> Result<()> {
    for (pack, start, expected) in recipe::segments(base, plan, data)? {
        let generation = r.generation(data, pack)?;
        let mut file = generation.open(f)?;
        let m = file.metadata().map_err(error)?;
        ensure(
            m.len() >= start && m.len() <= start + expected.len() as u64,
            "rollover unexpected witnessed extent",
        )?;
        if pack == base.pack {
            ensure(
                generation.dev == base.dev && generation.ino == base.ino && start == base.end,
                "rollover original prefix witness",
            )?;
            let mut prefix = vec![0; usize::try_from(base.end).map_err(error)?];
            file.read_exact(&mut prefix).map_err(error)?;
            f.c.recipe_prefix_read_bytes += base.end;
            ensure(
                hash(&prefix) == base.sha,
                "rollover committed prefix changed",
            )?;
        }
        let present = usize::try_from(m.len() - start).map_err(error)?;
        file.seek(SeekFrom::Start(start)).map_err(error)?;
        let mut bytes = vec![0; present];
        file.read_exact(&mut bytes).map_err(error)?;
        f.c.recipe_suffix_read_bytes += present as u64;
        ensure(
            bytes == expected[..present],
            "rollover conflicting partial suffix",
        )?;
        let left = &expected[present..];
        let count = remaining.map_or(left.len(), |n| n.min(left.len()));
        file.seek(SeekFrom::Start(m.len())).map_err(error)?;
        file.write_all(&left[..count]).map_err(error)?;
        if data {
            f.c.data_write_bytes += count as u64;
        } else {
            f.c.meta_write_bytes += count as u64;
        }
        if let Some(n) = remaining {
            *n -= count;
            if *n == 0 {
                return Err("cut during witnessed rollover payload".into());
            }
        }
        sync_file(&file, &mut f.c)?;
    }
    sync_dir(&f.dir, &mut f.c)?;
    Ok(())
}

fn verify_payload(f: &mut Fixture, r: &Rollover, payload: &Payload) -> Result<()> {
    let admission = &r.admission;
    ensure(
        payload.commitment()? == admission.payload && payload.semantic == admission.target,
        "rollover original payload commitment changed",
    )?;
    ensure(
        shape_digest(admission, payload)? == admission.corridor.shape_sha,
        "rollover corridor shape changed",
    )?;
    for (data, base, plan) in [
        (true, &admission.data_base, &payload.data),
        (false, &admission.meta_base, &payload.meta),
    ] {
        for (pack, start, suffix) in recipe::segments(base, plan, data)? {
            let mut file = r.generation(data, pack)?.open(f)?;
            let metadata = file.metadata().map_err(error)?;
            ensure(
                metadata.len() == start + suffix.len() as u64,
                "rollover payload not complete",
            )?;
            let mut bytes = vec![0; usize::try_from(metadata.len()).map_err(error)?];
            file.read_exact(&mut bytes).map_err(error)?;
            f.c.recipe_prefix_read_bytes += metadata.len();
            let prefix_len = usize::try_from(start).map_err(error)?;
            ensure(
                bytes[prefix_len..] == suffix,
                "rollover payload differs from original recipe",
            )?;
            if pack == base.pack {
                ensure(
                    hash(&bytes[..prefix_len]) == base.sha,
                    "rollover original payload prefix changed",
                )?;
            }
        }
    }
    Ok(())
}
fn observed_claims(f: &mut Fixture, r: &Rollover, payload: &Payload) -> Result<Vec<Claim>> {
    verify_payload(f, r, payload)?;
    let mut claims = vec![];
    for (data, pack, end, sealed) in claim_coordinates(&r.admission, payload)? {
        let witness = r.generation(data, pack)?;
        let mut file = witness.open(f)?;
        let m = file.metadata().map_err(error)?;
        ensure(m.len() == end, "rollover finalizer prefix extent")?;
        let mut bytes = vec![0; usize::try_from(end).map_err(error)?];
        file.read_exact(&mut bytes).map_err(error)?;
        f.c.recipe_prefix_read_bytes += end;
        let observed_charge = m
            .blocks()
            .checked_mul(512)
            .ok_or("rollover charge overflow")?
            .max(end);
        let mut claim = Claim {
            data,
            pack,
            dev: witness.dev,
            ino: witness.ino,
            extent: end,
            content_end: end,
            sha: hash(&bytes),
            sealed,
            observed_charge: if sealed { observed_charge } else { end },
            charge: None,
        };
        if sealed {
            claim.charge = Some(retirement_ledger::rollover_sealed_charge(
                &r.admission.base,
                &claim,
            )?);
        }
        claims.push(claim);
    }
    Ok(claims)
}
fn claim_prefix(claim: &Claim) -> recipe::Prefix {
    recipe::Prefix {
        pack: claim.pack,
        end: claim.content_end,
        dev: claim.dev,
        ino: claim.ino,
        observed_charge: claim.observed_charge,
        sha: claim.sha.clone(),
    }
}
fn finalize_plan(
    f: &mut Fixture,
    a: &Admission,
    payload: &Payload,
    original: &[Claim],
) -> Result<FinalizationPlan> {
    let mut claims = original.to_vec();
    with_overlay(f, a, payload, |f| {
        for _ in 0..16 {
            let mut plan = finalizer_once(f, a, &claims, false)?;
            ensure(
                plan.data.bytes <= a.corridor.data_bound
                    && plan.meta.bytes <= a.corridor.meta_bound,
                "actual finalizer exceeds original sizing proof",
            )?;
            let mut changed = false;
            for claim in &mut claims {
                if !claim.sealed {
                    let extent = if claim.data {
                        plan.data.end
                    } else {
                        plan.meta.end
                    };
                    if claim.extent != extent {
                        claim.extent = extent;
                        changed = true;
                    }
                }
            }
            if !changed {
                let data = claims
                    .iter()
                    .find(|c| c.data && !c.sealed)
                    .ok_or("rollover final data tip")?;
                let meta = claims
                    .iter()
                    .find(|c| !c.data && !c.sealed)
                    .ok_or("rollover final metadata tip")?;
                plan.recipe.data_base = claim_prefix(data);
                plan.recipe.meta_base = claim_prefix(meta);
                return Ok(plan);
            }
        }
        Err("rollover finalization fixed point exceeds bound".into())
    })
}

/// Called only after payload file and directory barriers. The returned binding
/// must be selected under the original token before finalizer replay begins.
fn bind_finalization(
    f: &mut Fixture,
    r: &Rollover,
    payload: &Payload,
) -> Result<(Rollover, FinalizationPlan)> {
    ensure(
        matches!(r.phase, Phase::Payload { .. }),
        "rollover payload phase required",
    )?;
    let claims = observed_claims(f, r, payload)?;
    let plan = finalize_plan(f, &r.admission, payload, &claims)?;
    let mut next = r.clone();
    next.phase = Phase::Finalization {
        generations: r.generations()?.to_vec(),
        recipe: Box::new(plan.recipe.clone()),
    };
    ensure(
        serde_json::to_vec(&next).map_err(error)?.len() <= GATE_MAX / 2,
        "rollover bounded finalization control recipe",
    )?;
    Ok((next, plan))
}
fn rebuild_finalizer(f: &mut Fixture, r: &Rollover, payload: &Payload) -> Result<FinalizationPlan> {
    let (Phase::Finalization { recipe: stored, .. } | Phase::Published { recipe: stored, .. }) =
        &r.phase
    else {
        return Err("rollover original finalization binding absent".into());
    };
    ensure(
        payload.commitment()? == r.admission.payload,
        "rollover payload commitment changed",
    )?;
    let plan = finalize_plan(f, &r.admission, payload, &stored.claims)?;
    ensure(
        &plan.recipe == stored.as_ref(),
        "rollover finalizer differs from original binding",
    )?;
    for claim in &stored.claims {
        r.generation(claim.data, claim.pack)?.open(f)?;
        with_overlay(f, &r.admission, payload, |f| claim.verify_extent(f, true))?;
        if claim.sealed {
            ensure(
                claim.charge.as_ref()
                    == Some(&retirement_ledger::rollover_sealed_charge(
                        &r.admission.base,
                        claim,
                    )?),
                "rollover sealed charge changed",
            )?;
        }
    }
    Ok(plan)
}

/// Transfer only observed allocation growth at the SAME publication selecting
/// the final roots. Admission remains immutable; outstanding R-C protects cleanup.
fn publication(
    f: &mut Fixture,
    r: &Rollover,
    payload: &Payload,
    source: &Store,
) -> Result<Rollover> {
    let plan = rebuild_finalizer(f, r, payload)?;
    verify_claim_range(r, payload, &plan.recipe)?;
    verify_finalizer_bytes(f, r, &plan)?;
    let (ledger, consumed) = retirement_ledger::rollover_transfer(
        f,
        &r.admission.base,
        &plan.recipe.claims,
        &plan.recipe.continuation,
        source,
    )?;
    let consumed = BTreeMap::from([(0, consumed)]);
    let mut next = r.clone();
    next.phase = Phase::Published {
        generations: r.generations()?.to_vec(),
        recipe: Box::new(plan.recipe),
        consumed,
        ledger: Box::new(ledger),
    };
    next.remaining(0)?;
    Ok(next)
}

// Shared validation deliberately allows only the next original phase. Neither
// changing the original budget nor lowering an arbitrary remaining number is a
// supported operation. Exact selected HEAD construction validates the transfer.
fn transition(old: &Rollover, new: &Rollover) -> Result<()> {
    ensure(
        old.admission == new.admission,
        "rollover immutable admission changed",
    )?;
    match (&old.phase, &new.phase) {
        (Phase::Birth, Phase::Payload { generations }) => {
            let expected = (old.admission.corridor.data_pack - old.admission.base.data_pack + 1)
                + (old.admission.corridor.meta_pack - old.admission.base.meta_pack + 1);
            ensure(
                generations.len() as u64 == expected,
                "rollover witness count",
            )?;
            let mut keys = HashSet::new();
            for g in generations {
                let (first, last) = if g.data {
                    (
                        old.admission.base.data_pack,
                        old.admission.corridor.data_pack,
                    )
                } else {
                    (
                        old.admission.base.meta_pack,
                        old.admission.corridor.meta_pack,
                    )
                };
                ensure(
                    (first..=last).contains(&g.pack) && keys.insert((g.data, g.pack)),
                    "rollover witness slot mismatch",
                )?;
            }
        }
        (Phase::Payload { generations: a }, Phase::Finalization { generations: b, .. }) => {
            ensure(a == b, "rollover witnessed generations changed")?;
        }
        (
            Phase::Finalization {
                generations: a,
                recipe: x,
            },
            Phase::Published {
                generations: b,
                recipe: y,
                ..
            },
        ) => {
            ensure(a == b && x == y, "rollover original finalizer changed")?;
            new.remaining(0)?;
        }
        _ => return Err("rollover phase rewind or skip".into()),
    }
    Ok(())
}

impl Rollover {
    pub(in super::super) fn initial_admission_sha(&self) -> Result<String> {
        Ok(hash(&serde_json::to_vec(&self.admission).map_err(error)?))
    }
    pub(in super::super) fn admitted(&self) -> &BTreeMap<u64, u64> {
        &self.admission.admitted
    }
    pub(in super::super) fn control(&self) -> u64 {
        self.admission.control
    }
    pub(in super::super) fn target(&self) -> &Head {
        &self.admission.target
    }
    pub(in super::super) fn base(&self) -> &Selection {
        &self.admission.base
    }
    pub(in super::super) fn origin(&self) -> Continuation {
        Continuation::from_selection(self.base())
    }
    pub(in super::super) fn initial(&self) -> Self {
        Self {
            admission: self.admission.clone(),
            phase: Phase::Birth,
        }
    }
    pub(in super::super) fn is_birth(&self) -> bool {
        matches!(self.phase, Phase::Birth)
    }
    pub(in super::super) fn generation_counts(&self) -> Result<(u64, u64)> {
        let data = self
            .admission
            .corridor
            .data_pack
            .checked_sub(self.base().data_pack)
            .ok_or("rollover data rewind")?;
        let meta = self
            .admission
            .corridor
            .meta_pack
            .checked_sub(self.base().meta_pack)
            .ok_or("rollover metadata rewind")?;
        ensure(
            data <= MAX_GENERATIONS && meta <= MAX_GENERATIONS && data + meta > 0,
            "rollover finite fresh slots",
        )?;
        Ok((data, meta))
    }
}
pub(in super::super) fn validate_gate_transition(
    selected: &Selection,
    gate: &GateState,
) -> Result<()> {
    let exact = selected
        .gate
        .holds
        .values()
        .find(|h| h.rollover.is_some())
        .ok_or("original rollover hold")?;
    ensure(
        selected.gate.holds.len() == 1,
        "rollover sole original hold",
    )?;
    let old = exact.rollover.as_ref().ok_or("original rollover")?;
    let mut expected = selected.gate.clone();
    if let Some(next_hold) = gate.holds.get(&exact.token) {
        let next = next_hold.rollover.as_ref().ok_or("rollover disappeared")?;
        let mut unchanged = next_hold.clone();
        unchanged.rollover.clone_from(&exact.rollover);
        ensure(
            unchanged == *exact,
            "rollover changed immutable original liability",
        )?;
        transition(old, next)?;
        expected.holds.insert(exact.token, next_hold.clone());
        if let Phase::Published {
            ledger, consumed, ..
        } = &next.phase
        {
            expected.ledger = Some(ledger.as_ref().clone());
            for (domain, bytes) in consumed {
                let d = expected
                    .domains
                    .get_mut(domain)
                    .ok_or("rollover unknown charge domain")?;
                d.charged = checked(d.charged, *bytes)?;
            }
        }
    } else {
        ensure(
            matches!(old.phase, Phase::Published { .. }),
            "rollover cleanup before publication",
        )?;
        expected.holds.remove(&exact.token);
    }
    ensure(*gate == expected, "rollover unrelated control change")
}

fn verify_finalizer_bytes(f: &mut Fixture, r: &Rollover, plan: &FinalizationPlan) -> Result<()> {
    for (data, base, writes) in [
        (true, &plan.recipe.data_base, &plan.data),
        (false, &plan.recipe.meta_base, &plan.meta),
    ] {
        for (pack, start, expected) in recipe::segments(base, writes, data)? {
            let mut file = r.generation(data, pack)?.open(f)?;
            let m = file.metadata().map_err(error)?;
            ensure(
                m.len() == start + expected.len() as u64,
                "rollover finalizer suffix incomplete",
            )?;
            let mut bytes = vec![0; usize::try_from(m.len()).map_err(error)?];
            file.read_exact(&mut bytes).map_err(error)?;
            let n = usize::try_from(start).map_err(error)?;
            ensure(
                hash(&bytes[..n]) == base.sha && bytes[n..] == expected,
                "rollover original finalizer bytes changed",
            )?;
            f.c.recipe_prefix_read_bytes += start;
            f.c.recipe_suffix_read_bytes += expected.len() as u64;
        }
    }
    Ok(())
}

impl Draft {
    pub(in super::super) fn initial(&self) -> Rollover {
        Rollover {
            admission: self.admission.clone(),
            phase: Phase::Birth,
        }
    }
    pub(in super::super) fn payload(self) -> Payload {
        self.payload
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Cut {
    None,
    BindingBeforeHead,
    BindingStagedHead,
    BindingAfterHead,
    ReconcileStagedUnlink,
    PayloadBefore,
    PayloadPartial(usize),
    PayloadAfterData,
    PayloadBarrier,
    FinalizationBeforeHead,
    FinalizationAfterHead,
    FinalizationStagedHead,
    FinalizerPartial(usize),
    FinalizerAfterData,
    FinalizerBarrier,
    PublicationBeforeHead,
    PublicationAfterHead,
    PublicationStagedHead,
    CleanupBeforeHead,
    CleanupAfterHead,
    CleanupStagedHead,
}
fn cut_at(cut: Cut, at: Cut) -> Result<()> {
    ensure(cut != at, "cut in original rollover phase")
}
fn selector_cut(cut: Cut, before: Cut, after: Cut) -> super::super::Cut {
    let staged = matches!(
        (cut, before),
        (Cut::BindingStagedHead, Cut::BindingBeforeHead)
            | (Cut::FinalizationStagedHead, Cut::FinalizationBeforeHead)
            | (Cut::PublicationStagedHead, Cut::PublicationBeforeHead)
            | (Cut::CleanupStagedHead, Cut::CleanupBeforeHead)
    );
    if staged {
        super::super::Cut::StagedHead
    } else if cut == before {
        super::super::Cut::BeforeHead
    } else if cut == after {
        super::super::Cut::AfterHead
    } else {
        super::super::Cut::None
    }
}

fn expected_selector(hold: &Liability) -> Result<Selection> {
    fresh_generation::validate_joint_identity(hold)?;
    let r = hold.rollover.as_ref().ok_or("rollover original state")?;

    let birth = hold.birth.as_ref().ok_or("rollover original birth")?;
    let mut h = r.base().clone();
    h.gate.last_token = hold.token;
    h.gate.holds.insert(hold.token, hold.clone());
    let phase = match &r.phase {
        Phase::Birth => 0,
        Phase::Payload { .. } => 1,
        Phase::Finalization { .. } => 2,
        Phase::Published {
            recipe,
            ledger,
            consumed,
            ..
        } => {
            let c = &recipe.continuation;
            h.semantic = r.target().clone();
            h.active = c.active.clone();
            h.recovery = c.recovery.clone();
            h.inventory = c.inventory.clone();
            h.data_pack = c.data_pack;
            h.data_end = c.data_end;
            h.meta_pack = c.meta_pack;
            h.meta_end = c.meta_end;
            h.gate.ledger = Some(ledger.as_ref().clone());
            for (id, n) in consumed {
                let d = h.gate.domains.get_mut(id).ok_or("rollover domain")?;
                d.charged = checked(d.charged, *n)?;
            }
            3
        }
    };
    h.epoch = checked(
        checked(
            checked(r.base().epoch, 1)?,
            fresh_generation::phase_count(birth)?,
        )?,
        phase,
    )?;
    h.gate.validate()?;
    Ok(h)
}
fn validate_generations(hold: &Liability) -> Result<()> {
    let r = hold.rollover.as_ref().ok_or("rollover original state")?;
    let a = &r.admission;
    let mut expected = vec![
        Generation {
            data: true,
            pack: a.data_base.pack,
            dev: a.data_base.dev,
            ino: a.data_base.ino,
        },
        Generation {
            data: false,
            pack: a.meta_base.pack,
            dev: a.meta_base.dev,
            ino: a.meta_base.ino,
        },
    ];
    expected.extend(
        fresh_generation::promoted_generations(hold.birth.as_ref().ok_or("rollover birth")?)?
            .into_iter()
            .map(|(data, pack, dev, ino)| Generation {
                data,
                pack,
                dev,
                ino,
            }),
    );
    expected.sort_by_key(|g| (!g.data, g.pack));
    ensure(
        r.generations()? == expected,
        "rollover witnessed list differs from original births",
    )?;
    Ok(())
}
fn bind_payload(hold: &Liability) -> Result<Liability> {
    let r = hold.rollover.as_ref().ok_or("rollover admission")?;
    ensure(
        r.is_birth(),
        "rollover payload transition requires completed birth",
    )?;
    let a = &r.admission;
    let mut generations = vec![
        Generation {
            data: true,
            pack: a.data_base.pack,
            dev: a.data_base.dev,
            ino: a.data_base.ino,
        },
        Generation {
            data: false,
            pack: a.meta_base.pack,
            dev: a.meta_base.dev,
            ino: a.meta_base.ino,
        },
    ];
    generations.extend(
        fresh_generation::promoted_generations(hold.birth.as_ref().ok_or("rollover birth")?)?
            .into_iter()
            .map(|(data, pack, dev, ino)| Generation {
                data,
                pack,
                dev,
                ino,
            }),
    );
    generations.sort_by_key(|g| (!g.data, g.pack));
    let mut next = r.clone();
    next.phase = Phase::Payload { generations };
    transition(r, &next)?;
    let mut bound = hold.clone();
    bound.rollover = Some(next);
    Ok(bound)
}
fn select_phase(
    f: &mut Fixture,
    old: &Liability,
    next: &Liability,
    cut: super::super::Cut,
) -> Result<()> {
    let candidate = expected_selector(next)?;
    ensure(
        f.head.gate.holds.get(&old.token) == Some(old),
        "rollover exact selected old liability",
    )?;
    fresh_generation::admit_gate_change(&f.head, &candidate.gate)?;
    ensure(
        candidate.epoch == checked(f.head.epoch, 1)?,
        "rollover phase epoch",
    )?;
    f.publish_exact(candidate, cut)
}

fn verify_claim_range(r: &Rollover, payload: &Payload, recipe: &Finalizer) -> Result<()> {
    let expected = claim_coordinates(&r.admission, payload)?;
    ensure(
        recipe.claims.len() == expected.len(),
        "rollover attribution count",
    )?;
    for (claim, (data, pack, content_end, sealed)) in recipe.claims.iter().zip(expected) {
        let g = r.generation(data, pack)?;
        let final_end = if data {
            recipe.continuation.data_end
        } else {
            recipe.continuation.meta_end
        };
        ensure(
            claim.data == data
                && claim.pack == pack
                && claim.dev == g.dev
                && claim.ino == g.ino
                && claim.content_end == content_end
                && claim.sealed == sealed
                && claim.extent == if sealed { content_end } else { final_end },
            "rollover allocation differs from contiguous admitted range",
        )?;
    }
    Ok(())
}

fn validate_observed_finalizer(f: &mut Fixture, r: &Rollover, payload: &Payload) -> Result<()> {
    let plan = rebuild_finalizer(f, r, payload)?;
    verify_claim_range(r, payload, &plan.recipe)?;
    for claim in &plan.recipe.claims {
        if claim.sealed {
            let file = r.generation(claim.data, claim.pack)?.open(f)?;
            let m = file.metadata().map_err(error)?;
            let observed = m
                .blocks()
                .checked_mul(512)
                .ok_or("rollover measured charge overflow")?
                .max(m.len());
            ensure(
                claim.observed_charge == observed,
                "rollover sealed observation changed",
            )?;
        } else {
            ensure(
                claim.observed_charge == claim.content_end,
                "rollover growable prefix observation",
            )?;
        }
    }
    Ok(())
}

/// Recover phase dispatch only after re-deriving its exact committed plan. This
/// does not accept a token/roots match while unrelated ledger fields differ.
pub(in super::super) fn reconcile(
    f: &mut Fixture,
    source: &Store,
    payload: &Payload,
    cut: Cut,
) -> Result<()> {
    ensure(
        fresh_generation::control(f, "HEAD")?.as_ref() == Some(&f.head),
        "unknown rollover HEAD",
    )?;
    if fresh_generation::control(f, "intent")?.is_none() {
        ensure(
            fresh_generation::control(f, "candidate")?.is_none()
                && fresh_generation::control(f, "HEAD.next")?.is_none(),
            "unknown orphan rollover control",
        )?;
        validate_selected(f, source, payload)?;
        return Ok(());
    }
    let old = fresh_generation::control(f, "intent")?.ok_or("rollover intent")?;
    let candidate = fresh_generation::control(f, "candidate")?.ok_or("rollover candidate")?;
    let old_hold = old
        .gate
        .holds
        .values()
        .find(|h| h.rollover.is_some())
        .ok_or("rollover old intent")?;
    let old_r = old_hold.rollover.as_ref().ok_or("rollover old state")?;
    if old_r.is_birth()
        && candidate
            .gate
            .holds
            .get(&old_hold.token)
            .and_then(|h| h.rollover.as_ref())
            .is_some_and(Rollover::is_birth)
    {
        return Ok(());
    }
    ensure(
        old == expected_selector(old_hold)?,
        "rollover unknown original phase selector",
    )?;
    if !old_r.is_birth() {
        validate_generations(old_hold)?;
    }
    let disk = fresh_generation::control(f, "HEAD")?.ok_or("rollover HEAD")?;
    ensure(
        disk == f.head && (disk == old || disk == candidate),
        "rollover unknown selected phase",
    )?;
    if let Some(next_hold) = candidate.gate.holds.get(&old_hold.token) {
        ensure(
            candidate == expected_selector(next_hold)?,
            "rollover unknown phase candidate",
        )?;
        validate_gate_transition(&old, &candidate.gate)?;
        validate_generations(next_hold)?;
        let r = next_hold.rollover.as_ref().ok_or("rollover candidate")?;
        for generation in r.generations()? {
            generation.open(f)?;
        }
        if !matches!(r.phase, Phase::Payload { .. }) {
            validate_observed_finalizer(f, r, payload)?;
        }
        if matches!(r.phase, Phase::Published { .. }) {
            let derived = publication(f, old_r, payload, source)?;
            ensure(derived == *r, "rollover atomic charge transfer changed")?;
        }
    } else {
        ensure(
            matches!(old_r.phase, Phase::Published { .. }),
            "rollover cleanup before publication",
        )?;
        let mut done = old.clone();
        done.gate.holds.remove(&old_hold.token);
        done.epoch = checked(done.epoch, 1)?;
        ensure(candidate == done, "rollover unknown cleanup selector")?;
        validate_observed_finalizer(f, old_r, payload)?;
        let plan = rebuild_finalizer(f, old_r, payload)?;
        verify_finalizer_bytes(f, old_r, &plan)?;
    }
    if f.dir.join("HEAD.next").exists() {
        let next = fresh_generation::control(f, "HEAD.next")?.ok_or("rollover staged HEAD")?;
        ensure(next == candidate, "rollover unknown staged selector")?;
        fs::remove_file(f.dir.join("HEAD.next")).map_err(error)?;
        f.c.files_deleted += 1;
        sync_dir(&f.dir, &mut f.c)?;
        cut_at(cut, Cut::ReconcileStagedUnlink)?;
    }
    f.reconcile()?;
    Ok(())
}

/// Journal/source barriers are performed by the Graph dispatcher before this
/// routine. Every durable physical effect remains covered by the original hold.
#[allow(
    clippy::too_many_lines,
    reason = "Keep original-token phase effect and barrier order together"
)]
pub(in super::super) fn advance(
    f: &mut Fixture,
    source: &Store,
    payload: &Payload,
    cut: Cut,
) -> Result<()> {
    reconcile(f, source, payload, cut)?;
    let mut hold = f
        .head
        .gate
        .holds
        .values()
        .next()
        .cloned()
        .ok_or("rollover original hold")?;
    if hold.rollover.as_ref().ok_or("rollover state")?.is_birth() {
        hold = fresh_generation::resume_joint(f, &hold)?;
        let next = bind_payload(&hold)?;
        select_phase(
            f,
            &hold,
            &next,
            selector_cut(cut, Cut::BindingBeforeHead, Cut::BindingAfterHead),
        )?;
        hold = f.head.gate.holds[&hold.token].clone();
    }
    validate_generations(&hold)?;
    ensure(
        f.head == expected_selector(&hold)?,
        "rollover current selector",
    )?;
    let mut r = hold.rollover.as_ref().ok_or("rollover state")?.clone();
    ensure(
        payload.commitment()? == r.admission.payload,
        "rollover original payload changed",
    )?;
    if let Phase::Published {
        generations,
        recipe,
        ..
    } = &r.phase
    {
        let mut untransferred = r.clone();
        untransferred.phase = Phase::Finalization {
            generations: generations.clone(),
            recipe: recipe.clone(),
        };
        ensure(
            publication(f, &untransferred, payload, source)? == r,
            "selected rollover charge transfer changed",
        )?;
    }
    if matches!(r.phase, Phase::Payload { .. }) {
        cut_at(cut, Cut::PayloadBefore)?;
        let mut left = if let Cut::PayloadPartial(n) = cut {
            Some(n)
        } else {
            None
        };
        replay(
            f,
            &r,
            true,
            &r.admission.data_base,
            &payload.data,
            &mut left,
        )?;
        cut_at(cut, Cut::PayloadAfterData)?;
        replay(
            f,
            &r,
            false,
            &r.admission.meta_base,
            &payload.meta,
            &mut left,
        )?;
        cut_at(cut, Cut::PayloadBarrier)?;
        let (next, _) = bind_finalization(f, &r, payload)?;
        let mut next_hold = hold.clone();
        next_hold.rollover = Some(next);
        select_phase(
            f,
            &hold,
            &next_hold,
            selector_cut(cut, Cut::FinalizationBeforeHead, Cut::FinalizationAfterHead),
        )?;
        hold = f.head.gate.holds[&hold.token].clone();
        r = hold.rollover.as_ref().unwrap().clone();
    }
    if matches!(r.phase, Phase::Finalization { .. }) {
        validate_observed_finalizer(f, &r, payload)?;
        let plan = rebuild_finalizer(f, &r, payload)?;
        let mut left = if let Cut::FinalizerPartial(n) = cut {
            Some(n)
        } else {
            None
        };
        replay(f, &r, true, &plan.recipe.data_base, &plan.data, &mut left)?;
        cut_at(cut, Cut::FinalizerAfterData)?;
        replay(f, &r, false, &plan.recipe.meta_base, &plan.meta, &mut left)?;
        cut_at(cut, Cut::FinalizerBarrier)?;
        let next = publication(f, &r, payload, source)?;
        let mut next_hold = hold.clone();
        next_hold.rollover = Some(next);
        let c = &plan.recipe.continuation;
        f.data = Fixture::reopen_arena(&f.dir, "data", c.data_pack, c.data_end)?;
        f.meta = Fixture::reopen_arena(&f.dir, "meta", c.meta_pack, c.meta_end)?;
        select_phase(
            f,
            &hold,
            &next_hold,
            selector_cut(cut, Cut::PublicationBeforeHead, Cut::PublicationAfterHead),
        )?;
    }
    Ok(())
}

pub(in super::super) fn cleanup(f: &mut Fixture, cut: Cut) -> Result<()> {
    let hold = f
        .head
        .gate
        .holds
        .values()
        .next()
        .cloned()
        .ok_or("rollover cleanup hold")?;
    let r = hold.rollover.as_ref().ok_or("rollover cleanup state")?;
    ensure(
        matches!(r.phase, Phase::Published { .. }) && f.head == expected_selector(&hold)?,
        "rollover cleanup requires exact atomic publication",
    )?;
    let mut next = f.head.clone();
    next.gate.holds.remove(&hold.token);
    next.epoch = checked(next.epoch, 1)?;
    fresh_generation::admit_gate_change(&f.head, &next.gate)?;
    f.publish_exact(
        next,
        selector_cut(cut, Cut::CleanupBeforeHead, Cut::CleanupAfterHead),
    )
}

fn widen(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Number(n) if n.is_u64() => *v = json!(u64::MAX),
        serde_json::Value::Array(items) => {
            for x in items {
                widen(x);
            }
        }
        serde_json::Value::Object(fields) => {
            for x in fields.values_mut() {
                widen(x);
            }
        }
        _ => {}
    }
}

/// NO-EFFECT upper envelope proof. The largest phase has every enrolled witness,
/// all changed allocation claims, final continuation, ledger and consumed map.
/// All numeric values are widened only in a transient serialization tree; this
/// tree cannot be deserialized into a hold or used as refundable attribution.
#[allow(
    clippy::too_many_lines,
    reason = "Keep maximum phase envelope construction visible as one proof"
)]
pub(in super::super) fn preflight_controls(f: &Fixture, hold: &Liability) -> Result<()> {
    let r = hold
        .rollover
        .as_ref()
        .ok_or("rollover envelope admission")?;
    r.generation_counts()?;
    ensure(
        r.admitted() == &hold.by_domain && r.remaining(0)? >= CLEANUP_RESERVE,
        "rollover original envelope budget",
    )?;
    let a = &r.admission;
    let physical = PRef {
        pack: u64::MAX,
        offset: u64::MAX,
        len: u64::MAX,
        sha: hash(b"sizing"),
    };
    let logical = Ref {
        offset: u64::MAX,
        len: u64::MAX,
        sha: hash(b"sizing"),
    };
    let mut claims = vec![];
    let mut generations = vec![];
    for (data, first, last) in [
        (true, a.base.data_pack, a.corridor.data_pack),
        (false, a.base.meta_pack, a.corridor.meta_pack),
    ] {
        for pack in first..=last {
            let sealed = pack < last;
            generations.push(Generation {
                data,
                pack,
                dev: u64::MAX,
                ino: u64::MAX,
            });
            claims.push(Claim {
                data,
                pack,
                dev: u64::MAX,
                ino: u64::MAX,
                extent: PACK,
                content_end: PACK,
                sha: hash(b"sizing"),
                sealed,
                observed_charge: u64::MAX,
                charge: sealed.then(|| {
                    retirement_ledger::AllocationCharge::sizing(
                        data,
                        pack,
                        u64::MAX,
                        u64::MAX,
                        PACK,
                    )
                }),
            });
        }
    }
    let prefix = recipe::Prefix {
        pack: u64::MAX,
        end: PACK,
        dev: u64::MAX,
        ino: u64::MAX,
        observed_charge: u64::MAX,
        sha: hash(b"sizing"),
    };
    let finalizer = Finalizer {
        claims,
        data_base: prefix.clone(),
        meta_base: prefix,
        data_sha: hash(b"sizing"),
        meta_sha: hash(b"sizing"),
        continuation: Continuation {
            recipe: None,
            inventory: Inventory {
                active: Some(logical.clone()),
                recovery: Some(logical),
                next: u64::MAX,
            },
            active: physical.clone(),
            recovery: physical,
            data_pack: u64::MAX,
            data_end: PACK,
            meta_pack: u64::MAX,
            meta_end: PACK,
        },
    };
    let mut maximum = r.clone();
    maximum.phase = Phase::Published {
        generations,
        recipe: Box::new(finalizer),
        consumed: BTreeMap::from([(0, u64::MAX)]),
        ledger: Box::new(a.base.gate.ledger.clone().ok_or("rollover ledger")?),
    };
    let mut h = f.head.clone();
    let mut largest = hold.clone();
    largest.rollover = Some(maximum);
    h.gate.holds.insert(hold.token, largest);
    let mut value = serde_json::to_value(h).map_err(error)?;
    widen(&mut value);
    let bytes = serde_json::to_vec(&value).map_err(error)?.len();
    // Standalone birth proves its entire encoding <=64KiB. Add that entire
    // amount on top of the current birth representation, plus phase tag slack.
    ensure(
        bytes
            .checked_add(64 * 1024 + 256)
            .is_some_and(|n| n <= GATE_MAX),
        "rollover maximum phase selector exceeds standing control frame",
    )
}

#[cfg(test)]
pub(in super::super) fn assert_sealed_owned(f: &mut Fixture, data: bool, pack: u64) -> Result<()> {
    let claim = f
        .ownership_lookup(&f.head.active.clone(), pack * 2 + u64::from(data))?
        .ok_or("sealed original tip")?;
    ensure(
        claim.sealed && claim.charge.is_some(),
        "sealed original tip not attributed",
    )
}

fn validate_selected(f: &mut Fixture, source: &Store, payload: &Payload) -> Result<()> {
    let Some(hold) = f
        .head
        .gate
        .holds
        .values()
        .find(|h| h.rollover.is_some())
        .cloned()
    else {
        return Ok(());
    };
    fresh_generation::validate_joint_identity(&hold)?;
    let r = hold.rollover.as_ref().ok_or("rollover current phase")?;
    if r.is_birth() {
        return Ok(());
    }
    ensure(
        f.head == expected_selector(&hold)?,
        "unknown selected rollover phase before effects",
    )?;
    validate_generations(&hold)?;
    for generation in r.generations()? {
        generation.open(f)?;
    }
    if !matches!(r.phase, Phase::Payload { .. }) {
        validate_observed_finalizer(f, r, payload)?;
    }
    if let Phase::Published {
        generations,
        recipe,
        ..
    } = &r.phase
    {
        let mut untransferred = r.clone();
        untransferred.phase = Phase::Finalization {
            generations: generations.clone(),
            recipe: recipe.clone(),
        };
        ensure(
            publication(f, &untransferred, payload, source)? == *r,
            "selected rollover observed charge transfer changed",
        )?;
    }
    Ok(())
}
