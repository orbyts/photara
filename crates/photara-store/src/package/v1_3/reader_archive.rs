//! Scoped archive placement. Ownership remains fully checked; unrequested
//! historical locator/index descendants are not treated as current closure.
use super::{
    AccountingMetadata, AllocationInspection, BTreeMap, BTreeSet, FrameKind, Locator,
    LogicalRecords, LogicalTree, LogicalTreeKind, Membership, ObjectRef, OwnershipContext,
    PackageError, PackageUuid, PhysicalRecords, PhysicalRef, ReaderLimits, SelectedEnvelope,
    TreeLimits, Value, check, fields, json_ref, legacy_framing, nonnil,
};
use std::cell::RefCell;

struct Restricted<'a, P> {
    provider: &'a P,
    allowed: BTreeSet<PackageUuid>,
    visited: RefCell<BTreeSet<PackageUuid>>,
}
impl<P: PhysicalRecords> PhysicalRecords for Restricted<'_, P> {
    fn resolve(&self, reference: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError> {
        check(self.allowed.contains(&reference.allocation_id))?;
        let value = self.provider.resolve(reference, kind)?;
        self.visited.borrow_mut().insert(reference.allocation_id);
        Ok(value)
    }
    fn resolve_json(
        &self,
        reference: &PhysicalRef,
        kind: FrameKind,
        object: &ObjectRef,
    ) -> Result<Value, PackageError> {
        check(self.allowed.contains(&reference.allocation_id))?;
        let value = self.provider.resolve_json(reference, kind, object)?;
        self.visited.borrow_mut().insert(reference.allocation_id);
        Ok(value)
    }
}
pub(super) struct Archive<'a, P> {
    physical: Restricted<'a, P>,
    root: PhysicalRef,
    project: PackageUuid,
    limits: TreeLimits,
    pub objects: BTreeSet<ObjectRef>,
    pub allocations: BTreeSet<PackageUuid>,
}
impl<'a, P: PhysicalRecords> Archive<'a, P> {
    pub fn open(
        envelope: &SelectedEnvelope,
        provider: &'a P,
        inspection: &impl AllocationInspection,
        accounting: &AccountingMetadata,
        limits: &ReaderLimits,
    ) -> Result<Self, PackageError> {
        let placement = &envelope.root["archive_placement"];
        fields(placement, &["locator", "ownership"])?;
        let root: PhysicalRef = serde_json::from_value(placement["locator"].clone())
            .map_err(|_| PackageError::Record)?;
        let ownership = json_ref(&placement["ownership"])?;
        let registered = accounting
            .observations
            .keys()
            .map(|id| PackageUuid::parse(id))
            .collect::<Result<_, _>>()?;
        let mut physical = Restricted {
            provider,
            allowed: registered,
            visited: RefCell::new(BTreeSet::new()),
        };
        let header = physical.resolve(&root, FrameKind::Physical)?;
        if header["schema"]["id"] == "photara.storage.locator-overlay" {
            check(
                header["schema"]["version"] == 1
                    && header["role"] == "archive"
                    && header["allocation_id"] == root.allocation_id.to_string(),
            )?;
        }
        let locator = Locator::new(
            &physical,
            envelope.identity.project,
            root.clone(),
            limits.tree,
        );
        let tree = LogicalTree::new(
            &locator,
            envelope.identity.project,
            LogicalTreeKind::Ownership,
            limits.tree,
        )
        .audit(&ownership)?;
        let expected = tree
            .entries
            .iter()
            .map(|entry| {
                fields(entry, &["allocation_id", "claim"])?;
                nonnil(&entry["allocation_id"])
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        check(expected.len() == tree.entries.len())?;
        let proof = super::super::audit_snapshot_ownership(
            &locator,
            inspection,
            &ownership,
            &OwnershipContext {
                project: envelope.identity.project,
                accounting,
                expected_allocations: &expected,
                blobs: &BTreeMap::new(),
                tree_limits: limits.tree,
                max_prefix_bytes: limits.max_prefix_bytes,
            },
        )?;
        legacy_framing(provider, &locator, &proof)?;
        // Account for the locator pages and ownership records actually needed
        // to establish archive custody; later historical lookups keep this fence.
        check(physical.visited.borrow().is_subset(&proof.allocations))?;
        physical.allowed.clone_from(&proof.allocations);
        Ok(Self {
            physical,
            root,
            project: envelope.identity.project,
            limits: limits.tree,
            objects: proof.objects,
            allocations: proof.allocations,
        })
    }
}
impl<P: PhysicalRecords> LogicalRecords for Archive<'_, P> {
    fn resolve(&self, object: &ObjectRef, membership: Membership) -> Result<Value, PackageError> {
        Locator::new(&self.physical, self.project, self.root.clone(), self.limits)
            .json(object, membership)
    }
}
