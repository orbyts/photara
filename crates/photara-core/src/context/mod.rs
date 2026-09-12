//! Pure, bounded D18 context contracts. No host access or existing evaluator integration.
//!
//! Fallible public operations return redacted `ContextError` diagnostics; construction
//! DTOs must pass their validator before use. Submodules share this error contract.
#![allow(clippy::missing_errors_doc)]

pub mod cache;
pub mod expression;
mod interpreter;
pub use interpreter::EvaluatedValue;
pub mod metadata;
mod parser;
pub mod proposal;
pub mod snapshot;
pub mod value;
pub mod variable;

use crate::contracts::schema::{ContractError, Digest};
use serde::{Deserialize, Serialize};

pub const SOURCE_LIMIT: usize = 16 * 1024;
pub const VALUE_LIMIT: usize = 64 * 1024;
pub const SNAPSHOT_LIMIT: usize = 1024 * 1024;
pub const AST_NODES: usize = 1024;
pub const AST_DEPTH: usize = 32;
pub const DIRECT_DEPENDENCIES: usize = 256;
pub const EXPANDED_DEPENDENCIES: usize = 10_000;
pub const OPERATIONS: usize = 100_000;
pub type Result<T> = std::result::Result<T, ContextError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    Syntax,
    UnsupportedVersion,
    Unknown,
    Forbidden,
    Revoked,
    Tombstoned,
    Unavailable,
    Stale,
    StaleFingerprint,
    Ambiguous,
    Conflict,
    TypeMismatch,
    Cyclic,
    LimitExceeded,
    Overflow,
    InvalidCoordinate,
    ScopeMismatch,
    SourceAstMismatch,
    Privacy,
    ConsentRequired,
    Incomplete,
    AssetContextRequired,
    IdempotencyConflict,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
impl Span {
    pub fn display(self, source: &str) -> Result<DisplaySpan> {
        if self.start > self.end
            || self.end > source.len()
            || !source.is_char_boundary(self.start)
            || !source.is_char_boundary(self.end)
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        let position = |at: usize| {
            let prefix = &source[..at];
            let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
            let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            (line, column)
        };
        let (start_line, start_column) = position(self.start);
        let (end_line, end_column) = position(self.end);
        Ok(DisplaySpan {
            start_line,
            start_column,
            end_line,
            end_column,
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplaySpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("context {code:?}")]
#[serde(deny_unknown_fields)]
pub struct ContextError {
    pub code: ErrorCode,
    pub span: Option<Span>,
}
impl From<ErrorCode> for ContextError {
    fn from(code: ErrorCode) -> Self {
        Self { code, span: None }
    }
}
impl From<ContractError> for ContextError {
    fn from(e: ContractError) -> Self {
        match e {
            ContractError::Limit => ErrorCode::LimitExceeded,
            ContractError::Privacy => ErrorCode::Privacy,
            ContractError::Unsupported | ContractError::Version => ErrorCode::UnsupportedVersion,
            ContractError::Scope => ErrorCode::ScopeMismatch,
            ContractError::Duplicate | ContractError::Order => ErrorCode::Conflict,
            _ => ErrorCode::InvalidCoordinate,
        }
        .into()
    }
}
pub(crate) fn at(code: ErrorCode, start: usize, end: usize) -> ContextError {
    ContextError {
        code,
        span: Some(Span { start, end }),
    }
}
pub(crate) fn bytes<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>> {
    let b = crate::canonical_json(value)
        .map_err(|_| ContextError::from(ErrorCode::InvalidCoordinate))?;
    if b.len() > limit {
        return Err(ErrorCode::LimitExceeded.into());
    }
    Ok(b)
}
pub(crate) fn digest<T: Serialize>(value: &T, limit: usize) -> Result<Digest> {
    Ok(Digest::of_bytes(&bytes(value, limit)?))
}

/// Private device captures cannot be persisted through portable serializers.
/// ```compile_fail
/// fn serializable<T: serde::Serialize>() {}
/// serializable::<photara_core::context::snapshot::DeviceContextSnapshot>();
/// ```
/// Frozen evaluation inputs cannot be persisted as mutable/live context.
/// ```compile_fail
/// fn serializable<T: serde::Serialize>() {}
/// serializable::<photara_core::context::snapshot::FrozenContext>();
/// ```
/// Derived runtime values require explicit validated capture/proposal conversion.
/// ```compile_fail
/// fn serializable<T: serde::Serialize>() {}
/// serializable::<photara_core::context::EvaluatedValue>();
/// ```
pub struct RuntimeSerializationBoundary;
