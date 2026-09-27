//! Joint same-generation Graph and exact typed-ownership planning.
//! This is a disposable bounded planner; fresh pack generations are not enrolled here.
#[allow(
    clippy::wildcard_imports,
    reason = "Joint planner shares typed fixture primitives"
)]
use super::*;

/// These are captured once, before the original hold. Replay never recaptures
/// a partially grown tip and thereby changes the original ownership commitment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct Original {
    claims: Vec<Claim>,
}
impl Original {
    pub(in super::super) fn capture(f: &mut Fixture) -> Result<Self> {
        ensure(
            f.head.gate.holds.is_empty(),
            "joint Graph original capture before hold",
        )?;
        ensure(
            f.head.gate.ledger.is_some(),
            "joint Graph requires attributable bootstrap",
        )?;
        ensure(
            f.head.inventory.active.is_some() && f.head.inventory.recovery.is_some(),
            "joint Graph requires complete typed bootstrap",
        )?;
        let claims = vec![
            Claim::capture(f, true, f.data.pack)?,
            Claim::capture(f, false, f.meta.pack)?,
        ];
        for claim in &claims {
            let arena = if claim.data { &f.data } else { &f.meta };
            ensure(
                !claim.sealed && claim.charge.is_none() && claim.extent == arena.end,
                "joint Graph growable original generation",
            )?;
            let old = f
                .ownership_lookup(&f.head.active.clone(), claim.key()?)?
                .ok_or("joint Graph original tip ownership absent")?;
            ensure(
                old.dev == claim.dev && old.ino == claim.ino && old.extent == claim.extent,
                "joint Graph original owned tip extent",
            )?;
        }
        Ok(Self { claims })
    }
}

/// Semantic changes remain separate from physical locator materialization so
/// ownership updates can share exactly the same final locator rewrite.
pub(in super::super) struct SemanticDelta {
    pub(in super::super) data: WritePlan,
    pub(in super::super) updates: Vec<(u64, Option<Location>)>,
    pub(in super::super) semantic: Head,
    pub(in super::super) added: usize,
    pub(in super::super) removed: usize,
}
pub(in super::super) fn semantic_delta(
    f: &mut Fixture,
    source: &mut Store,
    old_end: u64,
) -> Result<SemanticDelta> {
    let old = f.head.clone();
    let semantic = source.head()?;
    ensure(
        semantic.recovery == old.semantic.active,
        "logical selected prefix discontinuity",
    )?;
    let (added, removed) = changes(source, &old.semantic.active, &semantic.active, old_end)?;
    let mut updates = BTreeMap::new();
    let mut data = WritePlan::from(&f.data);
    for r in &removed {
        ensure(
            f.lookup(&old.active, r.offset)?
                .is_some_and(|l| l.object == *r),
            "removed object absent",
        )?;
        updates.insert(r.offset, None);
    }
    for r in &added {
        let bytes = source.planned_bytes(r)?;
        let location = Location {
            membership: Membership::Semantic,
            object: r.clone(),
            data: data.push(bytes, Some(r.offset))?,
        };
        updates.insert(r.offset, Some(location));
    }
    Ok(SemanticDelta {
        data,
        updates: updates.into_iter().collect(),
        semantic,
        added: added.len(),
        removed: removed.len(),
    })
}

pub(in super::super) struct JointPlan {
    pub(in super::super) append: PreparedAppend,
    pub(in super::super) continuation: Continuation,
}
fn once(f: &mut Fixture, semantic: &SemanticDelta, claims: &[Claim]) -> Result<JointPlan> {
    let h = f.head.clone();
    ensure(
        h.gate.holds.is_empty(),
        "joint planning uses restored original unheld snapshot",
    )?;
    let mut builder = Builder {
        data: semantic.data.clone(),
        next: h.inventory.next,
        added: vec![],
        removed: vec![],
    };
    let mut ownership = h.inventory.active.clone();
    for claim in claims {
        claim.verify_extent(f, true)?;
        ownership = Some(builder.insert(f, &h.active, ownership, claim)?);
    }
    let ownership = ownership.ok_or("joint ownership root")?;
    let owned_updates = builder.updates();
    ensure(
        semantic.updates.iter().all(|(key, _)| *key < OWNER_KEY)
            && owned_updates.iter().all(|(key, _)| *key >= OWNER_KEY),
        "joint semantic/ownership namespace separation",
    )?;
    let updates = semantic
        .updates
        .iter()
        .chain(&owned_updates)
        .cloned()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect::<Vec<_>>();
    f.staged = Some(Vec::new());
    let active = f
        .multi(Some(h.active.clone()), &updates)?
        .ok_or("joint active locator")?;
    // Graph recovery is the prior ACTIVE semantic state and its inventory.
    // Both new roots own the new physical suffix; only active sees new Graph objects.
    let recovery = f
        .multi(Some(h.active), &owned_updates)?
        .ok_or("joint recovery locator")?;
    let pages = f.staged.take().ok_or("joint staged locator pages")?;
    let mut meta = WritePlan::from(&f.meta);
    let mut memo = HashMap::new();
    let active = meta.staged(&active, &pages, &mut memo)?;
    let recovery = meta.staged(&recovery, &pages, &mut memo)?;
    let continuation = Continuation {
        recipe: None,
        inventory: Inventory {
            active: Some(ownership.clone()),
            recovery: Some(ownership),
            next: builder.next,
        },
        active: active.clone(),
        recovery: recovery.clone(),
        data_pack: builder.data.pack,
        data_end: builder.data.end,
        meta_pack: meta.pack,
        meta_end: meta.end,
    };
    let append = PreparedAppend {
        data: builder.data,
        meta,
        active,
        recovery,
        semantic: semantic.semantic.clone(),
        added: semantic.added,
        removed: semantic.removed,
    };
    Ok(JointPlan {
        append,
        continuation,
    })
}
pub(in super::super) fn plan(
    f: &mut Fixture,
    source: &mut Store,
    old_end: u64,
    original: &Original,
) -> Result<JointPlan> {
    let result = plan_inner(f, source, old_end, original);
    f.staged = None;
    f.cache.clear();
    result
}
fn plan_inner(
    f: &mut Fixture,
    source: &mut Store,
    old_end: u64,
    original: &Original,
) -> Result<JointPlan> {
    ensure(
        original.claims.len() == 2 && original.claims[0].data && !original.claims[1].data,
        "joint original tip claim shape",
    )?;
    let semantic = semantic_delta(f, source, old_end)?;
    let mut claims = original.claims.clone();
    for claim in &claims {
        let arena = if claim.data { &f.data } else { &f.meta };
        ensure(
            claim.pack == arena.pack
                && claim.content_end == arena.end
                && claim.extent == arena.end
                && !claim.sealed
                && claim.charge.is_none(),
            "joint original same-tip coordinates",
        )?;
    }
    for _ in 0..16 {
        let plan = once(f, &semantic, &claims)?;
        ensure(
            plan.append.data.pack == f.data.pack && plan.append.meta.pack == f.meta.pack,
            "joint Graph rollover requires original-token fresh-generation enrollment",
        )?;
        let mut changed = false;
        for claim in &mut claims {
            let end = if claim.data {
                plan.append.data.end
            } else {
                plan.append.meta.end
            };
            if claim.extent != end {
                claim.extent = end;
                changed = true;
            }
        }
        if !changed {
            return Ok(plan);
        }
    }
    Err("joint Graph extent fixed point exceeds bounded planning".into())
}

/// Retained surrogate-source bytes are accounted separately from package tips.
/// This registry never authorizes retirement or returns filesystem availability.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct SourceCharge {
    pub(super) dev: u64,
    pub(super) ino: u64,
    pub(super) extent: u64,
    pub(super) charged_high_water: u64,
    pub(super) standing_scratch: u64,
}
impl SourceCharge {
    const STANDING_SCRATCH: u64 = 4 * PACK + 65536;
    pub(in super::super) fn capture(source: &Store) -> Result<Self> {
        let m = source.file.metadata().map_err(error)?;
        let named = fs::symlink_metadata(source.dir.join("objects.pack")).map_err(error)?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && named.is_file()
                && named.dev() == m.dev()
                && named.ino() == m.ino(),
            "private retained Graph source generation",
        )?;
        Ok(Self {
            dev: m.dev(),
            ino: m.ino(),
            extent: m.len(),
            charged_high_water: m
                .blocks()
                .checked_mul(512)
                .ok_or("source charge overflow")?
                .max(m.len()),
            standing_scratch: Self::STANDING_SCRATCH,
        })
    }
    pub(in super::super) fn charge(&self) -> Result<u64> {
        ensure(
            self.standing_scratch == Self::STANDING_SCRATCH
                && self.charged_high_water >= self.extent,
            "source charge shape",
        )?;
        checked(self.charged_high_water, self.standing_scratch)
    }
    pub(in super::super) fn observe_growth(&self, source: &Store) -> Result<(Self, u64)> {
        self.charge()?;
        let observed = Self::capture(source)?;
        ensure(
            observed.dev == self.dev && observed.ino == self.ino && observed.extent >= self.extent,
            "retained source replaced or truncated",
        )?;
        let mut next = observed;
        next.charged_high_water = next.charged_high_water.max(self.charged_high_water);
        let growth = next.charged_high_water - self.charged_high_water;
        Ok((next, growth))
    }
}

impl Original {
    pub(in super::super) fn verify_selected(
        &self,
        f: &mut Fixture,
        c: &Continuation,
    ) -> Result<()> {
        ensure(
            f.head.active == c.active
                && f.head.recovery == c.recovery
                && f.head.inventory == c.inventory,
            "joint selected continuation/inventory",
        )?;
        for original in &self.claims {
            let mut expected = original.clone();
            expected.extent = if expected.data {
                c.data_end
            } else {
                c.meta_end
            };
            expected.verify(f)?;
            for locator in [f.head.active.clone(), f.head.recovery.clone()] {
                ensure(
                    f.ownership_lookup(&locator, expected.key()?)?.as_ref() == Some(&expected),
                    "joint selected tip ownership differs from original plan",
                )?;
            }
        }
        Ok(())
    }
}
