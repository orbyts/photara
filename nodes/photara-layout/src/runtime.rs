use std::collections::BTreeMap;

use photara_core::{
    AssetSet, CanonicalDigest, NodeDefinitionRef, NodeEvaluationOutput, NodeEvaluationRequest,
    NodeExecutionError, NodeRuntime, PackageVersion, PortId, canonical_digest,
};

use crate::{DEFINITION_ID, LayoutPlan, LayoutState, PACKAGE_ID, resolve_layout};

/// Deterministic semantic Layout evaluator.
///
/// Evaluation deliberately performs no proxy request. Proxies are requested
/// separately for runtime preview and cannot affect this authoritative output.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutNodeRuntime;

impl NodeRuntime for LayoutNodeRuntime {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<CanonicalDigest> {
        is_layout_definition(definition).then(|| {
            canonical_digest(&(DEFINITION_ID, 1_u32, "photara.layout-runtime-v1"))
                .expect("built-in implementation fingerprint is canonical")
        })
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError> {
        if !is_layout_definition(&request.node.definition) {
            return Err(execution_error(
                "photara.layout.wrong-definition",
                "Layout runtime received a different node definition",
            ));
        }
        if request.cancellation.is_cancelled() {
            return Err(execution_error(
                "photara.layout.cancelled",
                "Layout evaluation was cancelled",
            ));
        }
        let authored_state = request.node.authored_state.as_ref().ok_or_else(|| {
            execution_error(
                "photara.layout.missing-authored-state",
                "Layout node has no authored state",
            )
        })?;
        let state = LayoutState::from_schema_value(authored_state).map_err(|error| {
            execution_error("photara.layout.invalid-authored-state", error.to_string())
        })?;
        let assets_port = PortId::parse("assets").expect("built-in port ID is valid");
        let asset_values = request.inputs.get(&assets_port).ok_or_else(|| {
            execution_error(
                "photara.layout.missing-assets",
                "Layout requires one explicit AssetSet input",
            )
        })?;
        if asset_values.len() != 1 {
            return Err(execution_error(
                "photara.layout.invalid-assets-cardinality",
                "Layout requires exactly one AssetSet value",
            ));
        }
        let assets = AssetSet::from_typed_value(&asset_values[0])
            .map_err(|error| execution_error("photara.layout.invalid-assets", error.to_string()))?;
        let plan: LayoutPlan = resolve_layout(&state, &assets)
            .map_err(|error| execution_error(error.code(), error.to_string()))?;
        let layout_port = PortId::parse("layout").expect("built-in port ID is valid");
        Ok(NodeEvaluationOutput {
            outputs: BTreeMap::from([(
                layout_port,
                vec![plan.to_typed_value().map_err(|error| {
                    execution_error("photara.layout.output-serialization", error.to_string())
                })?],
            )]),
        })
    }
}

fn is_layout_definition(definition: &NodeDefinitionRef) -> bool {
    definition.package_id.as_str() == PACKAGE_ID
        && definition.package_version == PackageVersion::new(0, 2, 0)
        && definition.definition_id.as_str() == DEFINITION_ID
        && definition.definition_version.get() == 1
}

fn execution_error(code: impl Into<String>, message: impl Into<String>) -> NodeExecutionError {
    NodeExecutionError {
        code: code.into(),
        message: message.into(),
    }
}
