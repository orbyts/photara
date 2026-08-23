use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use photara_asset_set_node::{AssetSetNodePackage, AssetSetNodeRuntime, asset_set_state_schema};
use photara_core::{
    AssetId, AssetSet, CommandId, Connection, ConnectionId, DefinitionResolver, Diagnostic,
    DiagnosticSeverity, GraphCommand, GraphCommandEnvelope, GraphDocument, GraphId,
    NodeDefinitionRef, NodeInstance, NodeInstanceId, PackageRequirement, PortDirection,
    PortEndpoint, PortId, ProjectAsset, ProjectCommand, ProjectCommandEnvelope, ProjectDocument,
    ProjectId, ProjectRelativePath, REPRESENTATION_FORMAT_EXTENSION_KEY, RepresentationBinding,
    RepresentationDescriptor, RepresentationRevisionEvidence, RepresentationStorageBindingId,
    RequestId, SchemaValue, ValueTypeRegistry, apply_graph_command, apply_project_command,
    asset_set_value_type_descriptor, canonical_digest,
};
use photara_disk_node::{
    DiskFolderProvider, DiskFolderState, DiskNodePackage, DiskNodeRuntime, DiskRevisionMode,
};
use photara_layout_node::{
    BundledCanvasProfile, CellArrangement, CellContentMode, LayoutCanvas, LayoutCell,
    LayoutCommand, LayoutCommandError, LayoutFrame, LayoutNodePackage, LayoutNodeRuntime,
    LayoutState, NormalizedPoint, NormalizedRect, NormalizedUnit, QuarterTurn,
    apply_layout_command, layout_plan_value_type_descriptor, resolve_layout,
};
use photara_node_sdk::{
    NodeCatalogVisibility, NodePackage, NodePackageRegistry, node_presentation,
};
#[cfg(target_os = "macos")]
use photara_proxy::{
    AssetContextProjectProxyService, ImageIoCoreImageGenerator, ImageIoGeneratorConfig,
    ProjectProxyService, ProjectVisualProxyRequest, ProjectVisualProxyService, ProxyServiceConfig,
    layout_interaction_preview_profile, standard_gallery_preview_profile,
};
use photara_proxy::{ProxyArtifact, ProxyArtifactDisposition};
use photara_store::{FileSystemStateStore, ProjectRepository, prepare_local_tiff_pair_import};
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    asset_materializer::RuntimeAwareMaterializer, evaluation::EvaluationHandle,
    runtime_registry::RuntimeRegistry,
};

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeDiagnosticDto {
    pub code: String,
    pub severity: BridgeDiagnosticSeverity,
    pub message: String,
    pub node_id: Option<String>,
    pub port_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeStructuredErrorDto {
    pub code: String,
    pub message: String,
    pub diagnostic: BridgeDiagnosticDto,
    pub details_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeGraphSnapshotDto {
    pub graph_id: String,
    pub revision: u64,
    pub digest: String,
    pub connections: Vec<BridgeConnectionDto>,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
#[allow(clippy::struct_field_names)]
pub struct BridgeConnectionDto {
    pub connection_id: String,
    pub output_node_id: String,
    pub output_port_id: String,
    pub input_node_id: String,
    pub input_port_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgePortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeInspectionFieldDto {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgePortInspectionDto {
    pub port_id: String,
    pub direction: BridgePortDirection,
    pub value_type_id: String,
    pub value_type_version: u32,
    pub connected_node_id: Option<String>,
    pub connected_node_name: Option<String>,
    pub summary: Vec<BridgeInspectionFieldDto>,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub package_id: String,
    pub package_version: String,
    pub definition_id: String,
    pub definition_version: u32,
    pub brand_name: String,
    pub icon_resource_id: String,
    pub theme_color_role: Option<String>,
    pub accent_srgb_hex: Option<String>,
    pub inspector_contribution_id: Option<String>,
    pub workspace_contribution_id: Option<String>,
    pub default_activation_id: Option<String>,
    pub has_workspace: bool,
    pub status: String,
    pub ports: Vec<BridgePortInspectionDto>,
    pub output_summary: Vec<BridgeInspectionFieldDto>,
    pub disk: Option<BridgeDiskInspectionDto>,
    pub layout: Option<BridgeLayoutInspectionDto>,
    pub diagnostics: Vec<BridgeDiagnosticDto>,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeDiskInspectionDto {
    pub folder_binding_id: String,
    pub recursive: bool,
    pub accepted_asset_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeDiskBindingDto {
    pub node_id: String,
    pub folder_binding_id: String,
    pub folder_display_name: String,
}

/// Runtime-only local source reference suitable for a native placeholder
/// thumbnail service such as macOS Quick Look. It is never project state.
#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeNativeThumbnailSourceDto {
    pub asset_id: String,
    pub local_path: String,
    pub source_fingerprint: String,
    pub source_verified: bool,
}

/// Immutable catalog entry for one exact installed node definition.
#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeAvailableNodeDefinitionDto {
    pub package_id: String,
    pub package_version: String,
    pub definition_id: String,
    pub definition_version: u32,
    pub display_name: String,
    pub brand_name: String,
    pub icon_resource_id: String,
    pub theme_color_role: Option<String>,
    pub accent_srgb_hex: Option<String>,
    pub catalog_path: Vec<String>,
    pub search_terms: Vec<String>,
    pub inspector_contribution_id: Option<String>,
    pub workspace_contribution_id: Option<String>,
    pub default_activation_id: Option<String>,
}

/// Exact definition coordinate selected from the immutable application catalog.
#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeNodeDefinitionRefDto {
    pub package_id: String,
    pub package_version: String,
    pub definition_id: String,
    pub definition_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeLayoutInspectionDto {
    pub authored_state_digest: String,
    pub canvas: BridgeLayoutCanvasInspectionDto,
    pub frames: Vec<BridgeLayoutFrameInspectionDto>,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeLayoutCanvasInspectionDto {
    pub kind: BridgeLayoutCanvasKind,
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub horizontal_units: Option<u32>,
    pub vertical_units: Option<u32>,
    pub long_edge_pixels: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutCanvasKind {
    Portrait3x4,
    Vertical9x16,
    CustomPixels,
    CustomAspect,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeLayoutFrameInspectionDto {
    pub frame_id: String,
    pub index: u64,
    pub arrangement: BridgeLayoutArrangement,
    pub cells: Vec<BridgeLayoutCellInspectionDto>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutArrangement {
    One,
    HorizontalStack,
    VerticalStack,
    UniformGrid,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeLayoutCellInspectionDto {
    pub cell_id: String,
    pub index: u64,
    pub asset_id: Option<String>,
    pub content_mode: BridgeLayoutContentMode,
    pub focal_x: Option<u32>,
    pub focal_y: Option<u32>,
    pub crop_rect: Option<BridgeNormalizedRectDto>,
    pub custom_rect: Option<BridgeNormalizedRectDto>,
    pub resolved_rect: BridgeNormalizedRectDto,
    pub resolved_pixel_rect: BridgePixelRectDto,
    pub quarter_turn: BridgeQuarterTurn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgePixelRectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutContentMode {
    Fit,
    Fill,
    Crop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeNormalizedRectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeQuarterTurn {
    Zero,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutArrangementEdit {
    One,
    HorizontalStack,
    VerticalStack,
    UniformGrid { columns: u32 },
    Custom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutCellEdit {
    Fit {
        alignment_x: u32,
        alignment_y: u32,
    },
    Fill {
        focal_x: u32,
        focal_y: u32,
    },
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    SetQuarterTurn {
        quarter_turn: BridgeQuarterTurn,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutStructureEdit {
    InsertFrame {
        index: u64,
    },
    RemoveFrame {
        frame_id: String,
    },
    MoveFrame {
        frame_id: String,
        to_index: u64,
    },
    SetFrameArrangement {
        frame_id: String,
        arrangement: BridgeLayoutArrangementEdit,
    },
    InsertCell {
        frame_id: String,
        index: u64,
    },
    RemoveCell {
        frame_id: String,
        cell_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeAssetDto {
    pub asset_id: String,
    pub display_name: String,
    pub format_label: Option<String>,
    pub representation_count: u64,
    pub visual_revision: Option<String>,
    pub visual_revision_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeAssetImportDto {
    pub asset_id: String,
    pub snapshot: BridgeProjectSnapshotDto,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeProxyDisposition {
    CacheHit,
    Generated,
    SharedInFlight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeProxyChannelDepth {
    U8,
    U16,
    F16,
    F32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeProxyDynamicRange {
    Sdr,
    Hdr,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeProxyDescriptorDto {
    pub asset_id: String,
    pub local_path: String,
    pub disposition: BridgeProxyDisposition,
    pub cache_key: String,
    pub source_fingerprint: String,
    pub content_fingerprint: String,
    pub profile_id: String,
    pub profile_version: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub channel_depth: BridgeProxyChannelDepth,
    pub has_alpha: bool,
    pub encoding_id: String,
    pub encoding_version: u32,
    pub color_space_id: String,
    pub embedded_icc_fingerprint: Option<String>,
    pub dynamic_range: BridgeProxyDynamicRange,
    pub reference_white_nits: Option<u32>,
    pub hdr_headroom_millistops: Option<u32>,
    pub pixels_are_orientation_normalized: bool,
    pub byte_length: u64,
}

/// Leased reference to a verified disposable proxy file.
///
/// Holding this object prevents cache eviction while a client displays it.
#[derive(uniffi::Object)]
pub struct BridgeProxyReference {
    asset_id: AssetId,
    artifact: ProxyArtifact,
}

#[uniffi::export]
impl BridgeProxyReference {
    #[must_use]
    pub fn descriptor(&self) -> BridgeProxyDescriptorDto {
        proxy_descriptor(self.asset_id, &self.artifact)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeProjectSnapshotDto {
    pub project_id: String,
    pub project_revision: u64,
    pub title: String,
    pub graph: BridgeGraphSnapshotDto,
    pub assets: Vec<BridgeAssetDto>,
    pub nodes: Vec<BridgeNodeDto>,
    pub diagnostics: Vec<BridgeDiagnosticDto>,
    pub dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeCommandResponseDto {
    pub command_id: String,
    pub applied: bool,
    pub previous_graph_revision: u64,
    pub snapshot: Option<BridgeProjectSnapshotDto>,
    pub error: Option<BridgeStructuredErrorDto>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeLayoutCanvas {
    Portrait3x4 {
        long_edge_pixels: u32,
    },
    Vertical9x16 {
        long_edge_pixels: u32,
    },
    CustomPixels {
        width: u32,
        height: u32,
    },
    CustomAspect {
        horizontal_units: u32,
        vertical_units: u32,
        long_edge_pixels: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeEvaluationPhase {
    Validating,
    Planning,
    Evaluating,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeEvaluationProgressDto {
    pub request_id: String,
    pub evaluation_id: String,
    pub phase: BridgeEvaluationPhase,
    pub completed_nodes: u64,
    pub total_nodes: u64,
    pub node_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum BridgeEvaluationStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeEvaluationFinishedDto {
    pub request_id: String,
    pub evaluation_id: String,
    pub status: BridgeEvaluationStatus,
    pub evaluation_digest: Option<String>,
    pub error: Option<BridgeStructuredErrorDto>,
}

#[derive(Debug, Error, uniffi::Error)]
pub enum BridgeError {
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },
    #[error("project store error: {message}")]
    Store { message: String },
    #[error("bridge state error: {message}")]
    State { message: String },
}

#[uniffi::export(foreign)]
pub trait EvaluationObserver: Send + Sync {
    fn on_progress(&self, progress: BridgeEvaluationProgressDto);
    fn on_finished(&self, result: BridgeEvaluationFinishedDto);
}

#[derive(uniffi::Object)]
pub struct PhotaraApplication {
    store: FileSystemStateStore,
    definitions: Arc<NodePackageRegistry>,
    runtimes: Arc<RuntimeRegistry>,
    value_types: Arc<ValueTypeRegistry>,
    proxy_cache_root: PathBuf,
    proxy_helper_executable: PathBuf,
    proxy_generation_concurrency: NonZeroUsize,
}

#[derive(Clone)]
struct HostRuntimeServices {
    definitions: Arc<NodePackageRegistry>,
    runtimes: Arc<RuntimeRegistry>,
    value_types: Arc<ValueTypeRegistry>,
}

impl PhotaraApplication {
    fn runtime_services(&self) -> HostRuntimeServices {
        HostRuntimeServices {
            definitions: Arc::clone(&self.definitions),
            runtimes: Arc::clone(&self.runtimes),
            value_types: Arc::clone(&self.value_types),
        }
    }
}

#[uniffi::export]
impl PhotaraApplication {
    /// Opens the application service over one directory-backed project store.
    ///
    /// # Errors
    ///
    /// Returns a store or registration error when initialization fails.
    #[uniffi::constructor]
    pub fn open(
        store_root: String,
        proxy_cache_root: String,
        proxy_helper_executable: String,
        proxy_generation_concurrency: u32,
    ) -> Result<Arc<Self>, BridgeError> {
        let proxy_generation_concurrency = usize::try_from(proxy_generation_concurrency)
            .ok()
            .and_then(NonZeroUsize::new)
            .filter(|value| value.get() <= 4)
            .ok_or_else(|| BridgeError::InvalidArgument {
                message: "proxy generation concurrency must be between 1 and 4".to_owned(),
            })?;
        let store = FileSystemStateStore::open(PathBuf::from(store_root)).map_err(|error| {
            BridgeError::Store {
                message: error.to_string(),
            }
        })?;
        let asset_set_manifest = AssetSetNodePackage.manifest();
        let layout_manifest = LayoutNodePackage.manifest();
        let disk_manifest = DiskNodePackage.manifest();
        let mut definitions = NodePackageRegistry::default();
        definitions
            .register_manifest(asset_set_manifest.clone())
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        definitions
            .register_manifest(layout_manifest.clone())
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        definitions
            .register_manifest(disk_manifest.clone())
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        let mut runtimes = RuntimeRegistry::default();
        runtimes
            .register_manifest(&asset_set_manifest, AssetSetNodeRuntime)
            .and_then(|()| runtimes.register_manifest(&layout_manifest, LayoutNodeRuntime))
            .and_then(|()| runtimes.register_manifest(&disk_manifest, DiskNodeRuntime))
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        let mut value_types = ValueTypeRegistry::default();
        value_types
            .register(asset_set_value_type_descriptor())
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        value_types
            .register(layout_plan_value_type_descriptor())
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        Ok(Arc::new(Self {
            store,
            definitions: Arc::new(definitions),
            runtimes: Arc::new(runtimes),
            value_types: Arc::new(value_types),
            proxy_cache_root: PathBuf::from(proxy_cache_root),
            proxy_helper_executable: PathBuf::from(proxy_helper_executable),
            proxy_generation_concurrency,
        }))
    }

    /// Returns the visible installed definition catalog in stable presentation order.
    #[must_use]
    pub fn available_node_definitions(&self) -> Vec<BridgeAvailableNodeDefinitionDto> {
        let mut catalog = self
            .definitions
            .manifests()
            .flat_map(|manifest| {
                manifest.definitions.iter().filter_map(|definition| {
                    let presentation = node_presentation(definition).ok().flatten()?;
                    if presentation.catalog_visibility == NodeCatalogVisibility::Hidden {
                        return None;
                    }
                    Some(BridgeAvailableNodeDefinitionDto {
                        package_id: manifest.package_id.to_string(),
                        package_version: manifest.package_version.to_string(),
                        definition_id: definition.id.to_string(),
                        definition_version: definition.version.get(),
                        display_name: definition.display_name.clone(),
                        brand_name: presentation.brand.name,
                        icon_resource_id: presentation.brand.icon_resource_id,
                        theme_color_role: presentation.brand.theme_color_role,
                        accent_srgb_hex: presentation.brand.accent_srgb_hex,
                        catalog_path: presentation.catalog_path,
                        search_terms: presentation.search_terms,
                        inspector_contribution_id: presentation.inspector_contribution_id,
                        workspace_contribution_id: presentation.workspace_contribution_id,
                        default_activation_id: presentation.default_activation_id,
                    })
                })
            })
            .collect::<Vec<_>>();
        catalog.sort_by(|left, right| {
            left.catalog_path
                .cmp(&right.catalog_path)
                .then_with(|| left.brand_name.cmp(&right.brand_name))
                .then_with(|| left.definition_id.cmp(&right.definition_id))
        });
        catalog
    }

    /// Creates and durably publishes an empty portable project.
    ///
    /// # Errors
    ///
    /// Returns a validation or store error when creation fails.
    pub fn create_project(&self, title: String) -> Result<Arc<PhotaraProject>, BridgeError> {
        let project =
            ProjectDocument::new(ProjectId::new(), title, GraphDocument::new(GraphId::new()))
                .map_err(|error| BridgeError::State {
                    message: error.to_string(),
                })?;
        let mut store = self.store.clone();
        store
            .create_project(project.clone())
            .map_err(|error| BridgeError::Store {
                message: error.to_string(),
            })?;
        PhotaraProject::new(
            store,
            self.runtime_services(),
            project,
            self.proxy_cache_root.clone(),
            self.proxy_helper_executable.clone(),
            self.proxy_generation_concurrency,
        )
    }

    /// Reopens one project by portable semantic identity.
    ///
    /// # Errors
    ///
    /// Returns an argument or store error when the identity is invalid, absent,
    /// or unreadable.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_project(&self, project_id: String) -> Result<Arc<PhotaraProject>, BridgeError> {
        let project_id: ProjectId = parse_uuid_id(&project_id, "project ID")?;
        let project = self
            .store
            .load_project(project_id)
            .map_err(|error| BridgeError::Store {
                message: error.to_string(),
            })?
            .ok_or_else(|| BridgeError::InvalidArgument {
                message: format!("unknown project {project_id}"),
            })?;
        PhotaraProject::new(
            self.store.clone(),
            self.runtime_services(),
            project,
            self.proxy_cache_root.clone(),
            self.proxy_helper_executable.clone(),
            self.proxy_generation_concurrency,
        )
    }

    /// Imports and opens one validated portable project document.
    ///
    /// If the same semantic project already exists in the application store,
    /// the documents must be identical; this never silently overwrites a
    /// divergent project with the same identity.
    ///
    /// # Errors
    ///
    /// Returns a file, validation, identity-conflict, or store error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn open_project_document(
        &self,
        document_path: String,
    ) -> Result<Arc<PhotaraProject>, BridgeError> {
        let path = PathBuf::from(document_path);
        let json = fs::read_to_string(&path).map_err(|error| BridgeError::Store {
            message: format!(
                "could not read project document {}: {error}",
                path.display()
            ),
        })?;
        let imported = ProjectDocument::from_json(&json).map_err(|error| BridgeError::State {
            message: format!("invalid portable project document: {error}"),
        })?;
        let mut store = self.store.clone();
        let project = if let Some(existing) =
            store
                .load_project(imported.project_id)
                .map_err(|error| BridgeError::Store {
                    message: error.to_string(),
                })? {
            let existing_digest =
                canonical_digest(&existing).map_err(|error| BridgeError::State {
                    message: error.to_string(),
                })?;
            let imported_digest =
                canonical_digest(&imported).map_err(|error| BridgeError::State {
                    message: error.to_string(),
                })?;
            if existing_digest != imported_digest {
                return Err(BridgeError::State {
                    message: format!(
                        "project {} already exists with different content",
                        imported.project_id
                    ),
                });
            }
            existing
        } else {
            store
                .create_project(imported.clone())
                .map_err(|error| BridgeError::Store {
                    message: error.to_string(),
                })?;
            imported
        };
        PhotaraProject::new(
            store,
            self.runtime_services(),
            project,
            self.proxy_cache_root.clone(),
            self.proxy_helper_executable.clone(),
            self.proxy_generation_concurrency,
        )
    }
}

#[derive(Clone)]
struct LayoutUndoEntry {
    undo: GraphCommand,
    redo: GraphCommand,
}

struct PreparedLayoutEdit {
    forward: GraphCommand,
    reverse: GraphCommand,
}

struct ProjectSessionState {
    project: ProjectDocument,
    dirty: bool,
    undo: Vec<LayoutUndoEntry>,
    redo: Vec<LayoutUndoEntry>,
}

#[derive(uniffi::Object)]
pub struct PhotaraProject {
    store: FileSystemStateStore,
    definitions: Arc<NodePackageRegistry>,
    runtimes: Arc<RuntimeRegistry>,
    value_types: Arc<ValueTypeRegistry>,
    project_root: PathBuf,
    #[cfg(target_os = "macos")]
    proxy_service: Arc<ProjectProxyService<ImageIoCoreImageGenerator>>,
    folder_bindings: Mutex<BTreeMap<Uuid, PathBuf>>,
    representation_bindings: Mutex<BTreeMap<RepresentationStorageBindingId, PathBuf>>,
    state: Mutex<ProjectSessionState>,
}

impl PhotaraProject {
    fn new(
        store: FileSystemStateStore,
        services: HostRuntimeServices,
        project: ProjectDocument,
        proxy_cache_root: PathBuf,
        proxy_helper_executable: PathBuf,
        proxy_generation_concurrency: NonZeroUsize,
    ) -> Result<Arc<Self>, BridgeError> {
        let HostRuntimeServices {
            definitions,
            runtimes,
            value_types,
        } = services;
        let project_root = store
            .root()
            .join("project-data")
            .join(project.project_id.to_string());
        fs::create_dir_all(&project_root).map_err(|error| BridgeError::Store {
            message: format!("could not create project resource directory: {error}"),
        })?;
        #[cfg(target_os = "macos")]
        let mut proxy_config =
            ProxyServiceConfig::conservative(proxy_cache_root, 20 * 1024 * 1024 * 1024);
        #[cfg(target_os = "macos")]
        {
            proxy_config.max_concurrent_generations = proxy_generation_concurrency;
        }
        #[cfg(target_os = "macos")]
        let proxy_service = ProjectProxyService::open(
            project.project_id,
            proxy_config,
            ImageIoCoreImageGenerator::new(ImageIoGeneratorConfig {
                helper_executable: proxy_helper_executable,
            }),
        )
        .map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
        #[cfg(not(target_os = "macos"))]
        let _ = (
            proxy_cache_root,
            proxy_helper_executable,
            proxy_generation_concurrency,
        );
        Ok(Arc::new(Self {
            store,
            definitions,
            runtimes,
            value_types,
            project_root,
            #[cfg(target_os = "macos")]
            proxy_service: Arc::new(proxy_service),
            folder_bindings: Mutex::new(BTreeMap::new()),
            representation_bindings: Mutex::new(BTreeMap::new()),
            state: Mutex::new(ProjectSessionState {
                project,
                dirty: false,
                undo: Vec::new(),
                redo: Vec::new(),
            }),
        }))
    }
}

#[uniffi::export]
impl PhotaraProject {
    /// Returns an immutable view of the current in-memory project session.
    ///
    /// # Errors
    ///
    /// Returns a state error when the session cannot be read or digested.
    pub fn snapshot(&self) -> Result<BridgeProjectSnapshotDto, BridgeError> {
        let state = self.lock_state()?;
        project_snapshot(&state, &self.definitions)
    }

    /// Creates an ordinary node from an exact installed catalog coordinate.
    ///
    /// Package-specific default authored state is constructed inside the Rust
    /// host; native clients do not interpret node schemas.
    ///
    /// # Panics
    ///
    /// Panics only if Photara's compile-time nonzero Layout default is invalid.
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_node(
        &self,
        expected_graph_revision: u64,
        definition: BridgeNodeDefinitionRefDto,
    ) -> BridgeCommandResponseDto {
        let layout = LayoutNodePackage.manifest();
        let layout_definition = &layout.definitions[0];
        if definition.package_id == layout.package_id.as_str()
            && definition.package_version == layout.package_version.to_string()
            && definition.definition_id == layout_definition.id.as_str()
            && definition.definition_version == layout_definition.version.get()
        {
            let command_id = CommandId::new();
            let canvas = LayoutCanvas::portrait_3x4(
                NonZeroU32::new(4_000).expect("built-in default is nonzero"),
            );
            let authored_state = match LayoutState::new(canvas).to_schema_value() {
                Ok(value) => value,
                Err(error) => {
                    return rejected_command(
                        command_id,
                        expected_graph_revision,
                        "photara.layout.invalid-default-state",
                        error.to_string(),
                    );
                }
            };
            return self.apply_core_command(
                command_id,
                expected_graph_revision,
                GraphCommand::AddNode {
                    instance: NodeInstance {
                        id: NodeInstanceId::new(),
                        definition: NodeDefinitionRef {
                            package_id: layout.package_id,
                            package_version: layout.package_version,
                            definition_id: layout_definition.id.clone(),
                            definition_version: layout_definition.version,
                        },
                        configuration: SchemaValue {
                            schema: layout_definition.config_schema.clone(),
                            value: json!({}),
                        },
                        authored_state: Some(authored_state),
                        extensions: BTreeMap::new(),
                    },
                },
            );
        }
        let disk = DiskNodePackage.manifest();
        let disk_definition = &disk.definitions[0];
        if definition.package_id == disk.package_id.as_str()
            && definition.package_version == disk.package_version.to_string()
            && definition.definition_id == disk_definition.id.as_str()
            && definition.definition_version == disk_definition.version.get()
        {
            let command_id = CommandId::new();
            let authored_state = match DiskFolderState::default().to_schema_value() {
                Ok(value) => value,
                Err(error) => {
                    return rejected_command(
                        command_id,
                        expected_graph_revision,
                        "photara.disk.invalid-default-state",
                        error.to_string(),
                    );
                }
            };
            return self.apply_core_command(
                command_id,
                expected_graph_revision,
                GraphCommand::AddNode {
                    instance: NodeInstance {
                        id: NodeInstanceId::new(),
                        definition: NodeDefinitionRef {
                            package_id: disk.package_id,
                            package_version: disk.package_version,
                            definition_id: disk_definition.id.clone(),
                            definition_version: disk_definition.version,
                        },
                        configuration: SchemaValue {
                            schema: disk_definition.config_schema.clone(),
                            value: json!({}),
                        },
                        authored_state: Some(authored_state),
                        extensions: BTreeMap::new(),
                    },
                },
            );
        }
        rejected_command(
            CommandId::new(),
            expected_graph_revision,
            "photara.bridge.unsupported-node-creation",
            format!(
                "definition {}@{} from {} {} has no registered creation factory",
                definition.definition_id,
                definition.definition_version,
                definition.package_id,
                definition.package_version
            ),
        )
    }

    /// Atomically saves this session against its last opened project revision.
    ///
    /// # Errors
    ///
    /// Returns a state or store error for lock failure, revision conflict, or
    /// durable-write failure.
    pub fn save(&self) -> Result<BridgeProjectSnapshotDto, BridgeError> {
        let mut state = self.lock_state()?;
        let expected = state.project.revision;
        let next = expected.checked_next().ok_or_else(|| BridgeError::State {
            message: "project revision exhausted".to_owned(),
        })?;
        let mut replacement = state.project.clone();
        replacement.revision = next;
        let mut store = self.store.clone();
        store
            .replace_project(replacement.clone(), expected)
            .map_err(|error| BridgeError::Store {
                message: error.to_string(),
            })?;
        state.project = replacement;
        state.dirty = false;
        project_snapshot(&state, &self.definitions)
    }

    /// Adds an ordinary Layout node using a revision-checked Core command.
    ///
    /// # Panics
    ///
    /// Panics only if Photara's compile-time built-in port ID is invalid.
    pub fn add_layout_node(
        &self,
        expected_graph_revision: u64,
        canvas: BridgeLayoutCanvas,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let canvas = match layout_canvas(canvas) {
            Ok(canvas) => canvas,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.bridge.invalid-canvas",
                    error.to_string(),
                );
            }
        };
        let source_manifest = AssetSetNodePackage.manifest();
        let source_definition = &source_manifest.definitions[0];
        let source_id = NodeInstanceId::new();
        let source = NodeInstance {
            id: source_id,
            definition: NodeDefinitionRef {
                package_id: source_manifest.package_id,
                package_version: source_manifest.package_version,
                definition_id: source_definition.id.clone(),
                definition_version: source_definition.version,
            },
            configuration: SchemaValue {
                schema: source_definition.config_schema.clone(),
                value: json!({}),
            },
            authored_state: Some(SchemaValue {
                schema: asset_set_state_schema(),
                value: json!({"assets": []}),
            }),
            extensions: BTreeMap::new(),
        };
        let manifest = LayoutNodePackage.manifest();
        let definition = &manifest.definitions[0];
        let layout_id = NodeInstanceId::new();
        let instance = NodeInstance {
            id: layout_id,
            definition: NodeDefinitionRef {
                package_id: manifest.package_id,
                package_version: manifest.package_version,
                definition_id: definition.id.clone(),
                definition_version: definition.version,
            },
            configuration: SchemaValue {
                schema: definition.config_schema.clone(),
                value: json!({}),
            },
            authored_state: match LayoutState::new(canvas).to_schema_value() {
                Ok(value) => Some(value),
                Err(error) => {
                    return rejected_command(
                        command_id,
                        expected_graph_revision,
                        "photara.layout.invalid-authored-state",
                        error.to_string(),
                    );
                }
            },
            extensions: BTreeMap::new(),
        };
        let response = self.apply_core_command(
            command_id,
            expected_graph_revision,
            GraphCommand::Batch {
                commands: vec![
                    GraphCommand::AddNode { instance: source },
                    GraphCommand::AddNode { instance },
                    GraphCommand::Connect {
                        connection: Connection {
                            id: ConnectionId::new(),
                            output: PortEndpoint {
                                node_id: source_id,
                                port_id: PortId::parse("assets")
                                    .expect("built-in port ID is valid"),
                            },
                            input: PortEndpoint {
                                node_id: layout_id,
                                port_id: PortId::parse("assets")
                                    .expect("built-in port ID is valid"),
                            },
                            extensions: BTreeMap::new(),
                        },
                    },
                ],
            },
        );
        if response.applied
            && let Ok(mut state) = self.lock_state()
        {
            state.undo.clear();
            state.redo.clear();
        }
        response
    }

    /// Applies an authored normalized crop and records its exact inverse.
    #[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
    pub fn set_layout_cell_crop(
        &self,
        expected_graph_revision: u64,
        node_id: String,
        frame_id: String,
        cell_id: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> BridgeCommandResponseDto {
        self.edit_layout_cell(
            expected_graph_revision,
            node_id,
            frame_id,
            cell_id,
            BridgeLayoutCellEdit::Crop {
                x,
                y,
                width,
                height,
            },
        )
    }

    /// Applies one intentional cell edit as one revision-checked Core command.
    #[allow(clippy::needless_pass_by_value)]
    pub fn edit_layout_cell(
        &self,
        expected_graph_revision: u64,
        node_id: String,
        frame_id: String,
        cell_id: String,
        edit: BridgeLayoutCellEdit,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let prepared = self.prepare_cell_edit(&node_id, &frame_id, &cell_id, edit);
        self.apply_prepared_layout_edit(
            command_id,
            expected_graph_revision,
            prepared,
            "photara.layout.invalid-cell-edit",
        )
    }

    /// Applies one intentional frame/cell structure edit through Core.
    #[allow(clippy::needless_pass_by_value)]
    pub fn edit_layout_structure(
        &self,
        expected_graph_revision: u64,
        node_id: String,
        edit: BridgeLayoutStructureEdit,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let prepared = self.prepare_structure_edit(&node_id, edit);
        self.apply_prepared_layout_edit(
            command_id,
            expected_graph_revision,
            prepared,
            "photara.layout.invalid-structure-edit",
        )
    }

    pub fn undo_layout(&self, expected_graph_revision: u64) -> BridgeCommandResponseDto {
        self.apply_history_command(expected_graph_revision, true)
    }

    pub fn redo_layout(&self, expected_graph_revision: u64) -> BridgeCommandResponseDto {
        self.apply_history_command(expected_graph_revision, false)
    }

    /// Imports a paired HDR/SDR TIFF as one project-owned semantic asset.
    ///
    /// The selected files are copied into portable project resources. Their
    /// original absolute paths never enter the project document.
    ///
    /// # Errors
    ///
    /// Returns an argument, file-copy, asset validation, or session error.
    pub fn import_local_tiff_pair(
        &self,
        display_name: String,
        hdr_source_path: String,
        sdr_source_path: String,
    ) -> Result<BridgeAssetImportDto, BridgeError> {
        if display_name.trim().is_empty() {
            return Err(BridgeError::InvalidArgument {
                message: "asset display name must not be empty".to_owned(),
            });
        }
        let hdr_source = PathBuf::from(hdr_source_path);
        let sdr_source = PathBuf::from(sdr_source_path);
        let import_id = RequestId::new().to_string();
        let relative_directory = format!("representations/{import_id}");
        let destination_directory = self.project_root.join(&relative_directory);
        fs::create_dir_all(&destination_directory).map_err(|error| BridgeError::Store {
            message: format!("could not create representation directory: {error}"),
        })?;
        let hdr_relative =
            ProjectRelativePath::parse(format!("{relative_directory}/flattened-hdr.tiff"))
                .map_err(|error| BridgeError::InvalidArgument {
                    message: error.to_string(),
                })?;
        let sdr_relative =
            ProjectRelativePath::parse(format!("{relative_directory}/flattened-sdr.tiff"))
                .map_err(|error| BridgeError::InvalidArgument {
                    message: error.to_string(),
                })?;
        fs::copy(&hdr_source, self.project_root.join(hdr_relative.as_str())).map_err(|error| {
            BridgeError::Store {
                message: format!("could not import HDR TIFF: {error}"),
            }
        })?;
        fs::copy(&sdr_source, self.project_root.join(sdr_relative.as_str())).map_err(|error| {
            BridgeError::Store {
                message: format!("could not import SDR TIFF: {error}"),
            }
        })?;

        let prepared = prepare_local_tiff_pair_import(
            &self.project_root,
            display_name,
            hdr_relative,
            sdr_relative,
        )
        .map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
        let asset_id = prepared.asset.id;
        let mut state = self.lock_state()?;
        let result = apply_project_command(
            &state.project,
            &ProjectCommandEnvelope {
                command_id: CommandId::new(),
                project_id: state.project.project_id,
                expected_revision: state.project.revision,
                command: ProjectCommand::AddAsset {
                    asset: prepared.asset,
                    resources: prepared.resources,
                },
            },
        )
        .map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
        state.project = result.project;
        state.dirty = true;
        Ok(BridgeAssetImportDto {
            asset_id: asset_id.to_string(),
            snapshot: project_snapshot(&state, &self.definitions)?,
        })
    }

    /// Explicitly adds an asset to a Layout's connected `AssetSet` and assigns it
    /// to one cell. Gallery selection is not consulted.
    #[allow(clippy::needless_pass_by_value)]
    pub fn bind_asset_to_layout(
        &self,
        expected_graph_revision: u64,
        layout_node_id: String,
        frame_id: String,
        cell_id: String,
        asset_id: String,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let prepared = self.prepare_asset_binding(&layout_node_id, &frame_id, &cell_id, &asset_id);
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.bridge.invalid-asset-binding",
                    error,
                );
            }
        };
        self.apply_prepared_layout_edit(
            command_id,
            expected_graph_revision,
            Ok(prepared),
            "photara.bridge.invalid-asset-binding",
        )
    }

    /// Attaches one device-local authorized folder to a portable Disk binding.
    /// This runtime operation does not change graph or project semantics.
    ///
    /// # Errors
    ///
    /// Returns an identity, node-state, folder-availability, or lock error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn attach_disk_folder(
        &self,
        node_id: String,
        folder_path: String,
    ) -> Result<BridgeDiskBindingDto, BridgeError> {
        let node_id: NodeInstanceId = parse_uuid_id(&node_id, "Disk node ID")?;
        let folder = PathBuf::from(folder_path);
        if !folder.is_dir() {
            return Err(BridgeError::InvalidArgument {
                message: format!(
                    "Disk binding is not a readable folder: {}",
                    folder.display()
                ),
            });
        }
        let disk = {
            let state = self.lock_state()?;
            let node = state
                .project
                .graph
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .ok_or_else(|| BridgeError::InvalidArgument {
                    message: format!("unknown Disk node {node_id}"),
                })?;
            if node.definition.package_id.as_str() != photara_disk_node::PACKAGE_ID {
                return Err(BridgeError::InvalidArgument {
                    message: format!("node {node_id} is not a Disk definition"),
                });
            }
            DiskFolderState::from_schema_value(node.authored_state.as_ref().ok_or_else(|| {
                BridgeError::State {
                    message: format!("Disk {node_id} has no authored state"),
                }
            })?)
            .map_err(|message| BridgeError::State { message })?
        };
        self.folder_bindings
            .lock()
            .map_err(|_| BridgeError::State {
                message: "Disk folder-binding lock was poisoned".to_owned(),
            })?
            .insert(disk.folder_binding_id, folder.clone());
        Ok(BridgeDiskBindingDto {
            node_id: node_id.to_string(),
            folder_binding_id: disk.folder_binding_id.to_string(),
            folder_display_name: folder.file_name().map_or_else(
                || folder.display().to_string(),
                |name| name.to_string_lossy().into(),
            ),
        })
    }

    /// Explicitly clears one Disk node's previously accepted source
    /// membership before a newly granted folder is scanned. Other project
    /// assets are retained.
    #[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
    pub fn clear_disk_assets(
        &self,
        expected_graph_revision: u64,
        node_id: String,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let node_id: NodeInstanceId = match parse_uuid_id(&node_id, "Disk node ID") {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.disk.invalid-node",
                    error.to_string(),
                );
            }
        };
        let Ok(mut state) = self.lock_state() else {
            return rejected_command(
                command_id,
                expected_graph_revision,
                "photara.bridge.lock-poisoned",
                "project session lock was poisoned".to_owned(),
            );
        };
        let actual = state.project.graph.revision;
        if actual.get() != expected_graph_revision {
            return rejected_command(
                command_id,
                actual.get(),
                "revision-conflict",
                format!(
                    "expected graph revision {expected_graph_revision}, actual {}",
                    actual.get()
                ),
            );
        }
        let Some(node) = state
            .project
            .graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
        else {
            return rejected_command(
                command_id,
                actual.get(),
                "photara.disk.unknown-node",
                format!("unknown Disk node {node_id}"),
            );
        };
        let mut disk = match node
            .authored_state
            .as_ref()
            .map(DiskFolderState::from_schema_value)
        {
            Some(Ok(value)) => value,
            Some(Err(message)) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.invalid-state",
                    message,
                );
            }
            None => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.missing-state",
                    format!("Disk {node_id} has no authored state"),
                );
            }
        };
        if disk.accepted_assets.assets.is_empty() {
            return match project_snapshot(&state, &self.definitions) {
                Ok(snapshot) => BridgeCommandResponseDto {
                    command_id: command_id.to_string(),
                    applied: true,
                    previous_graph_revision: actual.get(),
                    snapshot: Some(snapshot),
                    error: None,
                },
                Err(error) => rejected_command(
                    command_id,
                    actual.get(),
                    "photara.bridge.snapshot-failed",
                    error.to_string(),
                ),
            };
        }
        let removed_asset_ids = std::mem::take(&mut disk.accepted_assets.assets);
        let removed_binding_ids = state
            .project
            .asset_context
            .assets
            .iter()
            .filter(|asset| removed_asset_ids.contains(&asset.id))
            .flat_map(|asset| &asset.representations)
            .filter_map(|representation| match representation.binding {
                RepresentationBinding::RuntimeResolved { binding_id } => Some(binding_id),
                RepresentationBinding::ProjectResource { .. } => None,
            })
            .collect::<Vec<_>>();
        let project_result = match apply_project_command(
            &state.project,
            &ProjectCommandEnvelope {
                command_id,
                project_id: state.project.project_id,
                expected_revision: state.project.revision,
                command: ProjectCommand::ReconcileAssets {
                    remove_asset_ids: removed_asset_ids,
                    upsert_assets: Vec::new(),
                },
            },
        ) {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.asset-reconciliation-failed",
                    error.to_string(),
                );
            }
        };
        let authored_state = match disk.to_schema_value() {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.invalid-scan-state",
                    error.to_string(),
                );
            }
        };
        let graph_result = match apply_graph_command(
            &project_result.project.graph,
            &GraphCommandEnvelope {
                command_id,
                graph_id: state.project.graph.id,
                expected_revision: actual,
                command: GraphCommand::SetAuthoredState {
                    node_id,
                    authored_state: Some(authored_state),
                },
            },
            self.definitions.as_ref(),
            self.value_types.as_ref(),
        ) {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.membership-reconciliation-failed",
                    error.to_string(),
                );
            }
        };
        let mut project = project_result.project;
        project.graph = graph_result.graph;
        sync_requirements(&mut project);
        if let Ok(mut bindings) = self.representation_bindings.lock() {
            bindings.retain(|binding_id, _| !removed_binding_ids.contains(binding_id));
        }
        state.project = project;
        state.dirty = true;
        match project_snapshot(&state, &self.definitions) {
            Ok(snapshot) => BridgeCommandResponseDto {
                command_id: command_id.to_string(),
                applied: true,
                previous_graph_revision: actual.get(),
                snapshot: Some(snapshot),
                error: None,
            },
            Err(error) => rejected_command(
                command_id,
                actual.get(),
                "photara.bridge.snapshot-failed",
                error.to_string(),
            ),
        }
    }

    /// Quickly discovers an attached Disk folder from metadata observations
    /// without reading complete source files.
    #[allow(clippy::needless_pass_by_value)]
    pub fn discover_disk_folder(
        &self,
        expected_graph_revision: u64,
        node_id: String,
    ) -> BridgeCommandResponseDto {
        self.reconcile_disk_folder(
            expected_graph_revision,
            node_id,
            DiskRevisionMode::Observation,
        )
    }

    /// Verifies an attached Disk folder with complete content digests and
    /// atomically publishes the resulting revisions.
    #[allow(clippy::needless_pass_by_value)]
    pub fn scan_disk_folder(
        &self,
        expected_graph_revision: u64,
        node_id: String,
    ) -> BridgeCommandResponseDto {
        self.reconcile_disk_folder(expected_graph_revision, node_id, DiskRevisionMode::Content)
    }
}

impl PhotaraProject {
    #[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
    fn reconcile_disk_folder(
        &self,
        expected_graph_revision: u64,
        node_id: String,
        revision_mode: DiskRevisionMode,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let node_id: NodeInstanceId = match parse_uuid_id(&node_id, "Disk node ID") {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.disk.invalid-node",
                    error.to_string(),
                );
            }
        };
        let disk = {
            let Ok(state) = self.lock_state() else {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.bridge.lock-poisoned",
                    "project session lock was poisoned".to_owned(),
                );
            };
            if state.project.graph.revision.get() != expected_graph_revision {
                return rejected_command(
                    command_id,
                    state.project.graph.revision.get(),
                    "revision-conflict",
                    format!(
                        "expected graph revision {expected_graph_revision}, actual {}",
                        state.project.graph.revision.get()
                    ),
                );
            }
            let Some(node) = state
                .project
                .graph
                .nodes
                .iter()
                .find(|node| node.id == node_id)
            else {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.disk.unknown-node",
                    format!("unknown Disk node {node_id}"),
                );
            };
            match node
                .authored_state
                .as_ref()
                .map(DiskFolderState::from_schema_value)
            {
                Some(Ok(value)) => value,
                Some(Err(message)) => {
                    return rejected_command(
                        command_id,
                        expected_graph_revision,
                        "photara.disk.invalid-state",
                        message,
                    );
                }
                None => {
                    return rejected_command(
                        command_id,
                        expected_graph_revision,
                        "photara.disk.missing-state",
                        format!("Disk {node_id} has no authored state"),
                    );
                }
            }
        };
        let Some(folder) = self
            .folder_bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.get(&disk.folder_binding_id).cloned())
        else {
            return rejected_command(
                command_id,
                expected_graph_revision,
                "photara.disk.binding-unavailable",
                "Choose or restore this Disk node's folder before scanning".to_owned(),
            );
        };
        let prepared = match DiskFolderProvider::reconcile(&folder, &disk, revision_mode) {
            Ok(value) => value,
            Err(message) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.disk.scan-failed",
                    message,
                );
            }
        };
        let authored_state = match prepared.authored_state.to_schema_value() {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.disk.invalid-scan-state",
                    error.to_string(),
                );
            }
        };

        let Ok(mut state) = self.lock_state() else {
            return rejected_command(
                command_id,
                expected_graph_revision,
                "photara.bridge.lock-poisoned",
                "project session lock was poisoned".to_owned(),
            );
        };
        let actual = state.project.graph.revision;
        if actual.get() != expected_graph_revision {
            return rejected_command(
                command_id,
                actual.get(),
                "revision-conflict",
                "the graph changed while Disk was scanning; scan again".to_owned(),
            );
        }
        let project_result = match apply_project_command(
            &state.project,
            &ProjectCommandEnvelope {
                command_id,
                project_id: state.project.project_id,
                expected_revision: state.project.revision,
                command: ProjectCommand::ReconcileAssets {
                    remove_asset_ids: prepared.previous_asset_ids,
                    upsert_assets: prepared.assets,
                },
            },
        ) {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.asset-reconciliation-failed",
                    error.to_string(),
                );
            }
        };
        let graph_result = match apply_graph_command(
            &project_result.project.graph,
            &GraphCommandEnvelope {
                command_id,
                graph_id: state.project.graph.id,
                expected_revision: actual,
                command: GraphCommand::SetAuthoredState {
                    node_id,
                    authored_state: Some(authored_state),
                },
            },
            self.definitions.as_ref(),
            self.value_types.as_ref(),
        ) {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    actual.get(),
                    "photara.disk.membership-reconciliation-failed",
                    error.to_string(),
                );
            }
        };
        let mut project = project_result.project;
        project.graph = graph_result.graph;
        sync_requirements(&mut project);
        if let Err(error) = project.validate() {
            return rejected_command(
                command_id,
                actual.get(),
                "photara.disk.invalid-reconciliation",
                error.to_string(),
            );
        }
        if let Ok(mut bindings) = self.representation_bindings.lock() {
            bindings.extend(prepared.runtime_bindings);
        }
        state.project = project;
        state.dirty = true;
        match project_snapshot(&state, &self.definitions) {
            Ok(snapshot) => BridgeCommandResponseDto {
                command_id: command_id.to_string(),
                applied: true,
                previous_graph_revision: actual.get(),
                snapshot: Some(snapshot),
                error: None,
            },
            Err(error) => rejected_command(
                command_id,
                actual.get(),
                "photara.bridge.snapshot-failed",
                error.to_string(),
            ),
        }
    }
}

#[uniffi::export]
impl PhotaraProject {
    /// Explicitly connects two typed ports through one revision-checked Core command.
    #[allow(clippy::needless_pass_by_value)]
    pub fn connect_nodes(
        &self,
        expected_graph_revision: u64,
        output_node_id: String,
        output_port_id: String,
        input_node_id: String,
        input_port_id: String,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let parsed = (
            parse_uuid_id(&output_node_id, "output node ID"),
            PortId::parse(output_port_id),
            parse_uuid_id(&input_node_id, "input node ID"),
            PortId::parse(input_port_id),
        );
        let (Ok(output_node_id), Ok(output_port_id), Ok(input_node_id), Ok(input_port_id)) = parsed
        else {
            return rejected_command(
                command_id,
                expected_graph_revision,
                "photara.bridge.invalid-connection",
                "connection contains an invalid node or port identity".to_owned(),
            );
        };
        self.apply_core_command(
            command_id,
            expected_graph_revision,
            GraphCommand::Connect {
                connection: Connection {
                    id: ConnectionId::new(),
                    output: PortEndpoint {
                        node_id: output_node_id,
                        port_id: output_port_id,
                    },
                    input: PortEndpoint {
                        node_id: input_node_id,
                        port_id: input_port_id,
                    },
                    extensions: BTreeMap::new(),
                },
            },
        )
    }

    /// Returns a leased, verified shared SDR thumbnail for one project asset.
    ///
    /// # Errors
    ///
    /// Returns an identity, materialization, proxy generation, or cache error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn request_gallery_thumbnail(
        &self,
        asset_id: String,
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
        let asset_id = parse_uuid_id(&asset_id, "asset ID")?;
        self.request_asset_proxy(asset_id, None)
    }

    /// Returns a runtime-only local visual source for an opportunistic native
    /// HDR is preferred when available so the native client can map it to the
    /// actual display. This runtime presentation path is never authoritative.
    ///
    /// # Errors
    ///
    /// Returns an identity, binding, or availability error when no local visual
    /// representation can currently be resolved.
    #[allow(clippy::needless_pass_by_value)]
    pub fn native_thumbnail_source(
        &self,
        asset_id: String,
    ) -> Result<BridgeNativeThumbnailSourceDto, BridgeError> {
        let asset_id: AssetId = parse_uuid_id(&asset_id, "asset ID")?;
        let state = self.lock_state()?;
        let asset = state.project.asset_context.asset(asset_id).ok_or_else(|| {
            BridgeError::InvalidArgument {
                message: format!("unknown project asset {asset_id}"),
            }
        })?;
        let representation =
            preferred_visual_representation(asset).ok_or_else(|| BridgeError::State {
                message: format!("asset {asset_id} has no local visual representation"),
            })?;
        let path = match representation.binding {
            RepresentationBinding::ProjectResource { resource_id } => {
                let resource = state
                    .project
                    .resources
                    .iter()
                    .find(|resource| resource.id == resource_id)
                    .ok_or_else(|| BridgeError::State {
                        message: format!("project resource {resource_id} is missing"),
                    })?;
                self.project_root.join(resource.relative_path.as_str())
            }
            RepresentationBinding::RuntimeResolved { binding_id } => self
                .representation_bindings
                .lock()
                .map_err(|_| BridgeError::State {
                    message: "representation-binding lock was poisoned".to_owned(),
                })?
                .get(&binding_id)
                .cloned()
                .ok_or_else(|| BridgeError::State {
                    message: format!("runtime representation {binding_id} is unavailable"),
                })?,
        };
        if !path.is_file() {
            return Err(BridgeError::State {
                message: format!("visual source {} is unavailable", path.display()),
            });
        }
        Ok(BridgeNativeThumbnailSourceDto {
            asset_id: asset_id.to_string(),
            local_path: path.to_string_lossy().into_owned(),
            source_fingerprint: fingerprint_hex(representation.fingerprint),
            source_verified: representation.revision_evidence
                == RepresentationRevisionEvidence::ContentDigest,
        })
    }

    /// Returns a leased, verified shared HDR-aware proxy for a placed Layout
    /// cell. Rust interprets authored Layout state and resolves its asset.
    ///
    /// # Errors
    ///
    /// Returns an identity, Layout, materialization, generation, or cache error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn request_layout_cell_preview(
        &self,
        layout_node_id: String,
        frame_id: String,
        cell_id: String,
        max_long_edge: u32,
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
        let max_long_edge = NonZeroU32::new(max_long_edge)
            .filter(|pixels| (512..=2_048).contains(&pixels.get()))
            .ok_or_else(|| BridgeError::InvalidArgument {
                message: "Layout authoring preview long edge must be 512–2048 pixels".to_owned(),
            })?;
        let layout_node_id: NodeInstanceId = parse_uuid_id(&layout_node_id, "Layout node ID")?;
        let frame_id = parse_uuid_id(&frame_id, "frame ID")?;
        let cell_id = parse_uuid_id(&cell_id, "cell ID")?;
        let asset_id = {
            let state = self.lock_state()?;
            let node = state
                .project
                .graph
                .nodes
                .iter()
                .find(|node| node.id == layout_node_id)
                .ok_or_else(|| BridgeError::InvalidArgument {
                    message: format!("unknown Layout node {layout_node_id}"),
                })?;
            let layout =
                LayoutState::from_schema_value(node.authored_state.as_ref().ok_or_else(|| {
                    BridgeError::State {
                        message: format!("Layout {layout_node_id} has no authored state"),
                    }
                })?)
                .map_err(|error| BridgeError::State {
                    message: error.to_string(),
                })?;
            layout
                .frames
                .iter()
                .find(|frame| frame.id == frame_id)
                .and_then(|frame| frame.cells.iter().find(|cell| cell.id == cell_id))
                .ok_or_else(|| BridgeError::InvalidArgument {
                    message: format!("unknown Layout cell {cell_id}"),
                })?
                .asset_id
                .ok_or_else(|| BridgeError::InvalidArgument {
                    message: format!("Layout cell {cell_id} has no bound asset"),
                })?
        };
        self.request_asset_proxy(asset_id, Some(max_long_edge))
    }

    /// Creates a one-shot evaluation handle over an immutable graph snapshot.
    ///
    /// # Errors
    ///
    /// Returns a state error when the project session cannot be read.
    pub fn prepare_evaluation(&self) -> Result<Arc<EvaluationHandle>, BridgeError> {
        let state = self.lock_state()?;
        Ok(EvaluationHandle::new(
            state.project.graph.clone(),
            Arc::clone(&self.definitions),
            Arc::clone(&self.runtimes),
            Arc::clone(&self.value_types),
        ))
    }
}

impl PhotaraProject {
    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, ProjectSessionState>, BridgeError> {
        self.state.lock().map_err(|_| BridgeError::State {
            message: "project session lock was poisoned".to_owned(),
        })
    }

    #[cfg(target_os = "macos")]
    fn request_asset_proxy(
        &self,
        asset_id: AssetId,
        authoring_preview_long_edge: Option<NonZeroU32>,
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
        let project = self.lock_state()?.project.clone();
        if project.asset_context.asset(asset_id).is_none() {
            return Err(BridgeError::InvalidArgument {
                message: format!("unknown project asset {asset_id}"),
            });
        }
        let runtime_bindings = self
            .representation_bindings
            .lock()
            .map_err(|_| BridgeError::State {
                message: "representation-binding lock was poisoned".to_owned(),
            })?
            .clone();
        let materializer =
            RuntimeAwareMaterializer::new(&self.project_root, &project, &runtime_bindings);
        let services = AssetContextProjectProxyService::new(
            self.proxy_service.as_ref(),
            &project.asset_context,
            &materializer,
        );
        let profile = if let Some(long_edge) = authoring_preview_long_edge {
            layout_interaction_preview_profile(long_edge)
        } else {
            standard_gallery_preview_profile()
        };
        let artifact = services
            .request_visual_proxy(&ProjectVisualProxyRequest {
                request_id: RequestId::new(),
                project_id: project.project_id,
                asset_id,
                profile,
            })
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        Ok(Arc::new(BridgeProxyReference { asset_id, artifact }))
    }

    #[cfg(not(target_os = "macos"))]
    fn request_asset_proxy(
        &self,
        _asset_id: AssetId,
        _authoring_preview_long_edge: Option<NonZeroU32>,
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
        Err(BridgeError::State {
            message: "no proxy generator is configured for this platform".to_owned(),
        })
    }

    fn apply_core_command(
        &self,
        command_id: CommandId,
        expected_graph_revision: u64,
        command: GraphCommand,
    ) -> BridgeCommandResponseDto {
        let Ok(mut state) = self.lock_state() else {
            return rejected_command(
                command_id,
                expected_graph_revision,
                "photara.bridge.lock-poisoned",
                "project session lock was poisoned".to_owned(),
            );
        };
        let actual = state.project.graph.revision;
        if actual.get() != expected_graph_revision {
            return rejected_command(
                command_id,
                actual.get(),
                "revision-conflict",
                format!(
                    "expected graph revision {expected_graph_revision}, actual {}",
                    actual.get()
                ),
            );
        }
        let envelope = GraphCommandEnvelope {
            command_id,
            graph_id: state.project.graph.id,
            expected_revision: actual,
            command,
        };
        match apply_graph_command(
            &state.project.graph,
            &envelope,
            self.definitions.as_ref(),
            self.value_types.as_ref(),
        ) {
            Ok(result) => {
                let previous = result.previous_revision.get();
                state.project.graph = result.graph;
                sync_requirements(&mut state.project);
                state.dirty = true;
                match project_snapshot(&state, &self.definitions) {
                    Ok(snapshot) => BridgeCommandResponseDto {
                        command_id: command_id.to_string(),
                        applied: true,
                        previous_graph_revision: previous,
                        snapshot: Some(snapshot),
                        error: None,
                    },
                    Err(error) => rejected_command(
                        command_id,
                        previous,
                        "photara.bridge.snapshot-failed",
                        error.to_string(),
                    ),
                }
            }
            Err(error) => BridgeCommandResponseDto {
                command_id: command_id.to_string(),
                applied: false,
                previous_graph_revision: actual.get(),
                snapshot: None,
                error: Some(structured_error(
                    error.code(),
                    error.to_string(),
                    error.diagnostic(),
                    serde_json::to_string(&error).unwrap_or_else(|_| "{}".to_owned()),
                )),
            },
        }
    }

    #[allow(clippy::too_many_lines)]
    fn prepare_asset_binding(
        &self,
        layout_node_id: &str,
        frame_id: &str,
        cell_id: &str,
        asset_id: &str,
    ) -> Result<PreparedLayoutEdit, String> {
        let layout_node_id: NodeInstanceId =
            parse_uuid_id(layout_node_id, "Layout node ID").map_err(|error| error.to_string())?;
        let frame_id = parse_uuid_id(frame_id, "frame ID").map_err(|error| error.to_string())?;
        let cell_id = parse_uuid_id(cell_id, "cell ID").map_err(|error| error.to_string())?;
        let asset_id: AssetId =
            parse_uuid_id(asset_id, "asset ID").map_err(|error| error.to_string())?;
        let state = self.lock_state().map_err(|error| error.to_string())?;
        if state.project.asset_context.asset(asset_id).is_none() {
            return Err(format!("unknown project asset {asset_id}"));
        }
        let graph = &state.project.graph;
        let layout_node = graph
            .nodes
            .iter()
            .find(|node| node.id == layout_node_id)
            .ok_or_else(|| format!("unknown Layout node {layout_node_id}"))?;
        if layout_node.definition.package_id.as_str() != photara_layout_node::PACKAGE_ID {
            return Err(format!("node {layout_node_id} is not a Layout"));
        }
        let source_id = graph
            .connections
            .iter()
            .find(|connection| {
                connection.input.node_id == layout_node_id
                    && connection.input.port_id.as_str() == "assets"
            })
            .map(|connection| connection.output.node_id)
            .ok_or_else(|| format!("Layout {layout_node_id} has no explicit AssetSet input"))?;
        let source = graph
            .nodes
            .iter()
            .find(|node| node.id == source_id)
            .ok_or_else(|| format!("AssetSet source {source_id} is missing"))?;
        let source_state = source
            .authored_state
            .as_ref()
            .ok_or_else(|| format!("AssetSet source {source_id} has no authored state"))?;
        if source_state.schema != asset_set_state_schema() {
            return Err(format!("AssetSet source {source_id} has the wrong schema"));
        }
        let mut assets: AssetSet = serde_json::from_value(source_state.value.clone())
            .map_err(|error| error.to_string())?;
        if !assets.assets.contains(&asset_id) {
            assets.assets.push(asset_id);
        }
        assets
            .validate(&state.project.asset_context)
            .map_err(|error| error.to_string())?;

        let layout = LayoutState::from_schema_value(
            layout_node
                .authored_state
                .as_ref()
                .ok_or_else(|| format!("Layout {layout_node_id} has no authored state"))?,
        )
        .map_err(|error| error.to_string())?;
        let frame = layout
            .frames
            .iter()
            .find(|frame| frame.id == frame_id)
            .ok_or_else(|| format!("unknown frame {frame_id}"))?;
        let mut cell = frame
            .cells
            .iter()
            .find(|cell| cell.id == cell_id)
            .cloned()
            .ok_or_else(|| format!("unknown cell {cell_id}"))?;
        cell.asset_id = Some(asset_id);
        let applied = apply_layout_command(
            &layout,
            LayoutCommand::ReplaceCell {
                frame_id,
                cell_id,
                cell,
            },
        )
        .map_err(|error| error.to_string())?;
        let new_source_state = SchemaValue {
            schema: asset_set_state_schema(),
            value: serde_json::to_value(assets).map_err(|error| error.to_string())?,
        };
        let new_layout_state = applied
            .state
            .to_schema_value()
            .map_err(|error| error.to_string())?;
        let old_source_state = source_state.clone();
        let old_layout_state = layout_node
            .authored_state
            .clone()
            .ok_or_else(|| format!("Layout {layout_node_id} has no authored state"))?;
        Ok(PreparedLayoutEdit {
            forward: GraphCommand::Batch {
                commands: vec![
                    GraphCommand::SetAuthoredState {
                        node_id: source_id,
                        authored_state: Some(new_source_state),
                    },
                    GraphCommand::SetAuthoredState {
                        node_id: layout_node_id,
                        authored_state: Some(new_layout_state),
                    },
                ],
            },
            reverse: GraphCommand::Batch {
                commands: vec![
                    GraphCommand::SetAuthoredState {
                        node_id: source_id,
                        authored_state: Some(old_source_state),
                    },
                    GraphCommand::SetAuthoredState {
                        node_id: layout_node_id,
                        authored_state: Some(old_layout_state),
                    },
                ],
            },
        })
    }

    fn prepare_cell_edit(
        &self,
        node_id: &str,
        frame_id: &str,
        cell_id: &str,
        edit: BridgeLayoutCellEdit,
    ) -> Result<PreparedLayoutEdit, String> {
        let node_id: NodeInstanceId =
            parse_uuid_id(node_id, "node ID").map_err(|error| error.to_string())?;
        let frame_id =
            parse_uuid_id(frame_id, "Layout frame ID").map_err(|error| error.to_string())?;
        let cell_id =
            parse_uuid_id(cell_id, "Layout cell ID").map_err(|error| error.to_string())?;
        let (layout, old_authored_state) = self.layout_state(node_id)?;
        let frame = layout
            .frames
            .iter()
            .find(|frame| frame.id == frame_id)
            .ok_or_else(|| format!("unknown Layout frame {frame_id}"))?;
        let mut cell = frame
            .cells
            .iter()
            .find(|cell| cell.id == cell_id)
            .cloned()
            .ok_or_else(|| format!("unknown Layout cell {cell_id}"))?;
        match edit {
            BridgeLayoutCellEdit::Fit {
                alignment_x,
                alignment_y,
            } => {
                cell.content_mode = CellContentMode::Fit {
                    alignment: bridge_point(alignment_x, alignment_y)?,
                };
            }
            BridgeLayoutCellEdit::Fill { focal_x, focal_y } => {
                cell.content_mode = CellContentMode::Fill {
                    focal_point: bridge_point(focal_x, focal_y)?,
                };
            }
            BridgeLayoutCellEdit::Crop {
                x,
                y,
                width,
                height,
            } => {
                cell.content_mode = CellContentMode::Crop {
                    source_rect: bridge_rect(x, y, width, height)?,
                };
            }
            BridgeLayoutCellEdit::SetQuarterTurn { quarter_turn } => {
                cell.quarter_turn = quarter_turn.into();
            }
        }
        Self::prepare_layout_command(
            node_id,
            old_authored_state,
            &layout,
            LayoutCommand::ReplaceCell {
                frame_id,
                cell_id,
                cell,
            },
        )
    }

    fn prepare_structure_edit(
        &self,
        node_id: &str,
        edit: BridgeLayoutStructureEdit,
    ) -> Result<PreparedLayoutEdit, String> {
        let node_id: NodeInstanceId =
            parse_uuid_id(node_id, "node ID").map_err(|error| error.to_string())?;
        let (layout, old_authored_state) = self.layout_state(node_id)?;
        let command = match edit {
            BridgeLayoutStructureEdit::InsertFrame { index } => LayoutCommand::InsertFrame {
                index: usize::try_from(index).map_err(|_| "frame index is too large")?,
                frame: LayoutFrame::one_cell(),
            },
            BridgeLayoutStructureEdit::RemoveFrame { frame_id } => LayoutCommand::RemoveFrame {
                frame_id: parse_uuid_id(&frame_id, "Layout frame ID")
                    .map_err(|error| error.to_string())?,
            },
            BridgeLayoutStructureEdit::MoveFrame { frame_id, to_index } => {
                LayoutCommand::MoveFrame {
                    frame_id: parse_uuid_id(&frame_id, "Layout frame ID")
                        .map_err(|error| error.to_string())?,
                    to_index: usize::try_from(to_index).map_err(|_| "frame index is too large")?,
                }
            }
            BridgeLayoutStructureEdit::SetFrameArrangement {
                frame_id,
                arrangement,
            } => LayoutCommand::SetFrameArrangement {
                frame_id: parse_uuid_id(&frame_id, "Layout frame ID")
                    .map_err(|error| error.to_string())?,
                arrangement: arrangement.try_into()?,
            },
            BridgeLayoutStructureEdit::InsertCell { frame_id, index } => {
                LayoutCommand::InsertCell {
                    frame_id: parse_uuid_id(&frame_id, "Layout frame ID")
                        .map_err(|error| error.to_string())?,
                    index: usize::try_from(index).map_err(|_| "cell index is too large")?,
                    cell: LayoutCell::new(),
                }
            }
            BridgeLayoutStructureEdit::RemoveCell { frame_id, cell_id } => {
                LayoutCommand::RemoveCell {
                    frame_id: parse_uuid_id(&frame_id, "Layout frame ID")
                        .map_err(|error| error.to_string())?,
                    cell_id: parse_uuid_id(&cell_id, "Layout cell ID")
                        .map_err(|error| error.to_string())?,
                }
            }
        };
        Self::prepare_layout_command(node_id, old_authored_state, &layout, command)
    }

    fn layout_state(&self, node_id: NodeInstanceId) -> Result<(LayoutState, SchemaValue), String> {
        let state = self.lock_state().map_err(|error| error.to_string())?;
        let node = state
            .project
            .graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| format!("unknown node {node_id}"))?;
        if node.definition.package_id.as_str() != photara_layout_node::PACKAGE_ID {
            return Err(format!("node {node_id} is not a Layout"));
        }
        let authored_state = node
            .authored_state
            .clone()
            .ok_or_else(|| format!("node {node_id} has no authored state"))?;
        let layout =
            LayoutState::from_schema_value(&authored_state).map_err(|error| error.to_string())?;
        Ok((layout, authored_state))
    }

    fn prepare_layout_command(
        node_id: NodeInstanceId,
        old_authored_state: SchemaValue,
        layout: &LayoutState,
        command: LayoutCommand,
    ) -> Result<PreparedLayoutEdit, String> {
        let applied = apply_layout_command(layout, command).map_err(|error| error.to_string())?;
        let new_authored_state = applied
            .state
            .to_schema_value()
            .map_err(|error| error.to_string())?;
        Ok(PreparedLayoutEdit {
            forward: GraphCommand::SetAuthoredState {
                node_id,
                authored_state: Some(new_authored_state),
            },
            reverse: GraphCommand::SetAuthoredState {
                node_id,
                authored_state: Some(old_authored_state),
            },
        })
    }

    fn apply_prepared_layout_edit(
        &self,
        command_id: CommandId,
        expected_graph_revision: u64,
        prepared: Result<PreparedLayoutEdit, String>,
        error_code: &str,
    ) -> BridgeCommandResponseDto {
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                return rejected_command(command_id, expected_graph_revision, error_code, error);
            }
        };
        let response = self.apply_core_command(
            command_id,
            expected_graph_revision,
            prepared.forward.clone(),
        );
        if response.applied
            && let Ok(mut state) = self.lock_state()
        {
            state.undo.push(LayoutUndoEntry {
                undo: prepared.reverse,
                redo: prepared.forward,
            });
            state.redo.clear();
        }
        response
    }

    fn apply_history_command(
        &self,
        expected_graph_revision: u64,
        undo: bool,
    ) -> BridgeCommandResponseDto {
        let command_id = CommandId::new();
        let entry = {
            let Ok(mut state) = self.lock_state() else {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.bridge.lock-poisoned",
                    "project session lock was poisoned".to_owned(),
                );
            };
            let stack = if undo {
                &mut state.undo
            } else {
                &mut state.redo
            };
            match stack.pop() {
                Some(entry) => entry,
                None => {
                    return rejected_command(
                        command_id,
                        state.project.graph.revision.get(),
                        if undo {
                            "photara.layout.nothing-to-undo"
                        } else {
                            "photara.layout.nothing-to-redo"
                        },
                        if undo {
                            "there is no Layout command to undo".to_owned()
                        } else {
                            "there is no Layout command to redo".to_owned()
                        },
                    );
                }
            }
        };
        let command = if undo {
            entry.undo.clone()
        } else {
            entry.redo.clone()
        };
        let response = self.apply_core_command(command_id, expected_graph_revision, command);
        if let Ok(mut state) = self.lock_state() {
            let stack = if response.applied {
                if undo {
                    &mut state.redo
                } else {
                    &mut state.undo
                }
            } else if undo {
                &mut state.undo
            } else {
                &mut state.redo
            };
            stack.push(entry);
        }
        response
    }
}

fn project_snapshot(
    state: &ProjectSessionState,
    definitions: &NodePackageRegistry,
) -> Result<BridgeProjectSnapshotDto, BridgeError> {
    let graph_digest =
        canonical_digest(&state.project.graph).map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
    let mut diagnostics = Vec::new();
    let nodes = state
        .project
        .graph
        .nodes
        .iter()
        .map(|node| node_snapshot(node, &state.project, definitions, &mut diagnostics))
        .collect();
    Ok(BridgeProjectSnapshotDto {
        project_id: state.project.project_id.to_string(),
        project_revision: state.project.revision.get(),
        title: state.project.metadata.title.clone(),
        graph: BridgeGraphSnapshotDto {
            graph_id: state.project.graph.id.to_string(),
            revision: state.project.graph.revision.get(),
            digest: graph_digest.to_string(),
            connections: state
                .project
                .graph
                .connections
                .iter()
                .map(|connection| BridgeConnectionDto {
                    connection_id: connection.id.to_string(),
                    output_node_id: connection.output.node_id.to_string(),
                    output_port_id: connection.output.port_id.to_string(),
                    input_node_id: connection.input.node_id.to_string(),
                    input_port_id: connection.input.port_id.to_string(),
                })
                .collect(),
        },
        assets: state
            .project
            .asset_context
            .assets
            .iter()
            .map(|asset| BridgeAssetDto {
                asset_id: asset.id.to_string(),
                display_name: asset.display_name.clone(),
                format_label: preferred_visual_representation(asset)
                    .and_then(representation_format_label),
                representation_count: u64::try_from(asset.representations.len())
                    .unwrap_or(u64::MAX),
                visual_revision: preferred_visual_representation(asset)
                    .map(|representation| fingerprint_hex(representation.fingerprint)),
                visual_revision_verified: preferred_visual_representation(asset).is_some_and(
                    |representation| {
                        representation.revision_evidence
                            == RepresentationRevisionEvidence::ContentDigest
                    },
                ),
            })
            .collect(),
        nodes,
        diagnostics,
        dirty: state.dirty,
    })
}

fn preferred_visual_representation(asset: &ProjectAsset) -> Option<&RepresentationDescriptor> {
    asset
        .representations
        .iter()
        .find(|representation| {
            representation
                .capabilities
                .iter()
                .any(|capability| capability.as_str() == photara_core::HDR_CAPABILITY_ID)
        })
        .or_else(|| {
            asset.representations.iter().find(|representation| {
                representation
                    .capabilities
                    .iter()
                    .any(|capability| capability.as_str() == photara_core::IMAGE_CAPABILITY_ID)
            })
        })
}

fn representation_format_label(representation: &RepresentationDescriptor) -> Option<String> {
    representation
        .extensions
        .get(REPRESENTATION_FORMAT_EXTENSION_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            representation
                .capabilities
                .iter()
                .any(|capability| capability.as_str() == photara_core::TIFF_CAPABILITY_ID)
                .then(|| "TIFF".to_owned())
        })
}

fn node_snapshot(
    node: &NodeInstance,
    project: &ProjectDocument,
    definitions: &NodePackageRegistry,
    project_diagnostics: &mut Vec<BridgeDiagnosticDto>,
) -> BridgeNodeDto {
    let definition = definitions.resolve(&node.definition);
    let display_name = definition.map_or_else(
        || node.definition.definition_id.to_string(),
        |value| value.display_name.clone(),
    );
    let presentation = definition
        .and_then(|value| node_presentation(value).ok())
        .flatten();
    let brand_name = presentation
        .as_ref()
        .map_or_else(|| display_name.clone(), |value| value.brand.name.clone());
    let (layout, mut diagnostics) = layout_inspection(node, &project.graph);
    let disk = disk_inspection(node, &mut diagnostics);
    let ports = node_port_inspections(node, project, definitions, layout.as_ref());
    let output_summary = ports
        .iter()
        .filter(|port| port.direction == BridgePortDirection::Output)
        .flat_map(|port| port.summary.iter().cloned())
        .collect();
    let status = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == BridgeDiagnosticSeverity::Error)
    {
        "Error"
    } else if diagnostics.is_empty() {
        "Ready"
    } else {
        "Warning"
    };
    project_diagnostics.extend(diagnostics.iter().cloned());
    BridgeNodeDto {
        node_id: node.id.to_string(),
        display_name,
        package_id: node.definition.package_id.to_string(),
        package_version: node.definition.package_version.to_string(),
        definition_id: node.definition.definition_id.to_string(),
        definition_version: node.definition.definition_version.get(),
        brand_name,
        icon_resource_id: presentation.as_ref().map_or_else(
            || "photara.node.generic".to_owned(),
            |value| value.brand.icon_resource_id.clone(),
        ),
        theme_color_role: presentation
            .as_ref()
            .and_then(|value| value.brand.theme_color_role.clone()),
        accent_srgb_hex: presentation
            .as_ref()
            .and_then(|value| value.brand.accent_srgb_hex.clone()),
        inspector_contribution_id: presentation
            .as_ref()
            .and_then(|value| value.inspector_contribution_id.clone()),
        workspace_contribution_id: presentation
            .as_ref()
            .and_then(|value| value.workspace_contribution_id.clone()),
        default_activation_id: presentation
            .as_ref()
            .and_then(|value| value.default_activation_id.clone()),
        has_workspace: presentation
            .as_ref()
            .is_some_and(|value| value.workspace_contribution_id.is_some()),
        status: status.to_owned(),
        ports,
        output_summary,
        disk,
        layout,
        diagnostics: std::mem::take(&mut diagnostics),
    }
}

fn disk_inspection(
    node: &NodeInstance,
    diagnostics: &mut Vec<BridgeDiagnosticDto>,
) -> Option<BridgeDiskInspectionDto> {
    if node.definition.package_id.as_str() != photara_disk_node::PACKAGE_ID {
        return None;
    }
    let state = node
        .authored_state
        .as_ref()
        .ok_or_else(|| "Disk authored state is missing".to_owned())
        .and_then(DiskFolderState::from_schema_value);
    match state {
        Ok(state) => Some(BridgeDiskInspectionDto {
            folder_binding_id: state.folder_binding_id.to_string(),
            recursive: state.recursive,
            accepted_asset_count: u64::try_from(state.accepted_assets.assets.len())
                .unwrap_or(u64::MAX),
        }),
        Err(message) => {
            diagnostics.push(BridgeDiagnosticDto {
                code: "photara.disk.invalid-authored-state".to_owned(),
                severity: BridgeDiagnosticSeverity::Error,
                message,
                node_id: Some(node.id.to_string()),
                port_id: None,
            });
            None
        }
    }
}

fn node_port_inspections(
    node: &NodeInstance,
    project: &ProjectDocument,
    definitions: &NodePackageRegistry,
    layout: Option<&BridgeLayoutInspectionDto>,
) -> Vec<BridgePortInspectionDto> {
    let Some(definition) = definitions.resolve(&node.definition) else {
        return Vec::new();
    };
    definition
        .ports
        .iter()
        .map(|port| {
            let connection =
                project
                    .graph
                    .connections
                    .iter()
                    .find(|connection| match port.direction {
                        PortDirection::Input => {
                            connection.input.node_id == node.id
                                && connection.input.port_id == port.id
                        }
                        PortDirection::Output => {
                            connection.output.node_id == node.id
                                && connection.output.port_id == port.id
                        }
                    });
            let connected_node_id = connection.map(|connection| match port.direction {
                PortDirection::Input => connection.output.node_id,
                PortDirection::Output => connection.input.node_id,
            });
            let connected_node_name = connected_node_id.and_then(|connected_id| {
                project
                    .graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == connected_id)
                    .map(|candidate| {
                        definitions.resolve(&candidate.definition).map_or_else(
                            || candidate.definition.definition_id.to_string(),
                            |definition| definition.display_name.clone(),
                        )
                    })
            });
            BridgePortInspectionDto {
                port_id: port.id.to_string(),
                direction: match port.direction {
                    PortDirection::Input => BridgePortDirection::Input,
                    PortDirection::Output => BridgePortDirection::Output,
                },
                value_type_id: port.value_type.id.to_string(),
                value_type_version: port.value_type.version.get(),
                connected_node_id: connected_node_id.map(|id| id.to_string()),
                connected_node_name,
                summary: port_summary(node, port.direction, port.id.as_str(), project, layout),
            }
        })
        .collect()
}

fn port_summary(
    node: &NodeInstance,
    direction: PortDirection,
    port_id: &str,
    project: &ProjectDocument,
    layout: Option<&BridgeLayoutInspectionDto>,
) -> Vec<BridgeInspectionFieldDto> {
    if let Some(assets) = asset_set_visible_at_port(node, direction, port_id, &project.graph) {
        let representations = assets
            .assets
            .iter()
            .filter_map(|asset_id| project.asset_context.asset(*asset_id))
            .map(|asset| asset.representations.len())
            .sum::<usize>();
        return vec![
            inspection_field("Assets", assets.assets.len()),
            inspection_field("Representations", representations),
        ];
    }
    if direction == PortDirection::Output
        && port_id == "layout"
        && let Some(layout) = layout
    {
        return vec![
            inspection_field("Frames", layout.frames.len()),
            BridgeInspectionFieldDto {
                label: "Canvas".to_owned(),
                value: format!(
                    "{} × {}",
                    layout.canvas.width_pixels, layout.canvas.height_pixels
                ),
            },
            BridgeInspectionFieldDto {
                label: "Status".to_owned(),
                value: "Resolved".to_owned(),
            },
        ];
    }
    Vec::new()
}

fn asset_set_visible_at_port(
    node: &NodeInstance,
    direction: PortDirection,
    port_id: &str,
    graph: &GraphDocument,
) -> Option<AssetSet> {
    let source = match direction {
        PortDirection::Output if port_id == "assets" => Some(node),
        PortDirection::Input if port_id == "assets" => graph
            .connections
            .iter()
            .find(|connection| {
                connection.input.node_id == node.id && connection.input.port_id.as_str() == port_id
            })
            .and_then(|connection| {
                graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == connection.output.node_id)
            }),
        _ => None,
    }?;
    let state = source.authored_state.as_ref()?;
    if state.schema == asset_set_state_schema() {
        serde_json::from_value(state.value.clone()).ok()
    } else if state.schema == photara_disk_node::disk_state_schema() {
        DiskFolderState::from_schema_value(state)
            .ok()
            .map(|state| state.accepted_assets)
    } else {
        None
    }
}

fn inspection_field(label: &str, value: usize) -> BridgeInspectionFieldDto {
    BridgeInspectionFieldDto {
        label: label.to_owned(),
        value: value.to_string(),
    }
}

fn layout_inspection(
    node: &NodeInstance,
    graph: &GraphDocument,
) -> (Option<BridgeLayoutInspectionDto>, Vec<BridgeDiagnosticDto>) {
    if node.definition.package_id.as_str() != photara_layout_node::PACKAGE_ID {
        return (None, Vec::new());
    }
    let Some(value) = node.authored_state.as_ref() else {
        return (
            None,
            vec![simple_diagnostic(
                "photara.layout.missing-authored-state",
                "Layout node has no authored state",
                Some(node.id),
            )],
        );
    };
    match LayoutState::from_schema_value(value) {
        Ok(layout) => {
            let (assets, mut diagnostics) = match explicit_layout_assets(graph, node.id) {
                Ok(assets) => (assets, Vec::new()),
                Err(error) => (
                    AssetSet::default(),
                    vec![BridgeDiagnosticDto {
                        code: "photara.layout.asset-input-unavailable".to_owned(),
                        severity: BridgeDiagnosticSeverity::Warning,
                        message: error.to_string(),
                        node_id: Some(node.id.to_string()),
                        port_id: Some("assets".to_owned()),
                    }],
                ),
            };
            match BridgeLayoutInspectionDto::try_from((&layout, &assets)) {
                Ok(inspection) => (Some(inspection), diagnostics),
                Err(error) => {
                    diagnostics.push(simple_diagnostic(
                        "photara.layout.inspection-failed",
                        error.to_string(),
                        Some(node.id),
                    ));
                    (None, diagnostics)
                }
            }
        }
        Err(error) => (
            None,
            vec![simple_diagnostic(
                "photara.layout.invalid-authored-state",
                error.to_string(),
                Some(node.id),
            )],
        ),
    }
}

fn explicit_layout_assets(
    graph: &GraphDocument,
    layout_node_id: NodeInstanceId,
) -> Result<AssetSet, BridgeError> {
    let source_id = graph
        .connections
        .iter()
        .find(|connection| {
            connection.input.node_id == layout_node_id
                && connection.input.port_id.as_str() == "assets"
        })
        .map(|connection| connection.output.node_id)
        .ok_or_else(|| BridgeError::State {
            message: format!("Layout {layout_node_id} has no explicit AssetSet input"),
        })?;
    let source = graph
        .nodes
        .iter()
        .find(|node| node.id == source_id)
        .ok_or_else(|| BridgeError::State {
            message: format!("AssetSet source {source_id} is missing"),
        })?;
    let authored_state = source
        .authored_state
        .as_ref()
        .ok_or_else(|| BridgeError::State {
            message: format!("AssetSet source {source_id} has no authored state"),
        })?;
    if authored_state.schema == asset_set_state_schema() {
        return serde_json::from_value(authored_state.value.clone()).map_err(|error| {
            BridgeError::State {
                message: error.to_string(),
            }
        });
    }
    if authored_state.schema == photara_disk_node::disk_state_schema() {
        return DiskFolderState::from_schema_value(authored_state)
            .map(|state| state.accepted_assets)
            .map_err(|message| BridgeError::State { message });
    }
    Err(BridgeError::State {
        message: format!("AssetSet source {source_id} has an unsupported authored-state schema"),
    })
}

impl TryFrom<(&LayoutState, &AssetSet)> for BridgeLayoutInspectionDto {
    type Error = BridgeError;

    #[allow(clippy::too_many_lines)]
    fn try_from((layout, assets): (&LayoutState, &AssetSet)) -> Result<Self, Self::Error> {
        let size = layout
            .canvas
            .pixel_size()
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        let (kind, horizontal_units, vertical_units, long_edge_pixels) = match layout.canvas {
            LayoutCanvas::Bundled {
                profile,
                long_edge_pixels,
                ..
            } => (
                match profile {
                    BundledCanvasProfile::Portrait3x4 => BridgeLayoutCanvasKind::Portrait3x4,
                    BundledCanvasProfile::Vertical9x16 => BridgeLayoutCanvasKind::Vertical9x16,
                },
                None,
                None,
                Some(long_edge_pixels.get()),
            ),
            LayoutCanvas::CustomPixels { .. } => {
                (BridgeLayoutCanvasKind::CustomPixels, None, None, None)
            }
            LayoutCanvas::CustomAspect {
                horizontal_units,
                vertical_units,
                long_edge_pixels,
            } => (
                BridgeLayoutCanvasKind::CustomAspect,
                Some(horizontal_units.get()),
                Some(vertical_units.get()),
                Some(long_edge_pixels.get()),
            ),
        };
        let plan = resolve_layout(layout, assets).map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
        let frames = layout
            .frames
            .iter()
            .zip(&plan.frames)
            .enumerate()
            .map(
                |(index, (frame, resolved_frame))| BridgeLayoutFrameInspectionDto {
                    frame_id: frame.id.to_string(),
                    index: u64::try_from(index).unwrap_or(u64::MAX),
                    arrangement: match frame.arrangement {
                        CellArrangement::One => BridgeLayoutArrangement::One,
                        CellArrangement::HorizontalStack => {
                            BridgeLayoutArrangement::HorizontalStack
                        }
                        CellArrangement::VerticalStack => BridgeLayoutArrangement::VerticalStack,
                        CellArrangement::UniformGrid { .. } => BridgeLayoutArrangement::UniformGrid,
                        CellArrangement::Custom => BridgeLayoutArrangement::Custom,
                    },
                    cells: frame
                        .cells
                        .iter()
                        .zip(&resolved_frame.cells)
                        .enumerate()
                        .map(|(index, (cell, resolved_cell))| {
                            let (content_mode, focal_x, focal_y, crop_rect) =
                                match cell.content_mode {
                                    CellContentMode::Fit { alignment } => (
                                        BridgeLayoutContentMode::Fit,
                                        Some(alignment.x.get()),
                                        Some(alignment.y.get()),
                                        None,
                                    ),
                                    CellContentMode::Fill { focal_point } => (
                                        BridgeLayoutContentMode::Fill,
                                        Some(focal_point.x.get()),
                                        Some(focal_point.y.get()),
                                        None,
                                    ),
                                    CellContentMode::Crop { source_rect } => (
                                        BridgeLayoutContentMode::Crop,
                                        None,
                                        None,
                                        Some(source_rect.into()),
                                    ),
                                };
                            BridgeLayoutCellInspectionDto {
                                cell_id: cell.id.to_string(),
                                index: u64::try_from(index).unwrap_or(u64::MAX),
                                asset_id: cell.asset_id.map(|id| id.to_string()),
                                content_mode,
                                focal_x,
                                focal_y,
                                crop_rect,
                                custom_rect: cell.custom_rect.map(Into::into),
                                resolved_rect: resolved_cell.normalized_rect.into(),
                                resolved_pixel_rect: BridgePixelRectDto {
                                    x: resolved_cell.pixel_rect.x,
                                    y: resolved_cell.pixel_rect.y,
                                    width: resolved_cell.pixel_rect.width.get(),
                                    height: resolved_cell.pixel_rect.height.get(),
                                },
                                quarter_turn: cell.quarter_turn.into(),
                            }
                        })
                        .collect(),
                },
            )
            .collect();
        Ok(Self {
            authored_state_digest: layout
                .digest()
                .map_err(|error| BridgeError::State {
                    message: error.to_string(),
                })?
                .to_string(),
            canvas: BridgeLayoutCanvasInspectionDto {
                kind,
                width_pixels: size.width.get(),
                height_pixels: size.height.get(),
                horizontal_units,
                vertical_units,
                long_edge_pixels,
            },
            frames,
        })
    }
}

impl From<NormalizedRect> for BridgeNormalizedRectDto {
    fn from(rect: NormalizedRect) -> Self {
        Self {
            x: rect.x.get(),
            y: rect.y.get(),
            width: rect.width.get(),
            height: rect.height.get(),
        }
    }
}

impl From<QuarterTurn> for BridgeQuarterTurn {
    fn from(value: QuarterTurn) -> Self {
        match value {
            QuarterTurn::Zero => Self::Zero,
            QuarterTurn::Clockwise90 => Self::Clockwise90,
            QuarterTurn::Clockwise180 => Self::Clockwise180,
            QuarterTurn::Clockwise270 => Self::Clockwise270,
        }
    }
}

impl From<BridgeQuarterTurn> for QuarterTurn {
    fn from(value: BridgeQuarterTurn) -> Self {
        match value {
            BridgeQuarterTurn::Zero => Self::Zero,
            BridgeQuarterTurn::Clockwise90 => Self::Clockwise90,
            BridgeQuarterTurn::Clockwise180 => Self::Clockwise180,
            BridgeQuarterTurn::Clockwise270 => Self::Clockwise270,
        }
    }
}

impl TryFrom<BridgeLayoutArrangementEdit> for CellArrangement {
    type Error = String;

    fn try_from(value: BridgeLayoutArrangementEdit) -> Result<Self, Self::Error> {
        match value {
            BridgeLayoutArrangementEdit::One => Ok(Self::One),
            BridgeLayoutArrangementEdit::HorizontalStack => Ok(Self::HorizontalStack),
            BridgeLayoutArrangementEdit::VerticalStack => Ok(Self::VerticalStack),
            BridgeLayoutArrangementEdit::UniformGrid { columns } => Ok(Self::UniformGrid {
                columns: NonZeroU32::new(columns)
                    .ok_or_else(|| "grid columns must be greater than zero".to_owned())?,
            }),
            BridgeLayoutArrangementEdit::Custom => Ok(Self::Custom),
        }
    }
}

fn bridge_point(x: u32, y: u32) -> Result<NormalizedPoint, String> {
    Ok(NormalizedPoint {
        x: NormalizedUnit::new(x).map_err(|error| error.to_string())?,
        y: NormalizedUnit::new(y).map_err(|error| error.to_string())?,
    })
}

fn bridge_rect(x: u32, y: u32, width: u32, height: u32) -> Result<NormalizedRect, String> {
    Ok(NormalizedRect {
        x: NormalizedUnit::new(x).map_err(|error| error.to_string())?,
        y: NormalizedUnit::new(y).map_err(|error| error.to_string())?,
        width: NormalizedUnit::new(width).map_err(|error| error.to_string())?,
        height: NormalizedUnit::new(height).map_err(|error| error.to_string())?,
    })
}

impl From<Diagnostic> for BridgeDiagnosticDto {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: diagnostic.severity.into(),
            message: diagnostic.message,
            node_id: diagnostic.node_instance_id.map(|id| id.to_string()),
            port_id: diagnostic.port_id.map(|id| id.to_string()),
        }
    }
}

impl From<DiagnosticSeverity> for BridgeDiagnosticSeverity {
    fn from(severity: DiagnosticSeverity) -> Self {
        match severity {
            DiagnosticSeverity::Info => Self::Info,
            DiagnosticSeverity::Warning => Self::Warning,
            DiagnosticSeverity::Error => Self::Error,
        }
    }
}

pub(crate) fn structured_error(
    code: impl Into<String>,
    message: impl Into<String>,
    diagnostic: Diagnostic,
    details_json: String,
) -> BridgeStructuredErrorDto {
    BridgeStructuredErrorDto {
        code: code.into(),
        message: message.into(),
        diagnostic: diagnostic.into(),
        details_json,
    }
}

fn rejected_command(
    command_id: CommandId,
    previous_graph_revision: u64,
    code: &str,
    message: String,
) -> BridgeCommandResponseDto {
    BridgeCommandResponseDto {
        command_id: command_id.to_string(),
        applied: false,
        previous_graph_revision,
        snapshot: None,
        error: Some(BridgeStructuredErrorDto {
            code: code.to_owned(),
            message: message.clone(),
            diagnostic: BridgeDiagnosticDto {
                code: code.to_owned(),
                severity: BridgeDiagnosticSeverity::Error,
                message,
                node_id: None,
                port_id: None,
            },
            details_json: "{}".to_owned(),
        }),
    }
}

fn simple_diagnostic(
    code: &str,
    message: impl Into<String>,
    node_id: Option<NodeInstanceId>,
) -> BridgeDiagnosticDto {
    BridgeDiagnosticDto {
        code: code.to_owned(),
        severity: BridgeDiagnosticSeverity::Error,
        message: message.into(),
        node_id: node_id.map(|id| id.to_string()),
        port_id: None,
    }
}

fn proxy_descriptor(asset_id: AssetId, artifact: &ProxyArtifact) -> BridgeProxyDescriptorDto {
    let descriptor = &artifact.descriptor;
    let (dynamic_range, reference_white_nits, hdr_headroom_millistops) =
        match descriptor.dynamic_range {
            photara_core::ProxyDynamicRangeDescription::Sdr {
                reference_white_nits,
            } => (
                BridgeProxyDynamicRange::Sdr,
                reference_white_nits.map(NonZeroU32::get),
                None,
            ),
            photara_core::ProxyDynamicRangeDescription::Hdr {
                reference_white_nits,
                headroom_millistops,
            } => (
                BridgeProxyDynamicRange::Hdr,
                reference_white_nits.map(NonZeroU32::get),
                headroom_millistops,
            ),
        };
    BridgeProxyDescriptorDto {
        asset_id: asset_id.to_string(),
        local_path: artifact.local_path.to_string_lossy().into_owned(),
        disposition: match artifact.disposition {
            ProxyArtifactDisposition::CacheHit => BridgeProxyDisposition::CacheHit,
            ProxyArtifactDisposition::Generated => BridgeProxyDisposition::Generated,
            ProxyArtifactDisposition::SharedInFlight => BridgeProxyDisposition::SharedInFlight,
        },
        cache_key: descriptor.cache_key.to_string(),
        source_fingerprint: fingerprint_hex(descriptor.source_fingerprint),
        content_fingerprint: fingerprint_hex(descriptor.content_fingerprint),
        profile_id: descriptor.profile.id.to_string(),
        profile_version: descriptor.profile.version.get(),
        pixel_width: descriptor.pixel_width.get(),
        pixel_height: descriptor.pixel_height.get(),
        channel_depth: match descriptor.channel_depth {
            photara_core::ProxyChannelDepth::U8 => BridgeProxyChannelDepth::U8,
            photara_core::ProxyChannelDepth::U16 => BridgeProxyChannelDepth::U16,
            photara_core::ProxyChannelDepth::F16 => BridgeProxyChannelDepth::F16,
            photara_core::ProxyChannelDepth::F32 => BridgeProxyChannelDepth::F32,
        },
        has_alpha: descriptor.alpha == photara_core::ProxyAlphaPolicy::Preserve,
        encoding_id: descriptor.encoding.id.to_string(),
        encoding_version: descriptor.encoding.version.get(),
        color_space_id: descriptor.color_space.to_string(),
        embedded_icc_fingerprint: descriptor.embedded_icc_fingerprint.map(fingerprint_hex),
        dynamic_range,
        reference_white_nits,
        hdr_headroom_millistops,
        pixels_are_orientation_normalized: descriptor.orientation
            == photara_core::ProxyStoredOrientation::PixelsNormalized,
        byte_length: descriptor.byte_length,
    }
}

fn fingerprint_hex(fingerprint: photara_core::RepresentationFingerprint) -> String {
    let mut result = String::with_capacity(64);
    for byte in fingerprint.as_bytes() {
        write!(&mut result, "{byte:02x}").expect("writing to String is infallible");
    }
    result
}

fn layout_canvas(canvas: BridgeLayoutCanvas) -> Result<LayoutCanvas, BridgeError> {
    let nonzero = |value, label: &str| {
        NonZeroU32::new(value).ok_or_else(|| BridgeError::InvalidArgument {
            message: format!("{label} must be greater than zero"),
        })
    };
    match canvas {
        BridgeLayoutCanvas::Portrait3x4 { long_edge_pixels } => Ok(LayoutCanvas::portrait_3x4(
            nonzero(long_edge_pixels, "long edge")?,
        )),
        BridgeLayoutCanvas::Vertical9x16 { long_edge_pixels } => Ok(LayoutCanvas::vertical_9x16(
            nonzero(long_edge_pixels, "long edge")?,
        )),
        BridgeLayoutCanvas::CustomPixels { width, height } => Ok(LayoutCanvas::CustomPixels {
            width: nonzero(width, "width")?,
            height: nonzero(height, "height")?,
        }),
        BridgeLayoutCanvas::CustomAspect {
            horizontal_units,
            vertical_units,
            long_edge_pixels,
        } => Ok(LayoutCanvas::CustomAspect {
            horizontal_units: nonzero(horizontal_units, "horizontal units")?,
            vertical_units: nonzero(vertical_units, "vertical units")?,
            long_edge_pixels: nonzero(long_edge_pixels, "long edge")?,
        }),
    }
}

fn parse_uuid_id<T: DeserializeOwned>(value: &str, label: &str) -> Result<T, BridgeError> {
    serde_json::from_value(json!(value)).map_err(|_| BridgeError::InvalidArgument {
        message: format!("{label} {value:?} is not a UUID"),
    })
}

fn sync_requirements(project: &mut ProjectDocument) {
    let requirements: BTreeSet<_> = project
        .graph
        .nodes
        .iter()
        .map(|node| PackageRequirement {
            package_id: node.definition.package_id.clone(),
            package_version: node.definition.package_version.clone(),
        })
        .collect();
    project.required_packages = requirements.into_iter().collect();
}

impl From<LayoutCommandError> for BridgeError {
    fn from(error: LayoutCommandError) -> Self {
        Self::State {
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Condvar};

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("photara-bridge-{}", ProjectId::new()));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Default)]
    struct ObserverState {
        progress: Vec<BridgeEvaluationProgressDto>,
        finished: Option<BridgeEvaluationFinishedDto>,
    }

    #[derive(Default)]
    struct TestObserver {
        state: Mutex<ObserverState>,
        complete: Condvar,
    }

    impl EvaluationObserver for TestObserver {
        fn on_progress(&self, progress: BridgeEvaluationProgressDto) {
            self.state.lock().unwrap().progress.push(progress);
        }

        fn on_finished(&self, result: BridgeEvaluationFinishedDto) {
            let mut state = self.state.lock().unwrap();
            state.finished = Some(result);
            self.complete.notify_all();
        }
    }

    impl TestObserver {
        fn wait(&self) -> std::sync::MutexGuard<'_, ObserverState> {
            let state = self.state.lock().unwrap();
            self.complete
                .wait_while(state, |state| state.finished.is_none())
                .unwrap()
        }
    }

    fn definition_ref(definition: &BridgeAvailableNodeDefinitionDto) -> BridgeNodeDefinitionRefDto {
        BridgeNodeDefinitionRefDto {
            package_id: definition.package_id.clone(),
            package_version: definition.package_version.clone(),
            definition_id: definition.definition_id.clone(),
            definition_version: definition.definition_version,
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn branded_catalog_drives_disk_scan_connection_and_portable_reopen() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
            1,
        )
        .unwrap();
        let catalog = app.available_node_definitions();
        assert_eq!(catalog.len(), 2);
        let disk_definition = catalog
            .iter()
            .find(|definition| definition.definition_id == photara_disk_node::DEFINITION_ID)
            .unwrap();
        assert_eq!(disk_definition.catalog_path, ["Input", "Filesystem"]);
        assert_eq!(disk_definition.icon_resource_id, "photara.disk.folder");
        assert!(disk_definition.workspace_contribution_id.is_none());
        assert_eq!(
            disk_definition.default_activation_id.as_deref(),
            Some("photara.disk.open-folder")
        );
        let layout_definition = catalog
            .iter()
            .find(|definition| definition.definition_id == photara_layout_node::DEFINITION_ID)
            .unwrap();
        assert_eq!(layout_definition.catalog_path, ["Create", "Layout"]);
        assert!(layout_definition.workspace_contribution_id.is_some());
        assert_eq!(
            layout_definition.default_activation_id.as_deref(),
            Some("photara.layout.open-workspace")
        );

        let project = app.create_project("Live folder".to_owned()).unwrap();
        let initial = project.snapshot().unwrap();
        let added_disk = project.add_node(initial.graph.revision, definition_ref(disk_definition));
        assert!(added_disk.applied);
        let added_disk = added_disk.snapshot.unwrap();
        let disk = added_disk
            .nodes
            .iter()
            .find(|node| node.disk.is_some())
            .unwrap();
        assert_eq!(disk.icon_resource_id, "photara.disk.folder");
        assert!(!disk.has_workspace);
        let disk_id = disk.node_id.clone();
        let semantic_digest_before_attach = added_disk.graph.digest.clone();

        let folder = root.0.join("authorized-folder");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("portrait.tiff"), b"first TIFF revision").unwrap();
        let binding = project
            .attach_disk_folder(disk_id.clone(), folder.to_string_lossy().into_owned())
            .unwrap();
        assert_eq!(binding.node_id, disk_id);
        assert_eq!(
            project.snapshot().unwrap().graph.digest,
            semantic_digest_before_attach
        );

        let discovered = project.discover_disk_folder(added_disk.graph.revision, disk_id.clone());
        assert!(discovered.applied, "{:?}", discovered.error);
        let discovered = discovered.snapshot.unwrap();
        assert_eq!(discovered.assets.len(), 1);
        assert!(!discovered.assets[0].visual_revision_verified);
        let observed_source = project
            .native_thumbnail_source(discovered.assets[0].asset_id.clone())
            .unwrap();
        assert!(!observed_source.source_verified);

        let scanned = project.scan_disk_folder(discovered.graph.revision, disk_id.clone());
        assert!(scanned.applied, "{:?}", scanned.error);
        let scanned = scanned.snapshot.unwrap();
        assert_eq!(scanned.assets.len(), 1);
        assert!(scanned.assets[0].visual_revision_verified);
        let thumbnail_source = project
            .native_thumbnail_source(scanned.assets[0].asset_id.clone())
            .unwrap();
        assert!(thumbnail_source.source_verified);
        assert_eq!(thumbnail_source.asset_id, scanned.assets[0].asset_id);
        assert_eq!(
            PathBuf::from(thumbnail_source.local_path),
            folder.join("portrait.tiff")
        );
        assert_eq!(thumbnail_source.source_fingerprint.len(), 64);
        assert_eq!(
            project.snapshot().unwrap().graph.digest,
            scanned.graph.digest
        );
        assert_eq!(
            scanned
                .nodes
                .iter()
                .find(|node| node.node_id == disk_id)
                .unwrap()
                .disk
                .as_ref()
                .unwrap()
                .accepted_asset_count,
            1
        );

        let added_layout =
            project.add_node(scanned.graph.revision, definition_ref(layout_definition));
        assert!(added_layout.applied);
        let added_layout = added_layout.snapshot.unwrap();
        let layout = added_layout
            .nodes
            .iter()
            .find(|node| node.definition_id == photara_layout_node::DEFINITION_ID)
            .unwrap();
        assert!(layout.layout.is_some());
        let connected = project.connect_nodes(
            added_layout.graph.revision,
            disk_id,
            "assets".to_owned(),
            layout.node_id.clone(),
            "assets".to_owned(),
        );
        assert!(connected.applied, "{:?}", connected.error);
        let connected = connected.snapshot.unwrap();
        assert_eq!(
            connected
                .nodes
                .iter()
                .find(|node| node.node_id == layout.node_id)
                .unwrap()
                .ports[0]
                .summary[0]
                .value,
            "1"
        );

        let saved = project.save().unwrap();
        let cleared = project.clear_disk_assets(saved.graph.revision, binding.node_id.clone());
        assert!(cleared.applied, "{:?}", cleared.error);
        let cleared = cleared.snapshot.unwrap();
        assert!(cleared.assets.is_empty());
        assert_eq!(
            cleared
                .nodes
                .iter()
                .find(|node| node.node_id == binding.node_id)
                .unwrap()
                .disk
                .as_ref()
                .unwrap()
                .accepted_asset_count,
            0
        );
        let reopened = app.open_project(saved.project_id.clone()).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), saved);
        assert!(matches!(
            reopened.scan_disk_folder(saved.graph.revision, binding.node_id).error,
            Some(error) if error.code == "photara.disk.binding-unavailable"
        ));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn production_facade_saves_reopens_commands_and_undoes_layout() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
            1,
        )
        .unwrap();
        let project = app.create_project("Bridge project".to_owned()).unwrap();
        let initial = project.snapshot().unwrap();
        assert!(!initial.dirty);

        let added = project.add_layout_node(
            initial.graph.revision,
            BridgeLayoutCanvas::Portrait3x4 {
                long_edge_pixels: 4000,
            },
        );
        assert!(added.applied);
        let added = added.snapshot.unwrap();
        assert!(added.dirty);
        assert_eq!(added.nodes.len(), 2);
        let node = added
            .nodes
            .iter()
            .find(|node| node.layout.is_some())
            .unwrap();
        let layout = node.layout.as_ref().unwrap();
        assert!(node.has_workspace);
        assert_eq!(node.status, "Ready");
        assert_eq!(node.ports.len(), 2);
        assert_eq!(node.ports[0].direction, BridgePortDirection::Input);
        assert_eq!(
            node.ports[0].connected_node_name.as_deref(),
            Some("Project Assets")
        );
        assert_eq!(node.ports[0].summary[0].value, "0");
        let source = added
            .nodes
            .iter()
            .find(|node| node.layout.is_none())
            .unwrap();
        assert!(!source.has_workspace);
        assert_eq!(source.ports[0].direction, BridgePortDirection::Output);
        let frame_id = layout.frames[0].frame_id.clone();
        let cell_id = layout.frames[0].cells[0].cell_id.clone();
        let node_id = node.node_id.clone();

        let hdr_source = root.0.join("source-hdr.tiff");
        let sdr_source = root.0.join("source-sdr.tiff");
        fs::write(&hdr_source, b"hdr fixture").unwrap();
        fs::write(&sdr_source, b"sdr fixture").unwrap();
        let imported = project
            .import_local_tiff_pair(
                "Bound photograph".to_owned(),
                hdr_source.to_string_lossy().into_owned(),
                sdr_source.to_string_lossy().into_owned(),
            )
            .unwrap();
        assert_eq!(imported.snapshot.graph.digest, added.graph.digest);
        let native_hdr_path = PathBuf::from(
            project
                .native_thumbnail_source(imported.asset_id.clone())
                .unwrap()
                .local_path,
        );
        assert_eq!(
            native_hdr_path.file_name().and_then(|name| name.to_str()),
            Some("flattened-hdr.tiff")
        );
        assert_eq!(fs::read(native_hdr_path).unwrap(), b"hdr fixture");
        let bound = project.bind_asset_to_layout(
            imported.snapshot.graph.revision,
            node_id.clone(),
            frame_id.clone(),
            cell_id.clone(),
            imported.asset_id.clone(),
        );
        assert!(bound.applied);
        let bound = bound.snapshot.unwrap();
        assert_ne!(bound.graph.digest, added.graph.digest);
        assert_eq!(
            bound
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .frames[0]
                .cells[0]
                .asset_id,
            Some(imported.asset_id)
        );
        assert_eq!(
            bound
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .ports[0]
                .summary[0]
                .value,
            "1"
        );

        let undone_binding = project.undo_layout(bound.graph.revision);
        assert!(undone_binding.applied);
        let undone_binding = undone_binding.snapshot.unwrap();
        assert_eq!(
            undone_binding
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .frames[0]
                .cells[0]
                .asset_id,
            None
        );
        let redone_binding = project.redo_layout(undone_binding.graph.revision);
        assert!(redone_binding.applied);
        let bound = redone_binding.snapshot.unwrap();
        assert_eq!(
            bound
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .frames[0]
                .cells[0]
                .resolved_rect,
            BridgeNormalizedRectDto {
                x: 0,
                y: 0,
                width: 1_000_000,
                height: 1_000_000,
            }
        );

        let cropped = project.set_layout_cell_crop(
            bound.graph.revision,
            node_id,
            frame_id,
            cell_id,
            100_000,
            100_000,
            800_000,
            800_000,
        );
        assert!(cropped.applied);
        let cropped = cropped.snapshot.unwrap();
        let undone = project.undo_layout(cropped.graph.revision);
        assert!(undone.applied);

        let saved = project.save().unwrap();
        assert!(!saved.dirty);
        let reopened = app.open_project(saved.project_id.clone()).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), saved);
    }

    #[test]
    fn portable_project_document_can_be_imported_without_a_database() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
            1,
        )
        .unwrap();
        let project_id = ProjectId::new();
        let document = ProjectDocument::new(
            project_id,
            "Imported portable project",
            GraphDocument::new(GraphId::new()),
        )
        .unwrap();
        let path = root.0.join("shared.photara-project.json");
        fs::write(&path, document.to_pretty_json().unwrap()).unwrap();
        let opened = app
            .open_project_document(path.to_string_lossy().into_owned())
            .unwrap();
        assert_eq!(
            opened.snapshot().unwrap().project_id,
            project_id.to_string()
        );

        let conflicting = ProjectDocument::new(
            project_id,
            "Conflicting project",
            GraphDocument::new(GraphId::new()),
        )
        .unwrap();
        fs::write(&path, conflicting.to_pretty_json().unwrap()).unwrap();
        assert!(matches!(
            app.open_project_document(path.to_string_lossy().into_owned()),
            Err(BridgeError::State { message }) if message.contains("different content")
        ));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn semantic_structure_and_cell_edits_round_trip_through_exact_history() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
            1,
        )
        .unwrap();
        let project = app.create_project("Authoring".to_owned()).unwrap();
        let initial = project.snapshot().unwrap();
        let added = project.add_layout_node(
            initial.graph.revision,
            BridgeLayoutCanvas::Portrait3x4 {
                long_edge_pixels: 4000,
            },
        );
        let base = added.snapshot.unwrap();
        let node = base
            .nodes
            .iter()
            .find(|node| node.layout.is_some())
            .unwrap();
        let node_id = node.node_id.clone();
        let frame_id = node.layout.as_ref().unwrap().frames[0].frame_id.clone();
        let base_authored_digest = node.layout.as_ref().unwrap().authored_state_digest.clone();

        let arranged = project.edit_layout_structure(
            base.graph.revision,
            node_id.clone(),
            BridgeLayoutStructureEdit::SetFrameArrangement {
                frame_id: frame_id.clone(),
                arrangement: BridgeLayoutArrangementEdit::HorizontalStack,
            },
        );
        assert!(arranged.applied);
        let arranged = arranged.snapshot.unwrap();
        let inserted = project.edit_layout_structure(
            arranged.graph.revision,
            node_id.clone(),
            BridgeLayoutStructureEdit::InsertCell {
                frame_id: frame_id.clone(),
                index: 1,
            },
        );
        assert!(inserted.applied);
        let inserted = inserted.snapshot.unwrap();
        let layout = inserted
            .nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .unwrap()
            .layout
            .as_ref()
            .unwrap();
        assert_eq!(layout.frames[0].cells.len(), 2);
        assert_eq!(layout.frames[0].cells[0].resolved_rect.width, 500_000);
        let cell_id = layout.frames[0].cells[1].cell_id.clone();
        let filled = project.edit_layout_cell(
            inserted.graph.revision,
            node_id.clone(),
            frame_id.clone(),
            cell_id.clone(),
            BridgeLayoutCellEdit::Fill {
                focal_x: 250_000,
                focal_y: 750_000,
            },
        );
        assert!(filled.applied);
        let filled = filled.snapshot.unwrap();
        let rotated = project.edit_layout_cell(
            filled.graph.revision,
            node_id.clone(),
            frame_id,
            cell_id,
            BridgeLayoutCellEdit::SetQuarterTurn {
                quarter_turn: BridgeQuarterTurn::Clockwise90,
            },
        );
        assert!(rotated.applied);
        let authored_digest = rotated
            .snapshot
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .unwrap()
            .layout
            .as_ref()
            .unwrap()
            .authored_state_digest
            .clone();

        let mut revision = rotated.snapshot.unwrap().graph.revision;
        for _ in 0..4 {
            let response = project.undo_layout(revision);
            assert!(response.applied);
            revision = response.snapshot.unwrap().graph.revision;
        }
        let undone = project.snapshot().unwrap();
        assert_eq!(
            undone
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .authored_state_digest,
            base_authored_digest
        );
        for _ in 0..4 {
            let response = project.redo_layout(revision);
            assert!(response.applied);
            revision = response.snapshot.unwrap().graph.revision;
        }
        let redone = project.snapshot().unwrap();
        assert_eq!(
            redone
                .nodes
                .iter()
                .find(|node| node.node_id == node_id)
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .authored_state_digest,
            authored_digest
        );
    }

    #[test]
    fn evaluation_streams_progress_and_honors_pre_start_cancellation() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
            1,
        )
        .unwrap();
        let project = app.create_project("Evaluation".to_owned()).unwrap();

        let completed_observer = Arc::new(TestObserver::default());
        project
            .prepare_evaluation()
            .unwrap()
            .start(completed_observer.clone())
            .unwrap();
        let completed = completed_observer.wait();
        assert_eq!(
            completed.finished.as_ref().unwrap().status,
            BridgeEvaluationStatus::Completed
        );
        assert!(completed.progress.len() >= 3);
        drop(completed);

        let cancelled_observer = Arc::new(TestObserver::default());
        let evaluation = project.prepare_evaluation().unwrap();
        evaluation.cancel();
        evaluation.start(cancelled_observer.clone()).unwrap();
        let cancelled = cancelled_observer.wait();
        assert_eq!(
            cancelled.finished.as_ref().unwrap().status,
            BridgeEvaluationStatus::Cancelled
        );
        assert!(
            cancelled
                .progress
                .iter()
                .any(|event| event.phase == BridgeEvaluationPhase::Cancelled)
        );
    }
}
