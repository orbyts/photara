//! Disposable ownership-credit prerequisite, not production capacity qualification.
//! Fork of allocation-credit gate for bounded extending data/locator/control packs.
#![allow(
    dead_code,
    reason = "Inherited disposable primitives retained for comparison"
)]
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Instant,
};
type Result<T> = std::result::Result<T, String>;
const MAX_FRAME: usize = 256 * 1024;
const CONTROL: u64 = 4 * MAX_FRAME as u64;
const MARGIN: u64 = 64 * 1024;
const MAX_UNITS: usize = 128;
const MAX_PINS: usize = 64;
const MAX_HOLDS: usize = 16;
fn ensure(ok: bool, s: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(s.into()) }
}
fn io<T>(r: std::io::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("capacity overflow".into())
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(v).map_err(|e| e.to_string())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Unit {
    role: Role,
    sealed: bool,
    id: u64,
    domain: u64,
    bytes: u64,
    sha: String,
    dev: u64,
    ino: u64,
    charge: u64,
    retiring: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Hold {
    id: u64,
    domain: u64,
    bound: u64,
    bytes: u64,
    sha: String,
    started: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Class {
    AcceptedJournal,
    Undo,
    History,
    Conversion,
    Export,
    Backup,
    Reader,
    Unresolved,
    Relocation,
    Resource,
}
#[cfg(test)]
const CLASSES: [Class; 10] = [
    Class::AcceptedJournal,
    Class::Undo,
    Class::History,
    Class::Conversion,
    Class::Export,
    Class::Backup,
    Class::Reader,
    Class::Unresolved,
    Class::Relocation,
    Class::Resource,
];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Pin {
    token: u64,
    epoch: u64,
    class: Class,
    units: BTreeSet<u64>,
    snapshots: Vec<BTreeMap<u64, Prefix>>,
    unknown: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Ledger {
    growths: BTreeMap<u64, Growth>,
    active_prefixes: BTreeMap<u64, Prefix>,
    recovery_prefixes: BTreeMap<u64, Prefix>,
    generation: u64,
    next: u64,
    epoch: u64,
    limits: BTreeMap<u64, u64>,
    units: BTreeMap<u64, Unit>,
    holds: BTreeMap<u64, Hold>,
    pins: BTreeMap<u64, Pin>,
    active: BTreeSet<u64>,
    recovery: BTreeSet<u64>,
}
impl Ledger {
    fn token(&mut self) -> Result<u64> {
        let n = self.next;
        self.next = add(n, 1)?;
        Ok(n)
    }
    fn owned(&self, d: u64) -> Result<u64> {
        self.units
            .values()
            .filter(|u| u.domain == d)
            .try_fold(if d == 0 { CONTROL } else { 0 }, |n, u| add(n, u.charge))
    }
    fn held(&self, d: u64) -> Result<u64> {
        let growth = self
            .growths
            .values()
            .filter(|g| g.original.domain == d)
            .try_fold(0, |n, g| add(n, g.bound))?;
        self.holds
            .values()
            .filter(|h| h.domain == d)
            .try_fold(growth, |n, h| add(n, h.bound))
    }
    fn validate(&self) -> Result<()> {
        self.validate_prefixes()?;
        ensure(
            self.units.len() + self.holds.len() <= MAX_UNITS
                && self.pins.len() <= MAX_PINS
                && self.holds.len() <= MAX_HOLDS
                && self.limits.len() <= 4,
            "bounded fixture state",
        )?;
        for (&id, u) in &self.units {
            ensure(
                id == u.id
                    && id < self.next
                    && self.limits.contains_key(&u.domain)
                    && u.charge >= u.bytes,
                "owned allocation shape",
            )?;
        }
        for (&id, h) in &self.holds {
            ensure(
                id == h.id
                    && id < self.next
                    && !self.units.contains_key(&id)
                    && self.limits.contains_key(&h.domain)
                    && h.bound >= h.bytes,
                "liability shape",
            )?;
        }
        for (&id, p) in &self.pins {
            ensure(
                id == p.token
                    && id < self.next
                    && p.epoch == self.epoch
                    && p.units.iter().all(|u| self.units.contains_key(u)),
                "pin shape",
            )?;
        }
        for id in self.active.union(&self.recovery) {
            ensure(self.units.contains_key(id), "selected unowned allocation")?;
        }
        for (&d, &limit) in &self.limits {
            ensure(
                add(self.owned(d)?, self.held(d)?)? <= limit,
                "project allocation capacity",
            )?;
        }
        Ok(())
    }
    fn unpinned(&self, id: u64) -> Result<()> {
        ensure(self.growths.is_empty(), "pending growth fences retirement")?;
        ensure(
            !self.active.contains(&id) && !self.recovery.contains(&id),
            "selected active/recovery allocation",
        )?;
        ensure(
            self.pins
                .values()
                .all(|p| !p.unknown && !p.units.contains(&id)),
            "live/unknown obligation",
        )?;
        ensure(
            self.holds.values().all(|h| !h.started),
            "unknown allocation effect fences retirement",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Envelope {
    ledger: Ledger,
    sha: String,
}
impl Envelope {
    fn new(ledger: Ledger) -> Result<Self> {
        let sha = hash(&encode(&ledger)?);
        Ok(Self { ledger, sha })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Intent {
    old: String,
    candidate: String,
}
#[derive(Clone, Debug)]
struct Space {
    generation: u64,
    available: BTreeMap<u64, u64>,
    model_qualified: bool,
}
impl Space {
    fn modeled(l: &Ledger, available: u64) -> Self {
        Self {
            generation: l.generation,
            available: l.limits.keys().map(|d| (*d, available)).collect(),
            model_qualified: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    None,
    BeforeHead,
    AfterHead,
    BeforeWrite,
    AfterWrite,
    BeforeUnlink,
    AfterUnlink,
    AfterAbsenceBarrier,
}
#[derive(Clone, Debug, Default, Serialize)]
struct Counters {
    process_write_bytes: u64,
    prefix_append_bytes: u64,
    sync_calls: u64,
    publications: u64,
    candidate_pin_checks: u64,
    peak_observed_file_blocks: u64,
}
struct Fixture {
    dir: PathBuf,
    ledger: Ledger,
    digest: String,
    c: Counters,
}
impl Fixture {
    fn file(&self, id: u64) -> PathBuf {
        self.dir.join(format!("allocation-{id:020}.pack"))
    }
    fn barrier(&mut self, file: &File) -> Result<()> {
        io(file.sync_all())?;
        self.c.sync_calls += 1;
        Ok(())
    }
    fn dir_barrier(&mut self) -> Result<()> {
        let f = io(File::open(&self.dir))?;
        self.barrier(&f)
    }
    fn write<T: Serialize>(&mut self, name: &str, value: &T) -> Result<()> {
        let bytes = encode(value)?;
        ensure(bytes.len() <= MAX_FRAME, "bounded control frame")?;
        let mut f = io(OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.dir.join(name)))?;
        io(f.write_all(&bytes))?;
        self.c.process_write_bytes = add(self.c.process_write_bytes, bytes.len() as u64)?;
        self.barrier(&f)?;
        self.dir_barrier()?;
        self.c.peak_observed_file_blocks = self.c.peak_observed_file_blocks.max(self.allocated()?);
        Ok(())
    }
    fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
        ensure(
            io(fs::metadata(path))?.len() <= MAX_FRAME as u64,
            "control frame cap",
        )?;
        serde_json::from_slice(&io(fs::read(path))?).map_err(|e| e.to_string())
    }
    fn read_head(&self) -> Result<Envelope> {
        let h: Envelope = Self::read(&self.dir.join("HEAD"))?;
        ensure(hash(&encode(&h.ledger)?) == h.sha, "HEAD integrity")?;
        h.ledger.validate()?;
        Ok(h)
    }
    fn known(&self) -> Result<()> {
        ensure(
            !self.dir.join("intent").exists(),
            "unknown selection; retain all liabilities",
        )?;
        ensure(self.read_head()?.sha == self.digest, "stale owner HEAD")
    }
    fn create(dir: &Path, limit: u64) -> Result<Self> {
        let ledger = Ledger {
            growths: BTreeMap::new(),
            active_prefixes: BTreeMap::new(),
            recovery_prefixes: BTreeMap::new(),
            generation: 0,
            next: 1,
            epoch: 1,
            limits: BTreeMap::from([(0, limit)]),
            units: BTreeMap::new(),
            holds: BTreeMap::new(),
            pins: BTreeMap::new(),
            active: BTreeSet::new(),
            recovery: BTreeSet::new(),
        };
        ledger.validate()?;
        let h = Envelope::new(ledger.clone())?;
        let mut f = Self {
            dir: dir.into(),
            ledger,
            digest: h.sha.clone(),
            c: Counters::default(),
        };
        f.write("HEAD", &h)?;
        Ok(f)
    }
    fn open(dir: &Path) -> Result<Self> {
        let h: Envelope = Self::read(&dir.join("HEAD"))?;
        ensure(hash(&encode(&h.ledger)?) == h.sha, "HEAD integrity")?;
        h.ledger.validate()?;
        Ok(Self {
            dir: dir.into(),
            ledger: h.ledger,
            digest: h.sha,
            c: Counters::default(),
        })
    }
    fn publish(&mut self, mut next: Ledger, cut: Cut) -> Result<()> {
        self.known()?;
        next.generation = add(self.ledger.generation, 1)?;
        next.validate()?;
        let h = Envelope::new(next)?;
        ensure(
            encode(&h)?.len() <= MAX_FRAME,
            "candidate control frame preflight",
        )?;
        ensure(
            !self.dir.join("candidate").exists(),
            "unreconciled candidate",
        )?;
        self.write(
            "intent",
            &Intent {
                old: self.digest.clone(),
                candidate: h.sha.clone(),
            },
        )?;
        self.write("candidate", &h)?;
        if cut == Cut::BeforeHead {
            return Err("ENOSPC before HEAD; outcome fenced".into());
        }
        io(fs::rename(
            self.dir.join("candidate"),
            self.dir.join("HEAD"),
        ))?;
        if cut == Cut::AfterHead {
            return Err("post-effect EIO/unknown HEAD; outcome fenced".into());
        }
        self.dir_barrier()?;
        io(fs::remove_file(self.dir.join("intent")))?;
        self.dir_barrier()?;
        self.ledger = h.ledger;
        self.digest = h.sha;
        self.c.publications += 1;
        Ok(())
    }
    #[cfg(test)]
    fn reconcile(&mut self) -> Result<&'static str> {
        let intent: Intent = Self::read(&self.dir.join("intent"))?;
        let h = self.read_head()?;
        let outcome = if h.sha == intent.old {
            "old"
        } else if h.sha == intent.candidate {
            "candidate"
        } else {
            return Err("unknown HEAD; retain original intent/liability".into());
        };
        let file = io(File::open(self.dir.join("HEAD")))?;
        self.barrier(&file)?;
        self.dir_barrier()?;
        if self.dir.join("candidate").exists() {
            let c: Envelope = Self::read(&self.dir.join("candidate"))?;
            ensure(
                c.sha == intent.candidate && hash(&encode(&c.ledger)?) == c.sha,
                "candidate mismatch",
            )?;
            io(fs::remove_file(self.dir.join("candidate")))?;
        }
        io(fs::remove_file(self.dir.join("intent")))?;
        self.dir_barrier()?;
        self.ledger = h.ledger;
        self.digest = h.sha;
        Ok(outcome)
    }
    fn admit(&mut self, requests: &[(u64, Vec<u8>, u64)], space: &Space) -> Result<Vec<Hold>> {
        self.known()?;
        ensure(
            self.ledger.growths.is_empty(),
            "pending growth fences allocation admission",
        )?;
        ensure(
            space.model_qualified && space.generation == self.ledger.generation,
            "fresh qualified model observation required",
        )?;
        ensure(
            self.ledger.holds.values().all(|h| !h.started),
            "unknown effect fences new admission",
        )?;
        let mut requested = BTreeMap::<u64, u64>::new();
        let mut next = self.ledger.clone();
        let mut result = vec![];
        for (domain, bytes, bound) in requests {
            ensure(
                !bytes.is_empty() && bytes.len() <= MAX_FRAME && *bound >= bytes.len() as u64,
                "allocation bound",
            )?;
            requested.insert(
                *domain,
                add(requested.get(domain).copied().unwrap_or(0), *bound)?,
            );
            let id = next.token()?;
            let h = Hold {
                id,
                domain: *domain,
                bound: *bound,
                bytes: bytes.len() as u64,
                sha: hash(bytes),
                started: false,
            };
            next.holds.insert(id, h.clone());
            result.push(h);
        }
        ensure(!result.is_empty(), "empty admission")?;
        for (d, bytes) in requested {
            let available = space.available.get(&d).ok_or("unknown capacity domain")?;
            ensure(
                add(add(self.ledger.held(d)?, bytes)?, MARGIN)? <= *available,
                "fresh filesystem/quota observation refusal",
            )?;
        }
        next.validate()?;
        self.publish(next, Cut::None)?;
        Ok(result)
    }
    fn materialize(&mut self, original: &Hold, bytes: &[u8], cut: Cut) -> Result<Unit> {
        self.known()?;
        let mut hold = self
            .ledger
            .holds
            .get(&original.id)
            .cloned()
            .ok_or("original liability missing")?;
        ensure(
            hold.id == original.id
                && hold.domain == original.domain
                && hold.bound == original.bound
                && hold.bytes == original.bytes
                && hold.sha == original.sha,
            "liability identity",
        )?;
        ensure(
            hash(bytes) == hold.sha && bytes.len() as u64 == hold.bytes,
            "original payload required",
        )?;
        if !hold.started {
            hold.started = true;
            let mut next = self.ledger.clone();
            next.holds.insert(hold.id, hold.clone());
            self.publish(next, Cut::None)?;
        }
        if cut == Cut::BeforeWrite {
            return Err("ENOSPC before data effect; hold retained".into());
        }
        let path = self.file(hold.id);
        if !path.exists() {
            let mut f = io(OpenOptions::new().create_new(true).write(true).open(&path))?;
            io(f.write_all(bytes))?;
            self.c.process_write_bytes = add(self.c.process_write_bytes, bytes.len() as u64)?;
        }
        if cut == Cut::AfterWrite {
            return Err("post-effect ENOSPC; hold retained".into());
        }
        let metadata = io(fs::symlink_metadata(&path))?;
        ensure(
            metadata.file_type().is_file() && metadata.len() == hold.bytes && metadata.nlink() == 1,
            "allocation identity/type/extent",
        )?;
        ensure(
            hash(&io(fs::read(&path))?) == hold.sha,
            "original allocation verification",
        )?;
        let file = io(File::open(&path))?;
        self.barrier(&file)?;
        self.dir_barrier()?;
        let stable = io(fs::symlink_metadata(&path))?;
        ensure(
            stable.dev() == metadata.dev()
                && stable.ino() == metadata.ino()
                && stable.len() == metadata.len()
                && stable.nlink() == 1,
            "post-barrier allocation identity",
        )?;
        let charge = stable
            .blocks()
            .checked_mul(512)
            .ok_or("allocation overflow")?
            .max(hold.bytes);
        ensure(
            charge <= hold.bound,
            "observed allocation exceeds modeled bound; fence original liability",
        )?;
        self.c.peak_observed_file_blocks = self.c.peak_observed_file_blocks.max(self.allocated()?);
        let unit = Unit {
            role: Role::Data,
            sealed: true,
            id: hold.id,
            domain: hold.domain,
            bytes: hold.bytes,
            sha: hold.sha,
            dev: metadata.dev(),
            ino: metadata.ino(),
            charge,
            retiring: false,
        };
        let mut next = self.ledger.clone();
        next.holds.remove(&unit.id);
        next.units.insert(unit.id, unit.clone());
        self.publish(next, cut)?;
        Ok(unit)
    }
    fn select(&mut self, active: BTreeSet<u64>) -> Result<()> {
        self.known()?;
        ensure(
            self.ledger.growths.is_empty(),
            "pending growth fences selection",
        )?;
        ensure(
            active
                .iter()
                .all(|id| self.ledger.units.get(id).is_some_and(|u| !u.retiring)),
            "select only live verified units",
        )?;
        let mut next = self.ledger.clone();
        next.recovery_prefixes = next.active_prefixes;
        next.active_prefixes = active
            .iter()
            .map(|id| (*id, Prefix::of(&next.units[id])))
            .collect();
        next.recovery = next.active;
        next.active = active;
        self.publish(next, Cut::None)
    }
    #[cfg(test)]
    fn acquire(&mut self, class: Class, unknown: bool) -> Result<Pin> {
        self.known()?;
        let mut next = self.ledger.clone();
        let token = next.token()?;
        let pin = Pin {
            token,
            epoch: next.epoch,
            class,
            units: next.active.union(&next.recovery).copied().collect(),
            snapshots: vec![next.active_prefixes.clone(), next.recovery_prefixes.clone()],
            unknown,
        };
        next.pins.insert(token, pin.clone());
        self.publish(next, Cut::None)?;
        Ok(pin)
    }
    #[cfg(test)]
    fn release(
        &mut self,
        pin: &Pin,
        completed: bool,
        owner_ended: bool,
        unknown_resolved: bool,
    ) -> Result<()> {
        self.known()?;
        ensure(
            self.ledger.pins.get(&pin.token) == Some(pin) && pin.epoch == self.ledger.epoch,
            "exact pin/epoch required",
        )?;
        ensure(
            completed
                && (pin.class != Class::Reader || owner_ended)
                && (!pin.unknown || unknown_resolved),
            "completion/owner/unknown proof missing",
        )?;
        let mut next = self.ledger.clone();
        next.pins.remove(&pin.token);
        self.publish(next, Cut::None)
    }
    fn begin_retire(&mut self, unit: &Unit) -> Result<()> {
        self.known()?;
        self.c.candidate_pin_checks += self.ledger.pins.len() as u64;
        self.ledger.unpinned(unit.id)?;
        ensure(
            self.ledger.units.get(&unit.id) == Some(unit) && !unit.retiring && unit.sealed,
            "exact owned generation required",
        )?;
        let path = self.file(unit.id);
        let metadata = io(fs::symlink_metadata(&path))?;
        ensure(
            metadata.file_type().is_file()
                && metadata.dev() == unit.dev
                && metadata.ino() == unit.ino
                && metadata.len() == unit.bytes
                && metadata.nlink() == 1,
            "retirement preimage ownership",
        )?;
        ensure(
            hash(&io(fs::read(&path))?) == unit.sha,
            "retirement preimage content",
        )?;
        let mut next = self.ledger.clone();
        next.units.get_mut(&unit.id).unwrap().retiring = true;
        self.publish(next, Cut::None)
    }
    fn retire(&mut self, id: u64, cut: Cut) -> Result<u64> {
        self.known()?;
        let Some(unit) = self.ledger.units.get(&id).cloned() else {
            return Ok(0);
        };
        ensure(unit.retiring, "retirement must first be selected")?;
        self.ledger.unpinned(id)?;
        let path = self.file(id);
        if path.exists() {
            let metadata = io(fs::symlink_metadata(&path))?;
            ensure(
                metadata.file_type().is_file()
                    && metadata.dev() == unit.dev
                    && metadata.ino() == unit.ino
                    && metadata.len() == unit.bytes
                    && metadata.nlink() == 1,
                "retirement ownership/generation mismatch",
            )?;
            ensure(
                hash(&io(fs::read(&path))?) == unit.sha,
                "retirement content mismatch",
            )?;
            if cut == Cut::BeforeUnlink {
                return Err("ENOSPC/EIO before unlink; no credit".into());
            }
            io(fs::remove_file(&path))?;
        }
        if cut == Cut::AfterUnlink {
            return Err("post-effect unlink error; original charge retained".into());
        }
        self.dir_barrier()?;
        // Fresh namespace lookup after the barrier; absence alone was not credit.
        ensure(
            fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "retirement absence proof",
        )?;
        if cut == Cut::AfterAbsenceBarrier {
            return Err("lost retirement completion; charge retained".into());
        }
        let mut next = self.ledger.clone();
        next.unpinned(id)?;
        next.units.remove(&id);
        self.publish(next, cut)?;
        Ok(unit.charge)
    }
    fn allocated(&self) -> Result<u64> {
        let mut n = 0;
        for e in io(fs::read_dir(&self.dir))? {
            n = add(n, io(io(e)?.metadata())?.blocks() * 512)?;
        }
        Ok(n)
    }
}

fn run(cycles: u64) -> Result<serde_json::Value> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut f = Fixture::create(dir.path(), CONTROL + 128 * 1024)?;
    let payload = vec![7; 8192];
    let mut previous = None;
    let mut peak_charge = 0;
    let mut peak_allocated = 0;
    let mut refunded = 0;
    let mut rows = vec![];
    for cycle in 0..cycles {
        let before = f.c.process_write_bytes;
        let t = Instant::now();
        let space = Space::modeled(&f.ledger, 16 * 1024 * 1024);
        let hold = f
            .admit(&[(0, payload.clone(), 64 * 1024)], &space)?
            .remove(0);
        peak_charge = peak_charge.max(add(f.ledger.owned(0)?, f.ledger.held(0)?)?);
        let unit = f.materialize(&hold, &payload, Cut::None)?;
        f.select(BTreeSet::from([unit.id]))?;
        peak_allocated = peak_allocated.max(f.allocated()?);
        if let Some(old) = previous {
            ensure(
                f.begin_retire(&old).is_err(),
                "recovery root must retain old unit",
            )?;
            f.select(BTreeSet::from([unit.id]))?;
            f.begin_retire(&old)?;
            refunded = add(refunded, f.retire(old.id, Cut::None)?)?;
            ensure(f.retire(old.id, Cut::None)? == 0, "duplicate credit")?;
        }
        rows.push(json!({"cycle":cycle,"micros":t.elapsed().as_micros(),"process_bytes":f.c.process_write_bytes-before,"owned":f.ledger.owned(0)?,"held":f.ledger.held(0)?}));
        previous = Some(unit);
    }
    Ok(
        json!({"fixture":"allocation-ownership-credit-prerequisite","cycles":cycles,"rows":rows,"refunded_project_charge":refunded,"final_owned_project_charge":f.ledger.owned(0)?,"final_live_units":f.ledger.units.len(),"peak_modeled_owned_plus_hold":peak_charge,"peak_selected_boundary_file_blocks":peak_allocated,"control_allowance":CONTROL,"stats":f.c,"elapsed_ms":started.elapsed().as_millis(),"qualified_os_reservation":false,"qualified_saved":false,"actual_free_space_increased_by_credit":false,"graph_v3_integration":false,"limits":{"live_units":MAX_UNITS,"pins":MAX_PINS,"holds":MAX_HOLDS,"frame":MAX_FRAME}}),
    )
}
fn legacy_main() -> Result<()> {
    for cycles in [1, 8, 32] {
        println!("{}", run(cycles)?);
    }
    Ok(())
}

#[cfg(test)]
mod inherited_tests {
    use super::*;
    #[test]
    fn repeated_original_hold_recovery_reuses_the_same_physical_allocation() {
        let (d, mut f) = fixture();
        let bytes = vec![4; 8192];
        let hold = f
            .admit(
                &[(0, bytes.clone(), 64 * 1024)],
                &Space::modeled(&f.ledger, u64::MAX),
            )
            .unwrap()
            .remove(0);
        assert!(f.materialize(&hold, &bytes, Cut::AfterWrite).is_err());
        let original = fs::metadata(f.file(hold.id)).unwrap();
        for _ in 0..5 {
            assert!(f.materialize(&hold, &bytes, Cut::BeforeHead).is_err());
            drop(f);
            f = Fixture::open(d.path()).unwrap();
            assert_eq!(f.reconcile().unwrap(), "old");
            let observed = fs::metadata(f.file(hold.id)).unwrap();
            assert_eq!(observed.ino(), original.ino());
            assert_eq!(observed.len(), original.len());
            assert_eq!(f.ledger.held(0).unwrap(), hold.bound);
        }
        f.materialize(&hold, &bytes, Cut::None).unwrap();
        assert!(f.ledger.holds.is_empty());
        assert_eq!(f.ledger.units.len(), 1);
    }
    fn fixture() -> (tempfile::TempDir, Fixture) {
        let d = tempfile::tempdir().unwrap();
        let f = Fixture::create(d.path(), CONTROL + 128 * 1024).unwrap();
        (d, f)
    }
    fn unit(f: &mut Fixture) -> Unit {
        let b = vec![9; 8192];
        let h = f
            .admit(
                &[(0, b.clone(), 64 * 1024)],
                &Space::modeled(&f.ledger, 16 * 1024 * 1024),
            )
            .unwrap()
            .remove(0);
        f.materialize(&h, &b, Cut::None).unwrap()
    }
    #[test]
    fn two_selections_and_every_pin_class_precede_credit() {
        let (_d, mut f) = fixture();
        let old = unit(&mut f);
        f.select(BTreeSet::from([old.id])).unwrap();
        let pins: Vec<_> = CLASSES
            .into_iter()
            .map(|c| f.acquire(c, c == Class::Unresolved).unwrap())
            .collect();
        let new = unit(&mut f);
        f.select(BTreeSet::from([new.id])).unwrap();
        assert!(f.begin_retire(&old).is_err());
        f.select(BTreeSet::from([new.id])).unwrap();
        for p in pins {
            assert!(f.begin_retire(&old).is_err());
            assert!(f.release(&p, false, false, false).is_err());
            f.release(&p, true, true, true).unwrap();
            assert!(f.release(&p, true, true, true).is_err());
        }
        f.begin_retire(&old).unwrap();
        let before = f.ledger.owned(0).unwrap();
        assert_eq!(f.retire(old.id, Cut::None).unwrap(), old.charge);
        assert_eq!(f.ledger.owned(0).unwrap(), before - old.charge);
        assert_eq!(f.retire(old.id, Cut::None).unwrap(), 0);
    }
    #[test]
    fn deletion_never_invents_filesystem_availability() {
        let (_d, mut f) = fixture();
        let old = unit(&mut f);
        f.begin_retire(&old).unwrap();
        let stale = Space::modeled(&f.ledger, 16 * 1024 * 1024);
        f.retire(old.id, Cut::None).unwrap();
        let requests = [(0, vec![2; 8192], 64 * 1024)];
        assert!(f.admit(&requests, &stale).is_err());
        let mut fresh = Space::modeled(&f.ledger, 0);
        assert!(f.admit(&requests, &fresh).is_err());
        fresh.model_qualified = false;
        fresh.available.insert(0, u64::MAX);
        assert!(f.admit(&requests, &fresh).is_err());
    }
    #[test]
    fn shared_domain_liabilities_sum_before_any_effect() {
        let (_d, mut f) = fixture();
        let before = f.digest.clone();
        let requests = [(0, vec![1; 8192], 80 * 1024), (0, vec![2; 8192], 80 * 1024)];
        assert!(
            f.admit(&requests, &Space::modeled(&f.ledger, u64::MAX))
                .is_err()
        );
        assert_eq!(f.digest, before);
        assert!(f.ledger.holds.is_empty());
        let h = f
            .admit(
                &[(0, vec![1; 8192], 96 * 1024)],
                &Space::modeled(&f.ledger, u64::MAX),
            )
            .unwrap();
        assert!(
            f.admit(
                &[(0, vec![2; 8192], 40 * 1024)],
                &Space::modeled(&f.ledger, u64::MAX)
            )
            .is_err()
        );
        assert_eq!(f.ledger.holds.len(), h.len());
    }
    #[test]
    fn pre_and_post_effect_allocation_failure_keep_original_hold() {
        for cut in [
            Cut::BeforeWrite,
            Cut::AfterWrite,
            Cut::BeforeHead,
            Cut::AfterHead,
        ] {
            let (d, mut f) = fixture();
            let b = vec![3; 8192];
            let h = f
                .admit(
                    &[(0, b.clone(), 64 * 1024)],
                    &Space::modeled(&f.ledger, u64::MAX),
                )
                .unwrap()
                .remove(0);
            assert!(f.materialize(&h, &b, cut).is_err());
            drop(f);
            let mut f = Fixture::open(d.path()).unwrap();
            if f.dir.join("intent").exists() {
                f.reconcile().unwrap();
            }
            if f.ledger.holds.contains_key(&h.id) {
                assert!(
                    f.admit(
                        &[(0, b.clone(), 1 << 16)],
                        &Space::modeled(&f.ledger, u64::MAX)
                    )
                    .is_err()
                );
                f.materialize(&h, &b, Cut::None).unwrap();
            }
            assert_eq!(f.ledger.units.len(), 1);
            assert!(f.ledger.holds.is_empty());
        }
    }
    #[test]
    fn interruption_cannot_double_credit_or_admit_through_unknown_head() {
        for cut in [
            Cut::BeforeUnlink,
            Cut::AfterUnlink,
            Cut::AfterAbsenceBarrier,
            Cut::BeforeHead,
            Cut::AfterHead,
        ] {
            let (d, mut f) = fixture();
            let u = unit(&mut f);
            f.begin_retire(&u).unwrap();
            let old = f.ledger.owned(0).unwrap();
            assert!(f.retire(u.id, cut).is_err());
            assert_eq!(f.ledger.owned(0).unwrap(), old);
            if f.dir.join("intent").exists() {
                assert!(
                    f.admit(
                        &[(0, vec![1; 8192], 64 * 1024)],
                        &Space::modeled(&f.ledger, u64::MAX)
                    )
                    .is_err()
                );
            }
            drop(f);
            let mut f = Fixture::open(d.path()).unwrap();
            if f.dir.join("intent").exists() {
                f.reconcile().unwrap();
            }
            f.retire(u.id, Cut::None).unwrap();
            assert_eq!(f.ledger.owned(0).unwrap(), CONTROL);
            assert_eq!(f.retire(u.id, Cut::None).unwrap(), 0);
        }
    }
    #[test]
    fn wrong_generation_and_unknown_selector_refuse() {
        let (_d, mut f) = fixture();
        let u = unit(&mut f);
        f.begin_retire(&u).unwrap();
        let bytes = vec![8; 8192];
        io(fs::write(f.file(u.id), bytes)).unwrap();
        assert!(f.retire(u.id, Cut::None).is_err());
        assert!(f.file(u.id).exists());
        let mut h = f.ledger.clone();
        h.generation += 100;
        f.write(
            "intent",
            &Intent {
                old: "unknown".into(),
                candidate: "unknown".into(),
            },
        )
        .unwrap();
        assert!(f.reconcile().is_err());
        assert!(f.dir.join("intent").exists());
    }
    #[test]
    fn hardlink_ownership_and_stale_reader_epoch_do_not_release() {
        let (_d, mut f) = fixture();
        let u = unit(&mut f);
        f.select(BTreeSet::from([u.id])).unwrap();
        let pin = f.acquire(Class::Reader, false).unwrap();
        let mut stale = pin.clone();
        stale.epoch += 1;
        assert!(f.release(&stale, true, true, true).is_err());
        assert!(f.release(&pin, true, false, true).is_err());
        f.release(&pin, true, true, true).unwrap();
        f.select(BTreeSet::new()).unwrap();
        f.select(BTreeSet::new()).unwrap();
        let alias = f.dir.join("untracked-alias");
        fs::hard_link(f.file(u.id), &alias).unwrap();
        assert!(f.begin_retire(&u).is_err());
        assert!(f.file(u.id).exists());
        fs::remove_file(alias).unwrap();
        f.begin_retire(&u).unwrap();
        f.retire(u.id, Cut::None).unwrap();
    }
    #[test]
    fn repeated_turnover_returns_project_charge_without_tombstone_growth() {
        let r = run(8).unwrap();
        assert_eq!(r["final_live_units"], json!(1));
        assert!(r["refunded_project_charge"].as_u64().unwrap() > 0);
        assert_eq!(r["final_owned_project_charge"], json!(CONTROL + 8192));
    }
}

#[path = "ps2_prefix_ownership/adapter.rs"]
mod adapter;
use adapter::{Growth, Prefix, Role};
fn main() -> Result<()> {
    adapter::main_run()
}
