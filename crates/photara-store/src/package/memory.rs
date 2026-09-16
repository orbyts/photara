//! Bounded immutable input for virtual validation; never reads a filesystem.
use super::{PackageError, PackageLimits, digest, reader::internal_path};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct MemoryPackage {
    files: BTreeMap<String, Arc<[u8]>>,
    limits: PackageLimits,
}
impl MemoryPackage {
    /// Takes already supplied bytes. Limits may narrow but never raise reader ceilings.
    /// All supplied bytes (including unreachable retained data) count toward budgets.
    /// # Errors
    /// Rejects unsafe names, immutable hash mismatches and exceeded resource limits.
    pub fn new(
        files: BTreeMap<String, Vec<u8>>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        Self::from_shared(
            files.into_iter().map(|(k, v)| (k, Arc::from(v))).collect(),
            limits,
        )
    }
    pub(super) fn from_shared(
        files: BTreeMap<String, Arc<[u8]>>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        let ceiling = PackageLimits::default();
        let limits = PackageLimits {
            json: super::JsonLimits {
                max_bytes: limits.json.max_bytes.min(ceiling.json.max_bytes),
                max_depth: limits.json.max_depth.min(ceiling.json.max_depth),
                max_members: limits.json.max_members.min(ceiling.json.max_members),
                max_array_elements: limits
                    .json
                    .max_array_elements
                    .min(ceiling.json.max_array_elements),
            },
            max_objects: limits.max_objects.min(ceiling.max_objects),
            max_commits: limits.max_commits.min(ceiling.max_commits),
            max_total_json_bytes: limits
                .max_total_json_bytes
                .min(ceiling.max_total_json_bytes),
            max_blob_bytes: limits.max_blob_bytes.min(ceiling.max_blob_bytes),
            max_total_blob_bytes: limits
                .max_total_blob_bytes
                .min(ceiling.max_total_blob_bytes),
        };
        let (mut json, mut blobs, mut objects, mut commits) = (0usize, 0u64, 0usize, 0usize);
        for (path, bytes) in &files {
            internal_path(path)?;
            if path.starts_with("objects/") {
                objects += 1;
                let name = path.rsplit('/').next().ok_or(PackageError::Path)?;
                if name.trim_end_matches(".json") != digest(bytes).as_str() {
                    return Err(PackageError::Integrity);
                }
            }
            if path.starts_with("commits/") {
                commits += 1;
            }
            if path.starts_with("objects/blobs/") {
                if bytes.len() as u64 > limits.max_blob_bytes {
                    return Err(PackageError::Limit);
                }
                blobs = blobs
                    .checked_add(bytes.len() as u64)
                    .ok_or(PackageError::Limit)?;
            } else {
                if bytes.len() > limits.json.max_bytes {
                    return Err(PackageError::Limit);
                }
                json = json.checked_add(bytes.len()).ok_or(PackageError::Limit)?;
            }
            if objects > limits.max_objects
                || commits > limits.max_commits
                || json > limits.max_total_json_bytes
                || blobs > limits.max_total_blob_bytes
            {
                return Err(PackageError::Limit);
            }
        }
        Ok(Self { files, limits })
    }
    #[must_use]
    pub fn files(&self) -> &BTreeMap<String, Arc<[u8]>> {
        &self.files
    }
    #[must_use]
    pub fn limits(&self) -> PackageLimits {
        self.limits
    }
    pub(super) fn get(&self, name: &str) -> Result<&[u8], PackageError> {
        self.files
            .get(name)
            .map(AsRef::as_ref)
            .ok_or(PackageError::Integrity)
    }
}
