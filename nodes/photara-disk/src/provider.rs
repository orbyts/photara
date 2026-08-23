use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use photara_core::{
    AssetId, AssetRepresentationId, AssetSet, ProjectAsset, REPRESENTATION_FORMAT_EXTENSION_KEY,
    RepresentationBinding, RepresentationCapabilityId, RepresentationDescriptor,
    RepresentationFingerprint, RepresentationRevisionEvidence, RepresentationRoleId,
    RepresentationStorageBindingId,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::DiskFolderState;

/// Strength used while observing one authorized Disk folder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiskRevisionMode {
    /// Fast metadata observation suitable for publishing Gallery membership.
    Observation,
    /// Complete byte digest suitable for verified downstream materialization.
    Content,
}

/// Pure reconciliation prepared by the Disk provider for host publication.
pub struct DiskReconciliation {
    pub previous_asset_ids: Vec<AssetId>,
    pub assets: Vec<ProjectAsset>,
    pub runtime_bindings: BTreeMap<RepresentationStorageBindingId, PathBuf>,
    pub authored_state: DiskFolderState,
}

/// Authorized-folder provider behavior owned by the Disk node package.
pub struct DiskFolderProvider;

impl DiskFolderProvider {
    /// Enumerates supported files, observes their revisions, and prepares the
    /// next portable Disk membership without mutating project state.
    ///
    /// # Errors
    ///
    /// Returns a filesystem, metadata, or fingerprinting error when the
    /// authorized folder cannot be completely observed at the requested tier.
    ///
    /// # Panics
    ///
    /// Panics only if Photara's compile-time built-in capability or role IDs
    /// are invalid.
    pub fn reconcile(
        root: &Path,
        state: &DiskFolderState,
        revision_mode: DiskRevisionMode,
    ) -> Result<DiskReconciliation, String> {
        let mut files = Vec::new();
        collect_supported_files(root, root, state.recursive, &mut files)?;
        files.sort();
        let mut assets = Vec::with_capacity(files.len());
        let mut runtime_bindings = BTreeMap::new();
        for path in files {
            let relative = path.strip_prefix(root).map_err(|error| {
                format!(
                    "could not identify {} relative to folder: {error}",
                    path.display()
                )
            })?;
            let identity_key = relative.to_string_lossy();
            let asset_id = AssetId::from_uuid(stable_disk_uuid(
                "photara.disk.asset",
                state.folder_binding_id,
                &identity_key,
            ));
            let representation_id = AssetRepresentationId::from_uuid(stable_disk_uuid(
                "photara.disk.representation",
                state.folder_binding_id,
                &identity_key,
            ));
            let binding_id = RepresentationStorageBindingId::from_uuid(stable_disk_uuid(
                "photara.disk.storage-binding",
                state.folder_binding_id,
                &identity_key,
            ));
            let (fingerprint, revision_evidence) = match revision_mode {
                DiskRevisionMode::Observation => (
                    Self::observe_revision(&path)?,
                    RepresentationRevisionEvidence::FileObservation,
                ),
                DiskRevisionMode::Content => (
                    Self::fingerprint_contents(&path)?,
                    RepresentationRevisionEvidence::ContentDigest,
                ),
            };
            let mut capabilities = [
                photara_core::IMAGE_CAPABILITY_ID,
                photara_core::FLATTENED_IMAGE_CAPABILITY_ID,
            ]
            .into_iter()
            .map(|value| {
                RepresentationCapabilityId::parse(value)
                    .expect("built-in representation capability is valid")
            })
            .collect::<BTreeSet<_>>();
            if is_tiff(&path) {
                capabilities.insert(
                    RepresentationCapabilityId::parse(photara_core::TIFF_CAPABILITY_ID)
                        .expect("built-in TIFF capability is valid"),
                );
            }
            let extensions = normalized_format_label(&path).map_or_else(BTreeMap::new, |label| {
                BTreeMap::from([(
                    REPRESENTATION_FORMAT_EXTENSION_KEY.to_owned(),
                    serde_json::Value::String(label),
                )])
            });
            assets.push(ProjectAsset {
                id: asset_id,
                display_name: path.file_stem().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                ),
                representations: vec![RepresentationDescriptor {
                    id: representation_id,
                    role: RepresentationRoleId::parse(
                        photara_core::ORIGINAL_REPRESENTATION_ROLE_ID,
                    )
                    .expect("built-in representation role is valid"),
                    fingerprint,
                    revision_evidence,
                    capabilities,
                    binding: RepresentationBinding::RuntimeResolved { binding_id },
                    extensions,
                }],
                extensions: BTreeMap::new(),
            });
            runtime_bindings.insert(binding_id, path);
        }
        let previous_asset_ids = state.accepted_assets.assets.clone();
        let mut authored_state = state.clone();
        authored_state.accepted_assets = AssetSet {
            assets: assets.iter().map(|asset| asset.id).collect(),
        };
        Ok(DiskReconciliation {
            previous_asset_ids,
            assets,
            runtime_bindings,
            authored_state,
        })
    }

    /// Computes a cheap revision observation without reading file contents.
    ///
    /// # Errors
    ///
    /// Returns an error when file metadata or its modification time cannot be
    /// read or represented.
    pub fn observe_revision(path: &Path) -> Result<RepresentationFingerprint, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        let modified = metadata
            .modified()
            .map_err(|error| {
                format!(
                    "could not read modification time for {}: {error}",
                    path.display()
                )
            })?
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                format!("invalid modification time for {}: {error}", path.display())
            })?;
        let mut digest = Sha256::new();
        digest.update(b"photara.file-observation.v1");
        digest.update(metadata.len().to_be_bytes());
        digest.update(modified.as_secs().to_be_bytes());
        digest.update(modified.subsec_nanos().to_be_bytes());
        Ok(RepresentationFingerprint::sha256(digest.finalize().into()))
    }

    /// Computes a verified content revision by reading the complete file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be opened or read completely.
    pub fn fingerprint_contents(path: &Path) -> Result<RepresentationFingerprint, String> {
        let file = File::open(path)
            .map_err(|error| format!("could not open {}: {error}", path.display()))?;
        let mut reader = BufReader::new(file);
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        Ok(RepresentationFingerprint::sha256(digest.finalize().into()))
    }

    /// Re-observes a runtime file using the descriptor's declared evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence tier requires filesystem metadata or
    /// bytes that cannot be read.
    pub fn verify_revision(
        path: &Path,
        evidence: RepresentationRevisionEvidence,
        expected: RepresentationFingerprint,
    ) -> Result<RepresentationFingerprint, String> {
        match evidence {
            RepresentationRevisionEvidence::ContentDigest => Self::fingerprint_contents(path),
            RepresentationRevisionEvidence::FileObservation => Self::observe_revision(path),
            RepresentationRevisionEvidence::ProviderRevision => Ok(expected),
        }
    }
}

fn collect_supported_files(
    root: &Path,
    directory: &Path,
    recursive: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not read folder {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not read an entry in {}: {error}",
                directory.display()
            )
        })?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_file() && is_supported_visual_file(&entry.path()) {
            files.push(entry.path());
        } else if recursive && file_type.is_dir() && entry.path() != root {
            collect_supported_files(root, &entry.path(), true, files)?;
        }
    }
    Ok(())
}

fn is_supported_visual_file(path: &Path) -> bool {
    const STILL_IMAGE_EXTENSIONS: &[&str] = &[
        "3fr", "arw", "avif", "bmp", "cr2", "cr3", "dng", "erf", "exr", "fff", "gif", "heic",
        "heif", "iiq", "jpe", "jpeg", "jpg", "jxl", "mos", "nef", "nrw", "orf", "pef", "png",
        "psb", "psd", "raf", "raw", "rw2", "rwl", "sr2", "srf", "tif", "tiff", "webp",
    ];
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            STILL_IMAGE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

fn is_tiff(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("tif") || extension.eq_ignore_ascii_case("tiff")
        })
}

fn normalized_format_label(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(
        match extension.as_str() {
            "tif" | "tiff" => "TIFF",
            "jpg" | "jpeg" | "jpe" => "JPEG",
            value => value,
        }
        .to_ascii_uppercase(),
    )
}

fn stable_disk_uuid(domain: &str, folder_binding_id: Uuid, identity_key: &str) -> Uuid {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update(folder_binding_id.as_bytes());
    digest.update(identity_key.as_bytes());
    let hash = digest.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("photara-disk-provider-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reconciliation_is_stable_and_keeps_paths_runtime_only() {
        let root = TestRoot::new();
        fs::write(root.0.join("portrait.TIF"), b"pixels").unwrap();
        fs::write(root.0.join("notes.txt"), b"ignored").unwrap();
        let state = DiskFolderState::default();
        let first =
            DiskFolderProvider::reconcile(&root.0, &state, DiskRevisionMode::Observation).unwrap();
        let second =
            DiskFolderProvider::reconcile(&root.0, &state, DiskRevisionMode::Observation).unwrap();
        assert_eq!(first.assets.len(), 1);
        assert_eq!(first.assets[0].id, second.assets[0].id);
        assert_eq!(
            first.authored_state.accepted_assets.assets,
            vec![first.assets[0].id]
        );
        assert!(
            !serde_json::to_string(&first.authored_state)
                .unwrap()
                .contains(root.0.to_str().unwrap())
        );
        assert_eq!(first.runtime_bindings.len(), 1);
    }
}
