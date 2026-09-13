use crate::runtime::{authorize, bump, record, retry};
use crate::{Actor, ControlReceipt, Request, Result, Service, ServiceError, canonical, hash};
use photara_core::contracts::access::{ActionMask, ProjectAction};
use photara_core::contracts::{AccountId, OperationId, ProjectAccessGrantId};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimLibrary {
    pub operation: OperationId,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferManager {
    pub operation: OperationId,
    pub target: AccountId,
    pub new_grant: ProjectAccessGrantId,
    pub old_grant: ProjectAccessGrantId,
    pub expected_old_revision: i64,
}
impl Service {
    /// Explicitly claims the request's Library UUID. A collision never adopts existing content.
    /// # Errors
    /// Rejects collisions, invalid actor/device and stale generations on receipt retry.
    pub async fn claim_library(
        &self,
        actor: &Actor,
        r: Request,
        c: &ClaimLibrary,
    ) -> Result<ControlReceipt> {
        if r.scope.project().is_some()
            || c.display_name.trim().is_empty()
            || c.display_name.len() > 512
        {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self.context(actor, r, &[]).await?;
        let body = (r.scope, "claim-library", c);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM photara.libraries WHERE library_id=$1)",
        )
        .bind(r.scope.library().uuid())
        .fetch_one(&mut *tx)
        .await?;
        if exists {
            authorize(&mut tx, r.scope, ProjectAction::ManageAccess).await?;
            crate::runtime::check_generation(&mut tx, r).await?;
            if let Some(receipt) = retry(&mut tx, actor, r, c.operation, &body).await? {
                tx.commit().await?;
                return Ok(receipt);
            }
            return Err(ServiceError::Conflict);
        }
        if r.generation != 1 {
            return Err(ServiceError::Invalid);
        }
        sqlx::query("INSERT INTO photara.libraries VALUES($1,$2,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL,1,'{}')").bind(r.scope.library().uuid()).bind(&c.display_name).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara.library_contract_state VALUES($1,1,'cloud-member',NULL,1,1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_identity.memberships VALUES($1,$2,$3,'owner','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(Uuid::new_v4()).bind(r.scope.library().uuid()).bind(actor.account.uuid()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.scoped_streams VALUES($1,$2,'library',NULL,$3,0,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(r.scope.library().uuid()).bind(Uuid::new_v4()).execute(&mut *tx).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation: 1,
            target: r.scope.library().uuid(),
        };
        let request = canonical(&body)?;
        let response = canonical(&receipt)?;
        sqlx::query("INSERT INTO photara_private.library_claim_receipts VALUES($1,$2,$3,$3,$4,$5,'claimed',$6,$7,CURRENT_TIMESTAMP)").bind(actor.account.uuid()).bind(c.operation.uuid()).bind(r.scope.library().uuid()).bind(&request).bind(hash(&request).to_vec()).bind(&response).bind(hash(&response).to_vec()).execute(&mut *tx).await?;
        record(&mut tx, actor, r, &body, "claim-library", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// Atomic transfer permits a temporary no-manager state within the transaction.
    /// # Errors
    /// Requires an explicit current manager, active target and source-grant CAS.
    pub async fn transfer_manager(
        &self,
        actor: &Actor,
        r: Request,
        c: &TransferManager,
    ) -> Result<ControlReceipt> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        if actor.account == c.target {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self
            .begin(actor, r, ProjectAction::ManageAccess, &[c.target])
            .await?;
        let body = (r.scope, "transfer-manager", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        let old=sqlx::query("SELECT action_mask,state,revision FROM photara_identity.project_access_grants WHERE library_id=$1 AND project_id=$2 AND grant_id=$3 AND account_id=$4 FOR UPDATE").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.old_grant.uuid()).bind(actor.account.uuid()).fetch_optional(&mut *tx).await?.ok_or(ServiceError::Forbidden)?;
        if old.try_get::<i32, _>("action_mask")? != 255
            || old.try_get::<String, _>("state")? != "active"
            || old.try_get::<i64, _>("revision")? != c.expected_old_revision
        {
            return Err(ServiceError::Conflict);
        }
        sqlx::query("UPDATE photara_identity.project_access_grants SET state='revoked',revoked_at=CURRENT_TIMESTAMP,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE grant_id=$1 AND revision=$2").bind(c.old_grant.uuid()).bind(c.expected_old_revision).execute(&mut *tx).await?;
        crate::access::insert_grant(
            &mut tx,
            r.scope.library().uuid(),
            project,
            c.target,
            c.new_grant.uuid(),
            ActionMask::new(255).map_err(|_| ServiceError::Invalid)?,
        )
        .await?;
        let generation = bump(&mut tx, r.scope).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: c.new_grant.uuid(),
        };
        record(&mut tx, actor, r, &body, "transfer-manager", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
}
