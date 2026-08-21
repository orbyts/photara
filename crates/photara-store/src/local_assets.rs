use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

use photara_core::{
    AssetId, AssetRepresentationId, FLATTENED_IMAGE_CAPABILITY_ID, HDR_CAPABILITY_ID,
    HDR_REPRESENTATION_ROLE_ID, IMAGE_CAPABILITY_ID, MaterializedRepresentation, ProjectAsset,
    ProjectDocument, ProjectRelativePath, ProjectResourceId, ProjectResourceRef,
    RepresentationAvailability, RepresentationBinding, RepresentationCapabilityId,
    RepresentationDescriptor, RepresentationFingerprint, RepresentationMaterializationError,
    RepresentationMaterializationRequest, RepresentationMaterializer, RepresentationRoleId,
    SDR_CAPABILITY_ID, SDR_REPRESENTATION_ROLE_ID, TIFF_CAPABILITY_ID,
};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Runtime adapter for project-relative local representations.
pub struct LocalProjectAssetAdapter<'a> {
    project_root: &'a Path,
    project: &'a ProjectDocument,
}

impl<'a> LocalProjectAssetAdapter<'a> {
    #[must_use]
    pub const fn new(project_root: &'a Path, project: &'a ProjectDocument) -> Self {
        Self {
            project_root,
            project,
        }
    }

    fn descriptor(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<&RepresentationDescriptor, RepresentationMaterializationError> {
        let asset = self.project.asset_context.asset(request.asset_id).ok_or(
            RepresentationMaterializationError::UnknownAsset(request.asset_id),
        )?;
        let representation = asset
            .representations
            .iter()
            .find(|representation| representation.id == request.representation_id)
            .ok_or(RepresentationMaterializationError::UnknownRepresentation {
                asset_id: request.asset_id,
                representation_id: request.representation_id,
            })?;
        if representation.fingerprint != request.expected_fingerprint {
            return Err(RepresentationMaterializationError::StaleRequest {
                expected: request.expected_fingerprint,
                actual: representation.fingerprint,
            });
        }
        Ok(representation)
    }

    fn local_path(
        &self,
        representation: &RepresentationDescriptor,
    ) -> Result<PathBuf, RepresentationMaterializationError> {
        let RepresentationBinding::ProjectResource { resource_id } = representation.binding;
        let resource = self
            .project
            .resources
            .iter()
            .find(|resource| resource.id == resource_id)
            .ok_or_else(|| RepresentationMaterializationError::Backend {
                message: format!("project resource {resource_id} is missing"),
            })?;
        Ok(self.project_root.join(resource.relative_path.as_str()))
    }
}

impl RepresentationMaterializer for LocalProjectAssetAdapter<'_> {
    fn availability(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<RepresentationAvailability, RepresentationMaterializationError> {
        let path = self.local_path(self.descriptor(request)?)?;
        match path.metadata() {
            Ok(metadata) if metadata.is_file() => Ok(RepresentationAvailability::Available),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(RepresentationAvailability::Missing)
            }
            Ok(_) | Err(_) => Ok(RepresentationAvailability::Inaccessible),
        }
    }

    fn materialize(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<MaterializedRepresentation, RepresentationMaterializationError> {
        let descriptor = self.descriptor(request)?;
        let path = self.local_path(descriptor)?;
        let availability = self.availability(request)?;
        if availability != RepresentationAvailability::Available {
            return Err(RepresentationMaterializationError::Unavailable(
                availability,
            ));
        }
        let (actual, byte_length) =
            fingerprint_file(&path).map_err(|error| materialization_backend(&error))?;
        if actual != descriptor.fingerprint {
            return Err(RepresentationMaterializationError::SourceChanged {
                expected: descriptor.fingerprint,
                actual,
            });
        }
        Ok(MaterializedRepresentation {
            asset_id: request.asset_id,
            representation_id: request.representation_id,
            fingerprint: actual,
            local_path: path,
            byte_length,
        })
    }
}

/// Imports paired local HDR/SDR flattened TIFF fixtures as two representations
/// of one semantic asset.
///
/// This development adapter mutates an in-memory Project Document. The caller
/// remains responsible for applying the surrounding semantic command/revision
/// policy before durable replacement.
///
/// # Errors
///
/// Returns [`LocalAssetAdapterError`] for non-TIFF paths, missing/unreadable
/// files, or a project validation failure.
pub fn import_local_tiff_pair(
    project: &mut ProjectDocument,
    project_root: &Path,
    display_name: impl Into<String>,
    hdr_path: ProjectRelativePath,
    sdr_path: ProjectRelativePath,
) -> Result<AssetId, LocalAssetAdapterError> {
    validate_tiff_path(&hdr_path)?;
    validate_tiff_path(&sdr_path)?;
    let (hdr_fingerprint, _) = fingerprint_file(&project_root.join(hdr_path.as_str()))?;
    let (sdr_fingerprint, _) = fingerprint_file(&project_root.join(sdr_path.as_str()))?;

    let asset_id = AssetId::new();
    let hdr_resource = ProjectResourceId::new();
    let sdr_resource = ProjectResourceId::new();
    let asset = ProjectAsset {
        id: asset_id,
        display_name: display_name.into(),
        representations: vec![
            representation(
                HDR_REPRESENTATION_ROLE_ID,
                HDR_CAPABILITY_ID,
                hdr_resource,
                hdr_fingerprint,
            ),
            representation(
                SDR_REPRESENTATION_ROLE_ID,
                SDR_CAPABILITY_ID,
                sdr_resource,
                sdr_fingerprint,
            ),
        ],
        extensions: BTreeMap::new(),
    };

    let mut updated = project.clone();
    updated.resources.extend([
        ProjectResourceRef {
            id: hdr_resource,
            relative_path: hdr_path,
        },
        ProjectResourceRef {
            id: sdr_resource,
            relative_path: sdr_path,
        },
    ]);
    updated.asset_context.assets.push(asset);
    updated.validate()?;
    *project = updated;
    Ok(asset_id)
}

/// Re-fingerprints one local representation after its upstream content changes,
/// preserving asset and representation identity.
///
/// # Errors
///
/// Returns [`LocalAssetAdapterError`] when identities, bindings, files, or the
/// updated Project Document are invalid.
pub fn refresh_local_representation_fingerprint(
    project: &mut ProjectDocument,
    project_root: &Path,
    asset_id: AssetId,
    representation_id: AssetRepresentationId,
) -> Result<RepresentationFingerprint, LocalAssetAdapterError> {
    let representation = project
        .asset_context
        .representation(asset_id, representation_id)
        .ok_or(LocalAssetAdapterError::UnknownRepresentation {
            asset_id,
            representation_id,
        })?;
    let RepresentationBinding::ProjectResource { resource_id } = representation.binding;
    let resource = project
        .resources
        .iter()
        .find(|resource| resource.id == resource_id)
        .ok_or(LocalAssetAdapterError::MissingProjectResource(resource_id))?;
    let (fingerprint, _) = fingerprint_file(&project_root.join(resource.relative_path.as_str()))?;

    let mut updated = project.clone();
    let descriptor = updated
        .asset_context
        .asset_mut(asset_id)
        .and_then(|asset| {
            asset
                .representations
                .iter_mut()
                .find(|representation| representation.id == representation_id)
        })
        .ok_or(LocalAssetAdapterError::UnknownRepresentation {
            asset_id,
            representation_id,
        })?;
    descriptor.fingerprint = fingerprint;
    updated.validate()?;
    *project = updated;
    Ok(fingerprint)
}

fn representation(
    role: &str,
    dynamic_range: &str,
    resource_id: ProjectResourceId,
    fingerprint: RepresentationFingerprint,
) -> RepresentationDescriptor {
    RepresentationDescriptor {
        id: AssetRepresentationId::new(),
        role: RepresentationRoleId::parse(role).expect("built-in representation role is valid"),
        fingerprint,
        capabilities: [
            IMAGE_CAPABILITY_ID,
            TIFF_CAPABILITY_ID,
            FLATTENED_IMAGE_CAPABILITY_ID,
            dynamic_range,
        ]
        .into_iter()
        .map(|capability| {
            RepresentationCapabilityId::parse(capability)
                .expect("built-in representation capability is valid")
        })
        .collect::<BTreeSet<_>>(),
        binding: RepresentationBinding::ProjectResource { resource_id },
        extensions: BTreeMap::new(),
    }
}

fn validate_tiff_path(path: &ProjectRelativePath) -> Result<(), LocalAssetAdapterError> {
    let is_tiff = Path::new(path.as_str())
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("tif") || extension.eq_ignore_ascii_case("tiff")
        });
    if is_tiff {
        Ok(())
    } else {
        Err(LocalAssetAdapterError::NotTiff(path.clone()))
    }
}

fn fingerprint_file(
    path: &Path,
) -> Result<(RepresentationFingerprint, u64), LocalAssetAdapterError> {
    let file = File::open(path).map_err(|source| LocalAssetAdapterError::Io {
        operation: "open representation",
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut byte_length = 0_u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|source| LocalAssetAdapterError::Io {
                operation: "read representation",
                path: path.to_path_buf(),
                source,
            })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        byte_length = byte_length.saturating_add(count as u64);
    }
    Ok((
        RepresentationFingerprint::sha256(digest.finalize().into()),
        byte_length,
    ))
}

fn materialization_backend(error: &LocalAssetAdapterError) -> RepresentationMaterializationError {
    RepresentationMaterializationError::Backend {
        message: error.to_string(),
    }
}

#[derive(Debug, Error)]
pub enum LocalAssetAdapterError {
    #[error("local TIFF fixture path must end in .tif or .tiff: {0}")]
    NotTiff(ProjectRelativePath),
    #[error("asset {asset_id} representation {representation_id} does not exist")]
    UnknownRepresentation {
        asset_id: AssetId,
        representation_id: AssetRepresentationId,
    },
    #[error("project resource {0} does not exist")]
    MissingProjectResource(ProjectResourceId),
    #[error("invalid project after local asset update: {0}")]
    InvalidProject(#[from] photara_core::ProjectValidationError),
    #[error("could not {operation} at {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::fs;

    use photara_core::{
        AssetSet, GraphDocument, GraphId, ProjectId, ProjectRelativePath,
        RepresentationMaterializer,
    };

    use super::*;
    use crate::{FileSystemStateStore, ProjectRepository};

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("photara-stage-5-assets-{}", ProjectId::new()));
            fs::create_dir_all(path.join("fixtures")).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn paired_project(root: &Path) -> (ProjectDocument, AssetId) {
        fs::write(root.join("fixtures/hdr.tif"), b"fake-hdr-tiff-16-bit").unwrap();
        fs::write(root.join("fixtures/sdr.tif"), b"fake-sdr-tiff-8-bit").unwrap();
        let mut project = ProjectDocument::new(
            ProjectId::new(),
            "Paired TIFFs",
            GraphDocument::new(GraphId::new()),
        )
        .unwrap();
        let asset_id = import_local_tiff_pair(
            &mut project,
            root,
            "Edited photograph",
            ProjectRelativePath::parse("fixtures/hdr.tif").unwrap(),
            ProjectRelativePath::parse("fixtures/sdr.tif").unwrap(),
        )
        .unwrap();
        (project, asset_id)
    }

    fn assert_pair_contract(
        project: &ProjectDocument,
        asset_id: AssetId,
    ) -> (RepresentationDescriptor, RepresentationDescriptor) {
        let asset = project.asset_context.asset(asset_id).unwrap();
        assert_eq!(asset.representations.len(), 2);
        let hdr = asset.representations[0].clone();
        let sdr = asset.representations[1].clone();
        assert_ne!(hdr.id, sdr.id);
        assert_ne!(hdr.fingerprint, sdr.fingerprint);
        assert_eq!(hdr.role.as_str(), HDR_REPRESENTATION_ROLE_ID);
        assert_eq!(sdr.role.as_str(), SDR_REPRESENTATION_ROLE_ID);
        assert!(
            hdr.capabilities
                .iter()
                .any(|capability| capability.as_str() == HDR_CAPABILITY_ID)
        );
        assert!(
            sdr.capabilities
                .iter()
                .any(|capability| capability.as_str() == SDR_CAPABILITY_ID)
        );
        AssetSet {
            assets: vec![asset_id],
        }
        .validate(&project.asset_context)
        .unwrap();
        (hdr, sdr)
    }

    #[test]
    fn paired_tiffs_share_asset_identity_and_track_paths_and_content_independently() {
        let root = TestRoot::new();
        let (mut project, asset_id) = paired_project(&root.0);
        let (hdr, _sdr) = assert_pair_contract(&project, asset_id);
        assert_eq!(
            ProjectDocument::from_json(&project.to_pretty_json().unwrap()).unwrap(),
            project
        );
        let mut state_store = FileSystemStateStore::open(root.0.join("state-store")).unwrap();
        state_store.create_project(project.clone()).unwrap();
        assert_eq!(
            state_store.load_project(project.project_id).unwrap(),
            Some(project.clone())
        );

        let request = RepresentationMaterializationRequest {
            asset_id,
            representation_id: hdr.id,
            expected_fingerprint: hdr.fingerprint,
        };
        let adapter = LocalProjectAssetAdapter::new(&root.0, &project);
        assert_eq!(
            adapter.availability(&request).unwrap(),
            RepresentationAvailability::Available
        );
        assert_eq!(
            adapter.materialize(&request).unwrap().local_path,
            root.0.join("fixtures/hdr.tif")
        );

        fs::rename(
            root.0.join("fixtures/hdr.tif"),
            root.0.join("fixtures/hdr-moved.tiff"),
        )
        .unwrap();
        let RepresentationBinding::ProjectResource { resource_id } = hdr.binding;
        project
            .resources
            .iter_mut()
            .find(|resource| resource.id == resource_id)
            .unwrap()
            .relative_path = ProjectRelativePath::parse("fixtures/hdr-moved.tiff").unwrap();
        {
            let moved_adapter = LocalProjectAssetAdapter::new(&root.0, &project);
            assert_eq!(
                moved_adapter.materialize(&request).unwrap().local_path,
                root.0.join("fixtures/hdr-moved.tiff")
            );
            assert_eq!(
                project
                    .asset_context
                    .asset(asset_id)
                    .unwrap()
                    .representations[0]
                    .id,
                hdr.id
            );
            assert_eq!(
                project
                    .asset_context
                    .asset(asset_id)
                    .unwrap()
                    .representations[0]
                    .fingerprint,
                hdr.fingerprint
            );

            fs::write(
                root.0.join("fixtures/hdr-moved.tiff"),
                b"changed-fake-hdr-tiff-content",
            )
            .unwrap();
            assert!(matches!(
                moved_adapter.materialize(&request),
                Err(RepresentationMaterializationError::SourceChanged { .. })
            ));
        }
        let refreshed =
            refresh_local_representation_fingerprint(&mut project, &root.0, asset_id, hdr.id)
                .unwrap();
        assert_ne!(refreshed, hdr.fingerprint);
        assert_eq!(
            project
                .asset_context
                .asset(asset_id)
                .unwrap()
                .representations[0]
                .id,
            hdr.id
        );
        assert_eq!(
            project
                .asset_context
                .asset(asset_id)
                .unwrap()
                .representations[0]
                .fingerprint,
            refreshed
        );
    }
}
