//! Immutable DTO boundary for SwiftUI/AppKit and future native clients.

use photara_core::{
    APPLICATION_API_VERSION, CanonicalDigest, CommandId, Diagnostic, EvaluationError, EvaluationId,
    EvaluationPhase, EvaluationProgress, GraphCommandError, GraphCommandResult, GraphId,
    GraphRevision, NodeInstanceId, RequestId, canonical_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationInfo {
    pub api_version: u32,
    pub core_version: String,
    pub product_codename: String,
}

#[must_use]
pub fn application_info() -> ApplicationInfo {
    ApplicationInfo {
        api_version: APPLICATION_API_VERSION,
        core_version: env!("CARGO_PKG_VERSION").to_owned(),
        product_codename: "Photara".to_owned(),
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphSnapshotDto {
    pub graph_id: GraphId,
    pub revision: GraphRevision,
    pub digest: CanonicalDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandAppliedDto {
    pub command_id: CommandId,
    pub previous_revision: GraphRevision,
    pub snapshot: GraphSnapshotDto,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StructuredErrorDto {
    pub code: String,
    pub message: String,
    pub diagnostic: Diagnostic,
    pub details: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum CommandResponseDto {
    Applied(CommandAppliedDto),
    Rejected {
        command_id: CommandId,
        error: StructuredErrorDto,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationProgressDto {
    pub request_id: RequestId,
    pub evaluation_id: EvaluationId,
    pub phase: EvaluationPhase,
    pub completed_nodes: usize,
    pub total_nodes: usize,
    pub node_id: Option<NodeInstanceId>,
}

/// Converts an applied Core command into an immutable client response.
///
/// # Errors
///
/// Returns an error if the resulting graph cannot be canonically serialized.
pub fn command_applied(
    result: &GraphCommandResult,
) -> Result<CommandResponseDto, serde_json::Error> {
    Ok(CommandResponseDto::Applied(CommandAppliedDto {
        command_id: result.command_id,
        previous_revision: result.previous_revision,
        snapshot: GraphSnapshotDto {
            graph_id: result.graph.id,
            revision: result.revision,
            digest: canonical_digest(&result.graph)?,
        },
    }))
}

/// Converts a Core command error into a stable structured client response.
///
/// # Errors
///
/// Returns an error if the structured Core error cannot be serialized.
pub fn command_rejected(
    command_id: CommandId,
    error: &GraphCommandError,
) -> Result<CommandResponseDto, serde_json::Error> {
    Ok(CommandResponseDto::Rejected {
        command_id,
        error: StructuredErrorDto {
            code: error.code().to_owned(),
            message: error.to_string(),
            diagnostic: error.diagnostic(),
            details: serde_json::to_value(error)?,
        },
    })
}

/// Converts a Core evaluation error into an immutable client DTO.
///
/// # Errors
///
/// Returns an error if the structured Core error cannot be serialized.
pub fn evaluation_error(error: &EvaluationError) -> Result<StructuredErrorDto, serde_json::Error> {
    Ok(StructuredErrorDto {
        code: error.code().to_owned(),
        message: error.to_string(),
        diagnostic: error.diagnostic(),
        details: serde_json::to_value(error)?,
    })
}

impl From<&EvaluationProgress> for EvaluationProgressDto {
    fn from(progress: &EvaluationProgress) -> Self {
        Self {
            request_id: progress.request_id,
            evaluation_id: progress.evaluation_id,
            phase: progress.phase,
            completed_nodes: progress.completed_nodes,
            total_nodes: progress.total_nodes,
            node_id: progress.node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photara_core::GraphDocument;

    #[test]
    fn application_facade_is_explicitly_versioned() {
        let info = application_info();
        assert_eq!(info.api_version, 1);
        assert_eq!(info.product_codename, "Photara");
    }

    #[test]
    fn structured_command_errors_preserve_identity_and_machine_code() {
        let command_id = CommandId::new();
        let error = GraphCommandError::RevisionConflict {
            expected: GraphRevision::initial(),
            actual: GraphRevision::initial().next(),
        };
        let response = command_rejected(command_id, &error).unwrap();
        let encoded = serde_json::to_value(&response).unwrap();
        assert_eq!(encoded["status"], "rejected");
        assert_eq!(encoded["command_id"], command_id.to_string());
        assert_eq!(encoded["error"]["code"], "revision-conflict");
        assert_eq!(
            serde_json::from_value::<CommandResponseDto>(encoded).unwrap(),
            response
        );
    }

    #[test]
    fn progress_dto_keeps_request_evaluation_and_node_identity() {
        let progress = EvaluationProgress {
            request_id: RequestId::new(),
            evaluation_id: EvaluationId::new(),
            phase: EvaluationPhase::Evaluating,
            completed_nodes: 1,
            total_nodes: 2,
            node_id: Some(NodeInstanceId::new()),
        };
        let dto = EvaluationProgressDto::from(&progress);
        assert_eq!(dto.request_id, progress.request_id);
        assert_eq!(dto.evaluation_id, progress.evaluation_id);
        assert_eq!(dto.node_id, progress.node_id);
    }

    #[test]
    fn applied_command_dto_uses_the_resulting_revision_and_digest() {
        let graph = GraphDocument::new(GraphId::new());
        let result = GraphCommandResult {
            command_id: CommandId::new(),
            previous_revision: GraphRevision::initial(),
            revision: GraphRevision::initial().next(),
            graph: GraphDocument {
                revision: GraphRevision::initial().next(),
                ..graph
            },
        };
        let response = command_applied(&result).unwrap();
        let CommandResponseDto::Applied(applied) = response else {
            panic!("expected applied response");
        };
        assert_eq!(applied.snapshot.revision, result.revision);
        assert_eq!(
            applied.snapshot.digest,
            canonical_digest(&result.graph).unwrap()
        );
    }
}
