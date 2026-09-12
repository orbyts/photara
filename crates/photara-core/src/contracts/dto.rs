//! Versioned response coordinates only; no application facade or context evaluator.
use super::schema::{Digest, LocalRevision, QualifiedName, unique};
use super::{
    CommitId, ContractError, GraphId, LibraryId, NodeInstanceId, ProjectId, RequestId, Result,
};
use serde::{Deserialize, Serialize};

pub const CONTRACT_API_VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScopeRef {
    Library {
        library_id: LibraryId,
    },
    Project {
        project_id: ProjectId,
    },
    Graph {
        project_id: ProjectId,
        graph_id: GraphId,
    },
    Node {
        project_id: ProjectId,
        graph_id: GraphId,
        node_id: NodeInstanceId,
    },
}
/// Opaque server coordinate. It deliberately has no ordering or numeric accessor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServerRevision(String);
impl TryFrom<String> for ServerRevision {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        let suffix = s.strip_prefix("sr1:").ok_or(ContractError::Version)?;
        let n = suffix.parse::<i64>().map_err(|_| ContractError::Version)?;
        if n <= 0 || n.to_string() != suffix {
            return Err(ContractError::Version);
        }
        Ok(Self(s))
    }
}
impl From<ServerRevision> for String {
    fn from(v: ServerRevision) -> Self {
        v.0
    }
}
/// A generation is access state, never a record revision or content-stream epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct AuthorizationGeneration(i64);
impl AuthorizationGeneration {
    /// # Errors
    /// Rejects nonpositive generations.
    pub const fn new(n: i64) -> Result<Self> {
        if n > 0 {
            Ok(Self(n))
        } else {
            Err(ContractError::Version)
        }
    }
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl TryFrom<i64> for AuthorizationGeneration {
    type Error = ContractError;
    fn try_from(v: i64) -> Result<Self> {
        Self::new(v)
    }
}
impl From<AuthorizationGeneration> for i64 {
    fn from(v: AuthorizationGeneration) -> Self {
        v.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RevisionCoordinate {
    Local {
        revision: LocalRevision,
    },
    Server {
        revision: ServerRevision,
    },
    Package {
        commit_id: CommitId,
        commit_sha256: Digest,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseState {
    Loading,
    Ready,
    Stale,
    Unavailable,
    Forbidden,
    Unsupported,
    Conflict,
    Failed,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseMetadata {
    pub contract_api_version: u32,
    pub request_id: RequestId,
    pub scope: ScopeRef,
    pub base: Option<RevisionCoordinate>,
    pub authorization_generation: Option<AuthorizationGeneration>,
    pub state: ResponseState,
    pub diagnostic_codes: Vec<QualifiedName>,
}
impl ResponseMetadata {
    /// # Errors
    /// Rejects unsupported versions, unavailable ready bases and invalid scope unions.
    pub fn validate(&self) -> Result<()> {
        if self.contract_api_version != CONTRACT_API_VERSION {
            return Err(ContractError::Unsupported);
        }
        if self.state == ResponseState::Ready && self.base.is_none() {
            return Err(ContractError::Missing);
        }
        if matches!(self.scope, ScopeRef::Library { .. })
            && matches!(self.base, Some(RevisionCoordinate::Package { .. }))
        {
            return Err(ContractError::Scope);
        }
        if self.diagnostic_codes.len() > 256 {
            return Err(ContractError::Limit);
        }
        unique(&self.diagnostic_codes)
    }
    /// Caller supplies its latest request/base/generation; opaque coordinates only compare equal.
    #[must_use]
    pub fn matches_request(
        &self,
        request_id: RequestId,
        scope: ScopeRef,
        base: &Option<RevisionCoordinate>,
        generation: Option<AuthorizationGeneration>,
    ) -> bool {
        self.request_id == request_id
            && self.scope == scope
            && self.base == *base
            && self.authorization_generation == generation
    }
}
