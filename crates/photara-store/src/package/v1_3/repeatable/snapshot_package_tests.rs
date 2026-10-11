//! In-memory snapshot conversion of the existing integrated corpus. No writer,
//! filesystem publication, registration expansion or new fixture family.
use super::*;
use package::{ObjectRef, PackageUuid};
use std::collections::BTreeSet;
use v1_3::{LogicalRecords, Membership, PhysicalRecords};

fn obj(value: &Value) -> ObjectRef {
    serde_json::from_value(reference(value)).unwrap()
}
fn resolve(pool: &BTreeMap<ObjectRef, Value>, reference: &Value) -> Value {
    pool[&serde_json::from_value::<ObjectRef>(reference.clone()).unwrap()].clone()
}
fn member(value: &Value) -> Membership {
    let name = value["schema"]["id"].as_str().unwrap();
    if name.starts_with("photara.storage.")
        || name.starts_with("photara.package.retained-file-charge-")
    {
        Membership::Ownership
    } else {
        Membership::Semantic
    }
}
struct Convert {
    fixture: support::Fixture,
    pool: BTreeMap<ObjectRef, Value>,
    physical: BTreeMap<ObjectRef, Value>,
    cursors: BTreeMap<PackageUuid, usize>,
    project: Value,
    metadata: PackageUuid,
}
impl Convert {
    fn new() -> Self {
        let fixture = support::Fixture::load("integrated");
        let mut physical = BTreeMap::new();
        let mut pool = BTreeMap::new();
        let mut cursors = BTreeMap::new();
        for (id, allocation) in &fixture.allocations {
            let mut at = 0;
            while at < allocation.bytes.len() {
                let kind = allocation.bytes[at + 8];
                let length =
                    u32::from_le_bytes(allocation.bytes[at + 12..at + 16].try_into().unwrap())
                        as usize
                        + 16;
                if kind == 0 {
                    cursors.insert(*id, at);
                    break;
                }
                let bytes = &allocation.bytes[at..at + length];
                let value: Value = serde_json::from_slice(&bytes[16..]).unwrap();
                if kind != 3 {
                    let r = obj(&value);
                    pool.insert(r.clone(), value);
                    physical.entry(r).or_insert_with(|| json!({"allocation_id":id,"arena":allocation.arena,"offset":at.to_string(),"byte_length":length.to_string(),"record_sha256":hash(bytes)}));
                }
                at += length;
            }
        }
        for bytes in fixture.loose.values() {
            let value: Value = serde_json::from_slice(bytes).unwrap();
            pool.insert(obj(&value), value);
        }
        let project =
            serde_json::from_str::<Value>(fixture.data["bootstrap"]["commit"].as_str().unwrap())
                .unwrap()["project_id"]
                .clone();
        Self {
            fixture,
            pool,
            physical,
            cursors,
            project,
            metadata: PackageUuid::parse(&aid(8)).unwrap(),
        }
    }
    fn pack(&mut self, value: Value, kind: v1_3::FrameKind) -> Value {
        let metadata = kind == v1_3::FrameKind::Physical;
        let body = encode(&value);
        let id = if metadata {
            self.metadata
        } else {
            [2, 4, 6]
                .into_iter()
                .map(|n| PackageUuid::parse(&aid(n)).unwrap())
                .find(|id| {
                    self.cursors[id] + body.len() + 32 < self.fixture.allocations[id].bytes.len()
                })
                .expect("existing prepaid data extent")
        };
        if !metadata && let Some(existing) = self.physical.get(&obj(&value)) {
            return existing.clone();
        }
        let allocation = self.fixture.allocations.get_mut(&id).unwrap();
        let bytes = v1_3::encode_frame(allocation.arena, kind, &encode(&value), frames()).unwrap();
        let at = self.cursors[&id];
        let end = at + bytes.len();
        assert!(
            end + 16 < allocation.bytes.len(),
            "existing prepaid corpus extent"
        );
        allocation.bytes[at..end].copy_from_slice(&bytes);
        let remaining = allocation.bytes.len() - end - 16;
        let padding = v1_3::encode_frame(
            allocation.arena,
            v1_3::FrameKind::Padding,
            &vec![0; remaining],
            frames(),
        )
        .unwrap();
        allocation.bytes[end..].copy_from_slice(&padding);
        self.cursors.insert(id, end);
        let physical = json!({"allocation_id":id,"arena":allocation.arena,"offset":at.to_string(),"byte_length":bytes.len().to_string(),"record_sha256":hash(&bytes)});
        if !metadata {
            let r = obj(&value);
            self.pool.insert(r.clone(), value);
            self.physical.insert(r, physical.clone());
        }
        physical
    }
    fn add(&mut self, value: Value) -> ObjectRef {
        let r = obj(&value);
        let kind = if member(&value) == Membership::Ownership {
            v1_3::FrameKind::Ownership
        } else {
            v1_3::FrameKind::Semantic
        };
        self.pack(value, kind);
        r
    }
    fn tree(
        &mut self,
        name: &str,
        mut entries: Vec<Value>,
        key: Option<&str>,
        physical: bool,
    ) -> (Value, BTreeSet<ObjectRef>) {
        let order = |v: &Value| {
            let v = key.map_or(v, |key| &v[key]);
            if v.is_string() {
                v.as_str().unwrap().to_owned()
            } else {
                v["sha256"].as_str().unwrap().to_owned()
            }
        };
        entries.sort_by_key(order);
        let chunks = if entries.is_empty() {
            vec![&entries[..]]
        } else {
            entries
                .chunks(support::tree().max_leaf_entries)
                .collect::<Vec<_>>()
        };
        let mut children = Vec::new();
        let mut nodes = BTreeSet::new();
        for chunk in chunks {
            let value = json!({"schema":{"id":format!("{name}-leaf"),"version":1},"project_id":self.project,"extensions":{},"count":chunk.len().to_string(),"entries":chunk});
            let r = if physical {
                self.pack(value, v1_3::FrameKind::Physical)
            } else {
                let r = self.add(value);
                nodes.insert(r.clone());
                serde_json::to_value(r).unwrap()
            };
            let get = |v: &Value| key.map_or_else(|| v.clone(), |key| v[key].clone());
            children.push(json!({"first":chunk.first().map(get),"last":chunk.last().map(get),"count":chunk.len().to_string(),"child":r}));
        }
        if children.len() == 1 {
            return (children[0]["child"].clone(), nodes);
        }
        assert!(children.len() <= support::tree().max_branch_children);
        let value = json!({"schema":{"id":format!("{name}-branch"),"version":1},"project_id":self.project,"extensions":{},"count":entries.len().to_string(),"children":children});
        let r = if physical {
            self.pack(value, v1_3::FrameKind::Physical)
        } else {
            let r = self.add(value);
            nodes.insert(r.clone());
            serde_json::to_value(r).unwrap()
        };
        (r, nodes)
    }
    fn inventory(&mut self, objects: &BTreeSet<ObjectRef>) -> (Value, BTreeSet<ObjectRef>) {
        self.tree(
            "photara.package.inventory",
            objects
                .iter()
                .map(|r| serde_json::to_value(r).unwrap())
                .collect(),
            None,
            false,
        )
    }
    fn owner(
        &mut self,
        allocations: &BTreeSet<PackageUuid>,
        snapshot: bool,
    ) -> (Value, BTreeSet<ObjectRef>) {
        let mut nodes = BTreeSet::new();
        let mut entries = Vec::new();
        for id in allocations {
            let mut claim = self
                .pool
                .values()
                .find(|v| {
                    v["schema"] == json!({"id":"photara.storage.allocation-claim","version":1})
                        && v["allocation_id"] == id.to_string()
                })
                .unwrap()
                .clone();
            if snapshot {
                claim["schema"]["version"] = json!(2);
                claim
                    .as_object_mut()
                    .unwrap()
                    .remove("authenticated_prefix");
                claim["verification"] = json!("selected-records");
            }
            if !claim["sealed_charge"].is_null() {
                nodes.insert(serde_json::from_value(claim["sealed_charge"].clone()).unwrap());
            }
            let r = self.add(claim);
            nodes.insert(r.clone());
            entries.push(json!({"allocation_id":id,"claim":r}));
        }
        let (root, implementation) = self.tree(
            "photara.storage.ownership",
            entries,
            Some("allocation_id"),
            false,
        );
        nodes.extend(implementation);
        (root, nodes)
    }
    fn locator(&mut self, objects: &BTreeSet<ObjectRef>) -> Value {
        let entries=objects.iter().map(|r| json!({"object":r,"membership":if member(&self.pool[r]) == Membership::Ownership { "ownership" } else { "semantic" },"physical":self.physical[r]})).collect();
        self.tree("photara.storage.locator", entries, Some("object"), true)
            .0
    }
}
impl LogicalRecords for Convert {
    fn resolve(
        &self,
        r: &ObjectRef,
        membership: Membership,
    ) -> Result<Value, package::PackageError> {
        let value = self.pool.get(r).ok_or(package::PackageError::Integrity)?;
        if member(value) != membership {
            return Err(package::PackageError::Integrity);
        }
        Ok(value.clone())
    }
}
struct SnapshotFixture {
    fixture: support::Fixture,
    archived_receipt: (ObjectRef, Value),
    base: Value,
}
fn converted() -> SnapshotFixture {
    converted_archive(true, false)
}
#[expect(
    clippy::too_many_lines,
    reason = "Single existing-corpus conversion keeps physical/accounting identity fixed"
)]
fn converted_archive(owns_historical_allocation: bool, snapshot_recovery: bool) -> SnapshotFixture {
    let mut build = Convert::new();
    let old = build.fixture.verify().unwrap();
    let mut commit: Value =
        serde_json::from_str(build.fixture.data["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    let root = commit["root_set"].clone();
    let active = old.roles()["active"].state().clone();
    let identity = identity(&package(&build.fixture));
    let operations = v1_3::audit_operations(&build, &active, &identity, support::tree()).unwrap();
    let receipt = operations.receipts().last().unwrap();
    let archived_receipt = (obj(receipt), build.physical[&obj(receipt)].clone());
    let old_index = resolve(&build.pool, &active["operation_index"]);
    let base = json!({"schema":{"id":"photara.package.acceptance-base","version":1},"project_id":active["project_id"],"library_id":active["library_id"],"bootstrap_sha256":active["bootstrap_sha256"],"extensions":{},"checkpoint_id":"96000000-0000-4000-8000-000000009901","source_commit":{"commit_id":commit["commit_id"],"commit_sha256":hash(&encode(&commit))},"source_state":reference(&active),"authored_revision":active["authored_revision"],"authored_sha256":active["authored"]["sha256"],"accepted":active["accepted"],"journal_inclusion":active["journal_inclusion"],"receipts_by_id":old_index["by_id"],"receipts_by_ordinal":old_index["by_ordinal"],"receipt_count":active["accepted"]["through_ordinal"]});
    v1_3::snapshot::AcceptanceBase::parse(&encode(&base), &identity, support::limits().json)
        .unwrap()
        .audit_adoption(&build, &commit, &active, &identity, support::tree())
        .unwrap();
    let base_ref = build.add(base.clone());
    let (by_id, id_nodes) = build.tree(
        "photara.package.operation-id",
        vec![],
        Some("operation_id"),
        false,
    );
    let (by_ordinal, ordinal_nodes) = build.tree(
        "photara.package.operation-ordinal",
        vec![],
        Some("acceptance_ordinal"),
        false,
    );
    let index=build.add(json!({"schema":{"id":"photara.package.operation-index","version":3},"project_id":active["project_id"],"library_id":active["library_id"],"bootstrap_sha256":active["bootstrap_sha256"],"extensions":{},"base":base_ref,"accepted":active["accepted"],"by_id":by_id,"by_ordinal":by_ordinal}));
    let mut semantic = old.roles()["active"]
        .semantic()
        .difference(operations.objects())
        .cloned()
        .collect::<BTreeSet<_>>();
    semantic.extend([base_ref, index.clone()]);
    semantic.extend(id_nodes);
    semantic.extend(ordinal_nodes);
    let (inventory, active_inventory_nodes) = build.inventory(&semantic);
    let mut state = active.clone();
    state["schema"]["version"] = json!(3);
    state["operation_index"] = serde_json::to_value(index.clone()).unwrap();
    state["inventory"] = inventory;
    let state_ref = build.add(state.clone());
    let mut all_allocations = build.fixture.registration.allocations.clone();
    if !owns_historical_allocation {
        let receipt = build.pool[&archived_receipt.0].clone();
        build.physical.remove(&archived_receipt.0);
        let old = [2, 4].map(|n| {
            let id = PackageUuid::parse(&aid(n)).unwrap();
            (id, build.cursors.insert(id, 262_144).unwrap())
        });
        build.pack(receipt, v1_3::FrameKind::Semantic);
        for (id, cursor) in old {
            build.cursors.insert(id, cursor);
        }
        all_allocations.remove(&PackageUuid::parse(&aid(6)).unwrap());
    }
    let (archive_owner, archive_nodes) = build.owner(&all_allocations, true);
    let mut archived = operations.objects().clone();
    archived.extend(archive_nodes.iter().cloned());
    let archive_locator = build.locator(&archived);
    let mut global = old.global().clone();
    for proof in old.roles().values() {
        global.remove(proof.reference());
        for r in proof.semantic() {
            global.remove(r);
        }
    }
    for (label, proof) in old.roles() {
        if label == "active" || (snapshot_recovery && label == "recovery") {
            global.insert(state_ref.clone());
            global.extend(semantic.iter().cloned());
        } else {
            global.insert(proof.reference().clone());
            global.extend(proof.semantic().iter().cloned());
        }
    }
    global.extend(archive_nodes);
    let old_overlay = build
        .fixture
        .loose
        .iter()
        .find_map(|(r, b)| {
            let v: Value = serde_json::from_slice(b).unwrap();
            (v["schema"]["id"] == "photara.package.inventory-overlay").then_some((r.clone(), v))
        })
        .unwrap();
    let controls = old_overlay.1["controls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| serde_json::from_value::<ObjectRef>(r.clone()).unwrap())
        .collect::<BTreeSet<_>>();
    let base_members = global.difference(&controls).cloned().collect();
    let (global_root, global_nodes) = build.inventory(&base_members);
    let pins = v1_3::LogicalTree::new(
        &build,
        identity.project,
        v1_3::LogicalTreeKind::RetainedRoot,
        support::tree(),
    )
    .audit(&serde_json::from_value(root["pinned_roots"].clone()).unwrap())
    .unwrap();
    let mut shared = old.global().clone();
    for proof in old.roles().values() {
        shared.remove(proof.reference());
        for r in proof.semantic() {
            shared.remove(r);
        }
    }
    for r in &controls {
        shared.remove(r);
    }
    let accounting = old.accounting();
    for (sha, len) in accounting.implementation.iter().chain(&accounting.logical) {
        shared.insert(
            serde_json::from_value(
                json!({"kind":"json","sha256":sha,"byte_length":len.to_string()}),
            )
            .unwrap(),
        );
    }
    for r in &controls {
        shared.remove(r);
    }
    shared.extend(global_nodes);
    shared.extend(pins.nodes);
    // Ledger/envelope remain loose; accounting dependencies are already members
    // of the old global set. Inventory root itself is an implementation node.
    let mut placements = Vec::new();
    for (label, proof) in old.roles() {
        if snapshot_recovery && label == "recovery" {
            continue;
        }
        let (role_state, role_ref, mut wanted) = if label == "active" {
            (state.clone(), state_ref.clone(), semantic.clone())
        } else {
            (
                proof.state().clone(),
                proof.reference().clone(),
                proof.semantic().clone(),
            )
        };
        let nodes = if label == "active" {
            active_inventory_nodes.clone()
        } else {
            v1_3::LogicalTree::new(
                &build,
                identity.project,
                v1_3::LogicalTreeKind::Inventory,
                support::tree(),
            )
            .audit(&serde_json::from_value(role_state["inventory"].clone()).unwrap())
            .unwrap()
            .nodes
        };
        wanted.extend(nodes);
        wanted.insert(role_ref.clone());
        wanted.extend(shared.iter().cloned());
        let mut allocations = wanted
            .iter()
            .map(|r| serde_json::from_value(build.physical[r]["allocation_id"].clone()).unwrap())
            .collect::<BTreeSet<PackageUuid>>();
        build.metadata = PackageUuid::parse(&aid(match label.as_str() {
            "active" => 3,
            "recovery" => 5,
            _ => 7,
        }))
        .unwrap();
        allocations.insert(build.metadata);
        allocations.insert(PackageUuid::parse(&aid(8)).unwrap());
        allocations.insert(PackageUuid::parse(&aid(2)).unwrap());
        let (owner, nodes) = build.owner(&allocations, label == "active");
        wanted.extend(nodes);
        let locator = build.locator(&wanted);
        placements.push(json!({"root":role_ref,"root_id":role_state["root_id"],"locator":locator,"ownership":owner}));
    }
    build.metadata = PackageUuid::parse(&aid(8)).unwrap();
    let root_placements = build
        .tree(
            "photara.storage.root-placement",
            placements,
            Some("root"),
            true,
        )
        .0;
    let mut overlay = old_overlay.1;
    overlay["base"] = global_root;
    overlay["count"] = json!(global.len().to_string());
    build.fixture.loose.remove(&old_overlay.0);
    build.fixture.loose.insert(obj(&overlay), encode(&overlay));
    commit["minimum_reader"] = json!({"major":1,"minor":4});
    commit["required_features"]
        .as_array_mut()
        .unwrap()
        .push(json!(v1_3::snapshot::FEATURE));
    commit["required_features"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|v| v.as_str().unwrap().to_owned());
    commit["root_set"]["schema"]["version"] = json!(3);
    if snapshot_recovery {
        commit["root_set"]["recovery"] = serde_json::to_value(&state_ref).unwrap();
    }
    commit["root_set"]["active"] = serde_json::to_value(state_ref).unwrap();
    commit["root_set"]["operation_index"] = serde_json::to_value(index).unwrap();
    commit["root_set"]["inventory"] = reference(&overlay);
    commit["root_set"]["placement"]["root_placements"] = root_placements;
    commit["root_set"]["archive_placement"] =
        json!({"locator":archive_locator,"ownership":archive_owner});
    commit["inventory"] = state["inventory"].clone();
    let mut head: Value =
        serde_json::from_str(build.fixture.data["bootstrap"]["head"].as_str().unwrap()).unwrap();
    head["commit_sha256"] = json!(hash(&encode(&commit)));
    build.fixture.data["bootstrap"]["commit"] = json!(String::from_utf8(encode(&commit)).unwrap());
    build.fixture.data["bootstrap"]["head"] = json!(String::from_utf8(encode(&head)).unwrap());
    SnapshotFixture {
        fixture: build.fixture,
        archived_receipt,
        base,
    }
}

#[test]
fn snapshot_package_current_previous_pins_archive_and_exact_charges() {
    let converted = converted();
    let fixture = &converted.fixture;
    let old = support::Fixture::load("integrated");
    for (id, allocation) in &fixture.allocations {
        assert_eq!(
            (allocation.bytes.len(), allocation.charge),
            (old.allocations[id].bytes.len(), old.allocations[id].charge)
        );
    }
    let verified = verify(fixture);
    assert_eq!(
        verified.proof.roles["active"].state["accepted"]["through_ordinal"],
        "3"
    );
}

struct SelectedBytes<'a>(
    BTreeMap<PackageUuid, v1_3::SelectedAllocation<'a>>,
    &'a support::Fixture,
);
impl<'a> SelectedBytes<'a> {
    fn new(fixture: &'a support::Fixture) -> Self {
        Self(
            fixture
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        *id,
                        v1_3::SelectedAllocation::open(*id, a.arena, &a.bytes, frames()).unwrap(),
                    )
                })
                .collect(),
            fixture,
        )
    }
}
impl PhysicalRecords for SelectedBytes<'_> {
    fn audit_legacy_framing(&self, id: PackageUuid) -> Result<(), package::PackageError> {
        let a = self
            .1
            .allocations
            .get(&id)
            .ok_or(package::PackageError::Integrity)?;
        v1_3::PackedAllocation::open(id, a.arena, &a.bytes, frames())?;
        Ok(())
    }
    fn resolve(
        &self,
        r: &v1_3::PhysicalRef,
        kind: v1_3::FrameKind,
    ) -> Result<Value, package::PackageError> {
        self.0
            .get(&r.allocation_id)
            .ok_or(package::PackageError::Integrity)?
            .resolve(r, kind)
    }
    fn resolve_json(
        &self,
        r: &v1_3::PhysicalRef,
        kind: v1_3::FrameKind,
        object: &ObjectRef,
    ) -> Result<Value, package::PackageError> {
        self.0
            .get(&r.allocation_id)
            .ok_or(package::PackageError::Integrity)?
            .resolve_json(r, kind, object)
    }
}
fn settled(f: &support::Fixture) -> Result<v1_3::SettledPackage, package::PackageError> {
    v1_3::inspect_settled(
        &f.envelope(),
        &SelectedBytes::new(f),
        f,
        f,
        &f.registration,
        &support::limits(),
    )
}
fn recovery(f: &support::Fixture) -> Result<v1_3::RoleClosure, package::PackageError> {
    v1_3::inspect_recovery(
        &f.envelope(),
        &SelectedBytes::new(f),
        f,
        f,
        &support::limits(),
    )
}
fn flip(f: &mut support::Fixture, physical: &v1_3::PhysicalRef, offset: usize) {
    f.allocations
        .get_mut(&physical.allocation_id)
        .unwrap()
        .bytes[offset] ^= 1;
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One retained corpus distinguishes requested, archived, and live-role corruption obligations"
)]
fn snapshot_package_archive_requested_corruption_and_live_role_refusals() {
    let mut converted = converted();
    let f = &mut converted.fixture;
    let proof = settled(f).unwrap();
    assert_eq!(proof.roles()["recovery"].receipts().len(), 2);
    assert!(
        proof
            .roles()
            .iter()
            .any(|(name, role)| name.starts_with("pin:") && role.receipts().len() == 1)
    );
    recovery(f).unwrap();
    let physical: v1_3::PhysicalRef =
        serde_json::from_value(converted.archived_receipt.1.clone()).unwrap();
    let receipt = SelectedBytes::new(f)
        .resolve(&physical, v1_3::FrameKind::Semantic)
        .unwrap();
    let operation = PackageUuid::parse(receipt["operation_id"].as_str().unwrap()).unwrap();
    assert_eq!(
        proof
            .archived_operation(
                &f.envelope(),
                &SelectedBytes::new(f),
                f,
                operation,
                None,
                &support::limits()
            )
            .unwrap(),
        Some(receipt)
    );
    assert_eq!(converted.base["receipt_count"], "3");
    let offset = usize::try_from(physical.offset.get()).unwrap() + 16;
    flip(f, &physical, offset);
    // Corruption remains inside the fully charged owned extent, but outside all
    // mandatory current/previous/pinned closures. No historical data is removed.
    settled(f).unwrap();
    let seeded = verify(f);
    let logical = core::snapshot_closure(
        &seeded,
        f.registration.incarnation,
        frames(),
        &support::limits(),
        budget(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(logical.coordinate()).unwrap(),
        request().0["expected"]
    );
    let mut narrow = support::limits();
    narrow.semantic.max_objects = 0;
    assert!(
        core::snapshot_closure(
            &seeded,
            f.registration.incarnation,
            frames(),
            &narrow,
            budget()
        )
        .is_err()
    );
    recovery(f).unwrap();
    assert!(
        proof
            .archived_operation(
                &f.envelope(),
                &SelectedBytes::new(f),
                f,
                operation,
                None,
                &support::limits()
            )
            .is_err()
    );
    flip(f, &physical, offset);
    // A retained v1 role still requires complete framing, even for a frame whose
    // body is not a selected dependency. A new-profile resolver cannot weaken it.
    let header = usize::try_from(physical.offset.get()).unwrap();
    flip(f, &physical, header);
    assert!(settled(f).is_err());
    flip(f, &physical, header);
    for role in [v1_3::SelectedRole::Active, v1_3::SelectedRole::Recovery] {
        let source = SelectedBytes::new(f);
        let structural = f
            .envelope()
            .open_role(&source, role, support::tree())
            .unwrap();
        let entries = v1_3::PhysicalTree::new(
            &source,
            identity(&package(f)).project,
            v1_3::PhysicalTreeKind::Locator,
            support::tree(),
        )
        .audit(&structural.locator)
        .unwrap();
        let state_ref = reference(&structural.state);
        let physical: v1_3::PhysicalRef = serde_json::from_value(
            entries
                .iter()
                .find(|entry| entry["object"] == state_ref)
                .unwrap()["physical"]
                .clone(),
        )
        .unwrap();
        drop(source);
        let offset = usize::try_from(physical.offset.get()).unwrap() + 16;
        flip(f, &physical, offset);
        assert!(settled(f).is_err());
        if matches!(role, v1_3::SelectedRole::Recovery) {
            assert!(recovery(f).is_err());
        }
        flip(f, &physical, offset);
    }
    let id = physical.allocation_id;
    f.allocations.get_mut(&id).unwrap().inode += 1;
    assert!(settled(f).is_err());
}

#[test]
fn snapshot_package_archive_partition_fences_requested_receipt() {
    let converted = converted_archive(false, false);
    let f = &converted.fixture;
    let proof = settled(f).unwrap();
    let original: v1_3::PhysicalRef = serde_json::from_value(converted.archived_receipt.1).unwrap();
    let receipt = SelectedBytes::new(f)
        .resolve(&original, v1_3::FrameKind::Semantic)
        .unwrap();
    let operation = PackageUuid::parse(receipt["operation_id"].as_str().unwrap()).unwrap();
    assert!(
        proof
            .archived_operation(
                &f.envelope(),
                &SelectedBytes::new(f),
                f,
                operation,
                None,
                &support::limits()
            )
            .is_err()
    );
}

#[test]
fn snapshot_package_snapshot_recovery_keeps_its_base_and_archive_edges() {
    let converted = converted_archive(true, true);
    let f = &converted.fixture;
    settled(f).unwrap();
    let proof = recovery(f).unwrap();
    assert_eq!(proof.state()["schema"]["version"], 3);
    assert_eq!(proof.state()["accepted"]["through_ordinal"], "3");
    assert!(proof.receipts().is_empty());
    let bytes = SelectedBytes::new(f);
    let observed = f
        .envelope()
        .open_role(&bytes, v1_3::SelectedRole::Recovery, support::tree())
        .unwrap();
    assert_eq!(proof.reference(), &observed.reference);
}

#[test]
fn snapshot_package_turnover_local_base_binds_old_audit_and_clean_closure() {
    use super::super::checkpoint::{CheckpointIdentity, PreparedHistory};
    use super::super::journal::Journal;
    let old_fixture = support::Fixture::load("integrated");
    let old = verify(&old_fixture);
    let header = super::journal_tests::header(&old.package);
    let limits = super::journal_tests::limits();
    let reader = support::limits();
    let identity = CheckpointIdentity {
        package: identity(&old.package),
        device: PackageUuid::parse(header["device_id"].as_str().unwrap()).unwrap(),
        incarnation: old_fixture.registration.incarnation,
    };
    let context = super::journal_tests::context(&old_fixture, &identity.package, &reader);
    let mut bytes = Journal::create(&header, limits).unwrap();
    let journal = Journal::read(&bytes, &header, limits).unwrap();
    let verified = journal.verify_shared(&old.package, &context).unwrap();
    let inputs = attempt(&old, 501_000).planner;
    let session = verified.verify_session(&old, &inputs, budget()).unwrap();
    let owner = json!({"attachment_id":uid(501_001),"attachment_generation":"1","principal":request().1["provenance"]["principal"]});
    let barrier = session
        .prepare_barrier(
            &owner,
            "close",
            super::journal_tests::record(501_002),
            limits,
        )
        .unwrap();
    bytes.extend(barrier.bytes());
    let journal = Journal::read(&bytes, &header, limits).unwrap();
    let verified = journal.verify_shared(&old.package, &context).unwrap();
    let session = verified.verify_session(&old, &inputs, budget()).unwrap();
    let history = PreparedHistory::prepare(
        &session,
        &old,
        &identity,
        PackageUuid::parse(&uid(501_003)).unwrap(),
        (2, 2),
        &reader,
    )
    .unwrap();
    let converted = converted();
    let clean = verify(&converted.fixture);
    let checkpoint = v1_3::snapshot::AcceptanceBase::parse(
        &encode(&converted.base),
        &identity.package,
        reader.json,
    )
    .unwrap();
    let base = history
        .bind_package(&checkpoint, &clean, &identity, frames(), &reader, budget())
        .unwrap();
    assert_eq!(base.value()["coordinate"], session.coordinate());
    assert_eq!(base.value()["package_head"], clean.package.head);
    assert_eq!(base.value()["lookup"], *history.lookup());
    assert!(base.value()["mutation"].is_null());
    let mut altered = converted.base;
    altered["receipts_by_id"] = altered["receipts_by_ordinal"].clone();
    let changed =
        v1_3::snapshot::AcceptanceBase::parse(&encode(&altered), &identity.package, reader.json)
            .unwrap();
    assert!(
        history
            .bind_package(&changed, &clean, &identity, frames(), &reader, budget())
            .is_err()
    );
    assert!(
        history
            .bind_package(&checkpoint, &old, &identity, frames(), &reader, budget())
            .is_err()
    );
}
