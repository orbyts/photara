use std::{
    collections::BTreeMap,
    fs,
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use photara_core::{
    AssetId, AssetRepresentationId, ColorSpaceId, MaterializedRepresentation, ProjectId,
    ProxyAlphaPolicy, ProxyChannelDepth, ProxyColorPolicy, ProxyDynamicRangeDescription,
    ProxyDynamicRangePolicy, ProxyEncodingId, ProxyEncodingRef, ProxyEncodingVersion,
    ProxyGeneratorId, ProxyGeneratorRef, ProxyGeneratorVersion, ProxyOrientationPolicy,
    ProxyProfile, ProxyProfileId, ProxyProfileVersion, ProxyPurpose, ProxyRenderingIntent,
    ProxyRequest, ProxyResamplingFilter, ProxySizing, ProxyStoredOrientation,
    RepresentationAvailability, RepresentationFingerprint, RepresentationMaterializationError,
    RepresentationMaterializationRequest, RepresentationMaterializer, RequestId, ToneMapOperatorId,
    ToneMapPolicy, ToneMapVersion,
};

use super::*;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("photara-proxy-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct TestMaterializer {
    path: PathBuf,
    available: AtomicBool,
    calls: AtomicUsize,
}

impl TestMaterializer {
    fn new(path: PathBuf, available: bool) -> Self {
        Self {
            path,
            available: AtomicBool::new(available),
            calls: AtomicUsize::new(0),
        }
    }
}

impl RepresentationMaterializer for TestMaterializer {
    fn availability(
        &self,
        _request: &RepresentationMaterializationRequest,
    ) -> Result<RepresentationAvailability, RepresentationMaterializationError> {
        Ok(if self.available.load(Ordering::SeqCst) {
            RepresentationAvailability::Available
        } else {
            RepresentationAvailability::Missing
        })
    }

    fn materialize(
        &self,
        request: &RepresentationMaterializationRequest,
    ) -> Result<MaterializedRepresentation, RepresentationMaterializationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.available.load(Ordering::SeqCst) {
            return Err(RepresentationMaterializationError::Unavailable(
                RepresentationAvailability::Missing,
            ));
        }
        Ok(MaterializedRepresentation {
            asset_id: request.asset_id,
            representation_id: request.representation_id,
            fingerprint: request.expected_fingerprint,
            local_path: self.path.clone(),
            byte_length: fs::metadata(&self.path).unwrap().len(),
        })
    }
}

struct TestGenerator {
    calls: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
    payload_bytes: usize,
    delay: Duration,
}

impl TestGenerator {
    fn new(payload_bytes: usize, delay: Duration) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            payload_bytes,
            delay,
        }
    }
}

impl ProxyGenerator for TestGenerator {
    fn exact_ref(&self) -> ProxyGeneratorRef {
        ProxyGeneratorRef {
            id: ProxyGeneratorId::parse("photara.test.proxy-generator").unwrap(),
            version: ProxyGeneratorVersion::first(),
        }
    }

    fn generate(
        &self,
        request: &ProxyRequest,
        _source: &MaterializedRepresentation,
        destination: &Path,
    ) -> Result<GeneratedProxyMetadata, ProxyGenerationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        thread::sleep(self.delay);
        let seed = request.source_fingerprint.as_bytes()[0];
        fs::write(destination, vec![seed; self.payload_bytes]).unwrap();
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(GeneratedProxyMetadata {
            pixel_width: NonZeroU32::new(512).unwrap(),
            pixel_height: NonZeroU32::new(341).unwrap(),
            channel_depth: request.profile.channel_depth,
            alpha: request.profile.alpha,
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

fn profile(id: &str) -> ProxyProfile {
    ProxyProfile {
        id: ProxyProfileId::parse(id).unwrap(),
        version: ProxyProfileVersion::first(),
        purpose: ProxyPurpose::Thumbnail,
        sizing: ProxySizing::LongEdge {
            pixels: NonZeroU32::new(512).unwrap(),
        },
        allow_upscale: false,
        resampling: ProxyResamplingFilter::Lanczos3,
        orientation: ProxyOrientationPolicy::NormalizePixels,
        color: ProxyColorPolicy::Convert {
            destination: ColorSpaceId::parse("photara.color.srgb").unwrap(),
            rendering_intent: ProxyRenderingIntent::RelativeColorimetric,
            black_point_compensation: true,
        },
        dynamic_range: ProxyDynamicRangePolicy::SdrCompatible {
            tone_map: ToneMapPolicy {
                operator: ToneMapOperatorId::parse("photara.tonemap.reinhard-global").unwrap(),
                version: ToneMapVersion::first(),
                parameters: BTreeMap::new(),
            },
        },
        channel_depth: ProxyChannelDepth::U8,
        alpha: ProxyAlphaPolicy::Opaque,
        encoding: ProxyEncodingRef {
            id: ProxyEncodingId::parse("photara.proxy-encoding.png").unwrap(),
            version: ProxyEncodingVersion::first(),
        },
    }
}

fn request(project_id: ProjectId, source_byte: u8, profile_id: &str) -> ProxyRequest {
    ProxyRequest {
        request_id: RequestId::new(),
        project_id,
        asset_id: AssetId::new(),
        representation_id: AssetRepresentationId::new(),
        source_fingerprint: RepresentationFingerprint::sha256([source_byte; 32]),
        profile: profile(profile_id),
    }
}

fn service(
    root: &TestRoot,
    project_id: ProjectId,
    generator: TestGenerator,
    limit: usize,
    quota_bytes: u64,
) -> ProjectProxyService<TestGenerator> {
    ProjectProxyService::open(
        project_id,
        ProxyServiceConfig {
            cache_root: root.0.join("cache"),
            quota_bytes,
            max_concurrent_generations: NonZeroUsize::new(limit).unwrap(),
        },
        generator,
    )
    .unwrap()
}

#[test]
fn identical_requests_deduplicate_before_the_generation_limiter() {
    let root = TestRoot::new();
    let source = root.0.join("source.tiff");
    fs::write(&source, b"source").unwrap();
    let project_id = ProjectId::new();
    let service = Arc::new(service(
        &root,
        project_id,
        TestGenerator::new(128, Duration::from_millis(100)),
        2,
        1_000_000,
    ));
    let materializer = Arc::new(TestMaterializer::new(source, true));
    let proxy_request = request(project_id, 7, "photara.test.thumbnail");
    let mut threads = Vec::new();
    for _ in 0..8 {
        let service = Arc::clone(&service);
        let materializer = Arc::clone(&materializer);
        let proxy_request = proxy_request.clone();
        threads.push(thread::spawn(move || {
            service
                .request(&proxy_request, materializer.as_ref())
                .unwrap()
        }));
    }
    let artifacts: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();

    assert_eq!(service.generator.calls.load(Ordering::SeqCst), 1);
    assert_eq!(materializer.calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.metrics().peak_active_generations, 1);
    assert_eq!(service.metrics().deduplicated_waiters, 7);
    assert_eq!(
        artifacts
            .iter()
            .filter(|artifact| artifact.disposition == ProxyArtifactDisposition::Generated)
            .count(),
        1
    );
}

#[test]
fn distinct_requests_respect_the_explicit_generation_bound() {
    let root = TestRoot::new();
    let source = root.0.join("source.tiff");
    fs::write(&source, b"source").unwrap();
    let project_id = ProjectId::new();
    let service = Arc::new(service(
        &root,
        project_id,
        TestGenerator::new(64, Duration::from_millis(75)),
        2,
        1_000_000,
    ));
    let materializer = Arc::new(TestMaterializer::new(source, true));
    let mut threads = Vec::new();
    for source_byte in 1..=6 {
        let service = Arc::clone(&service);
        let materializer = Arc::clone(&materializer);
        let proxy_request = request(project_id, source_byte, "photara.test.thumbnail");
        threads.push(thread::spawn(move || {
            service
                .request(&proxy_request, materializer.as_ref())
                .unwrap()
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }

    assert_eq!(service.generator.calls.load(Ordering::SeqCst), 6);
    assert_eq!(service.generator.peak.load(Ordering::SeqCst), 2);
    assert_eq!(service.metrics().peak_active_generations, 2);
}

#[test]
fn cache_hits_are_verified_and_corruption_regenerates() {
    let root = TestRoot::new();
    let source = root.0.join("source.tiff");
    fs::write(&source, b"source").unwrap();
    let project_id = ProjectId::new();
    let service = service(
        &root,
        project_id,
        TestGenerator::new(128, Duration::ZERO),
        1,
        1_000_000,
    );
    let materializer = TestMaterializer::new(source, true);
    let proxy_request = request(project_id, 3, "photara.test.thumbnail");

    let generated = service.request(&proxy_request, &materializer).unwrap();
    let hit = service.request(&proxy_request, &materializer).unwrap();
    assert_eq!(hit.disposition, ProxyArtifactDisposition::CacheHit);
    fs::write(&generated.local_path, b"corrupt").unwrap();
    let recovered = service.request(&proxy_request, &materializer).unwrap();

    assert_eq!(recovered.disposition, ProxyArtifactDisposition::Generated);
    assert_eq!(service.generator.calls.load(Ordering::SeqCst), 2);
    assert_eq!(service.metrics().corruption_recoveries, 1);
}

#[test]
fn unavailable_sources_are_not_cached_and_can_be_retried_after_remount() {
    let root = TestRoot::new();
    let source = root.0.join("source.tiff");
    fs::write(&source, b"source").unwrap();
    let project_id = ProjectId::new();
    let service = service(
        &root,
        project_id,
        TestGenerator::new(64, Duration::ZERO),
        1,
        1_000_000,
    );
    let materializer = TestMaterializer::new(source, false);
    let proxy_request = request(project_id, 4, "photara.test.thumbnail");

    assert!(matches!(
        service.request(&proxy_request, &materializer),
        Err(ProxyServiceError::Materialization(
            RepresentationMaterializationError::Unavailable(RepresentationAvailability::Missing)
        ))
    ));
    materializer.available.store(true, Ordering::SeqCst);
    assert_eq!(
        service
            .request(&proxy_request, &materializer)
            .unwrap()
            .disposition,
        ProxyArtifactDisposition::Generated
    );
    assert_eq!(service.generator.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn quota_evicts_old_derived_entries_and_cache_clear_cannot_touch_project_state() {
    let root = TestRoot::new();
    let source = root.0.join("source.tiff");
    let project = root.0.join("authoritative.photara-project.json");
    fs::write(&source, b"source").unwrap();
    fs::write(&project, b"authoritative").unwrap();
    let project_id = ProjectId::new();
    let service = service(
        &root,
        project_id,
        TestGenerator::new(4_000, Duration::ZERO),
        1,
        6_500,
    );
    let materializer = TestMaterializer::new(source, true);
    service
        .request(
            &request(project_id, 1, "photara.test.thumbnail-a"),
            &materializer,
        )
        .unwrap();
    let held = service
        .request(
            &request(project_id, 2, "photara.test.thumbnail-b"),
            &materializer,
        )
        .unwrap();
    let objects = service.project_root().join("objects");
    assert_eq!(fs::read_dir(objects).unwrap().count(), 1);

    assert_eq!(service.clear_cache(), Err(ProxyServiceError::CacheBusy));
    drop(held);
    service.clear_cache().unwrap();
    assert_eq!(fs::read(&project).unwrap(), b"authoritative");
}

#[test]
fn conservative_policy_is_one_job_and_not_cpu_derived() {
    let config = ProxyServiceConfig::conservative("cache", 1024);
    assert_eq!(config.max_concurrent_generations, NonZeroUsize::MIN);
}

#[test]
fn layout_interaction_profile_defaults_to_1k_and_is_hdr_preserving() {
    let profile = standard_layout_interaction_preview_profile();
    assert_eq!(profile.purpose, ProxyPurpose::AuthoringPreview);
    assert_eq!(
        profile.sizing,
        ProxySizing::LongEdge {
            pixels: NonZeroU32::new(1_024).unwrap()
        }
    );
    assert_eq!(profile.channel_depth, ProxyChannelDepth::F16);
    assert_eq!(
        profile.dynamic_range,
        ProxyDynamicRangePolicy::PreserveSource
    );

    let larger = crate::layout_interaction_preview_profile(NonZeroU32::new(2_048).unwrap());
    assert_ne!(
        profile.exact_ref().unwrap().digest,
        larger.exact_ref().unwrap().digest
    );
}

#[test]
fn gallery_profile_is_a_tiny_hdr_preserving_tier() {
    let profile = crate::standard_gallery_preview_profile();
    assert_eq!(profile.purpose, ProxyPurpose::Thumbnail);
    assert_eq!(
        profile.sizing,
        ProxySizing::LongEdge {
            pixels: NonZeroU32::new(384).unwrap()
        }
    );
    assert_eq!(profile.channel_depth, ProxyChannelDepth::F16);
    assert_eq!(
        profile.dynamic_range,
        ProxyDynamicRangePolicy::PreserveSource
    );
}
