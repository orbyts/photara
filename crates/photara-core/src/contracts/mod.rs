//! Additive D19 contracts. These types grant no access and perform no I/O.
//!
//! V1 APIs, package codecs and evaluation keys remain independent. Wire decoding
//! validates invariants; raw DTOs with public fields must also pass `validate`.

pub mod access;
pub mod asset_set;
pub mod dto;
pub mod ids;
pub mod resource;
pub mod schema;

pub use ids::*;
pub use schema::{ContractError, Result};
