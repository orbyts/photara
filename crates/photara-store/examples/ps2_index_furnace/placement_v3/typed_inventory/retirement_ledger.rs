//! Opt-in actual-v3 attributable accounting in disposable local fixtures.
//! HEAD remains the sole selector. Standing dispatch capacity is charged once;
//! two current tips retain observed high-water charges, sealed leaves carry their
//! immutable registered charges, and selected original-token tickets carry charge
//! through removal of both ownership edges, unlink and directory-barrier recovery.
//!
//! No legacy high-water balance is reclassified into refundable ownership. This
//! mode starts with an empty gate in a fresh disposable physical fixture. A finite
//! pending enrollment preserves provenance until the original self-covered recipe
//! selects both typed roots. Fresh generation enrollment remains a refusal.
//!
//! dev/ino are local disposable evidence, not portable-copy/rebind semantics.
//! Imported state keeps the existing explicit full-audit/read-only boundary.
//! The one last-credit receipt reconciles immediate final-selector cuts; older
//! maintenance retries may refuse after intervening work but never refund twice.
//! Authored Graph receipts keep their separate lifetime indexed retention.
#[allow(
    clippy::wildcard_imports,
    reason = "Actual-v3 opt-in accounting fixture"
)]
use super::*;
#[path = "retirement_ledger/relocation.rs"]
pub(in super::super) mod relocation;

const BOOTSTRAP_UNITS: usize = 64;
const CONTROL_FRAMES: u64 = 4;
const CONTROL_NAMESPACE: u64 = 8 * 4096;
const CONTROL_UNCERTAINTY: u64 = 65536;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct StandingControl {
    frame_cap: u64,
    simultaneous_frames: u64,
    charged: u64,
}
impl StandingControl {
    fn new() -> Self {
        Self {
            frame_cap: GATE_MAX as u64,
            simultaneous_frames: CONTROL_FRAMES,
            charged: CONTROL_FRAMES * round(GATE_MAX as u64 + 4096)
                + CONTROL_NAMESPACE
                + CONTROL_UNCERTAINTY,
        }
    }
    fn validate(&self) -> Result<()> {
        ensure(*self == Self::new(), "unknown standing-control capacity")
    }
    fn admits(&self, h: &Selection, gate: &GateState) -> Result<()> {
        self.validate()?;
        let mut prospective = h.clone();
        prospective.gate = gate.clone();
        prospective.epoch = u64::MAX;
        for domain in prospective.gate.domains.values_mut() {
            domain.charged = u64::MAX;
            domain.limit = u64::MAX;
        }
        ensure(
            serde_json::to_vec(&prospective).map_err(error)?.len() as u64 <= self.frame_cap,
            "prospective control frame exceeds standing capacity",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct AllocationCharge {
    domain: u64,
    data: bool,
    pack: u64,
    dev: u64,
    ino: u64,
    extent: u64,
    charged_high_water: u64,
}
impl AllocationCharge {
    fn capture(f: &Fixture, claim: &Claim) -> Result<Self> {
        let arena = if claim.data { &f.data } else { &f.meta };
        let m = fs::symlink_metadata(arena.path(claim.pack)).map_err(error)?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && m.dev() == claim.dev
                && m.ino() == claim.ino
                && m.len() == claim.extent
                && claim.content_end == claim.extent,
            "attribution requires captured complete allocation generation",
        )?;
        Ok(Self {
            domain: 0,
            data: claim.data,
            pack: claim.pack,
            dev: claim.dev,
            ino: claim.ino,
            extent: claim.extent,
            charged_high_water: m
                .blocks()
                .checked_mul(512)
                .ok_or("allocation charge overflow")?
                .max(m.len()),
        })
    }
    fn observe_growth(&self, f: &Fixture) -> Result<(Self, u64)> {
        let arena = if self.data { &f.data } else { &f.meta };
        ensure(
            self.pack == arena.pack,
            "growth attribution requires registered current tip",
        )?;
        let m = fs::symlink_metadata(arena.path(self.pack)).map_err(error)?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && m.dev() == self.dev
                && m.ino() == self.ino
                && m.len() >= self.extent
                && m.len() == arena.end
                && m.len() <= PACK,
            "attributed tip generation/extent changed",
        )?;
        let observed = m
            .blocks()
            .checked_mul(512)
            .ok_or("allocation charge overflow")?
            .max(m.len());
        let mut next = self.clone();
        next.extent = m.len();
        next.charged_high_water = self.charged_high_water.max(observed);
        let delta = next.charged_high_water - self.charged_high_water;
        Ok((next, delta))
    }
    fn matches(&self, claim: &Claim) -> bool {
        self.domain == 0
            && self.data == claim.data
            && self.pack == claim.pack
            && self.dev == claim.dev
            && self.ino == claim.ino
            && self.extent == claim.extent
            && self.charged_high_water >= claim.extent
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct Ledger {
    control: StandingControl,
    // Exactly the two growable arena generations. Lifetime sealed attribution
    // resides in typed ownership leaves, never in a flat lifetime HEAD manifest.
    tips: Vec<AllocationCharge>,
    // Bootstrap-only provenance until both typed roots own the sealed records.
    // Cleared atomically after installation; never a lifetime flat manifest.
    pending_enrollment: Vec<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<graph::SourceCharge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_credit: Option<CreditReceipt>,
    // Concurrent original-operation authorizations, capped by MAX_HOLDS.
    // Persisted before unlink so a missing path cannot manufacture credit.
    unlink_authorizations: BTreeMap<u64, UnlinkAuthorization>,
}
impl Ledger {
    pub(in super::super) fn admit_control(&self, h: &Selection, gate: &GateState) -> Result<()> {
        ensure(
            self.tips.len() == 2 && self.tips[0].data && !self.tips[1].data,
            "attributed tip registry shape",
        )?;
        ensure(
            self.pending_enrollment.len() <= BOOTSTRAP_UNITS
                && self.unlink_authorizations.len() <= MAX_HOLDS,
            "bounded accounting control state",
        )?;
        self.control.admits(h, gate)
    }
    fn observe_tips(&self, f: &Fixture) -> Result<(Self, u64)> {
        let mut next = self.clone();
        let mut delta = 0;
        for charge in &mut next.tips {
            let (observed, growth) = charge.observe_growth(f)?;
            *charge = observed;
            delta = checked(delta, growth)?;
        }
        Ok((next, delta))
    }
}

/// Build attributable bootstrap inputs from actual files. The caller must select
/// both these typed leaves and this ledger under the original bootstrap hold;
/// it may not install only the lower global number. Existing high-water modes
/// stay unchanged and cannot invoke this enrollment retroactively.
fn bootstrap(f: &mut Fixture) -> Result<(Ledger, Vec<Claim>, u64)> {
    ensure(
        f.head.gate.domains.is_empty()
            && f.head.gate.holds.is_empty()
            && f.head.gate.pins.is_empty(),
        "attributed bootstrap requires fresh empty gate",
    )?;
    let count = checked(checked(f.data.pack, f.meta.pack)?, 2)?;
    ensure(
        count <= BOOTSTRAP_UNITS as u64,
        "bounded attributable bootstrap",
    )?;
    let mut expected_names = HashSet::from([String::from("HEAD")]);
    let mut claims = vec![];
    let mut tips = vec![];
    let control = StandingControl::new();
    let mut charge = control.charged;
    for data in [true, false] {
        let (tip, prefix) = if data {
            (f.data.pack, f.data.prefix)
        } else {
            (f.meta.pack, f.meta.prefix)
        };
        for pack in 0..=tip {
            expected_names.insert(format!("{prefix}-{pack}"));
            let mut claim = Claim::capture(f, data, pack)?;
            let allocation = AllocationCharge::capture(f, &claim)?;
            charge = checked(charge, allocation.charged_high_water)?;
            if pack == tip {
                tips.push(allocation.clone());
            }
            claim.charge = claim.sealed.then_some(allocation);
            claims.push(claim);
        }
    }
    let mut actual_names = HashSet::new();
    for entry in fs::read_dir(&f.dir).map_err(error)? {
        let entry = entry.map_err(error)?;
        actual_names.insert(
            entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 allocation namespace")?,
        );
    }
    ensure(
        actual_names == expected_names,
        "unknown bootstrap allocation/control generation",
    )?;
    let ledger = Ledger {
        control,
        tips,
        pending_enrollment: claims.clone(),
        source: None,
        last_credit: None,
        unlink_authorizations: BTreeMap::new(),
    };
    Ok((ledger, claims, charge))
}

#[path = "retirement_ledger/commitment.rs"]
mod commitment;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct RetirementTicket {
    token: u64,
    allocation: AllocationCharge,
    source: Claim,
    selected_active: PRef,
    selected_recovery: PRef,
    selected_inventory: Inventory,
    // All-class pin snapshot is bound before effects. An unrelated mutation or
    // pin change during an uncertain unlink requires revalidation, not credit.
    data_base: recipe::Prefix,
    meta_base: recipe::Prefix,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "relocation::optional_plan_text"
    )]
    data: Option<WritePlan>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "relocation::optional_plan_text"
    )]
    meta: Option<WritePlan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compact: Option<commitment::Compact>,
    tip_claims: Vec<Claim>,
    control_base: Selection,
}
impl RetirementTicket {
    fn from_plan(
        f: &mut Fixture,
        token: u64,
        source: &Claim,
        plan: &TypedPlan,
        tip_claims: Vec<Claim>,
    ) -> Result<Self> {
        ensure(
            source.sealed && source.content_end == source.extent,
            "only whole sealed generation may transfer charge",
        )?;
        let allocation = source
            .charge
            .clone()
            .ok_or("no attributable sealed charge")?;
        ensure(
            allocation.matches(source),
            "sealed ownership/charge identity mismatch",
        )?;
        let next = &plan.continuation;
        let data_base = recipe::Prefix::capture(&f.data)?;
        let meta_base = recipe::Prefix::capture(&f.meta)?;
        f.c.recipe_prefix_read_bytes += f.data.end + f.meta.end;
        Ok(Self {
            token,
            allocation,
            source: source.clone(),
            selected_active: next.active.clone(),
            selected_recovery: next.recovery.clone(),
            selected_inventory: next.inventory.clone(),
            data_base,
            meta_base,
            data: Some(plan.data.clone()),
            meta: Some(plan.meta.clone()),
            compact: None,
            tip_claims,
            control_base: f.head.clone(),
        })
    }
    pub(in super::super) fn validate_mode(&self, target: &Continuation) -> Result<()> {
        match (&self.data, &self.meta, &self.compact) {
            (Some(data), Some(meta), None) => {
                recipe::segments(&self.data_base, data, true)?;
                recipe::segments(&self.meta_base, meta, false)?;
                ensure(
                    data.pack == target.data_pack
                        && data.end == target.data_end
                        && meta.pack == target.meta_pack
                        && meta.end == target.meta_end,
                    "full retirement target coordinates",
                )
            }
            (None, None, Some(proof)) => {
                proof.validate(&self.data_base, &self.meta_base)?;
                proof.validate_target(target)
            }
            _ => Err("retirement requires exactly one full or compact recipe mode".into()),
        }
    }
    fn full_plans(&self) -> Result<(&WritePlan, &WritePlan)> {
        ensure(
            self.compact.is_none(),
            "legacy release requires full recipe",
        )?;
        Ok((
            self.data.as_ref().ok_or("full data plan absent")?,
            self.meta.as_ref().ok_or("full metadata plan absent")?,
        ))
    }
    fn verify_completed_bytes(&self, f: &mut Fixture, exact: &Liability) -> Result<()> {
        let target = exact
            .continuation
            .as_ref()
            .ok_or("retirement continuation")?;
        self.validate_mode(target)?;
        if let Some(proof) = &self.compact {
            proof.verify_complete(f, &self.data_base, &self.meta_base, target)?;
        }
        Ok(())
    }
    fn validate_selected(&self, f: &Fixture, exact: &Liability) -> Result<()> {
        self.validate_mode(
            exact
                .continuation
                .as_ref()
                .ok_or("retirement continuation")?,
        )?;
        ensure(
            !f.imported_read_only
                && f.head.gate.holds.len() == 1
                && f.head.gate.holds.get(&self.token) == Some(exact)
                && exact.token == self.token
                && exact.retirement == Some((self.allocation.data, self.allocation.pack))
                && exact.retirement_ticket.as_ref() == Some(self)
                && f.head.active == self.selected_active
                && f.head.recovery == self.selected_recovery
                && f.head.inventory == self.selected_inventory
                && f.head.semantic == exact.target,
            "exact selected original retirement ticket required",
        )?;
        ensure(
            self.allocation.matches(&self.source) && self.source.sealed,
            "retirement attribution shape",
        )?;
        ensure(
            f.head.gate.ledger.is_some(),
            "attributed project ledger absent",
        )
    }
}

impl Builder {
    /// Persistent typed removal; both locator membership and the ownership tree
    /// lose precisely the removed path. Other ownership and semantic edges stay.
    fn remove(
        &mut self,
        f: &mut Fixture,
        locator: &PRef,
        at: Option<Ref>,
        key: u64,
    ) -> Result<Option<Ref>> {
        let Some(at) = at else {
            return Ok(None);
        };
        ensure(self.removed.len() < 64 * 65, "bounded typed removal path")?;
        match self.node(f, locator, &at)? {
            OwnedNode::Leaf { claim } => {
                if claim.key()? != key {
                    return Ok(Some(at));
                }
                claim.verify(f)?;
                self.removed.push(at);
                Ok(None)
            }
            OwnedNode::Branch {
                bit,
                anchor,
                left,
                right,
            } => {
                if difference(key, anchor) < bit {
                    return Ok(Some(at));
                }
                let (new_left, new_right) = if direction(key, bit) {
                    (
                        Some(left.clone()),
                        self.remove(f, locator, Some(right.clone()), key)?,
                    )
                } else {
                    (
                        self.remove(f, locator, Some(left.clone()), key)?,
                        Some(right.clone()),
                    )
                };
                if new_left.as_ref() == Some(&left) && new_right.as_ref() == Some(&right) {
                    return Ok(Some(at));
                }
                self.removed.push(at);
                match (new_left, new_right) {
                    (None, r) | (r, None) => Ok(r),
                    (Some(left), Some(right)) => {
                        let anchor = self
                            .node(f, locator, &left)?
                            .anchor()?
                            .min(self.node(f, locator, &right)?.anchor()?);
                        self.put(&OwnedNode::Branch {
                            bit,
                            anchor,
                            left,
                            right,
                        })
                        .map(Some)
                    }
                }
            }
        }
    }
}

/// Keep the source charge live in a ticket while removing both current root
/// ownership edges. This is only a plan: admission must bind it to the original
/// ticket before the data/meta plans are replayed. Existing pins are immutable.
fn plan_release(f: &mut Fixture, source: &Claim) -> Result<(TypedPlan, Vec<Claim>)> {
    let units = [(true, f.data.pack), (false, f.meta.pack)];
    let mut tips = units
        .into_iter()
        .map(|(data, pack)| Claim::capture(f, data, pack))
        .collect::<Result<Vec<_>>>()?;
    for _ in 0..16 {
        let p = plan_release_inputs(f, source, &tips)?;
        ensure(
            p.data.pack == f.data.pack && p.meta.pack == f.meta.pack,
            "release rollover requires enrolled fresh generation",
        )?;
        let mut changed = false;
        for tip in &mut tips {
            let end = if tip.data { p.data.end } else { p.meta.end };
            if tip.extent != end {
                tip.extent = end;
                changed = true;
            }
        }
        if !changed {
            return Ok((p, tips));
        }
    }
    Err("bounded release self-coverage fixed point".into())
}
fn plan_release_inputs(f: &mut Fixture, source: &Claim, tips: &[Claim]) -> Result<TypedPlan> {
    ensure(
        !f.imported_read_only && f.head.gate.holds.is_empty(),
        "release plan requires idle audited fixture",
    )?;
    ensure(
        source.sealed && source.charge.as_ref().is_some_and(|c| c.matches(source)),
        "release requires attributable sealed generation",
    )?;
    source.verify(f)?;
    for tip in tips {
        tip.verify_extent(f, true)?;
    }
    let h = f.head.clone();
    let key = source.key()?;
    let mut found = false;
    for root in [&h.active, &h.recovery] {
        if let Some(claim) = f.ownership_lookup(root, key)? {
            ensure(
                claim == *source,
                "current roots disagree on sealed attribution",
            )?;
            found = true;
        }
    }
    ensure(found, "ownership already released without original ticket")?;
    let mut a = Builder {
        data: WritePlan::from(&f.data),
        next: h.inventory.next,
        added: vec![],
        removed: vec![],
    };
    let mut active_inventory = a.remove(f, &h.active, h.inventory.active.clone(), key)?;
    for tip in tips {
        active_inventory = Some(a.insert(f, &h.active, active_inventory, tip)?);
    }
    let au = a.updates();
    let (data, next, recovery_inventory, bu) = if h.inventory.active == h.inventory.recovery {
        (a.data, a.next, active_inventory.clone(), au.clone())
    } else {
        let mut b = Builder {
            data: a.data,
            next: a.next,
            added: vec![],
            removed: vec![],
        };
        let mut recovery = b.remove(f, &h.recovery, h.inventory.recovery.clone(), key)?;
        for tip in tips {
            recovery = Some(b.insert(f, &h.recovery, recovery, tip)?);
        }
        let bu = b.updates();
        (b.data, b.next, recovery, bu)
    };
    f.staged = Some(vec![]);
    let staged = (|| -> Result<(PRef, PRef)> {
        let a = f
            .multi(Some(h.active.clone()), &au)?
            .ok_or("release active semantic locator")?;
        let b = f
            .multi(Some(h.recovery.clone()), &bu)?
            .ok_or("release recovery semantic locator")?;
        Ok((a, b))
    })();
    let pages = f.staged.take().ok_or("release metadata staging")?;
    let (active, recovery) = staged?;
    let mut meta = WritePlan::from(&f.meta);
    let mut memo = HashMap::new();
    let active = meta.staged(&active, &pages, &mut memo)?;
    let recovery = meta.staged(&recovery, &pages, &mut memo)?;
    let continuation = Continuation {
        recipe: None,
        inventory: Inventory {
            active: active_inventory,
            recovery: recovery_inventory,
            next,
        },
        active,
        recovery,
        data_pack: data.pack,
        data_end: data.end,
        meta_pack: meta.pack,
        meta_end: meta.end,
    };
    Ok(TypedPlan {
        data,
        meta,
        continuation,
    })
}

fn ensure_no_selected_placement(f: &mut Fixture, ticket: &RetirementTicket) -> Result<()> {
    let allocation = &ticket.allocation;
    f.inventory_retirement_guard(allocation.pack, allocation.data)?;
    let records = if allocation.data {
        f.data.records(allocation.pack, true)?
    } else {
        f.meta.records(allocation.pack, false)?
    };
    ensure(
        records.len() <= TX_OBJECTS,
        "bounded retirement liveness proof",
    )?;
    let mut roots = vec![f.head.active.clone(), f.head.recovery.clone()];
    for pin in f.head.gate.pins.values() {
        roots.extend([pin.active.clone(), pin.recovery.clone()]);
    }
    for (key, r, bytes) in &records {
        for root in &roots {
            let referenced = if allocation.data {
                f.lookup(root, key.ok_or("retirement data frame")?)?
                    .is_some_and(|location| location.data == *r)
            } else {
                let page: Page = serde_json::from_slice(bytes).map_err(error)?;
                f.path_contains(root, r, page.anchor())?
            };
            ensure(
                !referenced,
                "active/recovery/all-pin placement blocks retirement",
            )?;
        }
    }
    f.c.candidate_records += records.len() as u64;
    f.c.candidate_read_bytes += allocation.extent;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct UnlinkAuthorization {
    ticket_sha: String,
    pins_sha: String,
}
impl UnlinkAuthorization {
    fn for_ticket(f: &Fixture, ticket: &RetirementTicket) -> Result<Self> {
        Ok(Self {
            ticket_sha: hash(&serde_json::to_vec(ticket).map_err(error)?),
            pins_sha: hash(&serde_json::to_vec(&f.head.gate.pins).map_err(error)?),
        })
    }
}
#[derive(Clone, Copy, Debug)]
enum RetireCut {
    None,
    BeforeUnlink,
    AfterUnlink,
    AfterDirectoryBarrier,
}

/// Only this original selected ticket permits the namespace effect. Authorization
/// is separately selected after full candidate liveness/generation validation,
/// before unlink. A missing file without that authorization always fences.
#[allow(
    clippy::too_many_lines,
    reason = "Original selected authorization, exact unlink and absence barriers form one sequence"
)]
fn retire(f: &mut Fixture, exact: &Liability, cut: RetireCut) -> Result<u64> {
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("retirement ticket absent")?;
    ticket.validate_selected(f, exact)?;
    ticket.verify_completed_bytes(f, exact)?;
    validate_ticket_controls(f, exact)?;
    ensure(
        !f.dir.join("intent").exists(),
        "unknown control selection fences retirement",
    )?;
    let a = &ticket.allocation;
    let arena = if a.data { &f.data } else { &f.meta };
    ensure(a.pack < arena.pack, "growable allocation cannot retire")?;
    let path = arena.path(a.pack);
    let expected = UnlinkAuthorization::for_ticket(f, ticket)?;
    let authorization = f
        .head
        .gate
        .ledger
        .as_ref()
        .unwrap()
        .unlink_authorizations
        .get(&exact.token)
        .cloned();
    if let Some(authorization) = authorization {
        ensure(
            authorization == expected,
            "unknown selected unlink authorization",
        )?;
    } else {
        // NotFound here is unknown disappearance, not a recoverable own unlink.
        ticket.source.verify(f)?;
        ensure_no_selected_placement(f, ticket)?;
        let mut gate = f.head.gate.clone();
        let ledger = gate.ledger.as_mut().unwrap();
        ensure(
            ledger.unlink_authorizations.len() < MAX_HOLDS,
            "bounded unlink authorization set",
        )?;
        ledger
            .unlink_authorizations
            .insert(exact.token, expected.clone());
        f.select_gate(gate)?;
    }
    ticket.validate_selected(f, exact)?;
    ensure(
        f.head
            .gate
            .ledger
            .as_ref()
            .unwrap()
            .unlink_authorizations
            .get(&exact.token)
            == Some(&expected),
        "original unlink authorization not selected",
    )?;
    if matches!(cut, RetireCut::BeforeUnlink) {
        return Err("cut before ticket unlink".into());
    }
    match fs::symlink_metadata(&path) {
        Ok(m) => {
            ensure(
                m.is_file()
                    && m.nlink() == 1
                    && m.dev() == a.dev
                    && m.ino() == a.ino
                    && m.len() == a.extent,
                "retirement generation changed; ticket retained",
            )?;
            ticket.source.verify(f)?;
            ensure_no_selected_placement(f, ticket)?;
            // Recheck the namespace immediately before unlink. This fixture does
            // not claim hostile concurrent namespace protection or a writer lease.
            let fresh = fs::symlink_metadata(&path).map_err(error)?;
            ensure(
                fresh.dev() == a.dev
                    && fresh.ino() == a.ino
                    && fresh.nlink() == 1
                    && fresh.is_file()
                    && fresh.len() == a.extent,
                "retirement namespace changed",
            )?;
            fs::remove_file(&path).map_err(error)?;
            f.c.files_deleted += 1;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Only the durable original-token authorization admits this branch.
            // Roots and all pin snapshots remain the proved selected commitments.
            ticket.validate_selected(f, exact)?;
        }
        Err(e) => return Err(error(e)),
    }
    if matches!(cut, RetireCut::AfterUnlink) {
        return Err("cut after ticket unlink; charge retained".into());
    }
    sync_dir(&f.dir, &mut f.c)?;
    if matches!(cut, RetireCut::AfterDirectoryBarrier) {
        return Err("cut after directory barrier; charge retained".into());
    }
    ensure(
        matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound),
        "fresh post-barrier absence required",
    )?;
    ticket.validate_selected(f, exact)?;
    Ok(a.charged_high_water)
}

/// Consume the still-selected ticket and project charge in one HEAD publication.
/// No retained in-memory success flag is enough: repeat the directory barrier,
/// absence and exact selected authorization checks immediately before selecting.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CreditReceipt {
    token: u64,
    ticket_sha: String,
    project_credit: u64,
    growth: u64,
}
fn completion_selector(f: &Fixture, exact: &Liability) -> Result<(Selection, CreditReceipt)> {
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("completion ticket")?;
    let (_, mut selected) = recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;
    selected.epoch = checked(selected.epoch, 2)?; // authorization, then credit
    let ledger = selected.gate.ledger.as_ref().ok_or("completion ledger")?;
    let (mut ledger, growth) = ledger.observe_tips(f)?;
    ensure(
        growth <= exact.by_domain.get(&0).copied().unwrap_or(0),
        "retirement growth exceeds original reserve",
    )?;
    let receipt = CreditReceipt {
        token: exact.token,
        ticket_sha: hash(&serde_json::to_vec(ticket).map_err(error)?),
        project_credit: ticket.allocation.charged_high_water,
        growth,
    };
    // The reconstructed release ledger has not yet stored its authorization;
    // the actual currently selected ledger must have it until final publication.
    ledger.unlink_authorizations.remove(&exact.token);
    ledger.last_credit = Some(receipt.clone());
    selected.gate.ledger = Some(ledger);
    selected.gate.holds.remove(&exact.token);
    let domain = selected
        .gate
        .domains
        .get_mut(&ticket.allocation.domain)
        .ok_or("retirement domain")?;
    domain.charged = checked(domain.charged, growth)?
        .checked_sub(receipt.project_credit)
        .ok_or("unattributable retirement credit")?;
    selected
        .gate
        .ledger
        .as_ref()
        .unwrap()
        .admit_control(&selected, &selected.gate)?;
    Ok((selected, receipt))
}
fn complete_retirement(f: &mut Fixture, exact: &Liability) -> Result<serde_json::Value> {
    complete_retirement_cut(f, exact, Cut::None)
}
fn complete_retirement_cut(
    f: &mut Fixture,
    exact: &Liability,
    cut: Cut,
) -> Result<serde_json::Value> {
    exact
        .retirement_ticket
        .as_ref()
        .ok_or("completion ticket")?
        .verify_completed_bytes(f, exact)?;
    if !f.head.gate.holds.contains_key(&exact.token) {
        let (expected, receipt) = completion_selector(f, exact)?;
        let disk: Selection = Fixture::bounded_read(&f.dir.join("HEAD"))?;
        ensure(
            f.head == expected && disk == expected,
            "retirement token absent without exact completion selector",
        )?;
        if f.dir.join("intent").exists() {
            let old: Selection = Fixture::bounded_read(&f.dir.join("intent"))?;
            let candidate: Selection = Fixture::bounded_read(&f.dir.join("candidate"))?;
            let ticket = exact.retirement_ticket.as_ref().unwrap();
            let (_, mut authorized) =
                recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;
            authorized.epoch = checked(authorized.epoch, 1)?;
            authorized
                .gate
                .ledger
                .as_mut()
                .unwrap()
                .unlink_authorizations
                .insert(exact.token, UnlinkAuthorization::for_ticket(f, ticket)?);
            ensure(
                old == authorized && candidate == expected,
                "unknown credit selection dispatch",
            )?;
            f.reconcile()?;
        }
        return Ok(
            json!({"original_token":exact.token,"already_consumed":true,"original_project_credit":receipt.project_credit,"project_retirement_credit":0,"filesystem_availability_credit":0}),
        );
    }
    if f.dir.join("intent").exists() {
        let (expected, _) = completion_selector(f, exact)?;
        let old: Selection = Fixture::bounded_read(&f.dir.join("intent"))?;
        let candidate: Selection = Fixture::bounded_read(&f.dir.join("candidate"))?;
        let ticket = exact.retirement_ticket.as_ref().unwrap();
        let (_, mut authorized) =
            recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;
        authorized.epoch = checked(authorized.epoch, 1)?;
        authorized
            .gate
            .ledger
            .as_mut()
            .unwrap()
            .unlink_authorizations
            .insert(exact.token, UnlinkAuthorization::for_ticket(f, ticket)?);
        ensure(
            old == authorized && candidate == expected && f.head == old,
            "unknown pending credit selection",
        )?;
        f.reconcile()?;
    }
    let credit = retire(f, exact, RetireCut::None)?;
    let (next, receipt) = completion_selector(f, exact)?;
    ensure(receipt.project_credit == credit, "credit proof mismatch")?;
    if cut == Cut::None {
        f.select_gate(next.gate.clone())?;
    } else {
        f.desired_gate = Some(next.gate.clone());
        let result = f.publish(
            next.active.clone(),
            next.recovery.clone(),
            next.semantic.clone(),
            cut,
        );
        if result.is_err() {
            f.desired_gate = None;
        }
        result?;
    }
    ensure(
        f.head == next,
        "credited selector differs from original settlement",
    )?;
    Ok(
        json!({"original_token":exact.token,"already_consumed":false,"observed_tip_growth":receipt.growth,"project_retirement_credit":credit,"filesystem_availability_credit":0}),
    )
}

pub(in super::super) fn complete_growth(f: &mut Fixture, exact: &Liability) -> Result<()> {
    ensure(
        f.head.gate.holds.len() == 1
            && f.head.gate.holds.get(&exact.token) == Some(exact)
            && exact.retirement.is_none()
            && exact.graph.is_none()
            && exact.birth.is_none(),
        "original growth hold",
    )?;
    let (mut ledger, growth) = f
        .head
        .gate
        .ledger
        .as_ref()
        .ok_or("attributed ledger")?
        .observe_tips(f)?;
    ensure(
        growth <= exact.by_domain.get(&0).copied().unwrap_or(0),
        "observed registered growth exceeds admission",
    )?;
    let mut all_installed = true;
    for claim in &ledger.pending_enrollment {
        for root in [f.head.active.clone(), f.head.recovery.clone()] {
            let selected = f.ownership_lookup(&root, claim.key()?)?;
            all_installed &= selected.as_ref().is_some_and(|c| {
                if claim.sealed {
                    c == claim
                } else {
                    c.data == claim.data
                        && c.pack == claim.pack
                        && c.dev == claim.dev
                        && c.ino == claim.ino
                        && c.content_end >= claim.content_end
                        && c.extent >= claim.extent
                        && c.charge.is_none()
                }
            });
        }
    }
    if all_installed {
        ledger.pending_enrollment.clear();
    }
    let mut gate = f.head.gate.clone();
    gate.holds.remove(&exact.token);
    gate.ledger = Some(ledger);
    let domain = gate.domains.get_mut(&0).ok_or("project domain")?;
    domain.charged = checked(domain.charged, growth)?;
    f.select_gate(gate)
}
pub(in super::super) fn initialize(f: &mut Fixture) -> Result<()> {
    initialize_cut(f, recipe::Fault::None)
}
fn initialize_cut(f: &mut Fixture, cut: recipe::Fault) -> Result<()> {
    if f.head.gate.ledger.is_none() {
        let (ledger, _claims, charged) = bootstrap(f)?;
        let mut gate = f.head.gate.clone();
        gate.domains.insert(
            0,
            CapacityDomain {
                limit: 64 * 1024 * 1024 * 1024,
                charged,
            },
        );
        gate.ledger = Some(ledger);
        f.select_gate(gate)?;
    }
    if f.head
        .gate
        .ledger
        .as_ref()
        .unwrap()
        .pending_enrollment
        .is_empty()
    {
        return Ok(());
    }
    let original_claims = f
        .head
        .gate
        .ledger
        .as_ref()
        .unwrap()
        .pending_enrollment
        .clone();
    for claim in &original_claims {
        claim.verify(f)?;
    }
    let hold = if let Some(hold) = f.head.gate.holds.values().next() {
        ensure(
            f.head.gate.holds.len() == 1
                && hold
                    .continuation
                    .as_ref()
                    .is_some_and(|c| c.recipe.is_some()),
            "unknown pending enrollment operation",
        )?;
        hold.clone()
    } else {
        recipe::prepare_self_covered(f)?.0
    };
    recipe::resume(f, &hold, cut)?;
    ensure(
        f.head
            .gate
            .ledger
            .as_ref()
            .unwrap()
            .pending_enrollment
            .is_empty(),
        "enrollment transfer incomplete",
    )
}
fn ensure_unpinned(f: &mut Fixture, source: &Claim) -> Result<()> {
    let key = source.key()?;
    let mut roots = vec![];
    for pin in f.head.gate.pins.values() {
        roots.extend([pin.active.clone(), pin.recovery.clone()]);
    }
    for root in &roots {
        ensure(
            f.ownership_lookup(root, key)?.is_none(),
            "pinned ownership refuses retirement admission",
        )?;
    }
    if roots.is_empty() {
        return Ok(());
    }
    let records = if source.data {
        f.data.records(source.pack, true)?
    } else {
        f.meta.records(source.pack, false)?
    };
    ensure(records.len() <= TX_OBJECTS, "bounded pin admission probe")?;
    for (key, physical, bytes) in records {
        for root in &roots {
            let live = if source.data {
                f.lookup(root, key.ok_or("pin data frame")?)?
                    .is_some_and(|l| l.data == physical)
            } else {
                let p: Page = serde_json::from_slice(&bytes).map_err(error)?;
                f.path_contains(root, &physical, p.anchor())?
            };
            ensure(!live, "pinned placement refuses retirement admission")?;
        }
    }
    Ok(())
}
fn prepare_retirement(f: &mut Fixture, source: &Claim) -> Result<Liability> {
    ensure(
        f.head
            .gate
            .ledger
            .as_ref()
            .is_some_and(|l| l.pending_enrollment.is_empty()),
        "retirement requires completed attributable enrollment",
    )?;
    ensure_unpinned(f, source)?;
    let (plan, tips) = plan_release(f, source)?;
    let token = checked(f.head.gate.last_token, 1)?;
    let ticket = RetirementTicket::from_plan(f, token, source, &plan, tips)?;
    ensure(
        serde_json::to_vec(&ticket).map_err(error)?.len() <= 48 * 1024,
        "retirement recipe byte cap",
    )?;
    let physical = physical_bound(Some(&plan.data), &plan.meta)?;
    let budget = checked(
        CONTROL,
        physical["allocation_bound"]
            .as_u64()
            .ok_or("retirement physical bound")?,
    )?;
    f.reserve_bound(
        f.head.semantic.clone(),
        Some((source.data, source.pack)),
        &[(0, budget)],
        Some(plan.continuation),
        None,
        Some(ticket),
    )
}
#[derive(Clone, Copy)]
enum ReleaseCut {
    None,
    Partial(usize),
    BeforeHead,
    AfterHead,
}
fn publish_release(f: &mut Fixture, exact: &Liability, cut: ReleaseCut) -> Result<()> {
    ensure(
        !f.imported_read_only
            && f.head.gate.holds.len() == 1
            && f.head.gate.holds.get(&exact.token) == Some(exact),
        "exact original release hold",
    )?;
    let ticket = exact.retirement_ticket.as_ref().ok_or("release ticket")?;
    let c = exact.continuation.as_ref().ok_or("release continuation")?;
    validate_ticket_controls(f, exact)?;
    if f.head.active == c.active && f.head.recovery == c.recovery && f.head.inventory == c.inventory
    {
        if f.dir.join("intent").exists() {
            f.reconcile()?;
        }
        return Ok(());
    }
    // Regenerate the exact typed removal/update recipe against the original
    // selected roots, without adopting a fresh token or expanded write plan.
    let saved = f.head.clone();
    let arena = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    f.head.active = exact.origin.active.clone();
    f.head.recovery = exact.origin.recovery.clone();
    f.head.inventory = exact.origin.inventory.clone();
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
    let regenerated = plan_release_inputs(f, &ticket.source, &ticket.tip_claims);
    f.head = saved;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = arena;
    f.cache.clear();
    f.staged = None;
    let regenerated = regenerated?;
    ensure(
        regenerated.continuation == *c
            && regenerated.data == *ticket.full_plans()?.0
            && regenerated.meta == *ticket.full_plans()?.1,
        "release differs from original typed plan",
    )?;
    let mut remaining = if let ReleaseCut::Partial(n) = cut {
        Some(n)
    } else {
        None
    };
    recipe::replay(
        f,
        true,
        &ticket.data_base,
        ticket.full_plans()?.0,
        &mut remaining,
    )?;
    recipe::replay(
        f,
        false,
        &ticket.meta_base,
        ticket.full_plans()?.1,
        &mut remaining,
    )?;
    f.authorized_hold = Some(exact.token);
    if f.dir.join("intent").exists() {
        f.reconcile()?;
    }
    f.publish(
        c.active.clone(),
        c.recovery.clone(),
        exact.target.clone(),
        match cut {
            ReleaseCut::BeforeHead => Cut::BeforeHead,
            ReleaseCut::AfterHead => Cut::AfterHead,
            _ => Cut::None,
        },
    )?;
    f.audit_combined()?;
    Ok(())
}

fn setup_attributed(
    n: u64,
    kind: Kind,
    dead_records: u64,
) -> Result<(tempfile::TempDir, Fixture, Claim)> {
    let (temp, mut source, _legacy) = setup(n, kind)?;
    let mut f = Fixture::create(&temp.path().join("attributed"), &mut source)?;
    // A real framed but unreferenced sealed metadata generation. This first
    // integration retires already-dead placements; relocating live records
    // into new generations remains a separate original-plan integration.
    f.meta.rotate(&mut f.c)?;
    let dead_pack = f.meta.pack;
    let page = serde_json::to_vec(&f.get(&f.head.active.clone())?).map_err(error)?;
    for _ in 0..dead_records {
        f.meta.append(&page, None, &mut f.c)?;
    }
    f.meta.rotate(&mut f.c)?;
    f.meta.sync(&mut f.c)?;
    f.publish(
        f.head.active.clone(),
        f.head.recovery.clone(),
        f.head.semantic.clone(),
        Cut::None,
    )?;
    initialize(&mut f)?;
    let claim = f
        .ownership_lookup(&f.head.active.clone(), dead_pack * 2)?
        .ok_or("attributed dead allocation")?;
    f.audit_combined()?;
    Ok((temp, f, claim))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup_ledger(kind: Kind) -> Result<(tempfile::TempDir, Fixture, Claim)> {
        setup_attributed(64, kind, 64)
    }
    #[test]
    fn ledger_pending_enrollment_cannot_recapture_corrupted_original_prefix() -> Result<()> {
        let (temp, mut source, _legacy) = setup(64, Kind::Btree)?;
        let mut f = Fixture::create(&temp.path().join("enrollment-corruption"), &mut source)?;
        let (ledger, _, charged) = bootstrap(&mut f)?;
        let mut gate = f.head.gate.clone();
        gate.domains.insert(
            0,
            CapacityDomain {
                limit: 64 * 1024 * 1024 * 1024,
                charged,
            },
        );
        gate.ledger = Some(ledger);
        f.select_gate(gate)?;
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.data.path(0))
            .map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        f = Fixture::reopen_combined(&f.dir)?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            initialize(&mut f)
                .unwrap_err()
                .contains("owned prefix corruption"),
            "corrupt enrollment prefix recaptured",
        )?;
        ensure(
            f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && f.head.gate.holds.is_empty(),
            "corrupt enrollment changed original charge/evidence",
        )
    }
    #[test]
    fn ledger_bootstrap_recovers_original_self_covered_recipe() -> Result<()> {
        for cut in [
            recipe::Fault::Partial(7),
            recipe::Fault::BeforeHead,
            recipe::Fault::AfterHead,
        ] {
            let (temp, mut source, _legacy) = setup(64, Kind::Btree)?;
            let mut f = Fixture::create(&temp.path().join("enrollment"), &mut source)?;
            ensure(initialize_cut(&mut f, cut).is_err(), "enrollment cut")?;
            let token = f
                .head
                .gate
                .holds
                .keys()
                .next()
                .copied()
                .ok_or("original bootstrap hold")?;
            f = Fixture::reopen_combined(&f.dir)?;
            initialize(&mut f)?;
            ensure(
                f.head.gate.last_token == token
                    && f.head.gate.holds.is_empty()
                    && f.head
                        .gate
                        .ledger
                        .as_ref()
                        .unwrap()
                        .pending_enrollment
                        .is_empty(),
                "bootstrap allocated replacement token or lost provenance",
            )?;
            for (data, pack, end) in [
                (true, f.data.pack, f.data.end),
                (false, f.meta.pack, f.meta.end),
            ] {
                for root in [f.head.active.clone(), f.head.recovery.clone()] {
                    let c = f
                        .ownership_lookup(&root, pack * 2 + u64::from(data))?
                        .ok_or("selected tip claim")?;
                    ensure(
                        c.extent == end && c.charge.is_none(),
                        "bootstrap left stale tip ownership",
                    )?;
                }
            }
            f.audit_combined()?;
        }
        Ok(())
    }
    #[test]
    fn ledger_credit_selector_cuts_are_exact_and_once_only() -> Result<()> {
        for cut in [Cut::BeforeHead, Cut::AfterHead] {
            let (_temp, mut f, claim) = setup_ledger(Kind::Btree)?;
            let hold = prepare_retirement(&mut f, &claim)?;
            publish_release(&mut f, &hold, ReleaseCut::None)?;
            ensure(
                complete_retirement_cut(&mut f, &hold, cut).is_err(),
                "credit selector cut",
            )?;
            f = Fixture::reopen_combined(&f.dir)?;
            complete_retirement(&mut f, &hold)?;
            let settled = f.head.clone();
            ensure(
                complete_retirement(&mut f, &hold)?["already_consumed"] == true
                    && f.head == settled,
                "credit duplicated after selector cut",
            )?;
            f.audit_combined()?;
        }
        Ok(())
    }
    #[test]
    fn ledger_unknown_control_and_other_hold_fence_retirement() -> Result<()> {
        let (_temp, mut f, claim) = setup_ledger(Kind::Radix)?;
        let hold = prepare_retirement(&mut f, &claim)?;
        ensure(
            publish_release(&mut f, &hold, ReleaseCut::BeforeHead).is_err(),
            "release selector cut",
        )?;
        let path = f.dir.join("candidate");
        let original = fs::read(&path).map_err(error)?;
        let mut candidate: Selection = serde_json::from_slice(&original).map_err(error)?;
        candidate.gate.domains.get_mut(&0).unwrap().charged += 1;
        fs::write(&path, serde_json::to_vec(&candidate).map_err(error)?).map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            publish_release(&mut f, &hold, ReleaseCut::None).is_err() && f.snapshot()? == before,
            "unknown non-root control replayed",
        )?;
        fs::write(&path, original).map_err(error)?;
        publish_release(&mut f, &hold, ReleaseCut::None)?;
        let mut gate = f.head.gate.clone();
        let mut other = hold.clone();
        other.token = gate.token()?;
        gate.holds.insert(other.token, other);
        f.select_gate(gate)?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            complete_retirement(&mut f, &hold).is_err()
                && f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
            "unrelated original hold failed to fence",
        )
    }
    #[test]
    fn ledger_standing_control_turnover_never_accumulates_charge() -> Result<()> {
        let (_temp, mut f, _) = setup_ledger(Kind::Btree)?;
        let charged = f.head.gate.domains[&0].charged;
        for _ in 0..64 {
            let pin = f.acquire(PinClass::ReaderLease, false)?;
            f.release(&pin, true, true, true)?;
            ensure(
                f.head.gate.domains[&0].charged == charged,
                "control turnover accumulated charge",
            )?;
        }
        f = Fixture::reopen_combined(&f.dir)?;
        ensure(
            f.head.gate.domains[&0].charged == charged,
            "standing charge changed on reopen",
        )?;
        f.audit_combined()?;
        Ok(())
    }
    #[test]
    fn ledger_all_extra_pins_refuse_before_retirement_hold_or_effects() -> Result<()> {
        let (_temp, mut f, claim) = setup_ledger(Kind::Radix)?;
        for class in EXTRA_CLASSES {
            let pin = f.acquire(class, class == PinClass::UnresolvedIntent)?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                prepare_retirement(&mut f, &claim).is_err(),
                "pinned retirement admitted",
            )?;
            ensure(
                f.snapshot()? == before
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && f.head.gate.holds.is_empty(),
                "pin refusal left effects or stuck hold",
            )?;
            f.release(&pin, true, true, true)?;
        }
        Ok(())
    }
    #[test]
    fn ledger_original_release_and_unlink_cuts_credit_exactly_once() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            for (release_cut, retire_cut) in [
                (ReleaseCut::Partial(7), RetireCut::BeforeUnlink),
                (ReleaseCut::BeforeHead, RetireCut::AfterUnlink),
                (ReleaseCut::AfterHead, RetireCut::AfterDirectoryBarrier),
            ] {
                let (_temp, mut f, claim) = setup_ledger(kind)?;
                let expected_credit = claim.charge.as_ref().unwrap().charged_high_water;
                let hold = prepare_retirement(&mut f, &claim)?;
                let original_charged = f.head.gate.domains[&0].charged;
                ensure(
                    publish_release(&mut f, &hold, release_cut).is_err(),
                    "release cut",
                )?;
                f = Fixture::reopen_combined(&f.dir)?;
                publish_release(&mut f, &hold, ReleaseCut::None)?;
                for root in [f.head.active.clone(), f.head.recovery.clone()] {
                    ensure(
                        f.ownership_lookup(&root, claim.key()?)?.is_none(),
                        "source retention not released",
                    )?;
                }
                ensure(
                    f.head.gate.domains[&0].charged == original_charged,
                    "release speculated credit",
                )?;
                ensure(retire(&mut f, &hold, retire_cut).is_err(), "unlink cut")?;
                ensure(
                    f.head.gate.domains[&0].charged == original_charged,
                    "failure credited charge",
                )?;
                f = Fixture::reopen_combined(&f.dir)?;
                publish_release(&mut f, &hold, ReleaseCut::None)?;
                let result = complete_retirement(&mut f, &hold)?;
                let growth = result["observed_tip_growth"].as_u64().unwrap();
                ensure(
                    result["project_retirement_credit"] == expected_credit
                        && result["filesystem_availability_credit"] == 0,
                    "wrong credit domain/value",
                )?;
                ensure(
                    f.head.gate.domains[&0].charged == original_charged + growth - expected_credit,
                    "charge transfer not exact",
                )?;
                let after = f.head.clone();
                ensure(
                    complete_retirement(&mut f, &hold)?["already_consumed"] == true
                        && f.head == after,
                    "duplicate project credit",
                )?;
                f.audit_combined()?;
            }
        }
        Ok(())
    }
    #[test]
    fn ledger_absence_without_selected_authorization_and_replaced_inode_keep_charge() -> Result<()>
    {
        for replaced in [false, true] {
            let (_temp, mut f, claim) = setup_ledger(Kind::Btree)?;
            let hold = prepare_retirement(&mut f, &claim)?;
            publish_release(&mut f, &hold, ReleaseCut::None)?;
            let path = f.meta.path(claim.pack);
            let original = f.dir.join("unknown-original");
            fs::rename(&path, &original).map_err(error)?;
            if replaced {
                fs::copy(&original, &path).map_err(error)?;
            }
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            let before = f.snapshot()?;
            ensure(
                complete_retirement(&mut f, &hold).is_err(),
                "unknown missing/replaced allocation credited",
            )?;
            ensure(
                f.snapshot()? == before
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && f.head.gate.holds.get(&hold.token) == Some(&hold),
                "uncertain retirement lost original evidence",
            )?;
        }
        Ok(())
    }
}

impl Ledger {
    pub(in super::super) fn enrollment_complete(&self) -> bool {
        self.pending_enrollment.is_empty()
    }
    pub(super) fn enrollment_charge(&self, data: bool, pack: u64) -> Option<AllocationCharge> {
        self.pending_enrollment
            .iter()
            .find(|c| c.data == data && c.pack == pack)
            .and_then(|c| c.charge.clone())
    }
    pub(in super::super) fn has_source(&self) -> bool {
        self.source.is_some()
    }
}
pub(in super::super) fn source_charge(f: &Fixture) -> Result<graph::SourceCharge> {
    f.head
        .gate
        .ledger
        .as_ref()
        .and_then(|l| l.source.clone())
        .ok_or("attributed source absent".into())
}
pub(in super::super) fn enroll_source(f: &mut Fixture, source: &Store) -> Result<()> {
    ensure(
        !f.imported_read_only && f.head.gate.holds.is_empty(),
        "source enrollment requires idle audited fixture",
    )?;
    let mut gate = f.head.gate.clone();
    let ledger = gate.ledger.as_mut().ok_or("attributed ledger absent")?;
    ensure(
        ledger.pending_enrollment.is_empty() && ledger.source.is_none(),
        "source already enrolled or bootstrap incomplete",
    )?;
    let charge = graph::SourceCharge::capture(source)?;
    let domain = gate.domains.get_mut(&0).ok_or("source charge domain")?;
    domain.charged = checked(domain.charged, charge.charge()?)?;
    ledger.source = Some(charge);
    f.select_gate(gate)
}
pub(in super::super) fn graph_completion_gate(
    f: &mut Fixture,
    exact: &Liability,
    source: &Store,
) -> Result<GateState> {
    ensure(
        !f.imported_read_only
            && f.head.gate.holds.len() == 1
            && f.head.gate.holds.get(&exact.token) == Some(exact)
            && exact.graph.is_some()
            && exact.retirement.is_none()
            && exact.retirement_ticket.is_none()
            && f.head.semantic == exact.target
            && exact.continuation.as_ref().is_some_and(|c| {
                f.head.active == c.active
                    && f.head.recovery == c.recovery
                    && f.head.inventory == c.inventory
            }),
        "original attributed Graph hold",
    )?;
    let original = exact
        .graph
        .as_ref()
        .and_then(|b| b.source_charge.as_ref())
        .ok_or("original Graph source commitment")?;
    let registered = source_charge(f)?;
    ensure(
        registered == *original,
        "source differs from original Graph admission",
    )?;
    let (next_source, source_growth) = registered.observe_growth(source)?;
    let (mut ledger, tip_growth) = f.head.gate.ledger.as_ref().unwrap().observe_tips(f)?;
    let growth = checked(tip_growth, source_growth)?;
    ensure(
        growth <= exact.by_domain.get(&0).copied().unwrap_or(0),
        "attributed Graph growth exceeds original admission",
    )?;
    ledger.source = Some(next_source);
    let mut gate = f.head.gate.clone();
    gate.holds.remove(&exact.token);
    gate.ledger = Some(ledger);
    let domain = gate.domains.get_mut(&0).ok_or("Graph charge domain")?;
    domain.charged = checked(domain.charged, growth)?;
    Ok(gate)
}

pub(in super::super) fn registered_source(
    f: &Fixture,
    source: &Store,
) -> Result<graph::SourceCharge> {
    let registered = source_charge(f)?;
    let (observed, _) = registered.observe_growth(source)?;
    ensure(
        observed.extent == registered.extent,
        "unsettled retained source extent before admission",
    )?;
    Ok(registered)
}

fn control_present(f: &Fixture, role: &str) -> Result<bool> {
    match fs::symlink_metadata(f.dir.join(role)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}
fn validate_ticket_controls(f: &Fixture, exact: &Liability) -> Result<()> {
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("ticket control identity")?;
    ticket.validate_mode(
        exact
            .continuation
            .as_ref()
            .ok_or("retirement continuation")?,
    )?;
    let (old, released) = recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;
    let mut authorized = released.clone();
    authorized.epoch = checked(authorized.epoch, 1)?;
    authorized
        .gate
        .ledger
        .as_mut()
        .ok_or("ticket control ledger")?
        .unlink_authorizations
        .insert(exact.token, UnlinkAuthorization::for_ticket(f, ticket)?);
    let known = [&old, &released, &authorized];
    let read = |name: &str| -> Result<Selection> {
        let path = f.dir.join(name);
        let m = fs::symlink_metadata(&path).map_err(error)?;
        ensure(
            m.is_file() && m.nlink() == 1 && m.len() <= GATE_MAX as u64,
            "private bounded retirement control",
        )?;
        Fixture::bounded_read(&path)
    };
    let selected = read("HEAD")?;
    ensure(
        selected == f.head && known.iter().any(|h| **h == selected),
        "unknown retirement control selector",
    )?;
    if control_present(f, "intent")? || control_present(f, "candidate")? {
        let intent = read("intent")?;
        let candidate = read("candidate")?;
        ensure(
            ((intent == old && candidate == released)
                || (intent == released && candidate == authorized))
                && (selected == intent || selected == candidate),
            "unknown retirement control dispatch",
        )?;
    }
    ensure(
        !control_present(f, "HEAD.next")?,
        "unfinished retirement selector staging fences effects",
    )
}

/// Exhaustive fixture audit: every unit is counted once across current/pinned
/// ownership, growable tips, original retirement tickets and finite enrollment.
/// This is deliberately not an ordinary-open or checkpoint lifetime scan.
#[allow(
    clippy::too_many_lines,
    reason = "Keep exact allocation union and held-growth provenance audit in one pass"
)]
pub(in super::super) fn audit_attribution(f: &mut Fixture) -> Result<()> {
    let Some(ledger) = f.head.gate.ledger.clone() else {
        return Ok(());
    };
    ledger.admit_control(&f.head, &f.head.gate)?;
    let mut units = BTreeMap::<(bool, u64), AllocationCharge>::new();
    let insert = |units: &mut BTreeMap<(bool, u64), AllocationCharge>,
                  charge: AllocationCharge|
     -> Result<()> {
        ensure(
            charge.domain == 0
                && charge.extent <= PACK
                && charge.charged_high_water >= charge.extent,
            "registered allocation shape",
        )?;
        if let Some(old) = units.insert((charge.data, charge.pack), charge.clone()) {
            ensure(old == charge, "conflicting authoritative generation charge")?;
        }
        Ok(())
    };
    for charge in &ledger.tips {
        insert(&mut units, charge.clone())?;
    }
    for claim in &ledger.pending_enrollment {
        if claim.sealed {
            insert(
                &mut units,
                claim.charge.clone().ok_or("unenrolled sealed charge")?,
            )?;
        }
    }
    let mut roots = vec![f.head.active.clone(), f.head.recovery.clone()];
    for pin in f.head.gate.pins.values() {
        roots.extend([pin.active.clone(), pin.recovery.clone()]);
    }
    let mut historical_growable = vec![];
    for root in roots {
        for r in f.audit_inventory(&root)? {
            if let OwnedNode::Leaf { claim } = f.owned_node(&root, &r)? {
                if claim.sealed {
                    let charge = claim
                        .charge
                        .clone()
                        .ok_or("sealed generation lacks registered charge")?;
                    ensure(charge.matches(&claim), "sealed charge/claim mismatch")?;
                    insert(&mut units, charge)?;
                } else {
                    ensure(
                        claim.charge.is_none(),
                        "growable claim carries competing charge",
                    )?;
                    historical_growable.push(claim);
                }
            }
        }
    }
    for hold in f.head.gate.holds.values() {
        if let Some(ticket) = &hold.retirement_ticket {
            ensure(
                ticket.token == hold.token && ticket.allocation.matches(&ticket.source),
                "retirement ticket charge identity",
            )?;
            insert(&mut units, ticket.allocation.clone())?;
        }
    }
    for claim in historical_growable {
        let canonical = units.get(&(claim.data, claim.pack));
        let identity = canonical.is_some_and(|c| c.dev == claim.dev && c.ino == claim.ino);
        let already_covered = canonical.is_some_and(|c| c.extent >= claim.extent);
        let current_tip = ledger.tips.iter().any(|t| {
            t.data == claim.data && t.pack == claim.pack && t.dev == claim.dev && t.ino == claim.ino
        });
        let mut covered_growth = false;
        if identity && !already_covered && current_tip {
            let holds = f.head.gate.holds.values().cloned().collect::<Vec<_>>();
            for h in holds {
                let Some(c) = &h.continuation else {
                    continue;
                };
                let (pack, end) = if claim.data {
                    (c.data_pack, c.data_end)
                } else {
                    (c.meta_pack, c.meta_end)
                };
                if c.active != f.head.active
                    || c.recovery != f.head.recovery
                    || c.inventory != f.head.inventory
                    || pack != claim.pack
                    || end < claim.extent
                {
                    continue;
                }
                if let Some(ticket) = &h.retirement_ticket {
                    ticket.validate_selected(f, &h)?;
                    validate_ticket_controls(f, &h)?;
                } else if let Some(binding) = &h.graph {
                    recipe::verify_controls(f, &h, binding.control_base())?;
                } else if let Some(original) = &c.recipe {
                    recipe::verify_controls(f, &h, &original.control_base)?;
                } else {
                    continue;
                }
                let (_, growth) = ledger.observe_tips(f)?;
                ensure(
                    growth
                        <= h.by_domain
                            .get(&0)
                            .copied()
                            .unwrap_or(0)
                            .saturating_sub(h.control),
                    "selected tip growth exceeds original allocation reserve",
                )?;
                covered_growth = true;
            }
        }
        ensure(
            identity && (already_covered || covered_growth),
            "historical growable claim has no canonical allocation charge",
        )?;
    }
    let mut total = ledger.control.charged;
    if let Some(source) = &ledger.source {
        total = checked(total, source.charge()?)?;
    }
    for charge in units.values() {
        total = checked(total, charge.charged_high_water)?;
    }
    ensure(
        f.head
            .gate
            .domains
            .get(&0)
            .is_some_and(|d| d.charged == total),
        "project charge differs from exact attributable units and standing pools",
    )
}

impl Ledger {
    pub(in super::super) fn validate(&self, gate: &GateState) -> Result<()> {
        self.control.validate()?;
        ensure(
            self.tips.len() == 2
                && self.tips[0].data
                && !self.tips[1].data
                && self
                    .tips
                    .iter()
                    .all(|t| t.domain == 0 && t.extent <= PACK && t.charged_high_water >= t.extent)
                && self.pending_enrollment.len() <= BOOTSTRAP_UNITS
                && self.unlink_authorizations.len() <= MAX_HOLDS,
            "bounded registered ledger shape",
        )?;
        if let Some(source) = &self.source {
            source.charge()?;
        }
        for (token, authorization) in &self.unlink_authorizations {
            let ticket = gate
                .holds
                .get(token)
                .and_then(|h| h.retirement_ticket.as_ref())
                .ok_or("unlink authorization without original ticket")?;
            ensure(
                ticket.token == *token
                    && authorization.ticket_sha
                        == hash(&serde_json::to_vec(ticket).map_err(error)?),
                "unlink authorization identity",
            )?;
        }
        Ok(())
    }
}

pub(super) fn run_cli(args: &[String]) -> Result<()> {
    let n = args
        .first()
        .map_or(Ok(64), |s| s.parse::<u64>().map_err(error))?;
    ensure((1..=128).contains(&n), "bounded attributed fixture size")?;
    let dead_records = args
        .get(1)
        .map_or(Ok(64), |s| s.parse::<u64>().map_err(error))?;
    ensure(
        (1..=64).contains(&dead_records),
        "bounded dead allocation fixture",
    )?;
    for kind in [Kind::Radix, Kind::Btree] {
        let start = Instant::now();
        let (_temp, mut f, claim) = setup_attributed(n, kind, dead_records)?;
        let initial = f.head.gate.domains[&0].charged;
        let initial_ledger = f.head.gate.ledger.clone();
        f.c = Counters::default();
        for _ in 0..32 {
            let pin = f.acquire(PinClass::ReaderLease, false)?;
            f.release(&pin, true, true, true)?;
        }
        ensure(
            f.head.gate.domains[&0].charged == initial,
            "standing control accumulation",
        )?;
        let turnover = json!({"selections":64,"charge_before":initial,"charge_after":f.head.gate.domains[&0].charged,"counters":f.c});
        f.c = Counters::default();
        let hold = prepare_retirement(&mut f, &claim)?;
        ensure(
            publish_release(&mut f, &hold, ReleaseCut::Partial(7)).is_err(),
            "partial release",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        publish_release(&mut f, &hold, ReleaseCut::None)?;
        let released_charge = f.head.gate.domains[&0].charged;
        ensure(
            retire(&mut f, &hold, RetireCut::AfterUnlink).is_err(),
            "unlink cut",
        )?;
        let unlinked_charge = f.head.gate.domains[&0].charged;
        f = Fixture::reopen_combined(&f.dir)?;
        ensure(
            complete_retirement_cut(&mut f, &hold, Cut::AfterHead).is_err(),
            "credit selection cut",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        let reconciled = complete_retirement(&mut f, &hold)?;
        f.audit_combined()?;
        let credited = f
            .head
            .gate
            .ledger
            .as_ref()
            .unwrap()
            .last_credit
            .as_ref()
            .unwrap();
        println!(
            "{}",
            json!({"fixture":"actual-v3-standing-control-original-ticket-retirement","n":n,"dead_records":dead_records,"kind":format!("{kind:?}"),"initial_ledger":initial_ledger,"control_turnover":turnover,"original_token":hold.token,"original_hold":hold.by_domain,"after_claim_release_charge":released_charge,"after_unlink_before_credit_charge":unlinked_charge,"final_charge":f.head.gate.domains[&0].charged,"credit_receipt":credited,"reconciled":reconciled,"filesystem_availability_credit":0,"already_dead_sealed_placement":true,"live_placement_relocation_integrated":false,"fresh_pack_enrollment":false,"qualified_saved":false,"elapsed_ms":start.elapsed().as_millis()})
        );
    }
    Ok(())
}

impl AllocationCharge {
    // Sizing only: never returned by an observation or eligible for publication.
    pub(super) fn sizing(data: bool, pack: u64, dev: u64, ino: u64, extent: u64) -> Self {
        Self {
            domain: 0,
            data,
            pack,
            dev,
            ino,
            extent,
            charged_high_water: u64::MAX,
        }
    }
}
pub(super) fn rollover_sealed_charge(base: &Selection, claim: &Claim) -> Result<AllocationCharge> {
    ensure(
        claim.sealed && claim.content_end == claim.extent && claim.observed_charge >= claim.extent,
        "rollover sealed complete measured allocation",
    )?;
    let ledger = base
        .gate
        .ledger
        .as_ref()
        .ok_or("rollover original ledger")?;
    let old = ledger
        .tips
        .iter()
        .find(|t| t.data == claim.data && t.pack == claim.pack);
    if let Some(old) = old {
        ensure(
            old.dev == claim.dev && old.ino == claim.ino && old.extent <= claim.extent,
            "rollover old tip attribution changed",
        )?;
    }
    Ok(AllocationCharge {
        domain: 0,
        data: claim.data,
        pack: claim.pack,
        dev: claim.dev,
        ino: claim.ino,
        extent: claim.extent,
        charged_high_water: claim
            .observed_charge
            .max(old.map_or(0, |c| c.charged_high_water)),
    })
}
#[allow(
    clippy::too_many_lines,
    reason = "Validate every allocation before atomically transferring the ledger"
)]
pub(super) fn rollover_transfer(
    f: &Fixture,
    base: &Selection,
    claims: &[Claim],
    c: &Continuation,
    source: &Store,
) -> Result<(Ledger, u64)> {
    let original = base
        .gate
        .ledger
        .as_ref()
        .ok_or("rollover original attributed ledger")?;
    ensure(
        original.pending_enrollment.is_empty(),
        "rollover bootstrap incomplete",
    )?;
    let mut ledger = original.clone();
    let mut observed = vec![];
    let mut keys = HashSet::new();
    for claim in claims {
        ensure(
            keys.insert((claim.data, claim.pack)),
            "rollover duplicate allocation attribution",
        )?;
        let arena = if claim.data { &f.data } else { &f.meta };
        let m = fs::symlink_metadata(arena.path(claim.pack)).map_err(error)?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && m.dev() == claim.dev
                && m.ino() == claim.ino
                && m.len() == claim.extent
                && claim.extent <= PACK,
            "rollover exact publication allocation",
        )?;
        let actual = m
            .blocks()
            .checked_mul(512)
            .ok_or("rollover allocation overflow")?
            .max(m.len());
        let previous = original
            .tips
            .iter()
            .find(|t| t.data == claim.data && t.pack == claim.pack);
        if let Some(previous) = previous {
            ensure(
                previous.dev == claim.dev
                    && previous.ino == claim.ino
                    && previous.extent <= claim.extent,
                "rollover transferred original tip generation",
            )?;
        }
        let charge = AllocationCharge {
            domain: 0,
            data: claim.data,
            pack: claim.pack,
            dev: claim.dev,
            ino: claim.ino,
            extent: claim.extent,
            charged_high_water: actual.max(previous.map_or(0, |p| p.charged_high_water)),
        };
        if claim.sealed {
            ensure(
                claim.charge.as_ref() == Some(&charge),
                "rollover observed sealed attribution changed",
            )?;
        } else {
            let (pack, end) = if claim.data {
                (c.data_pack, c.data_end)
            } else {
                (c.meta_pack, c.meta_end)
            };
            ensure(
                claim.charge.is_none() && claim.pack == pack && claim.extent == end,
                "rollover exact final tip attribution",
            )?;
        }
        observed.push((claim.sealed, charge));
    }
    for old in &original.tips {
        ensure(
            observed.iter().any(|(_, c)| {
                c.data == old.data && c.pack == old.pack && c.dev == old.dev && c.ino == old.ino
            }),
            "rollover original tip lost attribution",
        )?;
    }
    ledger.tips = observed
        .iter()
        .filter(|(sealed, _)| !*sealed)
        .map(|(_, c)| c.clone())
        .collect();
    ensure(
        ledger.tips.len() == 2 && ledger.tips[0].data && !ledger.tips[1].data,
        "rollover exactly two final tips",
    )?;
    let old_charge = original
        .tips
        .iter()
        .try_fold(0, |n, c| checked(n, c.charged_high_water))?;
    let new_charge = observed
        .iter()
        .try_fold(0, |n, (_, c)| checked(n, c.charged_high_water))?;
    let growth = new_charge
        .checked_sub(old_charge)
        .ok_or("rollover cannot refund old tips")?;
    let registered = original
        .source
        .as_ref()
        .ok_or("rollover original source attribution")?;
    let (next_source, source_growth) = registered.observe_growth(source)?;
    ledger.source = Some(next_source);
    Ok((ledger, checked(growth, source_growth)?))
}
