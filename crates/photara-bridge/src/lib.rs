//! Immutable DTO boundary for SwiftUI/AppKit and future native clients.

use photara_core::APPLICATION_API_VERSION;
use serde::{Deserialize, Serialize};

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
