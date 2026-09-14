//! Real `PostgreSQL` proofs. Ignored only because the explicit disposable runner provisions roles.
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use super::*;
use photara_core::contracts::access::*;
use photara_core::contracts::*;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct Fixture {
    pub(crate) service: Service,
    owner: storexa::Database,
    actors: Vec<Actor>,
    device: DeviceId,
    library: LibraryId,
    pub(crate) clock: Arc<auth::FakeClock>,
    auth: Arc<auth::FakeAuth0>,
}
impl Fixture {
    pub(crate) async fn into_service(self) -> Service {
        self.owner.close().await;
        self.service
    }
    pub(crate) async fn new() -> Self {
        let base = std::env::var("PHOTARA_TEST_MIGRATOR_URL")
            .expect("use scripts/verify_service_postgres.py");
        assert!(
            base.contains("photara_cxt3c")
                && base.contains("/private/tmp/")
                && base.contains("socket")
        );
        migrate(storexa::DatabaseConfig::from_url(&base).unwrap())
            .await
            .unwrap();
        let owner = storexa::Database::connect(storexa::DatabaseConfig::from_url(&base).unwrap())
            .await
            .unwrap();
        let clock = Arc::new(auth::FakeClock::new(1_800_000_000_000));
        let auth = Arc::new(auth::FakeAuth0::default());
        let configs = [
            "photara_test_api",
            "photara_test_control",
            "photara_test_auth",
        ]
        .map(|role| {
            storexa::DatabaseConfig::from_url(base.replace("photara_test_migrator", role))
                .unwrap()
                .with_max_connections(2)
        });
        let service = Service::connect(
            configs,
            auth.clone(),
            clock.clone(),
            "https://fake.invalid/".into(),
            "photara-test".into(),
            [37; 32],
        )
        .await
        .unwrap();
        let library = LibraryId::from_uuid(Uuid::new_v4()).unwrap();
        let device = DeviceId::from_uuid(Uuid::new_v4()).unwrap();
        let mut actors = Vec::new();
        let mut tx = owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("SELECT set_config('photara.library_id',$1,true)")
            .bind(library.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        for n in 0..10 {
            let account = AccountId::from_uuid(Uuid::new_v4()).unwrap();
            let identity = Uuid::new_v4();
            sqlx::query("INSERT INTO photara_identity.accounts VALUES($1,$2,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(account.uuid()).bind(format!("fake-{n}")).execute(&mut *tx).await.unwrap();
            sqlx::query("INSERT INTO photara_identity.account_identities VALUES($1,$2,'https://fake.invalid/',$3,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(identity).bind(account.uuid()).bind(account.to_string()).execute(&mut *tx).await.unwrap();
            sqlx::query("INSERT INTO photara_identity.devices VALUES($1,$2,'fake-device','active',CURRENT_TIMESTAMP,NULL)").bind(account.uuid()).bind(device.uuid()).execute(&mut *tx).await.unwrap();
            actors.push(Actor {
                account,
                identity,
                expires_ms: 1_900_000_000_000,
            });
        }
        sqlx::query("INSERT INTO photara.libraries VALUES($1,'Disposable Library','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL,1,'{}')").bind(library.uuid()).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO photara.library_contract_state VALUES($1,1,'cloud-member',NULL,1,1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(library.uuid()).execute(&mut *tx).await.unwrap();
        for (n, role) in ["owner", "admin", "editor", "viewer"].iter().enumerate() {
            sqlx::query("INSERT INTO photara_identity.memberships VALUES($1,$2,$3,$4,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(Uuid::new_v4()).bind(library.uuid()).bind(actors[n].account.uuid()).bind(role).execute(&mut *tx).await.unwrap();
        }
        sqlx::query("INSERT INTO photara_private.scoped_streams VALUES($1,$2,'library',NULL,$3,0,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(library.uuid()).bind(Uuid::new_v4()).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        Self {
            service,
            owner,
            actors,
            device,
            library,
            clock,
            auth,
        }
    }
    fn request(&self, project: Option<ProjectId>, generation: i64) -> Request {
        Request {
            scope: project.map_or(
                Scope::Library {
                    library: self.library,
                },
                |project| Scope::Project {
                    library: self.library,
                    project,
                },
            ),
            device: self.device,
            generation,
        }
    }
    async fn project(&self, generation: i64) -> (ProjectId, i64) {
        let project = ProjectId::from_uuid(Uuid::new_v4()).unwrap();
        let c = RegisterProject {
            operation: op(),
            project,
            disclosure: Disclosure {
                actor: self.actors[0].account,
                commit: CommitId::from_uuid(Uuid::new_v4()).unwrap(),
                commit_sha256: [1; 32],
                projection_sha256: [2; 32],
                policy_sha256: hash(&canonical(&ProjectPolicy::restricted()).unwrap()),
            },
        };
        let result = self
            .service
            .register_project(&self.actors[0], self.request(None, generation), &c)
            .await
            .unwrap();
        (project, result.generation)
    }
    pub(crate) async fn finish(self) {
        self.service.close().await;
        self.owner.close().await;
    }
}
fn op() -> OperationId {
    OperationId::from_uuid(Uuid::new_v4()).unwrap()
}
#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_disposable_contract() {
    let f = Fixture::new().await;
    let claims = auth::VerifiedClaims {
        issuer: "https://fake.invalid/".into(),
        subject: f.actors[0].account.to_string(),
        audience: "photara-test".into(),
        expires_ms: 1_900_000_000_000,
    };
    let identity_token = f.auth.issue(claims.clone()).unwrap();
    assert_eq!(
        f.service
            .authenticate(&identity_token)
            .await
            .unwrap()
            .account(),
        f.actors[0].account
    );
    assert!(
        f.service
            .authenticate("fabricated-signature")
            .await
            .is_err()
    );
    for bad in [
        auth::VerifiedClaims {
            issuer: "https://other.invalid/".into(),
            ..claims.clone()
        },
        auth::VerifiedClaims {
            audience: "other".into(),
            ..claims.clone()
        },
        auth::VerifiedClaims {
            expires_ms: 1,
            ..claims.clone()
        },
        auth::VerifiedClaims {
            subject: "unlinked-subject".into(),
            ..claims.clone()
        },
    ] {
        assert!(
            f.service
                .authenticate(&f.auth.issue(bad).unwrap())
                .await
                .is_err()
        );
    }
    let (project, mut generation) = f.project(1).await;
    let actions = [
        ProjectAction::Discover,
        ProjectAction::Read,
        ProjectAction::Edit,
        ProjectAction::Run,
        ProjectAction::Invite,
        ProjectAction::ManageStorage,
        ProjectAction::ManageContext,
        ProjectAction::ManageAccess,
    ];
    let mut cells = 0;
    for (n, actor) in f.actors.iter().enumerate() {
        for action in actions {
            assert_eq!(
                f.service
                    .permits(actor, f.request(Some(project), generation), action)
                    .await
                    .unwrap(),
                n == 0,
                "restricted actor {n} {action:?}"
            );
            cells += 1;
        }
    }
    for (n, actor) in f.actors.iter().enumerate() {
        for action in actions {
            let expected = [247u16, 247, 71, 3, 0, 0, 0, 0, 0, 0][n] & action as u16 != 0;
            assert_eq!(
                f.service
                    .permits(actor, f.request(None, generation), action)
                    .await
                    .unwrap(),
                expected,
                "Library actor {n} {action:?}"
            );
            cells += 1;
        }
    }
    // Real API login checks RLS without the protected controller privileges.
    for (n, actor) in f.actors.iter().enumerate() {
        let mut tx = f.service.api.begin().await.unwrap();
        runtime::set_context(&mut tx, actor, f.request(Some(project), generation))
            .await
            .unwrap();
        let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM photara.project_catalog")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(visible, i64::from(n == 0));
        let libs: i64 = sqlx::query_scalar("SELECT count(*) FROM photara.libraries")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(libs, 0);
        tx.rollback().await.unwrap();
    }
    let grant = ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap();
    let c = SetGrant {
        operation: op(),
        target: f.actors[4].account,
        grant,
        mask: ProjectPreset::Reader.mask(),
        change: GrantChange::Create,
        expected_revision: None,
    };
    let receipt = f
        .service
        .set_grant(&f.actors[0], f.request(Some(project), generation), &c)
        .await
        .unwrap();
    generation = receipt.generation;
    assert!(
        f.service
            .permits(
                &f.actors[4],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    assert!(
        !f.service
            .permits(
                &f.actors[4],
                f.request(None, generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    assert_eq!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &c)
            .await
            .unwrap(),
        receipt
    );
    let changed = SetGrant {
        mask: ProjectPreset::Author.mask(),
        ..c.clone()
    };
    assert!(matches!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &changed)
            .await,
        Err(ServiceError::Conflict)
    ));
    let revoke = SetGrant {
        operation: op(),
        change: GrantChange::Revoke,
        expected_revision: Some(1),
        ..c.clone()
    };
    generation = f
        .service
        .set_grant(&f.actors[0], f.request(Some(project), generation), &revoke)
        .await
        .unwrap()
        .generation;
    assert!(
        !f.service
            .permits(
                &f.actors[4],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    let bad = SetGrant {
        operation: op(),
        change: GrantChange::Replace,
        expected_revision: Some(2),
        ..c.clone()
    };
    assert!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &bad)
            .await
            .is_err()
    );
    let regrant = SetGrant {
        change: GrantChange::Regrant,
        ..bad
    };
    generation = f
        .service
        .set_grant(&f.actors[0], f.request(Some(project), generation), &regrant)
        .await
        .unwrap()
        .generation;
    // Fresh request generation is mandatory even for a retained successful receipt.
    assert!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), 1), &c)
            .await
            .is_err()
    );
    let inv = Invite {
        operation: op(),
        invitation: InvitationId::from_uuid(Uuid::new_v4()).unwrap(),
        target: f.actors[5].account,
        mask: ProjectPreset::Reader.mask(),
        expected_policy_revision: generation,
        expires_ms: 1_800_000_060_000,
    };
    let (_, token) = f
        .service
        .invite(&f.actors[0], f.request(Some(project), generation), &inv)
        .await
        .unwrap();
    assert!(
        !f.service
            .permits(
                &f.actors[5],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    let accept = AcceptInvitation {
        operation: op(),
        invitation: inv.invitation,
        grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        token,
    };
    assert!(
        f.service
            .accept_invitation(&f.actors[6], f.request(Some(project), generation), &accept)
            .await
            .is_err()
    );
    let bad = AcceptInvitation {
        token: [0; 32],
        ..accept.clone()
    };
    assert!(
        f.service
            .accept_invitation(&f.actors[5], f.request(Some(project), generation), &bad)
            .await
            .is_err()
    );
    let accepted = f
        .service
        .accept_invitation(&f.actors[5], f.request(Some(project), generation), &accept)
        .await
        .unwrap();
    generation = accepted.generation;
    assert_eq!(
        f.service
            .accept_invitation(&f.actors[5], f.request(Some(project), generation), &accept)
            .await
            .unwrap(),
        accepted
    );
    // Commit, explicit rollback and dropped transaction must clear every request field.
    for finish in 0..3 {
        let tx = f
            .service
            .begin(
                &f.actors[0],
                f.request(Some(project), generation),
                ProjectAction::Read,
                &[],
            )
            .await
            .unwrap();
        match finish {
            0 => tx.commit().await.unwrap(),
            1 => tx.rollback().await.unwrap(),
            _ => drop(tx),
        }
        let mut pooled = f.service.control.acquire().await.unwrap();
        for key in [
            "account_id",
            "identity_id",
            "library_id",
            "scope_kind",
            "project_id",
            "device_id",
            "purpose",
            "authorization_generation",
        ] {
            let value: Option<String> =
                sqlx::query_scalar("SELECT nullif(current_setting($1,true),'')")
                    .bind(format!("photara.{key}"))
                    .fetch_one(&mut *pooled)
                    .await
                    .unwrap();
            assert_eq!(value, None, "pooled {key}");
        }
    }
    let bootstrap = f
        .service
        .snapshot(&f.actors[0], f.request(None, generation))
        .await
        .unwrap();
    assert!(bootstrap.roots.is_empty());
    let root = StorageProjection {
        library: f.library,
        root: StorageLocationId::from_uuid(Uuid::new_v4()).unwrap(),
        revision: 1,
        display_name: "Test storage".into(),
        purpose: "source".into(),
        retired: false,
    };
    let content = PublishContent {
        operation: op(),
        expected_revision: None,
        root: ContentRoot::Storage(root.clone()),
    };
    assert!(
        f.service
            .publish_content(&f.actors[3], f.request(None, generation), &content)
            .await
            .is_err()
    );
    assert!(
        f.service
            .publish_content(&f.actors[0], f.request(Some(project), generation), &content)
            .await
            .is_err()
    );
    let accepted = f
        .service
        .publish_content(&f.actors[0], f.request(None, generation), &content)
        .await
        .unwrap();
    assert_eq!(
        f.service
            .publish_content(&f.actors[0], f.request(None, generation), &content)
            .await
            .unwrap(),
        accepted
    );
    let page = f
        .service
        .feed(
            &f.actors[0],
            f.request(None, generation),
            &bootstrap.cursor,
            100,
        )
        .await
        .unwrap();
    assert_eq!(page.batches.len(), 1);
    assert_eq!(page.batches[0].roots, vec![content.root.clone()]);
    f.service
        .acknowledge(&f.actors[0], f.request(None, generation), &page.cursor)
        .await
        .unwrap();
    assert!(
        f.service
            .acknowledge(&f.actors[0], f.request(None, generation), &bootstrap.cursor)
            .await
            .is_err()
    );
    assert!(
        f.service
            .feed(
                &f.actors[4],
                f.request(Some(project), generation),
                &page.cursor,
                100
            )
            .await
            .is_err()
    );
    let mut forged = serde_json::to_value(&page.cursor).unwrap();
    forged["signature"][0] = serde_json::json!(0);
    let forged = serde_json::from_value(forged).unwrap();
    assert!(
        f.service
            .feed(&f.actors[0], f.request(None, generation), &forged, 10)
            .await
            .is_err()
    );
    let changed = PublishContent {
        operation: content.operation,
        root: ContentRoot::Storage(StorageProjection {
            display_name: "changed".into(),
            ..root.clone()
        }),
        expected_revision: None,
    };
    assert!(
        f.service
            .publish_content(&f.actors[0], f.request(None, generation), &changed)
            .await
            .is_err()
    );
    let update = PublishContent {
        operation: op(),
        expected_revision: Some(1),
        root: ContentRoot::Storage(StorageProjection {
            revision: 2,
            display_name: "Updated storage".into(),
            ..root
        }),
    };
    let competing = PublishContent {
        operation: op(),
        ..update.clone()
    };
    let a = f
        .service
        .publish_content(&f.actors[0], f.request(None, generation), &update);
    let b = f
        .service
        .publish_content(&f.actors[0], f.request(None, generation), &competing);
    let (a, b) = tokio::join!(a, b);
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let project_bootstrap = f
        .service
        .snapshot(&f.actors[4], f.request(Some(project), generation))
        .await
        .unwrap();
    assert!(project_bootstrap.roots.is_empty());
    let projection = CatalogProjection {
        library: f.library,
        project,
        observation: Uuid::new_v4(),
        locator: Uuid::new_v4(),
        commit: CommitId::from_uuid(Uuid::new_v4()).unwrap(),
        commit_sha256: [8; 32],
        package_revision: 1,
        title: "Disposable Project".into(),
        asset_count: 3,
        graph_count: 2,
    };
    let observed = PublishContent {
        operation: op(),
        expected_revision: None,
        root: ContentRoot::Catalog(projection.clone()),
    };
    f.service
        .publish_content(
            &f.actors[0],
            f.request(Some(project), generation),
            &observed,
        )
        .await
        .unwrap();
    let project_page = f
        .service
        .feed(
            &f.actors[4],
            f.request(Some(project), generation),
            &project_bootstrap.cursor,
            100,
        )
        .await
        .unwrap();
    assert_eq!(project_page.batches[0].roots, vec![observed.root]);
    assert_eq!(
        f.service
            .snapshot(&f.actors[0], f.request(None, generation))
            .await
            .unwrap()
            .roots
            .len(),
        1
    );
    let bytes = b"fake image immutable bytes".to_vec();
    let media = FakeMedia::default();
    let upload = CreateUpload {
        operation: op(),
        digest: hash(&bytes),
        media_type: "image/png".into(),
        byte_length: i64::try_from(bytes.len()).unwrap(),
        purpose: Some(MediaPurpose::Cover),
        consent: Some(Disclosure {
            actor: f.actors[0].account,
            commit: projection.commit,
            commit_sha256: projection.commit_sha256,
            projection_sha256: hash(&canonical(&projection).unwrap()),
            policy_sha256: hash(&canonical(&ProjectPolicy::restricted()).unwrap()),
        }),
    };
    f.service
        .create_upload(&f.actors[0], f.request(Some(project), generation), &upload)
        .await
        .unwrap();
    assert!(
        f.service
            .finalize_upload(
                &f.actors[0],
                f.request(Some(project), generation),
                upload.operation,
                op(),
                &media
            )
            .await
            .is_err()
    );
    media.stage(upload.operation.uuid(), bytes).unwrap();
    assert!(
        f.service
            .finalize_upload(
                &f.actors[4],
                f.request(Some(project), generation),
                upload.operation,
                op(),
                &media
            )
            .await
            .is_err()
    );
    f.service
        .finalize_upload(
            &f.actors[0],
            f.request(Some(project), generation),
            upload.operation,
            op(),
            &media,
        )
        .await
        .unwrap();
    assert!(
        f.service
            .media_url(
                &f.actors[4],
                f.request(Some(project), generation),
                upload.digest,
                Some(MediaPurpose::Cover),
                &media
            )
            .await
            .unwrap()
            .starts_with("https://fake-media.invalid/")
    );
    assert!(
        f.service
            .media_url(
                &f.actors[4],
                f.request(Some(project), generation),
                upload.digest,
                Some(MediaPurpose::AssignedSnapshot),
                &media
            )
            .await
            .is_err()
    );
    assert!(
        f.service
            .media_url(
                &f.actors[4],
                f.request(None, generation),
                upload.digest,
                None,
                &media
            )
            .await
            .is_err()
    );
    assert_eq!(
        f.service
            .entitlement(
                &f.actors[4],
                f.request(Some(project), generation),
                "unknown"
            )
            .await
            .unwrap(),
        (false, None, false)
    );
    let library_bytes = b"Library-only media".to_vec();
    let library_upload = CreateUpload {
        operation: op(),
        digest: hash(&library_bytes),
        media_type: "image/png".into(),
        byte_length: i64::try_from(library_bytes.len()).unwrap(),
        purpose: None,
        consent: None,
    };
    f.service
        .create_upload(&f.actors[0], f.request(None, generation), &library_upload)
        .await
        .unwrap();
    media
        .stage(library_upload.operation.uuid(), library_bytes)
        .unwrap();
    f.service
        .finalize_upload(
            &f.actors[0],
            f.request(None, generation),
            library_upload.operation,
            op(),
            &media,
        )
        .await
        .unwrap();
    assert!(
        f.service
            .media_url(
                &f.actors[3],
                f.request(None, generation),
                library_upload.digest,
                None,
                &media
            )
            .await
            .is_ok()
    );
    assert!(
        f.service
            .media_url(
                &f.actors[4],
                f.request(Some(project), generation),
                library_upload.digest,
                Some(MediaPurpose::Cover),
                &media
            )
            .await
            .is_err()
    );
    let mut billing = f
        .service
        .begin(
            &f.actors[0],
            f.request(None, generation),
            ProjectAction::ManageAccess,
            &[],
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO photara_private.library_entitlement_grants(grant_id,library_id,capability_key,source,enabled,quota_limit,valid_from,valid_until,state,revision,created_at,updated_at) VALUES($1,$2,'test-quota','manual',true,42,CURRENT_TIMESTAMP-interval '1 hour',CURRENT_TIMESTAMP+interval '1 hour','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(f.library.uuid()).execute(&mut *billing).await.unwrap();
    sqlx::query("INSERT INTO photara_private.account_developer_grants(grant_id,account_id,capability_key,reason_code,valid_from,valid_until,state,revision,created_at,updated_at) VALUES($1,$2,'test-quota','disposable test',CURRENT_TIMESTAMP-interval '1 hour',CURRENT_TIMESTAMP+interval '1 hour','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(f.actors[4].account.uuid()).execute(&mut *billing).await.unwrap();
    sqlx::query("INSERT INTO photara_private.library_entitlement_grants(grant_id,library_id,capability_key,source,enabled,valid_from,valid_until,state,revision,created_at,updated_at) VALUES($1,$2,'expired','manual',true,CURRENT_TIMESTAMP-interval '2 hours',CURRENT_TIMESTAMP-interval '1 hour','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(f.library.uuid()).execute(&mut *billing).await.unwrap();
    billing.commit().await.unwrap();
    assert_eq!(
        f.service
            .entitlement(
                &f.actors[4],
                f.request(Some(project), generation),
                "test-quota"
            )
            .await
            .unwrap(),
        (true, Some(42), true)
    );
    assert_eq!(
        f.service
            .entitlement(
                &f.actors[5],
                f.request(Some(project), generation),
                "test-quota"
            )
            .await
            .unwrap(),
        (true, Some(42), false)
    );
    assert_eq!(
        f.service
            .entitlement(
                &f.actors[4],
                f.request(Some(project), generation),
                "expired"
            )
            .await
            .unwrap(),
        (false, None, false)
    );
    assert!(
        !f.service
            .permits(
                &f.actors[4],
                f.request(Some(project), generation),
                ProjectAction::Run
            )
            .await
            .unwrap()
    );
    f.clock.set(1_950_000_000_000);
    assert!(f.service.authenticate(&identity_token).await.is_err());
    f.clock.set(1_800_000_000_000);
    println!(
        "CXT3c: {cells} authorization cells; exact identity, Project-only isolation, receipts, explicit regrant, invitation token/target, pooled cleanup passed"
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_access_races_and_guards() {
    let f = Fixture::new().await;
    let (project, mut generation) = f.project(1).await;
    let mut tx = f
        .service
        .begin(
            &f.actors[0],
            f.request(Some(project), generation),
            ProjectAction::ManageAccess,
            &[],
        )
        .await
        .unwrap();
    let manager:Uuid=sqlx::query_scalar("SELECT grant_id FROM photara_identity.project_access_grants WHERE library_id=$1 AND project_id=$2 AND account_id=$3").bind(f.library.uuid()).bind(project.uuid()).bind(f.actors[0].account.uuid()).fetch_one(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let remove = SetGrant {
        operation: op(),
        target: f.actors[0].account,
        grant: ProjectAccessGrantId::from_uuid(manager).unwrap(),
        mask: ProjectPreset::Manager.mask(),
        change: GrantChange::Revoke,
        expected_revision: Some(1),
    };
    assert!(matches!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &remove)
            .await,
        Err(ServiceError::Conflict)
    ));
    assert!(
        f.service
            .permits(
                &f.actors[0],
                f.request(Some(project), generation),
                ProjectAction::ManageAccess
            )
            .await
            .unwrap()
    );
    // Last-identity revocation is checked against all active Project managers at commit.
    let mut tx = f
        .service
        .context(&f.actors[0], f.request(Some(project), generation), &[])
        .await
        .unwrap();
    sqlx::query("UPDATE photara_identity.account_identities SET state='revoked',revoked_at=CURRENT_TIMESTAMP,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE identity_id=$1").bind(f.actors[0].identity).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    let masks = [
        ProjectPreset::Discoverer,
        ProjectPreset::Reader,
        ProjectPreset::Author,
        ProjectPreset::Runner,
        ProjectPreset::AuthorRunner,
        ProjectPreset::Manager,
    ];
    for (n, preset) in masks.iter().enumerate() {
        let c = SetGrant {
            operation: op(),
            target: f.actors[n + 4].account,
            grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
            mask: preset.mask(),
            change: GrantChange::Create,
            expected_revision: None,
        };
        generation = f
            .service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &c)
            .await
            .unwrap()
            .generation;
    }
    let actions = [
        ProjectAction::Discover,
        ProjectAction::Read,
        ProjectAction::Edit,
        ProjectAction::Run,
        ProjectAction::Invite,
        ProjectAction::ManageStorage,
        ProjectAction::ManageContext,
        ProjectAction::ManageAccess,
    ];
    let mut cells = 0;
    for (n, preset) in masks.iter().enumerate() {
        for action in actions {
            assert_eq!(
                f.service
                    .permits(
                        &f.actors[n + 4],
                        f.request(Some(project), generation),
                        action
                    )
                    .await
                    .unwrap(),
                preset.mask().allows(action)
            );
            cells += 1;
        }
    }
    // Execute all preset cells through the ordinary API role's actual policy helpers too.
    for (n, preset) in masks.iter().enumerate() {
        let mut tx = f.service.api.begin().await.unwrap();
        runtime::set_context(
            &mut tx,
            &f.actors[n + 4],
            f.request(Some(project), generation),
        )
        .await
        .unwrap();
        for action in actions {
            let a = serde_json::to_value(action).unwrap();
            let allowed: bool = sqlx::query_scalar("SELECT photara_private.can_project($1,$2,$3)")
                .bind(f.library.uuid())
                .bind(project.uuid())
                .bind(a.as_str().unwrap())
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            assert_eq!(allowed, preset.mask().allows(action));
            cells += 1;
        }
        let catalog: i64 =
            sqlx::query_scalar("SELECT count(*) FROM photara.project_catalog WHERE project_id=$1")
                .bind(project.uuid())
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert_eq!(
            catalog,
            i64::from(preset.mask().allows(ProjectAction::Read))
        );
        tx.rollback().await.unwrap();
    }
    // API can neither read protected tables nor execute private trigger functions.
    for table in [
        "photara_identity.project_access_grants",
        "photara_identity.project_invitations",
        "photara_private.project_invitation_secrets",
        "photara_private.scoped_streams",
        "photara_private.scoped_changes",
        "photara_private.scoped_change_batches",
        "photara_private.scoped_command_receipts",
        "photara_private.scoped_sync_clients",
        "photara_private.media_objects",
        "photara_private.media_upload_sessions",
    ] {
        for verb in ["SELECT * FROM", "DELETE FROM", "UPDATE"] {
            let sql = if verb == "UPDATE" {
                {
                    let col = if table.ends_with("project_invitation_secrets") {
                        "invitation_id"
                    } else if table.ends_with("scoped_sync_clients") {
                        "stream_id"
                    } else {
                        "library_id"
                    };
                    format!("UPDATE {table} SET {col}={col}")
                }
            } else {
                format!("{verb} {table}")
            };
            let mut tx = f.service.api.begin().await.unwrap();
            runtime::set_context(&mut tx, &f.actors[9], f.request(Some(project), generation))
                .await
                .unwrap();
            let error = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .execute(&mut *tx)
                .await
                .unwrap_err();
            assert_eq!(
                error.as_database_error().unwrap().code().as_deref(),
                Some("42501")
            );
            tx.rollback().await.unwrap();
            cells += 1;
        }
    }
    let mut tx = f.service.api.begin().await.unwrap();
    let error = sqlx::query("SELECT photara_private.d19_lock_actor()")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("42501")
    );
    tx.rollback().await.unwrap();
    // An INSERT of a Library root through a Project-only context is denied even to a manager.
    let mut tx = f.service.api.begin().await.unwrap();
    runtime::set_context(&mut tx, &f.actors[9], f.request(Some(project), generation))
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO photara.storage_roots VALUES($1,$2,'deny','deny','source',1,'active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(Uuid::new_v4()).bind(f.library.uuid()).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    let stored = StorageProjection {
        library: f.library,
        root: StorageLocationId::from_uuid(Uuid::new_v4()).unwrap(),
        revision: 1,
        display_name: "RLS root".into(),
        purpose: "source".into(),
        retired: false,
    };
    f.service
        .publish_content(
            &f.actors[0],
            f.request(None, generation),
            &PublishContent {
                operation: op(),
                expected_revision: None,
                root: ContentRoot::Storage(stored.clone()),
            },
        )
        .await
        .unwrap();
    for (n, actor) in f.actors.iter().enumerate() {
        let mut tx = f.service.api.begin().await.unwrap();
        runtime::set_context(&mut tx, actor, f.request(None, generation))
            .await
            .unwrap();
        for action in actions {
            let value = serde_json::to_value(action).unwrap();
            let allowed: bool = sqlx::query_scalar("SELECT photara_private.d19_can_library($1,$2)")
                .bind(f.library.uuid())
                .bind(value.as_str().unwrap())
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            assert_eq!(
                allowed,
                [247u16, 247, 71, 3, 0, 0, 0, 0, 0, 0][n] & action as u16 != 0
            );
            cells += 1;
        }
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM photara.storage_roots WHERE library_id=$1")
                .bind(f.library.uuid())
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert_eq!(rows, i64::from(n < 4));
        cells += 1;
        let changed=sqlx::query("UPDATE photara.storage_roots SET display_name='RLS updated',revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND storage_root_id=$2").bind(f.library.uuid()).bind(stored.root.uuid()).execute(&mut *tx).await.unwrap().rows_affected();
        assert_eq!(changed, u64::from(n < 2));
        cells += 1;
        tx.rollback().await.unwrap();
        let mut tx = f.service.api.begin().await.unwrap();
        runtime::set_context(&mut tx, actor, f.request(None, generation))
            .await
            .unwrap();
        let result=sqlx::query("INSERT INTO photara.storage_roots VALUES($1,$2,'RLS insert','rls-insert','source',1,'active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(Uuid::new_v4()).bind(f.library.uuid()).execute(&mut *tx).await;
        assert_eq!(result.is_ok(), n < 2);
        cells += 1;
        tx.rollback().await.unwrap();
        let mut tx = f.service.api.begin().await.unwrap();
        runtime::set_context(&mut tx, actor, f.request(None, generation))
            .await
            .unwrap();
        assert!(
            sqlx::query("DELETE FROM photara.storage_roots WHERE library_id=$1")
                .bind(f.library.uuid())
                .execute(&mut *tx)
                .await
                .is_err()
        );
        cells += 1;
        tx.rollback().await.unwrap();
    }
    // Two access CAS operations race from the same generation: only one may commit.
    let a = SetGrant {
        operation: op(),
        target: f.actors[1].account,
        grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        mask: ProjectPreset::Reader.mask(),
        change: GrantChange::Create,
        expected_revision: None,
    };
    let b = SetGrant {
        operation: op(),
        target: f.actors[2].account,
        grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        ..a.clone()
    };
    let (a, b) = tokio::join!(
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &a),
        f.service
            .set_grant(&f.actors[0], f.request(Some(project), generation), &b)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    generation += 1;
    // Complete feed closure rejects a high-water advance with no receipt/batch.
    let mut tx = f
        .service
        .begin(
            &f.actors[0],
            f.request(Some(project), generation),
            ProjectAction::Edit,
            &[],
        )
        .await
        .unwrap();
    sqlx::query("UPDATE photara_private.scoped_streams SET last_sequence=last_sequence+1 WHERE library_id=$1 AND project_id=$2").bind(f.library.uuid()).bind(project.uuid()).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    // Atomic manager transfer revokes the source before inserting the replacement.
    let c = TransferManager {
        operation: op(),
        target: f.actors[3].account,
        new_grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        old_grant: ProjectAccessGrantId::from_uuid(manager).unwrap(),
        expected_old_revision: 1,
    };
    generation = f
        .service
        .transfer_manager(&f.actors[0], f.request(Some(project), generation), &c)
        .await
        .unwrap()
        .generation;
    assert!(
        !f.service
            .permits(
                &f.actors[0],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    assert!(
        f.service
            .permits(
                &f.actors[3],
                f.request(Some(project), generation),
                ProjectAction::ManageAccess
            )
            .await
            .unwrap()
    );
    let second = LibraryId::from_uuid(Uuid::new_v4()).unwrap();
    let request = Request {
        scope: Scope::Library { library: second },
        device: f.device,
        generation: 1,
    };
    let claim = ClaimLibrary {
        operation: op(),
        display_name: "Second disposable Library".into(),
    };
    let receipt = f
        .service
        .claim_library(&f.actors[0], request, &claim)
        .await
        .unwrap();
    assert_eq!(
        f.service
            .claim_library(&f.actors[0], request, &claim)
            .await
            .unwrap(),
        receipt
    );
    assert!(
        f.service
            .claim_library(&f.actors[4], request, &claim)
            .await
            .is_err()
    );
    assert!(
        !f.service
            .permits(
                &f.actors[3],
                Request {
                    scope: Scope::Project {
                        library: second,
                        project
                    },
                    device: f.device,
                    generation: 1
                },
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    // Current API floor refusal applies to an already connected service.
    let mut tx = f.owner.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE photara.schema_metadata SET minimum_api=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        f.service
            .permits(&f.actors[0], request, ProjectAction::Read)
            .await,
        Err(ServiceError::Unsupported)
    ));
    let mut tx = f.owner.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE photara.schema_metadata SET minimum_api=3")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    println!(
        "CXT3c: {cells} additional unprivileged access/privilege cells; last manager, last identity, atomic transfer, access CAS, feed rollback, Library collision and live floor refusal passed"
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_inherited_matrix_and_revocation() {
    let f = Fixture::new().await;
    let member = SetMembership {
        operation: op(),
        target: f.actors[6].account,
        role: LibraryRole::Editor,
        revoked: false,
        expected_revision: None,
    };
    let mut generation = f
        .service
        .set_membership(&f.actors[0], f.request(None, 1), &member)
        .await
        .unwrap()
        .generation;
    let project = ProjectId::from_uuid(Uuid::new_v4()).unwrap();
    let c = RegisterProject {
        operation: op(),
        project,
        disclosure: Disclosure {
            actor: f.actors[2].account,
            commit: CommitId::from_uuid(Uuid::new_v4()).unwrap(),
            commit_sha256: [1; 32],
            projection_sha256: [2; 32],
            policy_sha256: hash(&canonical(&ProjectPolicy::restricted()).unwrap()),
        },
    };
    generation = f
        .service
        .register_project(&f.actors[2], f.request(None, generation), &c)
        .await
        .unwrap()
        .generation;
    let projection = CatalogProjection {
        library: f.library,
        project,
        observation: Uuid::new_v4(),
        locator: Uuid::new_v4(),
        commit: c.disclosure.commit,
        commit_sha256: [1; 32],
        package_revision: 1,
        title: "Reported test projection".into(),
        asset_count: 0,
        graph_count: 0,
    };
    f.service
        .publish_content(
            &f.actors[2],
            f.request(Some(project), generation),
            &PublishContent {
                operation: op(),
                expected_revision: None,
                root: ContentRoot::Catalog(projection.clone()),
            },
        )
        .await
        .unwrap();
    let actions = [
        ProjectAction::Discover,
        ProjectAction::Read,
        ProjectAction::Edit,
        ProjectAction::Run,
        ProjectAction::Invite,
        ProjectAction::ManageStorage,
        ProjectAction::ManageContext,
        ProjectAction::ManageAccess,
    ];
    let mut cells = 0;
    for bits in [0, 1, 3, 71, 11, 79] {
        let mask = ActionMask::new(bits).unwrap();
        let policy = ProjectPolicy::try_from(PolicySpec {
            visibility: VisibilityPolicy::LibraryVisible,
            owner: mask,
            admin: mask,
            editor: mask,
            viewer: mask,
        })
        .unwrap();
        let current:i64=sqlx::query_scalar("SELECT revision FROM photara.project_access_policies WHERE library_id=$1 AND project_id=$2").bind(f.library.uuid()).bind(project.uuid()).fetch_one(f.service.auth.pool()).await.unwrap();
        let command = SetPolicy {
            operation: op(),
            expected_revision: current,
            policy: policy.clone(),
            disclosure: Disclosure {
                actor: f.actors[2].account,
                commit: projection.commit,
                commit_sha256: projection.commit_sha256,
                projection_sha256: hash(&canonical(&projection).unwrap()),
                policy_sha256: hash(&canonical(&policy).unwrap()),
            },
        };
        generation = f
            .service
            .set_policy(&f.actors[2], f.request(Some(project), generation), &command)
            .await
            .unwrap()
            .generation;
        for n in [0, 1, 6, 3] {
            let mut tx = f.service.api.begin().await.unwrap();
            runtime::set_context(&mut tx, &f.actors[n], f.request(Some(project), generation))
                .await
                .unwrap();
            for action in actions {
                let a = serde_json::to_value(action).unwrap();
                let allowed: bool =
                    sqlx::query_scalar("SELECT photara_private.can_project($1,$2,$3)")
                        .bind(f.library.uuid())
                        .bind(project.uuid())
                        .bind(a.as_str().unwrap())
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                assert_eq!(
                    allowed,
                    mask.allows(action),
                    "role {n} mask {bits} action {action:?}"
                );
                cells += 1;
            }
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM photara.package_observations WHERE project_id=$1",
            )
            .bind(project.uuid())
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            assert_eq!(count, i64::from(mask.allows(ProjectAction::Read)));
            tx.rollback().await.unwrap();
        }
    }
    // An active zero override inherits; explicit revocation denies inherited rights.
    let grant = SetGrant {
        operation: op(),
        target: f.actors[1].account,
        grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        mask: ActionMask::NONE,
        change: GrantChange::Create,
        expected_revision: None,
    };
    generation = f
        .service
        .set_grant(&f.actors[2], f.request(Some(project), generation), &grant)
        .await
        .unwrap()
        .generation;
    assert!(
        f.service
            .permits(
                &f.actors[1],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    let revoke = SetGrant {
        operation: op(),
        change: GrantChange::Revoke,
        expected_revision: Some(1),
        ..grant
    };
    let held = f
        .service
        .begin(
            &f.actors[1],
            f.request(Some(project), generation),
            ProjectAction::Read,
            &[],
        )
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(50),
            f.service
                .set_grant(&f.actors[2], f.request(Some(project), generation), &revoke)
        )
        .await
        .is_err()
    );
    held.commit().await.unwrap();
    generation = f
        .service
        .set_grant(&f.actors[2], f.request(Some(project), generation), &revoke)
        .await
        .unwrap()
        .generation;
    assert!(
        !f.service
            .permits(
                &f.actors[1],
                f.request(Some(project), generation),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    // Membership administration cannot self-grant a revoked Project or expose another scope.
    let self_grant = SetGrant {
        operation: op(),
        target: f.actors[0].account,
        grant: ProjectAccessGrantId::from_uuid(Uuid::new_v4()).unwrap(),
        mask: ProjectPreset::Manager.mask(),
        change: GrantChange::Create,
        expected_revision: None,
    };
    assert!(
        f.service
            .set_grant(
                &f.actors[0],
                f.request(Some(project), generation),
                &self_grant
            )
            .await
            .is_err()
    );
    for (n, actor) in f.actors.iter().enumerate() {
        for scope in [None, Some(project)] {
            let mut tx = f.service.api.begin().await.unwrap();
            runtime::set_context(&mut tx, actor, f.request(scope, generation))
                .await
                .unwrap();
            for sensitivity in ["ordinary", "personal", "restricted", "host-only"] {
                let allowed: bool =
                    sqlx::query_scalar("SELECT photara_private.can_read_library($1,$2)")
                        .bind(f.library.uuid())
                        .bind(sensitivity)
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                assert_eq!(
                    allowed,
                    scope.is_none()
                        && [0, 1, 2, 3, 6].contains(&n)
                        && ["ordinary", "personal"].contains(&sensitivity)
                );
                cells += 1;
            }
            tx.rollback().await.unwrap();
        }
    }
    println!(
        "CXT3c: {cells} inherited-policy and sensitivity cells; revocation/read serialization and admin self-grant denial passed"
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_schema_and_runtime_refusal() {
    let fixture = Fixture::new().await;
    let url = std::env::var("PHOTARA_TEST_MIGRATOR_URL").unwrap();
    for floor in [1, 4] {
        let mut tx = fixture.owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE photara.schema_metadata SET minimum_api=$1")
            .bind(floor)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert!(matches!(
            migrate(storexa::DatabaseConfig::from_url(&url).unwrap()).await,
            Err(ServiceError::Unsupported)
        ));
        let mut tx = fixture.owner.begin().await.unwrap();
        sqlx::query("SET LOCAL ROLE photara_owner")
            .execute(&mut *tx)
            .await
            .unwrap();
        let retained: i32 =
            sqlx::query_scalar("SELECT minimum_api FROM photara.schema_metadata WHERE singleton")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert_eq!(retained, floor);
        sqlx::query("UPDATE photara.schema_metadata SET minimum_api=3")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let mut tx = fixture.owner.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    let checksum: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM public._sqlx_migrations WHERE version=1")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    sqlx::query(
        "UPDATE public._sqlx_migrations SET checksum=decode(repeat('00',48),'hex') WHERE version=1",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        migrate(storexa::DatabaseConfig::from_url(&url).unwrap()).await,
        Err(ServiceError::Migration(_))
    ));
    assert!(matches!(
        runtime::validate_schema(&fixture.service.api).await,
        Err(ServiceError::Unsupported)
    ));
    let mut tx = fixture.owner.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE photara_owner")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE public._sqlx_migrations SET checksum=$1 WHERE version=1")
        .bind(checksum)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    migrate(storexa::DatabaseConfig::from_url(&url).unwrap())
        .await
        .unwrap();
    let configs = [
        "photara_test_migrator",
        "photara_test_control",
        "photara_test_auth",
    ]
    .map(|role| {
        storexa::DatabaseConfig::from_url(url.replace("photara_test_migrator", role)).unwrap()
    });
    assert!(matches!(
        Service::connect(
            configs,
            fixture.auth.clone(),
            fixture.clock.clone(),
            "https://fake.invalid/".into(),
            "photara-test".into(),
            [3; 32]
        )
        .await,
        Err(ServiceError::Forbidden)
    ));
    let mut tx = fixture
        .service
        .context(
            &fixture.actors[0],
            fixture.request(None, 1),
            &[fixture.actors[4].account],
        )
        .await
        .unwrap();
    sqlx::query("UPDATE photara_identity.accounts SET state='disabled',retired_at=CURRENT_TIMESTAMP,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1").bind(fixture.actors[4].account.uuid()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(
        !fixture
            .service
            .permits(
                &fixture.actors[4],
                fixture.request(None, 1),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    let mut tx = fixture
        .service
        .context(&fixture.actors[0], fixture.request(None, 1), &[])
        .await
        .unwrap();
    sqlx::query(
        "UPDATE photara_identity.devices SET state='revoked' WHERE account_id=$1 AND device_id=$2",
    )
    .bind(fixture.actors[1].account.uuid())
    .bind(fixture.device.uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(
        !fixture
            .service
            .permits(
                &fixture.actors[1],
                fixture.request(None, 1),
                ProjectAction::Read
            )
            .await
            .unwrap()
    );
    println!(
        "CXT3c: migration checksum/floor rollback, migration-owner runtime refusal, disabled Account and revoked device passed"
    );
    fixture.finish().await;
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL runner"]
async fn postgres_fake_sync_transport() {
    let fixture = Fixture::new().await;
    let actor = &fixture.actors[0];
    let request = fixture.request(None, 1);
    let mut client = FakeSync::new(actor, request);
    client.set_v1_unsettled(true);
    assert!(
        client
            .bootstrap(&fixture.service, actor, request)
            .await
            .is_err()
    );
    client.set_v1_unsettled(false);
    client
        .bootstrap(&fixture.service, actor, request)
        .await
        .unwrap();
    let root = StorageProjection {
        library: fixture.library,
        root: StorageLocationId::from_uuid(Uuid::new_v4()).unwrap(),
        revision: 1,
        display_name: "Transport root".into(),
        purpose: "source".into(),
        retired: false,
    };
    let content = PublishContent {
        operation: op(),
        expected_revision: None,
        root: ContentRoot::Storage(root.clone()),
    };
    client.seal(content.clone()).unwrap();
    client
        .dispatch(&fixture.service, actor, content.operation, true)
        .await
        .unwrap();
    assert_eq!(client.state(content.operation), Some(IntentState::Unknown));
    assert!(client.roots().is_empty());
    let changed = PublishContent {
        root: ContentRoot::Storage(StorageProjection {
            display_name: "collision".into(),
            ..root.clone()
        }),
        ..content.clone()
    };
    assert!(client.seal(changed).is_err());
    client
        .dispatch(&fixture.service, actor, content.operation, false)
        .await
        .unwrap();
    assert_eq!(
        client.state(content.operation),
        Some(IntentState::Acknowledged)
    );
    client.pull(&fixture.service, actor).await.unwrap();
    assert_eq!(client.state(content.operation), Some(IntentState::Applied));
    assert_eq!(client.roots(), vec![content.root]);
    let pending = PublishContent {
        operation: op(),
        expected_revision: Some(1),
        root: ContentRoot::Storage(StorageProjection {
            revision: 2,
            display_name: "Unpublished overlay".into(),
            ..root.clone()
        }),
    };
    client.seal(pending.clone()).unwrap();
    let command = SetMembership {
        operation: op(),
        target: fixture.actors[5].account,
        role: LibraryRole::Viewer,
        revoked: false,
        expected_revision: None,
    };
    let generation = fixture
        .service
        .set_membership(actor, request, &command)
        .await
        .unwrap()
        .generation;
    assert!(client.pull(&fixture.service, actor).await.is_err());
    assert!(client.access_lost());
    assert_eq!(client.roots().len(), 1);
    assert_eq!(client.state(pending.operation), Some(IntentState::Sealed));
    let current = fixture.request(None, generation);
    client
        .bootstrap(&fixture.service, actor, current)
        .await
        .unwrap();
    assert!(!client.access_lost());
    assert_eq!(client.state(pending.operation), Some(IntentState::Sealed));
    let competing = PublishContent {
        operation: op(),
        expected_revision: Some(1),
        root: ContentRoot::Storage(StorageProjection {
            revision: 2,
            display_name: "Remote edit".into(),
            ..root
        }),
    };
    fixture
        .service
        .publish_content(actor, current, &competing)
        .await
        .unwrap();
    assert!(
        client
            .dispatch(&fixture.service, actor, pending.operation, false)
            .await
            .is_err()
    );
    assert_eq!(client.state(pending.operation), Some(IntentState::Conflict));
    client.pull(&fixture.service, actor).await.unwrap();
    assert_eq!(client.roots(), vec![competing.root]);
    println!(
        "CXT3c: fake transport activation, sealed collision, lost-reply reconciliation, whole-batch apply, access-loss retention, rebootstrap and overlay conflict passed"
    );
    fixture.finish().await;
}
