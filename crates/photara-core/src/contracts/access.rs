//! Pure R2 authorization arithmetic over explicitly supplied current facts.
//! No clock, account discovery, grant issuance, cache lookup or persistence.
use super::schema::LocalRevision;
use super::{
    AccountId, ContractError, LibraryId, LocalPrincipalId, ProjectAccessGrantId, ProjectId, Result,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u16)]
pub enum ProjectAction {
    Discover = 1,
    Read = 2,
    Edit = 4,
    Run = 8,
    Invite = 16,
    ManageStorage = 32,
    ManageContext = 64,
    ManageAccess = 128,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct ActionMask(u16);
impl ActionMask {
    pub const NONE: Self = Self(0);
    /// # Errors
    /// Rejects unknown bits or missing discover/read/invite prerequisites.
    pub const fn new(bits: u16) -> Result<Self> {
        if bits > 255
            || (bits & 2 != 0 && bits & 1 == 0)
            || (bits & 0xfc != 0 && bits & 3 != 3)
            || (bits & 128 != 0 && bits & 16 == 0)
        {
            Err(ContractError::Mask)
        } else {
            Ok(Self(bits))
        }
    }
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }
    #[must_use]
    pub const fn allows(self, action: ProjectAction) -> bool {
        self.0 & action as u16 != 0
    }
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[must_use]
    pub const fn is_manager(self) -> bool {
        self.0 == 255
    }
}
impl TryFrom<u16> for ActionMask {
    type Error = ContractError;
    fn try_from(v: u16) -> Result<Self> {
        Self::new(v)
    }
}
impl From<ActionMask> for u16 {
    fn from(v: ActionMask) -> Self {
        v.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectPreset {
    Discoverer,
    Reader,
    Author,
    Runner,
    AuthorRunner,
    Manager,
}
impl ProjectPreset {
    #[must_use]
    pub const fn mask(self) -> ActionMask {
        ActionMask(match self {
            Self::Discoverer => 1,
            Self::Reader => 3,
            Self::Author => 71,
            Self::Runner => 11,
            Self::AuthorRunner => 79,
            Self::Manager => 255,
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryRole {
    Owner,
    Admin,
    Editor,
    Viewer,
}
impl LibraryRole {
    /// Library actions only; Project run/access are separately calculated.
    #[must_use]
    pub const fn library_actions(self) -> ActionMask {
        ActionMask(match self {
            Self::Owner | Self::Admin => 247,
            Self::Editor => 71,
            Self::Viewer => 3,
        })
    }
    #[must_use]
    pub const fn may_invite_role(self, target: Self) -> bool {
        matches!(self, Self::Owner)
            || (matches!(self, Self::Admin) && !matches!(target, Self::Owner))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityPolicy {
    Restricted,
    LibraryVisible,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySpec {
    pub visibility: VisibilityPolicy,
    pub owner: ActionMask,
    pub admin: ActionMask,
    pub editor: ActionMask,
    pub viewer: ActionMask,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PolicySpec", into = "PolicySpec")]
pub struct ProjectPolicy(PolicySpec);
impl ProjectPolicy {
    #[must_use]
    pub const fn restricted() -> Self {
        Self(PolicySpec {
            visibility: VisibilityPolicy::Restricted,
            owner: ActionMask::NONE,
            admin: ActionMask::NONE,
            editor: ActionMask::NONE,
            viewer: ActionMask::NONE,
        })
    }
    /// Explicit user choice; never inferred merely from membership.
    #[must_use]
    pub const fn library_visible_readers() -> Self {
        Self(PolicySpec {
            visibility: VisibilityPolicy::LibraryVisible,
            owner: ActionMask(3),
            admin: ActionMask(3),
            editor: ActionMask(3),
            viewer: ActionMask(3),
        })
    }
    #[must_use]
    pub const fn spec(&self) -> &PolicySpec {
        &self.0
    }
    #[must_use]
    pub const fn inherited(&self, role: LibraryRole) -> ActionMask {
        match role {
            LibraryRole::Owner => self.0.owner,
            LibraryRole::Admin => self.0.admin,
            LibraryRole::Editor => self.0.editor,
            LibraryRole::Viewer => self.0.viewer,
        }
    }
}
impl TryFrom<PolicySpec> for ProjectPolicy {
    type Error = ContractError;
    fn try_from(p: PolicySpec) -> Result<Self> {
        for mask in [p.owner, p.admin, p.editor, p.viewer] {
            if ![0, 1, 3, 71, 11, 79].contains(&mask.bits())
                || (p.visibility == VisibilityPolicy::Restricted && mask != ActionMask::NONE)
            {
                return Err(ContractError::Mask);
            }
        }
        Ok(Self(p))
    }
}
impl From<ProjectPolicy> for PolicySpec {
    fn from(p: ProjectPolicy) -> Self {
        p.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Principal {
    Account { account_id: AccountId },
    LocalController { principal_id: LocalPrincipalId },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum LibraryAuthority {
    LocalOnly { controller: LocalPrincipalId },
    CloudMember,
    ProjectOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrantState {
    Active,
    Revoked,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectGrant {
    pub id: ProjectAccessGrantId,
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub principal: Principal,
    pub actions: ActionMask,
    pub state: GrantState,
    pub revision: LocalRevision,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Membership {
    pub library_id: LibraryId,
    pub account_id: AccountId,
    pub role: LibraryRole,
    pub active: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistrationState {
    Pending,
    Active,
    Closed,
}
/// Supplied by the authenticated controller; this DTO is not an authentication proof.
#[derive(Clone, Debug)]
pub struct AccessFacts<'a> {
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub authority: LibraryAuthority,
    pub subject: Principal,
    pub account_active: bool,
    pub identity_active: bool,
    pub library_active: bool,
    pub registration: RegistrationState,
    pub policy: &'a ProjectPolicy,
    pub membership: Option<Membership>,
    pub grant: Option<&'a ProjectGrant>,
}
/// Calculates R2 effective rights, rejecting mixed-scope/principal evidence.
/// # Errors
/// Rejects contradictory Library/Project/principal/membership facts.
pub fn effective_project_access(f: &AccessFacts<'_>) -> Result<ActionMask> {
    let role = match (f.authority, f.subject) {
        (
            LibraryAuthority::LocalOnly { controller },
            Principal::LocalController { principal_id },
        ) if controller == principal_id => {
            if f.membership.is_some() {
                return Err(ContractError::Scope);
            }
            Some(LibraryRole::Owner)
        }
        (
            LibraryAuthority::CloudMember | LibraryAuthority::ProjectOnly,
            Principal::Account { account_id },
        ) => {
            if let Some(m) = f.membership {
                if m.library_id != f.library_id
                    || m.account_id != account_id
                    || f.authority == LibraryAuthority::ProjectOnly
                {
                    return Err(ContractError::Scope);
                }
                if m.active { Some(m.role) } else { None }
            } else {
                None
            }
        }
        _ => return Err(ContractError::Scope),
    };
    if let Some(g) = f.grant {
        if g.library_id != f.library_id || g.project_id != f.project_id || g.principal != f.subject
        {
            return Err(ContractError::Scope);
        }
        if g.state == GrantState::Revoked {
            return Ok(ActionMask::NONE);
        }
    }
    if !f.library_active
        || f.registration != RegistrationState::Active
        || (matches!(f.subject, Principal::Account { .. })
            && (!f.account_active || !f.identity_active))
    {
        return Ok(ActionMask::NONE);
    }
    let inherited = role.map_or(ActionMask::NONE, |r| f.policy.inherited(r));
    Ok(inherited.union(f.grant.map_or(ActionMask::NONE, |g| g.actions)))
}
/// A pending invitation confers no rights. This checks only the inviter's ceiling.
#[must_use]
pub const fn may_offer_project_grant(
    inviter: ActionMask,
    offered: ActionMask,
    overrides_deny: bool,
) -> bool {
    inviter.allows(ProjectAction::Invite)
        && inviter.contains(offered)
        && (!(overrides_deny || offered.allows(ProjectAction::ManageAccess))
            || inviter.allows(ProjectAction::ManageAccess))
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorizationFreshness {
    Current,
    LastObserved,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedOperation {
    Run,
    Publish,
    Effect,
}
/// Freshness is supplied by the host, never guessed from wall time here.
#[must_use]
pub const fn may_start_protected_operation(
    authority: LibraryAuthority,
    freshness: AuthorizationFreshness,
    actions: ActionMask,
    operation: ProtectedOperation,
) -> bool {
    let fresh = matches!(authority, LibraryAuthority::LocalOnly { .. })
        || matches!(freshness, AuthorizationFreshness::Current);
    fresh
        && actions.allows(match operation {
            ProtectedOperation::Run | ProtectedOperation::Effect => ProjectAction::Run,
            ProtectedOperation::Publish => ProjectAction::Edit,
        })
}
