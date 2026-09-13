// Integration fixtures intentionally exercise the complete public contract surface.
#![allow(clippy::wildcard_imports)]
use photara_core::contracts::{access::*, asset_set::*, resource::*, schema::*, *};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
fn uid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn project() -> ProjectId {
    ProjectId::from_uuid(uid(1)).unwrap()
}
fn library() -> LibraryId {
    LibraryId::from_uuid(uid(2)).unwrap()
}
fn account() -> AccountId {
    AccountId::from_uuid(uid(3)).unwrap()
}
fn name(s: &str) -> LocalName {
    LocalName::parse(s).unwrap()
}
fn qname(s: &str) -> QualifiedName {
    QualifiedName::parse(s).unwrap()
}
fn digest(n: u8) -> Digest {
    Digest::from_bytes([n; 32])
}
fn object(n: u8) -> ObjectRef {
    ObjectRef {
        kind: ObjectKind::Json,
        sha256: digest(n),
        byte_length: DecimalU64::new(64),
    }
}
fn member(n: u32) -> AssetSetMember {
    let asset_id = AssetId::from_uuid(uid(100 + u128::from(n))).unwrap();
    AssetSetMember {
        ordinal: n,
        asset_id,
        representations: vec![RepresentationSelection {
            project_id: project(),
            asset_id,
            representation_id: AssetRepresentationId::from_uuid(uid(20_000 + u128::from(n)))
                .unwrap(),
            content_revision_id: ContentRevisionId::from_uuid(uid(40_000 + u128::from(n))).unwrap(),
            descriptor: object(1),
        }],
        metadata: vec![],
        missing_facts: vec![],
    }
}
fn snapshot(n: u32) -> AssetSetSnapshot {
    AssetSetSnapshot::new(
        AssetSetSnapshotId::from_uuid(uid(5)).unwrap(),
        project(),
        (0..n).map(member).collect(),
    )
    .unwrap()
}
fn location() -> StorageLocationDescriptor {
    StorageLocationDescriptor {
        library_id: library(),
        location_id: StorageLocationId::from_uuid(uid(6)).unwrap(),
        revision: LocalRevision::INITIAL,
        kind: StorageKind::Filesystem,
        display_name: "Originals".into(),
        purpose: name("originals"),
        supported_rights: ResourceRights::new(vec![ResourceRight::Read]).unwrap(),
        lifecycle: Lifecycle::Active,
    }
}
fn external() -> ExternalResourceRevision {
    ExternalRevisionSpec {
        project_id: project(),
        external_ref_id: ExternalResourceRefId::from_uuid(uid(8)).unwrap(),
        revision_id: ExternalResourceRevisionId::from_uuid(uid(9)).unwrap(),
        source_library_id: library(),
        storage_location_id: location().location_id,
        coordinate: ExternalCoordinate::Filesystem {
            components: RelativeComponents::new(vec!["originals".into(), "café.tif".into()])
                .unwrap(),
        },
        content_evidence: ContentEvidence::Sha256 { digest: digest(1) },
        storage_class: StorageClass::ExternalSource,
    }
    .try_into()
    .unwrap()
}
fn facts(policy: &ProjectPolicy) -> AccessFacts<'_> {
    AccessFacts {
        library_id: library(),
        project_id: project(),
        authority: LibraryAuthority::CloudMember,
        subject: Principal::Account {
            account_id: account(),
        },
        account_active: true,
        identity_active: true,
        library_active: true,
        registration: RegistrationState::Active,
        policy,
        membership: None,
        grant: None,
    }
}
fn grant(mask: ActionMask) -> ProjectGrant {
    ProjectGrant {
        id: ProjectAccessGrantId::from_uuid(uid(7)).unwrap(),
        library_id: library(),
        project_id: project(),
        principal: Principal::Account {
            account_id: account(),
        },
        actions: mask,
        state: GrantState::Active,
        revision: LocalRevision::INITIAL,
    }
}

#[test]
fn every_new_id_rejects_nil_and_noncanonical_wire() {
    macro_rules! check {($($ty:ty),+)=>{$({
        assert!(<$ty>::from_uuid(Uuid::nil()).is_err());
        let id=<$ty>::from_uuid(uid(0xabcdef)).unwrap();
        let bytes=serde_json::to_vec(&id).unwrap();assert_eq!(serde_json::from_slice::<$ty>(&bytes).unwrap(),id);
        for raw in [uid(0xabcdef).simple().to_string(),uid(0xabcdef).to_string().to_uppercase(),format!("urn:uuid:{}",uid(1)),Uuid::nil().to_string()] {assert!(serde_json::from_value::<$ty>(json!(raw)).is_err());}
    })+};}
    check!(
        LibraryId,
        ProjectId,
        GraphId,
        NodeInstanceId,
        AssetId,
        AssetRepresentationId,
        ContentRevisionId,
        StorageLocationId,
        StorageSlotId,
        HostBindingId,
        DeviceId,
        ExternalResourceRefId,
        ExternalResourceRevisionId,
        ProjectResourceId,
        ResourceVersionId,
        AssetSetSnapshotId,
        AccountId,
        LocalPrincipalId,
        ProjectAccessGrantId,
        InvitationId,
        OperationId,
        ReceiptId,
        RunId,
        AttemptId,
        VariableId,
        VariableValueId,
        ExpressionId,
        ContextSnapshotId,
        RunOverrideId,
        CommitId,
        RequestId,
        PersonId,
        OrganizationId,
        LocationId,
        LocationKindId
    );
}
#[test]
fn compatibility_adapters_preserve_exact_uuid_and_old_json() {
    let old = uid(123);
    let lib = LibraryId::from_uuid(old).unwrap();
    assert_eq!(lib.uuid().as_bytes(), old.as_bytes());
    assert_eq!(
        StorageLocationId::from_legacy_storage_root_uuid(old)
            .unwrap()
            .legacy_storage_root_uuid(),
        old
    );
    let legacy = photara_core::ProjectId::from_uuid(old);
    let new = ProjectId::try_from(legacy).unwrap();
    assert_eq!(
        serde_json::to_vec(&legacy).unwrap(),
        serde_json::to_vec(&new).unwrap()
    );
    assert_eq!(photara_core::ProjectId::from(new), legacy);
    assert!(ProjectId::try_from(photara_core::ProjectId::from_uuid(Uuid::nil())).is_err());
    let resolver = photara_core::RepresentationStorageBindingId::from_uuid(old);
    assert_eq!(
        LegacyExternalResolverId::try_from(resolver)
            .unwrap()
            .legacy(),
        resolver
    );
}
#[test]
fn coordinates_versions_and_decimal_boundaries_validate_on_decode() {
    for s in ["", "a", "A.b", "a..b", "a.*", "a.b/", "a.b_"] {
        assert!(QualifiedName::parse(s).is_err());
    }
    for s in ["", "A", "0a", "a.b", "a/b", "a-b"] {
        assert!(LocalName::parse(s).is_err());
    }
    assert!(LocalName::parse("a".repeat(64)).is_ok());
    assert!(LocalName::parse("a".repeat(65)).is_err());
    assert!(serde_json::from_str::<Version>("0").is_err());
    assert!(serde_json::from_str::<LocalRevision>("-1").is_err());
    assert!(LocalRevision::new(i64::MAX).unwrap().next().is_err());
    let max = DecimalU64::new(u64::MAX);
    assert_eq!(
        serde_json::from_str::<DecimalU64>(&serde_json::to_string(&max).unwrap()).unwrap(),
        max
    );
    for raw in ["\"00\"", "\"+1\"", "\"18446744073709551616\"", "1"] {
        assert!(serde_json::from_str::<DecimalU64>(raw).is_err());
    }
    assert!(serde_json::from_value::<Digest>(json!("A".repeat(64))).is_err());
}
#[test]
fn all_masks_match_prerequisites_and_round_trip() {
    for bits in 0..=u16::MAX {
        let valid = bits <= 255
            && (bits & 2 == 0 || bits & 1 == 1)
            && (bits & 0xfc == 0 || bits & 3 == 3)
            && (bits & 128 == 0 || bits & 16 == 16);
        assert_eq!(ActionMask::new(bits).is_ok(), valid, "{bits}");
        assert_eq!(
            serde_json::from_value::<ActionMask>(json!(bits)).is_ok(),
            valid
        );
    }
}
#[test]
fn masks_are_closed_under_union_and_presets_are_exact() {
    let valid: Vec<_> = (0..=255).filter_map(|n| ActionMask::new(n).ok()).collect();
    for a in &valid {
        for b in &valid {
            assert_eq!(ActionMask::new(a.bits() | b.bits()).unwrap(), a.union(*b));
        }
    }
    for (preset, bits) in [
        (ProjectPreset::Discoverer, 1),
        (ProjectPreset::Reader, 3),
        (ProjectPreset::Author, 71),
        (ProjectPreset::Runner, 11),
        (ProjectPreset::AuthorRunner, 79),
        (ProjectPreset::Manager, 255),
    ] {
        assert_eq!(preset.mask().bits(), bits);
    }
    assert!(!ProjectPreset::Author.mask().allows(ProjectAction::Run));
    assert!(!ProjectPreset::Runner.mask().allows(ProjectAction::Edit));
}
#[test]
fn restricted_has_no_membership_or_admin_bypass() {
    let policy = ProjectPolicy::restricted();
    for role in [
        LibraryRole::Owner,
        LibraryRole::Admin,
        LibraryRole::Editor,
        LibraryRole::Viewer,
    ] {
        let mut f = facts(&policy);
        f.membership = Some(Membership {
            library_id: library(),
            account_id: account(),
            role,
            active: true,
        });
        assert_eq!(effective_project_access(&f).unwrap(), ActionMask::NONE);
    }
}
#[test]
fn policy_masks_reject_manager_or_nonzero_restricted_inheritance() {
    for bits in 0..=255 {
        if let Ok(mask) = ActionMask::new(bits) {
            let p = PolicySpec {
                visibility: VisibilityPolicy::LibraryVisible,
                owner: mask,
                admin: ActionMask::NONE,
                editor: ActionMask::NONE,
                viewer: ActionMask::NONE,
            };
            assert_eq!(
                ProjectPolicy::try_from(p).is_ok(),
                [0, 1, 3, 71, 11, 79].contains(&bits)
            );
        }
    }
    let mut p = PolicySpec::from(ProjectPolicy::library_visible_readers());
    p.visibility = VisibilityPolicy::Restricted;
    assert!(ProjectPolicy::try_from(p).is_err());
}
#[test]
fn revoke_denies_inheritance_while_remove_override_and_membership_revoke_differ() {
    let p = ProjectPolicy::library_visible_readers();
    let g = grant(ProjectPreset::Author.mask());
    let mut f = facts(&p);
    f.membership = Some(Membership {
        library_id: library(),
        account_id: account(),
        role: LibraryRole::Editor,
        active: true,
    });
    f.grant = Some(&g);
    assert_eq!(effective_project_access(&f).unwrap().bits(), 71);
    let mut g = g.clone();
    g.state = GrantState::Revoked;
    f.grant = Some(&g);
    assert_eq!(effective_project_access(&f).unwrap().bits(), 0);
    let mut g = g.clone();
    g.state = GrantState::Active;
    g.actions = ActionMask::NONE;
    f.grant = Some(&g);
    assert_eq!(effective_project_access(&f).unwrap().bits(), 3);
    let mut g = g.clone();
    g.actions = ProjectPreset::Author.mask();
    f.grant = Some(&g);
    f.membership.as_mut().unwrap().active = false;
    assert_eq!(effective_project_access(&f).unwrap().bits(), 71);
}
#[test]
fn disabled_closed_or_unregistered_denies_even_explicit_manager() {
    let p = ProjectPolicy::restricted();
    let g = grant(ProjectPreset::Manager.mask());
    for i in 0..5 {
        let mut f = facts(&p);
        f.grant = Some(&g);
        match i {
            0 => f.account_active = false,
            1 => f.identity_active = false,
            2 => f.library_active = false,
            3 => f.registration = RegistrationState::Pending,
            _ => f.registration = RegistrationState::Closed,
        }
        assert_eq!(effective_project_access(&f).unwrap(), ActionMask::NONE);
    }
}
#[test]
fn mixed_scope_and_principal_evidence_never_unions() {
    let p = ProjectPolicy::restricted();
    let mut g = grant(ProjectPreset::Manager.mask());
    g.project_id = ProjectId::from_uuid(uid(999)).unwrap();
    let mut f = facts(&p);
    f.grant = Some(&g);
    assert!(effective_project_access(&f).is_err());
    let mut g = g.clone();
    g.project_id = project();
    g.principal = Principal::Account {
        account_id: AccountId::from_uuid(uid(999)).unwrap(),
    };
    f.grant = Some(&g);
    assert!(effective_project_access(&f).is_err());
}
#[test]
fn project_only_and_local_controller_are_distinct_authorities() {
    let p = ProjectPolicy::restricted();
    let g = grant(ProjectPreset::Reader.mask());
    let mut f = facts(&p);
    f.authority = LibraryAuthority::ProjectOnly;
    f.grant = Some(&g);
    assert_eq!(effective_project_access(&f).unwrap().bits(), 3);
    f.membership = Some(Membership {
        library_id: library(),
        account_id: account(),
        role: LibraryRole::Owner,
        active: true,
    });
    assert!(effective_project_access(&f).is_err());
    let controller = LocalPrincipalId::from_uuid(uid(10)).unwrap();
    let mut g = grant(ProjectPreset::Manager.mask());
    g.principal = Principal::LocalController {
        principal_id: controller,
    };
    let mut f = facts(&p);
    f.authority = LibraryAuthority::LocalOnly { controller };
    f.subject = g.principal;
    f.account_active = false;
    f.identity_active = false;
    f.grant = Some(&g);
    assert_eq!(effective_project_access(&f).unwrap().bits(), 255);
    f.subject = Principal::Account {
        account_id: account(),
    };
    assert!(effective_project_access(&f).is_err());
}
#[test]
fn invitation_ceilings_and_fresh_access_do_not_infer_permissions() {
    assert!(!may_offer_project_grant(
        ProjectPreset::Author.mask(),
        ProjectPreset::Reader.mask(),
        false
    ));
    assert!(may_offer_project_grant(
        ProjectPreset::Manager.mask(),
        ProjectPreset::Manager.mask(),
        true
    ));
    let inviter = ActionMask::new(19).unwrap();
    assert!(may_offer_project_grant(
        inviter,
        ProjectPreset::Reader.mask(),
        false
    ));
    assert!(!may_offer_project_grant(
        inviter,
        ProjectPreset::Reader.mask(),
        true
    ));
    assert!(!may_offer_project_grant(
        inviter,
        ProjectPreset::Author.mask(),
        false
    ));
    assert!(!LibraryRole::Admin.may_invite_role(LibraryRole::Owner));
    assert!(LibraryRole::Owner.may_invite_role(LibraryRole::Owner));
    for op in [
        ProtectedOperation::Run,
        ProtectedOperation::Publish,
        ProtectedOperation::Effect,
    ] {
        assert!(!may_start_protected_operation(
            LibraryAuthority::CloudMember,
            AuthorizationFreshness::LastObserved,
            ProjectPreset::Manager.mask(),
            op
        ));
    }
}
#[test]
fn relative_components_reject_traversal_devices_and_all_control_bytes() {
    for s in [
        "",
        ".",
        "..",
        "/a",
        "a/b",
        "a\\b",
        "C:",
        "file:x",
        "a.",
        "a ",
        "a:b",
        "a?b",
        "a*b",
        "a<b",
        "a>b",
        "a|b",
        "a\"b",
        "CON",
        "nul.tif",
        "com1.txt",
        "LPT9",
        "com¹.txt",
        "CONIN$",
    ] {
        assert!(RelativeComponents::new(vec![s.into()]).is_err(), "{s}");
    }
    for c in 0..=31 {
        assert!(RelativeComponents::new(vec![format!("a{}b", char::from(c))]).is_err());
    }
    assert!(RelativeComponents::new(vec!["a\u{7f}b".into()]).is_err());
    assert!(RelativeComponents::new(vec!["a".repeat(256)]).is_err());
    assert!(RelativeComponents::new(vec!["a".into(); 129]).is_err());
    assert!(RelativeComponents::new(vec!["a".repeat(255); 17]).is_err());
}
#[test]
fn relative_spelling_and_provider_ids_are_never_normalized_or_interpreted() {
    for name in [
        "café.tif",
        "cafe\u{301}.tif",
        "A.tif",
        "a.tif",
        "%2e%2e",
        "COM10.tif",
    ] {
        let p = RelativeComponents::new(vec![name.into()]).unwrap();
        assert_eq!(p.parts(), [name]);
    }
    let p = RelativeComponents::new(vec!["a".into()])
        .unwrap()
        .join(&RelativeComponents::new(vec!["b".into()]).unwrap())
        .unwrap();
    assert_eq!(p.parts(), ["a", "b"]);
    let object = ProviderCoordinate::new("Folder/Object:Version").unwrap();
    assert_eq!(object.as_str(), "Folder/Object:Version");
    assert!(ProviderCoordinate::new("token\nsecret").is_err());
    let message = ProviderCoordinate::new("secret\nvalue")
        .unwrap_err()
        .to_string();
    assert!(!message.contains("secret"));
}
#[test]
fn external_union_scope_and_storage_classes_validate() {
    let revision = external();
    revision.validate_location(&location()).unwrap();
    let mut target = location();
    target.library_id = LibraryId::from_uuid(uid(999)).unwrap();
    assert!(revision.validate_location(&target).is_err());
    target = location();
    target.kind = StorageKind::Provider {
        provider_id: qname("example.provider"),
    };
    assert!(revision.validate_location(&target).is_err());
    let mut spec = ExternalRevisionSpec::from(revision);
    spec.storage_class = StorageClass::ManagedProject;
    assert!(ExternalResourceRevision::try_from(spec).is_err());
    let mut wire = serde_json::to_value(external()).unwrap();
    wire["host_path"] = json!("/secret");
    assert!(serde_json::from_value::<ExternalResourceRevision>(wire).is_err());
}
#[test]
fn provider_coordinate_and_revision_evidence_must_match_storage_kind() {
    let mut spec = ExternalRevisionSpec::from(external());
    spec.content_evidence = ContentEvidence::ProviderRevision {
        revision: ProviderCoordinate::new("r1").unwrap(),
    };
    assert!(ExternalResourceRevision::try_from(spec.clone()).is_err());
    spec.coordinate = ExternalCoordinate::Provider {
        provider_id: qname("example.provider"),
        namespace: ProviderCoordinate::new("media").unwrap(),
        object_id: ProviderCoordinate::new("id").unwrap(),
        revision: Some(ProviderCoordinate::new("r1").unwrap()),
    };
    let r = ExternalResourceRevision::try_from(spec).unwrap();
    let mut l = location();
    l.kind = StorageKind::Provider {
        provider_id: qname("example.provider"),
    };
    r.validate_location(&l).unwrap();
    l.kind = StorageKind::Provider {
        provider_id: qname("other.provider"),
    };
    assert!(r.validate_location(&l).is_err());
}
#[test]
fn slot_capture_pins_identity_revision_and_target_across_refresh() {
    let l = location();
    let mut slot = StorageSlotDescriptor {
        library_id: library(),
        slot_id: StorageSlotId::from_uuid(uid(77)).unwrap(),
        revision: LocalRevision::INITIAL,
        current_name: name("originals"),
        claimed_names: vec![name("originals")],
        display_name: "Originals".into(),
        location_id: l.location_id,
        lifecycle: Lifecycle::Active,
    };
    slot.validate_target(&l).unwrap();
    let capture = slot.capture();
    slot.revision = slot.revision.next().unwrap();
    slot.location_id = StorageLocationId::from_uuid(uid(99)).unwrap();
    assert_ne!(slot.capture(), capture);
    assert_eq!(capture.location_id, l.location_id);
    assert!(slot.validate_target(&l).is_err());
    slot.location_id = l.location_id;
    slot.current_name = name("renamed");
    assert!(slot.validate_target(&l).is_err());
    slot.claimed_names.push(name("renamed"));
    slot.validate_target(&l).unwrap();
}
#[test]
fn package_managed_resources_are_blob_only_and_root_artifacts_are_not_write_paths() {
    let mut spec = ManagedResourceSpec {
        project_id: project(),
        resource_id: ProjectResourceId::from_uuid(uid(11)).unwrap(),
        version_id: ResourceVersionId::from_uuid(uid(12)).unwrap(),
        blob: object(1),
        media_type: MediaType::parse("image/tiff").unwrap(),
    };
    assert!(ManagedProjectResource::try_from(spec.clone()).is_err());
    spec.blob.kind = ObjectKind::Blob;
    let resource = ManagedProjectResource::try_from(spec).unwrap();
    let write = ResourceRights::new(vec![ResourceRight::Create]).unwrap();
    for d in [
        ResourceDescriptor::Managed { resource },
        ResourceDescriptor::ProjectRoot {
            project_id: project(),
        },
        ResourceDescriptor::ProjectArtifacts {
            project_id: project(),
        },
    ] {
        assert!(d.validate_requested_rights(&write).is_err());
    }
}
#[test]
fn live_lease_is_bound_to_exact_resource_operation_and_expected_content() {
    let request = ResolutionRequest {
        project_id: project(),
        operation_id: OperationId::from_uuid(uid(13)).unwrap(),
        descriptor: ResourceDescriptor::External {
            revision: external(),
        },
        expected_content: Some(digest(1)),
        rights: ResourceRights::new(vec![ResourceRight::Read]).unwrap(),
        collision: CollisionPolicy::FailIfPresent,
        max_bytes: DecimalU64::new(100),
    };
    let lease = MaterializedResourceLease::from_host(
        MaterializedHandleId::from_host_uuid(uid(14)).unwrap(),
        request.clone(),
        "private-host-path",
    )
    .unwrap();
    assert_eq!(*lease.for_request(&request).unwrap(), "private-host-path");
    let mut wrong = request.clone();
    wrong.operation_id = OperationId::from_uuid(uid(88)).unwrap();
    assert!(lease.for_request(&wrong).is_err());
    wrong = request.clone();
    wrong.expected_content = Some(digest(2));
    assert!(lease.for_request(&wrong).is_err());
    assert!(!format!("{lease:?}").contains("private"));
    assert_eq!(
        format!("{:?}", SecretRef::from_host("raw-token")),
        "SecretRef([redacted])"
    );
}
#[test]
fn host_status_contract_is_portable_without_paths() {
    for host in [HostKind::Macos, HostKind::Windows, HostKind::Linux] {
        for status in [
            ResolutionStatus::NotBound,
            ResolutionStatus::Unavailable,
            ResolutionStatus::Denied,
            ResolutionStatus::Stale,
            ResolutionStatus::Ambiguous,
            ResolutionStatus::Unsupported,
        ] {
            let s = HostBindingStatus {
                device_id: DeviceId::from_uuid(uid(1)).unwrap(),
                library_id: library(),
                location_id: location().location_id,
                binding_id: HostBindingId::from_uuid(uid(2)).unwrap(),
                host,
                generation: LocalRevision::INITIAL,
                state: BindingState::Verified,
                selected: true,
                status,
            };
            s.validate().unwrap();
            let json = serde_json::to_string(&s).unwrap();
            assert!(!json.contains("path"));
        }
    }
}
#[test]
fn empty_snapshot_and_unusable_members_are_explicit() {
    let empty = snapshot(0);
    assert!(empty.pages(500).unwrap().is_empty());
    assert_eq!(empty.descriptor().unwrap().member_count, 0);
    AssetSetSnapshot::from_pages(&empty.descriptor().unwrap(), &[]).unwrap();
    let mut m = member(0);
    m.representations.clear();
    assert!(AssetSetSnapshot::new(empty.snapshot_id(), project(), vec![m.clone()]).is_err());
    m.missing_facts.push(MissingFact::NoUsableRepresentation);
    AssetSetSnapshot::new(empty.snapshot_id(), project(), vec![m]).unwrap();
}
#[test]
fn asset_membership_order_owners_and_revisions_are_checked() {
    let s = snapshot(2);
    let id = s.snapshot_id();
    let mut members = s.members().to_vec();
    members.swap(0, 1);
    assert!(AssetSetSnapshot::new(id, project(), members).is_err());
    let mut members = s.members().to_vec();
    members[1].asset_id = members[0].asset_id;
    assert!(AssetSetSnapshot::new(id, project(), members).is_err());
    let mut members = s.members().to_vec();
    members[0].representations[0].project_id = ProjectId::from_uuid(uid(77)).unwrap();
    assert!(AssetSetSnapshot::new(id, project(), members).is_err());
    let mut members = s.members().to_vec();
    members[1].representations[0].content_revision_id =
        members[0].representations[0].content_revision_id;
    assert!(AssetSetSnapshot::new(id, project(), members).is_err());
}
#[test]
fn metadata_dependencies_must_match_the_selected_revision() {
    let mut m = member(0);
    m.metadata.push(MetadataDependency {
        project_id: project(),
        asset_id: m.asset_id,
        target: MetadataTarget::Representation {
            representation_id: m.representations[0].representation_id,
            content_revision_id: ContentRevisionId::from_uuid(uid(999)).unwrap(),
        },
        object: object(2),
    });
    assert!(AssetSetSnapshot::new(snapshot(0).snapshot_id(), project(), vec![m.clone()]).is_err());
    m.metadata[0].target = MetadataTarget::Asset;
    AssetSetSnapshot::new(snapshot(0).snapshot_id(), project(), vec![m]).unwrap();
}
#[test]
fn semantic_digest_matches_unchanged_codec_and_excludes_snapshot_identity() {
    let s = snapshot(3);
    let expected =
        Digest::canonical(&json!({"project_id":project(),"members":s.members()})).unwrap();
    assert_eq!(s.content_digest().unwrap(), expected);
    let other = AssetSetSnapshot::new(
        AssetSetSnapshotId::from_uuid(uid(99)).unwrap(),
        project(),
        s.members().to_vec(),
    )
    .unwrap();
    assert_eq!(s.content_digest().unwrap(), other.content_digest().unwrap());
    assert_ne!(s.snapshot_id(), other.snapshot_id());
    assert_ne!(s.content_digest().unwrap(), s.membership_digest().unwrap());
    assert_ne!(s.content_digest().unwrap(), s.dependency_digest().unwrap());
}
#[test]
fn order_revision_metadata_and_missing_facts_each_change_content_digest() {
    let s = snapshot(2);
    let original = s.content_digest().unwrap();
    for i in 0..4 {
        let mut members = s.members().to_vec();
        let first_asset = members[0].asset_id;
        match i {
            0 => {
                members.swap(0, 1);
                members[0].ordinal = 0;
                members[1].ordinal = 1;
            }
            1 => {
                members[0].representations[0].content_revision_id =
                    ContentRevisionId::from_uuid(uid(99)).unwrap();
            }
            2 => members[0].metadata.push(MetadataDependency {
                project_id: project(),
                asset_id: first_asset,
                target: MetadataTarget::Asset,
                object: object(3),
            }),
            _ => members[0]
                .missing_facts
                .push(MissingFact::MetadataUnavailable {
                    schema: SchemaRef {
                        id: qname("example.metadata"),
                        version: Version::FIRST,
                    },
                }),
        }
        let changed = AssetSetSnapshot::new(s.snapshot_id(), project(), members).unwrap();
        assert_ne!(changed.content_digest().unwrap(), original);
    }
}
#[test]
fn page_boundaries_do_not_change_semantics_and_each_page_is_checksum_verified() {
    let s = snapshot(1001);
    for size in [1, 7, 499, 500] {
        let pages: Vec<_> = s
            .pages(size)
            .unwrap()
            .into_iter()
            .map(|p| (p.object_ref().unwrap(), p))
            .collect();
        assert_eq!(
            AssetSetSnapshot::from_pages(&s.descriptor().unwrap(), &pages).unwrap(),
            s
        );
    }
    let mut pages: Vec<_> = s
        .pages(500)
        .unwrap()
        .into_iter()
        .map(|p| (p.object_ref().unwrap(), p))
        .collect();
    pages[0].0.sha256 = digest(33);
    assert!(AssetSetSnapshot::from_pages(&s.descriptor().unwrap(), &pages).is_err());
}
#[test]
fn pages_reject_skip_duplicate_truncation_extra_and_scope_mismatch() {
    let s = snapshot(501);
    let original: Vec<_> = s
        .pages(500)
        .unwrap()
        .into_iter()
        .map(|p| (p.object_ref().unwrap(), p))
        .collect();
    for i in 0..5 {
        let mut pages = original.clone();
        match i {
            0 => {
                pages.remove(0);
            }
            1 => pages.push(pages[1].clone()),
            2 => {
                pages.pop();
            }
            3 => pages.swap(0, 1),
            _ => {
                pages[0].1.token.snapshot_id = AssetSetSnapshotId::from_uuid(uid(77)).unwrap();
                pages[0].0 = pages[0].1.object_ref().unwrap();
            }
        }
        assert!(AssetSetSnapshot::from_pages(&s.descriptor().unwrap(), &pages).is_err());
    }
}
#[test]
fn exact_snapshot_and_page_limits_do_not_truncate() {
    let max = snapshot(10_000);
    assert_eq!(max.members().len(), 10_000);
    assert_eq!(max.pages(500).unwrap().len(), 20);
    assert!(
        AssetSetSnapshot::new(
            max.snapshot_id(),
            project(),
            (0..10_001).map(member).collect()
        )
        .is_err()
    );
    assert!(max.pages(0).is_err());
    assert!(max.pages(501).is_err());
    let mut page = max.pages(500).unwrap().remove(0);
    page.token.start_ordinal = u32::MAX;
    assert!(page.validate().is_err());
}
#[test]
fn v1_assetset_wire_and_cache_semantics_remain_separate() {
    let old = photara_core::AssetSet {
        assets: vec![photara_core::AssetId::from_uuid(uid(100))],
    };
    let old_value = old.to_typed_value().unwrap();
    assert_eq!(old_value.value_type.version.get(), 1);
    assert_eq!(
        photara_core::AssetSet::from_typed_value(&old_value).unwrap(),
        old
    );
    assert!(AssetSetSnapshotDescriptor::from_typed_value(&old_value).is_err());
    let new = snapshot(1).descriptor().unwrap().to_typed_value().unwrap();
    assert!(photara_core::AssetSet::from_typed_value(&new).is_err());
    assert_eq!(
        AssetSetSnapshotDescriptor::from_typed_value(&new).unwrap(),
        snapshot(1).descriptor().unwrap()
    );
}
#[test]
fn variable_schema_admission_recursively_forbids_workflow_and_live_families() {
    for family in [
        ValueFamily::AssetSet,
        ValueFamily::MetadataSet,
        ValueFamily::MetadataPatch,
        ValueFamily::ArtifactSet,
        ValueFamily::GroupSet,
        ValueFamily::EffectReceipt,
        ValueFamily::LiveHandle,
        ValueFamily::SecretRef,
        ValueFamily::WorkflowOutputReference,
    ] {
        let leaf = ValueShape::Leaf {
            value_type: ValueTypeRef {
                id: qname("example.value"),
                version: Version::FIRST,
            },
            family,
        };
        let nested = ValueShape::Record {
            fields: BTreeMap::from([(
                name("items"),
                ValueShape::List {
                    item: Box::new(leaf),
                    max_items: 3,
                },
            )]),
        };
        assert_eq!(
            nested.validate_variable_shape(),
            Err(ContractError::Privacy)
        );
    }
    let spoof = ValueShape::Leaf {
        value_type: ValueTypeRef {
            id: qname("photara.asset-set"),
            version: Version::new(2).unwrap(),
        },
        family: ValueFamily::Configuration,
    };
    assert!(spoof.validate_variable_shape().is_err());
}
#[test]
fn strict_json_and_closed_unions_do_not_drop_unknown_or_duplicate_fields() {
    assert!(decode_strict::<serde_json::Value>(br#"{"a":1,"a":2}"#, 100).is_err());
    assert!(decode_strict::<serde_json::Value>(br#"{"x":{"a":1,"a":2}}"#, 100).is_err());
    assert!(decode_strict::<serde_json::Value>(b"{}", 1).is_err());
    let value = json!({"kind":"project-root","project_id":project(),"host_path":"/private"});
    assert!(serde_json::from_value::<ResourceDescriptor>(value).is_err());
}

#[test]
fn response_scopes_revision_domains_and_generations_do_not_alias() {
    use photara_core::contracts::dto::*;
    let scope = ScopeRef::Library {
        library_id: library(),
    };
    let request_id = RequestId::from_uuid(uid(1)).unwrap();
    let mut m = ResponseMetadata {
        contract_api_version: 2,
        request_id,
        scope,
        base: Some(RevisionCoordinate::Local {
            revision: LocalRevision::INITIAL,
        }),
        authorization_generation: Some(AuthorizationGeneration::new(1).unwrap()),
        state: ResponseState::Ready,
        diagnostic_codes: vec![],
    };
    m.validate().unwrap();
    assert!(m.matches_request(request_id, scope, &m.base, m.authorization_generation));
    assert!(!m.matches_request(
        request_id,
        scope,
        &m.base,
        Some(AuthorizationGeneration::new(2).unwrap())
    ));
    let remote = RevisionCoordinate::Server {
        revision: ServerRevision::try_from("sr1:1".to_owned()).unwrap(),
    };
    assert_ne!(m.base, Some(remote));
    m.base = Some(RevisionCoordinate::Package {
        commit_id: CommitId::from_uuid(uid(1)).unwrap(),
        commit_sha256: digest(1),
    });
    assert!(m.validate().is_err());
    m.base = None;
    assert!(m.validate().is_err());
    m.state = ResponseState::Loading;
    m.validate().unwrap();
    for raw in ["sr1:0", "sr1:01", "sr2:1", "1", "sr1:9223372036854775808"] {
        assert!(ServerRevision::try_from(raw.to_owned()).is_err());
    }
}
#[test]
fn provider_revision_evidence_and_requested_sha_must_agree() {
    let mut spec = ExternalRevisionSpec::from(external());
    spec.coordinate = ExternalCoordinate::Provider {
        provider_id: qname("example.provider"),
        namespace: ProviderCoordinate::new("media").unwrap(),
        object_id: ProviderCoordinate::new("id").unwrap(),
        revision: Some(ProviderCoordinate::new("r1").unwrap()),
    };
    spec.content_evidence = ContentEvidence::ProviderRevision {
        revision: ProviderCoordinate::new("r2").unwrap(),
    };
    assert!(ExternalResourceRevision::try_from(spec).is_err());
    let request = ResolutionRequest {
        project_id: project(),
        operation_id: OperationId::from_uuid(uid(13)).unwrap(),
        descriptor: ResourceDescriptor::External {
            revision: external(),
        },
        expected_content: Some(digest(2)),
        rights: ResourceRights::new(vec![ResourceRight::Read]).unwrap(),
        collision: CollisionPolicy::FailIfPresent,
        max_bytes: DecimalU64::new(100),
    };
    assert_eq!(request.validate(), Err(ContractError::Digest));
}
#[test]
fn invalid_selected_binding_rights_and_replacement_combinations_are_rejected() {
    assert!(ResourceRights::new(vec![ResourceRight::Read, ResourceRight::Read]).is_err());
    let mut s = HostBindingStatus {
        device_id: DeviceId::from_uuid(uid(1)).unwrap(),
        library_id: library(),
        location_id: location().location_id,
        binding_id: HostBindingId::from_uuid(uid(2)).unwrap(),
        host: HostKind::Linux,
        generation: LocalRevision::INITIAL,
        state: BindingState::Candidate,
        selected: true,
        status: ResolutionStatus::Unavailable,
    };
    assert!(s.validate().is_err());
    s.selected = false;
    s.status = ResolutionStatus::Ready;
    assert!(s.validate().is_err());
    let mut spec = ExternalRevisionSpec::from(external());
    spec.storage_class = StorageClass::ExternalOutput;
    let mut request = ResolutionRequest {
        project_id: project(),
        operation_id: OperationId::from_uuid(uid(13)).unwrap(),
        descriptor: ResourceDescriptor::External {
            revision: spec.try_into().unwrap(),
        },
        expected_content: Some(digest(1)),
        rights: ResourceRights::new(vec![ResourceRight::Replace]).unwrap(),
        collision: CollisionPolicy::FailIfPresent,
        max_bytes: DecimalU64::new(100),
    };
    assert!(request.validate().is_err());
    request.collision = CollisionPolicy::Replace {
        expected_target: digest(1),
    };
    request.validate().unwrap();
}
#[test]
fn variable_shape_depth_nodes_and_registered_names_cannot_expand_unbounded() {
    let leaf = ValueShape::Leaf {
        value_type: ValueTypeRef {
            id: qname("example.number"),
            version: Version::FIRST,
        },
        family: ValueFamily::Configuration,
    };
    leaf.validate_variable_shape().unwrap();
    let mut deep = leaf.clone();
    for _ in 0..33 {
        deep = ValueShape::List {
            item: Box::new(deep),
            max_items: 1,
        };
    }
    assert_eq!(deep.validate_variable_shape(), Err(ContractError::Limit));
    let many = ValueShape::Record {
        fields: (0..1024)
            .map(|i| (name(&format!("field_{i}")), leaf.clone()))
            .collect(),
    };
    assert_eq!(many.validate_variable_shape(), Err(ContractError::Limit));
}

#[test]
fn managed_media_types_are_mime_coordinates_not_paths_or_headers() {
    for v in [
        "image/tiff",
        "application/octet-stream",
        "application/vnd.example+json",
    ] {
        assert_eq!(MediaType::parse(v).unwrap().as_str(), v);
    }
    for v in [
        "",
        "image",
        "Image/TIFF",
        "image/tiff; charset=utf8",
        "https://image/tiff",
        "image/",
        "/tiff",
    ] {
        assert!(MediaType::parse(v).is_err());
    }
}
