use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use photara_core::{
    ProjectDocument, RepresentationAvailability, RepresentationBinding, RepresentationDescriptor,
    RepresentationMaterializationError, RepresentationMaterializationRequest,
    RepresentationMaterializer, RepresentationStorageBindingId,
};
use photara_disk_node::DiskFolderProvider;
use photara_store::LocalProjectAssetAdapter;

/// Project-resource plus runtime-provider materializer used by host services.
pub(crate) struct RuntimeAwareMaterializer<'a> {
    local: LocalProjectAssetAdapter<'a>,
    project: &'a ProjectDocument,
    runtime_bindings: &'a BTreeMap<RepresentationStorageBindingId, PathBuf>,
}

impl<'a> RuntimeAwareMaterializer<'a> {
    pub(crate) fn new(
        project_root: &'a Path,
        project: &'a ProjectDocument,
        runtime_bindings: &'a BTreeMap<RepresentationStorageBindingId, PathBuf>,
    ) -> Self {
        Self {
            local: LocalProjectAssetAdapter::new(project_root, project),
            project,
            runtime_bindings,
        }
    }

    fn descriptor(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<&RepresentationDescriptor, RepresentationMaterializationError> {
        let descriptor = self
            .project
            .asset_context
            .representation(request.asset_id, request.representation_id)
            .ok_or(RepresentationMaterializationError::UnknownRepresentation {
                asset_id: request.asset_id,
                representation_id: request.representation_id,
            })?;
        if descriptor.fingerprint != request.expected_fingerprint {
            return Err(RepresentationMaterializationError::StaleRequest {
                expected: request.expected_fingerprint,
                actual: descriptor.fingerprint,
            });
        }
        Ok(descriptor)
    }

    fn runtime_path(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<Option<&PathBuf>, RepresentationMaterializationError> {
        match self.descriptor(request)?.binding {
            RepresentationBinding::ProjectResource { .. } => Ok(None),
            RepresentationBinding::RuntimeResolved { binding_id } => self
                .runtime_bindings
                .get(&binding_id)
                .map(Some)
                .ok_or_else(|| RepresentationMaterializationError::Backend {
                    message: format!("runtime storage binding {binding_id} is unavailable"),
                }),
        }
    }
}

impl RepresentationMaterializer for RuntimeAwareMaterializer<'_> {
    fn availability(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<RepresentationAvailability, RepresentationMaterializationError> {
        let descriptor = self.descriptor(request)?;
        let path = match descriptor.binding {
            RepresentationBinding::ProjectResource { .. } => {
                return self.local.availability(request);
            }
            RepresentationBinding::RuntimeResolved { binding_id } => {
                let Some(path) = self.runtime_bindings.get(&binding_id) else {
                    return Ok(RepresentationAvailability::Missing);
                };
                path
            }
        };
        match path.metadata() {
            Ok(metadata) if metadata.is_file() => Ok(RepresentationAvailability::Available),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(RepresentationAvailability::Missing)
            }
            Ok(_) | Err(_) => Ok(RepresentationAvailability::Inaccessible),
        }
    }

    fn materialize(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<photara_core::MaterializedRepresentation, RepresentationMaterializationError> {
        let availability = self.availability(request)?;
        if availability != RepresentationAvailability::Available {
            return Err(RepresentationMaterializationError::Unavailable(
                availability,
            ));
        }
        let Some(path) = self.runtime_path(request)? else {
            return self.local.materialize(request);
        };
        let actual = DiskFolderProvider::verify_revision(
            path,
            self.descriptor(request)?.revision_evidence,
            request.expected_fingerprint,
        )
        .map_err(|message| RepresentationMaterializationError::Backend { message })?;
        if actual != request.expected_fingerprint {
            return Err(RepresentationMaterializationError::SourceChanged {
                expected: request.expected_fingerprint,
                actual,
            });
        }
        let byte_length = path
            .metadata()
            .map_err(|error| RepresentationMaterializationError::Backend {
                message: format!("could not inspect {}: {error}", path.display()),
            })?
            .len();
        Ok(photara_core::MaterializedRepresentation {
            asset_id: request.asset_id,
            representation_id: request.representation_id,
            fingerprint: actual,
            local_path: path.clone(),
            byte_length,
        })
    }
}
