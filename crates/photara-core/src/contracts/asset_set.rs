//! Immutable `AssetSet` v2. Membership is explicit; no `ProjectAssetContext` API.
use super::{
    ContractError, Result,
    ids::{AssetId, AssetRepresentationId, AssetSetSnapshotId, ContentRevisionId, ProjectId},
    schema::{DecimalU64, Digest, ObjectKind, ObjectRef, SchemaRef, Version, ordered_unique},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;

pub const MAX_MEMBERS: usize = 10_000;
pub const MAX_PAGE_MEMBERS: usize = 500;
pub const ASSET_SET_VERSION: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationSelection {
    pub project_id: ProjectId,
    pub asset_id: AssetId,
    pub representation_id: AssetRepresentationId,
    pub content_revision_id: ContentRevisionId,
    pub descriptor: ObjectRef,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MetadataTarget {
    Asset,
    Representation {
        representation_id: AssetRepresentationId,
        content_revision_id: ContentRevisionId,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataDependency {
    pub project_id: ProjectId,
    pub asset_id: AssetId,
    pub target: MetadataTarget,
    pub object: ObjectRef,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MissingFact {
    NoUsableRepresentation,
    RepresentationUnavailable {
        representation_id: AssetRepresentationId,
    },
    MetadataUnavailable {
        schema: SchemaRef,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSetMember {
    pub ordinal: u32,
    pub asset_id: AssetId,
    pub representations: Vec<RepresentationSelection>,
    pub metadata: Vec<MetadataDependency>,
    pub missing_facts: Vec<MissingFact>,
}
impl AssetSetMember {
    fn validate(&self, project_id: ProjectId) -> Result<()> {
        let ids: Vec<_> = self
            .representations
            .iter()
            .map(|r| r.representation_id)
            .collect();
        ordered_unique(&ids)?;
        ordered_unique(&self.metadata)?;
        ordered_unique(&self.missing_facts)?;
        if self.representations.is_empty()
            != self
                .missing_facts
                .contains(&MissingFact::NoUsableRepresentation)
        {
            return Err(ContractError::Missing);
        }
        for r in &self.representations {
            if r.project_id != project_id || r.asset_id != self.asset_id {
                return Err(ContractError::Scope);
            }
            if r.descriptor.kind != ObjectKind::Json {
                return Err(ContractError::Union);
            }
        }
        for m in &self.metadata {
            if m.project_id != project_id || m.asset_id != self.asset_id {
                return Err(ContractError::Scope);
            }
            if m.object.kind != ObjectKind::Json {
                return Err(ContractError::Union);
            }
            if let MetadataTarget::Representation {
                representation_id,
                content_revision_id,
            } = m.target
                && !self.representations.iter().any(|r| {
                    r.representation_id == representation_id
                        && r.content_revision_id == content_revision_id
                })
            {
                return Err(ContractError::Scope);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotSpec {
    pub schema_version: Version,
    pub snapshot_id: AssetSetSnapshotId,
    pub project_id: ProjectId,
    pub members: Vec<AssetSetMember>,
}
/// Validated immutable snapshot. Deserialization validates before exposing it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SnapshotSpec", into = "SnapshotSpec")]
pub struct AssetSetSnapshot(SnapshotSpec);
impl TryFrom<SnapshotSpec> for AssetSetSnapshot {
    type Error = ContractError;
    fn try_from(s: SnapshotSpec) -> Result<Self> {
        if s.schema_version != Version::FIRST {
            return Err(ContractError::Unsupported);
        }
        if s.members.len() > MAX_MEMBERS {
            return Err(ContractError::Limit);
        }
        let mut assets = BTreeSet::new();
        let mut representations = BTreeSet::new();
        let mut revisions = BTreeSet::new();
        for (ordinal, m) in s.members.iter().enumerate() {
            if usize::try_from(m.ordinal).map_err(|_| ContractError::Limit)? != ordinal {
                return Err(ContractError::Order);
            }
            if !assets.insert(m.asset_id) {
                return Err(ContractError::Duplicate);
            }
            m.validate(s.project_id)?;
            for r in &m.representations {
                if !representations.insert(r.representation_id)
                    || !revisions.insert(r.content_revision_id)
                {
                    return Err(ContractError::Duplicate);
                }
            }
        }
        Ok(Self(s))
    }
}
impl From<AssetSetSnapshot> for SnapshotSpec {
    fn from(s: AssetSetSnapshot) -> Self {
        s.0
    }
}
impl AssetSetSnapshot {
    /// # Errors
    /// Rejects scope, duplicate/order, dependency and size violations.
    pub fn new(
        snapshot_id: AssetSetSnapshotId,
        project_id: ProjectId,
        members: Vec<AssetSetMember>,
    ) -> Result<Self> {
        SnapshotSpec {
            schema_version: Version::FIRST,
            snapshot_id,
            project_id,
            members,
        }
        .try_into()
    }
    #[must_use]
    pub fn members(&self) -> &[AssetSetMember] {
        &self.0.members
    }
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.0.project_id
    }
    #[must_use]
    pub const fn snapshot_id(&self) -> AssetSetSnapshotId {
        self.0.snapshot_id
    }
    /// Streams the canonical semantic sequence, excluding snapshot ID and pages.
    /// # Errors
    /// Rejects canonical serialization failures.
    pub fn content_digest(&self) -> Result<Digest> {
        let mut hash = Sha256::new();
        hash.update(b"{\"members\":[");
        for (i, m) in self.0.members.iter().enumerate() {
            if i > 0 {
                hash.update(b",");
            }
            hash.update(crate::canonical_json(m).map_err(|_| ContractError::Serialization)?);
        }
        hash.update(b"],\"project_id\":");
        hash.update(
            crate::canonical_json(&self.0.project_id).map_err(|_| ContractError::Serialization)?,
        );
        hash.update(b"}");
        Ok(Digest::from_bytes(hash.finalize().into()))
    }
    /// Only ordered membership, with an explicit domain separate from content.
    /// # Errors
    /// Rejects canonical serialization failures.
    pub fn membership_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        struct Members {
            domain: &'static str,
            project_id: ProjectId,
            assets: Vec<AssetId>,
        }
        Digest::canonical(&Members {
            domain: "photara.asset-set-membership.v2",
            project_id: self.0.project_id,
            assets: self.0.members.iter().map(|m| m.asset_id).collect(),
        })
    }
    /// Complete selected revision/metadata/negative-fact dependencies in member order.
    /// # Errors
    /// Rejects canonical serialization failures.
    pub fn dependency_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        struct Dependencies<'a> {
            domain: &'static str,
            project_id: ProjectId,
            members: &'a [AssetSetMember],
        }
        Digest::canonical(&Dependencies {
            domain: "photara.asset-set-dependencies.v2",
            project_id: self.0.project_id,
            members: &self.0.members,
        })
    }
    /// # Errors
    /// Rejects digest serialization failures.
    pub fn descriptor(&self) -> Result<AssetSetSnapshotDescriptor> {
        Ok(AssetSetSnapshotDescriptor {
            value_type_version: ASSET_SET_VERSION,
            snapshot_id: self.0.snapshot_id,
            project_id: self.0.project_id,
            member_count: u32::try_from(self.0.members.len()).map_err(|_| ContractError::Limit)?,
            content_digest: self.content_digest()?,
            membership_digest: self.membership_digest()?,
            dependency_digest: self.dependency_digest()?,
        })
    }
    /// Builds immutable DTO pages and their exact Rust-generated `ObjectRefs`.
    /// # Errors
    /// Rejects zero/oversized pages or canonical serialization failures.
    pub fn pages(&self, page_size: usize) -> Result<Vec<AssetSetPage>> {
        if !(1..=MAX_PAGE_MEMBERS).contains(&page_size) {
            return Err(ContractError::Limit);
        }
        let descriptor = self.descriptor()?;
        self.0
            .members
            .chunks(page_size)
            .enumerate()
            .map(|(i, members)| {
                Ok(AssetSetPage {
                    token: AssetSetPageToken {
                        project_id: self.project_id(),
                        snapshot_id: self.snapshot_id(),
                        content_digest: descriptor.content_digest,
                        start_ordinal: u32::try_from(i * page_size)
                            .map_err(|_| ContractError::Limit)?,
                        count: u32::try_from(members.len()).map_err(|_| ContractError::Limit)?,
                    },
                    members: members.to_vec(),
                })
            })
            .collect()
    }
    /// Reassembles verified page bytes without any package I/O or ambient ledger.
    /// # Errors
    /// Rejects mismatched `ObjectRefs`, missing/extra/reordered pages or dependencies.
    pub fn from_pages(
        descriptor: &AssetSetSnapshotDescriptor,
        pages: &[(ObjectRef, AssetSetPage)],
    ) -> Result<Self> {
        descriptor.validate()?;
        let mut members = Vec::new();
        for (reference, page) in pages {
            page.validate()?;
            if page.object_ref()? != *reference {
                return Err(ContractError::Digest);
            }
            let t = &page.token;
            if t.project_id != descriptor.project_id
                || t.snapshot_id != descriptor.snapshot_id
                || t.content_digest != descriptor.content_digest
            {
                return Err(ContractError::Scope);
            }
            if usize::try_from(t.start_ordinal).map_err(|_| ContractError::Limit)? != members.len()
            {
                return Err(ContractError::Order);
            }
            if members.len() + page.members.len() > MAX_MEMBERS {
                return Err(ContractError::Limit);
            }
            members.extend(page.members.clone());
        }
        let snapshot = Self::new(descriptor.snapshot_id, descriptor.project_id, members)?;
        if snapshot.descriptor()? != *descriptor {
            return Err(ContractError::Digest);
        }
        Ok(snapshot)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSetSnapshotDescriptor {
    pub value_type_version: u32,
    pub project_id: ProjectId,
    pub snapshot_id: AssetSetSnapshotId,
    pub member_count: u32,
    pub content_digest: Digest,
    pub membership_digest: Digest,
    pub dependency_digest: Digest,
}
impl AssetSetSnapshotDescriptor {
    /// # Errors
    /// Rejects old/unknown versions or truncated/oversized membership claims.
    pub fn validate(&self) -> Result<()> {
        if self.value_type_version != ASSET_SET_VERSION {
            return Err(ContractError::Unsupported);
        }
        if u64::from(self.member_count) > MAX_MEMBERS as u64 {
            return Err(ContractError::Limit);
        }
        Ok(())
    }
    /// An additive typed value; v1 decoding rejects its exact version.
    /// # Errors
    /// Rejects invalid descriptor or JSON serialization.
    pub fn to_typed_value(&self) -> Result<crate::TypedValue> {
        self.validate()?;
        Ok(crate::TypedValue {
            value_type: crate::ValueTypeRef {
                id: crate::ValueTypeId::parse("photara.asset-set")
                    .map_err(|_| ContractError::Coordinate)?,
                version: crate::ValueTypeVersion::new(2).map_err(|_| ContractError::Version)?,
            },
            value: serde_json::to_value(self).map_err(|_| ContractError::Serialization)?,
        })
    }
    /// # Errors
    /// Rejects v1/unknown types and invalid v2 descriptors.
    pub fn from_typed_value(value: &crate::TypedValue) -> Result<Self> {
        if value.value_type.id.as_str() != "photara.asset-set"
            || value.value_type.version.get() != 2
        {
            return Err(ContractError::Unsupported);
        }
        let d: Self =
            serde_json::from_value(value.value.clone()).map_err(|_| ContractError::Canonical)?;
        d.validate()?;
        Ok(d)
    }
}
/// A scope/digest-bound page coordinate, not a signed cursor or access credential.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSetPageToken {
    pub project_id: ProjectId,
    pub snapshot_id: AssetSetSnapshotId,
    pub content_digest: Digest,
    pub start_ordinal: u32,
    pub count: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSetPage {
    pub token: AssetSetPageToken,
    pub members: Vec<AssetSetMember>,
}
impl AssetSetPage {
    /// # Errors
    /// Rejects empty/oversized pages, ordinal gaps and inconsistent scope.
    pub fn validate(&self) -> Result<()> {
        if self.members.is_empty()
            || self.members.len() > MAX_PAGE_MEMBERS
            || u64::from(self.token.count) != self.members.len() as u64
            || u64::from(self.token.start_ordinal) + u64::from(self.token.count)
                > MAX_MEMBERS as u64
        {
            return Err(ContractError::Limit);
        }
        let mut seen = BTreeSet::new();
        for (i, m) in self.members.iter().enumerate() {
            if u64::from(m.ordinal) != u64::from(self.token.start_ordinal) + i as u64 {
                return Err(ContractError::Order);
            }
            if !seen.insert(m.asset_id) {
                return Err(ContractError::Duplicate);
            }
            m.validate(self.token.project_id)?;
        }
        Ok(())
    }
    /// # Errors
    /// Rejects invalid pages or canonical serialization failures.
    pub fn object_ref(&self) -> Result<ObjectRef> {
        self.validate()?;
        let bytes = crate::canonical_json(self).map_err(|_| ContractError::Serialization)?;
        Ok(ObjectRef {
            kind: ObjectKind::Json,
            sha256: Digest::of_bytes(&bytes),
            byte_length: DecimalU64::new(bytes.len() as u64),
        })
    }
}
