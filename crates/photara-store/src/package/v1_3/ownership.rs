//! Exact per-root ownership checks against selected accounting and independently
//! supplied allocation observations. Success is not writable admission.
use super::super::{ObjectKind, ObjectRef, PackageError, PackageUuid, Sha256Hex};
use super::tree::{fields, number};
use super::{
    AccountingMetadata, Arena, LogicalRecords, LogicalTree, LogicalTreeKind, Membership, TreeLimits,
};
use photara_core::contracts::schema::QualifiedName;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationLayout {
    FramedJson,
    WholeBlob,
}
/// Comparison inputs supplied by an independently registered/pinned provider.
/// Copying selected ledger values into these fields does not establish trust.
pub struct AllocationObservation {
    pub arena: Arena,
    pub layout: AllocationLayout,
    pub extent: u64,
    pub registered_charge: u64,
    pub device: u64,
    pub inode: u64,
}
pub trait AllocationInspection {
    /// # Errors
    /// Refuses absent, unsafe or changed allocation identity. The registered
    /// charge comes from original admission, never from current selected bytes.
    fn observe(&self, allocation: PackageUuid) -> Result<AllocationObservation, PackageError>;
    /// Explicit framed-byte audit, bounded by `max_bytes`. Must use the same pinned
    /// allocation identity as `observe`. Never invoke this for whole-Blob media.
    /// # Errors
    /// Refuses length/budget, identity changes or IO failure.
    fn framed_prefix_digest(
        &self,
        allocation: PackageUuid,
        length: u64,
        max_bytes: u64,
    ) -> Result<Sha256Hex, PackageError>;
}
/// Inputs derived by the caller from authenticated selected locator/accounting
/// closure. They are not authority or a substitute for complete closure audit.
pub struct OwnershipContext<'a> {
    pub project: PackageUuid,
    pub accounting: &'a AccountingMetadata,
    pub expected_allocations: &'a BTreeSet<PackageUuid>,
    /// Exactly one whole-Blob identity for each raw allocation used by this root.
    pub blobs: &'a BTreeMap<PackageUuid, ObjectRef>,
    pub tree_limits: TreeLimits,
    pub max_prefix_bytes: u64,
}
pub struct OwnershipAudit {
    pub allocations: BTreeSet<PackageUuid>,
    pub objects: BTreeSet<ObjectRef>,
}
/// Checks the frozen claim shape, complete extents, immutable sealed-charge
/// references and exact once-only per-root ownership. Blob checks use metadata
/// and selected digest commitments; content hashing remains explicit Blob audit.
/// # Errors
/// Refuses any mismatch between selected records and independent observations,
/// malformed claims, duplicate/missing allocations, prefix corruption or budgets.
pub fn audit_ownership<P: LogicalRecords, A: AllocationInspection>(
    provider: &P,
    inspection: &A,
    root: &ObjectRef,
    context: &OwnershipContext<'_>,
) -> Result<OwnershipAudit, PackageError> {
    audit_ownership_profile(provider, inspection, root, context, false)
}
/// Snapshot-profile ownership metadata. Selected frame integrity/overlap must be
/// verified separately through its physical resolver; this is not a closure proof.
/// Existing claim-v1 records always retain their full-prefix obligations.
/// # Errors
/// Refuses malformed claim dispatch, changed observations/charges, unsupported
/// whole-Blob v2, missing allocations and every existing ownership refusal.
pub fn audit_snapshot_ownership<P: LogicalRecords, A: AllocationInspection>(
    provider: &P,
    inspection: &A,
    root: &ObjectRef,
    context: &OwnershipContext<'_>,
) -> Result<OwnershipAudit, PackageError> {
    audit_ownership_profile(provider, inspection, root, context, true)
}
#[expect(
    clippy::too_many_lines,
    reason = "Exact claim versions share all independent observation and charge checks"
)]
fn audit_ownership_profile<P: LogicalRecords, A: AllocationInspection>(
    provider: &P,
    inspection: &A,
    root: &ObjectRef,
    context: &OwnershipContext<'_>,
    snapshot: bool,
) -> Result<OwnershipAudit, PackageError> {
    if context.accounting.project_id != context.project.to_string() {
        return Err(PackageError::Integrity);
    }
    let tree = LogicalTree::new(
        provider,
        context.project,
        LogicalTreeKind::Ownership,
        context.tree_limits,
    )
    .audit(root)?;
    let mut objects = tree.nodes;
    let mut claimed = BTreeSet::new();
    let mut prefix_bytes = 0u64;
    for entry in tree.entries {
        fields(&entry, &["allocation_id", "claim"])?;
        let claim: ObjectRef =
            serde_json::from_value(entry["claim"].clone()).map_err(|_| PackageError::Record)?;
        if claim.kind != ObjectKind::Json || claim.byte_length.get() == 0 {
            return Err(PackageError::Record);
        }
        let value = provider.resolve(&claim, Membership::Ownership)?;
        objects.insert(claim);
        let selected_records =
            value["schema"] == json!({"id":"photara.storage.allocation-claim","version":2});
        if selected_records && !snapshot {
            return Err(PackageError::UnsupportedVersion);
        }
        let mut names = vec![
            "schema",
            "project_id",
            "extensions",
            "allocation_id",
            "arena",
            "layout",
            "owned_extent",
            "sealed",
            "sealed_charge",
        ];
        names.push(if selected_records {
            "verification"
        } else {
            "authenticated_prefix"
        });
        fields(&value, &names)?;
        if value["schema"]
            != json!({"id":"photara.storage.allocation-claim","version":if selected_records { 2 } else { 1 }})
            || value["project_id"] != context.project.to_string()
        {
            return Err(PackageError::UnsupportedVersion);
        }
        if selected_records
            && (value["extensions"] != json!({})
                || value["verification"] != "selected-records"
                || value["layout"] != "framed-json")
        {
            return Err(PackageError::Record);
        }
        if value["extensions"]
            .as_object()
            .ok_or(PackageError::Record)?
            .keys()
            .any(|key| QualifiedName::parse(key.clone()).is_err())
        {
            return Err(PackageError::Record);
        }
        let id = PackageUuid::parse(
            value["allocation_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?;
        if id.as_uuid().is_nil()
            || value["allocation_id"] != entry["allocation_id"]
            || !claimed.insert(id)
        {
            return Err(PackageError::Integrity);
        }
        let observed = inspection.observe(id)?;
        let selected = context
            .accounting
            .observations
            .get(&id.to_string())
            .ok_or(PackageError::Integrity)?;
        let arena = match observed.arena {
            Arena::Data => "data",
            Arena::Metadata => "metadata",
        };
        let layout = match observed.layout {
            AllocationLayout::FramedJson => "framed-json",
            AllocationLayout::WholeBlob => "whole-blob",
        };
        if observed.inode == 0
            || observed.registered_charge < observed.extent
            || value["arena"] != arena
            || value["layout"] != layout
            || number(&value["owned_extent"])? != observed.extent
            || number(&selected["measured_extent"])? != observed.extent
            || number(&selected["charged_high_water"])? != observed.registered_charge
            || selected["physical"]
                != json!({"device":observed.device.to_string(),"inode":observed.inode.to_string()})
        {
            return Err(PackageError::Integrity);
        }
        let length = if selected_records {
            if observed.layout != AllocationLayout::FramedJson || context.blobs.contains_key(&id) {
                return Err(PackageError::Integrity);
            }
            if value["sealed"] == true {
                observed.extent
            } else {
                0
            }
        } else {
            fields(&value["authenticated_prefix"], &["byte_length", "sha256"])?;
            let length = number(&value["authenticated_prefix"]["byte_length"])?;
            let expected = Sha256Hex::parse(
                value["authenticated_prefix"]["sha256"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            )?;
            if length > observed.extent {
                return Err(PackageError::Integrity);
            }
            match observed.layout {
                AllocationLayout::FramedJson => {
                    prefix_bytes = prefix_bytes
                        .checked_add(length)
                        .ok_or(PackageError::Limit)?;
                    if prefix_bytes > context.max_prefix_bytes {
                        return Err(PackageError::Limit);
                    }
                    if context.blobs.contains_key(&id)
                        || inspection.framed_prefix_digest(id, length, length)? != expected
                    {
                        return Err(PackageError::Integrity);
                    }
                }
                AllocationLayout::WholeBlob => {
                    let blob = context.blobs.get(&id).ok_or(PackageError::Integrity)?;
                    if observed.arena != Arena::Data
                        || blob.kind != ObjectKind::Blob
                        || value["sealed"] != true
                        || length != observed.extent
                        || blob.byte_length.get() != length
                        || blob.sha256 != expected
                    {
                        return Err(PackageError::Integrity);
                    }
                }
            }
            length
        };
        if value["sealed"] == true {
            let reference: ObjectRef = serde_json::from_value(value["sealed_charge"].clone())
                .map_err(|_| PackageError::Record)?;
            if reference.kind != ObjectKind::Json
                || reference.byte_length.get() == 0
                || length != observed.extent
                || context.accounting.sealed_refs.get(&id.to_string())
                    != Some(&(
                        reference.sha256.as_str().to_owned(),
                        reference.byte_length.get(),
                    ))
            {
                return Err(PackageError::Integrity);
            }
            let charge = provider.resolve(&reference, Membership::Ownership)?;
            if charge["allocation_id"] != id.to_string()
                || number(&charge["measured_extent"])? != observed.extent
                || number(&charge["charged_high_water"])? != observed.registered_charge
            {
                return Err(PackageError::Integrity);
            }
        } else if value["sealed"] != false
            || !value["sealed_charge"].is_null()
            || length != 0
            || !context.accounting.tip_ids.contains(&id.to_string())
        {
            return Err(PackageError::Integrity);
        }
    }
    if claimed != *context.expected_allocations
        || context.blobs.keys().any(|id| !claimed.contains(id))
    {
        return Err(PackageError::Integrity);
    }
    Ok(OwnershipAudit {
        allocations: claimed,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::super::accounting::{audit_settled_accounting, tests::setup};
    use super::*;
    use crate::package::{DecimalU64, digest};
    use serde_json::Value;
    struct Inspection {
        allocations: BTreeMap<PackageUuid, (Arena, Vec<u8>, u64, u64)>,
        replace_inode: bool,
    }
    impl AllocationInspection for Inspection {
        fn observe(&self, allocation: PackageUuid) -> Result<AllocationObservation, PackageError> {
            let (arena, bytes, device, inode) = self
                .allocations
                .get(&allocation)
                .ok_or(PackageError::Integrity)?;
            Ok(AllocationObservation {
                arena: *arena,
                layout: AllocationLayout::FramedJson,
                extent: bytes.len() as u64,
                registered_charge: (bytes.len() as u64).div_ceil(4096) * 4096,
                device: *device,
                inode: if self.replace_inode {
                    inode + 1
                } else {
                    *inode
                },
            })
        }
        fn framed_prefix_digest(
            &self,
            allocation: PackageUuid,
            length: u64,
            max_bytes: u64,
        ) -> Result<Sha256Hex, PackageError> {
            if length > max_bytes {
                return Err(PackageError::Limit);
            }
            let bytes = &self
                .allocations
                .get(&allocation)
                .ok_or(PackageError::Integrity)?
                .1;
            Ok(digest(
                bytes
                    .get(..usize::try_from(length).map_err(|_| PackageError::Limit)?)
                    .ok_or(PackageError::Integrity)?,
            ))
        }
    }
    struct NoPrefix<'a>(&'a Inspection);
    impl AllocationInspection for NoPrefix<'_> {
        fn observe(&self, allocation: PackageUuid) -> Result<AllocationObservation, PackageError> {
            self.0.observe(allocation)
        }
        fn framed_prefix_digest(
            &self,
            _: PackageUuid,
            _: u64,
            _: u64,
        ) -> Result<Sha256Hex, PackageError> {
            Err(PackageError::Integrity)
        }
    }
    fn put(records: &mut super::super::accounting::tests::Records, value: Value) -> Value {
        let bytes = photara_core::canonical_json(&value).unwrap();
        let reference =
            json!({"kind":"json","sha256":digest(&bytes),"byte_length":bytes.len().to_string()});
        records.0.insert(
            (digest(&bytes).as_str().to_owned(), bytes.len() as u64),
            value,
        );
        reference
    }
    fn selected_tree(
        source: &super::super::accounting::tests::Records,
        target: &mut super::super::accounting::tests::Records,
        reference: &ObjectRef,
    ) -> Value {
        let mut page = source.resolve(reference, Membership::Ownership).unwrap();
        if let Some(entries) = page.get_mut("entries").and_then(Value::as_array_mut) {
            for entry in entries {
                let claim_ref = serde_json::from_value(entry["claim"].clone()).unwrap();
                let mut claim = source.resolve(&claim_ref, Membership::Ownership).unwrap();
                claim["schema"]["version"] = json!(2);
                claim
                    .as_object_mut()
                    .unwrap()
                    .remove("authenticated_prefix");
                claim["verification"] = json!("selected-records");
                entry["claim"] = put(target, claim);
            }
        } else {
            for child in page["children"].as_array_mut().unwrap() {
                child["child"] = selected_tree(
                    source,
                    target,
                    &serde_json::from_value(child["child"].clone()).unwrap(),
                );
            }
        }
        put(target, page)
    }
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Existing corpus verifies both ownership dispatches against identical live observations"
    )]
    fn frozen_owned_tips_match_exact_witnesses_and_allocation_set() {
        let (provider, ledger, holds, limits) = setup();
        let accounting = audit_settled_accounting(&ledger, &holds, &provider, limits).unwrap();
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        let mut inspection = Inspection {
            allocations: BTreeMap::new(),
            replace_inode: false,
        };
        for (id, row) in corpus["allocations"].as_object().unwrap() {
            let hex = row["hex"].as_str().unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            inspection.allocations.insert(
                PackageUuid::parse(id).unwrap(),
                (
                    serde_json::from_value(row["arena"].clone()).unwrap(),
                    bytes,
                    number(&row["witness"]["device"]).unwrap(),
                    number(&row["witness"]["inode"]).unwrap(),
                ),
            );
        }
        let project = PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap();
        let mut checked = 0;
        for value in provider
            .0
            .values()
            .filter(|v| v["schema"]["id"] == "photara.storage.ownership-branch")
        {
            let bytes = photara_core::canonical_json(value).unwrap();
            let root = ObjectRef {
                kind: ObjectKind::Json,
                sha256: digest(&bytes),
                byte_length: DecimalU64::parse(&bytes.len().to_string()).unwrap(),
            };
            let entries =
                LogicalTree::new(&provider, project, LogicalTreeKind::Ownership, limits.tree)
                    .audit(&root)
                    .unwrap()
                    .entries;
            let expected = entries
                .iter()
                .map(|e| PackageUuid::parse(e["allocation_id"].as_str().unwrap()).unwrap())
                .collect();
            let blobs = BTreeMap::new();
            let context = OwnershipContext {
                project,
                accounting: &accounting,
                expected_allocations: &expected,
                blobs: &blobs,
                tree_limits: limits.tree,
                max_prefix_bytes: 1306,
            };
            let proof = audit_ownership(&provider, &inspection, &root, &context).unwrap();
            assert_eq!(proof.allocations, expected);
            // Reuse these same claims/observations for the additive snapshot
            // dispatch; no prefix hashing is allowed for a selected-record claim.
            let mut selected_provider =
                super::super::accounting::tests::Records(provider.0.clone());
            let selected_root: ObjectRef =
                serde_json::from_value(selected_tree(&provider, &mut selected_provider, &root))
                    .unwrap();
            let no_prefix = NoPrefix(&inspection);
            let selected_proof =
                audit_snapshot_ownership(&selected_provider, &no_prefix, &selected_root, &context)
                    .unwrap();
            assert_eq!(selected_proof.allocations, expected);
            assert!(
                audit_ownership(&selected_provider, &no_prefix, &selected_root, &context).is_err()
            );
            // Old v1 claim obligations are never waived even in a snapshot root.
            assert!(audit_snapshot_ownership(&provider, &no_prefix, &root, &context).is_err());
            let bounded = OwnershipContext {
                max_prefix_bytes: 0,
                ..context
            };
            if proof
                .allocations
                .contains(&PackageUuid::parse("96000000-0000-4000-8000-000000001001").unwrap())
            {
                assert!(matches!(
                    audit_ownership(&provider, &inspection, &root, &bounded),
                    Err(PackageError::Limit)
                ));
            }
            inspection.replace_inode = true;
            assert!(
                audit_snapshot_ownership(&selected_provider, &inspection, &selected_root, &context)
                    .is_err()
            );
            assert!(audit_ownership(&provider, &inspection, &root, &context).is_err());
            inspection.replace_inode = false;
            let empty = BTreeSet::new();
            let wrong = OwnershipContext {
                expected_allocations: &empty,
                ..context
            };
            assert!(audit_ownership(&provider, &inspection, &root, &wrong).is_err());
            checked += 1;
        }
        assert_eq!(checked, 3);
    }
}
