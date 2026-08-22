use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    num::NonZeroU32,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use photara_asset_set_node::{AssetSetNodePackage, AssetSetNodeRuntime, asset_set_state_schema};
use photara_core::{
    AssetId, AssetSet, CancellationToken, CommandId, Connection, ConnectionId, DefinitionResolver,
    Diagnostic, DiagnosticSeverity, EvaluationError, EvaluationId, EvaluationPhase,
    EvaluationRequest, GraphCommand, GraphCommandEnvelope, GraphDocument, GraphId,
    NodeDefinitionRef, NodeEvaluationOutput, NodeEvaluationRequest, NodeInstance, NodeInstanceId,
    NodeRuntime, PackageRequirement, PortEndpoint, PortId, ProjectCommand, ProjectCommandEnvelope,
    ProjectDocument, ProjectId, ProjectRelativePath, RequestId, SchemaValue, ValueTypeRegistry,
    apply_graph_command, apply_project_command, asset_set_value_type_descriptor, canonical_digest,
    evaluate_graph,
};
use photara_layout_node::{
    BundledCanvasProfile, CellArrangement, CellContentMode, LayoutCanvas, LayoutCommand,
    LayoutCommandError, LayoutNodePackage, LayoutNodeRuntime, LayoutState, NormalizedRect,
    NormalizedUnit, QuarterTurn, apply_layout_command, layout_plan_value_type_descriptor,
};
use photara_node_sdk::{NodePackage, NodePackageRegistry};
#[cfg(target_os = "macos")]
use photara_proxy::{
    AssetContextProjectProxyService, ImageIoCoreImageGenerator, ImageIoGeneratorConfig,
    ProjectProxyService, ProjectVisualProxyRequest, ProjectVisualProxyService, ProxyServiceConfig,
    standard_hdr_authoring_preview_profile, standard_sdr_thumbnail_profile,
};
use photara_proxy::{ProxyArtifact, ProxyArtifactDisposition};
use photara_store::{
    FileSystemStateStore, LocalProjectAssetAdapter, ProjectRepository,
    prepare_local_tiff_pair_import,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;

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
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub package_id: String,
    pub package_version: String,
    pub definition_id: String,
    pub definition_version: u32,
    pub layout: Option<BridgeLayoutInspectionDto>,
    pub diagnostics: Vec<BridgeDiagnosticDto>,
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
    pub quarter_turn: BridgeQuarterTurn,
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

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct BridgeAssetDto {
    pub asset_id: String,
    pub display_name: String,
    pub representation_count: u64,
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
    value_types: Arc<ValueTypeRegistry>,
    proxy_cache_root: PathBuf,
    proxy_helper_executable: PathBuf,
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
    ) -> Result<Arc<Self>, BridgeError> {
        let store = FileSystemStateStore::open(PathBuf::from(store_root)).map_err(|error| {
            BridgeError::Store {
                message: error.to_string(),
            }
        })?;
        let mut definitions = NodePackageRegistry::default();
        definitions
            .register_package(&AssetSetNodePackage)
            .map_err(|error| BridgeError::State {
                message: error.to_string(),
            })?;
        definitions
            .register_package(&LayoutNodePackage)
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
            value_types: Arc::new(value_types),
            proxy_cache_root: PathBuf::from(proxy_cache_root),
            proxy_helper_executable: PathBuf::from(proxy_helper_executable),
        }))
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
            Arc::clone(&self.definitions),
            Arc::clone(&self.value_types),
            project,
            self.proxy_cache_root.clone(),
            self.proxy_helper_executable.clone(),
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
            Arc::clone(&self.definitions),
            Arc::clone(&self.value_types),
            project,
            self.proxy_cache_root.clone(),
            self.proxy_helper_executable.clone(),
        )
    }
}

#[derive(Clone)]
struct LayoutUndoEntry {
    node_id: NodeInstanceId,
    command: LayoutCommand,
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
    value_types: Arc<ValueTypeRegistry>,
    project_root: PathBuf,
    #[cfg(target_os = "macos")]
    proxy_service: Arc<ProjectProxyService<ImageIoCoreImageGenerator>>,
    state: Mutex<ProjectSessionState>,
}

impl PhotaraProject {
    fn new(
        store: FileSystemStateStore,
        definitions: Arc<NodePackageRegistry>,
        value_types: Arc<ValueTypeRegistry>,
        project: ProjectDocument,
        proxy_cache_root: PathBuf,
        proxy_helper_executable: PathBuf,
    ) -> Result<Arc<Self>, BridgeError> {
        let project_root = store
            .root()
            .join("project-data")
            .join(project.project_id.to_string());
        fs::create_dir_all(&project_root).map_err(|error| BridgeError::Store {
            message: format!("could not create project resource directory: {error}"),
        })?;
        #[cfg(target_os = "macos")]
        let proxy_service = ProjectProxyService::open(
            project.project_id,
            ProxyServiceConfig::conservative(proxy_cache_root, 20 * 1024 * 1024 * 1024),
            ImageIoCoreImageGenerator::new(ImageIoGeneratorConfig {
                helper_executable: proxy_helper_executable,
            }),
        )
        .map_err(|error| BridgeError::State {
            message: error.to_string(),
        })?;
        #[cfg(not(target_os = "macos"))]
        let _ = (proxy_cache_root, proxy_helper_executable);
        Ok(Arc::new(Self {
            store,
            definitions,
            value_types,
            project_root,
            #[cfg(target_os = "macos")]
            proxy_service: Arc::new(proxy_service),
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
        let command_id = CommandId::new();
        let result = self.prepare_crop_command(&node_id, &frame_id, &cell_id, x, y, width, height);
        let (node_id, inverse, authored_state) = match result {
            Ok(value) => value,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.layout.invalid-command",
                    error,
                );
            }
        };
        let response = self.apply_core_command(
            command_id,
            expected_graph_revision,
            GraphCommand::SetAuthoredState {
                node_id,
                authored_state: Some(authored_state),
            },
        );
        if response.applied
            && let Ok(mut state) = self.lock_state()
        {
            state.undo.push(LayoutUndoEntry {
                node_id,
                command: inverse,
            });
            state.redo.clear();
        }
        response
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
        let command = match prepared {
            Ok(command) => command,
            Err(error) => {
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.bridge.invalid-asset-binding",
                    error,
                );
            }
        };
        let response = self.apply_core_command(command_id, expected_graph_revision, command);
        if response.applied
            && let Ok(mut state) = self.lock_state()
        {
            state.undo.clear();
            state.redo.clear();
        }
        response
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
        self.request_asset_proxy(asset_id, false)
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
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
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
        self.request_asset_proxy(asset_id, true)
    }

    /// Creates a one-shot evaluation handle over an immutable graph snapshot.
    ///
    /// # Errors
    ///
    /// Returns a state error when the project session cannot be read.
    pub fn prepare_evaluation(&self) -> Result<Arc<EvaluationHandle>, BridgeError> {
        let state = self.lock_state()?;
        Ok(Arc::new(EvaluationHandle {
            graph: state.project.graph.clone(),
            definitions: Arc::clone(&self.definitions),
            value_types: Arc::clone(&self.value_types),
            cancellation: CancellationToken::default(),
            started: AtomicBool::new(false),
        }))
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
        authoring_preview: bool,
    ) -> Result<Arc<BridgeProxyReference>, BridgeError> {
        let project = self.lock_state()?.project.clone();
        if project.asset_context.asset(asset_id).is_none() {
            return Err(BridgeError::InvalidArgument {
                message: format!("unknown project asset {asset_id}"),
            });
        }
        let materializer = LocalProjectAssetAdapter::new(&self.project_root, &project);
        let services = AssetContextProjectProxyService::new(
            self.proxy_service.as_ref(),
            &project.asset_context,
            &materializer,
        );
        let profile = if authoring_preview {
            standard_hdr_authoring_preview_profile()
        } else {
            standard_sdr_thumbnail_profile()
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
        _authoring_preview: bool,
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

    fn prepare_asset_binding(
        &self,
        layout_node_id: &str,
        frame_id: &str,
        cell_id: &str,
        asset_id: &str,
    ) -> Result<GraphCommand, String> {
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
        Ok(GraphCommand::Batch {
            commands: vec![
                GraphCommand::SetAuthoredState {
                    node_id: source_id,
                    authored_state: Some(SchemaValue {
                        schema: asset_set_state_schema(),
                        value: serde_json::to_value(assets).map_err(|error| error.to_string())?,
                    }),
                },
                GraphCommand::SetAuthoredState {
                    node_id: layout_node_id,
                    authored_state: Some(
                        applied
                            .state
                            .to_schema_value()
                            .map_err(|error| error.to_string())?,
                    ),
                },
            ],
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_crop_command(
        &self,
        node_id: &str,
        frame_id: &str,
        cell_id: &str,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(NodeInstanceId, LayoutCommand, SchemaValue), String> {
        let node_id: NodeInstanceId =
            parse_uuid_id(node_id, "node ID").map_err(|e| e.to_string())?;
        let frame_id = parse_uuid_id(frame_id, "Layout frame ID").map_err(|e| e.to_string())?;
        let cell_id = parse_uuid_id(cell_id, "Layout cell ID").map_err(|e| e.to_string())?;
        let source_rect = NormalizedRect {
            x: NormalizedUnit::new(x).map_err(|error| error.to_string())?,
            y: NormalizedUnit::new(y).map_err(|error| error.to_string())?,
            width: NormalizedUnit::new(width).map_err(|error| error.to_string())?,
            height: NormalizedUnit::new(height).map_err(|error| error.to_string())?,
        };
        let state = self.lock_state().map_err(|error| error.to_string())?;
        let node = state
            .project
            .graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| format!("unknown node {node_id}"))?;
        let layout = LayoutState::from_schema_value(
            node.authored_state
                .as_ref()
                .ok_or_else(|| format!("node {node_id} has no authored state"))?,
        )
        .map_err(|error| error.to_string())?;
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
        cell.content_mode = CellContentMode::Crop { source_rect };
        let command = LayoutCommand::ReplaceCell {
            frame_id,
            cell_id,
            cell,
        };
        let applied =
            apply_layout_command(&layout, command.clone()).map_err(|error| error.to_string())?;
        let authored_state = applied
            .state
            .to_schema_value()
            .map_err(|error| error.to_string())?;
        Ok((node_id, applied.inverse, authored_state))
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
        let prepared = self.prepare_history_authored_state(&entry);
        let (inverse, authored_state) = match prepared {
            Ok(value) => value,
            Err(error) => {
                if let Ok(mut state) = self.lock_state() {
                    let stack = if undo {
                        &mut state.undo
                    } else {
                        &mut state.redo
                    };
                    stack.push(entry);
                }
                return rejected_command(
                    command_id,
                    expected_graph_revision,
                    "photara.layout.history-invalid",
                    error,
                );
            }
        };
        let response = self.apply_core_command(
            command_id,
            expected_graph_revision,
            GraphCommand::SetAuthoredState {
                node_id: entry.node_id,
                authored_state: Some(authored_state),
            },
        );
        if let Ok(mut state) = self.lock_state() {
            if response.applied {
                let destination = if undo {
                    &mut state.redo
                } else {
                    &mut state.undo
                };
                destination.push(LayoutUndoEntry {
                    node_id: entry.node_id,
                    command: inverse,
                });
            } else {
                let source = if undo {
                    &mut state.undo
                } else {
                    &mut state.redo
                };
                source.push(entry);
            }
        }
        response
    }

    fn prepare_history_authored_state(
        &self,
        entry: &LayoutUndoEntry,
    ) -> Result<(LayoutCommand, SchemaValue), String> {
        let state = self.lock_state().map_err(|error| error.to_string())?;
        let node = state
            .project
            .graph
            .nodes
            .iter()
            .find(|node| node.id == entry.node_id)
            .ok_or_else(|| format!("unknown node {}", entry.node_id))?;
        let layout = LayoutState::from_schema_value(
            node.authored_state
                .as_ref()
                .ok_or_else(|| format!("node {} has no authored state", entry.node_id))?,
        )
        .map_err(|error| error.to_string())?;
        let applied = apply_layout_command(&layout, entry.command.clone())
            .map_err(|error| error.to_string())?;
        let authored_state = applied
            .state
            .to_schema_value()
            .map_err(|error| error.to_string())?;
        Ok((applied.inverse, authored_state))
    }
}

#[derive(uniffi::Object)]
pub struct EvaluationHandle {
    graph: GraphDocument,
    definitions: Arc<NodePackageRegistry>,
    value_types: Arc<ValueTypeRegistry>,
    cancellation: CancellationToken,
    started: AtomicBool,
}

#[uniffi::export]
impl EvaluationHandle {
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// Starts one evaluation and streams immutable events to Swift.
    ///
    /// # Errors
    ///
    /// Returns a state error when the handle was already started or its worker
    /// thread cannot be created.
    pub fn start(&self, observer: Arc<dyn EvaluationObserver>) -> Result<(), BridgeError> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(BridgeError::State {
                message: "evaluation handle has already started".to_owned(),
            });
        }
        let graph = self.graph.clone();
        let definitions = Arc::clone(&self.definitions);
        let value_types = Arc::clone(&self.value_types);
        let cancellation = self.cancellation.clone();
        thread::Builder::new()
            .name("photara-evaluation".to_owned())
            .spawn(move || {
                run_evaluation(
                    &graph,
                    definitions.as_ref(),
                    value_types.as_ref(),
                    &cancellation,
                    observer.as_ref(),
                );
            })
            .map_err(|error| BridgeError::State {
                message: format!("could not start evaluation thread: {error}"),
            })?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ApplicationRuntime;

impl NodeRuntime for ApplicationRuntime {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<photara_core::CanonicalDigest> {
        AssetSetNodeRuntime
            .implementation_fingerprint(definition)
            .or_else(|| LayoutNodeRuntime.implementation_fingerprint(definition))
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, photara_core::NodeExecutionError> {
        if AssetSetNodeRuntime
            .implementation_fingerprint(&request.node.definition)
            .is_some()
        {
            AssetSetNodeRuntime.evaluate(request)
        } else {
            LayoutNodeRuntime.evaluate(request)
        }
    }
}

fn run_evaluation(
    graph: &GraphDocument,
    definitions: &NodePackageRegistry,
    value_types: &ValueTypeRegistry,
    cancellation: &CancellationToken,
    observer: &dyn EvaluationObserver,
) {
    let request = EvaluationRequest {
        request_id: RequestId::new(),
        evaluation_id: EvaluationId::new(),
        graph_id: graph.id,
        revision: graph.revision,
        environment: canonical_digest(&"photara-bridge-environment-v1")
            .expect("static environment descriptor is canonical"),
    };
    let result = evaluate_graph(
        graph,
        &request,
        definitions,
        value_types,
        &ApplicationRuntime,
        cancellation,
        |progress| observer.on_progress(progress.into()),
    );
    let finished = match result {
        Ok(outcome) => BridgeEvaluationFinishedDto {
            request_id: request.request_id.to_string(),
            evaluation_id: request.evaluation_id.to_string(),
            status: BridgeEvaluationStatus::Completed,
            evaluation_digest: Some(outcome.evaluation_key.to_string()),
            error: None,
        },
        Err(error) => {
            let status = if matches!(error, EvaluationError::Cancelled { .. }) {
                BridgeEvaluationStatus::Cancelled
            } else {
                BridgeEvaluationStatus::Failed
            };
            let details_json = serde_json::to_string(&error).unwrap_or_else(|_| "{}".to_owned());
            BridgeEvaluationFinishedDto {
                request_id: request.request_id.to_string(),
                evaluation_id: request.evaluation_id.to_string(),
                status,
                evaluation_digest: None,
                error: Some(structured_error(
                    error.code(),
                    error.to_string(),
                    error.diagnostic(),
                    details_json,
                )),
            }
        }
    };
    observer.on_finished(finished);
}

impl From<photara_core::EvaluationProgress> for BridgeEvaluationProgressDto {
    fn from(progress: photara_core::EvaluationProgress) -> Self {
        Self {
            request_id: progress.request_id.to_string(),
            evaluation_id: progress.evaluation_id.to_string(),
            phase: progress.phase.into(),
            completed_nodes: u64::try_from(progress.completed_nodes).unwrap_or(u64::MAX),
            total_nodes: u64::try_from(progress.total_nodes).unwrap_or(u64::MAX),
            node_id: progress.node_id.map(|id| id.to_string()),
        }
    }
}

impl From<EvaluationPhase> for BridgeEvaluationPhase {
    fn from(phase: EvaluationPhase) -> Self {
        match phase {
            EvaluationPhase::Validating => Self::Validating,
            EvaluationPhase::Planning => Self::Planning,
            EvaluationPhase::Evaluating => Self::Evaluating,
            EvaluationPhase::Completed => Self::Completed,
            EvaluationPhase::Cancelled => Self::Cancelled,
        }
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
        .map(|node| node_snapshot(node, definitions, &mut diagnostics))
        .collect();
    Ok(BridgeProjectSnapshotDto {
        project_id: state.project.project_id.to_string(),
        project_revision: state.project.revision.get(),
        title: state.project.metadata.title.clone(),
        graph: BridgeGraphSnapshotDto {
            graph_id: state.project.graph.id.to_string(),
            revision: state.project.graph.revision.get(),
            digest: graph_digest.to_string(),
        },
        assets: state
            .project
            .asset_context
            .assets
            .iter()
            .map(|asset| BridgeAssetDto {
                asset_id: asset.id.to_string(),
                display_name: asset.display_name.clone(),
                representation_count: u64::try_from(asset.representations.len())
                    .unwrap_or(u64::MAX),
            })
            .collect(),
        nodes,
        diagnostics,
        dirty: state.dirty,
    })
}

fn node_snapshot(
    node: &NodeInstance,
    definitions: &NodePackageRegistry,
    project_diagnostics: &mut Vec<BridgeDiagnosticDto>,
) -> BridgeNodeDto {
    let display_name = definitions.resolve(&node.definition).map_or_else(
        || node.definition.definition_id.to_string(),
        |value| value.display_name.clone(),
    );
    let (layout, mut diagnostics) = layout_inspection(node);
    project_diagnostics.extend(diagnostics.iter().cloned());
    BridgeNodeDto {
        node_id: node.id.to_string(),
        display_name,
        package_id: node.definition.package_id.to_string(),
        package_version: node.definition.package_version.to_string(),
        definition_id: node.definition.definition_id.to_string(),
        definition_version: node.definition.definition_version.get(),
        layout,
        diagnostics: std::mem::take(&mut diagnostics),
    }
}

fn layout_inspection(
    node: &NodeInstance,
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
        Ok(layout) => match BridgeLayoutInspectionDto::try_from(&layout) {
            Ok(inspection) => (Some(inspection), Vec::new()),
            Err(error) => (
                None,
                vec![simple_diagnostic(
                    "photara.layout.inspection-failed",
                    error.to_string(),
                    Some(node.id),
                )],
            ),
        },
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

impl TryFrom<&LayoutState> for BridgeLayoutInspectionDto {
    type Error = BridgeError;

    #[allow(clippy::too_many_lines)]
    fn try_from(layout: &LayoutState) -> Result<Self, Self::Error> {
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
        let frames = layout
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| BridgeLayoutFrameInspectionDto {
                frame_id: frame.id.to_string(),
                index: u64::try_from(index).unwrap_or(u64::MAX),
                arrangement: match frame.arrangement {
                    CellArrangement::One => BridgeLayoutArrangement::One,
                    CellArrangement::HorizontalStack => BridgeLayoutArrangement::HorizontalStack,
                    CellArrangement::VerticalStack => BridgeLayoutArrangement::VerticalStack,
                    CellArrangement::UniformGrid { .. } => BridgeLayoutArrangement::UniformGrid,
                    CellArrangement::Custom => BridgeLayoutArrangement::Custom,
                },
                cells: frame
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(index, cell)| {
                        let (content_mode, focal_x, focal_y, crop_rect) = match cell.content_mode {
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
                            quarter_turn: cell.quarter_turn.into(),
                        }
                    })
                    .collect(),
            })
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

fn structured_error(
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

    #[test]
    fn production_facade_saves_reopens_commands_and_undoes_layout() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
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
    fn evaluation_streams_progress_and_honors_pre_start_cancellation() {
        let root = TestRoot::new();
        let app = PhotaraApplication::open(
            root.0.join("store").to_string_lossy().into_owned(),
            root.0.join("cache").to_string_lossy().into_owned(),
            root.0.join("proxy-helper").to_string_lossy().into_owned(),
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
