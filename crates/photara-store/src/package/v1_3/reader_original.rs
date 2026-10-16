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
    let selected_frames = selected_frames(&bytes)?;
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
    let mut verified = repeatable::VerifiedOriginal::from_reader(
        package,
        original,
        scope,
        registration.charge_unit,
        empty_hold,
    );
    verified.selected_frames = selected_frames;
    Ok(verified)
}

fn selected_frames(
    bytes: &Bytes<'_, impl AllocationInspection>,
) -> Result<repeatable::SelectedFrames, PackageError> {
    if !bytes.snapshot {
        return Ok(None);
    }
    let mut selected = Vec::new();
    for allocation in bytes.checked.borrow().values() {
        if let Checked::Snapshot(allocation) = allocation {
            selected.extend(allocation.selected_references()?);
        }
    }
    Ok(Some(selected))
}
pub(crate) struct OperationInspection {
    pub proof: super::SettledPackage,
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub selected_frames: repeatable::SelectedFrames,
}
// V4 binds the prior selected HEAD and authenticates only required old frames.
// Exact new suffix bytes remain mandatory; legacy codecs keep whole-file checks.
fn operation_bytes(
    package: &repeatable::shared::OriginalPackage,
    plan: &repeatable::RepeatablePlan,
) -> Result<(), PackageError> {
    let snapshot = plan.original["original_codec"] == "photara.codec.ps2-repeatable-admission-v4";
    for (id, expected) in &plan.files {
        let actual = package.allocations.get(id).ok_or(PackageError::Integrity)?;
        let old = plan
            .old
            .allocations
            .get(id)
            .ok_or(PackageError::Integrity)?;
        check(actual.arena == old.arena && actual.witness == old.witness)?;
        let start = if snapshot { old.bytes.len() } else { 0 };
        check(
            actual.bytes.len() == expected.len()
                && actual.bytes.get(start..) == expected.get(start..),
        )?;
    }
    Ok(())
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
    Ok(inspect_operation_details(
        package,
        identity,
        inspection,
        registration,
        frames,
        limits,
        plan,
        stage,
    )?
    .proof)
}
#[expect(
    clippy::too_many_arguments,
    reason = "Exact operation inspection and captured selected-frame closure"
)]
pub(crate) fn inspect_operation_details(
    package: &repeatable::shared::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    frames: FrameLimits,
    limits: &ReaderLimits,
    plan: &repeatable::RepeatablePlan,
    stage: usize,
) -> Result<OperationInspection, PackageError> {
    check(package.manifest == plan.old.manifest && package.allocations.len() == plan.files.len())?;
    operation_bytes(package, plan)?;
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
    let proof = inspect_selected(
        &envelope,
        &bytes,
        &bytes,
        &bytes,
        registration,
        limits,
        Some(&selected),
    )?;
    // Match the planning reader's selected closure, including packed hold edges.
    controls(&envelope, &bytes, limits, Some(&selected))?;
    Ok(OperationInspection {
        proof,
        #[cfg(any(test, feature = "controlled-disposable"))]
        selected_frames: selected_frames(&bytes)?,
    })
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
    operation_bytes(&package, plan)?;
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
    verified.completed_original = Some(super::object_ref(plan.original())?);
    Ok(verified)
}

/// First adoption reuses the exact selected old active locator and performs the
/// full reciprocal operation audit; a claimed base is never enough by itself.
pub(crate) fn audit_checkpoint_adoption(
    original: &repeatable::VerifiedOriginal,
    clean: &repeatable::VerifiedOriginal,
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
    if active.state["schema"]["version"] == 3 {
        let value = base.value();
        check(
            value["source_commit"]
                == serde_json::json!({"commit_id":package.commit["commit_id"],"commit_sha256":package.head["commit_sha256"]})
                && value["source_state"]
                    == serde_json::to_value(super::super::snapshot::object_ref(&active.state)?)
                        .map_err(|_| PackageError::Record)?
                && value["authored_revision"] == active.state["authored_revision"]
                && value["authored_sha256"] == active.state["authored"]["sha256"]
                && value["accepted"] == active.state["accepted"]
                && value["journal_inclusion"] == active.state["journal_inclusion"],
        )?;
        snapshot_base_binding(
            clean,
            &super::super::snapshot::object_ref(value)?,
            frames,
            limits,
        )?;
        return check(
            snapshot_entries(original, frames, limits)? == snapshot_entries(clean, frames, limits)?,
        );
    }
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
    check(active.state["schema"]["version"] == 3)?;
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

/// The registered local selector/BaseReceipt has already authenticated this
/// exact clean maintenance HEAD and Original. Independently audit its complete
/// required closure; do not replay retired pre-base authored transitions.
#[cfg(any(test, feature = "controlled-disposable"))]
#[expect(
    clippy::too_many_arguments,
    reason = "Local anchor plus independent physical reader inputs"
)]
pub(crate) fn verify_checkpoint_anchor(
    package: &repeatable::shared::OriginalPackage,
    identity: SelectionIdentity,
    inspection: &impl AllocationInspection,
    registration: &SettledRegistration,
    directory: DirectoryObservation,
    frames: FrameLimits,
    limits: &ReaderLimits,
    base: &Value,
    original: &Value,
) -> Result<repeatable::VerifiedOriginal, PackageError> {
    use serde_json::json;
    let reference = |value: &Value| -> Result<Value, PackageError> {
        serde_json::to_value(super::super::snapshot::object_ref(value)?)
            .map_err(|_| PackageError::Record)
    };
    check(
        package.head == base["package_head"]
            && reference(&package.commit)? == base["package_commit"]
            && package.commit["root_set"]["schema"]["version"] == 3,
    )?;
    let read = |r: &Value| -> Result<Value, PackageError> {
        let r = super::super::selected::json_ref(r)?;
        let b = package
            .loose
            .get(r.sha256.as_str())
            .ok_or(PackageError::Integrity)?;
        check(b.len() as u64 == r.byte_length.get() && digest(b) == r.sha256)?;
        crate::package::parse_canonical_json(b, limits.json)
    };
    let envelope_ref = &package.commit["root_set"]["placement"]["accounting"];
    let envelope = read(envelope_ref)?;
    let holds = super::super::selected::json_ref(&envelope["holds"])?;
    let original_ref = reference(original)?;
    check(read(&original_ref)? == *original)?;
    let mut phase = None;
    for bytes in package.loose.values() {
        let value = crate::package::parse_canonical_json(bytes, limits.json)?;
        if value["schema"]["id"] == "photara.storage.checkpoint-turnover-phase" {
            check(phase.is_none())?;
            phase = Some(value);
        }
    }
    let phase = phase.ok_or(PackageError::Integrity)?;
    let consumed = super::super::tree::number(&phase["consumed"])?;
    check(
        phase
            == json!({"schema":{"id":"photara.storage.checkpoint-turnover-phase","version":1},"project_id":original["project_id"],"library_id":original["library_id"],"bootstrap_sha256":original["bootstrap_sha256"],"extensions":{},"maintenance_id":original["maintenance_id"],"original":original_ref,"stage":"clean","consumed":consumed.to_string(),"remaining":"0"}),
    )?;
    let expected = [
        envelope_ref.clone(),
        envelope["ledger"].clone(),
        original_ref,
        reference(&phase)?,
        package.commit["root_set"]["inventory"].clone(),
    ]
    .into_iter()
    .map(|r| super::super::selected::json_ref(&r))
    .collect::<Result<BTreeSet<_>, _>>()?;
    check(
        expected.len() == 5
            && expected
                .iter()
                .map(|r| r.sha256.as_str())
                .collect::<BTreeSet<_>>()
                == package.loose.keys().map(String::as_str).collect(),
    )?;
    let selected = OperationSelection {
        original,
        expected_commit: &package.commit,
        expected_loose: &package.loose,
        packed_hold: &holds,
        packed_selected_hold: true,
        extra_global: &BTreeMap::new(),
    };
    let verified = verify_original_inner(
        package.clone(),
        identity,
        inspection,
        registration,
        directory,
        frames,
        limits,
        Some(&selected),
    )?;
    snapshot_base_binding(
        &verified,
        &super::super::selected::json_ref(&base["checkpoint"])?,
        frames,
        limits,
    )?;
    Ok(verified)
}

struct Pinned<'a, P> {
    provider: &'a P,
    allowed: BTreeSet<PackageUuid>,
}
impl<P: PhysicalRecords> PhysicalRecords for Pinned<'_, P> {
    fn resolve(&self, r: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError> {
        check(self.allowed.contains(&r.allocation_id))?;
        self.provider.resolve(r, kind)
    }
    fn resolve_json(
        &self,
        r: &PhysicalRef,
        kind: FrameKind,
        object: &ObjectRef,
    ) -> Result<Value, PackageError> {
        check(self.allowed.contains(&r.allocation_id))?;
        self.provider.resolve_json(r, kind, object)
    }
}

/// Scoped historical lookup from the same owned, independently verified bytes.
/// Archive custody was audited when this unforgeable wrapper was constructed;
/// requested paths/frames are authenticated again without rescanning payloads.
pub(crate) fn snapshot_receipt(
    original: &repeatable::VerifiedOriginal,
    operation: PackageUuid,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<Option<Value>, PackageError> {
    snapshot_history(original, Some(operation), frames, limits).map(|value| value.0)
}
pub(crate) fn snapshot_entries(
    original: &repeatable::VerifiedOriginal,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<Vec<Value>, PackageError> {
    snapshot_history(original, None, frames, limits).map(|value| value.1)
}
#[expect(
    clippy::too_many_lines,
    reason = "Shared exact current and fenced archive lookup"
)]
fn snapshot_history(
    original: &repeatable::VerifiedOriginal,
    operation: Option<PackageUuid>,
    frames: FrameLimits,
    limits: &ReaderLimits,
) -> Result<(Option<Value>, Vec<Value>), PackageError> {
    use super::super::{
        Locator, LogicalTree, LogicalTreeKind, Membership, PhysicalTree, PhysicalTreeKind,
    };
    let package = original.shared_package();
    let project = PackageUuid::parse(
        package.manifest["project_id"]
            .as_str()
            .ok_or(PackageError::Record)?,
    )?;
    check(package.commit["root_set"]["schema"]["version"] == 3)?;
    let provider = Bytes {
        package,
        inspection: &(),
        frames,
        snapshot: true,
        checked: RefCell::new(BTreeMap::new()),
    };
    let active = &original.proof.roles["active"].state;
    let active_ref = super::super::selected::json_ref(&package.commit["root_set"]["active"])?;
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
        &active_ref,
    )?
    .ok_or(PackageError::Integrity)?;
    let locator = Locator::new(
        &provider,
        project,
        super::super::selected::physical(&placement["locator"])?,
        limits.tree,
    );
    let index = locator.json(
        &super::super::selected::json_ref(&active["operation_index"])?,
        Membership::Semantic,
    )?;
    let identity = SelectionIdentity {
        project,
        library: PackageUuid::parse(
            package.commit["root_set"]["library_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
        bootstrap_sha256: digest(
            &photara_core::canonical_json(&package.manifest).map_err(|_| PackageError::Record)?,
        ),
    };
    if let Some(receipt) = original.proof.roles["active"]
        .receipts
        .iter()
        .find(|r| operation.is_some_and(|id| r["operation_id"] == id.to_string()))
    {
        let operation = operation.ok_or(PackageError::Integrity)?;
        let entry = LogicalTree::new(&locator, project, LogicalTreeKind::OperationId, limits.tree)
            .lookup(
                &super::super::selected::json_ref(&index["by_id"])?,
                &super::super::LogicalKey::Id(operation),
            )?
            .ok_or(PackageError::Integrity)?;
        let reciprocal = LogicalTree::new(
            &locator,
            project,
            LogicalTreeKind::OperationOrdinal,
            limits.tree,
        )
        .lookup(
            &super::super::selected::json_ref(&index["by_ordinal"])?,
            &super::super::LogicalKey::Ordinal(number(&entry["acceptance_ordinal"])?),
        )?;
        check(reciprocal.as_ref() == Some(&entry))?;
        let value = locator.json(
            &super::super::selected::json_ref(&entry["receipt"])?,
            Membership::Semantic,
        )?;
        check(value == *receipt)?;
        return crate::package::parse_canonical_json(
            &photara_core::canonical_json(&value).map_err(|_| PackageError::Record)?,
            limits.json,
        )
        .map(|value| (Some(value), Vec::new()));
    }
    let base = super::super::snapshot::AcceptanceBase::parse(
        &photara_core::canonical_json(&locator.json(
            &super::super::selected::json_ref(&index["base"])?,
            Membership::Semantic,
        )?)
        .map_err(|_| PackageError::Record)?,
        &identity,
        limits.json,
    )?;
    let archive = &package.commit["root_set"]["archive_placement"];
    let root = super::super::selected::physical(&archive["locator"])?;
    let locator = Locator::new(&provider, project, root.clone(), limits.tree);
    let ownership = LogicalTree::new(&locator, project, LogicalTreeKind::Ownership, limits.tree)
        .audit(&super::super::selected::json_ref(&archive["ownership"])?)?;
    let allowed = ownership
        .entries
        .iter()
        .map(|row| super::super::selected::nonnil(&row["allocation_id"]))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let pinned = Pinned {
        provider: &provider,
        allowed,
    };
    let archive = Locator::new(&pinned, project, root, limits.tree);
    if let Some(operation) = operation {
        return base
            .lookup(&archive, &identity, operation, None, limits.tree)
            .map(|value| (value, Vec::new()));
    }
    let mut entries = base.index_entries(&archive, &identity, limits.tree)?;
    let mut operations = entries
        .iter()
        .map(|entry| super::super::selected::nonnil(&entry["operation_id"]))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for receipt in &original.proof.roles["active"].receipts {
        check(
            operations.insert(super::super::selected::nonnil(&receipt["operation_id"])?)
                && number(&receipt["acceptance_ordinal"])? == entries.len() as u64 + 1,
        )?;
        entries.push(serde_json::json!({"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"acceptance_ordinal":receipt["acceptance_ordinal"],"receipt":super::super::snapshot::object_ref(receipt)?}));
    }
    if entries.len() > limits.tree.max_entries {
        return Err(PackageError::Limit);
    }
    check(entries.len() as u64 == number(&active["accepted"]["through_ordinal"])?)?;
    Ok((None, entries))
}
