//! Additive package 1.1 reader. Package envelopes and canonical encoding remain v1.
//! All original objects remain available; this module exposes no writer or evaluator.
mod closure;
mod links;
mod reader;
mod records;
use super::{Context, *};
pub(super) use closure::references;
use links::{validate_commit, validate_links};
use photara_core::{context as cx, contracts as ct};
pub use reader::validate_directory;
pub(super) use records::validate_record;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Verified immutable reader result; roots retain their exact version and bytes.
#[derive(Debug)]
pub struct ValidatedPackageV1_1 {
    pub bootstrap: Bootstrap,
    pub head: Head,
    pub commits: Vec<Commit>,
    pub authored: VerifiedObject,
    pub graphs: Vec<VerifiedObject>,
    pub objects: BTreeMap<Sha256Hex, VerifiedObject>,
    pub verified_blobs: usize,
    pub diagnostics: Vec<PackageDiagnostic>,
}
const FEATURES: [&str; 5] = [
    "photara.library-project.v1",
    "photara.resources.v2",
    "photara.asset-set.v2",
    "photara.context.v1",
    "photara.node-contract.v2",
];
fn format(v: &FormatVersion) -> Result<(), PackageError> {
    if v.major != 1 || v.minor > 1 {
        return Err(PackageError::UnsupportedVersion);
    }
    Ok(())
}
fn features(v: &[String]) -> Result<(), PackageError> {
    if v.windows(2).any(|w| w[0] >= w[1]) {
        return Err(PackageError::Record);
    }
    if v.iter().any(|s| {
        !FEATURES.contains(&s.as_str())
            && !matches!(
                s.as_str(),
                "photara.history.v1" | "photara.immutable-objects.v1"
            )
    }) {
        return Err(PackageError::UnsupportedFeature);
    }
    Ok(())
}
fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value, PackageError> {
    v.get(k).ok_or(PackageError::Record)
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, PackageError> {
    field(v, k)?.as_str().ok_or(PackageError::Record)
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, PackageError> {
    field(v, k)?.as_array().ok_or(PackageError::Record)
}
fn reference(v: &Value, k: &str) -> Result<ObjectRef, PackageError> {
    decode(field(v, k)?)
}
fn id(v: &Value, k: &str) -> Result<PackageUuid, PackageError> {
    let n = PackageUuid::parse(text(v, k)?)?;
    if n.as_uuid().is_nil() {
        return Err(PackageError::Record);
    }
    Ok(n)
}
fn hash(v: &Value, k: &str) -> Result<Sha256Hex, PackageError> {
    Sha256Hex::parse(text(v, k)?)
}
fn uint(v: &Value, k: &str) -> Result<u64, PackageError> {
    field(v, k)?.as_u64().ok_or(PackageError::Record)
}
fn decimal(v: &Value, k: &str) -> Result<u64, PackageError> {
    Ok(DecimalU64::parse(text(v, k)?)?.get())
}
fn kind(v: &Value) -> Result<(&str, u64), PackageError> {
    let s = field(v, "schema")?;
    Ok((text(s, "id")?, uint(s, "version")?))
}
fn check_kind(v: &Value, expected: &str) -> Result<(), PackageError> {
    if kind(v)?.0 != expected {
        return Err(PackageError::Integrity);
    }
    Ok(())
}
fn same(a: &Value, b: &Value, keys: &[&str]) -> Result<(), PackageError> {
    for k in keys {
        if field(a, k)? != field(b, k)? {
            return Err(PackageError::Integrity);
        }
    }
    Ok(())
}
fn bytes(v: &Value) -> Result<Vec<u8>, PackageError> {
    photara_core::canonical_json(v).map_err(|_| PackageError::Record)
}
fn checked<T>(r: Result<T, cx::ContextError>) -> Result<T, PackageError> {
    r.map_err(|_| PackageError::Record)
}
fn contract<T>(r: Result<T, ct::ContractError>) -> Result<T, PackageError> {
    r.map_err(|_| PackageError::Record)
}
fn target<'a>(c: &'a Context, r: &ObjectRef, k: &str) -> Result<&'a Value, PackageError> {
    let v = &c.object(r)?.value;
    check_kind(v, k)?;
    Ok(v)
}
fn payload(v: &Value) -> Value {
    let mut p = v.clone();
    for k in ["schema", "extensions", "project_id"] {
        p.as_object_mut().unwrap().remove(k);
    }
    p
}
fn scope(v: &Value, project: &Value) -> Result<ct::dto::ScopeRef, PackageError> {
    let s: ct::dto::ScopeRef = decode(v)?;
    match s {
        ct::dto::ScopeRef::Library { .. } => return Err(PackageError::Record),
        _ => {
            if field(v, "project_id")? != project {
                return Err(PackageError::Integrity);
            }
        }
    }
    Ok(s)
}
fn ordered_ids(v: &[Value], key: &str) -> Result<(), PackageError> {
    let ids = v
        .iter()
        .map(|x| id(x, key))
        .collect::<Result<Vec<_>, _>>()?;
    if ids.windows(2).any(|w| w[0] >= w[1]) {
        return Err(PackageError::Record);
    }
    Ok(())
}
fn known(v: &Value) -> Result<bool, PackageError> {
    let (k, n) = kind(v)?;
    Ok(matches!(
        (k, n),
        (
            "photara.project.authored"
                | "photara.project.saved-graph"
                | "photara.project.library-snapshot"
                | "photara.project.representation-content"
                | "photara.project.history"
                | "photara.history.run-start"
                | "photara.history.attempt-start"
                | "photara.history.effect-intent"
                | "photara.history.receipt",
            2
        ) | (
            "photara.project.asset-ledger"
                | "photara.project.resource-ledger"
                | "photara.project.managed-resource"
                | "photara.project.external-resource"
                | "photara.value.asset-set-snapshot"
                | "photara.value.asset-set-page"
                | "photara.context.authored"
                | "photara.context.variable"
                | "photara.context.expression"
                | "photara.context.snapshot"
                | "photara.metadata.observation"
                | "photara.metadata.patch"
                | "photara.context.change-proposal"
                | "photara.context.apply-receipt"
                | "photara.history.artifact"
                | "photara.value.group-set"
                | "photara.node.manifest",
            1
        )
    ))
}
