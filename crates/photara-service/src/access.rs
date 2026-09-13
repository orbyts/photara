//! Typed access commands. Every control mutation retains one receipt and audit.
use crate::runtime::{authorize, bump, check_generation, record, retry};
use crate::{
    Actor, ControlReceipt, Request, Result, Scope, Service, ServiceError, canonical, hash,
};
use photara_core::contracts::access::{
    ActionMask, LibraryRole, ProjectAction, ProjectPolicy, VisibilityPolicy,
};
use photara_core::contracts::{
    AccountId, CommitId, InvitationId, OperationId, ProjectAccessGrantId, ProjectId,
};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use storexa::Transaction;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disclosure {
    pub actor: AccountId,
    pub commit: CommitId,
    pub commit_sha256: [u8; 32],
    pub projection_sha256: [u8; 32],
    pub policy_sha256: [u8; 32],
}
impl Disclosure {
    fn validate(&self, actor: &Actor, policy: &ProjectPolicy) -> Result<()> {
        if self.actor != actor.account
            || self.commit_sha256 == [0; 32]
            || self.projection_sha256 == [0; 32]
            || self.policy_sha256 != hash(&canonical(policy)?)
        {
            return Err(ServiceError::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterProject {
    pub operation: OperationId,
    pub project: ProjectId,
    pub disclosure: Disclosure,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMembership {
    pub operation: OperationId,
    pub target: AccountId,
    pub role: LibraryRole,
    pub revoked: bool,
    pub expected_revision: Option<i64>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GrantChange {
    Create,
    Replace,
    Revoke,
    Regrant,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetGrant {
    pub operation: OperationId,
    pub target: AccountId,
    pub grant: ProjectAccessGrantId,
    pub mask: ActionMask,
    pub change: GrantChange,
    pub expected_revision: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetPolicy {
    pub operation: OperationId,
    pub expected_revision: i64,
    pub policy: ProjectPolicy,
    pub disclosure: Disclosure,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invite {
    pub operation: OperationId,
    pub invitation: InvitationId,
    pub target: AccountId,
    pub mask: ActionMask,
    pub expected_policy_revision: i64,
    pub expires_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptInvitation {
    pub operation: OperationId,
    pub invitation: InvitationId,
    pub grant: ProjectAccessGrantId,
    pub token: [u8; 32],
}
impl Service {
    /// # Errors
    /// Rejects missing Library edit access, collisions or invalid reported association evidence.
    pub async fn register_project(
        &self,
        actor: &Actor,
        r: Request,
        c: &RegisterProject,
    ) -> Result<ControlReceipt> {
        if r.scope.project().is_some() {
            return Err(ServiceError::Invalid);
        }
        c.disclosure.validate(actor, &ProjectPolicy::restricted())?;
        let mut tx = self.begin(actor, r, ProjectAction::Edit, &[]).await?;
        let body = (r.scope, "register-project", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        sqlx::query("INSERT INTO photara.project_ownership(project_id,library_id,registration_state,association_commit_id,association_commit_sha256,source_format,record_schema,revision,created_at,updated_at) VALUES($1,$2,'active',$3,$4,'photara.project.g2',1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(c.project.uuid()).bind(r.scope.library().uuid()).bind(c.disclosure.commit.uuid()).bind(c.disclosure.commit_sha256.to_vec()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara.project_access_policies VALUES($1,$2,'restricted',0,0,0,0,1,1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).bind(c.project.uuid()).execute(&mut *tx).await?;
        insert_grant(
            &mut tx,
            r.scope.library().uuid(),
            c.project,
            actor.account,
            Uuid::new_v4(),
            ActionMask::new(255).map_err(|_| ServiceError::Invalid)?,
        )
        .await?;
        sqlx::query("INSERT INTO photara.project_catalog VALUES($1,$2,'visible',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).bind(c.project.uuid()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.scoped_streams VALUES($1,$2,'project',$3,$4,0,CURRENT_TIMESTAMP)").bind(Uuid::new_v4()).bind(r.scope.library().uuid()).bind(c.project.uuid()).bind(Uuid::new_v4()).execute(&mut *tx).await?;
        let generation = bump(
            &mut tx,
            Scope::Project {
                library: r.scope.library(),
                project: c.project,
            },
        )
        .await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: c.project.uuid(),
        };
        record(&mut tx, actor, r, &body, "register-project", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// # Errors
    /// Requires an authorized role ceiling, existing active target, CAS and retained Library owner.
    pub async fn set_membership(
        &self,
        actor: &Actor,
        r: Request,
        c: &SetMembership,
    ) -> Result<ControlReceipt> {
        if r.scope.project().is_some() {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self
            .begin(actor, r, ProjectAction::ManageAccess, &[c.target])
            .await?;
        let body = (r.scope, "membership", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        let role:String=sqlx::query_scalar("SELECT role FROM photara_identity.memberships WHERE library_id=$1 AND account_id=$2 AND state='active'").bind(r.scope.library().uuid()).bind(actor.account.uuid()).fetch_one(&mut *tx).await?;
        let target=sqlx::query("SELECT role,revision FROM photara_identity.memberships WHERE library_id=$1 AND account_id=$2 FOR UPDATE").bind(r.scope.library().uuid()).bind(c.target.uuid()).fetch_optional(&mut *tx).await?;
        if role != "owner"
            && (c.role == LibraryRole::Owner
                || target
                    .as_ref()
                    .is_some_and(|t| t.get::<String, _>("role") == "owner"))
        {
            return Err(ServiceError::Forbidden);
        }
        let role = serde_json::to_value(c.role).map_err(|_| ServiceError::Invalid)?;
        let role = role.as_str().ok_or(ServiceError::Invalid)?;
        if let Some(t) = target {
            if Some(t.try_get::<i64, _>("revision")?) != c.expected_revision {
                return Err(ServiceError::Conflict);
            }
            let affected=sqlx::query("UPDATE photara_identity.memberships SET role=$3,state=$4,revoked_at=CASE WHEN $5 THEN CURRENT_TIMESTAMP ELSE NULL END,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND account_id=$2 AND revision=$6").bind(r.scope.library().uuid()).bind(c.target.uuid()).bind(role).bind(if c.revoked{"revoked"}else{"active"}).bind(c.revoked).bind(c.expected_revision).execute(&mut *tx).await?.rows_affected();
            if affected != 1 {
                return Err(ServiceError::Conflict);
            }
        } else {
            if c.expected_revision.is_some() || c.revoked {
                return Err(ServiceError::Conflict);
            }
            sqlx::query("INSERT INTO photara_identity.memberships VALUES($1,$2,$3,$4,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(Uuid::new_v4()).bind(r.scope.library().uuid()).bind(c.target.uuid()).bind(role).execute(&mut *tx).await?;
        }
        let generation = bump(&mut tx, r.scope).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: c.target.uuid(),
        };
        record(&mut tx, actor, r, &body, "membership", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// # Errors
    /// Requires Project manage-access, action containment, explicit regrant and expected revision.
    pub async fn set_grant(
        &self,
        actor: &Actor,
        r: Request,
        c: &SetGrant,
    ) -> Result<ControlReceipt> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        let mut tx = self
            .begin(actor, r, ProjectAction::ManageAccess, &[c.target])
            .await?;
        let body = (r.scope, "grant", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        contains(&mut tx, r.scope, c.mask).await?;
        let target=sqlx::query("SELECT grant_id,revision,state FROM photara_identity.project_access_grants WHERE library_id=$1 AND project_id=$2 AND account_id=$3 FOR UPDATE").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.target.uuid()).fetch_optional(&mut *tx).await?;
        if let Some(t) = target {
            if t.try_get::<Uuid, _>("grant_id")? != c.grant.uuid()
                || Some(t.try_get::<i64, _>("revision")?) != c.expected_revision
                || c.change == GrantChange::Create
            {
                return Err(ServiceError::Conflict);
            }
            let revoked = t.try_get::<String, _>("state")? == "revoked";
            if revoked != (c.change == GrantChange::Regrant) {
                return Err(ServiceError::Conflict);
            }
            sqlx::query("UPDATE photara_identity.project_access_grants SET action_mask=$4,state=$5,revoked_at=CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE NULL END,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND project_id=$2 AND grant_id=$3 AND revision=$7").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.grant.uuid()).bind(i32::from(c.mask.bits())).bind(if c.change==GrantChange::Revoke{"revoked"}else{"active"}).bind(c.change==GrantChange::Revoke).bind(c.expected_revision).execute(&mut *tx).await?;
        } else {
            if c.change != GrantChange::Create || c.expected_revision.is_some() {
                return Err(ServiceError::Conflict);
            }
            insert_grant(
                &mut tx,
                r.scope.library().uuid(),
                project,
                c.target,
                c.grant.uuid(),
                c.mask,
            )
            .await?;
        }
        let generation = bump(&mut tx, r.scope).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: c.grant.uuid(),
        };
        record(&mut tx, actor, r, &body, "grant", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// # Errors
    /// Requires Project manage-access, disclosure evidence and policy CAS.
    pub async fn set_policy(
        &self,
        actor: &Actor,
        r: Request,
        c: &SetPolicy,
    ) -> Result<ControlReceipt> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        c.disclosure.validate(actor, &c.policy)?;
        let mut tx = self
            .begin(actor, r, ProjectAction::ManageAccess, &[])
            .await?;
        let body = (r.scope, "policy", c);
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok(v);
        }
        for mask in [
            c.policy.spec().owner,
            c.policy.spec().admin,
            c.policy.spec().editor,
            c.policy.spec().viewer,
        ] {
            contains(&mut tx, r.scope, mask).await?;
        }
        let reported:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photara.package_observations WHERE library_id=$1 AND project_id=$2 AND commit_id=$3 AND commit_sha256=$4 AND projection_sha256=$5)").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.disclosure.commit.uuid()).bind(c.disclosure.commit_sha256.to_vec()).bind(c.disclosure.projection_sha256.to_vec()).fetch_one(&mut *tx).await?;
        if !reported {
            return Err(ServiceError::Forbidden);
        }
        let p = c.policy.spec();
        let n=sqlx::query("UPDATE photara.project_access_policies SET visibility_policy=$3,owner_mask=$4,admin_mask=$5,editor_mask=$6,viewer_mask=$7,authorization_generation=authorization_generation+1,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 AND project_id=$2 AND revision=$8").bind(r.scope.library().uuid()).bind(project.uuid()).bind(if p.visibility==VisibilityPolicy::Restricted{"restricted"}else{"library-visible"}).bind(i32::from(p.owner.bits())).bind(i32::from(p.admin.bits())).bind(i32::from(p.editor.bits())).bind(i32::from(p.viewer.bits())).bind(c.expected_revision).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(ServiceError::Conflict);
        }
        let generation=sqlx::query_scalar("UPDATE photara.library_contract_state SET authorization_generation=authorization_generation+1,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE library_id=$1 RETURNING authorization_generation").bind(r.scope.library().uuid()).fetch_one(&mut *tx).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: project.uuid(),
        };
        record(&mut tx, actor, r, &body, "policy", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// Produces a token for the caller to deliver; this method sends no message.
    /// # Errors
    /// Rejects invalid expiry, target, containment or policy CAS.
    pub async fn invite(
        &self,
        actor: &Actor,
        r: Request,
        c: &Invite,
    ) -> Result<(ControlReceipt, [u8; 32])> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        let now = self.clock.now_ms();
        if c.expires_ms <= now || c.expires_ms > now.saturating_add(604_800_000) {
            return Err(ServiceError::Invalid);
        }
        let mut tx = self
            .begin(actor, r, ProjectAction::Invite, &[c.target])
            .await?;
        let body = (r.scope, "invite", c);
        let token = self.sign("invitation", &canonical(&(actor.account, r.scope, c))?)?;
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            tx.commit().await?;
            return Ok((v, token));
        }
        contains(&mut tx, r.scope, c.mask).await?;
        let rev:i64=sqlx::query_scalar("SELECT revision FROM photara.project_access_policies WHERE library_id=$1 AND project_id=$2").bind(r.scope.library().uuid()).bind(project.uuid()).fetch_one(&mut *tx).await?;
        if rev != c.expected_policy_revision {
            return Err(ServiceError::Conflict);
        }
        sqlx::query("UPDATE photara_identity.project_invitations SET state='expired',completed_at=TIMESTAMPTZ 'epoch'+$4::bigint*INTERVAL '1 millisecond',revision=revision+1,updated_at=TIMESTAMPTZ 'epoch'+$4::bigint*INTERVAL '1 millisecond' WHERE library_id=$1 AND project_id=$2 AND target_account_id=$3 AND state='pending' AND expires_at<=TIMESTAMPTZ 'epoch'+$4::bigint*INTERVAL '1 millisecond'").bind(r.scope.library().uuid()).bind(project.uuid()).bind(c.target.uuid()).bind(now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_identity.project_invitations VALUES($1,$2,$3,$4,$5,$6,$7,TIMESTAMPTZ 'epoch'+$8::bigint*INTERVAL '1 millisecond','pending',NULL,NULL,1,1,TIMESTAMPTZ 'epoch'+$9::bigint*INTERVAL '1 millisecond',TIMESTAMPTZ 'epoch'+$9::bigint*INTERVAL '1 millisecond')").bind(c.invitation.uuid()).bind(r.scope.library().uuid()).bind(project.uuid()).bind(actor.account.uuid()).bind(c.target.uuid()).bind(i32::from(c.mask.bits())).bind(c.expected_policy_revision).bind(c.expires_ms).bind(now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.project_invitation_secrets VALUES($1,$2,1,NULL)")
            .bind(c.invitation.uuid())
            .bind(hash(&token).to_vec())
            .execute(&mut *tx)
            .await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation: r.generation,
            target: c.invitation.uuid(),
        };
        record(&mut tx, actor, r, &body, "invite", &receipt).await?;
        tx.commit().await?;
        Ok((receipt, token))
    }
    /// # Errors
    /// Requires exact target/token, current inviter rights, unexpired pending invitation and current policy.
    pub async fn accept_invitation(
        &self,
        actor: &Actor,
        r: Request,
        c: &AcceptInvitation,
    ) -> Result<ControlReceipt> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        // Read only bounded access facts to determine the complete lock set.
        let inviter:Uuid=sqlx::query_scalar("SELECT inviter_account_id FROM photara_identity.project_invitations WHERE invitation_id=$1 AND library_id=$2 AND project_id=$3 AND target_account_id=$4").bind(c.invitation.uuid()).bind(r.scope.library().uuid()).bind(project.uuid()).bind(actor.account.uuid()).fetch_optional(self.auth.pool()).await?.ok_or(ServiceError::Forbidden)?;
        let inviter = AccountId::from_uuid(inviter).map_err(|_| ServiceError::Integrity)?;
        let mut tx = self.context(actor, r, &[inviter]).await?;
        sqlx::query("SELECT library_id FROM photara.libraries WHERE library_id=$1 FOR UPDATE")
            .bind(r.scope.library().uuid())
            .fetch_one(&mut *tx)
            .await?;
        check_generation(&mut tx, r).await?;
        let body = (
            r.scope,
            "accept-invitation",
            c.operation,
            c.invitation,
            c.grant,
            hash(&c.token),
        );
        if let Some(v) = retry(&mut tx, actor, r, c.operation, &body).await? {
            authorize(&mut tx, r.scope, ProjectAction::Read).await?;
            tx.commit().await?;
            return Ok(v);
        }
        let row=sqlx::query("SELECT i.action_mask,i.expected_policy_revision,i.state,(extract(epoch FROM i.expires_at)*1000)::bigint AS expires_ms,s.token_verifier,s.consumed_at IS NULL AS unused FROM photara_identity.project_invitations i JOIN photara_private.project_invitation_secrets s USING(invitation_id) WHERE invitation_id=$1 AND target_account_id=$2 AND library_id=$3 AND project_id=$4 FOR UPDATE OF i,s").bind(c.invitation.uuid()).bind(actor.account.uuid()).bind(r.scope.library().uuid()).bind(project.uuid()).fetch_optional(&mut *tx).await?.ok_or(ServiceError::Forbidden)?;
        if row.try_get::<String, _>("state")? != "pending"
            || !row.try_get::<bool, _>("unused")?
            || row.try_get::<i64, _>("expires_ms")? <= self.clock.now_ms()
            || !crate::secret_eq(
                &row.try_get::<Vec<u8>, _>("token_verifier")?,
                &hash(&c.token),
            )
        {
            return Err(ServiceError::Forbidden);
        }
        let rev:i64=sqlx::query_scalar("SELECT revision FROM photara.project_access_policies WHERE library_id=$1 AND project_id=$2 FOR UPDATE").bind(r.scope.library().uuid()).bind(project.uuid()).fetch_one(&mut *tx).await?;
        if rev != row.try_get::<i64, _>("expected_policy_revision")? {
            return Err(ServiceError::Conflict);
        }
        let identity:Uuid=sqlx::query_scalar("SELECT identity_id FROM photara_identity.account_identities WHERE account_id=$1 AND state='active' ORDER BY identity_id LIMIT 1").bind(inviter.uuid()).fetch_one(&mut *tx).await?;
        let proxy = Actor {
            account: inviter,
            identity,
            expires_ms: actor.expires_ms,
        };
        crate::runtime::set_context(&mut tx, &proxy, r).await?;
        authorize(&mut tx, r.scope, ProjectAction::Invite).await?;
        let mask = ActionMask::new(
            u16::try_from(row.try_get::<i32, _>("action_mask")?)
                .map_err(|_| ServiceError::Integrity)?,
        )
        .map_err(|_| ServiceError::Integrity)?;
        contains(&mut tx, r.scope, mask).await?;
        crate::runtime::set_context(&mut tx, actor, r).await?;
        // Existing/revoked rows require the separate explicit regrant command.
        insert_grant(
            &mut tx,
            r.scope.library().uuid(),
            project,
            actor.account,
            c.grant.uuid(),
            mask,
        )
        .await?;
        let now = self.clock.now_ms();
        sqlx::query("UPDATE photara_identity.project_invitations SET state='accepted',accepted_grant_id=$2,completed_at=TIMESTAMPTZ 'epoch'+$3::bigint*INTERVAL '1 millisecond',updated_at=TIMESTAMPTZ 'epoch'+$3::bigint*INTERVAL '1 millisecond',revision=revision+1 WHERE invitation_id=$1 AND state='pending'").bind(c.invitation.uuid()).bind(c.grant.uuid()).bind(now).execute(&mut *tx).await?;
        sqlx::query("UPDATE photara_private.project_invitation_secrets SET consumed_at=TIMESTAMPTZ 'epoch'+$2::bigint*INTERVAL '1 millisecond' WHERE invitation_id=$1 AND consumed_at IS NULL").bind(c.invitation.uuid()).bind(now).execute(&mut *tx).await?;
        let generation = bump(&mut tx, r.scope).await?;
        let receipt = ControlReceipt {
            operation: c.operation,
            generation,
            target: c.grant.uuid(),
        };
        record(&mut tx, actor, r, &body, "accept-invitation", &receipt).await?;
        tx.commit().await?;
        Ok(receipt)
    }
}
pub(crate) async fn insert_grant(
    tx: &mut Transaction,
    library: Uuid,
    project: ProjectId,
    target: AccountId,
    grant: Uuid,
    mask: ActionMask,
) -> Result<()> {
    sqlx::query("INSERT INTO photara_identity.project_access_grants(grant_id,library_id,project_id,account_id,action_mask,state,record_schema,revision,created_at,updated_at) VALUES($1,$2,$3,$4,$5,'active',1,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").bind(grant).bind(library).bind(project.uuid()).bind(target.uuid()).bind(i32::from(mask.bits())).execute(&mut **tx).await?;
    Ok(())
}
async fn contains(tx: &mut Transaction, scope: Scope, mask: ActionMask) -> Result<()> {
    for action in [
        ProjectAction::Discover,
        ProjectAction::Read,
        ProjectAction::Edit,
        ProjectAction::Run,
        ProjectAction::Invite,
        ProjectAction::ManageStorage,
        ProjectAction::ManageContext,
        ProjectAction::ManageAccess,
    ] {
        if mask.allows(action) {
            authorize(tx, scope, action).await?;
        }
    }
    Ok(())
}

/// Minimal discovery DTO: intentionally contains no title, counts, locators or media.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDiscovery {
    pub project: ProjectId,
    pub library: photara_core::contracts::LibraryId,
    pub library_name: String,
    pub request_access_available: bool,
}
impl Service {
    /// # Errors
    /// Requires discover on the exact Project under current identity and access generation.
    pub async fn discover_project(&self, actor: &Actor, r: Request) -> Result<ProjectDiscovery> {
        let project = r.scope.project().ok_or(ServiceError::Invalid)?;
        let mut tx = self.begin(actor, r, ProjectAction::Discover, &[]).await?;
        let library_name: String =
            sqlx::query_scalar("SELECT display_name FROM photara.libraries WHERE library_id=$1")
                .bind(r.scope.library().uuid())
                .fetch_one(&mut *tx)
                .await?;
        let read: bool = sqlx::query_scalar("SELECT photara_private.can_project($1,$2,'read')")
            .bind(r.scope.library().uuid())
            .bind(project.uuid())
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(ProjectDiscovery {
            project,
            library: r.scope.library(),
            library_name,
            request_access_available: !read,
        })
    }
}
