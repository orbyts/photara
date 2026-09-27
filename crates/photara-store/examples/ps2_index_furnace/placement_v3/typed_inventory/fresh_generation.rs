//! Disposable actual-Fixture birth enrollment prerequisite, not permanent wire.
//!
//! Uses Liability.birth under the actual HEAD/gate; legacy holds keep None.
//! Ordinary completion/checkpoint resumption refuses this birth-only liability.
//!
//! A complete nonce-bound marker can establish the initial generation only in
//! this fixture's private, exclusive namespace. This does not qualify a production
//! lease or hostile-namespace policy. Empty/partial unbound markers fence; they
//! are never adopted or deleted. Once selected, inode witnesses are mandatory.
//!
//! All promoted packs remain EMPTY and fully covered by the original hold.
//! Graph journal acceptance, payload recipes, rollover and settlement are later
//! integration. In particular, created-and-sealed charges need a pre-admitted
//! finalization corridor in the last two tips; do not invent predicted refunds
//! or let charge-record insertion trigger an unreserved rollover.
#![allow(
    dead_code,
    reason = "Enrollment prerequisite has only disposable test callers until Graph composition"
)]
#[allow(clippy::wildcard_imports, reason = "Actual-v3 fixture child")]
use super::*;

const MAX_GENERATIONS: usize = 8;
const MAX_PER_ARENA: u64 = 4;
const MAX_BIRTH: usize = 64 * 1024;
const MARKER_MAX: u64 = 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Slot {
    data: bool,
    pack: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Plan {
    token: u64,
    nonce: String,
    request_sha: String,
    directory_dev: u64,
    directory_ino: u64,
    // Captured once with no holds. Never nest successive selected HEADs here.
    base: Selection,
    slots: Vec<Slot>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Witness {
    dev: u64,
    ino: u64,
    marker_len: u64,
    marker_sha: String,
    marker_observed_charge: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Phase {
    Intended,
    Bound(Witness),
    Empty(Witness),
    Promoted(Witness),
}
impl Phase {
    fn ordinal(&self) -> u64 {
        match self {
            Self::Intended => 0,
            Self::Bound(_) => 1,
            Self::Empty(_) => 2,
            Self::Promoted(_) => 3,
        }
    }
    fn witness(&self) -> Option<&Witness> {
        match self {
            Self::Intended => None,
            Self::Bound(w) | Self::Empty(w) | Self::Promoted(w) => Some(w),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Birth {
    plan: Plan,
    phases: Vec<Phase>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: String,
    token: u64,
    nonce: String,
    plan_sha: String,
    data: bool,
    pack: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    AfterCreate,
    PartialMarker,
    MarkerBytes,
    MarkerBarrier,
    BeforeWitnessHead,
    StagedWitnessHead,
    AfterWitnessHead,
    AfterStagedSelectorCleanup,
    AfterTruncate,
    EmptyBarrier,
    BeforeEmptyHead,
    AfterEmptyHead,
    AfterLink,
    LinkBarrier,
    AfterStageUnlink,
    PromotionBarrier,
    BeforePromotedHead,
    AfterPromotedHead,
}
fn cut(fault: Fault, at: Fault) -> Result<()> {
    ensure(fault != at, "injected birth cut; original hold retained")
}
fn selector_cut(fault: Fault, before: Fault, after: Fault) -> Cut {
    if fault == Fault::StagedWitnessHead && before == Fault::BeforeWitnessHead {
        Cut::StagedHead
    } else if fault == before {
        Cut::BeforeHead
    } else if fault == after {
        Cut::AfterHead
    } else {
        Cut::None
    }
}
impl Birth {
    fn marker(&self, slot: &Slot) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&Marker {
            schema: "example.fixture.ps2.birth.v1".into(),
            token: self.plan.token,
            nonce: self.plan.nonce.clone(),
            plan_sha: hash(&serde_json::to_vec(&self.plan).map_err(error)?),
            data: slot.data,
            pack: slot.pack,
        })
        .map_err(error)?;
        ensure(bytes.len() as u64 <= MARKER_MAX, "birth marker bound")?;
        Ok(bytes)
    }
    fn stage(&self, f: &Fixture, slot: &Slot) -> PathBuf {
        f.dir.join(format!(
            "birth-{}-{}-{}-{}",
            self.plan.token,
            self.plan.nonce,
            if slot.data { "data" } else { "meta" },
            slot.pack
        ))
    }
    fn destination(f: &Fixture, slot: &Slot) -> PathBuf {
        if slot.data {
            f.data.path(slot.pack)
        } else {
            f.meta.path(slot.pack)
        }
    }
    fn validate(&self) -> Result<()> {
        ensure(
            self.plan.base.gate.holds.is_empty()
                && self
                    .plan
                    .base
                    .gate
                    .ledger
                    .as_ref()
                    .is_some_and(retirement_ledger::Ledger::enrollment_complete)
                && self.plan.token == checked(self.plan.base.gate.last_token, 1)?
                && self.plan.nonce.len() == 32
                && self.plan.nonce.bytes().all(|b| b.is_ascii_hexdigit())
                && self.plan.request_sha.len() == 64
                && self.plan.request_sha.bytes().all(|b| b.is_ascii_hexdigit())
                && !self.plan.slots.is_empty()
                && self.plan.slots.len() <= MAX_GENERATIONS
                && self.phases.len() == self.plan.slots.len()
                && serde_json::to_vec(self).map_err(error)?.len() <= MAX_BIRTH,
            "bounded immutable birth plan",
        )?;
        let mut data = self.plan.base.data_pack;
        let mut meta = self.plan.base.meta_pack;
        let mut seen_meta = false;
        let mut incomplete = false;
        for (slot, phase) in self.plan.slots.iter().zip(&self.phases) {
            if slot.data {
                ensure(!seen_meta, "birth arena order")?;
                data = checked(data, 1)?;
                ensure(slot.pack == data, "birth data ordinal")?;
            } else {
                seen_meta = true;
                meta = checked(meta, 1)?;
                ensure(slot.pack == meta, "birth metadata ordinal")?;
            }
            if incomplete {
                ensure(*phase == Phase::Intended, "birth sequential phase order")?;
            }
            incomplete |= !matches!(phase, Phase::Promoted(_));
            if let Some(w) = phase.witness() {
                let bytes = self.marker(slot)?;
                ensure(
                    w.marker_len == bytes.len() as u64
                        && w.marker_sha == hash(&bytes)
                        && w.marker_observed_charge >= w.marker_len,
                    "birth marker witness commitment",
                )?;
            }
        }
        ensure(
            data - self.plan.base.data_pack <= MAX_PER_ARENA
                && meta - self.plan.base.meta_pack <= MAX_PER_ARENA,
            "birth arena cap",
        )
    }
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(e)),
    }
}
fn open_private(path: &Path, witness: Option<&Witness>, links: u64) -> Result<File> {
    let m = fs::symlink_metadata(path).map_err(error)?;
    ensure(
        m.is_file() && m.nlink() == links,
        "birth private regular file",
    )?;
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDWR | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(error)?;
    let file = File::from(fd);
    let opened = file.metadata().map_err(error)?;
    ensure(
        opened.dev() == m.dev()
            && opened.ino() == m.ino()
            && opened.nlink() == links
            && witness.is_none_or(|w| w.dev == m.dev() && w.ino == m.ino()),
        "birth original inode witness mismatch",
    )?;
    Ok(file)
}
fn read_marker(file: &mut File, expected: &[u8], allow_empty: bool) -> Result<()> {
    let len = file.metadata().map_err(error)?.len();
    ensure(
        (allow_empty && len == 0) || len == expected.len() as u64,
        "unbound empty or partial birth marker fences original hold",
    )?;
    let mut bytes = vec![0; usize::try_from(len).map_err(error)?];
    file.seek(SeekFrom::Start(0)).map_err(error)?;
    file.read_exact(&mut bytes).map_err(error)?;
    ensure(
        (allow_empty && bytes.is_empty()) || bytes == expected,
        "unknown birth marker; no adoption or deletion",
    )
}
fn control(f: &Fixture, role: &str) -> Result<Option<Selection>> {
    let path = f.dir.join(role);
    let Some(m) = metadata(&path)? else {
        return Ok(None);
    };
    ensure(m.len() <= GATE_MAX as u64, "birth control frame cap")?;
    let mut file = open_private(&path, None, 1)?;
    let mut bytes = vec![0; usize::try_from(m.len()).map_err(error)?];
    file.read_exact(&mut bytes).map_err(error)?;
    ensure(
        file.metadata().map_err(error)?.len() == m.len(),
        "birth control changed during read",
    )?;
    let selected: Selection = serde_json::from_slice(&bytes).map_err(error)?;
    ensure(
        serde_json::to_vec(&selected).map_err(error)? == bytes,
        "birth control bytes differ from exact canonical selector",
    )?;
    Ok(Some(selected))
}
fn validate_selector(h: &Selection, original: &Liability) -> Result<Birth> {
    let birth = h
        .gate
        .holds
        .get(&original.token)
        .and_then(|hold| hold.birth.clone())
        .ok_or("selected original birth absent")?;
    birth.validate()?;
    let initial = original.birth.as_ref().ok_or("original birth absent")?;
    initial.validate()?;
    ensure(birth.plan == initial.plan, "birth immutable plan changed")?;
    let canonical = canonical_initial(initial)?;
    let mut supplied = original.clone();
    supplied.birth.clone_from(&canonical.birth);
    ensure(
        supplied == canonical,
        "birth liability differs from immutable admitted plan",
    )?;
    let mut expected_hold = canonical;
    expected_hold.birth = Some(birth.clone());
    let mut expected = birth.plan.base.clone();
    expected.gate.last_token = original.token;
    expected.gate.holds.insert(original.token, expected_hold);
    expected.epoch = birth
        .phases
        .iter()
        .try_fold(checked(birth.plan.base.epoch, 1)?, |epoch, phase| {
            checked(epoch, phase.ordinal())
        })?;
    ensure(
        *h == expected,
        "unknown birth selector; original evidence retained",
    )?;
    Ok(birth)
}
fn canonical_initial(birth: &Birth) -> Result<Liability> {
    birth.validate()?;
    let mut initial = birth.clone();
    initial.phases.fill(Phase::Intended);
    let base = &birth.plan.base;
    let mut bounded = base.clone();
    bounded.gate.last_token = birth.plan.token;
    bounded.epoch = u64::MAX;
    for domain in bounded.gate.domains.values_mut() {
        domain.charged = u64::MAX;
        domain.limit = u64::MAX;
    }
    let len = serde_json::to_vec(&bounded).map_err(error)?.len();
    ensure(len <= GATE_MAX, "birth original control cap")?;
    let control = 4 * round(len as u64 + 4096) + 8 * 4096 + 65536 + 65536;
    let budget = (birth.plan.slots.len() as u64)
        .checked_mul(PACK + 32768)
        .ok_or("birth reserve overflow")?;
    Ok(Liability {
        birth: Some(initial),
        retirement_ticket: None,
        graph: None,
        token: birth.plan.token,
        epoch: base.epoch,
        target: base.semantic.clone(),
        retirement: None,
        by_domain: BTreeMap::from([(0, checked(budget, control)?)]),
        continuation: None,
        control,
        origin: Continuation::from_selection(base),
    })
}
fn transition(old: &Birth, new: &Birth) -> Result<()> {
    ensure(old.plan == new.plan, "birth transition plan changed")?;
    let changed = old
        .phases
        .iter()
        .zip(&new.phases)
        .filter(|(a, b)| a != b)
        .collect::<Vec<_>>();
    ensure(
        changed.len() == 1,
        "birth requires one bounded phase transition",
    )?;
    let (a, b) = changed[0];
    ensure(
        b.ordinal() == a.ordinal() + 1 && a.witness().is_none_or(|w| b.witness() == Some(w)),
        "birth phase rewound or inode witness changed",
    )
}
/// While a birth hold is selected, only its next exact phase may change HEAD.
/// This also prevents pin/capacity administration from silently invalidating
/// the admitted plan, and prevents any generic path from clearing its liability.
pub(in super::super) fn admit_gate_change(selected: &Selection, gate: &GateState) -> Result<()> {
    let current = selected.gate.holds.values().find(|h| h.birth.is_some());
    let incoming = gate.holds.values().find(|h| h.birth.is_some());
    if current.is_none() && incoming.is_none() {
        return Ok(());
    }
    let exact = current.or(incoming).ok_or("birth transition absent")?;
    let original = canonical_initial(exact.birth.as_ref().ok_or("birth absent")?)?;
    let mut prospective = selected.clone();
    prospective.gate = gate.clone();
    prospective.epoch = checked(selected.epoch, 1)?;
    let new = validate_selector(&prospective, &original)?;
    if current.is_some() {
        let old = validate_selector(selected, &original)?;
        transition(&old, &new)
    } else {
        ensure(
            new.plan.base == *selected && new.phases.iter().all(|p| *p == Phase::Intended),
            "birth initial admission differs from original base",
        )
    }
}
fn validate_directory(f: &Fixture, birth: &Birth) -> Result<()> {
    let m = fs::symlink_metadata(&f.dir).map_err(error)?;
    ensure(
        m.is_dir() && m.dev() == birth.plan.directory_dev && m.ino() == birth.plan.directory_ino,
        "birth private namespace identity changed",
    )
}
fn validate_witnesses(f: &Fixture, birth: &Birth) -> Result<()> {
    validate_directory(f, birth)?;
    for (slot, phase) in birth.plan.slots.iter().zip(&birth.phases) {
        let Some(w) = phase.witness() else {
            continue;
        };
        let stage = birth.stage(f, slot);
        let destination = Birth::destination(f, slot);
        match phase {
            Phase::Bound(_) => {
                ensure(
                    metadata(&destination)?.is_none(),
                    "premature birth destination",
                )?;
                read_marker(
                    &mut open_private(&stage, Some(w), 1)?,
                    &birth.marker(slot)?,
                    true,
                )?;
            }
            Phase::Empty(_) | Phase::Promoted(_) => {
                let a = metadata(&stage)?;
                let b = metadata(&destination)?;
                ensure(a.is_some() || b.is_some(), "bound birth generation absent")?;
                ensure(
                    !matches!(phase, Phase::Promoted(_)) || (a.is_none() && b.is_some()),
                    "promoted birth namespace mismatch",
                )?;
                let links = if a.is_some() && b.is_some() { 2 } else { 1 };
                for path in [&stage, &destination] {
                    if metadata(path)?.is_some() {
                        let file = open_private(path, Some(w), links)?;
                        ensure(
                            file.metadata().map_err(error)?.len() == 0,
                            "birth candidate acquired unexpected payload",
                        )?;
                    }
                }
            }
            Phase::Intended => unreachable!(),
        }
    }
    Ok(())
}

/// Select the complete immutable plan and capacity hold before stage creation.
fn reserve(
    f: &mut Fixture,
    data_count: u64,
    meta_count: u64,
    request_sha: &str,
) -> Result<Liability> {
    reserve_capped(f, data_count, meta_count, request_sha, MAX_BIRTH, GATE_MAX)
}
fn reserve_capped(
    f: &mut Fixture,
    data_count: u64,
    meta_count: u64,
    request_sha: &str,
    birth_cap: usize,
    head_cap: usize,
) -> Result<Liability> {
    ensure(
        !f.imported_read_only
            && f.head
                .gate
                .ledger
                .as_ref()
                .is_some_and(retirement_ledger::Ledger::enrollment_complete)
            && f.head.gate.holds.is_empty()
            && !f.dir.join("intent").exists()
            && data_count <= MAX_PER_ARENA
            && meta_count <= MAX_PER_ARENA
            && data_count + meta_count > 0,
        "birth requires idle attributed fixture and bounded fresh slots",
    )?;
    ensure(
        control(f, "HEAD")?.as_ref() == Some(&f.head),
        "birth admission selector differs",
    )?;
    for role in ["intent", "candidate", "HEAD.next"] {
        ensure(
            control(f, role)?.is_none(),
            "birth admission retains pending control evidence",
        )?;
    }
    let mut gate = f.head.gate.clone();
    let token = gate.token()?;
    let directory = fs::symlink_metadata(&f.dir).map_err(error)?;
    let slots = (1..=data_count)
        .map(|n| {
            Ok(Slot {
                data: true,
                pack: checked(f.head.data_pack, n)?,
            })
        })
        .chain((1..=meta_count).map(|n| {
            Ok(Slot {
                data: false,
                pack: checked(f.head.meta_pack, n)?,
            })
        }))
        .collect::<Result<Vec<_>>>()?;
    let birth = Birth {
        phases: vec![Phase::Intended; slots.len()],
        plan: Plan {
            token,
            nonce: uuid::Uuid::new_v4().simple().to_string(),
            request_sha: request_sha.into(),
            directory_dev: directory.dev(),
            directory_ino: directory.ino(),
            base: f.head.clone(),
            slots,
        },
    };
    birth.validate()?;
    validate_directory(f, &birth)?;
    for slot in &birth.plan.slots {
        ensure(
            metadata(&birth.stage(f, slot))?.is_none()
                && metadata(&Birth::destination(f, slot))?.is_none(),
            "occupied birth namespace refuses before reservation",
        )?;
    }
    // Includes full final pack capacity, marker staging and namespace/COW budget.
    // It is only a modeled admission bound, never qualified OS reservation.
    let budget = (birth.plan.slots.len() as u64)
        .checked_mul(PACK + 32768)
        .ok_or("birth reserve overflow")?;
    let control = checked(f.control_bound(&gate)?, 65536)?;
    let hold = Liability {
        birth: Some(birth),
        retirement_ticket: None,
        graph: None,
        token,
        epoch: f.head.epoch,
        target: f.head.semantic.clone(),
        retirement: None,
        by_domain: BTreeMap::from([(0, checked(budget, control)?)]),
        continuation: None,
        control,
        origin: Continuation::from_selection(&f.head),
    };
    gate.holds.insert(token, hold.clone());
    preflight_maximum(f, &gate, &hold, birth_cap, head_cap)?;
    gate.validate()?;
    f.select_gate(gate)?;
    validate_selector(&f.head, &hold)?;
    Ok(hold)
}
fn preflight_maximum(
    f: &Fixture,
    gate: &GateState,
    hold: &Liability,
    birth_cap: usize,
    head_cap: usize,
) -> Result<(usize, usize)> {
    let mut maximum = hold.clone();
    let birth = maximum.birth.as_mut().ok_or("maximum birth absent")?;
    for (index, slot) in birth.plan.slots.iter().enumerate() {
        let marker = birth.marker(slot)?;
        birth.phases[index] = Phase::Promoted(Witness {
            dev: u64::MAX,
            ino: u64::MAX,
            marker_len: marker.len() as u64,
            marker_sha: hash(&marker),
            marker_observed_charge: u64::MAX,
        });
    }
    birth.validate()?;
    let birth_bytes = serde_json::to_vec(birth).map_err(error)?.len();
    ensure(
        birth_bytes <= birth_cap,
        "maximum birth phases exceed admission cap",
    )?;
    maximum.control = u64::MAX;
    for value in maximum.by_domain.values_mut() {
        *value = u64::MAX;
    }
    let mut prospective = f.head.clone();
    prospective.gate = gate.clone();
    prospective.gate.holds.insert(hold.token, maximum);
    prospective.epoch = u64::MAX;
    for domain in prospective.gate.domains.values_mut() {
        domain.charged = u64::MAX;
        domain.limit = u64::MAX;
    }
    let head_bytes = serde_json::to_vec(&prospective).map_err(error)?.len();
    ensure(
        head_bytes <= head_cap,
        "maximum birth control exceeds admission cap",
    )?;
    f.head
        .gate
        .ledger
        .as_ref()
        .ok_or("birth attributed ledger")?
        .admit_control(&f.head, &prospective.gate)?;
    Ok((birth_bytes, head_bytes))
}
fn advance(
    f: &mut Fixture,
    original: &Liability,
    birth: &Birth,
    index: usize,
    phase: Phase,
    cut: Cut,
) -> Result<()> {
    let mut next = birth.clone();
    next.phases[index] = phase;
    next.validate()?;
    transition(birth, &next)?;
    let mut gate = f.head.gate.clone();
    gate.holds
        .get_mut(&original.token)
        .ok_or("birth hold absent")?
        .birth = Some(next);
    f.select_gate_cut(gate, cut)
}

/// Complete enrollment only; keep the original liability selected and charged.
#[allow(
    clippy::too_many_lines,
    reason = "Keep original-token phase effects and interruption ordering explicit in this disposable proof"
)]
fn resume(f: &mut Fixture, original: &Liability, fault: Fault) -> Result<Liability> {
    let selected = control(f, "HEAD")?.ok_or("birth selected HEAD absent")?;
    ensure(selected == f.head, "birth in-memory/selected state differs")?;
    let mut birth = validate_selector(&selected, original)?;
    validate_witnesses(f, &birth)?;
    let old = control(f, "intent")?;
    let candidate = control(f, "candidate")?;
    let next = control(f, "HEAD.next")?;
    ensure(
        old.is_some() == candidate.is_some(),
        "incomplete birth dispatch retains original hold",
    )?;
    ensure(
        next.is_none() || (old.is_some() && next == candidate),
        "unknown staged birth selector",
    )?;
    if let (Some(old), Some(candidate)) = (old, candidate) {
        let a = validate_selector(&old, original)?;
        let b = validate_selector(&candidate, original)?;
        ensure(
            selected == old || selected == candidate,
            "unknown birth selection",
        )?;
        transition(&a, &b)?;
        validate_witnesses(f, &b)?;
        if next.is_some() {
            // Dispatch and selected HEAD still retain the exact allowed state.
            // Remove only its verified redundant staging copy before reconcile.
            ensure(
                control(f, "HEAD.next")?.as_ref() == Some(&candidate),
                "staged birth selector changed before cleanup",
            )?;
            fs::remove_file(f.dir.join("HEAD.next")).map_err(error)?;
            f.c.files_deleted += 1;
            sync_dir(&f.dir, &mut f.c)?;
            cut(fault, Fault::AfterStagedSelectorCleanup)?;
        }
        f.reconcile()?;
        birth = validate_selector(&f.head, original)?;
    }
    for index in 0..birth.plan.slots.len() {
        let slot = birth.plan.slots[index].clone();
        let stage = birth.stage(f, &slot);
        let destination = Birth::destination(f, &slot);
        if birth.phases[index] == Phase::Intended {
            ensure(
                metadata(&destination)?.is_none(),
                "unbound occupied destination refuses",
            )?;
            let marker = birth.marker(&slot)?;
            if metadata(&stage)?.is_none() {
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .read(true)
                    .write(true)
                    .open(&stage)
                    .map_err(error)?;
                f.c.files_created += 1;
                cut(fault, Fault::AfterCreate)?;
                let count = if fault == Fault::PartialMarker {
                    marker.len() / 2
                } else {
                    marker.len()
                };
                file.write_all(&marker[..count]).map_err(error)?;
                cut(fault, Fault::PartialMarker)?;
                cut(fault, Fault::MarkerBytes)?;
            }
            let mut file = open_private(&stage, None, 1)?;
            read_marker(&mut file, &marker, false)?;
            sync_file(&file, &mut f.c)?;
            sync_dir(&f.dir, &mut f.c)?;
            cut(fault, Fault::MarkerBarrier)?;
            let m = file.metadata().map_err(error)?;
            let witness = Witness {
                dev: m.dev(),
                ino: m.ino(),
                marker_len: m.len(),
                marker_sha: hash(&marker),
                marker_observed_charge: m
                    .blocks()
                    .checked_mul(512)
                    .ok_or("birth allocation overflow")?
                    .max(m.len()),
            };
            advance(
                f,
                original,
                &birth,
                index,
                Phase::Bound(witness),
                selector_cut(fault, Fault::BeforeWitnessHead, Fault::AfterWitnessHead),
            )?;
            birth = validate_selector(&f.head, original)?;
        }
        if let Phase::Bound(w) = birth.phases[index].clone() {
            ensure(
                metadata(&destination)?.is_none(),
                "bound marker destination occupied",
            )?;
            let mut file = open_private(&stage, Some(&w), 1)?;
            read_marker(&mut file, &birth.marker(&slot)?, true)?;
            file.set_len(0).map_err(error)?;
            cut(fault, Fault::AfterTruncate)?;
            sync_file(&file, &mut f.c)?;
            sync_dir(&f.dir, &mut f.c)?;
            cut(fault, Fault::EmptyBarrier)?;
            advance(
                f,
                original,
                &birth,
                index,
                Phase::Empty(w),
                selector_cut(fault, Fault::BeforeEmptyHead, Fault::AfterEmptyHead),
            )?;
            birth = validate_selector(&f.head, original)?;
        }
        if let Phase::Empty(w) = birth.phases[index].clone() {
            validate_witnesses(f, &birth)?;
            if metadata(&destination)?.is_none() {
                fs::hard_link(&stage, &destination).map_err(error)?;
                cut(fault, Fault::AfterLink)?;
            }
            validate_witnesses(f, &birth)?;
            sync_dir(&f.dir, &mut f.c)?;
            cut(fault, Fault::LinkBarrier)?;
            if metadata(&stage)?.is_some() {
                // Only the recorded second link to this exact empty inode.
                open_private(&stage, Some(&w), 2)?;
                open_private(&destination, Some(&w), 2)?;
                fs::remove_file(&stage).map_err(error)?;
                cut(fault, Fault::AfterStageUnlink)?;
            }
            sync_dir(&f.dir, &mut f.c)?;
            let final_file = open_private(&destination, Some(&w), 1)?;
            ensure(
                final_file.metadata().map_err(error)?.len() == 0,
                "promoted birth payload changed",
            )?;
            sync_file(&final_file, &mut f.c)?;
            cut(fault, Fault::PromotionBarrier)?;
            advance(
                f,
                original,
                &birth,
                index,
                Phase::Promoted(w),
                selector_cut(fault, Fault::BeforePromotedHead, Fault::AfterPromotedHead),
            )?;
            birth = validate_selector(&f.head, original)?;
        }
    }
    validate_witnesses(f, &birth)?;
    let current = f
        .head
        .gate
        .holds
        .get(&original.token)
        .ok_or("birth hold vanished")?
        .clone();
    ensure(
        f.head.semantic == original.target && current.by_domain == original.by_domain,
        "birth changed semantic target or capacity",
    )?;
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(kind: Kind) -> Result<(tempfile::TempDir, Fixture)> {
        let (temp, mut source, _legacy) = setup(64, kind)?;
        let mut f = Fixture::create(&temp.path().join("birth"), &mut source)?;
        retirement_ledger::initialize(&mut f)?;
        Ok((temp, f))
    }
    #[test]
    fn birth_bound_cuts_resume_same_token_and_keep_original_capacity() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for fault in [
                Fault::MarkerBytes,
                Fault::MarkerBarrier,
                Fault::BeforeWitnessHead,
                Fault::StagedWitnessHead,
                Fault::AfterWitnessHead,
                Fault::AfterTruncate,
                Fault::EmptyBarrier,
                Fault::BeforeEmptyHead,
                Fault::AfterEmptyHead,
                Fault::AfterLink,
                Fault::LinkBarrier,
                Fault::AfterStageUnlink,
                Fault::PromotionBarrier,
                Fault::BeforePromotedHead,
                Fault::AfterPromotedHead,
            ] {
                let (_temp, mut f) = fixture(kind)?;
                let original = reserve(&mut f, 2, 2, &hash(b"original Graph request"))?;
                let charged = f.head.gate.domains[&0].charged;
                ensure(
                    resume(&mut f, &original, fault).is_err(),
                    "birth fault missing",
                )?;
                f = Fixture::reopen_combined(&f.dir)?;
                // Recover the admitted identity from durable state, not only a
                // cached pre-crash reservation value.
                let recovered = f
                    .head
                    .gate
                    .holds
                    .get(&original.token)
                    .ok_or("recovered birth hold")?
                    .clone();
                let completed = resume(&mut f, &recovered, Fault::None)?;
                ensure(
                    completed.token == original.token
                        && completed.by_domain == original.by_domain
                        && completed.birth.as_ref().is_some_and(|b| {
                            b.phases.iter().all(|p| matches!(p, Phase::Promoted(_)))
                        })
                        && f.head.gate.domains[&0].charged == charged,
                    "birth identity/hold/charge changed",
                )?;
                let bytes = fs::read(f.dir.join("HEAD")).map_err(error)?;
                let before = f.snapshot()?;
                resume(&mut f, &original, Fault::None)?;
                ensure(
                    fs::read(f.dir.join("HEAD")).map_err(error)? == bytes
                        && f.snapshot()? == before,
                    "birth retry duplicated effects",
                )?;
                f.audit_combined()?;
            }
        }
        Ok(())
    }
    #[test]
    fn birth_unbound_empty_partial_and_wrong_marker_fence_without_effects() -> Result<()> {
        for fault in [Fault::AfterCreate, Fault::PartialMarker] {
            let (_temp, mut f) = fixture(Kind::Btree)?;
            let original = reserve(&mut f, 1, 1, &hash(b"birth-fence"))?;
            ensure(
                resume(&mut f, &original, fault).is_err(),
                "birth marker cut",
            )?;
            f = Fixture::reopen_combined(&f.dir)?;
            let birth = original.birth.as_ref().ok_or("birth")?;
            let path = birth.stage(&f, &birth.plan.slots[0]);
            let staged = fs::read(&path).map_err(error)?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                resume(&mut f, &original, Fault::None).is_err(),
                "unbound partial adopted",
            )?;
            ensure(
                fs::read(&path).map_err(error)? == staged
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && f.snapshot()? == before,
                "fenced evidence changed",
            )?;
        }
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let original = reserve(&mut f, 1, 0, &hash(b"birth-marker"))?;
        ensure(
            resume(&mut f, &original, Fault::MarkerBarrier).is_err(),
            "marker cut",
        )?;
        let birth = original.birth.as_ref().ok_or("birth")?;
        let path = birth.stage(&f, &birth.plan.slots[0]);
        let mut bytes = fs::read(&path).map_err(error)?;
        bytes[0] = b'!';
        fs::write(&path, &bytes).map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            resume(&mut f, &original, Fault::None).is_err()
                && fs::read(path).map_err(error)? == bytes
                && f.snapshot()? == before,
            "wrong marker adopted or deleted",
        )
    }
    #[test]
    fn birth_bound_same_bytes_replacement_and_unknown_destination_refuse() -> Result<()> {
        for after_promotion in [false, true] {
            let (_temp, mut f) = fixture(Kind::Radix)?;
            let original = reserve(&mut f, 1, 0, &hash(b"birth-inode"))?;
            let fault = if after_promotion {
                Fault::AfterPromotedHead
            } else {
                Fault::AfterWitnessHead
            };
            ensure(resume(&mut f, &original, fault).is_err(), "witness cut")?;
            f = Fixture::reopen_combined(&f.dir)?;
            let birth = original.birth.as_ref().ok_or("birth")?;
            let slot = &birth.plan.slots[0];
            let path = if after_promotion {
                Birth::destination(&f, slot)
            } else {
                birth.stage(&f, slot)
            };
            let preserved = f.dir.join("held-original-birth");
            fs::rename(&path, &preserved).map_err(error)?;
            fs::copy(&preserved, &path).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                resume(&mut f, &original, Fault::None).is_err() && f.snapshot()? == before,
                "substituted bound inode accepted",
            )?;
        }
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let original = reserve(&mut f, 1, 0, &hash(b"birth-destination"))?;
        let birth = original.birth.as_ref().ok_or("birth")?;
        let target = Birth::destination(&f, &birth.plan.slots[0]);
        fs::write(&target, b"foreign").map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            resume(&mut f, &original, Fault::None).is_err()
                && fs::read(target).map_err(error)? == b"foreign"
                && f.snapshot()? == before,
            "unknown final occupant adopted/deleted",
        )
    }
    #[test]
    fn birth_capacity_and_unit_caps_refuse_before_namespace_effects() -> Result<()> {
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            reserve(&mut f, 5, 0, &hash(b"cap")).is_err(),
            "birth unit cap",
        )?;
        ensure(
            fs::read(f.dir.join("HEAD")).map_err(error)? == head && f.snapshot()? == before,
            "birth cap effects",
        )?;
        let mut gate = f.head.gate.clone();
        let d = gate.domains.get_mut(&0).ok_or("domain")?;
        d.limit = d.charged;
        f.select_gate(gate)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            reserve(&mut f, 1, 1, &hash(b"capacity")).is_err(),
            "birth capacity refusal",
        )?;
        ensure(
            fs::read(f.dir.join("HEAD")).map_err(error)? == head && f.snapshot()? == before,
            "birth refusal created namespace effects",
        )
    }
    #[test]
    fn birth_exact_staged_selector_cleanup_is_itself_recoverable() -> Result<()> {
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let original = reserve(&mut f, 1, 1, &hash(b"staged-control"))?;
        ensure(
            resume(&mut f, &original, Fault::StagedWitnessHead).is_err(),
            "staged HEAD cut",
        )?;
        ensure(
            f.dir.join("HEAD.next").exists(),
            "real staged selector missing",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        ensure(
            resume(&mut f, &original, Fault::AfterStagedSelectorCleanup).is_err(),
            "cleanup cut",
        )?;
        ensure(
            !f.dir.join("HEAD.next").exists()
                && f.dir.join("intent").exists()
                && f.dir.join("candidate").exists(),
            "cleanup discarded dispatch evidence",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        resume(&mut f, &original, Fault::None)?;
        ensure(
            f.head.gate.holds.contains_key(&original.token),
            "staged recovery cleared liability",
        )
    }
    #[test]
    fn birth_unknown_controls_and_changed_pin_or_charge_preserve_evidence() -> Result<()> {
        for mode in 0..5 {
            let (_temp, mut f) = fixture(Kind::Btree)?;
            let pin = f.acquire(PinClass::ReaderLease, false)?;
            let original = reserve(&mut f, 1, 1, &hash(b"control-identity"))?;
            let role = if mode < 2 {
                ensure(
                    resume(&mut f, &original, Fault::BeforeWitnessHead).is_err(),
                    "control cut",
                )?;
                let mut candidate = control(&f, "candidate")?.ok_or("candidate")?;
                if mode == 0 {
                    candidate.gate.domains.get_mut(&0).ok_or("domain")?.charged += 4096;
                } else {
                    candidate
                        .gate
                        .pins
                        .get_mut(&pin.token)
                        .ok_or("pin")?
                        .unknown = true;
                }
                fs::write(
                    f.dir.join("candidate"),
                    serde_json::to_vec(&candidate).map_err(error)?,
                )
                .map_err(error)?;
                "candidate"
            } else if mode == 2 {
                f.envelope("HEAD.next", &f.head.clone())?;
                "HEAD.next"
            } else if mode == 3 {
                f.envelope("candidate", &f.head.clone())?;
                "candidate"
            } else {
                ensure(
                    resume(&mut f, &original, Fault::StagedWitnessHead).is_err(),
                    "staged cut",
                )?;
                let mut next = control(&f, "HEAD.next")?.ok_or("next")?;
                next.gate.domains.get_mut(&0).ok_or("domain")?.limit += 1;
                fs::write(
                    f.dir.join("HEAD.next"),
                    serde_json::to_vec(&next).map_err(error)?,
                )
                .map_err(error)?;
                "HEAD.next"
            };
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let bytes = fs::read(f.dir.join(role)).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                resume(&mut f, &original, Fault::None).is_err(),
                "unknown control accepted",
            )?;
            ensure(
                fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && fs::read(f.dir.join(role)).map_err(error)? == bytes
                    && f.snapshot()? == before,
                "unknown control evidence changed",
            )?;
        }
        Ok(())
    }
    #[test]
    fn birth_eight_promoted_generations_retain_hold_and_fence_generic_paths() -> Result<()> {
        let (_temp, mut f) = fixture(Kind::Radix)?;
        let pin = f.acquire(PinClass::Undo, false)?;
        let original = reserve(&mut f, 4, 4, &hash(b"all-eight"))?;
        let current = resume(&mut f, &original, Fault::None)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            f.complete_liability(&current, &[(0, 0)]).is_err(),
            "birth ordinary completion",
        )?;
        ensure(
            retirement_ledger::complete_growth(&mut f, &current).is_err(),
            "birth direct growth completion",
        )?;
        ensure(
            f.acquire(PinClass::Backup, false).is_err(),
            "birth new pin invalidated plan",
        )?;
        ensure(
            f.release(&pin, true, true, true).is_err(),
            "birth pin released",
        )?;
        ensure(
            f.resume_compaction(&current).is_err(),
            "birth used as retirement token",
        )?;
        let (_other, mut source, _legacy) = setup(1, Kind::Btree)?;
        ensure(
            f.resume_checkpoint(&mut source, 0, &current).is_err(),
            "birth used as checkpoint token",
        )?;
        ensure(
            f.reserve(f.head.semantic.clone(), None, &[(0, 1)]).is_err(),
            "birth allowed second hold",
        )?;
        ensure(
            fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && f.snapshot()? == before
                && f.head.gate.holds.get(&current.token) == Some(&current),
            "birth generic path changed evidence",
        )?;
        f.audit_combined()?;
        Ok(())
    }
    #[test]
    fn birth_worst_phase_envelope_is_preflighted_before_admission() -> Result<()> {
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            reserve_capped(&mut f, 4, 4, &hash(b"envelope"), 1, GATE_MAX).is_err(),
            "maximum birth cap",
        )?;
        ensure(
            reserve_capped(&mut f, 4, 4, &hash(b"envelope"), MAX_BIRTH, 1).is_err(),
            "maximum control cap",
        )?;
        ensure(
            fs::read(f.dir.join("HEAD")).map_err(error)? == head && f.snapshot()? == before,
            "maximum envelope failure created effects",
        )?;
        let original = reserve(&mut f, 4, 4, &hash(b"envelope"))?;
        let initial = serde_json::to_vec(original.birth.as_ref().ok_or("birth")?)
            .map_err(error)?
            .len();
        let (maximum, _) = preflight_maximum(&f, &f.head.gate, &original, MAX_BIRTH, GATE_MAX)?;
        ensure(maximum > initial, "maximum witness shape did not grow")?;
        ensure(
            preflight_maximum(&f, &f.head.gate, &original, initial, GATE_MAX).is_err(),
            "initial-fitting cap admitted oversized final witnesses",
        )
    }
    #[test]
    fn birth_pending_bootstrap_symlink_and_hardlink_refuse() -> Result<()> {
        let (_temp, mut f) = fixture(Kind::Btree)?;
        let claim = Claim::capture(&mut f, true, 0)?;
        let mut gate = f.head.gate.clone();
        let mut value =
            serde_json::to_value(gate.ledger.as_ref().ok_or("ledger")?).map_err(error)?;
        value["pending_enrollment"] = serde_json::to_value(vec![claim]).map_err(error)?;
        gate.ledger = Some(serde_json::from_value(value).map_err(error)?);
        f.select_gate(gate)?;
        let before = f.snapshot()?;
        ensure(
            reserve(&mut f, 1, 1, &hash(b"bootstrap")).is_err() && f.snapshot()? == before,
            "unfinished bootstrap admitted birth",
        )?;
        for hardlink in [false, true] {
            let (_temp, mut f) = fixture(Kind::Btree)?;
            let original = reserve(&mut f, 1, 0, &hash(b"namespace"))?;
            let birth = original.birth.as_ref().ok_or("birth")?;
            let stage = birth.stage(&f, &birth.plan.slots[0]);
            let target = f.dir.join("foreign-marker");
            fs::write(&target, birth.marker(&birth.plan.slots[0])?).map_err(error)?;
            if hardlink {
                fs::hard_link(&target, &stage).map_err(error)?;
            } else {
                std::os::unix::fs::symlink(&target, &stage).map_err(error)?;
            }
            let bytes = fs::read(&target).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                resume(&mut f, &original, Fault::None).is_err()
                    && fs::read(target).map_err(error)? == bytes
                    && f.snapshot()? == before,
                "unsafe birth stage adopted or modified",
            )?;
        }
        Ok(())
    }
    #[test]
    fn birth_reopened_hold_cannot_authenticate_its_own_changed_reservation() -> Result<()> {
        for mode in 0..6 {
            let (_temp, mut f) = fixture(Kind::Btree)?;
            let original = reserve(&mut f, 1, 1, &hash(b"immutable-liability"))?;
            let mut selected = f.head.clone();
            let altered = selected.gate.holds.get_mut(&original.token).ok_or("hold")?;
            match mode {
                0 => {
                    altered.by_domain.insert(0, 0);
                }
                1 => {
                    let n = altered.by_domain[&0];
                    altered.by_domain.insert(0, n - 1);
                }
                2 => {
                    altered.control += 1;
                }
                3 => {
                    altered.origin.data_end += 1;
                }
                4 => {
                    altered.target.active.sha = hash(b"other-target");
                }
                _ => {
                    altered.epoch += 1;
                }
            }
            fs::write(
                f.dir.join("HEAD"),
                serde_json::to_vec(&selected).map_err(error)?,
            )
            .map_err(error)?;
            f = Fixture::reopen_combined(&f.dir)?;
            let recovered = f
                .head
                .gate
                .holds
                .get(&original.token)
                .ok_or("recovered")?
                .clone();
            let bytes = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                resume(&mut f, &recovered, Fault::None).is_err(),
                "recovered changed liability authenticated itself",
            )?;
            ensure(
                fs::read(f.dir.join("HEAD")).map_err(error)? == bytes && f.snapshot()? == before,
                "invalid recovered hold changed evidence",
            )?;
        }
        Ok(())
    }
}
