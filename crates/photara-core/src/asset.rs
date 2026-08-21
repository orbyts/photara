use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    AssetId, AssetRepresentationId, ProjectResourceId, RepresentationCapabilityId,
    RepresentationRoleId, RepresentationStorageBindingId, SchemaId, SchemaRef, SchemaVersion,
    TypedValue, ValueTypeDescriptor, ValueTypeId, ValueTypeRef, ValueTypeVersion,
};

pub const ASSET_SET_VALUE_TYPE_ID: &str = "photara.asset-set";
pub const ASSET_SET_SCHEMA_ID: &str = "photara.asset-set.value";
pub const ORIGINAL_REPRESENTATION_ROLE_ID: &str = "photara.representation.original";
pub const RAW_PREVIEW_REPRESENTATION_ROLE_ID: &str = "photara.representation.raw-preview";
pub const LAYERED_MASTER_REPRESENTATION_ROLE_ID: &str = "photara.representation.layered-master";
pub const HDR_REPRESENTATION_ROLE_ID: &str = "photara.representation.hdr-rendition";
pub const SDR_REPRESENTATION_ROLE_ID: &str = "photara.representation.sdr-rendition";
pub const IMAGE_CAPABILITY_ID: &str = "photara.media.image";
pub const TIFF_CAPABILITY_ID: &str = "photara.format.tiff";
pub const FLATTENED_IMAGE_CAPABILITY_ID: &str = "photara.image.flattened";
pub const HDR_CAPABILITY_ID: &str = "photara.dynamic-range.hdr";
pub const SDR_CAPABILITY_ID: &str = "photara.dynamic-range.sdr";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FingerprintAlgorithm {
    Sha256,
}

/// Immutable content identity for one representation revision.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RepresentationFingerprint {
    pub algorithm: FingerprintAlgorithm,
    pub digest: [u8; 32],
}

impl RepresentationFingerprint {
    #[must_use]
    pub const fn sha256(bytes: [u8; 32]) -> Self {
        Self {
            algorithm: FingerprintAlgorithm::Sha256,
            digest: bytes,
        }
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Portable binding from a semantic representation to either project-owned
/// storage or a stable handle whose machine/provider locator is runtime-only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RepresentationBinding {
    ProjectResource {
        resource_id: ProjectResourceId,
    },
    RuntimeResolved {
        binding_id: RepresentationStorageBindingId,
    },
}

/// Portable semantic description of one rendition of an asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepresentationDescriptor {
    pub id: AssetRepresentationId,
    pub role: RepresentationRoleId,
    pub fingerprint: RepresentationFingerprint,
    pub capabilities: BTreeSet<RepresentationCapabilityId>,
    pub binding: RepresentationBinding,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

/// One semantic asset with zero or more related representations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectAsset {
    pub id: AssetId,
    pub display_name: String,
    #[serde(default)]
    pub representations: Vec<RepresentationDescriptor>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

/// Portable project-owned asset inventory.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectAssetContext {
    #[serde(default)]
    pub assets: Vec<ProjectAsset>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl ProjectAssetContext {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty() && self.extensions.is_empty()
    }

    #[must_use]
    pub fn asset(&self, id: AssetId) -> Option<&ProjectAsset> {
        self.assets.iter().find(|asset| asset.id == id)
    }

    #[must_use]
    pub fn asset_mut(&mut self, id: AssetId) -> Option<&mut ProjectAsset> {
        self.assets.iter_mut().find(|asset| asset.id == id)
    }

    #[must_use]
    pub fn representation(
        &self,
        asset_id: AssetId,
        representation_id: AssetRepresentationId,
    ) -> Option<&RepresentationDescriptor> {
        self.asset(asset_id)?
            .representations
            .iter()
            .find(|representation| representation.id == representation_id)
    }

    /// Validates semantic identities, project-resource bindings, and portable
    /// extension state.
    ///
    /// # Errors
    ///
    /// Returns [`AssetContextError`] for duplicate identities, empty names,
    /// missing resource bindings, or excluded cache/runtime/UI state.
    pub fn validate(
        &self,
        resource_ids: &BTreeSet<ProjectResourceId>,
    ) -> Result<(), AssetContextError> {
        validate_extensions(&self.extensions)?;
        let mut asset_ids = BTreeSet::new();
        let mut representation_ids = BTreeSet::new();
        for asset in &self.assets {
            if !asset_ids.insert(asset.id) {
                return Err(AssetContextError::DuplicateAsset(asset.id));
            }
            if asset.display_name.trim().is_empty() {
                return Err(AssetContextError::EmptyAssetDisplayName(asset.id));
            }
            validate_extensions(&asset.extensions)?;
            for representation in &asset.representations {
                if !representation_ids.insert(representation.id) {
                    return Err(AssetContextError::DuplicateRepresentation(
                        representation.id,
                    ));
                }
                validate_extensions(&representation.extensions)?;
                if let RepresentationBinding::ProjectResource { resource_id } =
                    representation.binding
                    && !resource_ids.contains(&resource_id)
                {
                    return Err(AssetContextError::MissingProjectResource {
                        asset_id: asset.id,
                        representation_id: representation.id,
                        resource_id,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Explicit ordered membership passed through graph ports.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AssetSet {
    pub assets: Vec<AssetId>,
}

impl AssetSet {
    /// Validates duplicate-free membership against a project asset context.
    ///
    /// # Errors
    ///
    /// Returns [`AssetSetValueError`] for duplicate or unknown asset IDs.
    pub fn validate(&self, context: &ProjectAssetContext) -> Result<(), AssetSetValueError> {
        let mut unique = BTreeSet::new();
        for asset_id in &self.assets {
            if !unique.insert(*asset_id) {
                return Err(AssetSetValueError::DuplicateAsset(*asset_id));
            }
            if context.asset(*asset_id).is_none() {
                return Err(AssetSetValueError::UnknownAsset(*asset_id));
            }
        }
        Ok(())
    }

    /// Encodes the set as the exact general typed value consumed by Layout and
    /// future nodes.
    ///
    /// # Errors
    ///
    /// Returns a JSON error if serialization unexpectedly fails.
    pub fn to_typed_value(&self) -> Result<TypedValue, serde_json::Error> {
        Ok(TypedValue {
            value_type: asset_set_value_type_ref(),
            value: serde_json::to_value(self)?,
        })
    }

    /// Decodes an exact `photara.asset-set` typed value.
    ///
    /// # Errors
    ///
    /// Returns [`AssetSetValueError`] when the value type or payload is invalid.
    pub fn from_typed_value(value: &TypedValue) -> Result<Self, AssetSetValueError> {
        let expected = asset_set_value_type_ref();
        if value.value_type != expected {
            return Err(AssetSetValueError::WrongValueType {
                expected,
                actual: value.value_type.clone(),
            });
        }
        let asset_set: Self = serde_json::from_value(value.value.clone())
            .map_err(AssetSetValueError::InvalidPayload)?;
        let mut unique = BTreeSet::new();
        for asset_id in &asset_set.assets {
            if !unique.insert(*asset_id) {
                return Err(AssetSetValueError::DuplicateAsset(*asset_id));
            }
        }
        Ok(asset_set)
    }
}

#[must_use]
/// Returns the exact built-in `AssetSet` value-type coordinate.
///
/// # Panics
///
/// Panics only if Photara's compile-time canonical ID is invalid.
pub fn asset_set_value_type_ref() -> ValueTypeRef {
    ValueTypeRef {
        id: ValueTypeId::parse(ASSET_SET_VALUE_TYPE_ID).expect("built-in value type ID is valid"),
        version: ValueTypeVersion::first(),
    }
}

#[must_use]
/// Returns the built-in `AssetSet` descriptor and payload schema coordinate.
///
/// # Panics
///
/// Panics only if Photara's compile-time canonical IDs are invalid.
pub fn asset_set_value_type_descriptor() -> ValueTypeDescriptor {
    ValueTypeDescriptor {
        value_type: asset_set_value_type_ref(),
        display_name: "Asset Set".to_owned(),
        schema: SchemaRef {
            id: SchemaId::parse(ASSET_SET_SCHEMA_ID).expect("built-in schema ID is valid"),
            version: SchemaVersion::first(),
        },
    }
}

/// Runtime availability, intentionally absent from portable project JSON.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepresentationAvailability {
    Available,
    Missing,
    Inaccessible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepresentationMaterializationRequest {
    pub asset_id: AssetId,
    pub representation_id: AssetRepresentationId,
    pub expected_fingerprint: RepresentationFingerprint,
}

/// Runtime local materialization. Machine paths never enter portable documents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedRepresentation {
    pub asset_id: AssetId,
    pub representation_id: AssetRepresentationId,
    pub fingerprint: RepresentationFingerprint,
    pub local_path: PathBuf,
    pub byte_length: u64,
}

pub trait RepresentationMaterializer {
    /// Resolves current runtime availability without changing project semantics.
    ///
    /// # Errors
    ///
    /// Returns [`RepresentationMaterializationError`] for unknown identities.
    fn availability(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<RepresentationAvailability, RepresentationMaterializationError>;

    /// Materializes the exact requested fingerprint to a verified local file.
    ///
    /// # Errors
    ///
    /// Returns [`RepresentationMaterializationError`] when identities are
    /// unknown, the source is unavailable, or its content has changed.
    fn materialize(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<MaterializedRepresentation, RepresentationMaterializationError>;
}

fn validate_extensions(extensions: &BTreeMap<String, Value>) -> Result<(), AssetContextError> {
    const EXCLUDED: &[&str] = &[
        "runtime",
        "availability",
        "materialization",
        "proxy",
        "proxies",
        "thumbnail",
        "preview",
        "cache",
        "caches",
        "credentials",
        "secrets",
        "workspace",
        "gallery_selection",
    ];
    if let Some(key) = extensions
        .keys()
        .find(|key| EXCLUDED.contains(&key.as_str()))
    {
        return Err(AssetContextError::ExcludedPortableField(key.clone()));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AssetContextError {
    #[error("duplicate project asset {0}")]
    DuplicateAsset(AssetId),
    #[error("project asset {0} has an empty display name")]
    EmptyAssetDisplayName(AssetId),
    #[error("duplicate asset representation {0}")]
    DuplicateRepresentation(AssetRepresentationId),
    #[error(
        "asset {asset_id} representation {representation_id} references missing project resource {resource_id}"
    )]
    MissingProjectResource {
        asset_id: AssetId,
        representation_id: AssetRepresentationId,
        resource_id: ProjectResourceId,
    },
    #[error("portable asset context must not contain derived/runtime field {0:?}")]
    ExcludedPortableField(String),
}

#[derive(Debug, Error)]
pub enum AssetSetValueError {
    #[error("asset set contains duplicate asset {0}")]
    DuplicateAsset(AssetId),
    #[error("asset set references unknown asset {0}")]
    UnknownAsset(AssetId),
    #[error("asset set expected value type {expected:?}, got {actual:?}")]
    WrongValueType {
        expected: ValueTypeRef,
        actual: ValueTypeRef,
    },
    #[error("invalid asset-set payload: {0}")]
    InvalidPayload(serde_json::Error),
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum RepresentationMaterializationError {
    #[error("project asset {0} does not exist")]
    UnknownAsset(AssetId),
    #[error("asset {asset_id} representation {representation_id} does not exist")]
    UnknownRepresentation {
        asset_id: AssetId,
        representation_id: AssetRepresentationId,
    },
    #[error("representation request fingerprint is stale")]
    StaleRequest {
        expected: RepresentationFingerprint,
        actual: RepresentationFingerprint,
    },
    #[error("representation is not currently available: {0:?}")]
    Unavailable(RepresentationAvailability),
    #[error("representation content changed since it was described")]
    SourceChanged {
        expected: RepresentationFingerprint,
        actual: RepresentationFingerprint,
    },
    #[error("representation materialization failed: {message}")]
    Backend { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: AssetId, resource_id: ProjectResourceId) -> ProjectAsset {
        ProjectAsset {
            id,
            display_name: "Test Asset".to_owned(),
            representations: vec![RepresentationDescriptor {
                id: AssetRepresentationId::new(),
                role: RepresentationRoleId::parse("example.rendition.original").unwrap(),
                fingerprint: RepresentationFingerprint::sha256([7; 32]),
                capabilities: BTreeSet::new(),
                binding: RepresentationBinding::ProjectResource { resource_id },
                extensions: BTreeMap::new(),
            }],
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn asset_set_typed_value_preserves_order_and_rejects_duplicates() {
        let first = AssetId::new();
        let second = AssetId::new();
        let first_resource = ProjectResourceId::new();
        let second_resource = ProjectResourceId::new();
        let context = ProjectAssetContext {
            assets: vec![asset(first, first_resource), asset(second, second_resource)],
            extensions: BTreeMap::new(),
        };
        let set = AssetSet {
            assets: vec![second, first],
        };
        set.validate(&context).unwrap();
        let typed = set.to_typed_value().unwrap();
        assert_eq!(AssetSet::from_typed_value(&typed).unwrap(), set);
        assert!(matches!(
            AssetSet {
                assets: vec![first, first]
            }
            .validate(&context),
            Err(AssetSetValueError::DuplicateAsset(id)) if id == first
        ));

        let mut with_proxy = context;
        with_proxy
            .extensions
            .insert("proxy".to_owned(), serde_json::json!({"derived": true}));
        assert!(matches!(
            with_proxy.validate(&BTreeSet::from([first_resource, second_resource])),
            Err(AssetContextError::ExcludedPortableField(field)) if field == "proxy"
        ));
    }

    #[test]
    fn runtime_resolved_binding_is_portable_without_a_machine_locator() {
        let asset_id = AssetId::new();
        let context = ProjectAssetContext {
            assets: vec![ProjectAsset {
                id: asset_id,
                display_name: "Provider-owned RAW".to_owned(),
                representations: vec![RepresentationDescriptor {
                    id: AssetRepresentationId::new(),
                    role: RepresentationRoleId::parse(ORIGINAL_REPRESENTATION_ROLE_ID).unwrap(),
                    fingerprint: RepresentationFingerprint::sha256([11; 32]),
                    capabilities: BTreeSet::new(),
                    binding: RepresentationBinding::RuntimeResolved {
                        binding_id: RepresentationStorageBindingId::new(),
                    },
                    extensions: BTreeMap::new(),
                }],
                extensions: BTreeMap::new(),
            }],
            extensions: BTreeMap::new(),
        };

        context.validate(&BTreeSet::new()).unwrap();
        let json = serde_json::to_string(&context).unwrap();
        assert!(!json.contains('/'));
        assert!(!json.contains("credentials"));
        assert_eq!(
            serde_json::from_str::<ProjectAssetContext>(&json).unwrap(),
            context
        );
    }
}
