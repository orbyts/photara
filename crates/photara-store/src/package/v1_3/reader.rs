//! Complete settled selection inspection over caller-supplied immutable bytes.
//! This proof is neither writable admission nor external-media verification.
use super::super::{
    Commit, DecimalU64, JsonLimits, ObjectKind, ObjectRef, PackageError, PackageLimits,
    PackageUuid, Sha256Hex, digest, parse_canonical_json,
};
use super::selected::{json_ref, nonnil, schema};
use super::tree::{fields, number};
use super::{
    AccountingLimits, AccountingMetadata, AllocationInspection, BlobMetadata, Locator,
    LogicalRecords, LogicalTree, LogicalTreeKind, Membership, OwnershipContext, PhysicalRecords,
    PhysicalRef, PhysicalTree, PhysicalTreeKind, ResourceLimits, ResourceMetadata,
    SelectedEnvelope, SelectedRole, StructuralState, TreeLimits, audit_operations, audit_ownership,
    audit_settled_accounting, inspect_legacy_semantics, inspect_resources,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Exact loose control bytes. Implementations must reject unsafe identities and
/// enumerate their whole selected control namespace, including unknown objects.
pub trait ControlRecords {
    /// # Errors
    /// Refuses unsafe or unenumerable selected control namespaces.
    fn controls(&self) -> Result<BTreeSet<ObjectRef>, PackageError>;
    /// # Errors
    /// Refuses missing or changed controls; caller rechecks bounded canonical bytes.
    fn read(&self, reference: &ObjectRef) -> Result<Vec<u8>, PackageError>;
}
/// Original registered retained-file metadata; accounting digest is an original capture
/// commitment, not accounting claim that opening accounting package rereads retained media.
pub struct RetainedObservation {
    pub extent: u64,
    pub sha256: Sha256Hex,
    pub registered_charge: u64,
    pub device: u64,
    pub inode: u64,
}
/// Independent comparison inputs, never an authority minted from selected JSON.
/// Callers must retain these from their original registration/capture boundary.
pub struct SettledRegistration {
    pub profile: String,
    pub incarnation: PackageUuid,
    pub allocations: BTreeSet<PackageUuid>,
    pub conversion: Option<ObjectRef>,
    pub retained_files: BTreeMap<Vec<String>, RetainedObservation>,
    pub standing_control: u64,
    pub charge_unit: u64,
    pub directory_allowance: u64,
    pub retained_directory_allowance: u64,
}
#[derive(Clone, Copy)]
pub struct ReaderLimits {
    pub json: JsonLimits,
    pub tree: TreeLimits,
    pub semantic: PackageLimits,
    pub resources: ResourceLimits,
    pub accounting: AccountingLimits,
    pub max_roles: usize,
    pub max_prefix_bytes: u64,
}
/// One independently authenticated role. Readability does not certify absent
/// roles, aggregate physical accounting, mutation, or release eligibility.
pub struct RoleClosure {
    pub(crate) reference: ObjectRef,
    pub(crate) state: Value,
    pub(crate) semantic: BTreeSet<ObjectRef>,
    pub(crate) receipts: Vec<Value>,
    pub(crate) resources: Option<ResourceMetadata>,
    inventory_nodes: BTreeSet<ObjectRef>,
}
impl RoleClosure {
    #[must_use]
    pub fn reference(&self) -> &ObjectRef {
        &self.reference
    }
    #[must_use]
    pub fn state(&self) -> &Value {
        &self.state
    }
    #[must_use]
    pub fn semantic(&self) -> &BTreeSet<ObjectRef> {
        &self.semantic
    }
    #[must_use]
    pub fn receipts(&self) -> &[Value] {
        &self.receipts
    }
}
/// Privately constructed complete selected closure, still not writable authority.
pub struct SettledPackage {
    pub(crate) roles: BTreeMap<String, RoleClosure>,
    pub(crate) global: BTreeSet<ObjectRef>,
    pub(crate) root: Value,
    pub(crate) commit: Value,
    accounting: AccountingMetadata,
}
impl SettledPackage {
    #[must_use]
    pub fn roles(&self) -> &BTreeMap<String, RoleClosure> {
        &self.roles
    }
    #[must_use]
    pub fn global(&self) -> &BTreeSet<ObjectRef> {
        &self.global
    }
    #[must_use]
    pub fn root(&self) -> &Value {
        &self.root
    }
    #[must_use]
    pub fn commit(&self) -> &Value {
        &self.commit
    }
    #[must_use]
    pub fn accounting(&self) -> &AccountingMetadata {
        &self.accounting
    }
}
fn check(ok: bool) -> Result<(), PackageError> {
    if ok {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn object(value: &Value) -> Result<ObjectRef, PackageError> {
    serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)
}
fn from_key(key: &(String, u64)) -> Result<ObjectRef, PackageError> {
    Ok(ObjectRef {
        kind: ObjectKind::Json,
        sha256: Sha256Hex::parse(&key.0)?,
        byte_length: DecimalU64::parse(&key.1.to_string())?,
    })
}
fn refs(keys: &BTreeSet<(String, u64)>) -> Result<BTreeSet<ObjectRef>, PackageError> {
    keys.iter().map(from_key).collect()
}
fn control(
    control_records: &impl ControlRecords,
    reference: &ObjectRef,
    limits: JsonLimits,
) -> Result<Value, PackageError> {
    check(
        reference.kind == ObjectKind::Json
            && reference.byte_length.get() <= limits.max_bytes as u64,
    )?;
    let bytes = control_records.read(reference)?;
    check(bytes.len() as u64 == reference.byte_length.get() && digest(&bytes) == reference.sha256)?;
    parse_canonical_json(&bytes, limits)
}
/// Only regenerated original-operation codecs construct this input. It proves
/// the exact selected O/P chain before the common packed closure is inspected.
/// Public caller metadata cannot select an alternate packed hold.
pub(crate) struct OperationSelection<'a> {
    pub expected_commit: &'a Value,
    pub expected_loose: &'a BTreeMap<String, Vec<u8>>,
    pub packed_hold: &'a ObjectRef,
    pub extra_global: &'a BTreeMap<ObjectRef, Membership>,
}
struct Controls {
    ledger: Value,
    holds: ObjectRef,
    base: ObjectRef,
    members: BTreeSet<ObjectRef>,
    count: u64,
}
fn controls(
    envelope: &SelectedEnvelope,
    control_records: &impl ControlRecords,
    limits: &ReaderLimits,
    operation: Option<&OperationSelection<'_>>,
) -> Result<Controls, PackageError> {
    if let Some(selected) = operation {
        check(envelope.commit == *selected.expected_commit)?;
    }
    let envelope_ref = json_ref(&envelope.root["placement"]["accounting"])?;
    let envelope_body = control(control_records, &envelope_ref, limits.json)?;
    schema(
        &envelope_body,
        "photara.storage.accounting-envelope",
        1,
        &["ledger", "holds"],
        envelope.identity.project,
    )?;
    let ledger_ref = json_ref(&envelope_body["ledger"])?;
    let ledger = control(control_records, &ledger_ref, limits.json)?;
    check(
        ledger["project_id"] == envelope.root["project_id"]
            && ledger["conversion_source"] == envelope.root["conversion_source"],
    )?;
    let overlay_ref = json_ref(&envelope.root["inventory"])?;
    let overlay = control(control_records, &overlay_ref, limits.json)?;
    schema(
        &overlay,
        "photara.package.inventory-overlay",
        1,
        &["base", "controls", "count"],
        envelope.identity.project,
    )?;
    let members = if let Some(selected) = operation {
        let mut members = BTreeSet::new();
        for (sha, bytes) in selected.expected_loose {
            check(digest(bytes).as_str() == sha)?;
            let reference = ObjectRef {
                kind: ObjectKind::Json,
                sha256: digest(bytes),
                byte_length: DecimalU64::parse(&bytes.len().to_string())?,
            };
            check(control_records.read(&reference)? == *bytes)?;
            parse_canonical_json(bytes, limits.json)?;
            if reference != overlay_ref {
                members.insert(reference);
            }
        }
        check(
            members.contains(&envelope_ref)
                && members.contains(&ledger_ref)
                && members.contains(&json_ref(&envelope_body["holds"])?),
        )?;
        members
    } else {
        BTreeSet::from([envelope_ref, ledger_ref])
    };
    let list = overlay["controls"]
        .as_array()
        .ok_or(PackageError::Record)?
        .iter()
        .map(json_ref)
        .collect::<Result<Vec<_>, _>>()?;
    check(list == members.iter().cloned().collect::<Vec<_>>())?;
    let mut expected = members.clone();
    expected.insert(overlay_ref);
    check(control_records.controls()? == expected)?;
    // The standing pool is independently registered; prove actual selected
    // control bytes fit it without inventing allocation-rounding credit.
    let bytes = expected
        .iter()
        .try_fold(envelope.outer_bytes, |sum, reference| {
            sum.checked_add(reference.byte_length.get())
                .ok_or(PackageError::Limit)
        })?;
    check(bytes <= number(&ledger["standing_control"])?)?;
    Ok(Controls {
        ledger,
        holds: operation.map_or_else(
            || json_ref(&envelope_body["holds"]),
            |s| Ok(s.packed_hold.clone()),
        )?,
        base: json_ref(&overlay["base"])?,
        members,
        count: number(&overlay["count"])?,
    })
}
fn inventory(
    provider: &impl LogicalRecords,
    root: &ObjectRef,
    project: PackageUuid,
    limits: TreeLimits,
) -> Result<(BTreeSet<ObjectRef>, BTreeSet<ObjectRef>), PackageError> {
    let t = LogicalTree::new(provider, project, LogicalTreeKind::Inventory, limits).audit(root)?;
    Ok((
        t.entries.iter().map(object).collect::<Result<_, _>>()?,
        t.nodes,
    ))
}
fn pins(
    provider: &impl LogicalRecords,
    envelope: &SelectedEnvelope,
    limits: &ReaderLimits,
) -> Result<(Vec<Value>, BTreeSet<ObjectRef>), PackageError> {
    let t = LogicalTree::new(
        provider,
        envelope.identity.project,
        LogicalTreeKind::RetainedRoot,
        limits.tree,
    )
    .audit(&json_ref(&envelope.root["pinned_roots"])?)?;
    if t.entries.len().checked_add(2).ok_or(PackageError::Limit)? > limits.max_roles {
        return Err(PackageError::Limit);
    }
    for pin in &t.entries {
        fields(pin, &["pin_id", "reason", "root"])?;
        nonnil(&pin["pin_id"])?;
        json_ref(&pin["root"])?;
        check(matches!(
            pin["reason"].as_str(),
            Some("explicit-history" | "unresolved-recovery" | "undo")
        ))?;
    }
    Ok((t.entries, t.nodes))
}
fn role<P: PhysicalRecords, M: BlobMetadata>(
    envelope: &SelectedEnvelope,
    provider: &P,
    inspection: &M,
    state: &StructuralState,
    limits: &ReaderLimits,
) -> Result<RoleClosure, PackageError> {
    let loc = Locator::new(
        provider,
        envelope.identity.project,
        state.locator.clone(),
        limits.tree,
    );
    // The frozen selected codec spells the authenticated parent digest
    // `commit_sha256`; the legacy semantic DTO calls the same field `sha256`.
    // This is only the semantic validator's typed input, never a new selector.
    let mut semantic_commit = envelope.commit.clone();
    if let Some(parent) = semantic_commit["parent"].as_object_mut() {
        let digest = parent.remove("commit_sha256").ok_or(PackageError::Record)?;
        parent.insert("sha256".to_owned(), digest);
    }
    let mut commit: Commit =
        serde_json::from_value(semantic_commit).map_err(|_| PackageError::Record)?;
    commit.authored = json_ref(&state.state["authored"])?;
    commit.history = json_ref(&state.state["history"])?;
    commit.inventory = json_ref(&state.state["inventory"])?;
    let legacy = inspect_legacy_semantics(&loc, inspection, &commit, limits.semantic)?;
    check(
        loc.json(&commit.authored, Membership::Semantic)?["authored_revision"]
            == state.state["authored_revision"],
    )?;
    let operations = audit_operations(&loc, &state.state, &envelope.identity, limits.tree)?;
    let mut semantic = legacy.members().clone();
    semantic.extend(operations.objects().iter().cloned());
    let resources = if state.state["resource_state"].is_null() {
        None
    } else {
        let reference = inspect_resources(
            &loc,
            &json_ref(&state.state["resource_state"])?,
            nonnil(&state.state["root_id"])?,
            &envelope.identity,
            limits.resources,
        )?;
        semantic.extend(refs(&reference.closure)?);
        Some(reference)
    };
    let (expected, inventory_nodes) = inventory(
        &loc,
        &commit.inventory,
        envelope.identity.project,
        limits.tree,
    )?;
    check(expected == semantic)?;
    Ok(RoleClosure {
        reference: state.reference.clone(),
        state: state.state.clone(),
        semantic,
        receipts: operations.receipts().to_vec(),
        resources,
        inventory_nodes,
    })
}
struct Located {
    objects: BTreeMap<ObjectRef, Membership>,
    allocations: BTreeSet<PackageUuid>,
    blobs: BTreeMap<PackageUuid, ObjectRef>,
}
fn located<P: PhysicalRecords, M: BlobMetadata>(
    provider: &P,
    inspection: &M,
    envelope: &SelectedEnvelope,
    state: &StructuralState,
    limits: &ReaderLimits,
) -> Result<Located, PackageError> {
    let tree = PhysicalTree::new(
        provider,
        envelope.identity.project,
        PhysicalTreeKind::Locator,
        limits.tree,
    )
    .audit_with_pages(&state.locator)?;
    let loc = Locator::new(
        provider,
        envelope.identity.project,
        state.locator.clone(),
        limits.tree,
    );
    let mut allocations = tree
        .pages
        .iter()
        .map(|reference| reference.allocation_id)
        .collect::<BTreeSet<_>>();
    let mut objects = BTreeMap::new();
    let mut blobs = BTreeMap::new();
    for entry in tree.entries {
        fields(&entry, &["object", "membership", "physical"])?;
        let reference = object(&entry["object"])?;
        let membership: Membership = serde_json::from_value(entry["membership"].clone())
            .map_err(|_| PackageError::Record)?;
        check(objects.insert(reference.clone(), membership).is_none())?;
        if membership == Membership::Blob {
            let blob = loc.blob(&reference, inspection)?;
            let id = blob.allocation_id();
            if let Some(prior) = blobs.insert(id, reference) {
                check(prior == *blob.object())?;
            }
            allocations.insert(id);
        } else {
            loc.json(&reference, membership)?;
            let physical: PhysicalRef = serde_json::from_value(entry["physical"].clone())
                .map_err(|_| PackageError::Record)?;
            allocations.insert(physical.allocation_id);
        }
    }
    Ok(Located {
        objects,
        allocations,
        blobs,
    })
}
fn registered(
    provider: &impl LogicalRecords,
    envelope: &SelectedEnvelope,
    control_records: &Controls,
    accounting: &AccountingMetadata,
    registration: &SettledRegistration,
) -> Result<(), PackageError> {
    check(registration.charge_unit > 0)?;
    let round = |n: u64| {
        n.checked_add(registration.charge_unit - 1)
            .and_then(|n| n.checked_div(registration.charge_unit))
            .and_then(|n| n.checked_mul(registration.charge_unit))
            .ok_or(PackageError::Limit)
    };
    let mut charge = 0u64;
    for len in envelope
        .outer_lengths
        .iter()
        .copied()
        .chain(control_records.members.iter().map(|r| r.byte_length.get()))
        .chain([json_ref(&envelope.root["inventory"])?.byte_length.get()])
    {
        charge = charge.checked_add(round(len)?).ok_or(PackageError::Limit)?;
    }
    check(charge <= registration.standing_control)?;
    check(
        accounting.profile == registration.profile
            && accounting.incarnation == registration.incarnation.to_string()
            && accounting.standing == registration.standing_control
            && accounting.directory == registration.directory_allowance
            && accounting.retained_directory == registration.retained_directory_allowance,
    )?;
    check(
        accounting
            .allocations
            .iter()
            .map(|state| PackageUuid::parse(state))
            .collect::<Result<BTreeSet<_>, _>>()?
            == registration.allocations,
    )?;
    let conversion = if envelope.root["conversion_source"].is_null() {
        None
    } else {
        Some(json_ref(&envelope.root["conversion_source"])?)
    };
    check(conversion == registration.conversion)?;
    if let Some(reference) = conversion {
        let source = provider.resolve(&reference, Membership::Semantic)?;
        check(source["source_bootstrap_sha256"] == envelope.identity.bootstrap_sha256.as_str())?;
        let files = source["files"].as_array().ok_or(PackageError::Record)?;
        check(files.len() == registration.retained_files.len())?;
        for file in files {
            let path: Vec<String> = serde_json::from_value(file["components"].clone())
                .map_err(|_| PackageError::Record)?;
            let original = registration
                .retained_files
                .get(&path)
                .ok_or(PackageError::Integrity)?;
            check(
                number(&file["byte_length"])? == original.extent
                    && file["sha256"] == original.sha256.as_str(),
            )?;
        }
        for key in &accounting.logical {
            let reference = from_key(key)?;
            if reference == json_ref(&control_records.ledger["conversion_source"])?
                || reference == object_ref(&control_records.ledger)?
            {
                continue;
            }
            let value = provider.resolve(&reference, Membership::Ownership)?;
            if value["schema"]["id"] == "photara.storage.local-observation"
                && value["subject"]["kind"] == "retained-file"
            {
                let path: Vec<String> = serde_json::from_value(value["subject"]["path"].clone())
                    .map_err(|_| PackageError::Record)?;
                let original = registration
                    .retained_files
                    .get(&path)
                    .ok_or(PackageError::Integrity)?;
                check(
                    number(&value["measured_extent"])? == original.extent
                        && number(&value["charged_high_water"])? == original.registered_charge
                        && number(&value["physical"]["device"])? == original.device
                        && number(&value["physical"]["inode"])? == original.inode,
                )?;
            }
        }
    } else {
        check(registration.retained_files.is_empty())?;
    }
    Ok(())
}
fn object_ref(v: &Value) -> Result<ObjectRef, PackageError> {
    let bytes = photara_core::canonical_json(v).map_err(|_| PackageError::Record)?;
    Ok(ObjectRef {
        kind: ObjectKind::Json,
        sha256: digest(&bytes),
        byte_length: DecimalU64::parse(&bytes.len().to_string())?,
    })
}
fn dependencies(
    accounting: &AccountingMetadata,
    control_records: &Controls,
    pins: &BTreeSet<ObjectRef>,
    global_nodes: &BTreeSet<ObjectRef>,
    extra: &BTreeSet<ObjectRef>,
) -> Result<BTreeSet<ObjectRef>, PackageError> {
    let mut shared = refs(&accounting.logical)?;
    shared.extend(refs(&accounting.implementation)?);
    shared.remove(&object_ref(&control_records.ledger)?);
    shared.extend(pins.iter().cloned());
    shared.extend(global_nodes.iter().cloned());
    shared.extend(extra.iter().cloned());
    Ok(shared)
}
fn retention_dispatch(
    provider: &impl LogicalRecords,
    envelope: &SelectedEnvelope,
) -> Result<(), PackageError> {
    let evidence = provider.resolve(
        &json_ref(&envelope.root["retention_evidence"])?,
        Membership::Semantic,
    )?;
    if number(&evidence["count"])? > 0
        && !envelope.commit["required_features"]
            .as_array()
            .ok_or(PackageError::Record)?
            .iter()
            .any(|v| v == "photara.resource-retention-evidence.v1")
    {
        return Err(PackageError::UnsupportedFeature);
    }
    Ok(())
}
fn origin_roles(
    roles: &BTreeMap<String, RoleClosure>,
) -> BTreeMap<String, super::origins::OriginRole<'_>> {
    roles
        .iter()
        .map(|(k, reference)| {
            (
                k.clone(),
                super::origins::OriginRole {
                    state: &reference.state,
                    resources: reference.resources.as_ref(),
                },
            )
        })
        .collect()
}
#[expect(
    clippy::too_many_arguments,
    reason = "Explicit independently proved closure components"
)]
fn role_physical<P: PhysicalRecords, M: BlobMetadata + AllocationInspection>(
    envelope: &SelectedEnvelope,
    provider: &P,
    inspection: &M,
    state: &StructuralState,
    proof: &RoleClosure,
    accounting: &AccountingMetadata,
    shared: &BTreeSet<ObjectRef>,
    shared_allocations: &BTreeSet<PackageUuid>,
    limits: &ReaderLimits,
) -> Result<(BTreeSet<PackageUuid>, BTreeSet<ObjectRef>), PackageError> {
    let loc = Locator::new(
        provider,
        envelope.identity.project,
        state.locator.clone(),
        limits.tree,
    );
    let mut actual = located(provider, inspection, envelope, state, limits)?;
    actual.allocations.extend(shared_allocations);
    let owner = audit_ownership(
        &loc,
        inspection,
        &state.ownership,
        &OwnershipContext {
            project: envelope.identity.project,
            accounting,
            expected_allocations: &actual.allocations,
            blobs: &actual.blobs,
            tree_limits: limits.tree,
            max_prefix_bytes: limits.max_prefix_bytes,
        },
    )?;
    let mut expected = proof.semantic.clone();
    expected.extend(proof.inventory_nodes.iter().cloned());
    expected.insert(proof.reference.clone());
    expected.extend(shared.iter().cloned());
    expected.extend(owner.objects.iter().cloned());
    check(expected == actual.objects.keys().cloned().collect())?;
    Ok((actual.allocations, owner.objects))
}
/// Validates every selected root, exact per-root locator/ownership closure and
/// exact global logical union. It uses only supplied immutable records and
/// original registration comparisons; no filesystem or external media reads.
/// # Errors
/// Refuses any unsupported phase, missing or surplus typed dependency, wrong
/// shared prefix/identity/observation, malformed metadata or caller budget limit.
pub fn inspect_settled<P: PhysicalRecords, M: BlobMetadata + AllocationInspection>(
    envelope: &SelectedEnvelope,
    provider: &P,
    control_records: &impl ControlRecords,
    inspection: &M,
    registration: &SettledRegistration,
    limits: &ReaderLimits,
) -> Result<SettledPackage, PackageError> {
    inspect_selected(
        envelope,
        provider,
        control_records,
        inspection,
        registration,
        limits,
        None,
    )
}
#[expect(
    clippy::too_many_lines,
    reason = "Ordered complete selected-package proof"
)]
fn inspect_selected<P: PhysicalRecords, M: BlobMetadata + AllocationInspection>(
    envelope: &SelectedEnvelope,
    provider: &P,
    control_records: &impl ControlRecords,
    inspection: &M,
    registration: &SettledRegistration,
    limits: &ReaderLimits,
    operation: Option<&OperationSelection<'_>>,
) -> Result<SettledPackage, PackageError> {
    let control = controls(envelope, control_records, limits, operation)?;
    let active = envelope.open_role(provider, SelectedRole::Active, limits.tree)?;
    let active_loc = Locator::new(
        provider,
        envelope.identity.project,
        active.locator.clone(),
        limits.tree,
    );
    let (pins, pin_nodes) = pins(&active_loc, envelope, limits)?;
    let mut selected = BTreeMap::from([
        ("active".to_owned(), json_ref(&envelope.root["active"])?),
        ("recovery".to_owned(), json_ref(&envelope.root["recovery"])?),
    ]);
    for pin in &pins {
        selected.insert(
            format!("pin:{}", nonnil(&pin["pin_id"])?),
            json_ref(&pin["root"])?,
        );
    }
    let root: PhysicalRef =
        serde_json::from_value(envelope.root["placement"]["root_placements"].clone())
            .map_err(|_| PackageError::Record)?;
    let placement = PhysicalTree::new(
        provider,
        envelope.identity.project,
        PhysicalTreeKind::RootPlacement,
        limits.tree,
    )
    .audit_with_pages(&root)?;
    let shared_allocations = placement
        .pages
        .iter()
        .map(|reference| reference.allocation_id)
        .collect::<BTreeSet<_>>();
    let actual_roots = placement
        .entries
        .iter()
        .map(|v| json_ref(&v["root"]))
        .collect::<Result<BTreeSet<_>, _>>()?;
    check(actual_roots == selected.values().cloned().collect())?;
    let mut root_ids = BTreeMap::new();
    for entry in &placement.entries {
        let id = nonnil(&entry["root_id"])?;
        let reference = json_ref(&entry["root"])?;
        if let Some(old) = root_ids.insert(id, reference.clone()) {
            check(old == reference)?;
        }
    }
    let holds = active_loc.json(&control.holds, Membership::Ownership)?;
    let account =
        audit_settled_accounting(&control.ledger, &holds, &active_loc, limits.accounting)?;
    registered(&active_loc, envelope, &control, &account, registration)?;
    let (base, global_nodes) = inventory(
        &active_loc,
        &control.base,
        envelope.identity.project,
        limits.tree,
    )?;
    let mut structures = BTreeMap::new();
    let mut roles = BTreeMap::new();
    for (label, reference) in selected {
        let state = envelope.open_reference(provider, &reference, limits.tree)?;
        roles.insert(
            label.clone(),
            role(envelope, provider, inspection, &state, limits)?,
        );
        structures.insert(label, state);
    }
    retention_dispatch(&active_loc, envelope)?;
    let mut extra = super::origins::inspect(
        &active_loc,
        &envelope.root,
        &pins,
        &origin_roles(&roles),
        &envelope.identity,
        limits.resources,
        false,
    )?;
    if let Some(selected) = operation {
        extra.extend(selected.extra_global.keys().cloned());
    }
    let shared = dependencies(&account, &control, &pin_nodes, &global_nodes, &extra)?;
    let mut global = refs(&account.logical)?;
    global.extend(refs(&account.implementation)?);
    global.extend(pin_nodes);
    global.extend(extra);
    global.extend(control.members.iter().cloned());
    let mut allocations = BTreeSet::new();
    let mut associations = BTreeMap::new();
    let mut requirements = BTreeMap::new();
    for (label, proof) in &roles {
        if let Some(selected) = operation {
            let loc = Locator::new(
                provider,
                envelope.identity.project,
                structures[label].locator.clone(),
                limits.tree,
            );
            for (reference, membership) in selected.extra_global {
                loc.json(reference, *membership)?;
            }
        }
        let (used, _owner) = role_physical(
            envelope,
            provider,
            inspection,
            &structures[label],
            proof,
            &account,
            &shared,
            &shared_allocations,
            limits,
        )?;
        allocations.extend(used);
        global.extend(proof.semantic.iter().cloned());
        global.insert(proof.reference.clone());
        // Ownership and locator implementation records are authenticated by
        // physical keep-sets, not the semantic/global inventory member sets.
        if let Some(resource) = &proof.resources {
            for (id, value) in &resource.associations {
                if let Some(old) = associations.insert(id, value) {
                    check(old == value)?;
                }
            }
            for (id, value) in &resource.requirements {
                if let Some(old) = requirements.insert(id, value) {
                    check(old == value)?;
                }
            }
        }
        let active = &roles["active"].receipts;
        check(
            proof.receipts.len() <= active.len()
                && active[..proof.receipts.len()] == proof.receipts,
        )?;
    }
    check(allocations == registration.allocations && base.is_disjoint(&control.members))?;
    let actual_global = base
        .union(&control.members)
        .cloned()
        .collect::<BTreeSet<_>>();
    check(actual_global == global && global.len() as u64 == control.count)?;
    Ok(SettledPackage {
        roles,
        global,
        root: envelope.root.clone(),
        commit: envelope.commit.clone(),
        accounting: account,
    })
}
/// Independent recovery readability. This checks the actual supplied recovery
/// allocations against selected observations, but does not certify absent
/// active files, global registered totals, source retention or write eligibility.
/// # Errors
/// Refuses malformed selected controls or any missing/wrong recovery dependency.
pub fn inspect_recovery<P: PhysicalRecords, M: BlobMetadata + AllocationInspection>(
    envelope: &SelectedEnvelope,
    provider: &P,
    control_records: &impl ControlRecords,
    inspection: &M,
    limits: &ReaderLimits,
) -> Result<RoleClosure, PackageError> {
    let control = controls(envelope, control_records, limits, None)?;
    let state = envelope.open_role(provider, SelectedRole::Recovery, limits.tree)?;
    let loc = Locator::new(
        provider,
        envelope.identity.project,
        state.locator.clone(),
        limits.tree,
    );
    let (pins, pin_nodes) = pins(&loc, envelope, limits)?;
    let holds = loc.json(&control.holds, Membership::Ownership)?;
    let account = audit_settled_accounting(&control.ledger, &holds, &loc, limits.accounting)?;
    let (base, global_nodes) =
        inventory(&loc, &control.base, envelope.identity.project, limits.tree)?;
    check(
        base.is_disjoint(&control.members)
            && base
                .len()
                .checked_add(control.members.len())
                .ok_or(PackageError::Limit)? as u64
                == control.count,
    )?;
    let proof = role(envelope, provider, inspection, &state, limits)?;
    let roles = BTreeMap::from([("recovery".to_owned(), proof)]);
    retention_dispatch(&loc, envelope)?;
    let extra = super::origins::inspect(
        &loc,
        &envelope.root,
        &pins,
        &origin_roles(&roles),
        &envelope.identity,
        limits.resources,
        true,
    )?;
    let shared = dependencies(&account, &control, &pin_nodes, &global_nodes, &extra)?;
    let root: PhysicalRef =
        serde_json::from_value(envelope.root["placement"]["root_placements"].clone())
            .map_err(|_| PackageError::Record)?;
    let placement = PhysicalTree::new(
        provider,
        envelope.identity.project,
        PhysicalTreeKind::RootPlacement,
        limits.tree,
    )
    .audit_with_pages(&root)?;
    let shared_allocations = placement
        .pages
        .iter()
        .map(|reference| reference.allocation_id)
        .collect();
    let proof = roles.into_values().next().ok_or(PackageError::Integrity)?;
    role_physical(
        envelope,
        provider,
        inspection,
        &state,
        &proof,
        &account,
        &shared,
        &shared_allocations,
        limits,
    )?;
    let mut required = refs(&account.logical)?;
    required.extend(refs(&account.implementation)?);
    required.extend(pin_nodes);
    required.extend(extra);
    required.extend(proof.semantic.iter().cloned());
    required.insert(proof.reference.clone());
    required.extend(control.members.iter().cloned());
    for entry in &placement.entries {
        required.insert(json_ref(&entry["root"])?);
    }
    check(required.is_subset(&base.union(&control.members).cloned().collect()))?;
    Ok(proof)
}
#[path = "reader_original.rs"]
mod original;
pub use original::{DirectoryObservation, verify_original};
pub(crate) use original::{inspect_operation_package, verify_operation_original};
