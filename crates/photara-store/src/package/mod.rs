//! Read-only generation-two directory-package codecs and validation.
//!
//! This module has no publication, locks, database, provider, migration or UI
//! side effects. It never resolves external resource handles. Original canonical
//! object bytes remain available; validation does not rewrite unknown fields.

mod json;
mod reader;
mod records;
mod types;

pub use json::{JsonLimits, parse_canonical_json, parse_json};
pub use reader::validate_resource_path;
pub use types::*;

use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use thiserror::Error;

/// Redacted errors deliberately omit paths, JSON values and provider payloads.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PackageError {
    #[error("invalid or unsupported JSON input")]
    Json,
    #[error("JSON bytes are not canonical")]
    NonCanonical,
    #[error("invalid package record")]
    Record,
    #[error("package content integrity check failed")]
    Integrity,
    #[error("unsafe package path or file type")]
    Path,
    #[error("package validation resource limit exceeded")]
    Limit,
    #[error("unsupported package version, schema or codec")]
    UnsupportedVersion,
    #[error("unsupported required package feature")]
    UnsupportedFeature,
    #[error("package changed during read")]
    ChangedDuringRead,
    #[error("safe directory reads are unavailable on this platform")]
    UnsupportedPlatform,
    #[error("package I/O failed ({0:?})")]
    Io(std::io::ErrorKind),
}

/// Local reader budgets may be narrower than the portable format ceilings.
#[derive(Clone, Copy, Debug)]
pub struct PackageLimits {
    pub json: JsonLimits,
    pub max_objects: usize,
    pub max_commits: usize,
    pub max_total_json_bytes: usize,
    pub max_blob_bytes: u64,
    pub max_total_blob_bytes: u64,
}
impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            json: JsonLimits::default(),
            max_objects: 100_000,
            max_commits: 1024,
            max_total_json_bytes: 128 * 1024 * 1024,
            max_blob_bytes: 1024 * 1024 * 1024,
            max_total_blob_bytes: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageDiagnostic {
    /// Metadata is retained; this reader never resolves or runs node code.
    NodeManifestUnavailable,
    /// An external handle is not inspected or reported as verified bytes.
    ExternalResourceNotResolved,
    /// Names beyond the currently implemented name-policy repertoire are not
    /// given a uniqueness claim by this initial validator.
    GraphNameNormalizationUnsupported,
}

/// An exact verified JSON object, including uninterpreted optional fields.
#[derive(Clone, Debug)]
pub struct VerifiedObject {
    pub reference: ObjectRef,
    pub value: Value,
    pub canonical_bytes: Vec<u8>,
}

/// Fully checksum-verified supported package snapshot. There is no writer API.
#[derive(Debug)]
pub struct ValidatedPackage {
    pub bootstrap: Bootstrap,
    pub head: Head,
    /// Newest first, including every verified ancestor.
    pub commits: Vec<Commit>,
    pub authored: AuthoredProject,
    pub graphs: Vec<SavedGraph>,
    pub objects: BTreeMap<Sha256Hex, VerifiedObject>,
    pub verified_blobs: usize,
    pub diagnostics: Vec<PackageDiagnostic>,
}

/// Reads only the bounded bootstrap for safe metadata inspection. It is not a
/// claim that the package or its required features are supported or verified.
///
/// # Errors
/// Returns an input/path/I/O error. Unknown version/feature values are retained.
pub fn inspect_bootstrap(
    root: impl AsRef<Path>,
    limits: PackageLimits,
) -> Result<Bootstrap, PackageError> {
    let reader = reader::Reader::open(root.as_ref(), limits)?;
    let value = parse_canonical_json(&reader.json("manifest.json")?, limits.json)?;
    records::uuid_field(&value, "project_id")?;
    decode(&value)
}

/// Reads and validates one published HEAD and all retained ancestor closures.
/// All managed blobs are hashed; external handles are never followed. No file
/// is created or changed. On Unix, reads are no-follow, descriptor-relative and
/// reject hardlinks/nonregular files. Other platforms fail closed for now.
///
/// # Errors
/// Returns redacted version, integrity, path, input, concurrency or limit errors.
#[expect(
    clippy::too_many_lines,
    reason = "The read-only commit walk is kept in publication order for auditability"
)]
pub fn validate_directory(
    root: impl AsRef<Path>,
    limits: PackageLimits,
) -> Result<ValidatedPackage, PackageError> {
    let reader = reader::Reader::open(root.as_ref(), limits)?;
    let bootstrap_bytes = reader.json("manifest.json")?;
    let bootstrap_value = parse_canonical_json(&bootstrap_bytes, limits.json)?;
    records::uuid_field(&bootstrap_value, "project_id")?;
    let bootstrap: Bootstrap = decode(&bootstrap_value)?;
    if bootstrap.format != "photara.project-package"
        || bootstrap.canonical_json != "photara.canonical-json.v1"
    {
        return Err(PackageError::UnsupportedVersion);
    }
    supported_format(&bootstrap.format_version)?;
    features(&bootstrap.required_features)?;
    records::timestamp(&bootstrap.created_at)?;
    let head_bytes = reader.json("HEAD.json")?;
    let head_value = parse_canonical_json(&head_bytes, limits.json)?;
    let head: Head = decode(&head_value)?;
    records::uuid_field(&head_value, "project_id")?;
    schema(&head.schema, "photara.package.head")?;
    if head.project_id != bootstrap.project_id {
        return Err(PackageError::Integrity);
    }
    let mut context = Context {
        reader,
        limits,
        project_id: bootstrap.project_id,
        objects: BTreeMap::new(),
        blobs: BTreeMap::new(),
        json_bytes: bootstrap_bytes.len() + head_bytes.len(),
        blob_bytes: 0,
        diagnostics: Vec::new(),
    };
    let bootstrap_hash = digest(&bootstrap_bytes);
    let mut pending = Some(CommitParent {
        commit_id: head.commit_id,
        sha256: head.commit_sha256.clone(),
        extra: BTreeMap::new(),
    });
    let mut seen = BTreeSet::new();
    let mut commits: Vec<Commit> = Vec::new();
    while let Some(parent) = pending.take() {
        if commits.len() >= limits.max_commits {
            return Err(PackageError::Limit);
        }
        if !seen.insert(parent.commit_id) {
            return Err(PackageError::Integrity);
        }
        let bytes = context
            .reader
            .json(&format!("commits/{}.json", parent.commit_id))?;
        context.add_json(bytes.len())?;
        if digest(&bytes) != parent.sha256 {
            return Err(PackageError::Integrity);
        }
        let value = parse_canonical_json(&bytes, limits.json)?;
        records::uuid_field(&value, "project_id")?;
        let commit: Commit = decode(&value)?;
        schema(&commit.schema, "photara.package.commit")?;
        features(&commit.required_features)?;
        supported_format(&commit.minimum_reader)?;
        records::timestamp(&commit.created_at)?;
        if commit.project_id != bootstrap.project_id
            || commit.commit_id != parent.commit_id
            || commit.bootstrap_sha256 != bootstrap_hash
            || commit.package_revision.get() == 0
        {
            return Err(PackageError::Integrity);
        }
        if let Some(newer) = commits.last()
            && commit.package_revision.get().checked_add(1) != Some(newer.package_revision.get())
        {
            return Err(PackageError::Integrity);
        }
        if commit.parent.is_none() && commit.package_revision.get() != 1 {
            return Err(PackageError::Integrity);
        }
        let inventory_value = context.load_json(&commit.inventory)?;
        let inventory: Inventory = decode(&inventory_value)?;
        schema(&inventory.schema, "photara.package.inventory")?;
        if inventory.project_id != bootstrap.project_id {
            return Err(PackageError::Integrity);
        }
        if inventory.objects.len() > limits.max_objects {
            return Err(PackageError::Limit);
        }
        if inventory.objects.windows(2).any(|p| p[0] >= p[1]) {
            return Err(PackageError::Integrity);
        }
        let closure = context.closure(&[commit.authored.clone(), commit.history.clone()])?;
        if closure != inventory.objects.into_iter().collect() {
            return Err(PackageError::Integrity);
        }
        let authored_value = context.load_json(&commit.authored)?;
        let authored: AuthoredProject = decode(&authored_value)?;
        records::validate_authored(&authored, &context)?;
        records::require_kind(
            &context.object(&commit.history)?.value,
            "photara.project.history",
        )?;
        pending.clone_from(&commit.parent);
        commits.push(commit);
    }
    records::validate_links(&context)?;
    let current = commits.first().ok_or(PackageError::Integrity)?;
    let authored: AuthoredProject = decode(&context.object(&current.authored)?.value)?;
    let graphs = authored
        .graphs
        .iter()
        .map(|g| decode(&context.object(&g.document)?.value))
        .collect::<Result<_, PackageError>>()?;
    if context.reader.json("HEAD.json")? != head_bytes
        || context.reader.json("manifest.json")? != bootstrap_bytes
    {
        return Err(PackageError::ChangedDuringRead);
    }
    Ok(ValidatedPackage {
        bootstrap,
        head,
        commits,
        authored,
        graphs,
        objects: context.objects,
        verified_blobs: context.blobs.len(),
        diagnostics: context.diagnostics,
    })
}

pub(super) struct Context {
    reader: reader::Reader,
    limits: PackageLimits,
    project_id: photara_core::ProjectId,
    objects: BTreeMap<Sha256Hex, VerifiedObject>,
    blobs: BTreeMap<Sha256Hex, ObjectRef>,
    json_bytes: usize,
    blob_bytes: u64,
    diagnostics: Vec<PackageDiagnostic>,
}
impl Context {
    fn add_json(&mut self, n: usize) -> Result<(), PackageError> {
        self.json_bytes = self.json_bytes.checked_add(n).ok_or(PackageError::Limit)?;
        if self.json_bytes > self.limits.max_total_json_bytes {
            return Err(PackageError::Limit);
        }
        Ok(())
    }
    fn object(&self, r: &ObjectRef) -> Result<&VerifiedObject, PackageError> {
        if r.kind != ObjectKind::Json {
            return Err(PackageError::Record);
        }
        let object = self.objects.get(&r.sha256).ok_or(PackageError::Integrity)?;
        if object.reference != *r {
            return Err(PackageError::Integrity);
        }
        Ok(object)
    }
    fn load_json(&mut self, r: &ObjectRef) -> Result<Value, PackageError> {
        if r.kind != ObjectKind::Json {
            return Err(PackageError::Record);
        }
        if self.objects.contains_key(&r.sha256) {
            return Ok(self.object(r)?.value.clone());
        }
        if self.objects.len() + self.blobs.len() >= self.limits.max_objects {
            return Err(PackageError::Limit);
        }
        if r.byte_length.get() > self.limits.json.max_bytes as u64 {
            return Err(PackageError::Limit);
        }
        let bytes = self.reader.json(&r.path())?;
        self.add_json(bytes.len())?;
        if bytes.len() as u64 != r.byte_length.get() || digest(&bytes) != r.sha256 {
            return Err(PackageError::Integrity);
        }
        let value = parse_canonical_json(&bytes, self.limits.json)?;
        records::validate_record(&value, self.project_id, &mut self.diagnostics)?;
        self.objects.insert(
            r.sha256.clone(),
            VerifiedObject {
                reference: r.clone(),
                value: value.clone(),
                canonical_bytes: bytes,
            },
        );
        Ok(value)
    }
    fn closure(&mut self, roots: &[ObjectRef]) -> Result<BTreeSet<ObjectRef>, PackageError> {
        let mut pending = roots.to_vec();
        let mut found = BTreeSet::new();
        while let Some(reference) = pending.pop() {
            if !found.insert(reference.clone()) {
                continue;
            }
            match reference.kind {
                ObjectKind::Json => {
                    let value = self.load_json(&reference)?;
                    records::references(&value, &mut pending)?;
                }
                ObjectKind::Blob => {
                    if let Some(old) = self.blobs.get(&reference.sha256) {
                        if old != &reference {
                            return Err(PackageError::Integrity);
                        }
                        continue;
                    }
                    if self.objects.len() + self.blobs.len() >= self.limits.max_objects {
                        return Err(PackageError::Limit);
                    }
                    self.blob_bytes = self
                        .blob_bytes
                        .checked_add(reference.byte_length.get())
                        .ok_or(PackageError::Limit)?;
                    if self.blob_bytes > self.limits.max_total_blob_bytes {
                        return Err(PackageError::Limit);
                    }
                    self.reader.blob(&reference)?;
                    self.blobs.insert(reference.sha256.clone(), reference);
                }
            }
        }
        Ok(found)
    }
}
pub(super) fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, PackageError> {
    serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)
}
pub(super) fn digest(bytes: &[u8]) -> Sha256Hex {
    Sha256Hex::parse(&format!("{:x}", Sha256::digest(bytes))).expect("SHA-256 emits canonical hex")
}
pub(super) fn schema(s: &RecordSchema, expected: &str) -> Result<(), PackageError> {
    if s.id != expected || s.version != 1 {
        Err(PackageError::UnsupportedVersion)
    } else {
        Ok(())
    }
}
fn supported_format(v: &FormatVersion) -> Result<(), PackageError> {
    if v.major != 1 || v.minor != 0 {
        Err(PackageError::UnsupportedVersion)
    } else {
        Ok(())
    }
}
fn features(values: &[String]) -> Result<(), PackageError> {
    if values.windows(2).any(|p| p[0] >= p[1]) {
        return Err(PackageError::Record);
    }
    if values.iter().any(|v| {
        !matches!(
            v.as_str(),
            "photara.history.v1" | "photara.immutable-objects.v1"
        )
    }) {
        return Err(PackageError::UnsupportedFeature);
    }
    Ok(())
}
