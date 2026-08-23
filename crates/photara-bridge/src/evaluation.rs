use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use photara_core::{
    CancellationToken, EvaluationError, EvaluationId, EvaluationPhase, EvaluationRequest,
    GraphDocument, RequestId, ValueTypeRegistry, canonical_digest, evaluate_graph,
};
use photara_node_sdk::NodePackageRegistry;

use crate::{
    production::{
        BridgeError, BridgeEvaluationFinishedDto, BridgeEvaluationPhase,
        BridgeEvaluationProgressDto, BridgeEvaluationStatus, EvaluationObserver, structured_error,
    },
    runtime_registry::RuntimeRegistry,
};

/// One-shot evaluation over an immutable project graph snapshot.
#[derive(uniffi::Object)]
pub struct EvaluationHandle {
    graph: GraphDocument,
    definitions: Arc<NodePackageRegistry>,
    runtimes: Arc<RuntimeRegistry>,
    value_types: Arc<ValueTypeRegistry>,
    cancellation: CancellationToken,
    started: AtomicBool,
}

impl EvaluationHandle {
    pub(crate) fn new(
        graph: GraphDocument,
        definitions: Arc<NodePackageRegistry>,
        runtimes: Arc<RuntimeRegistry>,
        value_types: Arc<ValueTypeRegistry>,
    ) -> Arc<Self> {
        Arc::new(Self {
            graph,
            definitions,
            runtimes,
            value_types,
            cancellation: CancellationToken::default(),
            started: AtomicBool::new(false),
        })
    }
}

#[uniffi::export]
impl EvaluationHandle {
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// Starts one evaluation and streams immutable events to Swift.
    ///
    /// # Errors
    ///
    /// Returns a state error when the handle was already started or its worker
    /// thread cannot be created.
    pub fn start(&self, observer: Arc<dyn EvaluationObserver>) -> Result<(), BridgeError> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(BridgeError::State {
                message: "evaluation handle has already started".to_owned(),
            });
        }
        let graph = self.graph.clone();
        let definitions = Arc::clone(&self.definitions);
        let runtimes = Arc::clone(&self.runtimes);
        let value_types = Arc::clone(&self.value_types);
        let cancellation = self.cancellation.clone();
        thread::Builder::new()
            .name("photara-evaluation".to_owned())
            .spawn(move || {
                run_evaluation(
                    &graph,
                    definitions.as_ref(),
                    runtimes.as_ref(),
                    value_types.as_ref(),
                    &cancellation,
                    observer.as_ref(),
                );
            })
            .map_err(|error| BridgeError::State {
                message: format!("could not start evaluation thread: {error}"),
            })?;
        Ok(())
    }
}

fn run_evaluation(
    graph: &GraphDocument,
    definitions: &NodePackageRegistry,
    runtimes: &RuntimeRegistry,
    value_types: &ValueTypeRegistry,
    cancellation: &CancellationToken,
    observer: &dyn EvaluationObserver,
) {
    let request = EvaluationRequest {
        request_id: RequestId::new(),
        evaluation_id: EvaluationId::new(),
        graph_id: graph.id,
        revision: graph.revision,
        environment: canonical_digest(&"photara-bridge-environment-v1")
            .expect("static environment descriptor is canonical"),
    };
    let result = evaluate_graph(
        graph,
        &request,
        definitions,
        value_types,
        runtimes,
        cancellation,
        |progress| observer.on_progress(progress.into()),
    );
    let finished = match result {
        Ok(outcome) => BridgeEvaluationFinishedDto {
            request_id: request.request_id.to_string(),
            evaluation_id: request.evaluation_id.to_string(),
            status: BridgeEvaluationStatus::Completed,
            evaluation_digest: Some(outcome.evaluation_key.to_string()),
            error: None,
        },
        Err(error) => {
            let status = if matches!(error, EvaluationError::Cancelled { .. }) {
                BridgeEvaluationStatus::Cancelled
            } else {
                BridgeEvaluationStatus::Failed
            };
            let details_json = serde_json::to_string(&error).unwrap_or_else(|_| "{}".to_owned());
            BridgeEvaluationFinishedDto {
                request_id: request.request_id.to_string(),
                evaluation_id: request.evaluation_id.to_string(),
                status,
                evaluation_digest: None,
                error: Some(structured_error(
                    error.code(),
                    error.to_string(),
                    error.diagnostic(),
                    details_json,
                )),
            }
        }
    };
    observer.on_finished(finished);
}

impl From<photara_core::EvaluationProgress> for BridgeEvaluationProgressDto {
    fn from(progress: photara_core::EvaluationProgress) -> Self {
        Self {
            request_id: progress.request_id.to_string(),
            evaluation_id: progress.evaluation_id.to_string(),
            phase: progress.phase.into(),
            completed_nodes: u64::try_from(progress.completed_nodes).unwrap_or(u64::MAX),
            total_nodes: u64::try_from(progress.total_nodes).unwrap_or(u64::MAX),
            node_id: progress.node_id.map(|id| id.to_string()),
        }
    }
}

impl From<EvaluationPhase> for BridgeEvaluationPhase {
    fn from(phase: EvaluationPhase) -> Self {
        match phase {
            EvaluationPhase::Validating => Self::Validating,
            EvaluationPhase::Planning => Self::Planning,
            EvaluationPhase::Evaluating => Self::Evaluating,
            EvaluationPhase::Completed => Self::Completed,
            EvaluationPhase::Cancelled => Self::Cancelled,
        }
    }
}
