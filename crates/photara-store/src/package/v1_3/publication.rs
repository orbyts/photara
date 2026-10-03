//! Exact selector reconciliation and local publication ordering.
//!
//! These values are bookkeeping, not writable admission, storage qualification,
//! closure verification or an Accepted/Saved receipt. No new wire is serialized.
use super::super::{Head, JsonLimits, PackageError, parse_canonical_json};
use crate::package::planning::io::IoFailure;

/// Classification against the same original attempt, using exact bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectorObservation {
    /// Original selector remains. Earlier dependency writes may still exist.
    Original,
    /// Candidate selected; barriers, closure and original receipt still need checking.
    CandidateNeedsVerification,
    /// A recorded successful publication cannot be reconciled with the old selector.
    RollbackAfterReceipt,
    /// Neither original nor candidate. Preserve evidence and fence this attempt.
    Conflict,
}

/// Immutable original/candidate selector bytes for one publication attempt.
/// Construction checks selector syntax and identity, not the commit or its closure.
#[derive(Clone, Debug)]
pub struct SelectorComparison {
    original: Vec<u8>,
    candidate: Vec<u8>,
}
impl SelectorComparison {
    /// # Errors
    /// Refuses malformed/noncanonical HEADs, schema or project disagreement,
    /// identical commit IDs and budget exhaustion. Caller must retain original
    /// intent and independently authenticate both commits and candidate closure.
    pub fn parse(
        original: &[u8],
        candidate: &[u8],
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        fn head(bytes: &[u8], limits: JsonLimits) -> Result<Head, PackageError> {
            let value = parse_canonical_json(bytes, limits)?;
            let fields = value.as_object().ok_or(PackageError::Record)?;
            if fields.len() != 4
                || !["schema", "project_id", "commit_id", "commit_sha256"]
                    .iter()
                    .all(|key| fields.contains_key(*key))
                || value["schema"] != serde_json::json!({"id":"photara.package.head", "version":1})
            {
                return Err(PackageError::Record);
            }
            let project = crate::package::PackageUuid::parse(
                value["project_id"].as_str().ok_or(PackageError::Record)?,
            )?;
            if project.as_uuid().is_nil() {
                return Err(PackageError::Record);
            }
            let head: Head = serde_json::from_value(value).map_err(|_| PackageError::Record)?;
            if head.schema.id != "photara.package.head"
                || head.schema.version != 1
                || head.commit_id.as_uuid().is_nil()
            {
                return Err(PackageError::Record);
            }
            Ok(head)
        }
        let old = head(original, limits)?;
        let next = head(candidate, limits)?;
        if old.project_id != next.project_id || old.commit_id == next.commit_id {
            return Err(PackageError::Record);
        }
        Ok(Self {
            original: original.to_vec(),
            candidate: candidate.to_vec(),
        })
    }

    #[must_use]
    pub fn classify(&self, observed: &[u8], receipt_recorded: bool) -> SelectorObservation {
        if observed == self.candidate {
            SelectorObservation::CandidateNeedsVerification
        } else if observed == self.original {
            if receipt_recorded {
                SelectorObservation::RollbackAfterReceipt
            } else {
                SelectorObservation::Original
            }
        } else {
            SelectorObservation::Conflict
        }
    }
}

/// Completed steps in the immutable-first, selector-last publication protocol.
/// Calling these bookkeeping methods cannot authorize any adapter operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationStep {
    Prepared,
    IntentDurable,
    DependenciesDurable,
    CandidateVerified,
    IdentityAndHeadRechecked,
    HeadReplaced,
    HeadBarriersComplete,
    SelectedClosureVerified,
    ReceiptDurable,
}
impl PublicationStep {
    fn next(self) -> Option<Self> {
        Some(match self {
            Self::Prepared => Self::IntentDurable,
            Self::IntentDurable => Self::DependenciesDurable,
            Self::DependenciesDurable => Self::CandidateVerified,
            Self::CandidateVerified => Self::IdentityAndHeadRechecked,
            Self::IdentityAndHeadRechecked => Self::HeadReplaced,
            Self::HeadReplaced => Self::HeadBarriersComplete,
            Self::HeadBarriersComplete => Self::SelectedClosureVerified,
            Self::SelectedClosureVerified => Self::ReceiptDurable,
            Self::ReceiptDurable => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationOrderError {
    OutOfOrder,
    ReconciliationRequired,
}

/// Volatile progress retained alongside (never replacing) persisted intent.
/// Any adapter failure fences the attempt, including a pre-effect failure after
/// earlier successful effects. Resumption reconstructs state from durable evidence.
#[derive(Debug)]
pub struct PublicationProgress {
    completed: PublicationStep,
    failure: Option<IoFailure>,
}
impl Default for PublicationProgress {
    fn default() -> Self {
        Self {
            completed: PublicationStep::Prepared,
            failure: None,
        }
    }
}
impl PublicationProgress {
    #[must_use]
    pub fn completed(&self) -> PublicationStep {
        self.completed
    }

    #[must_use]
    pub fn failure(&self) -> Option<IoFailure> {
        self.failure
    }

    /// Record an adapter-completed step only after its required checks/barriers.
    /// # Errors
    /// Refuses skipped/repeated steps and any advance after a failure.
    pub fn complete(&mut self, step: PublicationStep) -> Result<(), PublicationOrderError> {
        if self.failure.is_some() {
            return Err(PublicationOrderError::ReconciliationRequired);
        }
        if self.completed.next() != Some(step) {
            return Err(PublicationOrderError::OutOfOrder);
        }
        self.completed = step;
        Ok(())
    }

    /// Preserve the first failure and the last completed step. A later failure
    /// must never downgrade an unknown outcome to "not performed".
    pub fn fail(&mut self, failure: IoFailure) {
        if self.failure.is_none() {
            self.failure = Some(failure);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::planning::io::WritePhase;
    use serde_json::Value;

    #[test]
    fn existing_phase_selectors_preserve_exact_recovery_and_rollback() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/selected-phase/linked.json"
        ))
        .unwrap();
        for (stage, cut) in corpus["selector_cuts"].as_object().unwrap() {
            let original = corpus["snapshots"][stage]["bootstrap"]["head"]
                .as_str()
                .unwrap()
                .as_bytes();
            let candidate = cut["HEAD.next"].as_str().unwrap().as_bytes();
            let compare =
                SelectorComparison::parse(original, candidate, JsonLimits::default()).unwrap();
            assert_eq!(
                compare.classify(original, false),
                SelectorObservation::Original
            );
            assert_eq!(
                compare.classify(original, true),
                SelectorObservation::RollbackAfterReceipt
            );
            assert_eq!(
                compare.classify(candidate, false),
                SelectorObservation::CandidateNeedsVerification
            );
            assert_eq!(
                compare.classify(candidate, true),
                SelectorObservation::CandidateNeedsVerification
            );
            let mut altered = candidate.to_vec();
            altered.push(b' ');
            assert_eq!(
                compare.classify(&altered, false),
                SelectorObservation::Conflict
            );
            assert!(SelectorComparison::parse(original, &altered, JsonLimits::default()).is_err());
            assert!(SelectorComparison::parse(original, original, JsonLimits::default()).is_err());
            for (key, bad) in [
                ("extra", serde_json::json!(true)),
                (
                    "project_id",
                    serde_json::json!("00000000-0000-0000-0000-000000000000"),
                ),
                (
                    "schema",
                    serde_json::json!({"id":"photara.package.head","version":1,"extra":true}),
                ),
            ] {
                let mut value: Value = serde_json::from_slice(candidate).unwrap();
                value[key] = bad;
                let bytes = photara_core::canonical_json(&value).unwrap();
                assert!(
                    SelectorComparison::parse(original, &bytes, JsonLimits::default()).is_err()
                );
            }
        }
    }

    #[test]
    fn every_failure_fences_later_progress_and_preserves_unknown() {
        let steps = [
            PublicationStep::IntentDurable,
            PublicationStep::DependenciesDurable,
            PublicationStep::CandidateVerified,
            PublicationStep::IdentityAndHeadRechecked,
            PublicationStep::HeadReplaced,
            PublicationStep::HeadBarriersComplete,
            PublicationStep::SelectedClosureVerified,
            PublicationStep::ReceiptDurable,
        ];
        for cut in 0..steps.len() {
            for failure in [
                IoFailure::NotPerformed(WritePhase::ReplaceHead),
                IoFailure::OutcomeUnknown(WritePhase::ReplaceHead),
            ] {
                let mut p = PublicationProgress::default();
                for step in &steps[..cut] {
                    p.complete(*step).unwrap();
                }
                let last = p.completed();
                p.fail(failure);
                p.fail(IoFailure::NotPerformed(WritePhase::Create));
                assert_eq!(p.failure(), Some(failure));
                assert_eq!(
                    p.complete(steps[cut]),
                    Err(PublicationOrderError::ReconciliationRequired)
                );
                assert_eq!(p.completed(), last);
            }
        }
        let mut p = PublicationProgress::default();
        assert_eq!(
            p.complete(PublicationStep::ReceiptDurable),
            Err(PublicationOrderError::OutOfOrder)
        );
        for step in steps {
            p.complete(step).unwrap();
        }
        assert_eq!(
            p.complete(PublicationStep::ReceiptDurable),
            Err(PublicationOrderError::OutOfOrder)
        );
    }
}
