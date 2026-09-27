//! Runtime Graph-born/sealed placement followed by exact typed relocation.
//! The preparatory padding belongs to the parent's pre-admission fixture; the
//! candidate tested here is a different inode born under the real Graph token.
#![cfg(test)]
#[allow(clippy::wildcard_imports, reason = "Actual Graph dispatch test child")]
use super::*;

#[test]
fn turnover_runtime_born_sealed_relocation_preserves_original_graph_receipts() -> Result<()> {
    for kind in [Kind::Radix, Kind::Btree] {
        for data in [true, false] {
            let (_temp, mut source, mut f, journal) = setup_fixture(kind, &[data])?;
            let before = f.head.clone();
            let first = plan(&mut source, 1)?;
            let birth_hold = admit_dedicated(&mut source, &mut f, &journal, &first)?;
            finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
            let born_pack = if data {
                f.head.data_pack
            } else {
                f.head.meta_pack
            };
            ensure(
                born_pack
                    > if data {
                        before.data_pack
                    } else {
                        before.meta_pack
                    },
                "candidate must be born under actual Graph admission",
            )?;
            let group = plan(&mut source, 1)?;
            let held = admit_dedicated(&mut source, &mut f, &journal, &group)?;
            finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
            rollover::assert_sealed_owned(&mut f, data, born_pack)?;
            let records = first
                .records
                .iter()
                .chain(&group.records)
                .cloned()
                .collect::<Vec<_>>();
            let selected = f.head.semantic.clone();
            let originals = records
                .iter()
                .map(|record| original(&mut f, record))
                .collect::<Result<Vec<_>>>()?;
            for class in EXTRA_CLASSES {
                let pin = f.acquire(class, class == PinClass::UnresolvedIntent)?;
                let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
                let snapshot = f.snapshot()?;
                ensure(
                    retirement_ledger::relocation::relocate_owned(&mut f, data, born_pack).is_err(),
                    "runtime born source escaped all-pin ownership",
                )?;
                ensure(
                    f.snapshot()? == snapshot
                        && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                        && f.head.gate.holds.is_empty(),
                    "pin refusal created runtime relocation effects",
                )?;
                f.release(&pin, true, true, true)?;
            }
            let row = retirement_ledger::relocation::relocate_owned(&mut f, data, born_pack)
                .map_err(|e| format!("runtime born {kind:?} data={data} pack={born_pack}: {e}"))?;
            ensure(
                f.head.semantic == selected && source.head()? == selected,
                "physical retirement changed original Graph publication",
            )?;
            for (record, reference) in records.iter().zip(&originals) {
                ensure(
                    original(&mut f, record)? == *reference,
                    "relocation replaced original Graph receipt",
                )?;
            }
            f.audit_combined()?;
            audit_graph(&mut f)?;
            println!(
                "{}",
                json!({"fixture":"runtime-graph-born-sealed-to-live-relocation",
                "kind":format!("{kind:?}"),"graph_birth_token":birth_hold.token,"graph_seal_token":held.token,
                "graph_records":records.len(),"born_pack":born_pack,"source_data":data,
                "all_extra_pin_classes":EXTRA_CLASSES.len(),"runtime_born_source":true,
                "relocation":row,"filesystem_availability_credit":0,"qualified_saved":false})
            );
        }
    }
    Ok(())
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Fixed-budget workload retains every original receipt across bounded groups"
)]
fn turnover_fixed_capacity_original_groups_and_bounded_refusals() -> Result<()> {
    let group_cap = std::env::var("PHOTARA_PS2_TURNOVER_GROUPS")
        .ok()
        .map(|v| v.parse::<usize>().map_err(error))
        .transpose()?
        .unwrap_or(128);
    let records_per_group = std::env::var("PHOTARA_PS2_TURNOVER_RECORDS")
        .ok()
        .map(|v| v.parse::<u64>().map_err(error))
        .transpose()?
        .unwrap_or(8);
    ensure(
        (64..=256).contains(&group_cap) && [8, 32].contains(&records_per_group),
        "bounded meaningful turnover configuration",
    )?;
    for headroom in [256 * 1024, 16 * 1024 * 1024] {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut source, mut f, journal) = setup_fixture(kind, &[true])?;
            let first = plan(&mut source, records_per_group)?;
            let (_, draft, _) = prepare(&mut source, &mut f, &first, None, true)?;
            let first_reserve = draft
                .initial()
                .admitted()
                .get(&0)
                .copied()
                .ok_or("original reserve")?;
            let limit = checked(
                f.head.gate.domains[&0].charged,
                checked(first_reserve, headroom)?,
            )?;
            let mut gate = f.head.gate.clone();
            gate.domains.get_mut(&0).ok_or("capacity domain")?.limit = limit;
            f.select_gate(gate)?;
            let initial_charge = f.head.gate.domains[&0].charged;
            let mut history = vec![];
            let mut samples = vec![];
            let mut refusals = vec![];
            for index in 0..group_cap {
                let group = plan(&mut source, records_per_group)?;
                let before = f.snapshot()?;
                let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
                let source_size = source.size();
                let original_hold = match admit_dedicated(&mut source, &mut f, &journal, &group) {
                    Ok(hold) => hold,
                    Err(e) => {
                        ensure(
                            f.head.gate.holds.is_empty()
                                && f.snapshot()? == before
                                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                                && source.size() == source_size
                                && fs::read_dir(&journal).map_err(error)?.next().is_none(),
                            "bounded Graph refusal created effects",
                        )?;
                        refusals.push(json!({"group":index,"phase":"Graph admission","reason":e}));
                        break;
                    }
                };
                finish(&mut f, &mut source, &journal, rollover::Cut::None)?;
                history.extend(group.records.clone());
                let mut maintenance = vec![];
                // Measure the initial garbage-rich source and periodic runtime
                // sealed tips. This experiment records both positive and negative
                // net outcomes; it does not define an automatic retirement policy.
                if index == 0 || index % 8 == 7 {
                    let before = f.snapshot()?;
                    let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
                    let candidate = if index == 0 {
                        0
                    } else {
                        original_hold.origin.data_pack
                    };
                    match retirement_ledger::relocation::relocate_owned(&mut f, true, candidate) {
                        Ok(row) => maintenance.push(row),
                        Err(e) => {
                            ensure(
                                f.head.gate.holds.is_empty()
                                    && f.snapshot()? == before
                                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
                                "bounded relocation refusal created effects",
                            )?;
                            refusals
                                .push(json!({"group":index,"phase":"live relocation","reason":e}));
                        }
                    }
                }
                for record in &history {
                    original(&mut f, record)?;
                }
                f.audit_combined()?;
                audit_graph(&mut f)?;
                ensure(
                    f.head.gate.domains[&0].limit == limit
                        && f.head.gate.domains[&0].charged <= limit,
                    "turnover changed original fixed capacity",
                )?;
                samples.push(json!({"group":index,"original_token":original_hold.token,
                "cumulative_graph_records":history.len(),"charged":f.head.gate.domains[&0].charged,
                "data_tip":f.head.data_pack,"meta_tip":f.head.meta_pack,"maintenance":maintenance}));
            }
            ensure(
                !samples.is_empty(),
                "fixed-capacity workload made no original progress",
            )?;
            println!(
                "{}",
                json!({"fixture":"runtime-graph-fixed-capacity-bounded-turnover",
            "kind":format!("{kind:?}"),"fixed_project_limit":limit,"initial_charge":initial_charge,
            "first_original_reserve":first_reserve,"predetermined_extra_headroom":headroom,
            "completed_groups":samples.len(),"samples":samples,"refusals":refusals,
            "finite_group_cap":group_cap,"records_per_group":records_per_group,"maintenance_is_product_policy":false,"filesystem_availability_credit":0,
            "lifetime_capacity_plateau_proven":false,"qualified_saved":false})
            );
        }
    }
    Ok(())
}
