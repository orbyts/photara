//! Typed local-only records. These are not S5 wire envelopes.
use super::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

macro_rules! ids {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(try_from = "Uuid", into = "Uuid")]
        pub struct $name(Uuid);
        impl $name {
            #[must_use] pub fn new() -> Self { Self(Uuid::new_v4()) }
            #[must_use] pub const fn uuid(self) -> Uuid { self.0 }
            pub(super) fn bytes(self) -> Vec<u8> { self.0.as_bytes().to_vec() }
            /// Decodes a non-nil UUID from its exact 16-byte storage form.
            /// # Errors
            /// Rejects malformed length and nil identity.
            pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
                Self::try_from(Uuid::from_slice(bytes).map_err(|_| Error::Corrupt)?)
            }
        }
        impl Default for $name { fn default() -> Self { Self::new() } }
        impl TryFrom<Uuid> for $name {
            type Error = Error;
            fn try_from(value: Uuid) -> Result<Self> {
                if value.is_nil() { Err(Error::Invalid) } else { Ok(Self(value)) }
            }
        }
        impl From<$name> for Uuid { fn from(value: $name) -> Self { value.0 } }
    )+};
}
ids!(
    LibraryId,
    PersonId,
    OrganizationId,
    RelationshipId,
    LocationKindId,
    LocationId,
    SocialProfileId,
    StorageRootId,
    ProjectId,
    LocatorId,
    DeviceId,
    ObservationId,
    SecureHandleId,
    MutationId,
    DatabaseId,
    ScopedChannelId,
    CommitId
);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Revision(i64);
impl Revision {
    pub const INITIAL: Self = Self(1);
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
    /// # Errors
    /// Fails rather than overflowing the signed `SQLite` revision range.
    pub fn next(self) -> Result<Self> {
        Self::try_from(self.0.checked_add(1).ok_or(Error::Limit)?)
    }
}
impl TryFrom<i64> for Revision {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err(Error::Invalid)
        }
    }
}
impl From<Revision> for i64 {
    fn from(value: Revision) -> Self {
        value.0
    }
}

/// UTC Unix milliseconds; this initial API accepts the RFC3339 year 0001–9999 range.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Timestamp(i64);
impl Timestamp {
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl TryFrom<i64> for Timestamp {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        if (-62_135_596_800_000..=253_402_300_799_999).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::Invalid)
        }
    }
}
impl From<Timestamp> for i64 {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lifecycle {
    Active,
    Tombstoned,
}
impl Lifecycle {
    pub(super) const fn sql(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Tombstoned => "tombstoned",
        }
    }
    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "tombstoned" => Ok(Self::Tombstoned),
            _ => Err(Error::Unsupported),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Metadata<I> {
    pub id: I,
    pub library_id: LibraryId,
    pub revision: Revision,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub state: Lifecycle,
}
impl<I> Metadata<I> {
    #[must_use]
    pub const fn new(id: I, library_id: LibraryId, at: Timestamp) -> Self {
        Self {
            id,
            library_id,
            revision: Revision::INITIAL,
            created_at: at,
            updated_at: at,
            state: Lifecycle::Active,
        }
    }
    /// Advances a caller's edit token; persistence still requires exact CAS.
    /// # Errors
    /// Rejects retired records and revision exhaustion.
    pub fn advance(&mut self, at: Timestamp) -> Result<Revision> {
        if self.state != Lifecycle::Active {
            return Err(Error::Retired);
        }
        let expected = self.revision;
        self.revision = self.revision.next()?;
        self.updated_at = at;
        Ok(expected)
    }
    /// # Errors
    /// Rejects repeated retirement and revision exhaustion.
    pub fn tombstone(&mut self, at: Timestamp) -> Result<Revision> {
        let expected = self.advance(at)?;
        self.state = Lifecycle::Tombstoned;
        Ok(expected)
    }
    pub(super) fn retired_at(&self) -> Option<i64> {
        (self.state == Lifecycle::Tombstoned).then_some(self.updated_at.get())
    }
}

pub type Extensions = BTreeMap<String, serde_json::Value>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub meta: Metadata<LibraryId>,
    pub display_name: String,
    pub extensions: Extensions,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartyDetails {
    pub display_name: String,
    pub description: String,
    pub aliases: BTreeSet<String>,
    pub labels: BTreeSet<String>,
    pub extensions: Extensions,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub meta: Metadata<PersonId>,
    pub details: PartyDetails,
    pub capabilities: BTreeSet<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub meta: Metadata<OrganizationId>,
    pub details: PartyDetails,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub meta: Metadata<RelationshipId>,
    pub person_id: PersonId,
    pub organization_id: OrganizationId,
    pub relationship_type: String,
    pub valid_from: Option<Timestamp>,
    pub valid_until: Option<Timestamp>,
    pub notes: String,
    pub labels: BTreeSet<String>,
    pub extensions: Extensions,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocationKind {
    pub meta: Metadata<LocationKindId>,
    pub canonical_display: String,
    pub description: String,
    /// Additive claims: omitted old terms remain reserved and are returned on read.
    pub aliases: BTreeSet<String>,
    pub extensions: Extensions,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub meta: Metadata<LocationId>,
    pub kind_id: LocationKindId,
    pub parent_id: Option<LocationId>,
    pub details: PartyDetails,
    pub address: Address,
    pub coordinates: Option<Coordinates>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Address {
    pub lines: Vec<String>,
    pub locality: String,
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

macro_rules! vocabulary {
    ($name:ident {$($variant:ident => $value:literal),+ $(,)?}) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename=$value)] $variant),+ }
        impl $name {
            pub(super) const fn sql(self)-> &'static str {match self {$(Self::$variant=>$value),+}}
            pub(super) fn parse(value:&str)->Result<Self> {match value {$($value=>Ok(Self::$variant)),+,_=>Err(Error::Unsupported)}}
        }
    };
}
vocabulary!(AccountKind {Unknown=>"unknown",Personal=>"personal",Creator=>"creator",Business=>"business",Organization=>"organization",Service=>"service",Other=>"other"});
vocabulary!(FetchState {Never=>"never",Available=>"available",Unavailable=>"unavailable",Revoked=>"revoked",Expired=>"expired",Error=>"error"});
vocabulary!(VerificationKind {Unverified=>"unverified",UserAsserted=>"user-asserted",ProviderAuthorized=>"provider-authorized"});
vocabulary!(Visibility {Visible=>"visible",Hidden=>"hidden"});

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum SocialOwner {
    Person(PersonId),
    Organization(OrganizationId),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderSubject {
    pub namespace: String,
    pub subject_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocialProvenance {
    pub source: String,
    pub notes: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocialProfile {
    pub meta: Metadata<SocialProfileId>,
    pub owner: SocialOwner,
    pub provider_id: String,
    pub subject: Option<ProviderSubject>,
    pub handle: Option<String>,
    pub display_name: String,
    pub profile_url: Option<String>,
    pub account_kind: AccountKind,
    pub provider_account_kind: Option<String>,
    pub verification: VerificationKind,
    pub verified_at: Option<Timestamp>,
    pub provenance: SocialProvenance,
    pub fetched_at: Option<Timestamp>,
    pub refreshed_at: Option<Timestamp>,
    pub next_refresh_after: Option<Timestamp>,
    pub fetch_state: FetchState,
    pub extensions: Extensions,
}
/// Manual handle matches are warnings, never automatic identity merges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteOutcome {
    pub mutation_id: MutationId,
    pub duplicate_manual_handles: Vec<SocialProfileId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StorageRoot {
    pub meta: Metadata<StorageRootId>,
    pub display_name: String,
    pub purpose: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub meta: Metadata<ProjectId>,
    pub visibility: Visibility,
    /// Device-local selection, never a cloud mutation payload.
    pub active_locator_id: Option<LocatorId>,
    pub selected_observation_id: Option<ObservationId>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RootedPath {
    pub storage_root_id: StorageRootId,
    pub relative_path: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectLocator {
    pub meta: Metadata<LocatorId>,
    pub project_id: ProjectId,
    pub rooted: Option<RootedPath>,
}

/// Secure references only, never bookmark bytes or provider credentials.
#[derive(Clone, Eq, PartialEq)]
pub enum DeviceBinding {
    Path(std::path::PathBuf),
    Bookmark(SecureHandleId),
    Provider(SecureHandleId),
}
impl std::fmt::Debug for DeviceBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceBinding([REDACTED])")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootBinding {
    pub library_id: LibraryId,
    pub storage_root_id: StorageRootId,
    pub binding: DeviceBinding,
    pub revision: Revision,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreInfo {
    pub database_id: DatabaseId,
    pub device_id: DeviceId,
    pub sqlite_version: String,
    pub migration_count: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreStats {
    pub closed: bool,
    pub connections: u32,
    pub idle: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalChange {
    pub sequence: i64,
    pub mutation_id: MutationId,
    pub entity_kind: String,
    pub entity_id: Uuid,
    pub revision: Revision,
    pub change_kind: String,
    pub post_state: serde_json::Value,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectObservation {
    pub id: ObservationId,
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub locator_id: LocatorId,
    pub commit_id: CommitId,
    pub commit_sha256: [u8; 32],
    pub package_revision: u64,
    pub title: String,
    pub lifecycle: String,
    pub asset_count: u64,
    pub graph_count: u64,
    pub observed_at: Timestamp,
}
