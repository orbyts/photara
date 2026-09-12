//! Manifest v2 registration/validation only. No runtime, installer or migration execution.
mod types;
mod validation;
pub use types::*;
pub use validation::*;
