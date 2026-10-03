//! Byte-bound adapter into the repeatable planner's private verified input.
use super::super::repeatable;
use super::super::tree::{fields, number};
use super::super::{
    AllocationInspection, AllocationLayout, AllocationObservation, Arena, BlobMetadata, FrameKind,
    FrameLimits, PackedAllocation, PhysicalRecords, PhysicalRef, SelectedEnvelope,
    SelectionIdentity,
};
use super::{
    ControlRecords, OperationSelection, ReaderLimits, SettledRegistration, check, controls,
    inspect_selected,
};
use crate::package::{
    DecimalU64, ObjectKind, ObjectRef, PackageError, PackageUuid, Sha256Hex, digest,
};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

/// Original directory identity comparison, supplied at the registration boundary.
/// It neither acquires a lease nor qualifies native durability.
#[derive(Clone, Copy)]
pub struct DirectoryObservation {
    pub device: u64,
    pub inode: u64,
}
struct Bytes<'a, M> {
    package: &'a repeatable::OriginalPackage,
    inspection: &'a M,
    frames: FrameLimits,
    checked: RefCell<BTreeMap<PackageUuid, PackedAllocation<'a>>>,
}
impl<'a, M> PhysicalRecords for Bytes<'a, M> {
    fn resolve(&self, r: &PhysicalRef, k: FrameKind) -> Result<Value, PackageError> {
        if !self.checked.borrow().contains_key(&r.allocation_id) {
            let package: &'a repeatable::OriginalPackage = self.package;
            let a = package
                .allocations
                .get(&r.allocation_id.to_string())
                .ok_or(PackageError::Integrity)?;
            let arena: Arena = serde_json::from_value(Value::String(a.arena.clone()))
                .map_err(|_| PackageError::Record)?;
            let allocation = PackedAllocation::open(r.allocation_id, arena, &a.bytes, self.frames)?;
            self.checked
                .borrow_mut()
                .insert(r.allocation_id, allocation);
        }
        self.checked
            .borrow()
            .get(&r.allocation_id)
            .ok_or(PackageError::Integrity)?
            .resolve(r, k)
    }
}
impl<M> ControlRecords for Bytes<'_, M> {
    fn controls(&self) -> Result<BTreeSet<ObjectRef>, PackageError> {
        self.package
            .loose
            .iter()
            .map(|(sha, b)| {
                check(digest(b).as_str() == sha)?;
                Ok(ObjectRef {
                    kind: ObjectKind::Json,
                    sha256: digest(b),
                    byte_length: DecimalU64::parse(&b.len().to_string())?,
                })
            })
            .collect()
    }
    fn read(&self, r: &ObjectRef) -> Result<Vec<u8>, PackageError> {
        self.package
            .loose
            .get(r.sha256.as_str())
            .cloned()
            .ok_or(PackageError::Integrity)
    }
}
impl<M> BlobMetadata for Bytes<'_, M> {
    fn extent(&self, id: PackageUuid) -> Result<u64, PackageError> {
        Ok(self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?
            .bytes
            .len() as u64)
    }
}
impl<M: AllocationInspection> AllocationInspection for Bytes<'_, M> {
    fn observe(&self, id: PackageUuid) -> Result<AllocationObservation, PackageError> {
        let observed = self.inspection.observe(id)?;
        let a = self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        fields(&a.witness, &["device", "inode"])?;
        check(
            observed.layout == AllocationLayout::FramedJson
                && observed.extent == a.bytes.len() as u64
                && observed.device == number(&a.witness["device"])?
                && observed.inode == number(&a.witness["inode"])?
                && serde_json::to_value(observed.arena).map_err(|_| PackageError::Record)?
                    == a.arena,
        )?;
        Ok(observed)
    }
    fn framed_prefix_digest(
        &self,
        id: PackageUuid,
        n: u64,
        max: u64,
    ) -> Result<Sha256Hex, PackageError> {
        self.observe(id)?;
        if n > max {
            return Err(PackageError::Limit);
        }
        let a = self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        let prefix = a
            .bytes
            .get(..usize::try_from(n).map_err(|_| PackageError::Limit)?)
            .ok_or(PackageError::Integrity)?;
        let original = self.inspection.framed_prefix_digest(id, n, max)?;
        check(digest(prefix) == original)?;
        Ok(original)
    }
}
/// Validates exactly the package subsequently handed to the pure planner. No
/// caller-supplied closure DTO or proof from different bytes can enter the wrapper.
/// Current repeatable Graph layout is framed JSON only; raw Blob packages remain
/// readable through `inspect_settled` and require a distinct mutation planner.
/// # Errors
/// Refuses incomplete selected closure, mismatched observations, nil directory
/// identity, unsupported physical layout or any caller resource limit.
pub fn verify_original(
    package: repeatable::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    directory: DirectoryObservation,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<repeatable::VerifiedOriginal, PackageError> {
    verify_original_inner(
        package,
        identity,
        inspection,
        registration,
        directory,
        frames,
        limits,
        None,
    )
}
#[expect(
    clippy::too_many_arguments,
    reason = "Separate raw bytes, trusted registration and exact regenerated operation"
)]
fn verify_original_inner(
    package: repeatable::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    directory: DirectoryObservation,
    frames: FrameLimits,
    limits: &ReaderLimits,
    operation: Option<&OperationSelection<'_>>,
) -> Result<repeatable::VerifiedOriginal, PackageError> {
    check(directory.inode != 0)?;
    let canonical = |v: &Value| photara_core::canonical_json(v).map_err(|_| PackageError::Record);
    let envelope = SelectedEnvelope::parse(
        &canonical(&package.manifest)?,
        &canonical(&package.head)?,
        &canonical(&package.commit)?,
        identity,
        limits.json,
    )?;
    check(
        package
            .allocations
            .keys()
            .map(|id| PackageUuid::parse(id))
            .collect::<Result<BTreeSet<_>, _>>()?
            == registration.allocations,
    )?;
    let bytes = Bytes {
        package: &package,
        inspection,
        frames,
        checked: RefCell::new(BTreeMap::new()),
    };
    let proof = inspect_selected(
        &envelope,
        &bytes,
        &bytes,
        &bytes,
        registration,
        limits,
        operation,
    )?;
    let empty_hold = controls(&envelope, &bytes, limits, operation)?.holds;
    drop(bytes);
    let to_key = |r: &ObjectRef| -> Result<(String, u64), PackageError> {
        check(r.kind == ObjectKind::Json)?;
        Ok((r.sha256.as_str().to_owned(), r.byte_length.get()))
    };
    let roles = proof
        .roles
        .into_iter()
        .map(|(label, r)| {
            Ok((
                label,
                repeatable::Role {
                    state: r.state,
                    semantic: r.semantic.iter().map(to_key).collect::<Result<_, _>>()?,
                    receipts: r.receipts,
                },
            ))
        })
        .collect::<Result<_, PackageError>>()?;
    let original = repeatable::OriginalProof {
        roles,
        global: proof.global.iter().map(to_key).collect::<Result<_, _>>()?,
    };
    let scope = serde_json::json!({"profile":registration.profile,"incarnation":registration.incarnation.to_string(),"directory":{"device":directory.device.to_string(),"inode":directory.inode.to_string()}});
    Ok(repeatable::VerifiedOriginal::from_reader(
        package,
        original,
        scope,
        registration.charge_unit,
        empty_hold,
    ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Plan-bound selected inspection with independent physical observations"
)]
pub(crate) fn inspect_operation_package(
    package: &repeatable::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    frames: FrameLimits,
    limits: &ReaderLimits,
    plan: &repeatable::RepeatablePlan,
    stage: usize,
) -> Result<super::SettledPackage, PackageError> {
    check(package.manifest == plan.old.manifest && package.allocations.len() == plan.files.len())?;
    for (id, expected) in &plan.files {
        check(
            package
                .allocations
                .get(id)
                .is_some_and(|a| a.bytes == *expected),
        )?;
    }
    let selected = plan.operation_selection(stage)?;
    let canonical = |v: &Value| photara_core::canonical_json(v).map_err(|_| PackageError::Record);
    let envelope = SelectedEnvelope::parse(
        &canonical(&package.manifest)?,
        &canonical(&package.head)?,
        &canonical(&package.commit)?,
        identity,
        limits.json,
    )?;
    let bytes = Bytes {
        package,
        inspection,
        frames,
        checked: RefCell::new(BTreeMap::new()),
    };
    inspect_selected(
        &envelope,
        &bytes,
        &bytes,
        &bytes,
        registration,
        limits,
        Some(&selected),
    )
}
#[expect(
    clippy::too_many_arguments,
    reason = "Only exact clean operation may mint a subsequent planning input"
)]
pub(crate) fn verify_operation_original(
    package: repeatable::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    directory: DirectoryObservation,
    frames: FrameLimits,
    limits: &ReaderLimits,
    plan: &repeatable::RepeatablePlan,
    stage: usize,
) -> Result<repeatable::VerifiedOriginal, PackageError> {
    check(
        stage == 6
            && package.manifest == plan.old.manifest
            && package.allocations.len() == plan.files.len(),
    )?;
    for (id, expected) in &plan.files {
        check(
            package
                .allocations
                .get(id)
                .is_some_and(|a| a.bytes == *expected),
        )?;
    }
    let scope = serde_json::json!({"profile":registration.profile,"incarnation":registration.incarnation.to_string(),"directory":{"device":directory.device.to_string(),"inode":directory.inode.to_string()}});
    check(scope == plan.original["scope"])?;
    let selected = plan.operation_selection(stage)?;
    let mut verified = verify_original_inner(
        package,
        identity,
        inspection,
        registration,
        directory,
        frames,
        limits,
        Some(&selected),
    )?;
    verified.retained_controls = plan.retained_controls.clone();
    Ok(verified)
}
