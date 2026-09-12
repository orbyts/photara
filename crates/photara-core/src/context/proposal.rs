//! Typed variable proposals and immutable receipt observations. Planning never applies changes.
use super::expression::{Coordinate, Dependency, project};
use super::snapshot::{Portability, Sensitivity};
use super::value::{Timestamp, TypedValue};
use super::{ErrorCode, Result, SNAPSHOT_LIMIT, digest};
use crate::contracts::{
    dto::{RevisionCoordinate, ScopeRef},
    ids::{AttemptId, ContextSnapshotId, LibraryId, OperationId, ProjectId, ReceiptId, RunId},
    schema::{Digest, LocalRevision, ObjectKind, ObjectRef},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProposalId(OperationId);
impl TryFrom<String> for ProposalId {
    type Error = super::ContextError;
    fn try_from(s: String) -> Result<Self> {
        Ok(Self(s.parse()?))
    }
}
impl From<ProposalId> for String {
    fn from(id: ProposalId) -> Self {
        id.0.to_string()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Authority {
    Library { library_id: LibraryId },
    Project { project_id: ProjectId },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalSpec {
    pub proposal_id: ProposalId,
    pub operation_id: OperationId,
    pub target: Dependency,
    pub expected_aggregate_revision: LocalRevision,
    pub expected_owner_revision: RevisionCoordinate,
    pub literal: TypedValue,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub attempt_id: AttemptId,
    pub snapshot_id: ContextSnapshotId,
    pub snapshot: ObjectRef,
    pub snapshot_digest: Digest,
    pub output_digest: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProposalSpec", into = "ProposalSpec")]
pub struct VariableChangeProposal(ProposalSpec);
impl TryFrom<ProposalSpec> for VariableChangeProposal {
    type Error = super::ContextError;
    fn try_from(s: ProposalSpec) -> Result<Self> {
        s.literal.validate(true)?;
        if !s.target.projection.is_empty()
            || !matches!(s.target.coordinate, Coordinate::Variable { .. })
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        if s.snapshot.kind != ObjectKind::Json || s.portability == Portability::HostOnly {
            return Err(ErrorCode::Privacy.into());
        }
        if let Coordinate::Variable { scope, .. } = s.target.coordinate
            && project(scope).is_some_and(|p| p != s.project_id)
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        super::bytes(&s, SNAPSHOT_LIMIT)?;
        Ok(Self(s))
    }
}
impl From<VariableChangeProposal> for ProposalSpec {
    fn from(s: VariableChangeProposal) -> Self {
        s.0
    }
}
impl VariableChangeProposal {
    #[must_use]
    pub fn spec(&self) -> &ProposalSpec {
        &self.0
    }
    pub fn request_digest(&self) -> Result<Digest> {
        digest(
            &serde_json::json!({"domain":"photara.context-proposal.v1","proposal":self}),
            SNAPSHOT_LIMIT,
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
    Running,
}
#[derive(Clone, Debug)]
pub struct TargetEvidence {
    pub target: Dependency,
    pub current_aggregate_revision: LocalRevision,
    pub current_owner_revision: RevisionCoordinate,
    pub expected_type: super::value::Type,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
    pub declared: bool,
    pub currently_authorized: bool,
}
#[derive(Clone, Debug)]
pub struct ProposalSourceEvidence {
    pub run_id: RunId,
    pub attempt_id: AttemptId,
    pub snapshot_id: ContextSnapshotId,
    pub snapshot: ObjectRef,
    pub snapshot_digest: Digest,
    pub output_digest: Digest,
    pub sensitivity: Sensitivity,
    pub portability: Portability,
}
#[derive(Clone, Debug)]
pub struct ApplyEvidence {
    pub run_id: RunId,
    pub outcome: RunOutcome,
    pub explicit_acceptance: bool,
    pub targets: Vec<TargetEvidence>,
    pub sources: Vec<ProposalSourceEvidence>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ApplyPlan {
    authority: Authority,
    proposals: Vec<VariableChangeProposal>,
    request_digest: Digest,
}
impl ApplyPlan {
    pub fn prepare(
        proposals: Vec<VariableChangeProposal>,
        evidence: &ApplyEvidence,
    ) -> Result<Self> {
        if proposals.is_empty() || proposals.len() > 1000 || evidence.targets.len() > 1000 {
            return Err(ErrorCode::LimitExceeded.into());
        }
        if !evidence.explicit_acceptance || evidence.outcome != RunOutcome::Succeeded {
            return Err(ErrorCode::Forbidden.into());
        }
        let mut seen = BTreeSet::new();
        let mut operations = BTreeSet::new();
        let mut evidence_keys = BTreeSet::new();
        let mut authority = None;
        for e in &evidence.targets {
            if !evidence_keys.insert(e.target.key()?) {
                return Err(ErrorCode::Conflict.into());
            }
        }
        for p in &proposals {
            let s = &p.0;
            if s.run_id != evidence.run_id {
                return Err(ErrorCode::ScopeMismatch.into());
            }
            let source = evidence
                .sources
                .iter()
                .find(|source| {
                    source.run_id == s.run_id
                        && source.attempt_id == s.attempt_id
                        && source.snapshot_id == s.snapshot_id
                        && source.snapshot == s.snapshot
                        && source.snapshot_digest == s.snapshot_digest
                        && source.output_digest == s.output_digest
                })
                .ok_or(ErrorCode::SourceAstMismatch)?;
            if s.sensitivity < source.sensitivity || s.portability < source.portability {
                return Err(ErrorCode::Privacy.into());
            }
            if !seen.insert(s.target.key()?) || !operations.insert(s.operation_id) {
                return Err(ErrorCode::Conflict.into());
            }
            let e = evidence
                .targets
                .iter()
                .find(|e| e.target == s.target)
                .ok_or(ErrorCode::Forbidden)?;
            if !e.declared || !e.currently_authorized {
                return Err(ErrorCode::Forbidden.into());
            }
            if e.current_aggregate_revision != s.expected_aggregate_revision
                || e.current_owner_revision != s.expected_owner_revision
            {
                return Err(ErrorCode::Conflict.into());
            }
            if e.expected_type != s.literal.ty {
                return Err(ErrorCode::TypeMismatch.into());
            }
            if s.sensitivity < e.sensitivity || s.portability < e.portability {
                return Err(ErrorCode::Privacy.into());
            }
            let Coordinate::Variable { scope, .. } = s.target.coordinate else {
                return Err(ErrorCode::InvalidCoordinate.into());
            };
            let current = match scope {
                ScopeRef::Library { library_id } => Authority::Library { library_id },
                _ => Authority::Project {
                    project_id: project(scope).ok_or(ErrorCode::ScopeMismatch)?,
                },
            };
            if authority.is_some_and(|a| a != current) {
                return Err(ErrorCode::ScopeMismatch.into());
            }
            authority = Some(current);
        }
        let authority = authority.ok_or(ErrorCode::InvalidCoordinate)?;
        let request_digest = digest(
            &serde_json::json!({"domain":"photara.context-apply-plan.v1","authority":authority,"proposals":proposals}),
            SNAPSHOT_LIMIT,
        )?;
        Ok(Self {
            authority,
            proposals,
            request_digest,
        })
    }
    #[must_use]
    pub const fn authority(&self) -> Authority {
        self.authority
    }
    #[must_use]
    pub const fn request_digest(&self) -> Digest {
        self.request_digest
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptOutcome {
    Applied,
    Rejected,
    Conflict,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSpec {
    pub receipt_id: ReceiptId,
    pub operation_id: OperationId,
    pub request_digest: Digest,
    pub authority: Authority,
    pub outcome: ReceiptOutcome,
    pub resulting_revisions: Vec<RevisionCoordinate>,
    pub prior_receipts: Vec<ReceiptId>,
    pub observed_at: Timestamp,
    pub evidence: Vec<ObjectRef>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReceiptSpec", into = "ReceiptSpec")]
pub struct ApplyReceipt(ReceiptSpec);
impl TryFrom<ReceiptSpec> for ApplyReceipt {
    type Error = super::ContextError;
    fn try_from(s: ReceiptSpec) -> Result<Self> {
        if s.prior_receipts.len() > 256
            || s.evidence.len() > 256
            || s.resulting_revisions.len() > 1000
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        crate::contracts::schema::ordered_unique(&s.prior_receipts)?;
        if s.prior_receipts.contains(&s.receipt_id)
            || (s.outcome == ReceiptOutcome::Applied) == s.resulting_revisions.is_empty()
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        for r in &s.resulting_revisions {
            if matches!(
                (s.authority, r),
                (
                    Authority::Library { .. },
                    RevisionCoordinate::Package { .. }
                ) | (Authority::Project { .. }, RevisionCoordinate::Server { .. })
            ) {
                return Err(ErrorCode::ScopeMismatch.into());
            }
        }
        Ok(Self(s))
    }
}
impl From<ApplyReceipt> for ReceiptSpec {
    fn from(s: ApplyReceipt) -> Self {
        s.0
    }
}
impl ApplyReceipt {
    #[must_use]
    pub fn spec(&self) -> &ReceiptSpec {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationKnowledge {
    Unobserved,
    Applied,
    Rejected,
    Conflict,
    Unknown,
    Disputed,
}
pub fn summarize_receipts(
    operation_id: OperationId,
    request_digest: Digest,
    receipts: &[ApplyReceipt],
) -> Result<ApplicationKnowledge> {
    if receipts.len() > 1000 {
        return Err(ErrorCode::LimitExceeded.into());
    }
    let mut ids = BTreeSet::new();
    let mut outcome = None;
    let mut unknown = false;
    let mut disputed = false;
    let mut authority = None;
    for r in receipts {
        let s = &r.0;
        if s.operation_id != operation_id || s.request_digest != request_digest {
            return Err(ErrorCode::IdempotencyConflict.into());
        }
        if !ids.insert(s.receipt_id) {
            return Err(ErrorCode::Conflict.into());
        }
        if authority.is_some_and(|a| a != s.authority) {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        authority = Some(s.authority);
        for prior in &s.prior_receipts {
            if !ids.contains(prior) {
                return Err(ErrorCode::Unknown.into());
            }
        }
        if s.outcome == ReceiptOutcome::Unknown {
            unknown = true;
        } else if outcome.is_some_and(|o| o != s.outcome) {
            disputed = true;
        } else {
            outcome = Some(s.outcome);
        }
    }
    if disputed {
        return Ok(ApplicationKnowledge::Disputed);
    }
    Ok(match outcome {
        Some(ReceiptOutcome::Applied) => ApplicationKnowledge::Applied,
        Some(ReceiptOutcome::Rejected) => ApplicationKnowledge::Rejected,
        Some(ReceiptOutcome::Conflict) => ApplicationKnowledge::Conflict,
        _ => {
            if unknown {
                ApplicationKnowledge::Unknown
            } else {
                ApplicationKnowledge::Unobserved
            }
        }
    })
}
