//! Atomic cloud projection of UI1's verified initial package coordinates.
use crate::*;
use photara_core::contracts::{
    DeviceId, OperationId,
    access::{ProjectAction, ProjectPolicy},
};
use photara_library::gen2::{CreateProjectCloudReceipt, CreateProjectCommand};
use uuid::Uuid;

fn child(operation: OperationId, domain: &str) -> Result<OperationId> {
    let digest = hash(&canonical(&(operation, domain))?);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    OperationId::from_uuid(Uuid::from_bytes(bytes)).map_err(|_| ServiceError::Invalid)
}
impl Service {
    /// Identity/device authentication, current authority, registration, restricted
    /// manager, catalog projection, feed batch and exact receipt share one transaction.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep the atomic creation transaction and exact receipt reconciliation in one auditable sequence"
    )]
    pub(crate) async fn create_project(
        &self,
        claims: &auth::VerifiedClaims,
        credential: &[u8],
        command: &CreateProjectCommand,
    ) -> Result<CreateProjectCloudReceipt> {
        if credential.len() != 32 || command.locator_id.is_nil() || command.observation_id.is_nil()
        {
            return Err(ServiceError::Invalid);
        }
        let initial = &command.initial;
        let package = photara_library::gen2::initial_creation_package(initial.clone())
            .map_err(|_| ServiceError::Invalid)?;
        let request_bytes = canonical(command)?;
        let mut tx = self.principal_transaction(claims).await?;
        let actor = onboarding::identity(&mut tx, claims)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let device = DeviceId::from_uuid(command.device_id).map_err(|_| ServiceError::Invalid)?;
        onboarding::device(&mut tx, &actor, device, &hash(credential), false)
            .await?
            .ok_or(ServiceError::Forbidden)?;
        let mut r = Request {
            scope: Scope::Library {
                library: initial.library_id,
            },
            device,
            generation: 1,
        };
        runtime::set_context(&mut tx, &actor, r).await?;
        runtime::authorize(&mut tx, r.scope, ProjectAction::Edit).await?;
        r.generation=sqlx::query_scalar("SELECT authorization_generation FROM photara.library_contract_state WHERE library_id=$1 FOR UPDATE").bind(initial.library_id.uuid()).fetch_one(&mut *tx).await?;
        runtime::set_context(&mut tx, &actor, r).await?;
        if let Some(row)=sqlx::query("SELECT * FROM photara_private.scoped_command_receipts WHERE library_id=$1 AND operation_id=$2").bind(initial.library_id.uuid()).bind(initial.operation_id.uuid()).fetch_optional(&mut *tx).await? {
            if row.try_get::<Uuid,_>("actor_account_id")?!=actor.account.uuid() || row.try_get::<Uuid,_>("device_id")?!=device.uuid() || row.try_get::<Vec<u8>,_>("request_canonical")?!=request_bytes || row.try_get::<Vec<u8>,_>("request_sha256")?!=hash(&request_bytes){return Err(ServiceError::Conflict);}
            runtime::authorize(&mut tx,Scope::Project{library:initial.library_id,project:initial.project_id},ProjectAction::Read).await?;
            let bytes:Vec<u8>=row.try_get("response_canonical")?;
            let receipt:CreateProjectCloudReceipt=serde_json::from_slice(&bytes).map_err(|_|ServiceError::Integrity)?;
            if canonical(&receipt)?!=bytes || hash(&bytes).as_slice()!=row.try_get::<Vec<u8>,_>("response_sha256")? {return Err(ServiceError::Integrity);}
            tx.commit().await?;return Ok(receipt);
        }
        let projection = CatalogProjection {
            library: initial.library_id,
            project: initial.project_id,
            observation: command.observation_id,
            locator: command.locator_id,
            commit: initial.commit_id,
            commit_sha256: package.commit_sha256,
            package_revision: 1,
            title: initial.title.clone(),
            asset_count: 0,
            graph_count: 1,
        };
        let registered = self
            .register_project_in(
                &mut tx,
                &actor,
                r,
                &RegisterProject {
                    operation: child(initial.operation_id, "register")?,
                    project: initial.project_id,
                    disclosure: Disclosure {
                        actor: actor.account,
                        commit: initial.commit_id,
                        commit_sha256: package.commit_sha256,
                        projection_sha256: hash(&canonical(&projection)?),
                        policy_sha256: hash(&canonical(&ProjectPolicy::restricted())?),
                    },
                },
            )
            .await?;
        let project_request = Request {
            scope: Scope::Project {
                library: initial.library_id,
                project: initial.project_id,
            },
            device,
            generation: registered.generation,
        };
        runtime::set_context(&mut tx, &actor, project_request).await?;
        runtime::authorize(&mut tx, project_request.scope, ProjectAction::Edit).await?;
        self.publish_content_in(
            &mut tx,
            &actor,
            project_request,
            &PublishContent {
                operation: child(initial.operation_id, "publish")?,
                expected_revision: None,
                root: ContentRoot::Catalog(projection),
            },
        )
        .await?;
        let account = sqlx::query(
            "SELECT display_name,revision FROM photara_identity.accounts WHERE account_id=$1",
        )
        .bind(actor.account.uuid())
        .fetch_one(&mut *tx)
        .await?;
        let grant:Uuid=sqlx::query_scalar("SELECT grant_id FROM photara_identity.project_access_grants WHERE library_id=$1 AND project_id=$2 AND account_id=$3 AND state='active'").bind(initial.library_id.uuid()).bind(initial.project_id.uuid()).bind(actor.account.uuid()).fetch_one(&mut *tx).await?;
        let receipt = CreateProjectCloudReceipt {
            command_sha256: format!("{:x}", Sha256::digest(&request_bytes)),
            operation_id: initial.operation_id.uuid(),
            account_id: actor.account.uuid(),
            account_name: account.try_get("display_name")?,
            account_revision: account.try_get("revision")?,
            grant_id: grant,
            generation: registered.generation,
            commit_sha256: package.commit_hex,
        };
        let response = canonical(&receipt)?;
        sqlx::query("INSERT INTO photara_private.scoped_command_receipts(library_id,operation_id,actor_account_id,device_id,scope_kind,project_id,command_kind,request_canonical,request_sha256,outcome,response_canonical,response_sha256,completed_at) VALUES($1,$2,$3,$4,'control',$5,'create-project',$6,$7,'control-accepted',$8,$9,CURRENT_TIMESTAMP)").bind(initial.library_id.uuid()).bind(initial.operation_id.uuid()).bind(actor.account.uuid()).bind(device.uuid()).bind(initial.project_id.uuid()).bind(&request_bytes).bind(hash(&request_bytes).to_vec()).bind(&response).bind(hash(&response).to_vec()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(receipt)
    }
}
