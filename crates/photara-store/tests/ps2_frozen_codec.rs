//! Production codec compatibility against the reviewed corpus, not a new fixture family.
use photara_store::package::{
    JsonLimits, PackageUuid,
    v1_3::{Arena, FrameKind, FrameLimits, PackedAllocation, PhysicalRef, encode_frame},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn limits() -> FrameLimits {
    FrameLimits {
        json: JsonLimits {
            max_bytes: 1 << 20,
            max_depth: 64,
            max_members: 4096,
            max_array_elements: 4096,
        },
        max_allocation_bytes: 16 << 20,
        max_frames: 65536,
    }
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn reference(id: &str, arena: &str, at: usize, frame: &[u8]) -> PhysicalRef {
    serde_json::from_value(json!({"allocation_id":id,"arena":arena,"offset":at.to_string(),"byte_length":frame.len().to_string(),"record_sha256":format!("{:x}",Sha256::digest(frame))})).unwrap()
}
#[test]
fn frozen_selected_blob_allocations_round_trip_exactly() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
    ))
    .unwrap();
    let mut count = 0;
    for (id, allocation) in fixture["allocations"].as_object().unwrap() {
        if allocation["layout"] == "whole-blob" {
            continue;
        }
        let arena: Arena = serde_json::from_value(allocation["arena"].clone()).unwrap();
        let bytes = unhex(allocation["hex"].as_str().unwrap());
        let checked =
            PackedAllocation::open(PackageUuid::parse(id).unwrap(), arena, &bytes, limits())
                .unwrap();
        let mut at = 0;
        while at < bytes.len() {
            let size =
                u32::from_le_bytes(bytes[at + 12..at + 16].try_into().unwrap()) as usize + 16;
            let frame = &bytes[at..at + size];
            let kind = match frame[8] {
                0 => FrameKind::Padding,
                1 => FrameKind::Semantic,
                2 => FrameKind::Ownership,
                3 => FrameKind::Physical,
                _ => panic!(),
            };
            assert_eq!(
                encode_frame(arena, kind, &frame[16..], limits()).unwrap(),
                frame
            );
            let r = reference(id, allocation["arena"].as_str().unwrap(), at, frame);
            if kind == FrameKind::Padding {
                assert!(checked.resolve(&r, kind).is_err());
            } else {
                assert_eq!(
                    checked.resolve(&r, kind).unwrap(),
                    serde_json::from_slice::<Value>(&frame[16..]).unwrap()
                );
                let wrong = if kind == FrameKind::Semantic {
                    FrameKind::Ownership
                } else {
                    FrameKind::Semantic
                };
                assert!(checked.resolve(&r, wrong).is_err());
                count += 1;
            }
            at += size;
        }
    }
    assert!(count > 10);
}
#[test]
fn framing_refuses_corruption_interior_coordinates_and_budget_exhaustion() {
    let id = "97000000-0000-4000-8000-000000001001";
    let identity = PackageUuid::parse(id).unwrap();
    let frame = encode_frame(Arena::Data, FrameKind::Semantic, b"{}", limits()).unwrap();
    let reader = PackedAllocation::open(identity, Arena::Data, &frame, limits()).unwrap();
    for field in [
        "offset",
        "byte_length",
        "allocation_id",
        "record_sha256",
        "arena",
    ] {
        let mut r = serde_json::to_value(reference(id, "data", 0, &frame)).unwrap();
        r[field] = match field {
            "offset" => json!("1"),
            "byte_length" => json!("17"),
            "allocation_id" => json!("97000000-0000-4000-8000-000000001002"),
            "arena" => json!("metadata"),
            _ => json!("0".repeat(64)),
        };
        assert!(
            reader
                .resolve(&serde_json::from_value(r).unwrap(), FrameKind::Semantic)
                .is_err()
        );
    }
    for index in [0, 8, 9, 12] {
        let mut bad = frame.clone();
        bad[index] = 255;
        assert!(PackedAllocation::open(identity, Arena::Data, &bad, limits()).is_err());
    }
    assert!(
        PackedAllocation::open(identity, Arena::Data, &frame[..frame.len() - 1], limits()).is_err()
    );
    let mut budget = limits();
    budget.max_frames = 0;
    assert!(PackedAllocation::open(identity, Arena::Data, &frame, budget).is_err());
    budget = limits();
    budget.json.max_bytes = 1;
    assert!(PackedAllocation::open(identity, Arena::Data, &frame, budget).is_err());
    assert!(encode_frame(Arena::Data, FrameKind::Semantic, b"{ }", limits()).is_err());
    assert!(encode_frame(Arena::Metadata, FrameKind::Semantic, b"{}", limits()).is_err());
    assert!(encode_frame(Arena::Data, FrameKind::Padding, b"\0x", limits()).is_err());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One frozen D19 scenario verifies the structural/media-audit boundary"
)]
fn scoped_locator_and_blob_audit_reuse_the_frozen_d19_package() {
    use photara_store::package::{
        ObjectRef, PackageError,
        v1_3::{
            BlobAudit, BlobMetadata, Locator, Membership, PhysicalTree, PhysicalTreeKind,
            TreeLimits,
        },
    };
    use std::{
        cell::Cell,
        collections::BTreeMap,
        io::{Cursor, Read},
    };
    struct Raw {
        id: PackageUuid,
        bytes: Vec<u8>,
        opens: Cell<usize>,
    }
    impl BlobMetadata for Raw {
        fn extent(&self, id: PackageUuid) -> Result<u64, PackageError> {
            assert_eq!(id, self.id);
            Ok(self.bytes.len() as u64)
        }
    }
    impl BlobAudit for Raw {
        fn open(&self, id: PackageUuid) -> Result<Box<dyn Read + '_>, PackageError> {
            assert_eq!(id, self.id);
            self.opens.set(self.opens.get() + 1);
            Ok(Box::new(Cursor::new(&self.bytes)))
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
    ))
    .unwrap();
    let commit: Value =
        serde_json::from_str(fixture["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    let project = PackageUuid::parse(commit["project_id"].as_str().unwrap()).unwrap();
    let storage: BTreeMap<_, _> = fixture["allocations"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, a)| a["layout"] != "whole-blob")
        .map(|(id, a)| {
            (
                PackageUuid::parse(id).unwrap(),
                (
                    serde_json::from_value::<Arena>(a["arena"].clone()).unwrap(),
                    unhex(a["hex"].as_str().unwrap()),
                ),
            )
        })
        .collect();
    let provider: BTreeMap<_, _> = storage
        .iter()
        .map(|(id, (arena, bytes))| {
            (
                *id,
                PackedAllocation::open(*id, *arena, bytes, limits()).unwrap(),
            )
        })
        .collect();
    let tree_limits = TreeLimits {
        max_depth: 16,
        max_pages: 256,
        max_entries: 4096,
        max_leaf_entries: 64,
        max_branch_children: 8,
    };
    let root: PhysicalRef =
        serde_json::from_value(commit["root_set"]["placement"]["root_placements"].clone()).unwrap();
    let tree = PhysicalTree::new(
        &provider,
        project,
        PhysicalTreeKind::RootPlacement,
        tree_limits,
    );
    let placements = tree.audit(&root).unwrap();
    for placement in placements {
        let object: ObjectRef = serde_json::from_value(placement["root"].clone()).unwrap();
        assert_eq!(
            tree.lookup(&root, &object).unwrap(),
            Some(placement.clone())
        );
        let locator = Locator::new(
            &provider,
            project,
            serde_json::from_value(placement["locator"].clone()).unwrap(),
            tree_limits,
        );
        let state = locator.json(&object, Membership::Semantic).unwrap();
        assert_eq!(state["schema"]["id"], "photara.package.state-root");
        let blob: ObjectRef = serde_json::from_value(fixture["expected"]["blob"].clone()).unwrap();
        let allocation =
            PackageUuid::parse(fixture["expected"]["blob_allocation"].as_str().unwrap()).unwrap();
        let raw_allocation = &fixture["allocations"][allocation.to_string()];
        let mut raw = Raw {
            id: allocation,
            bytes: unhex(raw_allocation["hex"].as_str().unwrap()),
            opens: Cell::new(0),
        };
        let parsed_commit: photara_store::package::Commit =
            serde_json::from_value(commit.clone()).unwrap();
        let semantic = photara_store::package::v1_3::inspect_legacy_semantics(
            &locator,
            &raw,
            &parsed_commit,
            photara_store::package::PackageLimits::default(),
        )
        .unwrap();
        let expected: std::collections::BTreeSet<photara_store::package::ObjectRef> =
            serde_json::from_value(fixture["expected"]["legacy_closure"].clone()).unwrap();
        assert_eq!(semantic.members(), &expected);
        let identity = photara_store::package::v1_3::SelectionIdentity {
            project,
            library: PackageUuid::parse(commit["root_set"]["library_id"].as_str().unwrap())
                .unwrap(),
            bootstrap_sha256: parsed_commit.bootstrap_sha256,
        };
        let operations = photara_store::package::v1_3::audit_operations(
            &locator,
            &state,
            &identity,
            tree_limits,
        )
        .unwrap();
        assert!(operations.receipts().is_empty());
        let structural = locator.blob(&blob, &raw).unwrap();
        assert_eq!(raw.opens.get(), 0);
        structural.audit(&raw, 5).unwrap();
        assert_eq!(raw.opens.get(), 1);
        raw.bytes[0] ^= 1;
        locator.blob(&blob, &raw).unwrap();
        assert!(structural.audit(&raw, 5).is_err());
    }
}

#[test]
fn selected_envelope_opens_both_roles_and_refuses_coherent_floor_feature_identity_changes() {
    use photara_store::package::{
        Sha256Hex,
        v1_3::{SelectedEnvelope, SelectedRole, SelectionIdentity, TreeLimits},
    };
    use std::collections::BTreeMap;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
    ))
    .unwrap();
    let bootstrap = &fixture["bootstrap"];
    let head = bootstrap["head"].as_str().unwrap().as_bytes();
    let manifest = bootstrap["manifest"].as_str().unwrap().as_bytes();
    let commit = bootstrap["commit"].as_str().unwrap().as_bytes();
    let value: Value = serde_json::from_slice(commit).unwrap();
    let identity = || SelectionIdentity {
        project: PackageUuid::parse(value["project_id"].as_str().unwrap()).unwrap(),
        library: PackageUuid::parse(value["root_set"]["library_id"].as_str().unwrap()).unwrap(),
        bootstrap_sha256: Sha256Hex::parse(value["bootstrap_sha256"].as_str().unwrap()).unwrap(),
    };
    let selected =
        SelectedEnvelope::parse(manifest, head, commit, identity(), limits().json).unwrap();
    let storage: BTreeMap<_, _> = fixture["allocations"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, a)| a["layout"] != "whole-blob")
        .map(|(id, a)| {
            (
                PackageUuid::parse(id).unwrap(),
                (
                    serde_json::from_value::<Arena>(a["arena"].clone()).unwrap(),
                    unhex(a["hex"].as_str().unwrap()),
                ),
            )
        })
        .collect();
    let provider: BTreeMap<_, _> = storage
        .iter()
        .map(|(id, (arena, bytes))| {
            (
                *id,
                PackedAllocation::open(*id, *arena, bytes, limits()).unwrap(),
            )
        })
        .collect();
    let tree_limits = TreeLimits {
        max_depth: 16,
        max_pages: 256,
        max_entries: 4096,
        max_leaf_entries: 64,
        max_branch_children: 8,
    };
    for role in [SelectedRole::Active, SelectedRole::Recovery] {
        selected.open_role(&provider, role, tree_limits).unwrap();
    }
    for case in 0..5 {
        let mut bad = value.clone();
        match case {
            0 => bad["minimum_reader"]["minor"] = json!(2),
            1 => {
                bad["required_features"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unsupported.feature"));
            }
            2 => {
                bad["required_features"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|f| f != "photara.scalable-storage.v1");
            }
            3 => bad["root_set"]["library_id"] = json!("62000000-0000-4000-8000-000000020099"),
            _ => bad["root_set"]["placement"]["generation"] = json!("01"),
        }
        let bytes = photara_core::canonical_json(&bad).unwrap();
        let mut changed_head: Value = serde_json::from_slice(head).unwrap();
        changed_head["commit_sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
        assert!(
            SelectedEnvelope::parse(
                manifest,
                &photara_core::canonical_json(&changed_head).unwrap(),
                &bytes,
                identity(),
                limits().json
            )
            .is_err()
        );
    }
}
