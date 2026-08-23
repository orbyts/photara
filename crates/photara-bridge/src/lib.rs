//! Immutable DTO boundary for SwiftUI/AppKit and future native clients.

uniffi::setup_scaffolding!();

mod asset_materializer;
mod evaluation;
mod production;
mod runtime_registry;

pub use evaluation::EvaluationHandle;

pub use production::{
    BridgeAssetDto, BridgeCommandResponseDto, BridgeDiagnosticDto, BridgeDiagnosticSeverity,
    BridgeError, BridgeEvaluationFinishedDto, BridgeEvaluationPhase, BridgeEvaluationProgressDto,
    BridgeEvaluationStatus, BridgeGraphSnapshotDto, BridgeLayoutCanvas, BridgeNodeDto,
    BridgeProjectSnapshotDto, BridgeStructuredErrorDto, EvaluationObserver, PhotaraApplication,
    PhotaraProject,
};

use photara_core::APPLICATION_API_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, uniffi::Record)]
pub struct ApplicationInfo {
    pub api_version: u32,
    pub core_version: String,
    pub product_codename: String,
}

#[must_use]
#[uniffi::export]
pub fn application_info() -> ApplicationInfo {
    ApplicationInfo {
        api_version: APPLICATION_API_VERSION,
        core_version: env!("CARGO_PKG_VERSION").to_owned(),
        product_codename: "Photara".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_facade_is_explicitly_versioned() {
        let info = application_info();
        assert_eq!(info.api_version, 1);
        assert_eq!(info.product_codename, "Photara");
    }
}
