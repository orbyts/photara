//! Legacy Graph/resource semantic closure over the frozen packed locator.
use super::super::{
    Commit, ObjectKind, ObjectRef, PackageDiagnostic, PackageError, PackageLimits, VerifiedObject,
    digest, v1_1,
};
use super::{BlobMetadata, Locator, Membership, PhysicalRecords, StructuralBlob};
use std::collections::{BTreeMap, BTreeSet};

/// Parsed legacy closure with metadata-only Blob checks; no media audit or writer grant.
pub struct SemanticClosure {
    members: BTreeSet<ObjectRef>,
    blobs: Vec<StructuralBlob>,
    diagnostics: Vec<PackageDiagnostic>,
}
impl SemanticClosure {
    #[must_use]
    pub fn members(&self) -> &BTreeSet<ObjectRef> {
        &self.members
    }
    #[must_use]
    pub fn blobs(&self) -> &[StructuralBlob] {
        &self.blobs
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[PackageDiagnostic] {
        &self.diagnostics
    }
}
/// Validates unchanged legacy authored/history records and their typed edges,
/// using the existing v1.1 link rules. Unknown optional JSON remains lossless and
/// does not create arbitrary reference edges. Blob content is not read or hashed.
/// # Errors
/// Refuses unsupported legacy schemas, wrong typed edges, missing/different
/// objects or metadata, identity disagreements and caller resource budgets.
pub fn inspect_legacy_semantics(
    locator: &Locator<'_, impl PhysicalRecords>,
    metadata: &impl BlobMetadata,
    commit: &Commit,
    limits: PackageLimits,
) -> Result<SemanticClosure, PackageError> {
    let mut pending = vec![commit.authored.clone(), commit.history.clone()];
    let mut members = BTreeSet::new();
    let mut objects = BTreeMap::new();
    let mut blobs = BTreeMap::new();
    let mut structural = Vec::new();
    let mut json_bytes = 0usize;
    let mut blob_bytes = 0u64;
    while let Some(reference) = pending.pop() {
        if members.contains(&reference) {
            continue;
        }
        if members.len() >= limits.max_objects {
            return Err(PackageError::Limit);
        }
        members.insert(reference.clone());
        match reference.kind {
            ObjectKind::Json => {
                if reference.byte_length.get() > limits.json.max_bytes as u64 {
                    return Err(PackageError::Limit);
                }
                let value = locator.json(&reference, Membership::Semantic)?;
                let canonical_bytes =
                    photara_core::canonical_json(&value).map_err(|_| PackageError::Record)?;
                json_bytes = json_bytes
                    .checked_add(canonical_bytes.len())
                    .ok_or(PackageError::Limit)?;
                if json_bytes > limits.max_total_json_bytes {
                    return Err(PackageError::Limit);
                }
                // Defend even alternate provider implementations before using typed edges.
                if digest(&canonical_bytes) != reference.sha256
                    || canonical_bytes.len() as u64 != reference.byte_length.get()
                {
                    return Err(PackageError::Integrity);
                }
                super::super::parse_canonical_json(&canonical_bytes, limits.json)?;
                v1_1::validate_record(&value, commit.project_id, &mut Vec::new())?;
                v1_1::references(&value, &mut pending)?;
                if pending.len() > limits.max_objects {
                    return Err(PackageError::Limit);
                }
                if objects
                    .insert(
                        reference.sha256.clone(),
                        VerifiedObject {
                            reference,
                            value,
                            canonical_bytes,
                        },
                    )
                    .is_some()
                {
                    return Err(PackageError::Integrity);
                }
            }
            ObjectKind::Blob => {
                blob_bytes = blob_bytes
                    .checked_add(reference.byte_length.get())
                    .ok_or(PackageError::Limit)?;
                if reference.byte_length.get() > limits.max_blob_bytes
                    || blob_bytes > limits.max_total_blob_bytes
                {
                    return Err(PackageError::Limit);
                }
                structural.push(locator.blob(&reference, metadata)?);
                if blobs.insert(reference.sha256.clone(), reference).is_some() {
                    return Err(PackageError::Integrity);
                }
            }
        }
    }
    let diagnostics = v1_1::validate_packed_semantics(commit, &members, objects, blobs, limits)?;
    Ok(SemanticClosure {
        members,
        blobs: structural,
        diagnostics,
    })
}
