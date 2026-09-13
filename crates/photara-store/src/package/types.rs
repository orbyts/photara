//! Versioned envelope types. Unknown fields remain lossless in `extra` maps.

use super::PackageError;
use photara_core::{GraphDocument, ProjectId};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub type ExtraFields = BTreeMap<String, Value>;

/// Canonically encoded UUID for package-level identities not yet Core records.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PackageUuid(Uuid);

impl PackageUuid {
    /// # Errors
    /// Rejects noncanonical UUID spelling, including uppercase and compact forms.
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        let id = Uuid::parse_str(value).map_err(|_| PackageError::Record)?;
        if id.to_string() != value {
            return Err(PackageError::Record);
        }
        Ok(Self(id))
    }
    #[must_use]
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for PackageUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl<'de> Deserialize<'de> for PackageUuid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Lowercase hex SHA-256 identity. Construction validates all 64 digits.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Sha256Hex(String);
impl Sha256Hex {
    /// # Errors
    /// Rejects malformed or noncanonical digests.
    pub fn parse(s: &str) -> Result<Self, PackageError> {
        if s.len() != 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(PackageError::Record);
        }
        Ok(Self(s.to_owned()))
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for Sha256Hex {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// Canonical unsigned decimal string; not a JSON floating-point counter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DecimalU64(u64);
impl DecimalU64 {
    /// # Errors
    /// Rejects numbers, signs, leading zeros, whitespace and u64 overflow.
    pub fn parse(s: &str) -> Result<Self, PackageError> {
        if s.is_empty()
            || (s.len() > 1 && s.starts_with('0'))
            || !s.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(PackageError::Record);
        }
        Ok(Self(s.parse().map_err(|_| PackageError::Record)?))
    }
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}
impl Serialize for DecimalU64 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKind {
    Blob,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub kind: ObjectKind,
    pub sha256: Sha256Hex,
    pub byte_length: DecimalU64,
}
impl ObjectRef {
    #[must_use]
    pub fn path(&self) -> String {
        match self.kind {
            ObjectKind::Json => format!("objects/json/sha256/{}.json", self.sha256.as_str()),
            ObjectKind::Blob => format!("objects/blobs/sha256/{}", self.sha256.as_str()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatVersion {
    pub major: u32,
    pub minor: u32,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordSchema {
    pub id: String,
    pub version: u32,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bootstrap {
    pub format: String,
    pub format_version: FormatVersion,
    pub project_id: ProjectId,
    pub created_at: String,
    pub canonical_json: String,
    pub required_features: Vec<String>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Head {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    pub commit_id: PackageUuid,
    pub commit_sha256: Sha256Hex,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitParent {
    pub commit_id: PackageUuid,
    pub sha256: Sha256Hex,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commit {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    pub commit_id: PackageUuid,
    pub package_revision: DecimalU64,
    pub parent: Option<CommitParent>,
    pub write_id: PackageUuid,
    pub created_at: String,
    pub bootstrap_sha256: Sha256Hex,
    pub minimum_reader: FormatVersion,
    pub required_features: Vec<String>,
    pub authored: ObjectRef,
    pub history: ObjectRef,
    pub inventory: ObjectRef,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    pub objects: Vec<ObjectRef>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedGraphRef {
    pub graph_id: PackageUuid,
    pub document: ObjectRef,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthoredProject {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    pub originating_library_id: Option<PackageUuid>,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub description: String,
    pub lifecycle: String,
    pub authored_revision: DecimalU64,
    pub party_assignments: ObjectRef,
    pub location_assignments: ObjectRef,
    pub asset_inventory: ObjectRef,
    pub resource_inventory: ObjectRef,
    pub graphs: Vec<NamedGraphRef>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequiredPackage {
    pub package_id: photara_core::NodePackageId,
    pub package_version: photara_core::PackageVersion,
    pub manifest: Option<ObjectRef>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedGraph {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    pub graph_id: PackageUuid,
    pub name: String,
    pub name_normalization_version: u32,
    pub metadata_revision: DecimalU64,
    pub created_at: String,
    pub updated_at: String,
    pub required_packages: Vec<RequiredPackage>,
    pub graph: GraphDocument,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Typed common envelope for Library-context, asset/resource and history records.
/// Per-schema fields are validated by the package record validator; the original
/// canonical bytes and every optional field remain available on the verified object.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnedRecord {
    pub schema: RecordSchema,
    pub project_id: ProjectId,
    #[serde(flatten)]
    pub fields: ExtraFields,
}
