//! Byte-bound adapter into the repeatable planner's private verified input.
use super::super::repeatable;
use super::super::tree::{fields, number};
use super::super::{
    AllocationInspection, AllocationLayout, AllocationObservation, Arena, BlobMetadata, FrameKind,
    FrameLimits, PackedAllocation, PhysicalRecords, PhysicalRef, SelectedAllocation,
    SelectedEnvelope, SelectionIdentity,
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
    package: &'a repeatable::shared::OriginalPackage,
    inspection: &'a M,
    frames: FrameLimits,
    snapshot: bool,
    checked: RefCell<BTreeMap<PackageUuid, Checked<'a>>>,
}
enum Checked<'a> {
    Legacy(PackedAllocation<'a>),
    Snapshot(SelectedAllocation<'a>),
}
impl Checked<'_> {
    fn resolve(&self, reference: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError> {
        match self {
            Self::Legacy(allocation) => allocation.resolve(reference, kind),
            Self::Snapshot(allocation) => allocation.resolve(reference, kind),
        }
    }
    fn resolve_json(
        &self,
        reference: &PhysicalRef,
        kind: FrameKind,
        object: &ObjectRef,
    ) -> Result<Value, PackageError> {
        match self {
            Self::Legacy(allocation) => allocation.resolve_json(reference, kind, object),
            Self::Snapshot(allocation) => allocation.resolve_json(reference, kind, object),
        }
    }
}
impl<'a, M> Bytes<'a, M> {
    fn open(&self, r: &PhysicalRef) -> Result<(), PackageError> {
        if !self.checked.borrow().contains_key(&r.allocation_id) {
            let package: &'a repeatable::shared::OriginalPackage = self.package;
            let a = package
                .allocations
                .get(&r.allocation_id.to_string())
                .ok_or(PackageError::Integrity)?;
            let arena: Arena = serde_json::from_value(Value::String(a.arena.clone()))
                .map_err(|_| PackageError::Record)?;
            let allocation = if self.snapshot {
                Checked::Snapshot(SelectedAllocation::open(
                    r.allocation_id,
                    arena,
                    &a.bytes,
                    self.frames,
                )?)
            } else {
                Checked::Legacy(PackedAllocation::open(
                    r.allocation_id,
                    arena,
                    &a.bytes,
                    self.frames,
                )?)
            };
            self.checked
                .borrow_mut()
                .insert(r.allocation_id, allocation);
        }
        Ok(())
    }
}
impl<M> PhysicalRecords for Bytes<'_, M> {
    fn audit_legacy_framing(&self, id: PackageUuid) -> Result<(), PackageError> {
        let a = self
            .package
            .allocations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        let arena = serde_json::from_value(Value::String(a.arena.clone()))
            .map_err(|_| PackageError::Record)?;
        PackedAllocation::open(id, arena, &a.bytes, self.frames)?;
        Ok(())
    }

    fn resolve_json(
        &self,
        r: &PhysicalRef,
        kind: FrameKind,
        object: &ObjectRef,
    ) -> Result<Value, PackageError> {
        self.open(r)?;
        self.checked
            .borrow()
            .get(&r.allocation_id)
            .ok_or(PackageError::Integrity)?
            .resolve_json(r, kind, object)
    }

    fn resolve(&self, r: &PhysicalRef, k: FrameKind) -> Result<Value, PackageError> {
        self.open(r)?;
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
        package.into(),
        identity,
        inspection,
        registration,
        directory,
        frames,
        limits,
        None,
    )
}
pub(crate) fn verify_shared_original(
    package: repeatable::shared::OriginalPackage,
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
    package: repeatable::shared::OriginalPackage,
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
        snapshot: envelope.root["schema"]["version"] == 3,
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
    package: &repeatable::shared::OriginalPackage,
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
        snapshot: envelope.root["schema"]["version"] == 3,
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
    package: repeatable::shared::OriginalPackage,
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

/// First adoption reuses the exact selected old active locator and performs the
/// full reciprocal operation audit; a claimed base is never enough by itself.
pub(crate) fn audit_checkpoint_adoption(
    original: &repeatable::VerifiedOriginal,
    base: &super::super::snapshot::AcceptanceBase,
    identity: &SelectionIdentity,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<(), PackageError> {
    use super::super::{Locator, PhysicalTree, PhysicalTreeKind};
    let package = original.shared_package();
    let provider = Bytes {
        package,
        inspection: &(),
        frames,
        snapshot: package.commit["root_set"]["schema"]["version"] == 3,
        checked: RefCell::new(BTreeMap::new()),
    };
    let active = original
        .proof
        .roles
        .get("active")
        .ok_or(PackageError::Integrity)?;
    let reference = super::super::selected::json_ref(&package.commit["root_set"]["active"])?;
    let placement = PhysicalTree::new(
        &provider,
        identity.project,
        PhysicalTreeKind::RootPlacement,
        limits.tree,
    )
    .lookup(
        &super::super::selected::physical(
            &package.commit["root_set"]["placement"]["root_placements"],
        )?,
        &reference,
    )?
    .ok_or(PackageError::Integrity)?;
    let locator = Locator::new(
        &provider,
        identity.project,
        super::super::selected::physical(&placement["locator"])?,
        limits.tree,
    );
    base.audit_adoption(
        &locator,
        &package.commit,
        &active.state,
        identity,
        limits.tree,
    )
}

/// Materialize only the already authenticated live Core graph/history closure.
/// Historical operation/archive frames remain lookup edges and are not scanned.
pub(crate) fn snapshot_core_objects(
    original: &repeatable::VerifiedOriginal,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<BTreeMap<(String, u64), Value>, PackageError> {
    use super::super::{Locator, Membership, PhysicalTree, PhysicalTreeKind};
    let package = original.shared_package();
    check(package.commit["root_set"]["schema"]["version"] == 3)?;
    let project = PackageUuid::parse(
        package.manifest["project_id"]
            .as_str()
            .ok_or(PackageError::Record)?,
    )?;
    let provider = Bytes {
        package,
        inspection: &(),
        frames,
        snapshot: true,
        checked: RefCell::new(BTreeMap::new()),
    };
    let active = original
        .proof
        .roles
        .get("active")
        .ok_or(PackageError::Integrity)?;
    let reference = super::super::selected::json_ref(&package.commit["root_set"]["active"])?;
    let placement = PhysicalTree::new(
        &provider,
        project,
        PhysicalTreeKind::RootPlacement,
        limits.tree,
    )
    .lookup(
        &super::super::selected::physical(
            &package.commit["root_set"]["placement"]["root_placements"],
        )?,
        &reference,
    )?
    .ok_or(PackageError::Integrity)?;
    let locator = Locator::new(
        &provider,
        project,
        super::super::selected::physical(&placement["locator"])?,
        limits.tree,
    );
    let mut pending = vec![
        super::super::selected::json_ref(&active.state["authored"])?,
        super::super::selected::json_ref(&active.state["history"])?,
    ];
    let mut objects = BTreeMap::new();
    let mut bytes = 0usize;
    while let Some(reference) = pending.pop() {
        let key = (
            reference.sha256.as_str().to_owned(),
            reference.byte_length.get(),
        );
        if objects.contains_key(&key) {
            continue;
        }
        check(active.semantic.contains(&key))?;
        if objects.len() >= limits.semantic.max_objects {
            return Err(PackageError::Limit);
        }
        bytes = bytes
            .checked_add(
                usize::try_from(reference.byte_length.get()).map_err(|_| PackageError::Limit)?,
            )
            .ok_or(PackageError::Limit)?;
        if bytes > limits.semantic.max_total_json_bytes {
            return Err(PackageError::Limit);
        }
        let value = locator.json(&reference, Membership::Semantic)?;
        let mut edges = Vec::new();
        crate::package::v1_1::references(&value, &mut edges)?;
        pending.extend(edges);
        objects.insert(key, value);
    }
    Ok(objects)
}

pub(crate) fn snapshot_base_binding(
    original: &repeatable::VerifiedOriginal,
    checkpoint: &ObjectRef,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<(), PackageError> {
    use super::super::{Locator, Membership, PhysicalTree, PhysicalTreeKind};
    let package = original.shared_package();
    let project = PackageUuid::parse(
        package.manifest["project_id"]
            .as_str()
            .ok_or(PackageError::Record)?,
    )?;
    let provider = Bytes {
        package,
        inspection: &(),
        frames,
        snapshot: true,
        checked: RefCell::new(BTreeMap::new()),
    };
    let active = original
        .proof
        .roles
        .get("active")
        .ok_or(PackageError::Integrity)?;
    check(active.state["schema"]["version"] == 3 && active.receipts.is_empty())?;
    let reference = super::super::selected::json_ref(&package.commit["root_set"]["active"])?;
    let placement = PhysicalTree::new(
        &provider,
        project,
        PhysicalTreeKind::RootPlacement,
        limits.tree,
    )
    .lookup(
        &super::super::selected::physical(
            &package.commit["root_set"]["placement"]["root_placements"],
        )?,
        &reference,
    )?
    .ok_or(PackageError::Integrity)?;
    let locator = Locator::new(
        &provider,
        project,
        super::super::selected::physical(&placement["locator"])?,
        limits.tree,
    );
    let index = locator.json(
        &super::super::selected::json_ref(&active.state["operation_index"])?,
        Membership::Semantic,
    )?;
    check(
        index["schema"] == serde_json::json!({"id":"photara.package.operation-index","version":3})
            && super::super::selected::json_ref(&index["base"])? == *checkpoint,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Exact maintenance original plus independent observations"
)]
pub(crate) fn verify_checkpoint_original(
    package: repeatable::shared::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    directory: DirectoryObservation,
    frames: FrameLimits,
    limits: &ReaderLimits,
    plan: &repeatable::checkpoint::TurnoverPlan,
    stage: usize,
) -> Result<repeatable::VerifiedOriginal, PackageError> {
    check(
        (2..=3).contains(&stage)
            && package.manifest == plan.old.manifest
            && package.allocations.len() == plan.files.len(),
    )?;
    for (id, bytes) in &plan.files {
        let actual = package.allocations.get(id).ok_or(PackageError::Integrity)?;
        let old = plan
            .old
            .allocations
            .get(id)
            .ok_or(PackageError::Integrity)?;
        check(
            actual.bytes == *bytes && actual.arena == old.arena && actual.witness == old.witness,
        )?;
    }
    let scope = serde_json::json!({"profile":registration.profile,"incarnation":registration.incarnation.to_string(),"directory":{"device":directory.device.to_string(),"inode":directory.inode.to_string()}});
    check(scope == plan.original()["scope"])?;
    let selected = plan.selection(stage)?;
    verify_original_inner(
        package,
        identity,
        inspection,
        registration,
        directory,
        frames,
        limits,
        Some(&selected),
    )
}
