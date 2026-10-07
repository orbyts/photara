//! Restart from selected original records and current supplied bytes, without a
//! retained in-memory plan. Historical prefix views never certify current tails.
use super::{Limits, OriginalPackage, RepeatablePlan, VerifiedOriginal, recovery, wire};
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
    collections::{BTreeMap, BTreeSet},
};

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
struct Replayer<'a, I> {
    context: &'a RestartContext<'a, I>,
    seen: BTreeSet<String>,
}
impl<I: AllocationInspection> Replayer<'_, I> {
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
        let old = recovery::reconstruct_original(current, &original, self.context.planner)?;
        let verified = self.verified(old)?;
        let intent = request(current, &original["request"]["intent"], self.context)?;
        let receipt = request(current, &original["request"]["receipt"], self.context)?;
        let plan = recovery::recover(
            &verified,
            &original,
            &intent,
            &receipt,
            self.context.planner,
        )?;
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
    check(context.directory.inode != 0 && context.registration.charge_unit > 0)?;
    let mut replay = Replayer {
        context,
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
#[cfg(any(test, feature = "controlled-disposable"))]
pub(crate) fn restore_intent_plan<I: AllocationInspection>(
    current: &OriginalPackage,
    original: &Value,
    intent: &Value,
    receipt: &Value,
    context: &RestartContext<'_, I>,
) -> Result<RepeatablePlan, PackageError> {
    IntentReplay::new(current, context).restore(original, intent, receipt)
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
}
impl<'a, 'b, I: AllocationInspection> IntentReplay<'a, 'b, I> {
    pub(crate) fn new(current: &'a OriginalPackage, context: &'a RestartContext<'b, I>) -> Self {
        Self {
            current,
            context,
            completed: None,
            lineage: BTreeSet::new(),
            pending: None,
        }
    }
    pub(crate) fn restore(
        &mut self,
        original: &Value,
        intent: &Value,
        receipt: &Value,
    ) -> Result<RepeatablePlan, PackageError> {
        let context = self.context;
        check(context.directory.inode != 0 && context.registration.charge_unit > 0)?;
        check(context.max_originals > 0)?;
        check(
            original["scope"]
                == serde_json::json!({"profile":context.registration.profile,"incarnation":context.registration.incarnation.to_string(),"directory":{"device":context.directory.device.to_string(),"inode":context.directory.inode.to_string()}}),
        )?;
        let key = wire::hash(&wire::encode(original));
        let old = recovery::reconstruct_original(self.current, original, context.planner)?;
        let mut replay = Replayer {
            context,
            seen: BTreeSet::from([key.clone()]),
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
            recovery::recover(verified, original, intent, receipt, context.planner)?
        } else {
            let verified = replay.verified(old)?;
            recovery::recover(&verified, original, intent, receipt, context.planner)?
        };
        self.lineage = replay.seen;
        self.pending = Some(key);
        Ok(plan)
    }
    pub(crate) fn complete(&mut self, plan: &RepeatablePlan) -> Result<(), PackageError> {
        check(self.pending.as_ref() == Some(&wire::hash(&wire::encode(plan.original()))))?;
        let verified = verify_completed_plan(self.current, plan, self.context)?;
        self.completed = Some((verified, self.lineage.clone()));
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
#[cfg(any(test, feature = "controlled-disposable"))]
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
        seen: BTreeSet::new(),
    }
    .verified(package)
}

/// Verify a checkpoint receipt's completed historical closure using only actual
/// retained prefixes. Later valid appends do not manufacture missing old bytes.
fn verify_completed_plan<I: AllocationInspection>(
    current: &OriginalPackage,
    plan: &RepeatablePlan,
    context: &RestartContext<'_, I>,
) -> Result<VerifiedOriginal, PackageError> {
    let stage = &plan.stages()[6];
    let mut package = plan.old.clone();
    package.head = stage.head.clone();
    package.commit = stage.commit.clone();
    package.loose = stage.loose.clone();
    for (id, allocation) in &mut package.allocations {
        let actual = current.allocations.get(id).ok_or(PackageError::Integrity)?;
        let target = plan.files.get(id).ok_or(PackageError::Integrity)?;
        check(actual.arena == allocation.arena && actual.witness == allocation.witness)?;
        check(actual.bytes.starts_with(target))?;
        allocation.bytes.clone_from(target);
    }
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
    v1_3::reader::verify_operation_original(
        package.clone(),
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
