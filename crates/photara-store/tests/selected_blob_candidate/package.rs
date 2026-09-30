//! Trusted caller context for the shared driver; no selected wire record creates this authority.
use super::{
    keys::{JsonKey, MixedObjectKey},
    provider::{RawDescription, RawMetadata},
};
use photara_store::package::ObjectRef;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub(crate) type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone)]
pub(crate) struct Identity {
    pub project: String,
    pub library: String,
    pub bootstrap_sha256: String,
    pub manifest_bytes: Vec<u8>,
}
#[derive(Clone)]
pub(crate) struct AllocationRegistration {
    pub arena: String,
    pub layout: String,
    pub description: RawDescription,
    pub registered_charge: u64,
}
#[derive(Clone)]
pub(crate) struct Registration {
    pub profile: String,
    pub incarnation: String,
    pub allocations: BTreeMap<String, AllocationRegistration>,
    pub standing_control: u64,
    pub directory_allowance: u64,
}
pub(crate) struct SemanticMembers {
    pub json: BTreeSet<JsonKey>,
    pub blobs: BTreeSet<MixedObjectKey>,
}
/// Only legacy semantic closure is adaptable. HEAD, `StateRoot`, operation indexes,
/// physical trees, memberships and accounting remain the common reader's job.
pub(crate) trait RootSemantic {
    fn legacy_members(
        &self,
        state: &Value,
        resolve: &mut dyn FnMut(&Value) -> Result<Value>,
        blob_extent: &mut dyn FnMut(&ObjectRef) -> Result<u64>,
    ) -> Result<SemanticMembers>;
}
pub(crate) struct PackageContext<'a> {
    pub identity: &'a Identity,
    pub registration: &'a Registration,
    pub required_features: &'a BTreeSet<String>,
    pub semantic: &'a dyn RootSemantic,
    pub raw: &'a dyn RawMetadata,
}
