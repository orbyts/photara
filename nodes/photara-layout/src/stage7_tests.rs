use std::{
    collections::BTreeMap,
    fs,
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use photara_core::{
    AssetSet, CancellationToken, ColorSpaceId, EvaluationId, GraphDocument, GraphId,
    MaterializedRepresentation, NodeDefinitionRef, NodeEvaluationRequest, NodeInstance,
    NodeInstanceId, NodeRuntime, ProjectDocument, ProjectId, ProxyAlphaPolicy, ProxyChannelDepth,
    ProxyDynamicRangeDescription, ProxyGeneratorId, ProxyGeneratorRef, ProxyGeneratorVersion,
    ProxyStoredOrientation, RequestId, SchemaValue, canonical_digest,
};
use photara_node_sdk::{NodePackage, NodePackageManifest};
use photara_proxy::{
    AssetContextProjectProxyService, GeneratedProxyMetadata, ProjectProxyService,
    ProjectVisualProxyRequest, ProjectVisualProxyService, ProxyArtifact, ProxyArtifactDisposition,
    ProxyGenerationError, ProxyGenerator, ProxyServiceConfig, ProxyServiceError,
    standard_sdr_thumbnail_profile,
};
use photara_store::{
    FileSystemStateStore, LocalProjectAssetAdapter, ProjectRepository, import_local_tiff_pair,
};
use serde_json::json;

use crate::{
    CellContentMode, LayoutCanvas, LayoutNodePackage, LayoutNodeRuntime, LayoutPlan, LayoutState,
    NormalizedRect, NormalizedUnit, QuarterTurn, resolve_layout,
};

fn request_plan_proxies(
    plan: &LayoutPlan,
    project_id: ProjectId,
    services: &dyn ProjectVisualProxyService,
) -> Result<BTreeMap<photara_core::AssetId, ProxyArtifact>, ProxyServiceError> {
    let profile = standard_sdr_thumbnail_profile();
    plan.frames
        .iter()
        .flat_map(|frame| frame.cells.iter().filter_map(|cell| cell.asset_id))
        .map(|asset_id| {
            services
                .request_visual_proxy(&ProjectVisualProxyRequest {
                    request_id: RequestId::new(),
                    project_id,
                    asset_id,
                    profile: profile.clone(),
                })
                .map(|artifact| (asset_id, artifact))
        })
        .collect()
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("photara-layout-stage-7-{}", ProjectId::new()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FixtureGenerator {
    calls: Arc<AtomicUsize>,
}

impl ProxyGenerator for FixtureGenerator {
    fn exact_ref(&self) -> ProxyGeneratorRef {
        ProxyGeneratorRef {
            id: ProxyGeneratorId::parse("photara.test.layout-proxy-generator").unwrap(),
            version: ProxyGeneratorVersion::first(),
        }
    }

    fn generate(
        &self,
        request: &photara_core::ProxyRequest,
        _source: &MaterializedRepresentation,
        destination: &Path,
    ) -> Result<GeneratedProxyMetadata, ProxyGenerationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        fs::write(destination, request.source_fingerprint.as_bytes())
            .map_err(|error| ProxyGenerationError::new(error.to_string()))?;
        Ok(GeneratedProxyMetadata {
            pixel_width: NonZeroU32::new(512).unwrap(),
            pixel_height: NonZeroU32::new(341).unwrap(),
            channel_depth: ProxyChannelDepth::U8,
            alpha: ProxyAlphaPolicy::Opaque,
            encoding: request.profile.encoding.clone(),
            color_space: ColorSpaceId::parse("photara.color.srgb").unwrap(),
            embedded_icc_fingerprint: None,
            dynamic_range: ProxyDynamicRangeDescription::Sdr {
                reference_white_nits: None,
            },
            orientation: ProxyStoredOrientation::PixelsNormalized,
        })
    }
}

fn layout_state(canvas: LayoutCanvas, asset_id: photara_core::AssetId, crop_x: u32) -> LayoutState {
    let mut state = LayoutState::new(canvas);
    let cell = &mut state.frames[0].cells[0];
    cell.asset_id = Some(asset_id);
    cell.content_mode = CellContentMode::Crop {
        source_rect: NormalizedRect {
            x: NormalizedUnit::new(crop_x).unwrap(),
            y: NormalizedUnit::new(50_000).unwrap(),
            width: NormalizedUnit::new(800_000).unwrap(),
            height: NormalizedUnit::new(900_000).unwrap(),
        },
    };
    cell.quarter_turn = QuarterTurn::Clockwise90;
    state
}

fn layout_instance(manifest: &NodePackageManifest, state: &LayoutState) -> NodeInstance {
    let definition = &manifest.definitions[0];
    NodeInstance {
        id: NodeInstanceId::new(),
        definition: NodeDefinitionRef {
            package_id: manifest.package_id.clone(),
            package_version: manifest.package_version.clone(),
            definition_id: definition.id.clone(),
            definition_version: definition.version,
        },
        configuration: SchemaValue {
            schema: definition.config_schema.clone(),
            value: json!({}),
        },
        authored_state: Some(state.to_schema_value().unwrap()),
        extensions: BTreeMap::new(),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn independent_layouts_reuse_project_proxies_and_survive_complete_cache_deletion() {
    let root = TestRoot::new();
    let project_root = root.0.join("project");
    fs::create_dir_all(project_root.join("representations")).unwrap();
    fs::write(
        project_root.join("representations/hdr.tiff"),
        b"hdr-fixture",
    )
    .unwrap();
    fs::write(
        project_root.join("representations/sdr.tiff"),
        b"sdr-fixture",
    )
    .unwrap();

    let project_id = ProjectId::new();
    let empty =
        ProjectDocument::new(project_id, "Stage 7", GraphDocument::new(GraphId::new())).unwrap();
    let mut asset_shell = empty;
    let asset_id = import_local_tiff_pair(
        &mut asset_shell,
        &project_root,
        "Shared photograph",
        "representations/hdr.tiff".parse().unwrap(),
        "representations/sdr.tiff".parse().unwrap(),
    )
    .unwrap();

    let portrait = layout_state(
        LayoutCanvas::portrait_3x4(NonZeroU32::new(4000).unwrap()),
        asset_id,
        50_000,
    );
    let vertical = layout_state(
        LayoutCanvas::vertical_9x16(NonZeroU32::new(3840).unwrap()),
        asset_id,
        150_000,
    );
    assert_ne!(portrait.digest().unwrap(), vertical.digest().unwrap());

    let manifest = LayoutNodePackage.manifest();
    let portrait_node = layout_instance(&manifest, &portrait);
    let vertical_node = layout_instance(&manifest, &vertical);
    let mut graph = GraphDocument::new(GraphId::new());
    graph.nodes = vec![portrait_node.clone(), vertical_node.clone()];
    let mut project = ProjectDocument::new(project_id, "Stage 7", graph).unwrap();
    project.resources = asset_shell.resources;
    project.asset_context = asset_shell.asset_context;
    project.validate().unwrap();

    let asset_set = AssetSet {
        assets: vec![asset_id],
    };
    let portrait_plan = resolve_layout(&portrait, &asset_set).unwrap();
    let vertical_plan = resolve_layout(&vertical, &asset_set).unwrap();
    assert_eq!(portrait_plan.canvas.width.get(), 3000);
    assert_eq!(vertical_plan.canvas.width.get(), 2160);

    let mut store = FileSystemStateStore::open(root.0.join("state")).unwrap();
    store.create_project(project.clone()).unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let proxy_service = ProjectProxyService::open(
        project_id,
        ProxyServiceConfig {
            cache_root: root.0.join("cache"),
            quota_bytes: 1_000_000,
            max_concurrent_generations: NonZeroUsize::MIN,
        },
        FixtureGenerator {
            calls: Arc::clone(&calls),
        },
    )
    .unwrap();
    let materializer = LocalProjectAssetAdapter::new(&project_root, &project);
    let visual_service =
        AssetContextProjectProxyService::new(&proxy_service, &project.asset_context, &materializer);
    let portrait_proxies =
        request_plan_proxies(&portrait_plan, project_id, &visual_service).unwrap();
    let vertical_proxies =
        request_plan_proxies(&vertical_plan, project_id, &visual_service).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(portrait_proxies.len(), 1);
    assert_eq!(
        vertical_proxies[&asset_id].disposition,
        ProxyArtifactDisposition::CacheHit
    );
    let proxy_path = portrait_proxies[&asset_id].local_path.clone();
    assert!(proxy_path.is_file());

    drop(portrait_proxies);
    drop(vertical_proxies);
    proxy_service.clear_cache().unwrap();
    assert!(!proxy_path.exists());

    drop(store);
    let reopened_store = FileSystemStateStore::open(root.0.join("state")).unwrap();
    let reopened = reopened_store.load_project(project_id).unwrap().unwrap();
    assert_eq!(reopened, project);
    assert_eq!(
        LayoutState::from_schema_value(reopened.graph.nodes[0].authored_state.as_ref().unwrap())
            .unwrap(),
        portrait
    );
    assert_eq!(
        LayoutState::from_schema_value(reopened.graph.nodes[1].authored_state.as_ref().unwrap())
            .unwrap(),
        vertical
    );
}

#[test]
fn semantic_runtime_uses_explicit_asset_set_and_never_requests_proxies() {
    let asset_id = photara_core::AssetId::new();
    let state = layout_state(
        LayoutCanvas::portrait_3x4(NonZeroU32::new(4000).unwrap()),
        asset_id,
        100_000,
    );
    let manifest = LayoutNodePackage.manifest();
    let node = layout_instance(&manifest, &state);
    let assets = AssetSet {
        assets: vec![asset_id],
    };
    let request = NodeEvaluationRequest {
        request_id: RequestId::new(),
        evaluation_id: EvaluationId::new(),
        node,
        inputs: BTreeMap::from([(
            "assets".parse().unwrap(),
            vec![assets.to_typed_value().unwrap()],
        )]),
        evaluation_key: canonical_digest(&"layout-test-key").unwrap(),
        cancellation: CancellationToken::default(),
    };

    let output = LayoutNodeRuntime.evaluate(request).unwrap();
    let value = &output.outputs[&"layout".parse().unwrap()][0];
    let plan: LayoutPlan = serde_json::from_value(value.value.clone()).unwrap();
    assert_eq!(plan.canvas.width.get(), 3000);
    let encoded = serde_json::to_string(&plan).unwrap();
    assert!(!encoded.contains("proxy"));
    assert!(!encoded.contains("cache"));
}
