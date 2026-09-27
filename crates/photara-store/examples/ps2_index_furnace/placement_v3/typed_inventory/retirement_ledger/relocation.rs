//! Disposable same-tip relocation composed with typed ownership retirement.
//! Both selected logical roots remain unchanged. The original ticket retains
//! the sealed allocation's charge until authorized unlink/barrier/absence.
//! No fresh generation, filesystem availability credit, or production deletion.
#[allow(
    clippy::wildcard_imports,
    reason = "Composes actual fixture ticket machinery"
)]
use super::*;

/// Fixture-only ticket codec. JSON records retain their exact UTF-8 bytes;
/// ordinary recipes and legacy ticket byte-array decoding remain unchanged.
pub(super) mod plan_text {
    #[allow(
        clippy::wildcard_imports,
        reason = "Ticket codec shares fixture primitive types"
    )]
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct TextBody {
        utf8: String,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum Body {
        Text(TextBody),
        Legacy(Vec<u8>),
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        pack: u64,
        end: u64,
        bytes: u64,
        writes: Vec<(Option<u64>, PRef, Body)>,
    }
    fn validate(plan: &WritePlan) -> Result<()> {
        ensure(
            plan.end <= PACK && plan.writes.len() <= TX_OBJECTS * 130,
            "bounded ticket plan records",
        )?;
        let mut total = 0;
        let mut previous: Option<(u64, u64, bool)> = None;
        for (key, reference, bytes) in &plan.writes {
            let data = key.is_some();
            let frame = if data { 12 } else { 4 };
            let len = u64::try_from(bytes.len()).map_err(error)?;
            let size = checked(len, frame)?;
            ensure(
                size <= PACK
                    && (data || len <= 4092)
                    && reference.len == len
                    && reference.sha == hash(bytes)
                    && reference.offset >= frame
                    && checked(reference.offset, len)? <= PACK,
                "ticket body length/hash/bound",
            )?;
            if let Some((pack, end, was_data)) = previous {
                let (next_pack, next_end) = if checked(end, size)? > PACK {
                    (checked(pack, 1)?, 0)
                } else {
                    (pack, end)
                };
                ensure(
                    data == was_data
                        && reference.pack == next_pack
                        && reference.offset == checked(next_end, frame)?,
                    "ticket physical continuity",
                )?;
            }
            total = checked(total, size)?;
            previous = Some((reference.pack, checked(reference.offset, len)?, data));
        }
        ensure(
            total == plan.bytes
                && previous.is_none_or(|(pack, end, _)| pack == plan.pack && end == plan.end),
            "ticket final extent/byte commitment",
        )
    }
    pub fn serialize<S: serde::Serializer>(
        plan: &WritePlan,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        validate(plan).map_err(serde::ser::Error::custom)?;
        let writes = plan
            .writes
            .iter()
            .map(|(key, reference, bytes)| {
                let utf8 = String::from_utf8(bytes.clone()).map_err(serde::ser::Error::custom)?;
                Ok((*key, reference.clone(), Body::Text(TextBody { utf8 })))
            })
            .collect::<std::result::Result<Vec<_>, S::Error>>()?;
        Wire {
            pack: plan.pack,
            end: plan.end,
            bytes: plan.bytes,
            writes,
        }
        .serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<WritePlan, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        ensure(
            serde_json::to_vec(&wire)
                .map_err(serde::de::Error::custom)?
                .len()
                <= GATE_MAX,
            "bounded encoded ticket plan",
        )
        .map_err(serde::de::Error::custom)?;
        let plan = WritePlan {
            pack: wire.pack,
            end: wire.end,
            bytes: wire.bytes,
            writes: wire
                .writes
                .into_iter()
                .map(|(key, reference, body)| {
                    let bytes = match body {
                        Body::Text(t) => t.utf8.into_bytes(),
                        Body::Legacy(bytes) => bytes,
                    };
                    (key, reference, bytes)
                })
                .collect(),
        };
        validate(&plan).map_err(serde::de::Error::custom)?;
        Ok(plan)
    }
}

pub(super) mod optional_plan_text {
    use super::{WritePlan, plan_text};
    use serde::{Deserialize, Serialize};
    #[derive(Serialize)]
    struct Borrowed<'a>(#[serde(with = "plan_text")] &'a WritePlan);
    #[derive(Deserialize)]
    struct Owned(#[serde(with = "plan_text")] WritePlan);
    #[allow(
        clippy::ref_option,
        reason = "Serde with adapter requires the field reference"
    )]
    pub fn serialize<S: serde::Serializer>(p: &Option<WritePlan>, s: S) -> Result<S::Ok, S::Error> {
        p.as_ref().map(Borrowed).serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<Option<WritePlan>, D::Error> {
        Ok(Option::<Owned>::deserialize(d)?.map(|p| p.0))
    }
}

fn plan(f: &mut Fixture, source: &Claim) -> Result<(TypedPlan, Vec<Claim>)> {
    let mut tips = [(true, f.data.pack), (false, f.meta.pack)]
        .into_iter()
        .map(|(data, pack)| Claim::capture(f, data, pack))
        .collect::<Result<Vec<_>>>()?;
    for _ in 0..16 {
        let p = plan_inputs(f, source, &tips)?;
        ensure(
            p.data.pack == f.data.pack && p.meta.pack == f.meta.pack,
            "live relocation requires already enrolled same tips",
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
    Err("bounded live relocation self-coverage fixed point".into())
}

/// The old locator is an exact union of semantic and ownership records. Copy
/// either membership when physically selected; owner path replacements below
/// override copied locations for removed nodes, avoiding orphan locator keys.
#[allow(
    clippy::too_many_lines,
    reason = "Keep joint locator and ownership planning together"
)]
fn plan_inputs(f: &mut Fixture, source: &Claim, tips: &[Claim]) -> Result<TypedPlan> {
    ensure(
        !f.imported_read_only && f.head.gate.holds.is_empty(),
        "live relocation requires idle audited fixture",
    )?;
    ensure(
        source.sealed && source.charge.as_ref().is_some_and(|c| c.matches(source)),
        "live relocation requires attributable sealed source",
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
            ensure(claim == *source, "relocation sealed attribution differs")?;
            found = true;
        }
    }
    ensure(found, "relocation source ownership absent")?;
    let records = if source.data {
        f.data.records(source.pack, true)?
    } else {
        f.meta.records(source.pack, false)?
    };
    ensure(records.len() <= TX_OBJECTS, "bounded relocation candidate")?;
    f.c.candidate_read_bytes += source.extent;
    f.c.candidate_records += records.len() as u64;
    let mut data = WritePlan::from(&f.data);
    let mut au = BTreeMap::new();
    let mut bu = BTreeMap::new();
    let mut targets = vec![];
    for (record_key, physical, bytes) in records {
        if source.data {
            let record_key = record_key.ok_or("relocation data key")?;
            let a = f
                .lookup(&h.active, record_key)?
                .filter(|l| l.data == physical);
            let b = f
                .lookup(&h.recovery, record_key)?
                .filter(|l| l.data == physical);
            if a.is_none() && b.is_none() {
                continue;
            }
            let copied = data.push(bytes, Some(record_key))?;
            if let Some(mut location) = a {
                location.data = copied.clone();
                au.insert(record_key, Some(location));
            }
            if let Some(mut location) = b {
                location.data = copied;
                bu.insert(record_key, Some(location));
            }
        } else {
            let page: Page = serde_json::from_slice(&bytes).map_err(error)?;
            targets.push((physical, page.anchor()));
        }
    }
    let mut a = Builder {
        data,
        next: h.inventory.next,
        added: vec![],
        removed: vec![],
    };
    let mut active_inventory = a.remove(f, &h.active, h.inventory.active.clone(), key)?;
    for tip in tips {
        active_inventory = Some(a.insert(f, &h.active, active_inventory, tip)?);
    }
    let ownership_updates = a.updates();
    au.extend(ownership_updates.clone());
    let (data, next, recovery_inventory) = if h.inventory.active == h.inventory.recovery {
        bu.extend(ownership_updates);
        (a.data, a.next, active_inventory.clone())
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
        bu.extend(b.updates());
        (b.data, b.next, recovery)
    };
    f.staged = Some(vec![]);
    let staged = (|| -> Result<(PRef, PRef)> {
        let a = f
            .multi(Some(h.active.clone()), &au.into_iter().collect::<Vec<_>>())?
            .ok_or("relocation active locator")?;
        let b = f
            .multi(
                Some(h.recovery.clone()),
                &bu.into_iter().collect::<Vec<_>>(),
            )?
            .ok_or("relocation recovery locator")?;
        // Rewriting after owner/semantic deltas also covers source pages which
        // remain below newly staged ancestors, independently in either root.
        let mut memo = HashMap::new();
        Ok((
            f.rewrite_metadata(&a, &targets, &mut memo)?,
            f.rewrite_metadata(&b, &targets, &mut memo)?,
        ))
    })();
    let pages = f.staged.take().ok_or("relocation locator staging")?;
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

fn prepare(f: &mut Fixture, source: &Claim) -> Result<Liability> {
    prepare_mode(f, source, true)
}
fn prepare_mode(f: &mut Fixture, source: &Claim, compact: bool) -> Result<Liability> {
    ensure(
        f.head
            .gate
            .ledger
            .as_ref()
            .is_some_and(Ledger::enrollment_complete),
        "live relocation requires complete enrollment",
    )?;
    ensure_unpinned(f, source)?;
    let (p, tips) = plan(f, source)?;
    let token = checked(f.head.gate.last_token, 1)?;
    let mut ticket = RetirementTicket::from_plan(f, token, source, &p, tips)?;
    if compact {
        let proof = commitment::Compact::capture(f, &ticket.data_base, &ticket.meta_base, &p)?;
        let rebuilt = plan_inputs(f, source, &ticket.tip_claims)?;
        proof.verify_rebuild(f, &ticket.data_base, &ticket.meta_base, &rebuilt)?;
        ticket.data = None;
        ticket.meta = None;
        ticket.compact = Some(proof);
    }
    ticket.validate_mode(&p.continuation)?;
    ensure(
        serde_json::to_vec(&ticket).map_err(error)?.len() <= 48 * 1024,
        "live relocation recipe byte cap",
    )?;
    let physical = physical_bound(Some(&p.data), &p.meta)?;
    let budget = checked(
        CONTROL,
        physical["allocation_bound"]
            .as_u64()
            .ok_or("relocation bound")?,
    )?;
    f.reserve_bound(
        f.head.semantic.clone(),
        Some((source.data, source.pack)),
        &[(0, budget)],
        Some(p.continuation),
        None,
        Some(ticket),
    )
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    Partial(usize),
    AfterData,
    BeforeHead,
    StagedHead,
    AfterHead,
    AfterStagedCleanup,
}

fn read_control(f: &Fixture, role: &str) -> Result<Selection> {
    let path = f.dir.join(role);
    let m = fs::symlink_metadata(&path).map_err(error)?;
    ensure(
        m.is_file() && m.nlink() == 1 && m.len() <= GATE_MAX as u64,
        "private relocation control",
    )?;
    let fd = rustix::fs::open(
        &path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(error)?;
    let mut file = File::from(fd);
    let opened = file.metadata().map_err(error)?;
    ensure(
        opened.is_file()
            && opened.nlink() == 1
            && opened.dev() == m.dev()
            && opened.ino() == m.ino()
            && opened.len() == m.len(),
        "relocation control changed",
    )?;
    let mut bytes = vec![0; usize::try_from(m.len()).map_err(error)?];
    file.read_exact(&mut bytes).map_err(error)?;
    ensure(
        file.metadata().map_err(error)?.len() == m.len(),
        "relocation control length changed",
    )?;
    let h: Selection = serde_json::from_slice(&bytes).map_err(error)?;
    ensure(
        serde_json::to_vec(&h).map_err(error)? == bytes,
        "canonical relocation control",
    )?;
    Ok(h)
}

/// Remove only the exact redundant selector while its original dispatch still
/// exists. A cleanup cut retains intent/candidate, so retry remains decidable.
fn staged_cleanup(f: &mut Fixture, exact: &Liability, fault: Fault) -> Result<()> {
    if !control_present(f, "HEAD.next")? {
        return Ok(());
    }
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("relocation ticket")?;
    let (old, candidate) = recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;

    ensure(
        read_control(f, "HEAD")? == f.head
            && (f.head == old || f.head == candidate)
            && read_control(f, "intent")? == old
            && read_control(f, "candidate")? == candidate
            && read_control(f, "HEAD.next")? == candidate,
        "unknown relocation staged selector",
    )?;
    fs::remove_file(f.dir.join("HEAD.next")).map_err(error)?;
    f.c.files_deleted += 1;
    sync_dir(&f.dir, &mut f.c)?;
    ensure(
        !matches!(fault, Fault::AfterStagedCleanup),
        "cut after relocation staged cleanup",
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "Keep original-token proof ordering before replay and selector effects explicit"
)]
fn publish(f: &mut Fixture, exact: &Liability, fault: Fault) -> Result<()> {
    ensure(
        !f.imported_read_only
            && f.head.gate.holds.len() == 1
            && f.head.gate.holds.get(&exact.token) == Some(exact),
        "original relocation hold",
    )?;
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("relocation ticket")?;
    let c = exact
        .continuation
        .as_ref()
        .ok_or("relocation continuation")?;
    ticket.validate_mode(c)?;
    if f.head.active == c.active && f.head.recovery == c.recovery && f.head.inventory == c.inventory
    {
        ticket.verify_completed_bytes(f, exact)?;
        let source_path = if ticket.source.data {
            f.data.path(ticket.source.pack)
        } else {
            f.meta.path(ticket.source.pack)
        };
        match fs::symlink_metadata(&source_path) {
            Ok(_) => ticket.source.verify(f)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                ensure(
                    f.head
                        .gate
                        .ledger
                        .as_ref()
                        .ok_or("retirement ledger")?
                        .unlink_authorizations
                        .get(&exact.token)
                        == Some(&UnlinkAuthorization::for_ticket(f, ticket)?),
                    "missing source before selected unlink authorization",
                )?;
            }
            Err(e) => return Err(error(e)),
        }
        f.audit()?;
        staged_cleanup(f, exact, fault)?;
        validate_ticket_controls(f, exact)?;
        if f.dir.join("intent").exists() {
            f.reconcile()?;
        }
        f.audit_combined()?;
        let path = if ticket.source.data {
            f.data.path(ticket.source.pack)
        } else {
            f.meta.path(ticket.source.pack)
        };
        if path.try_exists().map_err(error)? {
            ensure_no_selected_placement(f, ticket)?;
        }
        return Ok(());
    }
    let saved = f.head.clone();
    let arena = (f.data.pack, f.data.end, f.meta.pack, f.meta.end);
    f.head = ticket.control_base.clone();
    f.data.pack = exact.origin.data_pack;
    f.data.end = exact.origin.data_end;
    f.meta.pack = exact.origin.meta_pack;
    f.meta.end = exact.origin.meta_end;
    f.cache.clear();
    let regenerated = plan_inputs(f, &ticket.source, &ticket.tip_claims);
    f.head = saved;
    (f.data.pack, f.data.end, f.meta.pack, f.meta.end) = arena;
    f.cache.clear();
    f.staged = None;
    let regenerated = regenerated?;
    ensure(
        regenerated.continuation == *c,
        "relocation differs from original typed plan",
    )?;
    if let Some(proof) = &ticket.compact {
        proof.verify_rebuild(f, &ticket.data_base, &ticket.meta_base, &regenerated)?;
    } else {
        let (data, meta) = ticket.full_plans()?;
        ensure(
            regenerated.data == *data && regenerated.meta == *meta,
            "relocation full plans changed",
        )?;
    }
    staged_cleanup(f, exact, fault)?;
    validate_ticket_controls(f, exact)?;
    let mut remaining = if let Fault::Partial(n) = fault {
        Some(n)
    } else {
        None
    };
    recipe::replay(
        f,
        true,
        &ticket.data_base,
        &regenerated.data,
        &mut remaining,
    )?;
    ensure(
        !matches!(fault, Fault::AfterData),
        "cut after relocation data",
    )?;
    recipe::replay(
        f,
        false,
        &ticket.meta_base,
        &regenerated.meta,
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
        match fault {
            Fault::BeforeHead => Cut::BeforeHead,
            Fault::StagedHead => Cut::StagedHead,
            Fault::AfterHead => Cut::AfterHead,
            _ => Cut::None,
        },
    )?;
    ensure_no_selected_placement(f, ticket)?;
    f.audit_combined()?;
    Ok(())
}

/// Before releasing the fallback source, authenticate the complete selected
/// candidate closure, including relocated semantic and ownership records.
fn complete(f: &mut Fixture, exact: &Liability) -> Result<serde_json::Value> {
    complete_cut(f, exact, Cut::None)
}
fn complete_cut(f: &mut Fixture, exact: &Liability, cut: Cut) -> Result<serde_json::Value> {
    ensure(
        !f.imported_read_only,
        "imported relocation completion requires explicit audit",
    )?;
    let ticket = exact
        .retirement_ticket
        .as_ref()
        .ok_or("completion ticket")?;
    ticket.verify_completed_bytes(f, exact)?;
    let path = if ticket.source.data {
        f.data.path(ticket.source.pack)
    } else {
        f.meta.path(ticket.source.pack)
    };
    let absent = || -> Result<()> {
        ensure(
            matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound),
            "unexpected source generation after credit preparation; preserve all evidence",
        )
    };
    if !f.head.gate.holds.contains_key(&exact.token) {
        absent()?;
    }
    // A prior credit HEAD cut has a legitimate authorization -> completion
    // dispatch. Reconcile that exact original pair before the attribution audit,
    // whose held-growth selector rules intentionally exclude final credit dispatch.
    if f.dir.join("intent").exists() {
        let ticket = exact
            .retirement_ticket
            .as_ref()
            .ok_or("completion ticket")?;
        let (expected, _) = completion_selector(f, exact)?;
        let (_, mut authorized) =
            recipe::accounting::selectors_from_base(exact, &ticket.control_base)?;
        authorized.epoch = checked(authorized.epoch, 1)?;
        authorized
            .gate
            .ledger
            .as_mut()
            .ok_or("authorization ledger")?
            .unlink_authorizations
            .insert(exact.token, UnlinkAuthorization::for_ticket(f, ticket)?);
        if read_control(f, "intent")? == authorized && read_control(f, "candidate")? == expected {
            ensure(
                (f.head == authorized || f.head == expected)
                    && read_control(f, "HEAD")? == f.head
                    && !control_present(f, "HEAD.next")?,
                "unknown relocation credit selection",
            )?;
            // Both legal credit-cut states follow barriered absence. Check all
            // typed closures before discarding dispatch; a replacement or broken
            // symlink is never ours to remove. Full attribution follows exact
            // dispatch reconciliation, without relaxing its selector rules.
            absent()?;
            f.audit()?;
            for pin in f.head.gate.pins.values().cloned().collect::<Vec<_>>() {
                f.audit_root(&pin.active, &pin.semantic.active)?;
                f.audit_root(&pin.recovery, &pin.semantic.recovery)?;
            }
            f.reconcile()?;
        }
    }
    f.audit_combined()?;
    complete_retirement_cut(f, exact, cut)
}

/// Synthetic physical baseline constructed before any attributable admission.
/// This is not evidence of runtime rollover. A leaf describing data-0 lives in
/// sealed data-1 alongside semantic frames; data-1's own descriptor lives in
/// the later tip. Every charge is measured from its actual frozen generation.
fn setup_mixed(kind: Kind, garbage: usize) -> Result<(tempfile::TempDir, Fixture)> {
    setup_mixed_records(kind, garbage, false)
}
#[allow(
    clippy::too_many_lines,
    reason = "Explicit synthetic frozen baseline before admission"
)]
fn setup_mixed_records(
    kind: Kind,
    garbage: usize,
    all_live: bool,
) -> Result<(tempfile::TempDir, Fixture)> {
    ensure(garbage <= 128, "bounded synthetic garbage records")?;
    let (temp, mut source, _) = setup(if all_live { 64 } else { 8 }, kind)?;
    let mut f = Fixture::create(&temp.path().join("frozen-mixed"), &mut source)?;
    f.data.rotate(&mut f.c)?;
    let mut other = Claim::capture(&mut f, true, 0)?;
    other.charge = Some(AllocationCharge::capture(&f, &other)?);
    let owner_bytes = serde_json::to_vec(&OwnedNode::Leaf {
        claim: other.clone(),
    })
    .map_err(error)?;
    let owner = Ref {
        offset: OWNER_KEY + 1,
        len: owner_bytes.len() as u64,
        sha: hash(&owner_bytes),
    };
    let physical = f.data.append(&owner_bytes, Some(owner.offset), &mut f.c)?;
    let mut au = vec![(
        owner.offset,
        Some(Location {
            membership: Membership::Ownership,
            object: owner.clone(),
            data: physical,
        }),
    )];
    let mut bu = au.clone();
    let old = f.head.clone();
    let garbage_body = owner_bytes.clone();
    let mut baseline_records = f.data.records(0, true)?;
    baseline_records.sort_by_key(|(_, _, bytes)| bytes.len());
    let mut picked_a = false;
    let mut picked_b = false;
    for (key, physical, bytes) in baseline_records {
        let key = key.ok_or("synthetic data key")?;
        let a = f.lookup(&old.active, key)?.filter(|l| l.data == physical);
        let b = f.lookup(&old.recovery, key)?.filter(|l| l.data == physical);
        if (a.is_none() && b.is_none())
            || (!all_live && (a.is_none() || picked_a) && (b.is_none() || picked_b))
        {
            continue;
        }
        picked_a |= a.is_some();
        picked_b |= b.is_some();
        let copied = f.data.append(&bytes, Some(key), &mut f.c)?;
        if let Some(mut l) = a {
            l.data = copied.clone();
            au.push((key, Some(l)));
        }
        if let Some(mut l) = b {
            l.data = copied;
            bu.push((key, Some(l)));
        }
    }
    ensure(picked_a && picked_b, "synthetic live records")?;
    let garbage_key = owner.offset;
    for _ in 0..garbage {
        f.data.append(&garbage_body, Some(garbage_key), &mut f.c)?;
    }
    ensure(f.data.pack == 1, "synthetic garbage stays in sealed source")?;
    let active = f.multi(Some(old.active), &au)?.ok_or("synthetic active")?;
    let recovery = f
        .multi(Some(old.recovery), &bu)?
        .ok_or("synthetic recovery")?;
    f.data.rotate(&mut f.c)?;
    f.meta.rotate(&mut f.c)?;
    let inventory = Inventory {
        active: Some(owner.clone()),
        recovery: Some(owner),
        next: 2,
    };
    install_baseline(&mut f, active, recovery, inventory, GateState::default())?;
    let (mut ledger, mut claims, mut initial_charge) = bootstrap(&mut f)?;
    ensure(claims.contains(&other), "frozen foreign claim changed")?;
    claims.retain(|c| c.key().ok() != other.key().ok());
    let mut selected = None;
    for _ in 0..16 {
        let p = plan_claim_extents(&mut f, &claims, true)?;
        ensure(
            p.data.pack == f.data.pack && p.meta.pack == f.meta.pack,
            "synthetic baseline tip bound",
        )?;
        let mut changed = false;
        for claim in &mut claims {
            if !claim.sealed {
                let end = if claim.data { p.data.end } else { p.meta.end };
                if claim.extent != end {
                    claim.extent = end;
                    changed = true;
                }
            }
        }
        if !changed {
            selected = Some(p);
            break;
        }
    }
    let p = selected.ok_or("synthetic baseline fixed point")?;
    p.data.write(&mut f.data, &mut f.c)?;
    p.meta.write(&mut f.meta, &mut f.c)?;
    let (observed, growth) = ledger.observe_tips(&f)?;
    ledger = observed;
    ledger.pending_enrollment.clear();
    initial_charge = checked(initial_charge, growth)?;
    let gate = GateState {
        ledger: Some(ledger),
        domains: BTreeMap::from([(
            0,
            CapacityDomain {
                limit: 64 * 1024 * 1024 * 1024,
                charged: initial_charge,
            },
        )]),
        ..GateState::default()
    };
    install_baseline(
        &mut f,
        p.continuation.active,
        p.continuation.recovery,
        p.continuation.inventory,
        gate,
    )?;
    f.audit_combined()?;
    Ok((temp, f))
}

fn install_baseline(
    f: &mut Fixture,
    active: PRef,
    recovery: PRef,
    inventory: Inventory,
    gate: GateState,
) -> Result<()> {
    ensure(
        f.head.gate.domains.is_empty(),
        "synthetic baseline precedes attributed admission",
    )?;
    f.data.sync(&mut f.c)?;
    f.meta.sync(&mut f.c)?;
    f.head.active = active;
    f.head.recovery = recovery;
    f.head.inventory = inventory;
    f.head.gate = gate;
    f.head.data_pack = f.data.pack;
    f.head.data_end = f.data.end;
    f.head.meta_pack = f.meta.pack;
    f.head.meta_end = f.meta.end;
    f.head.epoch = checked(f.head.epoch, 1)?;
    fs::write(
        f.dir.join("HEAD"),
        serde_json::to_vec(&f.head).map_err(error)?,
    )
    .map_err(error)?;
    sync_file(&File::open(f.dir.join("HEAD")).map_err(error)?, &mut f.c)?;
    sync_dir(&f.dir, &mut f.c)?;
    f.cache.clear();
    Ok(())
}

fn source_claim(f: &mut Fixture, data: bool) -> Result<Claim> {
    let pack = u64::from(data); // mixed data-1 or complete live locator metadata-0
    f.ownership_lookup(&f.head.active.clone(), pack * 2 + u64::from(data))?
        .ok_or("synthetic source attribution".into())
}

fn live_memberships(f: &mut Fixture, source: &Claim) -> Result<(u64, u64, u64)> {
    let mut counts = (0, 0, 0);
    let records = if source.data {
        f.data.records(source.pack, true)?
    } else {
        f.meta.records(source.pack, false)?
    };
    f.c.candidate_read_bytes += source.extent;
    f.c.candidate_records += records.len() as u64;
    for (key, physical, bytes) in records {
        for root in [f.head.active.clone(), f.head.recovery.clone()] {
            if source.data {
                if let Some(location) = f
                    .lookup(&root, key.ok_or("membership key")?)?
                    .filter(|l| l.data == physical)
                {
                    match location.membership {
                        Membership::Semantic => counts.0 += 1,
                        Membership::Ownership => counts.1 += 1,
                    }
                }
            } else {
                let page: Page = serde_json::from_slice(&bytes).map_err(error)?;
                if f.path_contains(&root, &physical, page.anchor())? {
                    counts.2 += 1;
                }
            }
        }
    }
    Ok(counts)
}

pub(in super::super::super) fn run(
    garbage: usize,
    kind: Kind,
    data: bool,
) -> Result<serde_json::Value> {
    let (_temp, mut f) = setup_mixed(kind, garbage)?;
    f.c = Counters::default();
    let mut row = relocate_owned(&mut f, data, u64::from(data))?;
    row["kind"] = json!(format!("{kind:?}"));
    row["garbage_records"] = json!(garbage);
    row["semantic_keys"] = json!(8);
    row["synthetic_frozen_physical_baseline"] = json!(true);
    row["runtime_rollover"] = json!(false);
    Ok(row)
}

/// Runtime caller supplies an existing fully attributed fixture. This never
/// constructs or enrolls a source: only its exact selected sealed ownership
/// claim can authorize the original relocation/ticket/credit sequence.
pub(in super::super::super) fn relocate_owned(
    f: &mut Fixture,
    data: bool,
    pack: u64,
) -> Result<serde_json::Value> {
    let key = pack
        .checked_mul(2)
        .and_then(|v| v.checked_add(u64::from(data)))
        .ok_or("relocation allocation identity overflow")?;
    let source = f
        .ownership_lookup(&f.head.active.clone(), key)?
        .or(f.ownership_lookup(&f.head.recovery.clone(), key)?)
        .ok_or("selected relocation source ownership absent")?;
    let memberships = live_memberships(f, &source)?;
    let initial = f
        .head
        .gate
        .domains
        .get(&0)
        .ok_or("relocation project domain")?
        .charged;
    let semantic = f.head.semantic.clone();
    let started = Instant::now();
    let hold = prepare(f, &source)?;
    let ticket = hold.retirement_ticket.as_ref().unwrap();
    let bytes = if let Some(proof) = &ticket.compact {
        proof.bytes()
    } else {
        let (data, meta) = ticket.full_plans()?;
        (data.bytes, meta.bytes)
    };
    publish(f, &hold, Fault::None)?;
    let selected_charge = f.head.gate.domains[&0].charged;
    complete(f, &hold)?;
    let receipt = f
        .head
        .gate
        .ledger
        .as_ref()
        .unwrap()
        .last_credit
        .clone()
        .ok_or("relocation receipt")?;
    ensure(f.head.semantic == semantic, "relocation semantics changed")?;
    f.audit_combined()?;
    Ok(json!({"fixture":"actual-v3-same-tip-live-owned-relocation",
        "recipe_mode":"compact-framed-commitment",
        "source_data":data,"live_selected_memberships":memberships,"source":source,"original_token":hold.token,"original_hold":hold.by_domain,
        "initial_project_charge":initial,"after_both_root_release_charge":selected_charge,
        "final_project_charge":f.head.gate.domains[&0].charged,"credit_receipt":receipt,
        "data_encoded_bytes":bytes.0,"metadata_encoded_bytes":bytes.1,
        "net_project_credit":i128::from(initial)-i128::from(f.head.gate.domains[&0].charged),
        "filesystem_availability_credit":0,"qualified_saved":false,
        "counters":f.c,"elapsed_ms":started.elapsed().as_millis()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relocation_ticket_text_codec_exact_legacy_and_corruption_bounds() -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct Encoded {
            #[serde(with = "plan_text")]
            plan: WritePlan,
        }
        let (_temp, f) = setup_mixed(Kind::Btree, 0)?;
        let mut plan = WritePlan::from(&f.data);
        plan.push(
            serde_json::to_vec(&json!({"unicode":"é雪", "escape":"\n\"\\"})).map_err(error)?,
            Some(11),
        )?;
        plan.push(b"{\"next\":true}".to_vec(), Some(12))?;
        let encoded = serde_json::to_vec(&Encoded { plan: plan.clone() }).map_err(error)?;
        let decoded: Encoded = serde_json::from_slice(&encoded).map_err(error)?;
        ensure(
            decoded.plan == plan && serde_json::to_vec(&decoded).map_err(error)? == encoded,
            "text ticket changed exact bytes or canonical encoding",
        )?;
        let legacy = json!({"plan":serde_json::to_value(&plan).map_err(error)?});
        let decoded: Encoded = serde_json::from_value(legacy).map_err(error)?;
        ensure(decoded.plan == plan, "legacy byte-array ticket changed")?;
        let original: serde_json::Value = serde_json::from_slice(&encoded).map_err(error)?;
        for mode in 0..5 {
            let mut bad = original.clone();
            match mode {
                0 => bad["plan"]["writes"][0][1]["len"] = json!(1),
                1 => bad["plan"]["writes"][0][1]["sha"] = json!(hash(b"wrong")),
                2 => {
                    let at = bad["plan"]["writes"][1][1]["offset"].as_u64().unwrap();
                    bad["plan"]["writes"][1][1]["offset"] = json!(at + 1);
                }
                3 => bad["plan"]["writes"][0][2]["utf8"] = json!("x".repeat(PACK as usize)),
                _ => bad["plan"]["writes"][0][2]["unknown"] = json!(true),
            }
            ensure(
                serde_json::from_value::<Encoded>(bad).is_err(),
                "malformed text ticket accepted",
            )?;
        }
        Ok(())
    }
    #[test]
    fn relocation_live_semantic_owned_and_locator_records_preserve_both_roots() -> Result<()> {
        for kind in [Kind::Btree, Kind::Radix] {
            for data in [true, false] {
                let (_temp, mut f) = setup_mixed(kind, 0)?;
                let source = source_claim(&mut f, data)?;
                let memberships = live_memberships(&mut f, &source)?;
                ensure(
                    if data {
                        memberships.0 > 0 && memberships.1 > 0
                    } else {
                        memberships.2 > 0
                    },
                    "fixture candidate must contain real selected membership",
                )?;
                ensure(
                    f.head.active != f.head.recovery,
                    "independent selected root coverage",
                )?;
                let semantic = f.head.semantic.clone();
                let initial = f.head.gate.domains[&0].charged;
                let hold = prepare(&mut f, &source)?;
                publish(&mut f, &hold, Fault::None)?;
                ensure(
                    f.head.semantic == semantic && f.head.gate.domains[&0].charged == initial,
                    "relocation changed logical roots or refunded before unlink",
                )?;
                ensure_no_selected_placement(&mut f, hold.retirement_ticket.as_ref().unwrap())?;
                let result = complete(&mut f, &hold)?;
                let after = f.head.gate.domains[&0].charged;
                complete(&mut f, &hold)?;
                ensure(
                    f.head.gate.domains[&0].charged == after,
                    "duplicate relocation credit",
                )?;
                ensure(
                    f.head.semantic == semantic,
                    "retirement altered semantic roots",
                )?;
                f.audit_combined()?;
                assert!(!result.is_null());
            }
        }
        Ok(())
    }

    #[test]
    fn relocation_original_recipe_cuts_reopen_without_duplicate_copy_or_credit() -> Result<()> {
        for kind in [Kind::Btree, Kind::Radix] {
            for data in [true, false] {
                for fault in [
                    Fault::Partial(7),
                    Fault::Partial(usize::MAX),
                    Fault::AfterData,
                    Fault::BeforeHead,
                    Fault::StagedHead,
                    Fault::AfterHead,
                ] {
                    let (_temp, mut f) = setup_mixed(kind, 0)?;
                    let source = source_claim(&mut f, data)?;
                    let hold = prepare(&mut f, &source)?;
                    let charged = f.head.gate.domains[&0].charged;
                    let fault = if matches!(fault, Fault::Partial(usize::MAX)) {
                        Fault::Partial(
                            usize::try_from(
                                hold.retirement_ticket
                                    .as_ref()
                                    .unwrap()
                                    .compact
                                    .as_ref()
                                    .unwrap()
                                    .bytes()
                                    .0,
                            )
                            .map_err(error)?
                                + 7,
                        )
                    } else {
                        fault
                    };
                    ensure(
                        publish(&mut f, &hold, fault).is_err(),
                        "missing relocation cut",
                    )?;
                    f = Fixture::reopen_combined(&f.dir)?;
                    publish(&mut f, &hold, Fault::None)?;
                    let selected = f.snapshot()?;
                    publish(&mut f, &hold, Fault::None)?;
                    ensure(
                        f.snapshot()? == selected && f.head.gate.domains[&0].charged == charged,
                        "replay duplicated copy or premature credit",
                    )?;
                    complete(&mut f, &hold)?;
                    ensure(
                        f.head.gate.last_token == hold.token,
                        "replay replaced token",
                    )?;
                    f.audit_combined()?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn relocation_staged_cleanup_unknown_controls_and_all_pins_fence() -> Result<()> {
        let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
        let source = source_claim(&mut f, true)?;
        for class in EXTRA_CLASSES {
            let pin = f.acquire(class, class == PinClass::UnresolvedIntent)?;
            let before = f.snapshot()?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            ensure(
                prepare(&mut f, &source).is_err(),
                "live pinned source admitted",
            )?;
            ensure(
                f.snapshot()? == before
                    && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                    && f.head.gate.holds.is_empty(),
                "pin refusal created effects",
            )?;
            f.release(&pin, true, true, true)?;
        }
        let hold = prepare(&mut f, &source)?;
        ensure(
            publish(&mut f, &hold, Fault::StagedHead).is_err(),
            "staged relocation cut",
        )?;
        let exact_next = fs::read(f.dir.join("HEAD.next")).map_err(error)?;
        let mut changed: Selection = serde_json::from_slice(&exact_next).map_err(error)?;
        changed.gate.domains.get_mut(&0).unwrap().charged += 1;
        fs::write(
            f.dir.join("HEAD.next"),
            serde_json::to_vec(&changed).map_err(error)?,
        )
        .map_err(error)?;
        let before = f.snapshot()?;
        ensure(
            publish(&mut f, &hold, Fault::None).is_err() && f.snapshot()? == before,
            "unknown staged selector removed",
        )?;
        fs::write(f.dir.join("HEAD.next"), exact_next).map_err(error)?;
        ensure(
            publish(&mut f, &hold, Fault::AfterStagedCleanup).is_err(),
            "cleanup cut",
        )?;
        ensure(
            !f.dir.join("HEAD.next").exists()
                && f.dir.join("intent").exists()
                && f.dir.join("candidate").exists(),
            "cleanup lost dispatch",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        publish(&mut f, &hold, Fault::None)?;
        complete(&mut f, &hold)?;
        f.audit_combined()?;
        Ok(())
    }

    #[test]
    fn relocation_unlink_and_credit_cuts_keep_charge_until_exact_once_credit() -> Result<()> {
        for data in [true, false] {
            for cut in [
                RetireCut::BeforeUnlink,
                RetireCut::AfterUnlink,
                RetireCut::AfterDirectoryBarrier,
            ] {
                let (_temp, mut f) = setup_mixed(Kind::Radix, 0)?;
                let source = source_claim(&mut f, data)?;
                let hold = prepare(&mut f, &source)?;
                publish(&mut f, &hold, Fault::None)?;
                let charged = f.head.gate.domains[&0].charged;
                ensure(retire(&mut f, &hold, cut).is_err(), "unlink cut")?;
                ensure(
                    f.head.gate.domains[&0].charged == charged,
                    "early project credit",
                )?;
                f = Fixture::reopen_combined(&f.dir)?;
                complete(&mut f, &hold)?;
                let final_charge = f.head.gate.domains[&0].charged;
                complete(&mut f, &hold)?;
                ensure(
                    f.head.gate.domains[&0].charged == final_charge,
                    "second refund",
                )?;
                f.audit_combined()?;
            }
            for cut in [Cut::BeforeHead, Cut::AfterHead] {
                let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
                let source = source_claim(&mut f, data)?;
                let hold = prepare(&mut f, &source)?;
                publish(&mut f, &hold, Fault::None)?;
                ensure(complete_cut(&mut f, &hold, cut).is_err(), "credit cut")?;
                f = Fixture::reopen_combined(&f.dir)?;
                complete(&mut f, &hold)?;
                f.audit_combined()?;
            }
        }
        Ok(())
    }

    #[test]
    fn relocation_capacity_pending_work_and_candidate_corruption_refuse_without_unlink()
    -> Result<()> {
        let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
        let source = source_claim(&mut f, true)?;
        let mut gate = f.head.gate.clone();
        gate.domains.get_mut(&0).unwrap().limit = gate.domains[&0].charged;
        f.select_gate(gate)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            prepare(&mut f, &source).is_err()
                && f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
            "capacity refusal created effects",
        )?;
        let mut gate = f.head.gate.clone();
        gate.domains.get_mut(&0).unwrap().limit = 64 * 1024 * 1024 * 1024;
        f.select_gate(gate)?;
        let pending = f.reserve(f.head.semantic.clone(), None, &[(0, 4096)])?;
        let before = f.snapshot()?;
        ensure(
            prepare(&mut f, &source).is_err()
                && f.snapshot()? == before
                && f.head.gate.holds.get(&pending.token) == Some(&pending),
            "unrelated original work bypassed",
        )?;

        let (_temp, mut f) = setup_mixed(Kind::Radix, 0)?;
        let source = source_claim(&mut f, true)?;
        let original_plan = plan(&mut f, &source)?.0;
        let hold = prepare(&mut f, &source)?;
        publish(&mut f, &hold, Fault::None)?;
        let (_, physical, _) = original_plan
            .data
            .writes
            .iter()
            .find(|(key, _, _)| key.is_some_and(|k| k < OWNER_KEY))
            .ok_or("copied semantic frame")?;
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.data.path(physical.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(physical.offset)).map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        // Structural reopen may detect the damaged object early; otherwise the
        // candidate completion branch must audit closure before any unlink.
        match Fixture::reopen_combined(&f.dir) {
            Ok(mut reopened) => ensure(
                publish(&mut reopened, &hold, Fault::None).is_err(),
                "selected candidate corruption escaped closure audit",
            )?,
            Err(_) => {}
        }
        ensure(
            f.data.path(source.pack).exists()
                && f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head,
            "candidate corruption discarded source or evidence",
        )?;
        Ok(())
    }

    #[test]
    fn relocation_overhead_counterexample_and_high_garbage_measurements() -> Result<()> {
        for kind in [Kind::Btree, Kind::Radix] {
            for garbage in [0, 128] {
                let row = run(garbage, kind, true)?;
                println!("{row}");
                let net = row["net_project_credit"]
                    .as_i64()
                    .ok_or("net project delta")?;
                let credit = row["credit_receipt"]["project_credit"]
                    .as_i64()
                    .ok_or("credit")?;
                let growth = row["credit_receipt"]["growth"].as_i64().ok_or("growth")?;
                ensure(
                    net == credit - growth && row["filesystem_availability_credit"] == 0,
                    "measured project delta differs from exact credit minus growth",
                )?;
            }
        }
        Ok(())
    }

    #[test]
    fn compact_large_live_plan_passes_unchanged_ticket_cap() -> Result<()> {
        for kind in [Kind::Radix, Kind::Btree] {
            let (_temp, mut f) = setup_mixed_records(kind, 0, true)?;
            let source = source_claim(&mut f, true)?;
            let before = f.snapshot()?;
            ensure(
                prepare_mode(&mut f, &source, false)
                    .unwrap_err()
                    .contains("recipe byte cap"),
                "legacy full-body candidate should exceed cap",
            )?;
            ensure(
                before == f.snapshot()? && f.head.gate.holds.is_empty(),
                "full refusal effects",
            )?;
            let hold = prepare(&mut f, &source)?;
            let ticket = hold.retirement_ticket.as_ref().unwrap();
            ensure(
                ticket.data.is_none()
                    && ticket.meta.is_none()
                    && serde_json::to_vec(ticket).map_err(error)?.len() <= 48 * 1024,
                "compact bounded ticket",
            )?;
            publish(&mut f, &hold, Fault::None)?;
            ensure(
                retire(&mut f, &hold, RetireCut::AfterUnlink).is_err(),
                "unlink cut",
            )?;
            f = Fixture::reopen_combined(&f.dir)?;
            publish(&mut f, &hold, Fault::None)?;
            complete(&mut f, &hold)?;
            complete(&mut f, &hold)?;
            f.audit_combined()?;
        }
        Ok(())
    }

    #[test]
    fn compact_original_prefix_and_manifest_fields_cannot_be_recaptured() -> Result<()> {
        for replace in [false, true] {
            let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
            let source = source_claim(&mut f, true)?;
            let p = plan(&mut f, &source)?.0;
            let data = recipe::Prefix::capture(&f.data)?;
            let meta = recipe::Prefix::capture(&f.meta)?;
            let path = f.data.path(data.pack);
            if replace {
                fs::rename(&path, path.with_extension("original")).map_err(error)?;
                fs::copy(path.with_extension("original"), &path).map_err(error)?;
            } else {
                let mut file = OpenOptions::new().write(true).open(&path).map_err(error)?;
                file.write_all(b"!").map_err(error)?;
            }
            ensure(
                commitment::Compact::capture(&mut f, &data, &meta, &p).is_err(),
                "old original prefix was recaptured or rebound",
            )?;
        }
        let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
        let source = source_claim(&mut f, true)?;
        let hold = prepare(&mut f, &source)?;
        publish(&mut f, &hold, Fault::None)?;
        for after_unlink in [false, true] {
            if after_unlink {
                ensure(
                    retire(&mut f, &hold, RetireCut::AfterUnlink).is_err(),
                    "unlink cut",
                )?;
            }
            let ticket = hold.retirement_ticket.as_ref().unwrap();
            let proof = ticket.compact.as_ref().unwrap();
            for field in [
                "ino",
                "original_sha",
                "frames",
                "final_end",
                "typed_plan_sha",
            ] {
                let mut value = serde_json::to_value(proof).map_err(error)?;
                if field == "typed_plan_sha" {
                    value[field] = json!("0".repeat(64));
                } else if field == "original_sha" {
                    value["data"][field] = json!("0".repeat(64));
                } else {
                    value["data"][field] = json!(value["data"][field].as_u64().unwrap() + 1);
                }
                let changed: commitment::Compact = serde_json::from_value(value).map_err(error)?;
                let snapshot = f.snapshot()?;
                ensure(
                    changed
                        .verify_complete(
                            &mut f,
                            &ticket.data_base,
                            &ticket.meta_base,
                            hold.continuation.as_ref().unwrap(),
                        )
                        .is_err(),
                    "altered completed manifest accepted",
                )?;
                ensure(snapshot == f.snapshot()?, "manifest proof mutated files")?;
            }
        }
        complete(&mut f, &hold)?;
        Ok(())
    }

    #[test]
    fn compact_mixed_modes_refuse_and_full_serialization_is_preserved() -> Result<()> {
        let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
        let source = source_claim(&mut f, true)?;
        let (p, tips) = plan(&mut f, &source)?;
        let full = RetirementTicket::from_plan(&mut f, 1, &source, &p, tips)?;
        let value = serde_json::to_value(&full).map_err(error)?;
        #[derive(Serialize)]
        struct Prior<'a>(#[serde(with = "plan_text")] &'a WritePlan);
        ensure(
            value["data"] == serde_json::to_value(Prior(&p.data)).map_err(error)?
                && value.get("compact").is_none(),
            "legacy Some plan serialization changed",
        )?;
        let proof = commitment::Compact::capture(&mut f, &full.data_base, &full.meta_base, &p)?;
        for mode in 0..4 {
            let mut changed = full.clone();
            match mode {
                0 => {
                    changed.compact = Some(proof.clone());
                }
                1 => {
                    changed.data = None;
                }
                2 => {
                    changed.data = None;
                    changed.meta = None;
                }
                _ => {
                    changed.meta = None;
                    changed.compact = Some(proof.clone());
                }
            }
            ensure(
                changed.validate_mode(&p.continuation).is_err(),
                "ambiguous recipe mode accepted",
            )?;
            let before = f.snapshot()?;
            let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
            ensure(
                f.reserve_bound(
                    f.head.semantic.clone(),
                    Some((source.data, source.pack)),
                    &[(0, 16 * 1024 * 1024)],
                    Some(p.continuation.clone()),
                    None,
                    Some(changed),
                )
                .is_err(),
                "generic admission accepted ambiguous ticket",
            )?;
            ensure(
                before == f.snapshot()?
                    && head == fs::read(f.dir.join("HEAD")).map_err(error)?
                    && f.head.gate.holds.is_empty(),
                "ambiguous ticket admission effects",
            )?;
        }
        // The old full mode still executes the same original-token path.
        let hold = prepare_mode(&mut f, &source, false)?;
        publish(&mut f, &hold, Fault::None)?;
        complete(&mut f, &hold)?;
        Ok(())
    }

    #[test]
    fn compact_selected_source_changes_preserve_original_dispatch() -> Result<()> {
        for mode in 0..3 {
            let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
            let source = source_claim(&mut f, true)?;
            let hold = prepare(&mut f, &source)?;
            ensure(
                publish(&mut f, &hold, Fault::AfterHead).is_err(),
                "selected cut",
            )?;
            let path = f.data.path(source.pack);
            if mode == 0 {
                fs::rename(&path, path.with_extension("original")).map_err(error)?;
                fs::copy(path.with_extension("original"), &path).map_err(error)?;
            } else if mode == 1 {
                OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .map_err(error)?
                    .write_all(b"!")
                    .map_err(error)?;
            } else {
                fs::remove_file(&path).map_err(error)?;
            }
            let read = |f: &Fixture| {
                ["HEAD", "intent", "candidate"]
                    .into_iter()
                    .map(|n| fs::read(f.dir.join(n)).map_err(error))
                    .collect::<Result<Vec<_>>>()
            };
            let controls = read(&f)?;
            let before = f.snapshot()?;
            ensure(
                publish(&mut f, &hold, Fault::None).is_err(),
                "changed selected source accepted",
            )?;
            ensure(
                controls == read(&f)? && before == f.snapshot()?,
                "changed source lost original evidence",
            )?;
        }
        Ok(())
    }

    #[test]
    fn compact_destination_corruption_retains_staged_dispatch() -> Result<()> {
        for mode in 0..3 {
            let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
            let source = source_claim(&mut f, true)?;
            let p = plan(&mut f, &source)?.0;
            let hold = prepare(&mut f, &source)?;
            ensure(
                publish(&mut f, &hold, Fault::StagedHead).is_err(),
                "staged cut",
            )?;
            let path = f.data.path(p.data.pack);
            if mode == 0 {
                fs::rename(&path, path.with_extension("original")).map_err(error)?;
                fs::copy(path.with_extension("original"), &path).map_err(error)?;
            } else if mode == 1 {
                OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .map_err(error)?
                    .write_all(b"!")
                    .map_err(error)?;
            } else {
                // Raw manifest covers even staged ownership frames omitted from
                // the final exact locator closure.
                let c = hold.continuation.as_ref().unwrap();
                let old = f.head.clone();
                f.head = read_control(&f, "candidate")?;
                f.cache.clear();
                let mut unselected = None;
                for (key, r, _) in &p.data.writes {
                    let key = key.unwrap();
                    if f.lookup(&c.active, key)?.is_none_or(|v| v.data != *r)
                        && f.lookup(&c.recovery, key)?.is_none_or(|v| v.data != *r)
                    {
                        unselected = Some(r);
                        break;
                    }
                }
                f.head = old;
                f.cache.clear();
                let r = unselected.ok_or("fixture needs intermediate unreachable frame")?;
                let mut file = OpenOptions::new().write(true).open(&path).map_err(error)?;
                file.seek(SeekFrom::Start(r.offset)).map_err(error)?;
                file.write_all(b"!").map_err(error)?;
            }
            let controls = ["HEAD", "intent", "candidate", "HEAD.next"]
                .into_iter()
                .map(|n| fs::read(f.dir.join(n)).map_err(error))
                .collect::<Result<Vec<_>>>()?;
            let before = f.snapshot()?;
            ensure(
                publish(&mut f, &hold, Fault::None).is_err(),
                "unknown destination accepted",
            )?;
            let after = ["HEAD", "intent", "candidate", "HEAD.next"]
                .into_iter()
                .map(|n| fs::read(f.dir.join(n)).map_err(error))
                .collect::<Result<Vec<_>>>()?;
            ensure(
                controls == after && before == f.snapshot()?,
                "proof failure discarded dispatch",
            )?;
        }
        Ok(())
    }

    #[test]
    fn relocation_real_large_live_recipe_cap_refuses_without_effects() -> Result<()> {
        let (_temp, mut f) = setup_mixed_records(Kind::Btree, 0, true)?;
        let source = source_claim(&mut f, true)?;
        let before = f.snapshot()?;
        let head = fs::read(f.dir.join("HEAD")).map_err(error)?;
        ensure(
            prepare_mode(&mut f, &source, false)
                .unwrap_err()
                .contains("recipe byte cap"),
            "real live candidate should exceed bounded recipe",
        )?;
        ensure(
            f.snapshot()? == before
                && fs::read(f.dir.join("HEAD")).map_err(error)? == head
                && f.head.gate.holds.is_empty(),
            "oversized relocation created effects",
        )
    }

    #[test]
    fn relocation_credit_retry_occupants_and_typed_corruption_preserve_dispatch() -> Result<()> {
        for cut in [Cut::BeforeHead, Cut::AfterHead] {
            for mode in 0..3 {
                let (_temp, mut f) = setup_mixed(Kind::Btree, 0)?;
                let source = source_claim(&mut f, true)?;
                let original_plan = plan(&mut f, &source)?.0;
                let hold = prepare(&mut f, &source)?;
                publish(&mut f, &hold, Fault::None)?;
                ensure(complete_cut(&mut f, &hold, cut).is_err(), "credit cut")?;
                let path = f.data.path(source.pack);
                if mode == 0 {
                    fs::write(&path, b"unknown replacement").map_err(error)?;
                } else if mode == 1 {
                    std::os::unix::fs::symlink(f.dir.join("absent-target"), &path)
                        .map_err(error)?;
                } else {
                    let (_, reference, _) = original_plan
                        .data
                        .writes
                        .iter()
                        .find(|(key, _, _)| key.is_some_and(|k| k >= OWNER_KEY))
                        .ok_or("relocated ownership record")?;
                    let mut file = OpenOptions::new()
                        .write(true)
                        .open(f.data.path(reference.pack))
                        .map_err(error)?;
                    file.seek(SeekFrom::Start(reference.offset))
                        .map_err(error)?;
                    file.write_all(b"!").map_err(error)?;
                }
                f = Fixture::reopen_combined(&f.dir)?;
                let controls = ["HEAD", "intent", "candidate"]
                    .into_iter()
                    .map(|name| fs::read(f.dir.join(name)).map_err(error))
                    .collect::<Result<Vec<_>>>()?;
                let before = f.snapshot()?;
                ensure(
                    complete(&mut f, &hold).is_err(),
                    "unknown post-credit state accepted",
                )?;
                let after = ["HEAD", "intent", "candidate"]
                    .into_iter()
                    .map(|name| fs::read(f.dir.join(name)).map_err(error))
                    .collect::<Result<Vec<_>>>()?;
                ensure(
                    controls == after && f.snapshot()? == before,
                    "failed credit retry discarded dispatch or touched unknown generation",
                )?;
                if mode < 2 {
                    ensure(fs::symlink_metadata(path).is_ok(), "replacement deleted")?;
                }
            }
        }
        Ok(())
    }
}
