//! Restart from selected original records and current supplied bytes, without a
//! retained in-memory plan. Historical prefix views never certify current tails.
use super::{Limits, OriginalPackage, RepeatablePlan, VerifiedOriginal, recovery, wire};
#[cfg(any(test, feature = "controlled-disposable"))]
use crate::package::Sha256Hex;
use crate::package::v1_3::{
    self, AllocationInspection, AllocationLayout, AllocationObservation, DirectoryObservation,
    FrameLimits, ReaderLimits, SelectionIdentity, SettledRegistration,
};
use crate::package::{
    JsonLimits, ObjectRef, PackageError, PackageUuid, digest, parse_canonical_json,
};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

#[path = "restart_prefix.rs"]
pub(super) mod prefix_index;
#[path = "restart_reader.rs"]
mod reader_memo;
#[path = "restart_snapshot.rs"]
#[cfg(any(test, feature = "controlled-disposable"))]
pub(super) mod snapshot_memo;

/// All observations are independent registered/pinned inputs. No IO or lease is
/// acquired, and success is neither write permission nor a durable receipt.
pub struct RestartContext<'a, I> {
    pub inspection: &'a I,
    pub registration: &'a SettledRegistration,
    pub directory: DirectoryObservation,
    pub identity: &'a SelectionIdentity,
    pub frames: FrameLimits,
    pub reader: &'a ReaderLimits,
    pub planner: Limits,
    pub max_originals: usize,
}
fn check(ok: bool) -> Result<(), PackageError> {
    if ok {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn identity(v: &SelectionIdentity) -> SelectionIdentity {
    SelectionIdentity {
        project: v.project,
        library: v.library,
        bootstrap_sha256: v.bootstrap_sha256.clone(),
    }
}
fn number(v: &Value) -> Result<u64, PackageError> {
    wire::number(v).map_err(|_| PackageError::Record)
}
fn rounded(n: u64, unit: u64) -> Result<u64, PackageError> {
    if unit == 0 {
        return Err(PackageError::Integrity);
    }
    n.checked_add(unit - 1)
        .and_then(|n| n.checked_div(unit))
        .and_then(|n| n.checked_mul(unit))
        .ok_or(PackageError::Limit)
}
fn loose(package: &OriginalPackage, r: &Value, limits: JsonLimits) -> Result<Value, PackageError> {
    let reference: ObjectRef =
        serde_json::from_value(r.clone()).map_err(|_| PackageError::Record)?;
    check(
        reference.kind == crate::package::ObjectKind::Json
            && reference.byte_length.get() <= limits.max_bytes as u64,
    )?;
    let bytes = package
        .loose
        .get(reference.sha256.as_str())
        .ok_or(PackageError::Integrity)?;
    check(bytes.len() as u64 == reference.byte_length.get() && digest(bytes) == reference.sha256)?;
    parse_canonical_json(bytes, limits)
}
struct Packed<'a> {
    package: &'a OriginalPackage,
    limits: FrameLimits,
    checked: RefCell<BTreeMap<PackageUuid, v1_3::PackedAllocation<'a>>>,
}
impl<'a> v1_3::PhysicalRecords for Packed<'a> {
    fn resolve(&self, r: &v1_3::PhysicalRef, kind: v1_3::FrameKind) -> Result<Value, PackageError> {
        if !self.checked.borrow().contains_key(&r.allocation_id) {
            let package: &'a OriginalPackage = self.package;
            let a = package
                .allocations
                .get(&r.allocation_id.to_string())
                .ok_or(PackageError::Integrity)?;
            let arena = serde_json::from_value(Value::String(a.arena.clone()))
                .map_err(|_| PackageError::Record)?;
            self.checked.borrow_mut().insert(
                r.allocation_id,
                v1_3::PackedAllocation::open(r.allocation_id, arena, &a.bytes, self.limits)?,
            );
        }
        self.checked
            .borrow()
            .get(&r.allocation_id)
            .ok_or(PackageError::Integrity)?
            .resolve(r, kind)
    }
}
fn request<I>(
    package: &OriginalPackage,
    r: &Value,
    context: &RestartContext<'_, I>,
) -> Result<Value, PackageError> {
    let reference: ObjectRef =
        serde_json::from_value(r.clone()).map_err(|_| PackageError::Record)?;
    if package.loose.contains_key(reference.sha256.as_str()) {
        return loose(package, r, context.reader.json);
    }
    // Published receipts are real semantic edges. Resolve their actual selected
    // locator; embedded bodies and diagnostic object pools are not substitutes.
    let e = v1_3::SelectedEnvelope::parse(
        &wire::encode(&package.manifest),
        &wire::encode(&package.head),
        &wire::encode(&package.commit),
        identity(context.identity),
        context.reader.json,
    )?;
    let provider = Packed {
        package,
        limits: context.frames,
        checked: RefCell::new(BTreeMap::new()),
    };
    let role = e.open_role(&provider, v1_3::SelectedRole::Active, context.reader.tree)?;
    v1_3::Locator::new(
        &provider,
        context.identity.project,
        role.locator,
        context.reader.tree,
    )
    .json(&reference, v1_3::Membership::Semantic)
}
/// Resolves one semantic record from a privately verified original's actual
/// selected locator. Used for planner inputs, never a diagnostic object pool.
#[cfg(any(test, feature = "controlled-disposable"))]
pub(crate) fn original_json(
    original: &VerifiedOriginal,
    reference: &Value,
    frames: FrameLimits,
    reader: &v1_3::ReaderLimits,
) -> Result<Value, PackageError> {
    let package = original.package();
    let selection = v1_3::SelectionIdentity {
        project: PackageUuid::parse(
            package.manifest["project_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
        library: PackageUuid::parse(
            package.commit["root_set"]["library_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
        bootstrap_sha256: Sha256Hex::parse(
            package.commit["bootstrap_sha256"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
    };
    let envelope = v1_3::SelectedEnvelope::parse(
        &wire::encode(&package.manifest),
        &wire::encode(&package.head),
        &wire::encode(&package.commit),
        selection,
        reader.json,
    )?;
    let provider = Packed {
        package,
        limits: frames,
        checked: RefCell::new(BTreeMap::new()),
    };
    let role = envelope.open_role(&provider, v1_3::SelectedRole::Active, reader.tree)?;
    let reference: ObjectRef =
        serde_json::from_value(reference.clone()).map_err(|_| PackageError::Record)?;
    v1_3::Locator::new(
        &provider,
        PackageUuid::parse(
            package.manifest["project_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
        role.locator,
        reader.tree,
    )
    .json(&reference, v1_3::Membership::Semantic)
}

fn controls(package: &OriginalPackage, limits: JsonLimits) -> Result<(Value, Value), PackageError> {
    let envelope = loose(
        package,
        &package.commit["root_set"]["placement"]["accounting"],
        limits,
    )?;
    let hold = loose(package, &envelope["holds"], limits)?;
    check(
        hold["schema"]["id"] == "photara.storage.hold-leaf"
            && hold["schema"]["version"] == 1
            && hold["count"] == "1",
    )?;
    let rows = hold["entries"].as_array().ok_or(PackageError::Record)?;
    check(rows.len() == 1)?;
    let original = loose(package, &rows[0]["original"], limits)?;
    let phase = loose(package, &rows[0]["phase"], limits)?;
    check(
        phase["original"] == rows[0]["original"]
            && phase["token"] == rows[0]["token"]
            && original["token"] == rows[0]["token"],
    )?;
    Ok((original, phase))
}
struct Prefix<'a, I> {
    package: &'a OriginalPackage,
    actual: &'a I,
    unit: u64,
    tip_charges: BTreeMap<String, u64>,
}
impl<I: AllocationInspection> AllocationInspection for Prefix<'_, I> {
    fn observe(&self, id: PackageUuid) -> Result<AllocationObservation, PackageError> {
        let current = self.actual.observe(id)?;
        let original = self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        let extent = original.bytes.len() as u64;
        let minimum = rounded(extent, self.unit)?;
        let charge = self
            .tip_charges
            .get(&id.to_string())
            .copied()
            .unwrap_or(current.registered_charge);
        check(
            charge >= minimum
                && (self.tip_charges.contains_key(&id.to_string()) || current.extent == extent),
        )?;
        check(
            current.layout == AllocationLayout::FramedJson
                && current.extent >= extent
                && current.registered_charge >= charge
                && serde_json::to_value(current.arena).map_err(|_| PackageError::Record)?
                    == original.arena
                && number(&original.witness["device"])? == current.device
                && number(&original.witness["inode"])? == current.inode,
        )?;
        Ok(AllocationObservation {
            extent,
            registered_charge: charge,
            ..current
        })
    }
    fn framed_prefix_digest(
        &self,
        id: PackageUuid,
        length: u64,
        max: u64,
    ) -> Result<crate::package::Sha256Hex, PackageError> {
        self.observe(id)?;
        check(length <= max)?;
        let a = self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        let prefix = a
            .bytes
            .get(..usize::try_from(length).map_err(|_| PackageError::Limit)?)
            .ok_or(PackageError::Integrity)?;
        let actual = self.actual.framed_prefix_digest(id, length, max)?;
        check(actual == digest(prefix))?;
        Ok(actual)
    }
}
struct Replayer<'a, 'm, I> {
    context: &'a RestartContext<'a, I>,
    memo: Option<&'m mut CompiledPlans>,
    seen: BTreeSet<String>,
}
impl<I: AllocationInspection> Replayer<'_, '_, I> {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Own bounded historical snapshot throughout recursive verification"
    )]
    fn verified(&mut self, package: OriginalPackage) -> Result<VerifiedOriginal, PackageError> {
        let e = loose(
            &package,
            &package.commit["root_set"]["placement"]["accounting"],
            self.context.reader.json,
        )?;
        // Empty holds are packed in the original settled codec. Nonempty
        // operation holds are loose and must replay their own exact original.
        let terminal = package
            .loose
            .contains_key(e["holds"]["sha256"].as_str().ok_or(PackageError::Record)?);
        let ledger = loose(&package, &e["ledger"], self.context.reader.json)?;
        let mut tip_charges = BTreeMap::new();
        for tip in ledger["tips"].as_array().ok_or(PackageError::Record)? {
            let id = tip["allocation_id"].as_str().ok_or(PackageError::Record)?;
            let allocation = package.allocations.get(id).ok_or(PackageError::Integrity)?;
            check(
                number(&tip["extent"])? == allocation.bytes.len() as u64
                    && tip_charges
                        .insert(id.to_owned(), number(&tip["registered_charge"])?)
                        .is_none(),
            )?;
        }
        let prefix = Prefix {
            package: &package,
            actual: self.context.inspection,
            unit: self.context.registration.charge_unit,
            tip_charges,
        };
        if !terminal {
            return v1_3::verify_original(
                package.clone(),
                identity(self.context.identity),
                &prefix,
                self.context.registration,
                self.context.directory,
                self.context.frames,
                self.context.reader,
            );
        }
        let plan = self.plan(&package)?;
        check(plan.stages()[6].head == package.head)?;
        if let Some(memo) = self.memo.as_deref_mut() {
            let context = self.context;
            let maximum = memo.maximum(context.planner).saturating_sub(memo.bytes);
            return memo.reader.verify(
                &package,
                &prefix,
                context,
                Some(&plan),
                maximum,
                None,
                |inspection| {
                    v1_3::reader::verify_operation_original(
                        package.clone(),
                        identity(context.identity),
                        inspection,
                        context.registration,
                        context.directory,
                        context.frames,
                        context.reader,
                        &plan,
                        6,
                    )
                },
            );
        }
        v1_3::reader::verify_operation_original(
            package.clone(),
            identity(self.context.identity),
            &prefix,
            self.context.registration,
            self.context.directory,
            self.context.frames,
            self.context.reader,
            &plan,
            6,
        )
    }
    fn plan(&mut self, current: &OriginalPackage) -> Result<RepeatablePlan, PackageError> {
        let (original, _phase) = controls(current, self.context.reader.json)?;
        let key = wire::hash(&wire::encode(&original));
        if self.seen.len() >= self.context.max_originals {
            return Err(PackageError::Limit);
        }
        check(self.seen.insert(key))?;
        check(
            original["scope"]
                == serde_json::json!({"profile":self.context.registration.profile,"incarnation":self.context.registration.incarnation.to_string(),"directory":{"device":self.context.directory.device.to_string(),"inode":self.context.directory.inode.to_string()}}),
        )?;
        let old = if let Some(memo) = self.memo.as_deref() {
            memo.reconstruct(current, &original, self.context.planner)?
        } else {
            recovery::reconstruct_original(current, &original, self.context.planner)?
        };
        let verified = self.verified(old)?;
        let intent = request(current, &original["request"]["intent"], self.context)?;
        let receipt = request(current, &original["request"]["receipt"], self.context)?;
        let plan = if let Some(memo) = self.memo.as_deref_mut() {
            Arc::try_unwrap(memo.recover(
                &verified,
                &original,
                &intent,
                &receipt,
                self.context.planner,
                self.context.max_originals,
                None,
            )?)
            .map_err(|_| PackageError::Integrity)?
        } else {
            recovery::recover(
                &verified,
                &original,
                &intent,
                &receipt,
                self.context.planner,
            )?
        };
        let stage = plan
            .stages()
            .iter()
            .position(|s| s.head == current.head)
            .ok_or(PackageError::Integrity)?;
        check(
            plan.stages()[stage].commit == current.commit
                && plan.stages()[stage].loose == current.loose,
        )?;
        check(current.allocations.len() == plan.files.len())?;
        let mut growth = 0u64;
        for (id, target) in &plan.files {
            let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
            let original = &plan.old.allocations[id];
            let start = original.bytes.len();
            check(
                actual.arena == original.arena
                    && actual.witness == original.witness
                    && actual.bytes.len() >= start
                    && target.starts_with(&actual.bytes),
            )?;
            let payload = *plan.payload_ends.get(id).unwrap_or(&start);
            let maximum = match stage {
                0 => start,
                1 | 2 => payload,
                _ => target.len(),
            };
            check(
                actual.bytes.len() <= maximum
                    && (stage < 2 || actual.bytes.len() >= payload)
                    && (stage < 4 || actual.bytes.len() == target.len()),
            )?;
            let before = if let Some(tip) = plan.original["old"]["ledger"]["tips"]
                .as_array()
                .ok_or(PackageError::Record)?
                .iter()
                .find(|t| t["allocation_id"] == *id)
            {
                number(&tip["registered_charge"])?
            } else {
                self.context
                    .inspection
                    .observe(PackageUuid::parse(id)?)?
                    .registered_charge
            };
            let after = rounded(
                actual.bytes.len() as u64,
                self.context.registration.charge_unit,
            )?
            .max(before);
            growth = growth
                .checked_add(after.checked_sub(before).ok_or(PackageError::Integrity)?)
                .ok_or(PackageError::Limit)?;
        }
        check(growth <= plan.reserved())?;
        Ok(plan)
    }
}
/// Regenerates the selected original using bounded persisted-chain recursion.
/// Current suffixes must be exact allowed prefixes of that same original recipe;
/// no bytes are rewritten, removed, or recovered from a golden package corpus.
/// # Errors
/// Refuses malformed/unsupported original chains, changed actual witnesses,
/// invalid old prefixes, unknown suffixes, selector differences and work limits.
pub fn restore_plan<I: AllocationInspection>(
    current: &OriginalPackage,
    context: &RestartContext<'_, I>,
) -> Result<RepeatablePlan, PackageError> {
    restore_plan_inner(current, context, None)
}
#[cfg(any(test, feature = "controlled-disposable"))]
pub(crate) fn restore_plan_cached<I: AllocationInspection>(
    current: &OriginalPackage,
    context: &RestartContext<'_, I>,
    memo: &mut CompiledPlans,
) -> Result<RepeatablePlan, PackageError> {
    memo.trim(context.planner, context.max_originals);
    restore_plan_inner(current, context, Some(memo))
}
fn restore_plan_inner<I: AllocationInspection>(
    current: &OriginalPackage,
    context: &RestartContext<'_, I>,
    memo: Option<&mut CompiledPlans>,
) -> Result<RepeatablePlan, PackageError> {
    check(context.directory.inode != 0 && context.registration.charge_unit > 0)?;
    let mut replay = Replayer {
        context,
        memo,
        seen: BTreeSet::new(),
    };
    let plan = replay.plan(current)?;
    for (id, a) in &current.allocations {
        let observed = context.inspection.observe(PackageUuid::parse(id)?)?;
        check(
            observed.extent == a.bytes.len() as u64
                && observed.device == number(&a.witness["device"])?
                && observed.inode == number(&a.witness["inode"])?
                && observed.registered_charge
                    >= rounded(a.bytes.len() as u64, context.registration.charge_unit)?,
        )?;
    }
    if let Some(stage) = plan
        .stages()
        .iter()
        .position(|s| s.head == current.head)
        .filter(|s| *s >= 5)
    {
        v1_3::reader::inspect_operation_package(
            current,
            identity(context.identity),
            context.inspection,
            context.registration,
            context.frames,
            context.reader,
            &plan,
            stage,
        )?;
    }
    Ok(plan)
}

/// Validate a persisted checkpoint intent before its first selected package
/// effect. This restores the original recipe, not a proof of current suffixes
/// or publication; execution must independently validate those before effects.
#[cfg(test)]
pub(crate) fn restore_intent_plan<I: AllocationInspection>(
    current: &OriginalPackage,
    original: &Value,
    intent: &Value,
    receipt: &Value,
    context: &RestartContext<'_, I>,
) -> Result<RepeatablePlan, PackageError> {
    IntentReplay::new(current, context).restore(original, intent, receipt)
}

#[cfg(any(test, feature = "controlled-disposable"))]
pub(crate) fn restore_intent_plan_cached<I: AllocationInspection>(
    current: &OriginalPackage,
    original: &Value,
    intent: &Value,
    receipt: &Value,
    context: &RestartContext<'_, I>,
    memo: &mut CompiledPlans,
) -> Result<RepeatablePlan, PackageError> {
    memo.trim(context.planner, context.max_originals);
    check(
        context.directory.inode != 0
            && context.registration.charge_unit > 0
            && context.max_originals > 0,
    )?;
    check(
        original["scope"]
            == serde_json::json!({"profile":context.registration.profile,"incarnation":context.registration.incarnation.to_string(),"directory":{"device":context.directory.device.to_string(),"inode":context.directory.inode.to_string()}}),
    )?;
    let old = memo.reconstruct(current, original, context.planner)?;
    if same_package(&old, current)
        && let Some(verified) = memo.current_original(current, context)?
    {
        let lineage = memo
            .journal
            .as_ref()
            .ok_or(PackageError::Integrity)?
            .lineage();
        check(!lineage.contains(&wire::hash(&wire::encode(original))))?;
        if lineage.len() >= context.max_originals {
            return Err(PackageError::Limit);
        }
        return Arc::try_unwrap(memo.recover(
            &verified,
            original,
            intent,
            receipt,
            context.planner,
            context.max_originals,
            None,
        )?)
        .map_err(|_| PackageError::Integrity);
    }
    let mut replay = Replayer {
        context,
        memo: Some(memo),
        seen: BTreeSet::from([wire::hash(&wire::encode(original))]),
    };
    let verified = replay.verified(old)?;
    Arc::try_unwrap(
        replay
            .memo
            .as_deref_mut()
            .ok_or(PackageError::Integrity)?
            .recover(
                &verified,
                original,
                intent,
                receipt,
                context.planner,
                context.max_originals,
                None,
            )?,
    )
    .map_err(|_| PackageError::Integrity)
}

/// Pure deterministic compilation memo. It never stores a live read/authority
/// proof: every hit requires a freshly verified original with exact bytes.
#[derive(Default)]
pub(crate) struct CompiledPlans {
    entries: BTreeMap<String, (CachedPlan, usize)>,
    reader: reader_memo::ReaderMemo,
    canonical: super::canonical_memo::CanonicalMemo,
    order: VecDeque<String>,
    bytes: usize,
    byte_ceiling: Option<usize>,
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) journal: Option<super::journal::JournalMemo>,
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) session: Option<super::journal::session::cache::SessionMemo>,
    #[cfg(test)]
    pub(crate) session_replayed: usize,
    #[cfg(test)]
    pub(crate) session_record_work: usize,
    #[cfg(any(test, feature = "controlled-disposable"))]
    candidates: snapshot_memo::Candidates,
    #[cfg(test)]
    pub(crate) candidate_checks: usize,
    #[cfg(test)]
    pub(crate) compilations: usize,
    #[cfg(test)]
    pub(crate) hits: usize,
    #[cfg(test)]
    pub(crate) journal_prefix_hashed: usize,
    #[cfg(test)]
    pub(crate) journal_replayed: usize,
    #[cfg(test)]
    pub(crate) journal_hits: usize,
}
impl CompiledPlans {
    pub(super) fn raw_maximum(&self, limits: Limits) -> usize {
        let maximum = usize::try_from(limits.max_total_allocation_bytes).unwrap_or(usize::MAX);
        self.byte_ceiling.map_or(maximum, |n| n.min(maximum))
    }
    fn maximum(&self, limits: Limits) -> usize {
        let maximum = self.raw_maximum(limits);
        let maximum = maximum
            .saturating_sub(prefix_index::allowance(maximum))
            .saturating_sub(self.canonical.bytes);
        #[cfg(any(test, feature = "controlled-disposable"))]
        let maximum = maximum
            .saturating_sub(self.journal.as_ref().map_or(0, |m| m.cost))
            .saturating_sub(self.session.as_ref().map_or(0, |m| m.cost))
            .saturating_sub(self.candidates.bytes);
        maximum
    }
    #[cfg(test)]
    pub(crate) fn set_test_ceiling(&mut self, maximum: usize) {
        self.byte_ceiling = Some(maximum);
    }

    pub(crate) fn trim(&mut self, limits: Limits, max_entries: usize) {
        self.canonical.set_maximum(self.raw_maximum(limits) / 4);
        #[cfg(any(test, feature = "controlled-disposable"))]
        {
            if self
                .journal
                .as_ref()
                .is_some_and(|m| m.cost > self.raw_maximum(limits) / 8)
            {
                self.journal = None;
            }
            if self
                .session
                .as_ref()
                .is_some_and(|m| m.cost > self.raw_maximum(limits) / 4)
            {
                self.session = None;
            }
            self.candidates.trim(self.raw_maximum(limits) / 16);
        }
        let maximum = self.maximum(limits);
        self.reader.trim_entries(max_entries);
        if self.bytes.saturating_add(self.reader.bytes) > maximum {
            self.reader.clear();
        }
        while self.entries.len() > max_entries || self.bytes > maximum {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            if let Some((_, size)) = self.entries.remove(&old) {
                self.bytes -= size;
            }
        }
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn inspect_candidate<I: AllocationInspection>(
        &mut self,
        package: &OriginalPackage,
        context: &RestartContext<'_, I>,
        plan: &RepeatablePlan,
        stage: usize,
    ) -> Result<(), PackageError> {
        self.trim(context.planner, context.max_originals);
        let _ = plan.operation_selection(stage)?;
        check(
            package.manifest == plan.old.manifest && package.allocations.len() == plan.files.len(),
        )?;
        for (id, expected) in &plan.files {
            check(
                package
                    .allocations
                    .get(id)
                    .is_some_and(|a| a.bytes == *expected),
            )?;
        }
        let selected = &plan.stages[stage];
        let key = wire::hash(&wire::encode(&serde_json::json!([
            reader_memo::fingerprint(package, context, Some(plan), None),
            stage,
            selected.head,
            selected.commit,
            selected
                .loose
                .iter()
                .map(|(k, v)| serde_json::json!([k, v.len(), wire::hash(v)]))
                .collect::<Vec<_>>()
        ])));
        if self.candidates.matches(&key, context.inspection)? {
            return Ok(());
        }
        let observed = snapshot_memo::Recording::new(
            context.inspection,
            snapshot_memo::Observations::default(),
        );
        let fresh = RestartContext {
            inspection: &observed,
            registration: context.registration,
            directory: context.directory,
            identity: context.identity,
            frames: context.frames,
            reader: context.reader,
            planner: context.planner,
            max_originals: context.max_originals,
        };
        let maximum = self.maximum(context.planner).saturating_sub(self.bytes);
        self.reader
            .inspect_candidate(package, &fresh, plan, stage, maximum)?;
        #[cfg(test)]
        {
            self.candidate_checks += 1;
        }
        self.candidates.insert(
            key,
            observed.finish(),
            self.raw_maximum(context.planner) / 16,
        );
        self.trim(context.planner, context.max_originals);
        Ok(())
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn current_original<I: AllocationInspection>(
        &mut self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<Option<VerifiedOriginal>, PackageError> {
        self.trim(context.planner, context.max_originals);
        let result = match &self.journal {
            Some(saved) => saved.current_original(current, context),
            None => Ok(None),
        };
        #[cfg(feature = "controlled-disposable")]
        if std::env::var_os("PHOTARA_PS3_TRACE_REPLAY").is_some() {
            eprintln!(
                "private current-original memo: {}",
                if matches!(&result, Ok(Some(_))) {
                    "hit"
                } else {
                    "miss"
                }
            );
        }
        result
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn snapshot_key<I>(
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> String {
        reader_memo::fingerprint(current, context, None, None)
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn snapshot_context<I>(
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> String {
        let scope = OriginalPackage {
            manifest: current.manifest.clone(),
            head: Value::Null,
            commit: Value::Null,
            loose: BTreeMap::new(),
            allocations: BTreeMap::new(),
        };
        reader_memo::fingerprint(&scope, context, None, None)
    }
    pub(crate) fn reconstruct(
        &self,
        current: &OriginalPackage,
        original: &Value,
        limits: Limits,
    ) -> Result<OriginalPackage, PackageError> {
        self.reconstruct_indexed(current, original, limits, None)
    }
    fn reconstruct_indexed(
        &self,
        current: &OriginalPackage,
        original: &Value,
        limits: Limits,
        prefixes: Option<&prefix_index::Prefixes<'_>>,
    ) -> Result<OriginalPackage, PackageError> {
        let bounds = planner_limits(limits);
        if let Some((cached, _)) = self
            .entries
            .values()
            .find(|(cached, _)| cached.plan.original == *original && cached.limits == bounds)
        {
            return recovery::reconstruct_cached(
                current,
                original,
                limits,
                &cached.plan.old,
                &cached.plan.original_objects,
                prefixes,
            );
        }
        recovery::reconstruct_original(current, original, limits)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Exact replay inputs and optional snapshot-bound hash index remain explicit"
    )]
    fn recover(
        &mut self,
        verified: &VerifiedOriginal,
        original: &Value,
        intent: &Value,
        receipt: &Value,
        limits: Limits,
        max_entries: usize,
        prefixes: Option<&prefix_index::Prefixes<'_>>,
    ) -> Result<Arc<RepeatablePlan>, PackageError> {
        self.trim(limits, max_entries);
        let basis = compiled_basis(verified, original, intent, receipt, limits);
        let key = wire::hash(&basis);
        if let Some((cached, _)) = self.entries.get(&key) {
            check(cached.plan.original == *original && cached.plan.prepared.receipt == *receipt)?;
            let plan = cached.materialize(verified.package(), prefixes)?;
            let supplied = wire::encode(intent);
            check(
                plan.stages[0].loose.get(
                    original["request"]["intent"]["sha256"]
                        .as_str()
                        .ok_or(PackageError::Record)?,
                ) == Some(&supplied),
            )?;
            #[cfg(test)]
            {
                self.hits += 1;
            }
            return Ok(Arc::new(plan));
        }
        #[cfg(test)]
        {
            self.compilations += 1;
        }
        let attempt: super::Attempt =
            serde_json::from_value(original["payload_recipe"]["planner"].clone())
                .map_err(|_| PackageError::Record)?;
        let plan = Arc::new(
            super::plan::compile_cached(
                verified,
                intent,
                receipt,
                attempt,
                limits,
                Some(&mut self.canonical),
            )
            .map_err(|_| PackageError::Integrity)?,
        );
        check(plan.original == *original)?;
        self.remember(&basis, &plan, limits, max_entries, false)?;
        Ok(plan)
    }
    /// Compile a fresh admission once and retain its exact deterministic output
    /// for subsequent pre-effect journal verification. This accepts no caller-
    /// constructed plan or reader proof; ordinary compilation still runs first.
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn compile(
        &mut self,
        verified: &VerifiedOriginal,
        intent: &Value,
        receipt: &Value,
        attempt: super::plan::Attempt,
        limits: Limits,
        max_entries: usize,
    ) -> wire::Result<RepeatablePlan> {
        self.trim(limits, max_entries);
        #[cfg(test)]
        {
            self.compilations += 1;
        }
        let plan = super::plan::compile_cached(
            verified,
            intent,
            receipt,
            attempt,
            limits,
            Some(&mut self.canonical),
        )?;
        let basis = compiled_basis(verified, plan.original(), intent, receipt, limits);
        self.remember(&basis, &plan, limits, max_entries, true)
            .map_err(|_| "compiled plan memo")?;
        Ok(plan)
    }
    fn remember(
        &mut self,
        basis: &[u8],
        plan: &RepeatablePlan,
        limits: Limits,
        max_entries: usize,
        fresh: bool,
    ) -> Result<(), PackageError> {
        self.trim(limits, max_entries);
        let maximum = self.maximum(limits);
        let key = wire::hash(basis);
        if self.entries.contains_key(&key) {
            return Ok(());
        }
        // Scan-resistant admission: a journal replay must not evict its warm
        // prefix merely because the immutable history exceeds the optional cache.
        // Oversize/new entries remain uncached, with identical verification.
        let cached = CachedPlan::compact(plan, limits)?;
        let cost = cached.byte_cost().and_then(|n| n.checked_add(basis.len()));
        // A newly admitted operation is the imminent retry working set. Unlike
        // a historical scan, it may displace older optional entries so it does
        // not compile again immediately before its first effects.
        if fresh
            && max_entries > 0
            && let Some(cost) = cost.filter(|cost| *cost <= maximum)
        {
            if self.reader.bytes.saturating_add(cost) > maximum {
                self.reader.clear();
            }
            while self.entries.len() >= max_entries
                || self
                    .bytes
                    .saturating_add(self.reader.bytes)
                    .saturating_add(cost)
                    > maximum
            {
                let Some(old) = self.order.pop_front() else {
                    break;
                };
                if let Some((_, size)) = self.entries.remove(&old) {
                    self.bytes -= size;
                }
            }
        }
        if let Some(cost) = cost.filter(|cost| {
            self.entries.len() < max_entries
                && self
                    .bytes
                    .checked_add(self.reader.bytes)
                    .and_then(|n| n.checked_add(*cost))
                    .is_some_and(|n| n <= maximum)
        }) {
            self.entries.insert(key.clone(), (cached, cost));
            self.order.push_back(key);
            self.bytes += cost;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn reader_work(&self) -> (usize, usize) {
        (self.reader.misses, self.reader.hits)
    }
    #[cfg(test)]
    pub(crate) fn retained(&self) -> (usize, usize) {
        (
            self.entries.len(),
            self.bytes
                + self.canonical.bytes
                + self.reader.bytes
                + self.journal.as_ref().map_or(0, |j| j.cost)
                + self.session.as_ref().map_or(0, |m| m.cost)
                + self.candidates.bytes,
        )
    }
}
fn compiled_basis(
    verified: &VerifiedOriginal,
    original: &Value,
    intent: &Value,
    receipt: &Value,
    limits: Limits,
) -> Vec<u8> {
    let retained = verified.retained_controls.iter().map(|(reference,membership)| {
            serde_json::json!({"record":reference,"membership":match membership {v1_3::Membership::Semantic=>"semantic",v1_3::Membership::Ownership=>"ownership",v1_3::Membership::Blob=>"blob"}})
        }).collect::<Vec<_>>();
    wire::encode(&serde_json::json!({
        "original":original,"intent":intent,"receipt":receipt,
        "scope":verified.scope,"charge_unit":verified.charge_unit,
        "empty_hold":verified.empty_hold,"retained":retained,
        "limits":planner_limits(limits)
    }))
}
pub(super) fn planner_limits(limits: Limits) -> Value {
    let json = limits.semantic.json;
    let semantic = limits.semantic;
    serde_json::json!([
        json.max_bytes as u64,
        json.max_depth as u64,
        json.max_members as u64,
        json.max_array_elements as u64,
        semantic.max_objects as u64,
        semantic.max_commits as u64,
        semantic.max_total_json_bytes as u64,
        semantic.max_blob_bytes,
        semantic.max_total_blob_bytes,
        limits.max_frames as u64,
        limits.max_total_allocation_bytes,
        limits.max_entries as u64,
        limits.max_objects as u64,
        limits.max_allocation_growth,
        limits.max_allocation_bytes
    ])
}
// Only immutable deterministic output is retained. Original pack bytes are
// supplied afresh, authenticated by the reader, and checked against these exact
// length/digest/arena/witness commitments before any cached suffix is appended.
struct CachedPlan {
    plan: RepeatablePlan,
    prefixes: BTreeMap<String, (usize, String)>,
    controls: BTreeMap<String, Vec<u8>>,
    stage_keys: Vec<Vec<String>>,
    limits: Value,
}
impl CachedPlan {
    fn compact(p: &RepeatablePlan, limits: Limits) -> Result<Self, PackageError> {
        let old = OriginalPackage {
            manifest: p.old.manifest.clone(),
            head: p.old.head.clone(),
            commit: p.old.commit.clone(),
            loose: p.old.loose.clone(),
            allocations: p
                .old
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        super::input::Allocation {
                            arena: a.arena.clone(),
                            witness: a.witness.clone(),
                            bytes: Vec::new(),
                        },
                    )
                })
                .collect(),
        };
        let mut prefixes = BTreeMap::new();
        let mut files = BTreeMap::new();
        for (id, allocation) in &p.old.allocations {
            let len = allocation.bytes.len();
            prefixes.insert(id.clone(), (len, wire::hash(&allocation.bytes)));
            let target = p.files.get(id).ok_or(PackageError::Integrity)?;
            check(target.starts_with(&allocation.bytes))?;
            files.insert(id.clone(), target[len..].to_vec());
        }
        check(files.len() == p.files.len())?;
        let mut controls = BTreeMap::new();
        let mut stage_keys = Vec::new();
        let stages = p
            .stages
            .iter()
            .map(|stage| {
                stage_keys.push(stage.loose.keys().cloned().collect());
                for (key, bytes) in &stage.loose {
                    if let Some(previous) = controls.insert(key.clone(), bytes.clone()) {
                        check(previous == *bytes)?;
                    }
                }
                Ok(super::plan::SelectedStage {
                    head: stage.head.clone(),
                    commit: stage.commit.clone(),
                    loose: BTreeMap::new(),
                })
            })
            .collect::<Result<Vec<_>, PackageError>>()?;
        Ok(Self {
            plan: RepeatablePlan {
                original_objects: p.original_objects.clone(),
                old,
                files,
                stages,
                original: p.original.clone(),
                payload_ends: p.payload_ends.clone(),
                prepared: p.prepared.clone(),
                reservation: p.reservation,
                peak: p.peak,
                control_role_peak: p.control_role_peak,
                attempt: p.attempt.clone(),
                empty_hold: p.empty_hold.clone(),
                retained_controls: p.retained_controls.clone(),
                extra_global: p.extra_global.clone(),
            },
            prefixes,
            controls,
            stage_keys,
            limits: planner_limits(limits),
        })
    }
    fn materialize(
        &self,
        current: &OriginalPackage,
        prefixes: Option<&prefix_index::Prefixes<'_>>,
    ) -> Result<RepeatablePlan, PackageError> {
        let p = &self.plan;
        check(
            current.manifest == p.old.manifest
                && current.head == p.old.head
                && current.commit == p.old.commit
                && current.loose == p.old.loose
                && current.allocations.len() == p.old.allocations.len(),
        )?;
        let mut files = BTreeMap::new();
        for (id, expected) in &p.old.allocations {
            let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
            let (length, digest) = self.prefixes.get(id).ok_or(PackageError::Integrity)?;
            check(
                actual.arena == expected.arena
                    && actual.witness == expected.witness
                    && actual.bytes.len() == *length
                    && prefixes
                        .map_or_else(|| wire::hash(&actual.bytes), |p| p.hash(id, &actual.bytes))
                        == *digest,
            )?;
            let suffix = p.files.get(id).ok_or(PackageError::Integrity)?;
            let mut bytes = Vec::with_capacity(
                actual
                    .bytes
                    .len()
                    .checked_add(suffix.len())
                    .ok_or(PackageError::Limit)?,
            );
            bytes.extend_from_slice(&actual.bytes);
            bytes.extend_from_slice(suffix);
            files.insert(id.clone(), bytes);
        }
        let stages = p
            .stages
            .iter()
            .zip(&self.stage_keys)
            .map(|(stage, keys)| {
                let loose = keys
                    .iter()
                    .map(|key| {
                        Ok((
                            key.clone(),
                            self.controls
                                .get(key)
                                .ok_or(PackageError::Integrity)?
                                .clone(),
                        ))
                    })
                    .collect::<Result<_, PackageError>>()?;
                Ok(super::plan::SelectedStage {
                    head: stage.head.clone(),
                    commit: stage.commit.clone(),
                    loose,
                })
            })
            .collect::<Result<_, PackageError>>()?;
        Ok(RepeatablePlan {
            original_objects: p.original_objects.clone(),
            old: current.clone(),
            files,
            stages,
            original: p.original.clone(),
            payload_ends: p.payload_ends.clone(),
            prepared: p.prepared.clone(),
            reservation: p.reservation,
            peak: p.peak,
            control_role_peak: p.control_role_peak,
            attempt: p.attempt.clone(),
            empty_hold: p.empty_hold.clone(),
            retained_controls: p.retained_controls.clone(),
            extra_global: p.extra_global.clone(),
        })
    }
    fn byte_cost(&self) -> Option<usize> {
        // Conservative owned-storage estimate, including Value/container overhead;
        // not allocator RSS or durable accounting. Prefix bytes are never retained.
        let p = &self.plan;
        let metadata = serde_json::json!({"old_manifest":p.old.manifest,"old_head":p.old.head,"old_commit":p.old.commit,
            "old_allocations":p.old.allocations.iter().map(|(id,a)| serde_json::json!([id,a.arena,a.witness])).collect::<Vec<_>>(),
            "original":p.original,"receipt":p.prepared.receipt,"authored":p.prepared.authored.values().collect::<Vec<_>>(),
            "stages":p.stages.iter().map(|s| serde_json::json!([s.head,s.commit])).collect::<Vec<_>>(),
            "attempt":p.attempt,"empty_hold":p.empty_hold,"prefixes":self.prefixes,"keys":self.stage_keys,
            "payload_ends":p.payload_ends,"retained_refs":p.retained_controls.keys().collect::<Vec<_>>(),"objects":p.original_objects.iter().collect::<Vec<_>>(),"limits":self.limits});
        let mut bytes = value_storage(&metadata)?;
        for (key, value) in p.old.loose.iter().chain(&p.files).chain(&self.controls) {
            bytes = bytes
                .checked_add(key.capacity())?
                .checked_add(value.capacity())?
                .checked_add(128)?;
        }
        // Authored-map digest keys and retained Membership tags are separate
        // from their Value bodies. Account for keys/nodes as well as metadata.
        for (digest, _) in p.prepared.authored.keys() {
            bytes = bytes.checked_add(digest.capacity())?.checked_add(256)?;
        }
        bytes = bytes.checked_add(p.retained_controls.len().checked_mul(256)?)?;
        bytes = bytes.checked_add(p.extra_global.len().checked_mul(256)?)?;
        bytes.checked_add(std::mem::size_of::<Self>())
    }
}
pub(super) fn value_storage(value: &Value) -> Option<usize> {
    let base = std::mem::size_of::<Value>();
    match value {
        Value::String(s) => base.checked_add(s.capacity()),
        Value::Array(a) => a.iter().try_fold(
            base.checked_add(a.capacity().checked_mul(base)?)?,
            |n, v| n.checked_add(value_storage(v)?),
        ),
        Value::Object(o) => o.iter().try_fold(base, |n, (k, v)| {
            n.checked_add(k.capacity())?
                .checked_add(128)?
                .checked_add(value_storage(v)?)
        }),
        _ => Some(base),
    }
}

/// One journal inspection only. A completed prefix can be reused as the next
/// original only after exact byte equality; no proof crosses snapshot boundaries.
/// Retain one predecessor, rather than every historical allocation copy.
pub(crate) struct IntentReplay<'a, 'b, I> {
    current: &'a OriginalPackage,
    context: &'a RestartContext<'b, I>,
    completed: Option<(VerifiedOriginal, BTreeSet<String>)>,
    lineage: BTreeSet<String>,
    pending: Option<String>,
    prefixes: prefix_index::Prefixes<'a>,
    resume: Option<(CompactOriginal, BTreeSet<String>)>,
}
impl<'a, 'b, I: AllocationInspection> IntentReplay<'a, 'b, I> {
    pub(crate) fn new(current: &'a OriginalPackage, context: &'a RestartContext<'b, I>) -> Self {
        Self {
            current,
            context,
            completed: None,
            lineage: BTreeSet::new(),
            pending: None,
            resume: None,
            prefixes: prefix_index::Prefixes::new(
                current,
                usize::try_from(context.planner.max_total_allocation_bytes).unwrap_or(usize::MAX),
            ),
        }
    }
    pub(super) fn seed(&mut self, original: CompactOriginal, lineage: BTreeSet<String>) {
        self.resume = Some((original, lineage));
    }
    pub(super) fn completed_metadata(&self) -> Option<(CompactOriginal, BTreeSet<String>)> {
        self.completed
            .as_ref()
            .map(|(v, l)| (CompactOriginal::new(v), l.clone()))
            .or_else(|| self.resume.clone())
    }
    #[cfg(test)]
    pub(crate) fn prefix_hashed(&self) -> usize {
        self.prefixes.hashed.get()
    }
    #[cfg(test)]
    pub(crate) fn restore(
        &mut self,
        original: &Value,
        intent: &Value,
        receipt: &Value,
    ) -> Result<RepeatablePlan, PackageError> {
        Arc::try_unwrap(self.restore_shared(original, intent, receipt, None)?)
            .map_err(|_| PackageError::Integrity)
    }
    pub(crate) fn restore_shared(
        &mut self,
        original: &Value,
        intent: &Value,
        receipt: &Value,
        mut memo: Option<&mut CompiledPlans>,
    ) -> Result<Arc<RepeatablePlan>, PackageError> {
        if let Some((saved, lineage)) = self.resume.take() {
            self.completed = Some((saved.materialize(self.current)?, lineage));
        }
        let context = self.context;
        if let Some(cache) = memo.as_deref() {
            self.prefixes.limit(cache.raw_maximum(context.planner));
        }
        check(context.directory.inode != 0 && context.registration.charge_unit > 0)?;
        check(context.max_originals > 0)?;
        check(
            original["scope"]
                == serde_json::json!({"profile":context.registration.profile,"incarnation":context.registration.incarnation.to_string(),"directory":{"device":context.directory.device.to_string(),"inode":context.directory.inode.to_string()}}),
        )?;
        let key = wire::hash(&wire::encode(original));
        let old = if let Some(cache) = memo.as_deref() {
            cache.reconstruct_indexed(
                self.current,
                original,
                context.planner,
                Some(&self.prefixes),
            )?
        } else {
            recovery::reconstruct_original(self.current, original, context.planner)?
        };
        let mut replay = Replayer {
            context,
            memo: None,
            seen: BTreeSet::from([key.clone()]),
        };
        let initial = if self.completed.is_none() {
            let envelope = loose(
                &old,
                &old.commit["root_set"]["placement"]["accounting"],
                context.reader.json,
            )?;
            let terminal = old.loose.contains_key(
                envelope["holds"]["sha256"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            );
            // Recursive terminal predecessors still collect the full cycle/depth
            // lineage. Only the settled header uses this baseline memo.
            Some(if terminal {
                replay.verified(old.clone())?
            } else if let Some(cache) = memo.as_deref_mut() {
                verify_original_prefix_cached(&old, context, cache)?
            } else {
                replay.verified(old.clone())?
            })
        } else {
            None
        };
        let mut recover = |verified: &VerifiedOriginal| {
            if let Some(memo) = memo.as_deref_mut() {
                memo.recover(
                    verified,
                    original,
                    intent,
                    receipt,
                    context.planner,
                    context.max_originals,
                    Some(&self.prefixes),
                )
            } else {
                recovery::recover(verified, original, intent, receipt, context.planner)
                    .map(Arc::new)
            }
        };
        let plan = if let Some((verified, ancestry)) = &self.completed {
            // A matching HEAD alone cannot authorize reuse of altered controls,
            // witnesses or payloads. Reconstruction checks actual current prefixes.
            check(same_package(&old, verified.package()))?;
            check(!ancestry.contains(&key))?;
            if ancestry.len() >= context.max_originals {
                return Err(PackageError::Limit);
            }
            replay.seen.extend(ancestry.iter().cloned());
            drop(old);
            recover(verified)?
        } else {
            drop(old);
            recover(initial.as_ref().ok_or(PackageError::Integrity)?)?
        };
        // The next pending plan owns its original already. Preserve the last
        // completed boundary as metadata while waiting for the receipt instead
        // of retaining another full historical package through its verification.
        if let Some((previous, lineage)) = self.completed.take() {
            self.resume = Some((CompactOriginal::new(&previous), lineage));
        }
        self.lineage = replay.seen;
        self.pending = Some(key);
        Ok(plan)
    }
    pub(crate) fn complete(
        &mut self,
        plan: &RepeatablePlan,
        memo: Option<&mut CompiledPlans>,
    ) -> Result<(), PackageError> {
        check(self.pending.as_ref() == Some(&wire::hash(&wire::encode(plan.original()))))?;
        let verified =
            verify_completed_plan(self.current, plan, self.context, memo, Some(&self.prefixes))?;
        self.completed = Some((verified, self.lineage.clone()));
        self.resume = None;
        self.pending = None;
        Ok(())
    }
}
fn same_package(a: &OriginalPackage, b: &OriginalPackage) -> bool {
    a.manifest == b.manifest
        && a.head == b.head
        && a.commit == b.commit
        && a.loose == b.loose
        && a.allocations.len() == b.allocations.len()
        && a.allocations
            .iter()
            .zip(&b.allocations)
            .all(|((aid, a), (bid, b))| {
                aid == bid && a.arena == b.arena && a.witness == b.witness && a.bytes == b.bytes
            })
}

/// Private historical view for the session's authenticated journal header base.
/// Actual current suffixes remain the executor's separate proof obligation.
pub(crate) fn verify_original_prefix<I: AllocationInspection>(
    package: OriginalPackage,
    context: &RestartContext<'_, I>,
) -> Result<VerifiedOriginal, PackageError> {
    check(
        context.directory.inode != 0
            && context.registration.charge_unit > 0
            && context.max_originals > 0,
    )?;
    Replayer {
        context,
        memo: None,
        seen: BTreeSet::new(),
    }
    .verified(package)
}

/// Same deterministic historical-base proof with fresh observation replay.
pub(crate) fn verify_original_prefix_cached<I: AllocationInspection>(
    package: &OriginalPackage,
    context: &RestartContext<'_, I>,
    memo: &mut CompiledPlans,
) -> Result<VerifiedOriginal, PackageError> {
    memo.trim(context.planner, context.max_originals);
    let maximum = memo.maximum(context.planner).saturating_sub(memo.bytes);
    memo.reader.verify(
        package,
        context.inspection,
        context,
        None,
        maximum,
        None,
        |inspection| {
            let fresh = RestartContext {
                inspection,
                registration: context.registration,
                directory: context.directory,
                identity: context.identity,
                frames: context.frames,
                reader: context.reader,
                planner: context.planner,
                max_originals: context.max_originals,
            };
            verify_original_prefix(package.clone(), &fresh)
        },
    )
}

/// Verify a checkpoint receipt's completed historical closure using only actual
/// retained prefixes. Later valid appends do not manufacture missing old bytes.
#[expect(
    clippy::too_many_lines,
    reason = "Keep exact current borrowing and historical-prefix fallback within the same receipt proof"
)]
pub(super) fn verify_completed_plan<I: AllocationInspection>(
    current: &OriginalPackage,
    plan: &RepeatablePlan,
    context: &RestartContext<'_, I>,
    memo: Option<&mut CompiledPlans>,
    prefixes: Option<&prefix_index::Prefixes<'_>>,
) -> Result<VerifiedOriginal, PackageError> {
    let stage = &plan.stages()[6];
    for (id, allocation) in &plan.old.allocations {
        let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
        let target = plan.files.get(id).ok_or(PackageError::Integrity)?;
        check(actual.arena == allocation.arena && actual.witness == allocation.witness)?;
        check(actual.bytes.starts_with(target))?;
    }
    // The most recent receipt usually selects the actual current bytes. Borrow
    // them; older receipts still get the exact bounded historical prefix view.
    let package = if current.manifest == plan.old.manifest
        && current.head == stage.head
        && current.commit == stage.commit
        && current.loose == stage.loose
        && current.allocations.len() == plan.files.len()
        && current
            .allocations
            .iter()
            .all(|(id, a)| plan.files.get(id) == Some(&a.bytes))
    {
        std::borrow::Cow::Borrowed(current)
    } else {
        std::borrow::Cow::Owned(OriginalPackage {
            manifest: plan.old.manifest.clone(),
            head: stage.head.clone(),
            commit: stage.commit.clone(),
            loose: stage.loose.clone(),
            allocations: plan
                .old
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        super::Allocation {
                            arena: a.arena.clone(),
                            witness: a.witness.clone(),
                            bytes: plan.files[id].clone(),
                        },
                    )
                })
                .collect(),
        })
    };
    let envelope = loose(
        &package,
        &package.commit["root_set"]["placement"]["accounting"],
        context.reader.json,
    )?;
    let ledger = loose(&package, &envelope["ledger"], context.reader.json)?;
    let tip_charges = ledger["tips"]
        .as_array()
        .ok_or(PackageError::Record)?
        .iter()
        .map(|tip| {
            Ok((
                tip["allocation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?
                    .to_owned(),
                number(&tip["registered_charge"])?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, PackageError>>()?;
    let prefix = Prefix {
        package: &package,
        actual: context.inspection,
        unit: context.registration.charge_unit,
        tip_charges,
    };
    if let Some(memo) = memo {
        let maximum = memo.maximum(context.planner).saturating_sub(memo.bytes);
        return memo.reader.verify(
            &package,
            &prefix,
            context,
            Some(plan),
            maximum,
            prefixes,
            |inspection| {
                v1_3::reader::verify_operation_original(
                    package.as_ref().clone(),
                    identity(context.identity),
                    inspection,
                    context.registration,
                    context.directory,
                    context.frames,
                    context.reader,
                    plan,
                    6,
                )
            },
        );
    }
    v1_3::reader::verify_operation_original(
        package.as_ref().clone(),
        identity(context.identity),
        &prefix,
        context.registration,
        context.directory,
        context.frames,
        context.reader,
        plan,
        6,
    )
}

#[derive(Clone)]
pub(super) struct CompactOriginal {
    package: OriginalPackage,
    lengths: BTreeMap<String, usize>,
    proof: super::input::OriginalProof,
    scope: Value,
    charge: u64,
    empty: ObjectRef,
    retained: BTreeMap<ObjectRef, v1_3::Membership>,
}
impl CompactOriginal {
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn selector_matches(&self, current: &OriginalPackage) -> bool {
        self.package.head == current.head
            && self.package.commit == current.commit
            && self.package.loose == current.loose
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn tips(&self, json: JsonLimits) -> Result<BTreeSet<PackageUuid>, PackageError> {
        let envelope = loose(
            &self.package,
            &self.package.commit["root_set"]["placement"]["accounting"],
            json,
        )?;
        let ledger = loose(&self.package, &envelope["ledger"], json)?;
        ledger["tips"]
            .as_array()
            .ok_or(PackageError::Record)?
            .iter()
            .map(|tip| {
                PackageUuid::parse(tip["allocation_id"].as_str().ok_or(PackageError::Record)?)
            })
            .collect()
    }
    fn new(v: &VerifiedOriginal) -> Self {
        let package = OriginalPackage {
            manifest: v.package.manifest.clone(),
            head: v.package.head.clone(),
            commit: v.package.commit.clone(),
            loose: v.package.loose.clone(),
            allocations: v
                .package
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        super::Allocation {
                            bytes: Vec::new(),
                            arena: a.arena.clone(),
                            witness: a.witness.clone(),
                        },
                    )
                })
                .collect(),
        };
        Self {
            package,
            lengths: v
                .package
                .allocations
                .iter()
                .map(|(id, a)| (id.clone(), a.bytes.len()))
                .collect(),
            proof: v.proof.clone(),
            scope: v.scope.clone(),
            charge: v.charge_unit,
            empty: v.empty_hold.clone(),
            retained: v.retained_controls.clone(),
        }
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn exact_current<I: AllocationInspection>(
        &self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<Option<VerifiedOriginal>, PackageError> {
        let p = &self.package;
        if p.manifest != current.manifest
            || p.head != current.head
            || p.commit != current.commit
            || p.loose != current.loose
            || p.allocations.len() != current.allocations.len()
            || current
                .allocations
                .iter()
                .any(|(id, a)| self.lengths.get(id) != Some(&a.bytes.len()))
        {
            return Ok(None);
        }
        // Historical validation clamps tip charge/extent. Reuse as a current
        // full proof only when those independent observations are identical.
        let envelope = loose(
            current,
            &current.commit["root_set"]["placement"]["accounting"],
            context.reader.json,
        )?;
        let ledger = loose(current, &envelope["ledger"], context.reader.json)?;
        for tip in ledger["tips"].as_array().ok_or(PackageError::Record)? {
            let id =
                PackageUuid::parse(tip["allocation_id"].as_str().ok_or(PackageError::Record)?)?;
            let actual = context.inspection.observe(id)?;
            if actual.registered_charge != number(&tip["registered_charge"])?
                || actual.extent
                    != current
                        .allocations
                        .get(&id.to_string())
                        .ok_or(PackageError::Integrity)?
                        .bytes
                        .len() as u64
            {
                return Ok(None);
            }
        }
        self.materialize(current).map(Some)
    }
    fn materialize(&self, current: &OriginalPackage) -> Result<VerifiedOriginal, PackageError> {
        let mut package = self.package.clone();
        check(package.allocations.len() == current.allocations.len())?;
        for (id, a) in &mut package.allocations {
            let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
            check(actual.arena == a.arena && actual.witness == a.witness)?;
            a.bytes = actual
                .bytes
                .get(..*self.lengths.get(id).ok_or(PackageError::Integrity)?)
                .ok_or(PackageError::Integrity)?
                .to_vec();
        }
        let mut verified = VerifiedOriginal::from_reader(
            package,
            self.proof.clone(),
            self.scope.clone(),
            self.charge,
            self.empty.clone(),
        );
        verified.retained_controls = self.retained.clone();
        Ok(verified)
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn cost(&self) -> Option<usize> {
        let p = &self.package;
        let data = serde_json::json!({"manifest":p.manifest,"head":p.head,"commit":p.commit,"allocations":p.allocations.iter().map(|(id,a)|serde_json::json!([id,a.arena,a.witness])).collect::<Vec<_>>(),"lengths":self.lengths,"roles":self.proof.roles.iter().map(|(id,r)|serde_json::json!([id,r.state,r.semantic,r.receipts])).collect::<Vec<_>>(),"global":self.proof.global,"scope":self.scope,"empty":self.empty,"retained":self.retained.iter().map(|(r,m)|serde_json::json!([r,format!("{m:?}")])).collect::<Vec<_>>()});
        p.loose.iter().try_fold(value_storage(&data)?, |n, (k, v)| {
            n.checked_add(k.capacity())?
                .checked_add(v.capacity())?
                .checked_add(256)
        })
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Opt-in immutable capture benchmark assertions"
)]
pub(super) fn lookup_profile(
    package: &OriginalPackage,
    frames: FrameLimits,
    reader: &ReaderLimits,
    identity: &SelectionIdentity,
) {
    use v1_3::PhysicalRecords;
    struct Counted<'a> {
        inner: Packed<'a>,
        reads: std::cell::Cell<usize>,
    }
    impl PhysicalRecords for Counted<'_> {
        fn resolve(
            &self,
            r: &v1_3::PhysicalRef,
            kind: v1_3::FrameKind,
        ) -> Result<Value, PackageError> {
            self.reads.set(self.reads.get() + 1);
            self.inner.resolve(r, kind)
        }
    }
    let provider = Counted {
        inner: Packed {
            package,
            limits: frames,
            checked: RefCell::new(BTreeMap::new()),
        },
        reads: std::cell::Cell::new(0),
    };
    let envelope = v1_3::SelectedEnvelope::parse(
        &wire::encode(&package.manifest),
        &wire::encode(&package.head),
        &wire::encode(&package.commit),
        v1_3::SelectionIdentity {
            project: identity.project,
            library: identity.library,
            bootstrap_sha256: identity.bootstrap_sha256.clone(),
        },
        reader.json,
    )
    .unwrap();
    let state = envelope
        .open_role(&provider, v1_3::SelectedRole::Active, reader.tree)
        .unwrap();
    let tree = v1_3::PhysicalTree::new(
        &provider,
        identity.project,
        v1_3::PhysicalTreeKind::Locator,
        reader.tree,
    );
    let all = tree.audit(&state.locator).unwrap();
    let keys = all
        .iter()
        .take(20)
        .map(|v| serde_json::from_value::<ObjectRef>(v["object"].clone()).unwrap())
        .collect::<Vec<_>>();
    provider.reads.set(0);
    let before = std::time::Instant::now();
    for key in &keys {
        let entries = tree.audit(&state.locator).unwrap();
        assert!(
            entries
                .iter()
                .any(|v| serde_json::from_value::<ObjectRef>(v["object"].clone()).unwrap() == *key)
        );
    }
    let full = (before.elapsed(), provider.reads.replace(0));
    let before = std::time::Instant::now();
    for key in &keys {
        assert!(tree.lookup(&state.locator, key).unwrap().is_some());
    }
    let scoped = (before.elapsed(), provider.reads.get());
    eprintln!(
        "captured scoped locator: keys={} entries={} full={full:?} scoped={scoped:?}",
        keys.len(),
        all.len()
    );
    assert!(scoped.1 < full.1);
}
