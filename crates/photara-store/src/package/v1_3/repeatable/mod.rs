//! Additive repeatable layout. Older codec implementations and bytes stay unchanged.
mod input;
pub use input::{
    Allocation, AssociationId, Corridor, OriginalPackage, OriginalProof, PlannerInputs, Role,
    RoleAllocation, TreeGeometry, parse_inputs,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub semantic: crate::package::PackageLimits,
    pub max_frames: usize,
    pub max_total_allocation_bytes: u64,
    pub max_entries: usize,
    pub max_objects: usize,
    pub max_allocation_growth: u64,
    pub max_allocation_bytes: u64,
}
mod wire {
    use serde_json::Value;
    pub(super) type Result<T> = std::result::Result<T, &'static str>;
    pub(super) type Key = (String, u64);
    pub(super) fn ensure(ok: bool, error: &'static str) -> Result<()> {
        if ok { Ok(()) } else { Err(error) }
    }
    pub(super) fn text(v: &Value) -> Result<&str> {
        v.as_str().ok_or("string")
    }
    pub(super) fn number(v: &Value) -> Result<u64> {
        let s = text(v)?;
        let n = s.parse::<u64>().map_err(|_| "decimal")?;
        ensure(n.to_string() == s, "canonical decimal")?;
        Ok(n)
    }
    pub(super) fn uuid(v: &Value) -> Result<()> {
        let s = text(v)?;
        let id = uuid::Uuid::parse_str(s).map_err(|_| "UUID")?;
        ensure(!id.is_nil() && id.to_string() == s, "canonical UUID")
    }
    pub(super) fn encode(v: &Value) -> Vec<u8> {
        photara_core::canonical_json(v).expect("parsed JSON has canonical representation")
    }
    pub(super) fn hash(b: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(b))
    }
    pub(super) fn reference(v: &Value) -> Value {
        let b = encode(v);
        serde_json::json!({"kind":"json","sha256":hash(&b),"byte_length":b.len().to_string()})
    }
    pub(super) fn key(v: &Value) -> Result<Key> {
        let r: crate::package::ObjectRef =
            serde_json::from_value(v.clone()).map_err(|_| "JSON reference")?;
        ensure(
            r.kind == crate::package::ObjectKind::Json && r.byte_length.get() > 0,
            "JSON reference",
        )?;
        Ok((r.sha256.as_str().into(), r.byte_length.get()))
    }
    pub(super) fn parse(bytes: &[u8]) -> Result<Value> {
        crate::package::parse_canonical_json(
            bytes,
            crate::package::JsonLimits {
                max_bytes: bytes.len(),
                max_depth: 64,
                max_members: usize::MAX,
                max_array_elements: usize::MAX,
            },
        )
        .map_err(|_| "canonical JSON")
    }
}
mod layout;

mod core;
mod plan;
pub use plan::{Attempt, RepeatablePlan, SelectedStage, SelectorIds, VerifiedOriginal, compile};
mod execute;
pub use execute::{Completed, EffectFailure, ExecutionError, Observation, RepeatableIo, execute};
mod recovery;
#[cfg(test)]
mod tests;
pub use recovery::{Admission, admit, reconstruct_original, recover};
mod restart;
pub use restart::{RestartContext, restore_plan};
