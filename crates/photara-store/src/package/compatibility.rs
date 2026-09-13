//! Explicit source mappings only. These DTOs neither migrate storage nor confer
//! ownership/access. Forward Generation Two storage uses Library nomenclature.
use super::{ObjectRef, PackageError};
use photara_core::contracts::{
    ids::{
        ExternalResourceRefId, ExternalResourceRevisionId, GraphId, LegacyExternalResolverId,
        LibraryId, NodeInstanceId, ProjectId, StorageLocationId,
    },
    schema::Digest,
};
use serde::{Deserialize, Serialize};

/// Historical source identity mapping, with the original UUID retained.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryIdentityMapping {
    pub source: super::PackageUuid,
    pub destination: LibraryId,
}
impl LibraryIdentityMapping {
    /// # Errors
    /// Rejects inconsistent explicit source and destination coordinates.
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.source.as_uuid() != self.destination.uuid() {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationIdentityMapping {
    pub source: super::PackageUuid,
    pub destination: StorageLocationId,
}
impl LocationIdentityMapping {
    /// # Errors
    /// Rejects inconsistent explicit source and destination coordinates.
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.source.as_uuid() != self.destination.uuid() {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
}
/// Origin is historical evidence, never the destination association.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectAssociationMapping {
    pub project_id: ProjectId,
    pub source_object: ObjectRef,
    pub source_digest: Digest,
    pub origin_library_id: Option<LibraryId>,
    pub chosen_library_id: LibraryId,
}
impl ProjectAssociationMapping {
    /// # Errors
    /// Rejects inconsistent explicit source and destination coordinates.
    pub fn validate(
        &self,
        verified_project: ProjectId,
        verified_source: &ObjectRef,
        existing_owner: Option<LibraryId>,
    ) -> Result<(), PackageError> {
        if self.project_id != verified_project
            || &self.source_object != verified_source
            || String::from(self.source_digest) != verified_source.sha256.as_str()
            || existing_owner.is_some_and(|id| id != self.chosen_library_id)
        {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
}
/// A legacy resolver is never a device `HostBindingId`; an unspecified root stays
/// unresolved until an explicit logical mapping is supplied.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExternalResolverMapping {
    Unresolved {
        project_id: ProjectId,
        source_resolver_id: LegacyExternalResolverId,
        source_root_id: Option<super::PackageUuid>,
    },
    Mapped {
        project_id: ProjectId,
        source_resolver_id: LegacyExternalResolverId,
        source_root_id: Option<super::PackageUuid>,
        chosen_location_id: StorageLocationId,
        external_ref_id: ExternalResourceRefId,
        revision_id: ExternalResourceRevisionId,
        revision: ObjectRef,
    },
}
/// Explicit selected source release and snapshot; no Project inventory fallback.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSelectionMapping {
    pub project_id: ProjectId,
    pub graph_id: GraphId,
    pub node_id: NodeInstanceId,
    pub old_pin: photara_core::NodeDefinitionRef,
    pub chosen_pin: photara_core::NodeDefinitionRef,
    pub snapshot: photara_core::contracts::asset_set::AssetSetSnapshot,
    pub output_port: photara_core::contracts::schema::LocalName,
    pub migration_digest: Digest,
}
impl SourceSelectionMapping {
    /// # Errors
    /// Rejects inconsistent explicit source and destination coordinates.
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.snapshot.project_id() != self.project_id || self.old_pin == self.chosen_pin {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
}
/// Presentation remapping applies only to this exact selected definition release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionPresentationMapping {
    pub source_pin: photara_core::NodeDefinitionRef,
    pub source_catalog_path: Vec<String>,
    pub source_contribution: Option<String>,
    pub selected_manifest: ObjectRef,
    pub selected_definition: photara_core::NodeDefinitionRef,
    pub category_id: photara_core::contracts::schema::QualifiedName,
    pub work_surface: Option<photara_core::contracts::schema::QualifiedName>,
}
