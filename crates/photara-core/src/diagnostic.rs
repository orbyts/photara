use serde::{Deserialize, Serialize};

use crate::{NodeInstanceId, PortId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_instance_id: Option<NodeInstanceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_id: Option<PortId>,
}
