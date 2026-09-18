//! Actual-v3 observation and no-effect ownership-placement comparison.
//! Deliberately not a durable refundable ledger or adopted reserved-key schema.
#[allow(
    clippy::wildcard_imports,
    reason = "Disposable child exercises actual v3 internals"
)]
use super::*;
use std::os::unix::fs::MetadataExt;

const OWNER_KEY: u64 = 1 << 63;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Owner {
    data: bool,
    pack: u64,
    dev: u64,
    ino: u64,
    extent: u64,
    charge: u64,
    sealed: bool,
}
impl Owner {
    fn observe(f: &Fixture, data: bool, pack: u64) -> Result<Self> {
        let arena = if data { &f.data } else { &f.meta };
        let m = fs::symlink_metadata(arena.path(pack)).map_err(error)?;
        ensure(
            m.is_file() && m.nlink() == 1 && m.len() <= PACK,
            "exact private pack",
        )?;
        Ok(Self {
            data,
            pack,
            dev: m.dev(),
            ino: m.ino(),
            extent: m.len(),
            charge: m
                .blocks()
                .checked_mul(512)
                .ok_or("allocation overflow")?
                .max(m.len()),
            sealed: pack < arena.pack,
        })
    }
    fn key(&self) -> Result<u64> {
        let ordinal = self
            .pack
            .checked_mul(2)
            .and_then(|v| v.checked_add(u64::from(self.data)))
            .ok_or("ownership key overflow")?;
        ensure(ordinal < OWNER_KEY, "reserved-key model capacity")?;
        Ok(OWNER_KEY | ordinal)
    }
}
struct Observed {
    units: BTreeMap<(bool, u64), Owner>,
}
impl Observed {
    // Bootstrap/audit only. Ordinary steps below touch the bounded planned range.
    fn bootstrap(f: &Fixture) -> Result<Self> {
        let mut units = BTreeMap::new();
        for (data, end) in [(true, f.data.pack), (false, f.meta.pack)] {
            for pack in 0..=end {
                let owner = Owner::observe(f, data, pack)?;
                units.insert((data, pack), owner);
            }
        }
        Ok(Self { units })
    }
    fn charge(&self, fixed: bool) -> u64 {
        self.units
            .values()
            .map(|o| if fixed { PACK } else { o.charge })
            .sum()
    }
    fn update(&mut self, f: &Fixture, old: &Selection) -> Result<Vec<Owner>> {
        let mut changed = vec![];
        for (data, start, end) in [
            (true, old.data_pack, f.data.pack),
            (false, old.meta_pack, f.meta.pack),
        ] {
            ensure(
                end >= start && end - start <= 8194,
                "bounded observed update",
            )?;
            for pack in start..=end {
                let mut owner = Owner::observe(f, data, pack)?;
                if let Some(prior) = self.units.get(&(data, pack)) {
                    ensure(
                        owner.dev == prior.dev
                            && owner.ino == prior.ino
                            && owner.extent >= prior.extent,
                        "prefix generation changed",
                    )?;
                    owner.charge = owner.charge.max(prior.charge);
                    if owner == *prior {
                        continue;
                    }
                }
                self.units.insert((data, pack), owner.clone());
                changed.push(owner);
            }
        }
        Ok(changed)
    }
}

// This uses real v3 locator insertion and encoding, without writing plans.
// It deliberately returns the feedback obligations rather than pretending that
// one descriptor insertion has accounted for the packs created by that insertion.
struct PlacementPlan {
    data: WritePlan,
    meta: WritePlan,
    active: PRef,
    recovery: PRef,
}
impl PlacementPlan {
    fn continuation(&self) -> Continuation {
        Continuation {
            recipe: None,
            inventory: Inventory::default(),
            active: self.active.clone(),
            recovery: self.recovery.clone(),
            data_pack: self.data.pack,
            data_end: self.data.end,
            meta_pack: self.meta.pack,
            meta_end: self.meta.end,
        }
    }
}
fn placement_plan(f: &mut Fixture, owners: &[Owner], fixed: bool) -> Result<PlacementPlan> {
    let original_head = f.head.clone();
    let mut data = WritePlan::from(&f.data);
    let mut updates = Vec::new();
    for owner in owners {
        let mut descriptor = owner.clone();
        if fixed {
            descriptor.charge = PACK;
        }
        let bytes = serde_json::to_vec(&descriptor).map_err(error)?;
        let k = descriptor.key()?;
        ensure(
            f.lookup(&original_head.active, k)?.is_none(),
            "reserved model key collides",
        )?;
        let physical = data.push(bytes.clone(), Some(k))?;
        updates.push((
            k,
            Some(Location {
                membership: Membership::Semantic,
                object: Ref {
                    offset: k,
                    len: bytes.len() as u64,
                    sha: hash(&bytes),
                },
                data: physical,
            }),
        ));
    }
    updates.sort_by_key(|(k, _)| *k);
    f.staged = Some(Vec::new());
    let root = f
        .multi(Some(original_head.active.clone()), &updates)?
        .ok_or("empty ownership preview")?;
    let recovery = f
        .multi(Some(original_head.recovery.clone()), &updates)?
        .ok_or("empty recovery ownership preview")?;
    let pages = f.staged.take().ok_or("ownership pages")?;
    let mut meta = WritePlan::from(&f.meta);
    let mut memo = HashMap::new();
    let active = meta.staged(&root, &pages, &mut memo)?;
    let recovery = meta.staged(&recovery, &pages, &mut memo)?;
    ensure(f.head == original_head, "preview changed selection")?;
    Ok(PlacementPlan {
        data,
        meta,
        active,
        recovery,
    })
}
fn preview(f: &mut Fixture, owners: &[Owner], fixed: bool) -> Result<serde_json::Value> {
    let PlacementPlan { data, meta, .. } = placement_plan(f, owners, fixed)?;
    Ok(
        json!({"descriptors":owners.len(),"data_bytes":data.bytes,"metadata_bytes":meta.bytes,"metadata_pages":meta.writes.len(),"data_packs_closed_by_descriptor_publication":data.pack-f.data.pack,"metadata_packs_closed_by_descriptor_publication":meta.pack-f.meta.pack,"new_active_data_extent":data.end,"new_active_metadata_extent":meta.end,"physical_bound":physical_bound(Some(&data),&meta)?,"writes_performed":0,"feedback_not_resolved":true}),
    )
}

// Bounded layout fixed point. Existing packs can become sealed while storing
// their descriptors. Their final extent must then be described, and both new
// locator roots must be in the original continuation. A newly created AND sealed
// pack has no pre-effect inode witness: refuse rather than invent one.
fn closure_plan(
    f: &mut Fixture,
    seed: &[Owner],
    max_rounds: usize,
) -> Result<(PlacementPlan, Vec<Owner>, usize)> {
    let mut owners = seed
        .iter()
        .cloned()
        .map(|o| ((o.data, o.pack), o))
        .collect::<BTreeMap<_, _>>();
    for round in 1..=max_rounds.min(8) {
        ensure(owners.len() <= 64, "ownership closure descriptor cap")?;
        let plan = placement_plan(f, &owners.values().cloned().collect::<Vec<_>>(), false)?;
        let mut next = owners.clone();
        for (data, arena, write) in [(true, &f.data, &plan.data), (false, &f.meta, &plan.meta)] {
            ensure(
                write.pack <= arena.pack + 1,
                "newly-created sealed pack has no pre-effect ownership witness",
            )?;
            for pack in arena.pack..write.pack {
                let mut owner = Owner::observe(f, data, pack)?;
                owner.extent = write
                    .writes
                    .iter()
                    .filter(|(_, r, _)| r.pack == pack)
                    .map(|(_, r, _)| r.offset + r.len)
                    .max()
                    .unwrap_or(owner.extent);
                // Full pack remains reserved/charged, not an observed-allocation claim.
                owner.charge = PACK;
                owner.sealed = true;
                next.insert((data, pack), owner);
            }
        }
        if owners == next {
            return Ok((plan, owners.into_values().collect(), round));
        }
        owners = next;
    }
    Err("ownership closure did not converge within bounded no-effect planning".into())
}
fn closure_report(f: &mut Fixture, owners: &[Owner]) -> Result<serde_json::Value> {
    let (plan, closed, rounds) = closure_plan(f, owners, 8)?;
    let reserve = physical_bound(Some(&plan.data), &plan.meta)?;
    Ok(
        json!({"rounds":rounds,"descriptors":closed.len(),"data_bytes":plan.data.bytes,"metadata_bytes":plan.meta.bytes,"metadata_pages":plan.meta.writes.len(),"physical_plan":reserve,"original_continuation":plan.continuation(),"writes_performed":0,"sealed_charge_uses_pack_cap":true,"active_tip_ownership_not_integrated":true}),
    )
}

fn run(n: u64, kind: Kind) -> Result<serde_json::Value> {
    let begin = Instant::now();
    let (_temp, mut source, mut f) = setup(n, kind)?;
    let mut observed = Observed::bootstrap(&f)?;
    let baseline = json!({"units":observed.units.len(),"incremental_project_charge":observed.charge(false),"fixed_capacity_project_charge":observed.charge(true),"bootstrap_ms":begin.elapsed().as_millis()});
    let mut samples = vec![];
    for group in [1, 8, 32] {
        let source_end = source.size();
        logical_group(&mut source, group)?;
        let old = f.head.clone();
        let before_charge = observed.charge(false);
        let before_fixed = observed.charge(true);
        let before_alloc = f.snapshot()?.1;
        f.c = Counters::default();
        let now = Instant::now();
        let actual = f.checkpoint(&mut source, source_end, Fault::None)?;
        let checkpoint_us = now.elapsed().as_micros();
        let checkpoint_counters = json!(f.c);
        let changed = observed.update(&f, &old)?;
        let sealed = changed
            .iter()
            .filter(|o| o.sealed)
            .cloned()
            .collect::<Vec<_>>();
        let all_descriptor_preview = preview(&mut f, &changed, false)?;
        let sealed_descriptor_preview = preview(&mut f, &sealed, true)?;
        let closure = closure_report(&mut f, &sealed)?;
        samples.push(json!({"group":group,"actual_v3_checkpoint_us":checkpoint_us,"actual_v3_counters":checkpoint_counters,"actual_v3_result":actual,"observed_pack_charge_delta":observed.charge(false)-before_charge,"fixed_pack_capacity_delta":observed.charge(true)-before_fixed,"observed_regular_file_delta":i128::from(f.snapshot()?.1)-i128::from(before_alloc),"changed_units":changed.len(),"newly_sealed_units":sealed.len(),"incremental_descriptor_preview":all_descriptor_preview,"fixed_sealed_descriptor_preview":sealed_descriptor_preview,"bounded_sealed_closure":closure}));
    }
    let now = Instant::now();
    let mut reopened = Fixture::reopen_combined(&f.dir)?;
    reopened.structural_probe()?;
    let open = json!({"us":now.elapsed().as_micros(),"counters":reopened.c});
    reopened.c = Counters::default();
    let now = Instant::now();
    reopened.retry(&id(1), &request(&id(1)))?;
    let retry = json!({"us":now.elapsed().as_micros(),"counters":reopened.c});
    Ok(
        json!({"n":n,"kind":format!("{kind:?}"),"actual_v3":true,"durable_refundable_ownership_integrated":false,"baseline":baseline,"samples":samples,"structural_open":open,"old_retry":retry,"incremental_charge":observed.charge(false),"fixed_capacity_charge":observed.charge(true),"legacy_v3_high_water_charge":f.head.gate.domains[&0].charged,"ownership_units":observed.units.len(),"elapsed_ms":begin.elapsed().as_millis()}),
    )
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    for count in if args.is_empty() {
        vec![1000]
    } else {
        args.iter()
            .map(|s| s.parse::<u64>().map_err(error))
            .collect::<Result<Vec<_>>>()?
    } {
        ensure((1..=10000).contains(&count), "bounded probe count")?;
        for kind in [Kind::Radix, Kind::Btree] {
            println!("{}", run(count, kind)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_preview_is_no_effect_and_counts_real_locator_pages() -> Result<()> {
        let (_temp, _source, mut f) = setup(32, Kind::Radix)?;
        let owners = [
            Owner::observe(&f, true, f.data.pack)?,
            Owner::observe(&f, false, f.meta.pack)?,
        ];
        let head = f.head.clone();
        let snapshot = f.snapshot()?;
        let p = preview(&mut f, &owners, false)?;
        ensure(
            p["metadata_pages"].as_u64().unwrap() > 0,
            "real locator path must change",
        )?;
        ensure(
            f.head == head && f.snapshot()? == snapshot,
            "no-effect preview",
        )
    }
    #[test]
    fn descriptor_insertion_can_close_an_unaccounted_metadata_pack() -> Result<()> {
        let (_temp, _source, mut f) = setup(32, Kind::Radix)?;
        // No physical mutation: a planning tail near the pack boundary exposes
        // the ownership self-allocation dependency deterministically.
        let owner = Owner::observe(&f, true, f.data.pack)?;
        let actual = f.meta.end;
        f.meta.end = PACK - 1;
        let p = preview(&mut f, &[owner], false)?;
        f.meta.end = actual;
        ensure(
            p["metadata_packs_closed_by_descriptor_publication"]
                .as_u64()
                .unwrap()
                > 0,
            "metadata feedback must remain an explicit obligation",
        )
    }
    #[test]
    fn observed_prefix_charge_is_idempotent_but_not_durable_authority() -> Result<()> {
        let (_temp, mut source, mut f) = setup(32, Kind::Btree)?;
        let mut observed = Observed::bootstrap(&f)?;
        let old = f.head.clone();
        let end = source.size();
        logical_group(&mut source, 8)?;
        f.checkpoint(&mut source, end, Fault::None)?;
        observed.update(&f, &old)?;
        let charge = observed.charge(false);
        ensure(
            observed.update(&f, &old)?.is_empty() && observed.charge(false) == charge,
            "repeat must not double charge",
        )?;
        ensure(
            f.head.gate.domains[&0].charged > charge,
            "legacy high-water remains separate",
        )
    }
    #[test]
    fn bounded_closure_covers_feedback_and_original_extent_without_writes() -> Result<()> {
        let (_temp, _source, mut f) = setup(32, Kind::Radix)?;
        let owner = Owner::observe(&f, true, f.data.pack)?;
        let actual = f.meta.end;
        let before = f.snapshot()?;
        f.meta.end = PACK - 1;
        let (p, owners, rounds) = closure_plan(&mut f, &[owner], 8)?;
        ensure(
            rounds >= 2 && owners.iter().any(|o| !o.data && o.pack == f.meta.pack),
            "feedback ownership omitted",
        )?;
        ensure(
            p.continuation().meta_pack == f.meta.pack + 1,
            "original continuation misses closure rollover",
        )?;
        ensure(
            physical_bound(Some(&p.data), &p.meta)?["allocation_bound"]
                .as_u64()
                .unwrap()
                >= p.data.bytes + p.meta.bytes,
            "reserve misses closure bytes",
        )?;
        f.meta.end = actual;
        ensure(f.snapshot()? == before, "closure planning wrote files")
    }
    #[test]
    fn bounded_closure_exhaustion_refuses_without_effect() -> Result<()> {
        let (_temp, _source, mut f) = setup(32, Kind::Btree)?;
        let owner = Owner::observe(&f, true, f.data.pack)?;
        let actual = f.meta.end;
        let head = f.head.clone();
        let before = f.snapshot()?;
        f.meta.end = PACK - 1;
        ensure(
            closure_plan(&mut f, &[owner], 1).is_err(),
            "one-round budget must refuse feedback",
        )?;
        f.meta.end = actual;
        ensure(
            f.head == head && f.snapshot()? == before,
            "closure exhaustion had effects",
        )
    }
    #[test]
    fn actual_v3_recovery_refuses_unmodeled_ownership_namespace_and_retains_hold() -> Result<()> {
        for corrupt in [false, true] {
            let (_temp, mut source, mut f) = setup(512, Kind::Radix)?;
            ensure(f.data.pack > 0, "fixture needs sealed pack")?;
            let owner = Owner::observe(&f, true, 0)?;
            let (plan, _, _) = closure_plan(&mut f, &[owner.clone()], 8)?;
            let cont = plan.continuation();
            let physical = physical_bound(Some(&plan.data), &plan.meta)?["allocation_bound"]
                .as_u64()
                .unwrap();
            let hold = f.reserve_with(
                f.head.semantic.clone(),
                None,
                &[(0, physical + 4 * f.control_bound(&f.head.gate)?)],
                Some(cont.clone()),
            )?;
            f.authorized_hold = Some(hold.token);
            let descriptor = plan
                .data
                .writes
                .iter()
                .find(|(key, _, _)| *key == Some(owner.key().unwrap()))
                .unwrap()
                .1
                .clone();
            plan.data.write(&mut f.data, &mut f.c)?;
            plan.meta.write(&mut f.meta, &mut f.c)?;
            ensure(
                f.publish(
                    plan.active,
                    plan.recovery,
                    f.head.semantic.clone(),
                    Cut::BeforeHead,
                )
                .is_err(),
                "cut did not interrupt",
            )?;
            if corrupt {
                let mut file = OpenOptions::new()
                    .write(true)
                    .open(f.data.path(descriptor.pack))
                    .map_err(error)?;
                file.seek(SeekFrom::Start(descriptor.offset))
                    .map_err(error)?;
                file.write_all(b"!").map_err(error)?;
                file.sync_all().map_err(error)?;
            }
            f = Fixture::reopen_combined(&f.dir)?;
            let selected = f.head.clone();
            let refusal = f.resume_checkpoint(&mut source, 0, &hold).unwrap_err();
            ensure(
                refusal == "extraneous or duplicate locator leaf",
                "actual v3 must reject non-semantic ownership leaves",
            )?;
            ensure(
                f.head == selected
                    && f.head.gate.holds.get(&hold.token) == Some(&hold)
                    && f.dir.join("intent").exists(),
                "refusal released original liability or changed selection",
            )?;
        }
        Ok(())
    }
}
