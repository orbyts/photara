//! PS1 pure package checkpoint planning. No publication, journal, clock, random
//! allocation, live package access or durable Saved receipt. Existing 1.1 rules
//! validate both the base and the complete virtual candidate.
mod commands;
mod contracts;
pub mod io;
pub use contracts::*;

use super::{reader::Reader, *};
use photara_core::{
    DefinitionResolver, GraphId, ValueTypeRegistry, canonical_json, creation::PackageExtension,
};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlanError {
    #[error("package validation failed: {0}")]
    Package(#[from] PackageError),
    #[error("authored coordinate conflict")]
    RevisionConflict,
    #[error("exact package HEAD, manifest or incarnation changed")]
    ExternalChange,
    #[error("unsupported command or lossy schema mapping")]
    UnsupportedCommand,
    #[error("invalid semantic command")]
    Validation,
    #[error("revision exhausted")]
    RevisionExhausted,
    #[error("immutable name collision")]
    ImmutableConflict,
    #[error("outer filename cutover required before write admission")]
    FilenameCutoverRequired,
    #[error("storage capability profile is not qualified")]
    UnsupportedStorage,
}

/// Exact bytes as well as their digest are pinned. Numeric revision or a re-encoded
/// HEAD is never a CAS token. The incarnation is local, not portable content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadToken {
    incarnation: IncarnationId,
    manifest: Arc<[u8]>,
    head: Arc<[u8]>,
    head_digest: Sha256Hex,
    manifest_digest: Sha256Hex,
    revision: DecimalU64,
}
impl HeadToken {
    #[must_use]
    pub fn head_bytes(&self) -> &[u8] {
        &self.head
    }
    #[must_use]
    pub fn head_digest(&self) -> &Sha256Hex {
        &self.head_digest
    }
    #[must_use]
    pub fn manifest_digest(&self) -> &Sha256Hex {
        &self.manifest_digest
    }
    #[must_use]
    pub fn package_revision(&self) -> DecimalU64 {
        self.revision
    }
    /// Pure comparison only, not a hardware CAS. PS2 must call under a qualified
    /// exclusive lease after rechecking pinned root/parent/lock identities.
    /// # Errors
    /// Any byte or incarnation mismatch is an external change, including ABA
    /// replacement with an identical numeric revision.
    pub fn compare(
        &self,
        incarnation: IncarnationId,
        manifest: &[u8],
        head: &[u8],
    ) -> Result<(), PlanError> {
        if self.incarnation != incarnation
            || self.manifest.as_ref() != manifest
            || self.head.as_ref() != head
        {
            return Err(PlanError::ExternalChange);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct VerifiedClosure {
    files: MemoryPackage,
    verified: v1_1::ValidatedPackageV1_1,
    token: HeadToken,
    coordinate: AuthoredCoordinate,
}
impl VerifiedClosure {
    /// Verifies all input with the current reader; cannot be forged from a public
    /// reader DTO. Bytes remain immutable and no locator is opened.
    /// # Errors
    /// Refuses invalid or unsupported closures and resource exhaustion.
    pub fn verify(files: MemoryPackage, incarnation: IncarnationId) -> Result<Self, PlanError> {
        let verified = v1_1::validate_memory(&files)?;
        // This writer targets existing 1.1 authored v2 packages only; no upgrade.
        if verified.bootstrap.format_version.minor != 1
            || verified.authored.value["schema"]["version"] != 2
        {
            return Err(PlanError::UnsupportedCommand);
        }
        if verified
            .diagnostics
            .contains(&PackageDiagnostic::NodeManifestUnavailable)
        {
            return Err(PlanError::UnsupportedCommand);
        }
        let head = Arc::from(files.get("HEAD.json")?);
        let manifest = Arc::from(files.get("manifest.json")?);
        let token = HeadToken {
            incarnation,
            head_digest: digest(&head),
            manifest_digest: digest(&manifest),
            head,
            manifest,
            revision: verified.commits[0].package_revision,
        };
        commands::check_schemas(&verified)?;
        let coordinate = coordinate(&verified)?;
        Ok(Self {
            files,
            verified,
            token,
            coordinate,
        })
    }
    #[must_use]
    pub fn token(&self) -> &HeadToken {
        &self.token
    }
    #[must_use]
    pub fn coordinate(&self) -> &AuthoredCoordinate {
        &self.coordinate
    }
    #[must_use]
    pub fn files(&self) -> &MemoryPackage {
        &self.files
    }
}

#[derive(Debug)]
pub struct PlannedFile {
    name: String,
    bytes: Arc<[u8]>,
    sha256: Sha256Hex,
}
impl PlannedFile {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn sha256(&self) -> &Sha256Hex {
        &self.sha256
    }
    #[must_use]
    pub fn byte_length(&self) -> usize {
        self.bytes.len()
    }
    /// # Errors
    /// Existing immutable data is reusable only on exact byte/length/hash agreement.
    pub fn verify_existing(&self, bytes: &[u8]) -> Result<(), PlanError> {
        if bytes != self.bytes.as_ref() || digest(bytes) != self.sha256 {
            return Err(PlanError::ImmutableConflict);
        }
        Ok(())
    }
}
#[derive(Debug)]
pub struct CheckpointPlan {
    ids: CheckpointIds,
    expected: HeadToken,
    candidate: VerifiedClosure,
    immutable_files: Vec<PlannedFile>,
    receipt: PreparedReceipt,
}
impl CheckpointPlan {
    #[must_use]
    pub fn ids(&self) -> CheckpointIds {
        self.ids
    }
    #[must_use]
    pub fn expected_head(&self) -> &HeadToken {
        &self.expected
    }
    #[must_use]
    pub fn candidate(&self) -> &VerifiedClosure {
        &self.candidate
    }
    #[must_use]
    pub fn immutable_files(&self) -> &[PlannedFile] {
        &self.immutable_files
    }
    #[must_use]
    pub fn receipt(&self) -> &PreparedReceipt {
        &self.receipt
    }
}
#[derive(Debug)]
pub enum PlanOutcome {
    Unchanged(PreparedReceipt),
    Checkpoint(Box<CheckpointPlan>),
}

#[derive(Clone, Copy)]
pub struct PlanRequest<'a> {
    pub mutation: &'a MutationRequest,
    pub expected_head: &'a HeadToken,
    pub ids: CheckpointIds,
    pub naming: &'a PackageNamingPolicy,
    pub observed_extension: &'a PackageExtension,
}

/// Deterministic one-transaction checkpoint. Retry uses identical request and IDs;
/// durable operation-ID dedupe belongs to PS2's journal, not this stateless function.
/// # Errors
/// Refuses stale tokens/coordinates, unsupported mappings, overflow, immutable
/// conflicts, unsafe naming admission or any current-reader candidate violation.
#[expect(
    clippy::too_many_lines,
    reason = "Checkpoint assembly in dependency order"
)]
pub fn plan<R: DefinitionResolver>(
    base: &VerifiedClosure,
    request: PlanRequest<'_>,
    definitions: &R,
    value_types: &ValueTypeRegistry,
) -> Result<PlanOutcome, PlanError> {
    request.naming.admit_write(request.observed_extension)?;
    if request.expected_head != &base.token {
        return Err(PlanError::ExternalChange);
    }
    let mutation = request.mutation;
    if mutation.version != 1 {
        return Err(PlanError::UnsupportedCommand);
    }
    if mutation.expected != base.coordinate {
        return Err(PlanError::RevisionConflict);
    }
    super::records::timestamp(&mutation.updated_at)?;
    commands::admit(mutation, base.files.limits().json)?;
    let request_bytes = canonical_json(mutation).map_err(|_| PlanError::Validation)?;
    // Strict size/depth/array/member admission before semantic application.
    parse_canonical_json(&request_bytes, base.files.limits().json)?;
    let mut files = base.files.files().clone();
    let mut authored = base.verified.authored.value.clone();
    let changed = commands::apply(
        base,
        mutation,
        &mut authored,
        &mut files,
        definitions,
        value_types,
    )?;
    if !changed {
        return Ok(PlanOutcome::Unchanged(PreparedReceipt {
            operation_id: mutation.operation_id,
            request_digest: digest(&request_bytes),
            before: base.coordinate.clone(),
            after: base.coordinate.clone(),
        }));
    }
    authored["authored_revision"] = json!(next(base.coordinate.revision)?);
    authored["updated_at"] = json!(mutation.updated_at);
    let authored_ref = put_object(&mut files, &authored, base.files.limits())?;
    let current = &base.verified.commits[0];
    // Reuse the reader's schema-directed edge traversal. Inventories are per
    // commit reachable closures, not a union of all historical inventories.
    // Ancestors and their own closures remain byte-for-byte present in files.
    let interim = MemoryPackage::from_shared(files.clone(), base.files.limits())?;
    let mut context = Context {
        generation_two: true,
        reader: Reader::Memory(interim),
        limits: base.files.limits(),
        project_id: base.verified.bootstrap.project_id,
        objects: BTreeMap::new(),
        blobs: BTreeMap::new(),
        json_bytes: 0,
        blob_bytes: 0,
        diagnostics: Vec::new(),
    };
    let closure = context.closure(&[authored_ref.clone(), current.history.clone()])?;
    // Preserve opaque inventory/envelope fields by patching the original Value.
    let mut inventory = base.verified.objects[&current.inventory.sha256]
        .value
        .clone();
    inventory["objects"] = json!(closure);
    let inventory_ref = put_object(&mut files, &inventory, base.files.limits())?;
    let commit_name = format!("commits/{}.json", request.ids.commit_id);
    if files.contains_key(&commit_name)
        || base
            .verified
            .commits
            .iter()
            .any(|c| c.write_id == request.ids.write_id.uuid())
    {
        return Err(PlanError::ImmutableConflict);
    }
    let old_name = format!("commits/{}.json", current.commit_id);
    let mut commit = parse_canonical_json(base.files.get(&old_name)?, base.files.limits().json)?;
    commit["commit_id"] = json!(request.ids.commit_id);
    commit["write_id"] = json!(request.ids.write_id.uuid());
    commit["package_revision"] = json!(next(current.package_revision)?);
    commit["parent"] =
        json!({"commit_id":current.commit_id,"sha256":base.verified.head.commit_sha256});
    commit["created_at"] = json!(mutation.updated_at);
    commit["authored"] = json!(authored_ref);
    commit["inventory"] = json!(inventory_ref);
    let commit_bytes = encode(&commit, base.files.limits())?;
    let mut head = parse_canonical_json(base.token.head_bytes(), base.files.limits().json)?;
    head["commit_id"] = json!(request.ids.commit_id);
    head["commit_sha256"] = json!(digest(&commit_bytes));
    insert_immutable(&mut files, commit_name, commit_bytes)?;
    files.insert(
        "HEAD.json".into(),
        Arc::from(encode(&head, base.files.limits())?),
    );
    let candidate = VerifiedClosure::verify(
        MemoryPackage::from_shared(files, base.files.limits())?,
        base.token.incarnation,
    )?;
    let immutable_files = candidate
        .files
        .files()
        .iter()
        .filter(|(name, _)| name.as_str() != "HEAD.json" && !base.files.files().contains_key(*name))
        .map(|(name, bytes)| PlannedFile {
            name: name.clone(),
            bytes: bytes.clone(),
            sha256: digest(bytes),
        })
        .collect();
    let receipt = PreparedReceipt {
        operation_id: mutation.operation_id,
        request_digest: digest(&request_bytes),
        before: base.coordinate.clone(),
        after: candidate.coordinate.clone(),
    };
    Ok(PlanOutcome::Checkpoint(Box::new(CheckpointPlan {
        ids: request.ids,
        expected: base.token.clone(),
        candidate,
        immutable_files,
        receipt,
    })))
}

fn coordinate(package: &v1_1::ValidatedPackageV1_1) -> Result<AuthoredCoordinate, PlanError> {
    let mut graphs = BTreeMap::new();
    for graph in &package.graphs {
        let v = &graph.value;
        if v["schema"]["version"] != 2 || v["graph"]["schema_version"] != 1 {
            return Err(PlanError::UnsupportedCommand);
        }
        let id: GraphId = decode(&v["graph_id"])?;
        let revision = v["graph"]["revision"]
            .as_u64()
            .ok_or(PlanError::Validation)?;
        graphs.insert(
            id,
            GraphCoordinate {
                revision: decimal(revision)?,
                semantic_digest: digest(
                    &canonical_json(&decode::<photara_core::GraphDocument>(&v["graph"])?)
                        .map_err(|_| PlanError::Validation)?,
                ),
                payload_digest: digest(
                    &canonical_json(&v["graph"]).map_err(|_| PlanError::Validation)?,
                ),
                envelope_digest: graph.reference.sha256.clone(),
            },
        );
    }
    Ok(AuthoredCoordinate {
        revision: decode(&package.authored.value["authored_revision"])?,
        authored_digest: package.authored.reference.sha256.clone(),
        graphs,
    })
}
fn decimal(n: u64) -> Result<DecimalU64, PlanError> {
    Ok(DecimalU64::parse(&n.to_string())?)
}
fn next(n: DecimalU64) -> Result<DecimalU64, PlanError> {
    decimal(n.get().checked_add(1).ok_or(PlanError::RevisionExhausted)?)
}
fn encode(value: &Value, limits: PackageLimits) -> Result<Vec<u8>, PlanError> {
    let bytes = canonical_json(value).map_err(|_| PlanError::Validation)?;
    parse_canonical_json(&bytes, limits.json)?;
    Ok(bytes)
}
fn insert_immutable(
    files: &mut BTreeMap<String, Arc<[u8]>>,
    name: String,
    bytes: Vec<u8>,
) -> Result<(), PlanError> {
    if let Some(existing) = files.get(&name) {
        if existing.as_ref() != bytes {
            return Err(PlanError::ImmutableConflict);
        }
    } else {
        files.insert(name, Arc::from(bytes));
    }
    Ok(())
}
fn put_object(
    files: &mut BTreeMap<String, Arc<[u8]>>,
    value: &Value,
    limits: PackageLimits,
) -> Result<ObjectRef, PlanError> {
    let bytes = encode(value, limits)?;
    let reference = ObjectRef {
        kind: ObjectKind::Json,
        sha256: digest(&bytes),
        byte_length: decimal(bytes.len() as u64)?,
    };
    insert_immutable(files, reference.path(), bytes)?;
    Ok(reference)
}
