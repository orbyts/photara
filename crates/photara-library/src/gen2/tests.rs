use super::*;
use std::collections::BTreeSet;

fn at() -> Timestamp {
    Timestamp::try_from(1_789_142_400_000).unwrap()
}

async fn fixture() -> (tempfile::TempDir, LocalLibraryStore, Library) {
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let store = LocalLibraryStore::open(
        temp.path().join("local-g2.sqlite"),
        OpenMode::CreateNew,
        DeviceId::new(),
        at(),
    )
    .await
    .unwrap();
    let id = LibraryId::new();
    let library = Library {
        meta: Metadata::new(id, id, at()),
        display_name: "Studio".into(),
        extensions: Extensions::new(),
    };
    store.put_library(&library, None).await.unwrap();
    (temp, store, library)
}
fn party(name: &str) -> PartyDetails {
    PartyDetails {
        display_name: name.into(),
        ..PartyDetails::default()
    }
}
fn person(library: LibraryId) -> Person {
    Person {
        meta: Metadata::new(PersonId::new(), library, at()),
        details: party("Alex"),
        capabilities: [
            "photara.role.photographer".into(),
            "photara.role.stylist".into(),
        ]
        .into(),
    }
}
fn organization(library: LibraryId) -> Organization {
    Organization {
        meta: Metadata::new(OrganizationId::new(), library, at()),
        details: party("Client Organization"),
    }
}
fn kind(library: LibraryId, name: &str) -> LocationKind {
    LocationKind {
        meta: Metadata::new(LocationKindId::new(), library, at()),
        canonical_display: name.into(),
        description: String::new(),
        aliases: BTreeSet::default(),
        extensions: Extensions::new(),
    }
}
fn location(library: LibraryId, kind: LocationKindId, parent: Option<LocationId>) -> Location {
    Location {
        meta: Metadata::new(LocationId::new(), library, at()),
        kind_id: kind,
        parent_id: parent,
        details: party("Ocean Beach"),
        address: Address::default(),
        coordinates: None,
    }
}
fn relationship(
    library: LibraryId,
    person: PersonId,
    organization: OrganizationId,
) -> Relationship {
    Relationship {
        meta: Metadata::new(RelationshipId::new(), library, at()),
        person_id: person,
        organization_id: organization,
        relationship_type: "photara.relationship.client".into(),
        valid_from: None,
        valid_until: None,
        notes: String::new(),
        labels: BTreeSet::default(),
        extensions: Extensions::new(),
    }
}
fn social(library: LibraryId, owner: SocialOwner) -> SocialProfile {
    SocialProfile {
        meta: Metadata::new(SocialProfileId::new(), library, at()),
        owner,
        provider_id: "example.social".into(),
        subject: None,
        handle: Some("alex".into()),
        display_name: "Alex".into(),
        profile_url: Some("https://example.invalid/alex".into()),
        account_kind: AccountKind::Personal,
        provider_account_kind: None,
        verification: VerificationKind::UserAsserted,
        verified_at: Some(at()),
        provenance: SocialProvenance {
            source: "manual".into(),
            notes: String::new(),
        },
        fetched_at: None,
        refreshed_at: None,
        next_refresh_after: None,
        fetch_state: FetchState::Never,
        extensions: Extensions::new(),
    }
}

#[tokio::test]
async fn migrations_initialize_reopen_and_preserve_family() {
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let path = temp.path().join("local-g2.sqlite");
    let device = DeviceId::new();
    let store = LocalLibraryStore::open(&path, OpenMode::CreateNew, device, at())
        .await
        .unwrap();
    assert_eq!(store.info().migration_count, 6);
    store.verify_integrity().await.unwrap();
    let id = store.info().database_id;
    store.close().await;
    let reopened = LocalLibraryStore::open(&path, OpenMode::OpenExisting, device, at())
        .await
        .unwrap();
    assert_eq!(reopened.info().database_id, id);
    reopened.verify_integrity().await.unwrap();
    reopened.close().await;
}

#[tokio::test]
async fn full_schema_pragmas_and_storexa_lifecycle() {
    let (_temp, store, library) = fixture().await;
    let tables:i64=sqlx::query_scalar("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'_sqlx_migrations'").fetch_one(store.db.pool()).await.unwrap();
    assert_eq!(tables, 44);
    assert_eq!(store.info().migration_count, 6);
    store.health().await.unwrap();
    for (sql, expected) in [
        ("PRAGMA foreign_keys", 1),
        ("PRAGMA recursive_triggers", 1),
        ("PRAGMA synchronous", 2),
        ("PRAGMA busy_timeout", 2000),
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(sql)
                .fetch_one(store.db.pool())
                .await
                .unwrap(),
            expected
        );
    }
    let mut lease = store.db.acquire().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT 1")
            .fetch_one(&mut *lease)
            .await
            .unwrap(),
        1
    );
    drop(lease);
    let mut tx = store.db.begin().await.unwrap();
    sqlx::query("INSERT INTO account_cache VALUES(?, 'Transient','active','sr1:1',0)")
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_cache")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        0
    );
    let mut tx = store.db.begin().await.unwrap();
    sqlx::query("INSERT INTO account_cache VALUES(?, 'Transient','active','sr1:1',0)")
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .execute(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_cache")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        0
    );
    assert_eq!(store.library(library.meta.id).await.unwrap(), Some(library));
    let clone = store.clone();
    store.close().await;
    assert!(clone.stats().closed);
    assert_eq!(clone.health().await, Err(Error::Closed));
}

#[tokio::test]
async fn migration_checksum_newer_family_device_and_existing_v1_refusal() {
    for damage in ["checksum", "future", "reader", "writer", "device", "family"] {
        let (temp, store, _) = fixture().await;
        let device = store.info.device_id;
        match damage {
            "checksum" => {
                sqlx::query("UPDATE _sqlx_migrations SET checksum=zeroblob(48) WHERE version=2")
                    .execute(store.db.pool())
                    .await
                    .unwrap();
            }
            "future" => {
                sqlx::query("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(99,'Future',1,zeroblob(48),0)").execute(store.db.pool()).await.unwrap();
            }
            "reader" => {
                sqlx::query("UPDATE schema_metadata SET minimum_reader=2")
                    .execute(store.db.pool())
                    .await
                    .unwrap();
            }
            "family" => {
                sqlx::query("PRAGMA application_id=123")
                    .execute(store.db.pool())
                    .await
                    .unwrap();
            }
            "writer" => {
                sqlx::query("UPDATE schema_metadata SET minimum_writer=2")
                    .execute(store.db.pool())
                    .await
                    .unwrap();
            }
            _ => {}
        }
        store.close().await;
        let result = LocalLibraryStore::open(
            temp.path().join("local-g2.sqlite"),
            OpenMode::OpenExisting,
            if damage == "device" {
                DeviceId::new()
            } else {
                device
            },
            at(),
        )
        .await;
        let expected = match damage {
            "checksum" | "future" => Error::Migration,
            "family" => Error::ForeignDatabase,
            _ => Error::Unsupported,
        };
        assert_eq!(result.unwrap_err(), expected, "{damage}");
    }
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let path = temp.path().join("existing-v1.sqlite");
    let mut old = crate::SqliteLibraryRepository::open(&path).unwrap();
    let old_person = crate::LibraryRecord::Person {
        header: crate::LibraryRecordHeader {
            owner_id: "owner".into(),
            record_id: Uuid::new_v4(),
            revision: 1,
            updated_at_millis: 0,
            deleted: false,
            thumbnail_digest: None,
        },
        display_name: "Preserved".into(),
        aliases: vec![],
        roles: vec![],
    };
    crate::LibraryRepository::put(&mut old, &old_person, None).unwrap();
    drop(old);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        LocalLibraryStore::open(&path, OpenMode::OpenExisting, DeviceId::new(), at()).await,
        Err(Error::ForeignDatabase)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(
        LocalLibraryStore::open(&path, OpenMode::CreateNew, DeviceId::new(), at())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
async fn library_person_organization_crud_cas_history_and_tombstones() {
    let (_temp, store, mut ws) = fixture().await;
    let w = ws.meta.id;
    let mut p = person(w);
    p.details.labels = ["Preferred".into(), "Editorial".into()].into();
    store.put_person(&p, None).await.unwrap();
    assert_eq!(store.person(w, p.meta.id).await.unwrap(), Some(p.clone()));
    assert_eq!(
        store.people(w, None, 500, false).await.unwrap(),
        [p.clone()]
    );
    let mut org = organization(w);
    org.details.labels = ["Client".into()].into();
    store.put_organization(&org, None).await.unwrap();
    assert_eq!(
        store.organization(w, org.meta.id).await.unwrap(),
        Some(org.clone())
    );
    let expected = p.meta.advance(at()).unwrap();
    p.details.display_name = "Renamed".into();
    p.capabilities = ["photara.role.model".into()].into();
    store.put_person(&p, Some(expected)).await.unwrap();
    assert_eq!(
        store.put_person(&p, Some(expected)).await,
        Err(Error::Conflict)
    );
    let changes = store.changes(w, 0, 500).await.unwrap();
    assert_eq!(changes.len(), 4);
    assert_eq!(changes[1].post_state["details"]["display_name"], "Alex");
    assert_eq!(changes[3].post_state["details"]["display_name"], "Renamed");
    assert!(
        store
            .person(LibraryId::new(), p.meta.id)
            .await
            .unwrap()
            .is_none()
    );
    let expected = p.meta.tombstone(at()).unwrap();
    store.put_person(&p, Some(expected)).await.unwrap();
    assert!(store.people(w, None, 10, false).await.unwrap().is_empty());
    assert_eq!(store.people(w, None, 10, true).await.unwrap(), [p]);
    let expected = org.meta.advance(at()).unwrap();
    org.details.description = "A client".into();
    store.put_organization(&org, Some(expected)).await.unwrap();
    let expected = org.meta.tombstone(at()).unwrap();
    store.put_organization(&org, Some(expected)).await.unwrap();
    let expected = ws.meta.advance(at()).unwrap();
    ws.display_name = "Renamed library".into();
    store.put_library(&ws, Some(expected)).await.unwrap();
    let expected = ws.meta.tombstone(at()).unwrap();
    store.put_library(&ws, Some(expected)).await.unwrap();
    assert!(store.libraries(None, 10, false).await.unwrap().is_empty());
    assert_eq!(
        store.put_person(&person(w), None).await,
        Err(Error::Constraint)
    );
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn global_identity_cannot_cross_library_or_change_created_timestamp() {
    let (_temp, store, ws) = fixture().await;
    let mut p = person(ws.meta.id);
    store.put_person(&p, None).await.unwrap();
    let id = LibraryId::new();
    let other = Library {
        meta: Metadata::new(id, id, at()),
        display_name: "Other".into(),
        extensions: Extensions::default(),
    };
    store.put_library(&other, None).await.unwrap();
    p.meta.library_id = id;
    assert_eq!(store.put_person(&p, None).await, Err(Error::Conflict));
    p.meta.library_id = ws.meta.id;
    let expected = p.meta.advance(at()).unwrap();
    p.meta.created_at = Timestamp::try_from(0).unwrap();
    assert_eq!(
        store.put_person(&p, Some(expected)).await,
        Err(Error::Conflict)
    );
    assert_eq!(store.changes(ws.meta.id, 0, 20).await.unwrap().len(), 2);
}

#[tokio::test]
async fn kind_alias_closure_both_orders_rename_tombstone_and_unicode() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/fixtures/generation-two/normalization-and-limits.json"
    ))
    .unwrap();
    for vector in vectors["normalizer"]["vectors"].as_array().unwrap() {
        assert_eq!(
            normalize_term(vector["input"].as_str().unwrap()).unwrap(),
            vector["key"].as_str().unwrap()
        );
    }
    for first in ["Beach", "beaches"] {
        let (_temp, store, ws) = fixture().await;
        let mut k = kind(ws.meta.id, first);
        store.put_location_kind(&k, None).await.unwrap();
        for competing in ["beach", "BEACH", "beaches"] {
            assert_eq!(
                store
                    .put_location_kind(&kind(ws.meta.id, competing), None)
                    .await,
                Err(Error::Conflict)
            );
            assert_eq!(
                store
                    .resolve_location_kind(ws.meta.id, competing)
                    .await
                    .unwrap()
                    .unwrap()
                    .meta
                    .id,
                k.meta.id
            );
        }
        let mut alias = kind(ws.meta.id, "Coast");
        alias.aliases.insert("BEACHES".into());
        assert_eq!(
            store.put_location_kind(&alias, None).await,
            Err(Error::Conflict)
        );
        k = store
            .location_kind(ws.meta.id, k.meta.id)
            .await
            .unwrap()
            .unwrap();
        let expected = k.meta.advance(at()).unwrap();
        k.canonical_display = "Shore".into();
        store.put_location_kind(&k, Some(expected)).await.unwrap();
        assert_eq!(
            store
                .resolve_location_kind(ws.meta.id, first)
                .await
                .unwrap()
                .unwrap()
                .meta
                .id,
            k.meta.id
        );
        k = store
            .location_kind(ws.meta.id, k.meta.id)
            .await
            .unwrap()
            .unwrap();
        let expected = k.meta.tombstone(at()).unwrap();
        store.put_location_kind(&k, Some(expected)).await.unwrap();
        assert!(
            store
                .location_kinds(ws.meta.id, None, 20, false)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            store
                .put_location_kind(&kind(ws.meta.id, "beaches"), None)
                .await,
            Err(Error::Conflict)
        );
        assert_eq!(
            store
                .resolve_location_kind(ws.meta.id, "Beach")
                .await
                .unwrap()
                .unwrap()
                .meta
                .state,
            Lifecycle::Tombstoned
        );
        store.verify_integrity().await.unwrap();
    }
}

#[tokio::test]
async fn competing_independent_pools_create_one_concept_only() {
    let (temp, store, ws) = fixture().await;
    let other = LocalLibraryStore::open(
        temp.path().join("local-g2.sqlite"),
        OpenMode::OpenExisting,
        store.info.device_id,
        at(),
    )
    .await
    .unwrap();
    let a = kind(ws.meta.id, "Beach");
    let b = kind(ws.meta.id, "beaches");
    let (a, b) = tokio::join!(
        store.put_location_kind(&a, None),
        other.put_location_kind(&b, None)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(a == Err(Error::Conflict) || b == Err(Error::Conflict));
    assert_eq!(
        store
            .location_kinds(ws.meta.id, None, 20, true)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.changes(ws.meta.id, 0, 20).await.unwrap().len(), 2);
    other.close().await;
}

#[tokio::test]
async fn required_kind_hierarchy_cross_library_and_retirement_guards() {
    let (_temp, store, ws) = fixture().await;
    let mut k = kind(ws.meta.id, "Beach");
    store.put_location_kind(&k, None).await.unwrap();
    let mut parent = location(ws.meta.id, k.meta.id, None);
    store.put_location(&parent, None).await.unwrap();
    let mut child = location(ws.meta.id, k.meta.id, Some(parent.meta.id));
    store.put_location(&child, None).await.unwrap();
    assert_eq!(
        store
            .put_location(&location(ws.meta.id, LocationKindId::new(), None), None)
            .await,
        Err(Error::Constraint)
    );
    let expected = parent.meta.advance(at()).unwrap();
    parent.parent_id = Some(child.meta.id);
    assert_eq!(
        store.put_location(&parent, Some(expected)).await,
        Err(Error::Constraint)
    );
    parent.parent_id = None;
    parent.meta.state = Lifecycle::Tombstoned;
    assert_eq!(
        store.put_location(&parent, Some(expected)).await,
        Err(Error::Constraint)
    );
    let ke = k.meta.tombstone(at()).unwrap();
    assert_eq!(
        store.put_location_kind(&k, Some(ke)).await,
        Err(Error::Constraint)
    );
    let other = LibraryId::new();
    let w2 = Library {
        meta: Metadata::new(other, other, at()),
        display_name: "Other".into(),
        extensions: Extensions::default(),
    };
    store.put_library(&w2, None).await.unwrap();
    assert_eq!(
        store
            .put_location(&location(other, k.meta.id, None), None)
            .await,
        Err(Error::Constraint)
    );
    let ce = child.meta.tombstone(at()).unwrap();
    store.put_location(&child, Some(ce)).await.unwrap();
    store.put_location(&parent, Some(expected)).await.unwrap();
    store.put_location_kind(&k, Some(ke)).await.unwrap();
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn relationship_roles_intervals_parent_guards_and_rollback() {
    let (_temp, store, ws) = fixture().await;
    let mut p = person(ws.meta.id);
    let o = organization(ws.meta.id);
    store.put_person(&p, None).await.unwrap();
    store.put_organization(&o, None).await.unwrap();
    let mut r = relationship(ws.meta.id, p.meta.id, o.meta.id);
    r.valid_from = Some(Timestamp::try_from(0).unwrap());
    r.valid_until = Some(Timestamp::try_from(10).unwrap());
    store.put_relationship(&r, None).await.unwrap();
    let mut adjacent = relationship(ws.meta.id, p.meta.id, o.meta.id);
    adjacent.valid_from = r.valid_until;
    store.put_relationship(&adjacent, None).await.unwrap();
    let mut overlap = relationship(ws.meta.id, p.meta.id, o.meta.id);
    assert_eq!(
        store.put_relationship(&overlap, None).await,
        Err(Error::Constraint)
    );
    overlap.relationship_type = "photara.relationship.agent".into();
    store.put_relationship(&overlap, None).await.unwrap();
    let expected = p.meta.tombstone(at()).unwrap();
    let before = store.changes(ws.meta.id, 0, 100).await.unwrap().len();
    assert_eq!(
        store.put_person(&p, Some(expected)).await,
        Err(Error::Constraint)
    );
    assert_eq!(
        store.changes(ws.meta.id, 0, 100).await.unwrap().len(),
        before
    );
    let expected = r.meta.advance(at()).unwrap();
    r.notes = "Retained context".into();
    store.put_relationship(&r, Some(expected)).await.unwrap();
    let expected = r.meta.tombstone(at()).unwrap();
    store.put_relationship(&r, Some(expected)).await.unwrap();
    assert_eq!(
        store.relationship(ws.meta.id, r.meta.id).await.unwrap(),
        Some(r)
    );
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn social_manual_warning_subject_reservation_and_owner_immutability() {
    let (_temp, store, ws) = fixture().await;
    let mut p = person(ws.meta.id);
    let o = organization(ws.meta.id);
    store.put_person(&p, None).await.unwrap();
    store.put_organization(&o, None).await.unwrap();
    let mut a = social(ws.meta.id, SocialOwner::Person(p.meta.id));
    store.put_social_profile(&a, None).await.unwrap();
    let mut b = social(ws.meta.id, SocialOwner::Organization(o.meta.id));
    let result = store.put_social_profile(&b, None).await.unwrap();
    assert_eq!(result.duplicate_manual_handles, [a.meta.id]);
    let expected = a.meta.advance(at()).unwrap();
    a.subject = Some(ProviderSubject {
        namespace: "provider-app-1".into(),
        subject_id: "stable-123".into(),
    });
    store.put_social_profile(&a, Some(expected)).await.unwrap();
    let expected_b = b.meta.advance(at()).unwrap();
    b.subject = a.subject.clone();
    assert_eq!(
        store.put_social_profile(&b, Some(expected_b)).await,
        Err(Error::Conflict)
    );
    let mut moved = a.clone();
    let expected = moved.meta.advance(at()).unwrap();
    moved.owner = SocialOwner::Organization(o.meta.id);
    assert_eq!(
        store.put_social_profile(&moved, Some(expected)).await,
        Err(Error::Constraint)
    );
    let expected = a.meta.advance(at()).unwrap();
    a.handle = Some("changed-handle".into());
    store.put_social_profile(&a, Some(expected)).await.unwrap();
    let mut swapped = a.clone();
    let expected = swapped.meta.advance(at()).unwrap();
    swapped.subject.as_mut().unwrap().subject_id = "replacement".into();
    assert_eq!(
        store.put_social_profile(&swapped, Some(expected)).await,
        Err(Error::Constraint)
    );
    let expected = p.meta.tombstone(at()).unwrap();
    assert_eq!(
        store.put_person(&p, Some(expected)).await,
        Err(Error::Constraint)
    );
    let expected = a.meta.tombstone(at()).unwrap();
    store.put_social_profile(&a, Some(expected)).await.unwrap();
    assert_eq!(
        store.put_social_profile(&b, Some(expected_b)).await,
        Err(Error::Conflict)
    );
    store.put_person(&p, Some(Revision::INITIAL)).await.unwrap();
    assert_eq!(
        store.social_profile(ws.meta.id, a.meta.id).await.unwrap(),
        Some(a)
    );
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn typed_limits_and_diagnostics_do_not_leak_values() {
    assert!(Revision::try_from(0).is_err());
    assert!(Revision::try_from(i64::MAX).unwrap().next().is_err());
    assert!(PersonId::try_from(Uuid::nil()).is_err());
    assert!(Timestamp::try_from(i64::MAX).is_err());
    assert!(normalize_term(&"x".repeat(513)).is_err());
    let (_temp, store, ws) = fixture().await;
    let mut p = person(ws.meta.id);
    p.details.display_name = "secret-sensitive-name".repeat(100);
    let err = store.put_person(&p, None).await.unwrap_err();
    assert!(!format!("{err:?} {err}").contains("secret"));
    p.details.display_name = "x".repeat(512);
    store.put_person(&p, None).await.unwrap();
    let expected = p.meta.advance(at()).unwrap();
    p.details.labels = ["Beach".into(), "BEACH".into()].into();
    assert_eq!(
        store.put_person(&p, Some(expected)).await,
        Err(Error::Conflict)
    );
    let mut profile = social(ws.meta.id, SocialOwner::Person(p.meta.id));
    profile.profile_url = Some("https://user:secret@example.invalid/profile?token=secret".into());
    assert_eq!(
        store.put_social_profile(&profile, None).await,
        Err(Error::Invalid)
    );
    let sql_error = sqlx::query("SELECT secret_missing_column FROM libraries")
        .fetch_all(store.db.pool())
        .await
        .unwrap_err();
    let err = Error::from(sql_error);
    assert!(!format!("{err:?} {err}").contains("secret"));
    assert!(!format!("{store:?}").contains("sqlite"));
    assert!(
        !format!(
            "{:?}",
            DeviceBinding::Path("/private/secret/location".into())
        )
        .contains("secret")
    );
}

#[tokio::test]
async fn catalog_roots_locators_bindings_are_device_only_and_cas_checked() {
    let (_temp, store, ws) = fixture().await;
    let w = ws.meta.id;
    let mut root = StorageRoot {
        meta: Metadata::new(StorageRootId::new(), w, at()),
        display_name: "Projects".into(),
        purpose: "photara.projects".into(),
    };
    store.put_storage_root(&root, None).await.unwrap();
    let mut entry = CatalogEntry {
        meta: Metadata::new(ProjectId::new(), w, at()),
        visibility: Visibility::Visible,
        active_locator_id: None,
        selected_observation_id: None,
    };
    store.put_catalog(&entry, None).await.unwrap();
    let mut locator = ProjectLocator {
        meta: Metadata::new(LocatorId::new(), w, at()),
        project_id: entry.meta.id,
        rooted: Some(RootedPath {
            storage_root_id: root.meta.id,
            relative_path: "Nested/Example.photara".into(),
        }),
    };
    store.put_locator(&locator, None).await.unwrap();
    let mut bad = locator.clone();
    bad.meta = Metadata::new(LocatorId::new(), w, at());
    bad.rooted.as_mut().unwrap().relative_path = "../escape.photara".into();
    assert_eq!(store.put_locator(&bad, None).await, Err(Error::Invalid));
    let before = store.changes(w, 0, 100).await.unwrap().len();
    let mut binding = RootBinding {
        library_id: w,
        storage_root_id: root.meta.id,
        binding: DeviceBinding::Path("/Volumes/Private/Projects".into()),
        revision: Revision::INITIAL,
        updated_at: at(),
    };
    store.put_root_binding(&binding, None).await.unwrap();
    assert_eq!(
        store.root_binding(w, root.meta.id).await.unwrap(),
        Some(binding.clone())
    );
    assert_eq!(store.changes(w, 0, 100).await.unwrap().len(), before);
    binding.binding = DeviceBinding::Bookmark(SecureHandleId::new());
    binding.revision = binding.revision.next().unwrap();
    store
        .put_root_binding(&binding, Some(Revision::INITIAL))
        .await
        .unwrap();
    assert_eq!(
        store
            .put_root_binding(&binding, Some(Revision::INITIAL))
            .await,
        Err(Error::Conflict)
    );
    let expected = root.meta.tombstone(at()).unwrap();
    assert_eq!(
        store.put_storage_root(&root, Some(expected)).await,
        Err(Error::Constraint)
    );
    let expected = locator.meta.tombstone(at()).unwrap();
    store.put_locator(&locator, Some(expected)).await.unwrap();
    store
        .put_storage_root(&root, Some(Revision::INITIAL))
        .await
        .unwrap();
    let expected = entry.meta.advance(at()).unwrap();
    entry.visibility = Visibility::Hidden;
    store.put_catalog(&entry, Some(expected)).await.unwrap();
    assert_eq!(
        store.catalog_entry(w, entry.meta.id).await.unwrap(),
        Some(entry)
    );
    for change in store.changes(w, 0, 100).await.unwrap() {
        let text = change.post_state.to_string();
        assert!(
            !text.contains("/Volumes/")
                && !text.contains("secure_handle")
                && !text.contains("selected_observation")
        );
    }
    store.verify_integrity().await.unwrap();
}

fn package_copy(temp: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let root = temp.path().join(name);
    std::fs::create_dir(&root).unwrap();
    let archive: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/fixtures/generation-two/package-specimen.json"
    ))
    .unwrap();
    for entry in archive["files"].as_array().unwrap() {
        let path = root.join(entry["path"].as_str().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, entry["utf8"].as_str().unwrap()).unwrap();
    }
    root
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "One scenario keeps its package verification, explicit selection and duplicate-copy assertions together"
)]
async fn verified_observations_select_explicitly_and_never_publish_packages() {
    let (temp, store, ws) = fixture().await;
    let w = ws.meta.id;
    let package = package_copy(&temp, "Specimen.photara");
    let before = std::fs::read(package.join("HEAD.json")).unwrap();
    let project =
        ProjectId::try_from(Uuid::parse_str("60000000-0000-4000-8000-000000000011").unwrap())
            .unwrap();
    let root = StorageRoot {
        meta: Metadata::new(StorageRootId::new(), w, at()),
        display_name: "Test root".into(),
        purpose: "photara.projects".into(),
    };
    store.put_storage_root(&root, None).await.unwrap();
    store
        .put_root_binding(
            &RootBinding {
                library_id: w,
                storage_root_id: root.meta.id,
                binding: DeviceBinding::Path(temp.path().to_owned()),
                revision: Revision::INITIAL,
                updated_at: at(),
            },
            None,
        )
        .await
        .unwrap();
    let entry = CatalogEntry {
        meta: Metadata::new(project, w, at()),
        visibility: Visibility::Visible,
        active_locator_id: None,
        selected_observation_id: None,
    };
    store.put_catalog(&entry, None).await.unwrap();
    let mut locator = ProjectLocator {
        meta: Metadata::new(LocatorId::new(), w, at()),
        project_id: project,
        rooted: Some(RootedPath {
            storage_root_id: root.meta.id,
            relative_path: "Specimen.photara".into(),
        }),
    };
    store.put_locator(&locator, None).await.unwrap();
    let observation = store
        .observe_package(w, locator.meta.id, &package, at())
        .await
        .unwrap();
    assert_eq!(observation.graph_count, 2);
    assert_eq!(observation.asset_count, 2);
    assert_eq!(
        store
            .observe_package(w, locator.meta.id, &package, at())
            .await
            .unwrap(),
        observation
    );
    assert!(
        store
            .catalog_entry(w, project)
            .await
            .unwrap()
            .unwrap()
            .selected_observation_id
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_party_projection")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM project_location_projection")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        1
    );
    let selected = store
        .select_observation(w, project, Revision::INITIAL, Some(observation.id), at())
        .await
        .unwrap();
    assert_eq!(selected.selected_observation_id, Some(observation.id));
    assert_eq!(
        store
            .select_observation(w, project, Revision::INITIAL, None, at())
            .await,
        Err(Error::Conflict)
    );
    let expected = locator.meta.tombstone(at()).unwrap();
    assert_eq!(
        store.put_locator(&locator, Some(expected)).await,
        Err(Error::Constraint)
    );
    let copy = package_copy(&temp, "Duplicate.photara");
    let other = ProjectLocator {
        meta: Metadata::new(LocatorId::new(), w, at()),
        project_id: project,
        rooted: Some(RootedPath {
            storage_root_id: root.meta.id,
            relative_path: "Duplicate.photara".into(),
        }),
    };
    store.put_locator(&other, None).await.unwrap();
    store
        .observe_package(w, other.meta.id, &copy, at())
        .await
        .unwrap();
    assert_eq!(
        store
            .catalog_entry(w, project)
            .await
            .unwrap()
            .unwrap()
            .selected_observation_id,
        Some(observation.id)
    );
    assert_eq!(std::fs::read(package.join("HEAD.json")).unwrap(), before);
    std::fs::write(copy.join("HEAD.json"), b"{}").unwrap();
    assert_eq!(
        store.observe_package(w, other.meta.id, &copy, at()).await,
        Err(Error::Corrupt)
    );
    let cleared = store
        .select_observation(w, project, selected.meta.revision, None, at())
        .await
        .unwrap();
    assert!(cleared.selected_observation_id.is_none());
    store.put_locator(&locator, Some(expected)).await.unwrap();
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn deferred_claim_and_unexposed_merge_paths_fail_atomically() {
    let (_temp, store, ws) = fixture().await;
    let a = kind(ws.meta.id, "Beach");
    let b = kind(ws.meta.id, "Studio");
    store.put_location_kind(&a, None).await.unwrap();
    store.put_location_kind(&b, None).await.unwrap();
    let mut tx = store.write().await.unwrap();
    assert!(sqlx::query("UPDATE location_kind_terms SET location_kind_id=? WHERE library_id=? AND term_key='beach'").bind(b.meta.id.bytes()).bind(ws.meta.id.bytes()).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    let mut tx = store.write().await.unwrap();
    assert!(
        sqlx::query("DELETE FROM location_kind_terms WHERE library_id=? AND term_key='beach'")
            .bind(ws.meta.id.bytes())
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut tx = store.write().await.unwrap();
    sqlx::query("UPDATE location_kinds SET canonical_key='unclaimed',local_revision=local_revision+1 WHERE location_kind_id=?").bind(a.meta.id.bytes()).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(
        store
            .location_kind(ws.meta.id, a.meta.id)
            .await
            .unwrap()
            .unwrap()
            .meta
            .revision,
        Revision::INITIAL
    );
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn failed_migration_and_replacement_connection_preserve_invariants() {
    use sqlx::SqlSafeStr as _;
    let (_temp, store, _) = fixture().await;
    let lease = store.db.acquire().await.unwrap();
    lease.close().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA recursive_triggers")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        1
    );
    let mut migrations = MIGRATOR.iter().cloned().collect::<Vec<_>>();
    migrations.push(sqlx::migrate::Migration::new(
        7,
        "Deliberate failure".into(),
        sqlx::migrate::MigrationType::Simple,
        "CREATE TABLE rollback_probe(id INTEGER); INVALID SQL;".into_sql_str(),
        false,
    ));
    let migrator = sqlx::migrate::Migrator::with_migrations(migrations);
    let mut tx = store.write().await.unwrap();
    assert!(migrator.run(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_schema WHERE name='rollback_probe'"
        )
        .fetch_one(store.db.pool())
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(store.db.pool())
            .await
            .unwrap(),
        6
    );
    store.verify_integrity().await.unwrap();
}

#[tokio::test]
async fn cloud_association_and_unknown_normalizer_are_not_silently_local() {
    for association in [true, false] {
        let (temp, store, ws) = fixture().await;
        let device = store.info.device_id;
        if association {
            let account = Uuid::new_v4();
            sqlx::query("INSERT INTO account_cache VALUES(?,'Synthetic','active','sr1:1',0)")
                .bind(account.as_bytes().to_vec())
                .execute(store.db.pool())
                .await
                .unwrap();
            sqlx::query("INSERT INTO sync_targets VALUES(?,?,?,'photara-cloud','fixture.invalid','paused',0,0)").bind(Uuid::new_v4().as_bytes().to_vec()).bind(ws.meta.id.bytes()).bind(account.as_bytes().to_vec()).execute(store.db.pool()).await.unwrap();
        } else {
            sqlx::query(
                "INSERT INTO normalization_policies VALUES(2,'99.0.0',zeroblob(32),'Future')",
            )
            .execute(store.db.pool())
            .await
            .unwrap();
            sqlx::query("UPDATE libraries SET term_policy_version=2,local_revision=local_revision+1 WHERE library_id=?").bind(ws.meta.id.bytes()).execute(store.db.pool()).await.unwrap();
        }
        store.close().await;
        assert_eq!(
            LocalLibraryStore::open(
                temp.path().join("local-g2.sqlite"),
                OpenMode::OpenExisting,
                device,
                at()
            )
            .await
            .unwrap_err(),
            Error::Unsupported
        );
    }
}
