//! Bounded prefix-accounting adapter; v3-compatible framing, not a v3 writer.
#[allow(
    clippy::wildcard_imports,
    reason = "Disposable child shares the forked ownership model"
)]
use super::*;
use std::io::{Read, Seek, SeekFrom};
const PACK: u64 = 256 * 1024;
const CHUNK: usize = 8192;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) enum Role {
    Data,
    Locator,
    Control,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Prefix {
    bytes: u64,
    sha: String,
}
impl Prefix {
    pub(super) fn of(u: &Unit) -> Self {
        Self {
            bytes: u.bytes,
            sha: u.sha.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Growth {
    token: u64,
    pub(super) original: Unit,
    suffix: Vec<u8>,
    target: Prefix,
    pub(super) bound: u64,
}
impl Ledger {
    pub(super) fn validate_prefixes(&self) -> Result<()> {
        ensure(self.growths.len() <= 3, "bounded multi-arena growth")?;
        let mut units = BTreeSet::new();
        for (&token, g) in &self.growths {
            ensure(
                token == g.token
                    && token < self.next
                    && units.insert(g.original.id)
                    && self.units.get(&g.original.id) == Some(&g.original)
                    && !g.original.sealed
                    && !g.original.retiring
                    && g.target.bytes == add(g.original.bytes, g.suffix.len() as u64)?
                    && g.target.bytes <= PACK
                    && g.suffix.len() <= CHUNK,
                "growth identity/extent",
            )?;
        }
        for (ids, prefixes) in [
            (&self.active, &self.active_prefixes),
            (&self.recovery, &self.recovery_prefixes),
        ] {
            ensure(
                ids == &prefixes.keys().copied().collect(),
                "selected prefix membership",
            )?;
            self.check_snapshot(prefixes)?;
        }
        for p in self.pins.values() {
            ensure(p.snapshots.len() == 2, "pin prefix snapshots")?;
            let mut ids = BTreeSet::new();
            for s in &p.snapshots {
                self.check_snapshot(s)?;
                ids.extend(s.keys());
            }
            ensure(ids == p.units, "pin prefix membership")?;
        }
        Ok(())
    }
    fn check_snapshot(&self, s: &BTreeMap<u64, Prefix>) -> Result<()> {
        for (id, p) in s {
            let u = self.units.get(id).ok_or("unowned prefix")?;
            ensure(p.bytes <= u.bytes && p.sha.len() == 64, "prefix boundary")?;
        }
        Ok(())
    }
}
impl Fixture {
    fn prefix(&self, u: &Unit, p: &Prefix) -> Result<Vec<u8>> {
        ensure(p.bytes <= PACK, "bounded prefix")?;
        let path = self.file(u.id);
        let m = io(fs::symlink_metadata(&path))?;
        ensure(
            m.file_type().is_file()
                && m.dev() == u.dev
                && m.ino() == u.ino
                && m.nlink() == 1
                && m.len() >= p.bytes
                && m.len() <= PACK,
            "prefix file identity/extent",
        )?;
        let mut file = io(File::open(path))?;
        let mut bytes = vec![0; usize::try_from(p.bytes).map_err(|_| "prefix size")?];
        io(file.read_exact(&mut bytes))?;
        ensure(hash(&bytes) == p.sha, "prefix digest")?;
        Ok(bytes)
    }
    fn verify_snapshot(&self, s: &BTreeMap<u64, Prefix>) -> Result<()> {
        for (id, p) in s {
            self.prefix(self.ledger.units.get(id).ok_or("snapshot owner")?, p)?;
        }
        Ok(())
    }
    fn verify_all_prefixes(&self) -> Result<()> {
        self.verify_snapshot(&self.ledger.active_prefixes)?;
        self.verify_snapshot(&self.ledger.recovery_prefixes)?;
        for pin in self.ledger.pins.values() {
            for snapshot in &pin.snapshots {
                self.verify_snapshot(snapshot)?;
            }
        }
        Ok(())
    }
    fn pack(&mut self, role: Role, bytes: &[u8]) -> Result<Unit> {
        let h = self
            .admit(
                &[(0, bytes.to_vec(), PACK + 16384)],
                &Space::modeled(&self.ledger, u64::MAX),
            )?
            .remove(0);
        let u = self.materialize(&h, bytes, Cut::None)?;
        let mut next = self.ledger.clone();
        let v = next.units.get_mut(&u.id).unwrap();
        v.role = role;
        v.sealed = false;
        self.publish(next, Cut::None)?;
        Ok(self.ledger.units[&u.id].clone())
    }
    fn plan_growth(&mut self, requests: &[(u64, Vec<u8>)], space: &Space) -> Result<Vec<Growth>> {
        self.known()?;
        ensure(
            self.ledger.growths.is_empty() && self.ledger.holds.is_empty(),
            "original allocation/growth must resolve first",
        )?;
        ensure(
            space.model_qualified && space.generation == self.ledger.generation,
            "fresh qualified model availability required",
        )?;
        ensure(
            !requests.is_empty() && requests.len() <= 3,
            "bounded arena group",
        )?;
        let mut next = self.ledger.clone();
        let mut ids = BTreeSet::new();
        let mut by_domain = BTreeMap::<u64, u64>::new();
        let mut result = vec![];
        for (id, suffix) in requests {
            let u = self.ledger.units.get(id).ok_or("growth owner")?.clone();
            ensure(
                !u.sealed
                    && !u.retiring
                    && ids.insert(*id)
                    && !suffix.is_empty()
                    && suffix.len() <= CHUNK,
                "active unique pack and chunk cap",
            )?;
            ensure(
                io(fs::metadata(self.file(*id)))?.len() == u.bytes,
                "unreconciled physical tail",
            )?;
            let mut bytes = self.prefix(&u, &Prefix::of(&u))?;
            bytes.extend(suffix);
            ensure(bytes.len() as u64 <= PACK, "seal/rollover required")?;
            let bound = PACK.saturating_sub(u.charge) + 16384;
            by_domain.insert(
                u.domain,
                add(by_domain.get(&u.domain).copied().unwrap_or(0), bound)?,
            );
            let token = next.token()?;
            let g = Growth {
                token,
                original: u,
                suffix: suffix.clone(),
                target: Prefix {
                    bytes: bytes.len() as u64,
                    sha: hash(&bytes),
                },
                bound,
            };
            next.growths.insert(token, g.clone());
            result.push(g);
        }
        for (d, bytes) in by_domain {
            ensure(
                add(add(self.ledger.held(d)?, bytes)?, MARGIN)?
                    <= *space.available.get(&d).ok_or("capacity domain")?,
                "fresh availability refusal",
            )?;
        }
        self.publish(next, Cut::None)?;
        Ok(result)
    }
    fn apply_growth(&mut self, group: &[Growth], cut: Cut) -> Result<()> {
        self.known()?;
        if self.ledger.growths.is_empty() {
            for g in group {
                let u = self
                    .ledger
                    .units
                    .get(&g.original.id)
                    .ok_or("completed growth owner")?;
                ensure(u.bytes >= g.target.bytes, "growth not selected")?;
                self.prefix(u, &g.target)?;
            }
            return Ok(());
        }
        ensure(
            group.len() == self.ledger.growths.len()
                && group
                    .iter()
                    .all(|g| self.ledger.growths.get(&g.token) == Some(g)),
            "exact original group required",
        )?;
        let mut next = self.ledger.clone();
        for (index, g) in group.iter().enumerate() {
            let mut expected = self.prefix(&g.original, &Prefix::of(&g.original))?;
            expected.extend(&g.suffix);
            ensure(hash(&expected) == g.target.sha, "growth commitment")?;
            let path = self.file(g.original.id);
            let mut file = io(OpenOptions::new().read(true).append(true).open(&path))?;
            let m = io(file.metadata())?;
            ensure(
                m.dev() == g.original.dev
                    && m.ino() == g.original.ino
                    && m.nlink() == 1
                    && m.len() >= g.original.bytes
                    && m.len() <= g.target.bytes,
                "original growth extent",
            )?;
            let mut existing = vec![0; usize::try_from(m.len()).map_err(|_| "extent")?];
            io(file.seek(SeekFrom::Start(0)))?;
            io(file.read_exact(&mut existing))?;
            ensure(
                existing == expected[..existing.len()],
                "partial suffix corruption; retain liability",
            )?;
            if cut == Cut::BeforeWrite {
                return Err("ENOSPC before append; original group held".into());
            }
            let remaining = &expected[existing.len()..];
            io(file.write_all(remaining))?;
            self.c.process_write_bytes = add(self.c.process_write_bytes, remaining.len() as u64)?;
            self.c.prefix_append_bytes = add(self.c.prefix_append_bytes, remaining.len() as u64)?;
            if cut == Cut::AfterWrite && index == 0 {
                return Err("post-effect first-arena failure; whole group held".into());
            }
            self.barrier(&file)?;
            self.dir_barrier()?;
            let stable = io(file.metadata())?;
            ensure(
                stable.len() == g.target.bytes
                    && stable.dev() == g.original.dev
                    && stable.ino() == g.original.ino
                    && stable.nlink() == 1,
                "post-barrier growth identity",
            )?;
            self.prefix(&g.original, &g.target)?;
            let charge = stable
                .blocks()
                .checked_mul(512)
                .ok_or("block overflow")?
                .max(g.original.charge)
                .max(g.target.bytes);
            ensure(
                charge - g.original.charge <= g.bound,
                "observed growth exceeds original hold; fence",
            )?;
            let u = next.units.get_mut(&g.original.id).unwrap();
            u.bytes = g.target.bytes;
            u.sha.clone_from(&g.target.sha);
            u.charge = charge;
            self.c.peak_observed_file_blocks =
                self.c.peak_observed_file_blocks.max(self.allocated()?);
        }
        next.growths.clear();
        self.publish(next, cut)
    }
    fn seal(&mut self, id: u64) -> Result<Unit> {
        self.known()?;
        ensure(
            self.ledger.growths.is_empty() && self.ledger.holds.is_empty(),
            "unresolved growth cannot seal",
        )?;
        let u = self.ledger.units.get(&id).ok_or("seal owner")?.clone();
        ensure(!u.retiring && !u.sealed, "seal once")?;
        ensure(
            io(fs::metadata(self.file(id)))?.len() == u.bytes,
            "seal selected prefix only",
        )?;
        self.prefix(&u, &Prefix::of(&u))?;
        let charge = self.ledger.owned(u.domain)?;
        let mut next = self.ledger.clone();
        next.units.get_mut(&id).unwrap().sealed = true;
        self.publish(next, Cut::None)?;
        ensure(
            self.ledger.owned(u.domain)? == charge,
            "sealing double charged ownership",
        )?;
        Ok(self.ledger.units[&id].clone())
    }
}
// Exact v3 Arena framing: LE u32 payload length, optional data u64 stable key.
fn frame(role: Role, ordinal: u64) -> Result<Vec<u8>> {
    let body = encode(
        &json!({"fixture_role":role,"ordinal":ordinal,"commitment":hash(&ordinal.to_le_bytes())}),
    )?;
    let mut bytes = u32::try_from(body.len())
        .map_err(|_| "frame")?
        .to_le_bytes()
        .to_vec();
    if role == Role::Data {
        bytes.extend(ordinal.to_le_bytes());
    }
    bytes.extend(body);
    Ok(bytes)
}
fn requests(f: &Fixture, ids: &[u64], start: u64, count: u64) -> Result<Vec<(u64, Vec<u8>)>> {
    ids.iter()
        .map(|id| {
            let mut bytes = vec![];
            for n in start..start + count {
                bytes.extend(frame(f.ledger.units[id].role, n)?);
            }
            Ok((*id, bytes))
        })
        .collect()
}
fn setup() -> Result<(tempfile::TempDir, Fixture, Vec<u64>)> {
    let d = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut f = Fixture::create(d.path(), CONTROL + 4 * 1024 * 1024)?;
    let mut ids = vec![];
    for r in [Role::Data, Role::Locator, Role::Control] {
        ids.push(f.pack(r, &frame(r, 0)?)?.id);
    }
    f.select(ids.iter().copied().collect())?;
    Ok((d, f, ids))
}
pub(super) fn main_run() -> Result<()> {
    for count in [1, 8, 32] {
        let (d, mut f, ids) = setup()?;
        f.c = Counters::default();
        let before = f.allocated()?;
        let mut rows = vec![];
        let mut peak = 0;
        for batch in 0..8 {
            let t = Instant::now();
            let request = requests(&f, &ids, 1 + batch * count, count)?;
            let payload_bytes: usize = request.iter().map(|(_, bytes)| bytes.len()).sum();
            let before_bytes = f.c.process_write_bytes;
            let before_sync = f.c.sync_calls;
            let before_allocated = f.allocated()?;
            let g = f.plan_growth(&request, &Space::modeled(&f.ledger, u64::MAX))?;
            peak = peak.max(add(f.ledger.owned(0)?, f.ledger.held(0)?)?);
            let held = f.ledger.held(0)?;
            f.apply_growth(&g, Cut::None)?;
            f.select(ids.iter().copied().collect())?;
            let checkpoint_micros = t.elapsed().as_micros();
            let audit = Instant::now();
            f.verify_all_prefixes()?;
            rows.push(json!({"batch":batch,"records_per_role":count,"payload_bytes":payload_bytes,"process_bytes":f.c.process_write_bytes-before_bytes,"sync_calls":f.c.sync_calls-before_sync,"net_allocated_file_bytes":i128::from(f.allocated()?)-i128::from(before_allocated),"held_increment":held,"owned_charge":f.ledger.owned(0)?,"checkpoint_micros":checkpoint_micros,"separate_prefix_audit_micros":audit.elapsed().as_micros()}));
        }
        let checkpoint_stats = json!(f.c);
        let open = Instant::now();
        let reopened = Fixture::open(d.path())?;
        let open_micros = open.elapsed().as_micros();
        ensure(reopened.digest == f.digest, "structural reopen selection")?;
        let head_bytes = io(fs::metadata(d.path().join("HEAD")))?.len();
        let seal = Instant::now();
        let before_seal = f.ledger.owned(0)?;
        for id in &ids {
            f.seal(*id)?;
        }
        ensure(
            f.ledger.owned(0)? == before_seal,
            "seal transferred charge twice",
        )?;
        let seal_micros = seal.elapsed().as_micros();
        let net_allocated = i128::from(f.allocated()?) - i128::from(before);
        let turnover = Instant::now();
        f.select(BTreeSet::new())?;
        ensure(
            f.begin_retire(&f.ledger.units[&ids[0]].clone()).is_err(),
            "recovery must retain sealed generation",
        )?;
        f.select(BTreeSet::new())?;
        let turnover_micros = turnover.elapsed().as_micros();
        let retirement = Instant::now();
        let mut released = 0;
        for id in &ids {
            f.begin_retire(&f.ledger.units[id].clone())?;
            released += f.retire(*id, Cut::None)?;
            ensure(f.retire(*id, Cut::None)? == 0, "repeated credit")?;
        }
        println!(
            "{}",
            json!({"fixture":"prefix-ownership-adapter","group":count,"groups":8,"roles":["data","locator","control"],"rows":rows,"checkpoint_stats":checkpoint_stats,"all_phase_stats":f.c,"peak_owned_plus_growth_hold":peak,"sealed_owned_charge":before_seal,"final_owned_charge_after_retirement":f.ledger.owned(0)?,"net_allocated_file_bytes_at_seal":net_allocated,"structural_reopen_micros":open_micros,"structural_reopen_head_bytes":head_bytes,"structural_reopen_payload_read_bytes":0,"seal_micros":seal_micros,"two_selection_turnover_micros":turnover_micros,"retirement_micros":retirement.elapsed().as_micros(),"retired_project_charge":released,"sealed_units":ids.len(),"qualified_os_reservation":false,"qualified_saved":false,"actual_v3_writer_integrated":false,"genuine_graph_integrated":false})
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_prefixes_survive_atomic_multi_arena_growth_and_seal() {
        let (_d, mut f, ids) = setup().unwrap();
        let original = f.ledger.active_prefixes.clone();
        let pins: Vec<_> = CLASSES
            .into_iter()
            .map(|c| f.acquire(c, false).unwrap())
            .collect();
        let g = f
            .plan_growth(
                &requests(&f, &ids, 1, 32).unwrap(),
                &Space::modeled(&f.ledger, u64::MAX),
            )
            .unwrap();
        f.apply_growth(&g, Cut::None).unwrap();
        f.verify_all_prefixes().unwrap();
        assert_eq!(f.ledger.active_prefixes, original);
        let charge = f.ledger.owned(0).unwrap();
        let bytes = f.c.process_write_bytes;
        f.apply_growth(&g, Cut::None).unwrap();
        assert_eq!(f.ledger.owned(0).unwrap(), charge);
        assert_eq!(f.c.process_write_bytes, bytes);
        f.select(ids.iter().copied().collect()).unwrap();
        assert_eq!(f.ledger.recovery_prefixes, original);
        for id in &ids {
            let u = f.seal(*id).unwrap();
            assert!(f.begin_retire(&u).is_err());
        }
        assert_eq!(f.ledger.owned(0).unwrap(), charge);
        f.select(BTreeSet::new()).unwrap();
        f.select(BTreeSet::new()).unwrap();
        for p in pins {
            assert!(f.begin_retire(&f.ledger.units[&ids[0]].clone()).is_err());
            f.release(&p, true, true, true).unwrap();
        }
        for id in ids {
            let u = f.ledger.units[&id].clone();
            f.begin_retire(&u).unwrap();
            assert_eq!(f.retire(id, Cut::None).unwrap(), u.charge);
        }
        assert_eq!(f.ledger.owned(0).unwrap(), CONTROL);
    }
    #[test]
    fn all_growth_cuts_preserve_old_prefix_and_do_not_charge_twice() {
        for cut in [
            Cut::BeforeWrite,
            Cut::AfterWrite,
            Cut::BeforeHead,
            Cut::AfterHead,
        ] {
            let (d, mut f, ids) = setup().unwrap();
            let old = f.ledger.active_prefixes.clone();
            let g = f
                .plan_growth(
                    &requests(&f, &ids, 1, 8).unwrap(),
                    &Space::modeled(&f.ledger, u64::MAX),
                )
                .unwrap();
            assert!(f.apply_growth(&g, cut).is_err());
            f.verify_snapshot(&old).unwrap();
            drop(f);
            let mut f = Fixture::open(d.path()).unwrap();
            if f.dir.join("intent").exists() {
                f.reconcile().unwrap();
            }
            f.apply_growth(&g, Cut::None).unwrap();
            f.verify_snapshot(&old).unwrap();
            let charge = f.ledger.owned(0).unwrap();
            f.apply_growth(&g, Cut::None).unwrap();
            assert_eq!(f.ledger.owned(0).unwrap(), charge);
            assert!(f.ledger.growths.is_empty());
        }
    }
    #[test]
    fn partial_valid_suffix_resumes_but_corrupt_tail_fences() {
        for corrupt in [false, true] {
            let (_d, mut f, ids) = setup().unwrap();
            let g = f
                .plan_growth(
                    &requests(&f, &ids, 1, 8).unwrap(),
                    &Space::modeled(&f.ledger, u64::MAX),
                )
                .unwrap();
            let first = &g[0];
            let mut file = OpenOptions::new()
                .append(true)
                .open(f.file(first.original.id))
                .unwrap();
            let mut half = first.suffix[..first.suffix.len() / 2].to_vec();
            if corrupt {
                half[0] ^= 1;
            }
            file.write_all(&half).unwrap();
            let result = f.apply_growth(&g, Cut::None);
            assert_eq!(result.is_err(), corrupt);
            if corrupt {
                assert_eq!(f.ledger.growths.len(), 3);
                assert!(f.seal(ids[0]).is_err());
            }
        }
    }
    #[test]
    fn active_prefix_cannot_retire_or_cross_chunk_bound() {
        let (_d, mut f, ids) = setup().unwrap();
        let u = f.ledger.units[&ids[0]].clone();
        f.select(BTreeSet::new()).unwrap();
        f.select(BTreeSet::new()).unwrap();
        assert!(f.begin_retire(&u).is_err());
        let before = f.digest.clone();
        assert!(
            f.plan_growth(
                &[(ids[0], vec![0; CHUNK + 1])],
                &Space::modeled(&f.ledger, u64::MAX)
            )
            .is_err()
        );
        assert_eq!(f.digest, before);
        f.seal(ids[0]).unwrap();
        assert!(
            f.plan_growth(
                &[(ids[0], vec![0; 16])],
                &Space::modeled(&f.ledger, u64::MAX)
            )
            .is_err()
        );
    }
    #[test]
    fn exact_pack_boundary_seals_and_rolls_to_distinct_owned_generation() {
        let d = tempfile::tempdir().unwrap();
        let mut f = Fixture::create(d.path(), CONTROL + 4 * PACK).unwrap();
        let u = f.pack(Role::Locator, &vec![7; PACK as usize - 64]).unwrap();
        f.select(BTreeSet::from([u.id])).unwrap();
        let before = f.digest.clone();
        assert!(
            f.plan_growth(&[(u.id, vec![8; 65])], &Space::modeled(&f.ledger, u64::MAX))
                .is_err()
        );
        assert_eq!(f.digest, before);
        let g = f
            .plan_growth(&[(u.id, vec![8; 64])], &Space::modeled(&f.ledger, u64::MAX))
            .unwrap();
        f.apply_growth(&g, Cut::None).unwrap();
        let old = f.seal(u.id).unwrap();
        assert_eq!(old.bytes, PACK);
        let new = f
            .pack(Role::Locator, &frame(Role::Locator, 1).unwrap())
            .unwrap();
        assert_ne!(old.ino, new.ino);
        f.select(BTreeSet::from([new.id])).unwrap();
        assert!(f.begin_retire(&old).is_err());
        f.verify_all_prefixes().unwrap();
        f.select(BTreeSet::from([new.id])).unwrap();
        f.begin_retire(&old).unwrap();
        assert_eq!(f.retire(old.id, Cut::None).unwrap(), old.charge);
        assert_eq!(f.ledger.owned(0).unwrap(), CONTROL + new.charge);
    }
    #[test]
    fn repeated_pre_head_cuts_reuse_exact_extent_under_original_hold() {
        let (d, mut f, ids) = setup().unwrap();
        let g = f
            .plan_growth(
                &requests(&f, &ids, 1, 32).unwrap(),
                &Space::modeled(&f.ledger, u64::MAX),
            )
            .unwrap();
        let hold = f.ledger.held(0).unwrap();
        for _ in 0..5 {
            assert!(f.apply_growth(&g, Cut::BeforeHead).is_err());
            f = Fixture::open(d.path()).unwrap();
            f.reconcile().unwrap();
            assert_eq!(f.ledger.held(0).unwrap(), hold);
            for growth in &g {
                let metadata = fs::metadata(f.file(growth.original.id)).unwrap();
                assert_eq!(metadata.len(), growth.target.bytes);
                assert_eq!(metadata.ino(), growth.original.ino);
            }
        }
        f.apply_growth(&g, Cut::None).unwrap();
        assert_eq!(f.c.prefix_append_bytes, 0);
        assert!(f.ledger.growths.is_empty());
    }
    #[test]
    fn shared_domain_group_admission_is_atomic_and_space_is_independent() {
        let (_d, mut f, ids) = setup().unwrap();
        let request = requests(&f, &ids, 1, 8).unwrap();
        let bound: u64 = ids
            .iter()
            .map(|id| PACK.saturating_sub(f.ledger.units[id].charge) + 16384)
            .sum();
        let before = f.digest.clone();
        let writes = f.c.process_write_bytes;
        assert!(
            f.plan_growth(&request, &Space::modeled(&f.ledger, bound + MARGIN - 1))
                .is_err()
        );
        assert_eq!(f.digest, before);
        assert_eq!(f.c.process_write_bytes, writes);
        let mut next = f.ledger.clone();
        next.limits
            .insert(0, f.ledger.owned(0).unwrap() + bound - 1);
        f.publish(next, Cut::None).unwrap();
        let before = f.digest.clone();
        assert!(
            f.plan_growth(&request, &Space::modeled(&f.ledger, u64::MAX))
                .is_err()
        );
        assert_eq!(f.digest, before);
        assert!(f.ledger.growths.is_empty());
    }
}
