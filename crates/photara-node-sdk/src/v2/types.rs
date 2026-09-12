use photara_core::{
    PackageVersion,
    contracts::{
        access::ActionMask,
        resource::{HostKind, HostPlace, RelativeComponents, ResourceRights, StorageClass},
        schema::{
            DecimalU64, Digest, LocalName, QualifiedName, SchemaRef, ValueFamily, ValueShape,
            ValueTypeRef, Version,
        },
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionCoordinate {
    pub package_id: QualifiedName,
    pub package_version: PackageVersion,
    pub definition_id: QualifiedName,
    pub definition_version: Version,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequiredFeature {
    TypedPorts,
    ContextDeclarations,
    ResourceDescriptors,
    AssetSetV2,
    Inspector,
    WorkSurface,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortDirection {
    Input,
    Output,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortCardinality {
    One,
    Optional,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortContract {
    pub id: LocalName,
    pub direction: PortDirection,
    pub cardinality: PortCardinality,
    pub value_type: ValueTypeRef,
    pub schema: SchemaRef,
    pub family: ValueFamily,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldMode {
    LiteralOnly,
    Expression,
    Template,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoredVisibility {
    Public,
    Private,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldContract {
    pub name: LocalName,
    pub mode: FieldMode,
    pub visibility: AuthoredVisibility,
    pub shape: ValueShape,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredSchema {
    pub schema: SchemaRef,
    pub fields: Vec<FieldContract>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SelectorScope {
    Library,
    Project,
    Graph,
    Node,
    InputPort { port_id: LocalName },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessMode {
    Captured,
    Live,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CapabilityRequirement {
    Selector {
        slot: LocalName,
        required: bool,
        scope: SelectorScope,
        access: AccessMode,
        actions: ActionMask,
        value_type: ValueTypeRef,
        fields: Vec<LocalName>,
        max_items: u32,
    },
    Resource {
        slot: LocalName,
        required: bool,
        access: AccessMode,
        storage_class: StorageClass,
        rights: ResourceRights,
        subtree: Option<RelativeComponents>,
        max_items: u32,
        max_bytes: DecimalU64,
        operations: Vec<LocalName>,
    },
    Place {
        slot: LocalName,
        required: bool,
        access: AccessMode,
        place: HostPlace,
        rights: ResourceRights,
        subtree: Option<RelativeComponents>,
        max_bytes: DecimalU64,
        operations: Vec<LocalName>,
    },
    Credential {
        slot: LocalName,
        required: bool,
        provider_id: QualifiedName,
        operations: Vec<LocalName>,
    },
}
impl CapabilityRequirement {
    #[must_use]
    pub const fn slot(&self) -> &LocalName {
        match self {
            Self::Selector { slot, .. }
            | Self::Resource { slot, .. }
            | Self::Place { slot, .. }
            | Self::Credential { slot, .. } => slot,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectOperation {
    pub id: LocalName,
    pub kind: QualifiedName,
    pub target_slot: LocalName,
    pub credential_slot: Option<LocalName>,
    pub rights: ResourceRights,
    pub request_schema: SchemaRef,
    pub receipt_schema: SchemaRef,
    pub replacement_supported: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExecutionContract {
    Pure,
    Read,
    Effect { operations: Vec<EffectOperation> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Determinism {
    Deterministic,
    Captured { facts: Vec<LocalName> },
    NonDeterministic,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheScope {
    None,
    Device,
    Project,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheContract {
    pub scope: CacheScope,
    pub evaluation_key_version: Version,
    pub verified_capture_required: bool,
}
/// Opaque source coordinates only. Binding/capture/AST execution waits for `CXT1b`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedFact {
    pub id: LocalName,
    pub value_type: ValueTypeRef,
    pub source_slot: LocalName,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextDeclaration {
    pub contract_version: Version,
    pub expression_version: Version,
    pub captured_facts: Vec<CapturedFact>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    pub id: QualifiedName,
    pub contract_version: Version,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InspectorSection {
    Identity,
    Ports,
    Parameters,
    StateSummaries,
    Effects,
    Diagnostics,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectorContract {
    pub schema: SchemaRef,
    pub contribution: Contribution,
    pub sections: Vec<InspectorSection>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HostComponentId {
    #[serde(rename = "photara.browser.assets")]
    Assets,
    #[serde(rename = "photara.picker.person")]
    Person,
    #[serde(rename = "photara.picker.organization")]
    Organization,
    #[serde(rename = "photara.picker.location")]
    Location,
    #[serde(rename = "photara.picker.location-kind")]
    LocationKind,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ComponentBinding {
    AssetPort { port_id: LocalName },
    LibraryQuery { selector_slot: LocalName },
    AssignmentSubset { selector_slot: LocalName },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentAction {
    Inspect,
    Select,
    CreateLibraryRecord,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostComponentRequest {
    pub instance_id: LocalName,
    pub component_id: HostComponentId,
    pub contract_version: Version,
    pub input: ComponentBinding,
    pub actions: Vec<ComponentAction>,
    pub result_schema: SchemaRef,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkSurfaceContract {
    pub contribution: Contribution,
    pub components: Vec<HostComponentRequest>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationContract {
    pub brand: String,
    pub display_name: String,
    pub icon_resource: QualifiedName,
    pub taxonomy_version: Version,
    pub primary_category: QualifiedName,
    pub search_terms: Vec<String>,
    pub provider_tags: Vec<QualifiedName>,
    pub capability_tags: Vec<QualifiedName>,
    pub activation: Option<QualifiedName>,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCoordinate {
    pub runtime_id: QualifiedName,
    pub contract_version: Version,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCoordinate {
    pub tool_id: QualifiedName,
    pub version: PackageVersion,
    pub implementation_digest: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformContract {
    pub platform: HostKind,
    pub runtimes: Vec<RuntimeCoordinate>,
    pub tools: Vec<ToolCoordinate>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionAxes {
    pub contract_version: Version,
    pub implementation_digest: Digest,
    pub context_version: Version,
    pub expression_version: Version,
    pub evaluation_key_version: Version,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IdRemapContract {
    NoIdBearingState,
    Typed { schema: SchemaRef },
    UnsupportedOpaque,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationDeclaration {
    pub migration_id: QualifiedName,
    pub from: DefinitionCoordinate,
    pub to: DefinitionCoordinate,
    pub from_config: SchemaRef,
    pub to_config: SchemaRef,
    pub from_state: Option<SchemaRef>,
    pub to_state: Option<SchemaRef>,
    pub deterministic: bool,
    pub implementation_digest: Digest,
    pub id_remap: IdRemapContract,
    pub diagnostics_schema: SchemaRef,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDefinitionV2 {
    pub coordinate: DefinitionCoordinate,
    pub ports: Vec<PortContract>,
    pub configuration_schema: AuthoredSchema,
    pub state_schema: Option<AuthoredSchema>,
    pub execution: ExecutionContract,
    pub capabilities: Vec<CapabilityRequirement>,
    pub determinism: Determinism,
    pub cache: CacheContract,
    pub context: ContextDeclaration,
    pub inspector: InspectorContract,
    pub work_surface: Option<WorkSurfaceContract>,
    pub presentation: PresentationContract,
    pub platforms: Vec<PlatformContract>,
    pub versions: VersionAxes,
    pub migrations: Vec<MigrationDeclaration>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestSpec {
    pub manifest_schema_version: u32,
    pub contract_version: Version,
    pub package_id: QualifiedName,
    pub package_version: PackageVersion,
    pub display_name: String,
    pub required_features: Vec<RequiredFeature>,
    pub definitions: Vec<NodeDefinitionV2>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ManifestSpec", into = "ManifestSpec")]
pub struct NodePackageManifestV2(pub(super) ManifestSpec);
impl From<NodePackageManifestV2> for ManifestSpec {
    fn from(m: NodePackageManifestV2) -> Self {
        m.0
    }
}

/// Exact registry supplied by trusted schema registration, not a live service.
#[derive(Clone, Debug, Default)]
pub struct ContractRegistry {
    pub(super) schemas: BTreeSet<SchemaRef>,
    pub(super) values: BTreeMap<ValueTypeRef, (SchemaRef, ValueFamily)>,
}
/// Host facts are independent of what a package declares about itself.
#[derive(Clone, Debug)]
pub struct HostAvailability {
    pub platform: HostKind,
    pub installed_implementation: Option<Digest>,
    pub trusted: bool,
    pub runtimes: BTreeSet<RuntimeCoordinate>,
    pub tools: BTreeSet<ToolCoordinate>,
    pub native_skin_available: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackReason {
    Ready,
    LegacyV1,
    MissingRuntime,
    Untrusted,
    IncompatibleRuntime,
    UnsupportedPlatform,
    UnsupportedContract,
    MissingSchema,
    Malformed,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// These flags represent independent host facilities, not product state variants.
#[allow(clippy::struct_excessive_bools)]
pub struct NodeFallbackDescriptor {
    pub reason: FallbackReason,
    pub preserve_exact_bytes: bool,
    pub host_owned_inspector: bool,
    pub portable_evaluation: bool,
    pub semantic_edits: bool,
    pub node_presentation_code: bool,
    pub migration_execution: bool,
}
/// Original bytes remain opaque and are never automatically reserialized or run.
pub struct ManifestInspection {
    pub(super) original: Vec<u8>,
    pub manifest: Option<NodePackageManifestV2>,
    pub fallback: NodeFallbackDescriptor,
}
impl ManifestInspection {
    #[must_use]
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
}
