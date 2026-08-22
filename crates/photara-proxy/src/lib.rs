//! Project-scoped, disposable proxy generation and cache infrastructure.
//!
//! This crate is a runtime service. It does not persist proxies in the portable
//! project document and it does not expose backend image objects to Core.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{ImageIoCoreImageGenerator, ImageIoGeneratorConfig};

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use photara_core::{
    AssetId, ColorSpaceId, HDR_CAPABILITY_ID, IMAGE_CAPABILITY_ID, MaterializedRepresentation,
    ProjectAssetContext, ProjectId, ProxyAlphaPolicy, ProxyCacheKey, ProxyChannelDepth,
    ProxyColorPolicy, ProxyDescriptor, ProxyDynamicRangeDescription, ProxyDynamicRangePolicy,
    ProxyEncodingId, ProxyEncodingRef, ProxyEncodingVersion, ProxyGeneratorRef,
    ProxyOrientationPolicy, ProxyProfile, ProxyProfileId, ProxyProfileVersion, ProxyPurpose,
    ProxyRenderingIntent, ProxyRequest, ProxyResamplingFilter, ProxySizing, ProxyStoredOrientation,
    RepresentationFingerprint, RepresentationMaterializationError,
    RepresentationMaterializationRequest, RepresentationMaterializer, RequestId, SDR_CAPABILITY_ID,
    ToneMapOperatorId, ToneMapPolicy, ToneMapVersion,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Media-general request made by a node or UI consumer to project services.
/// Representation selection, materialization, and cache location remain hidden.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVisualProxyRequest {
    pub request_id: RequestId,
    pub project_id: ProjectId,
    pub asset_id: AssetId,
    pub profile: ProxyProfile,
}

/// Narrow project-service interface consumed by Layout and future visual nodes.
pub trait ProjectVisualProxyService {
    /// Resolves a compatible representation and returns a leased derived proxy.
    ///
    /// # Errors
    ///
    /// Returns [`ProxyServiceError`] when the asset or compatible representation
    /// is unavailable, materialization fails, or proxy generation fails.
    fn request_visual_proxy(
        &self,
        request: &ProjectVisualProxyRequest,
    ) -> Result<ProxyArtifact, ProxyServiceError>;
}

/// Runtime binding of asset context and materialization to the shared cache.
/// Neither binding nor generated artifacts enter portable project state.
pub struct AssetContextProjectProxyService<'a, G> {
    proxy_service: &'a ProjectProxyService<G>,
    asset_context: &'a ProjectAssetContext,
    materializer: &'a dyn RepresentationMaterializer,
}

impl<'a, G> AssetContextProjectProxyService<'a, G> {
    #[must_use]
    pub const fn new(
        proxy_service: &'a ProjectProxyService<G>,
        asset_context: &'a ProjectAssetContext,
        materializer: &'a dyn RepresentationMaterializer,
    ) -> Self {
        Self {
            proxy_service,
            asset_context,
            materializer,
        }
    }
}

impl<G: ProxyGenerator> ProjectVisualProxyService for AssetContextProjectProxyService<'_, G> {
    fn request_visual_proxy(
        &self,
        request: &ProjectVisualProxyRequest,
    ) -> Result<ProxyArtifact, ProxyServiceError> {
        let asset = self
            .asset_context
            .asset(request.asset_id)
            .ok_or(ProxyServiceError::UnknownAsset(request.asset_id))?;
        let desired_capability = match request.profile.dynamic_range {
            ProxyDynamicRangePolicy::SdrCompatible { .. } => SDR_CAPABILITY_ID,
            ProxyDynamicRangePolicy::PreserveSource
            | ProxyDynamicRangePolicy::HdrCapable { .. } => HDR_CAPABILITY_ID,
        };
        let representation = asset
            .representations
            .iter()
            .find(|representation| {
                representation
                    .capabilities
                    .iter()
                    .any(|capability| capability.as_str() == desired_capability)
            })
            .or_else(|| {
                asset.representations.iter().find(|representation| {
                    representation
                        .capabilities
                        .iter()
                        .any(|capability| capability.as_str() == IMAGE_CAPABILITY_ID)
                })
            })
            .ok_or(ProxyServiceError::NoCompatibleRepresentation {
                asset_id: request.asset_id,
                required_capability: desired_capability,
            })?;
        self.proxy_service.request(
            &ProxyRequest {
                request_id: request.request_id,
                project_id: request.project_id,
                asset_id: request.asset_id,
                representation_id: representation.id,
                source_fingerprint: representation.fingerprint,
                profile: request.profile.clone(),
            },
            self.materializer,
        )
    }
}

/// Initial reusable SDR thumbnail profile measured in Stage 6A.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in IDs or sizes are invalid.
#[must_use]
pub fn standard_sdr_thumbnail_profile() -> ProxyProfile {
    ProxyProfile {
        id: ProxyProfileId::parse("photara.proxy.thumbnail-sdr")
            .expect("built-in profile ID is valid"),
        version: ProxyProfileVersion::first(),
        purpose: ProxyPurpose::Thumbnail,
        sizing: ProxySizing::LongEdge {
            pixels: NonZeroU32::new(512).expect("built-in size is nonzero"),
        },
        allow_upscale: false,
        resampling: ProxyResamplingFilter::Lanczos3,
        orientation: ProxyOrientationPolicy::NormalizePixels,
        color: ProxyColorPolicy::Convert {
            destination: ColorSpaceId::parse("photara.color.srgb")
                .expect("built-in color-space ID is valid"),
            rendering_intent: ProxyRenderingIntent::RelativeColorimetric,
            black_point_compensation: true,
        },
        dynamic_range: ProxyDynamicRangePolicy::SdrCompatible {
            tone_map: ToneMapPolicy {
                operator: ToneMapOperatorId::parse("photara.tonemap.reinhard-global")
                    .expect("built-in tone-map ID is valid"),
                version: ToneMapVersion::first(),
                parameters: BTreeMap::new(),
            },
        },
        channel_depth: ProxyChannelDepth::U8,
        alpha: ProxyAlphaPolicy::Opaque,
        encoding: ProxyEncodingRef {
            id: ProxyEncodingId::parse("photara.proxy-encoding.png")
                .expect("built-in encoding ID is valid"),
            version: ProxyEncodingVersion::first(),
        },
    }
}

/// Small HDR-preserving Gallery preview for sources whose native thumbnail
/// provider is not responsive or color-correct.
///
/// The 384 px default sits inside the 256–512 px tiny-preview tier. It uses the
/// same measured float `ImageIO` path as authoring previews while keeping output
/// and cache cost substantially smaller.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in IDs or size are invalid.
#[must_use]
pub fn standard_gallery_preview_profile() -> ProxyProfile {
    let mut profile = standard_hdr_authoring_preview_profile();
    profile.id =
        ProxyProfileId::parse("photara.proxy.gallery-hdr").expect("built-in profile ID is valid");
    profile.purpose = ProxyPurpose::Thumbnail;
    profile.sizing = ProxySizing::LongEdge {
        pixels: NonZeroU32::new(384).expect("built-in size is nonzero"),
    };
    profile
}

/// HDR-preserving authoring image for Layout composition and crop UI.
///
/// This is intentionally not a high-quality editing or export representation.
/// Final authored geometry remains normalized Core state applied to originals
/// by downstream provider/render nodes. Native clients constrain its displayed
/// headroom for mixed thumbnail/workspace presentation and let the OS tone-map
/// it on SDR displays.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in profile ID or size is invalid.
#[must_use]
pub fn standard_layout_interaction_preview_profile() -> ProxyProfile {
    layout_interaction_preview_profile(NonZeroU32::new(1_024).expect("built-in size is nonzero"))
}

/// Builds the Layout authoring profile for a device-selected preview size.
///
/// The size is runtime presentation policy and remains outside authored Layout
/// state. It is part of the complete proxy profile, so different choices have
/// distinct cache identities and may safely coexist.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in profile ID is invalid.
#[must_use]
pub fn layout_interaction_preview_profile(long_edge: NonZeroU32) -> ProxyProfile {
    let mut profile = standard_hdr_authoring_preview_profile();
    profile.id = ProxyProfileId::parse("photara.proxy.layout-interaction-hdr")
        .expect("built-in profile ID is valid");
    profile.sizing = ProxySizing::LongEdge { pixels: long_edge };
    profile
}

/// Initial reusable HDR authoring-preview profile measured in Stage 6A.
///
/// # Panics
///
/// Panics only if Photara's compile-time built-in IDs or sizes are invalid.
#[must_use]
pub fn standard_hdr_authoring_preview_profile() -> ProxyProfile {
    ProxyProfile {
        id: ProxyProfileId::parse("photara.proxy.authoring-hdr")
            .expect("built-in profile ID is valid"),
        version: ProxyProfileVersion::first(),
        purpose: ProxyPurpose::AuthoringPreview,
        sizing: ProxySizing::LongEdge {
            pixels: NonZeroU32::new(2048).expect("built-in size is nonzero"),
        },
        allow_upscale: false,
        resampling: ProxyResamplingFilter::Lanczos3,
        orientation: ProxyOrientationPolicy::NormalizePixels,
        color: ProxyColorPolicy::PreserveEmbedded,
        dynamic_range: ProxyDynamicRangePolicy::PreserveSource,
        channel_depth: ProxyChannelDepth::F16,
        alpha: ProxyAlphaPolicy::Opaque,
        encoding: ProxyEncodingRef {
            id: ProxyEncodingId::parse("photara.proxy-encoding.tiff")
                .expect("built-in encoding ID is valid"),
            version: ProxyEncodingVersion::first(),
        },
    }
}

/// Runtime policy for one project's disposable proxy cache.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyServiceConfig {
    pub cache_root: PathBuf,
    pub quota_bytes: u64,
    pub max_concurrent_generations: NonZeroUsize,
}

impl ProxyServiceConfig {
    /// Conservative initial policy measured for the first Apple backend.
    ///
    /// The HDR authoring-preview path reached about 954 MiB peak RSS per job on
    /// Quasar. One generation at a time is therefore the explicit initial
    /// default; it is intentionally unrelated to logical CPU count.
    #[must_use]
    pub fn conservative(cache_root: impl Into<PathBuf>, quota_bytes: u64) -> Self {
        Self {
            cache_root: cache_root.into(),
            quota_bytes,
            max_concurrent_generations: NonZeroUsize::MIN,
        }
    }
}

/// Metadata observed or guaranteed by a backend after it writes a proxy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedProxyMetadata {
    pub pixel_width: NonZeroU32,
    pub pixel_height: NonZeroU32,
    pub channel_depth: ProxyChannelDepth,
    pub alpha: ProxyAlphaPolicy,
    pub encoding: ProxyEncodingRef,
    pub color_space: ColorSpaceId,
    pub embedded_icc_fingerprint: Option<RepresentationFingerprint>,
    pub dynamic_range: ProxyDynamicRangeDescription,
    pub orientation: ProxyStoredOrientation,
}

/// Backend boundary used by the project service.
pub trait ProxyGenerator: Send + Sync {
    #[must_use]
    fn exact_ref(&self) -> ProxyGeneratorRef;

    /// Writes exactly one proxy payload to `destination`.
    ///
    /// # Errors
    ///
    /// Returns a backend error when the requested exact profile is unsupported
    /// or generation cannot complete.
    fn generate(
        &self,
        request: &ProxyRequest,
        source: &MaterializedRepresentation,
        destination: &Path,
    ) -> Result<GeneratedProxyMetadata, ProxyGenerationError>;
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("proxy generation failed: {message}")]
pub struct ProxyGenerationError {
    pub message: String,
}

impl ProxyGenerationError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProxyArtifactDisposition {
    CacheHit,
    Generated,
    SharedInFlight,
}

/// A verified runtime handle to a disposable proxy payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyArtifact {
    pub descriptor: ProxyDescriptor,
    pub local_path: PathBuf,
    pub disposition: ProxyArtifactDisposition,
    lease: Arc<()>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProxyServiceMetrics {
    pub cache_hits: u64,
    pub generated: u64,
    pub deduplicated_waiters: u64,
    pub corruption_recoveries: u64,
    pub peak_active_generations: usize,
}

/// Project-scoped shared service with pre-scheduling request deduplication.
pub struct ProjectProxyService<G> {
    project_id: ProjectId,
    config: ProxyServiceConfig,
    generator: Arc<G>,
    in_flight: Mutex<HashMap<ProxyCacheKey, Arc<InFlight>>>,
    cache_operations: Mutex<()>,
    leases: Mutex<HashMap<ProxyCacheKey, Weak<()>>>,
    limiter: GenerationLimiter,
    cache_hits: AtomicU64,
    generated: AtomicU64,
    deduplicated_waiters: AtomicU64,
    corruption_recoveries: AtomicU64,
}

impl<G: ProxyGenerator> ProjectProxyService<G> {
    /// Opens one project's derived proxy cache.
    ///
    /// # Errors
    ///
    /// Returns [`ProxyServiceError::Io`] when cache directories cannot be
    /// created.
    pub fn open(
        project_id: ProjectId,
        config: ProxyServiceConfig,
        generator: G,
    ) -> Result<Self, ProxyServiceError> {
        let project_root = config.cache_root.join(project_id.to_string());
        create_dir_all(&project_root.join("objects"))?;
        create_dir_all(&project_root.join("staging"))?;
        Ok(Self {
            project_id,
            limiter: GenerationLimiter::new(config.max_concurrent_generations),
            config,
            generator: Arc::new(generator),
            in_flight: Mutex::new(HashMap::new()),
            cache_operations: Mutex::new(()),
            leases: Mutex::new(HashMap::new()),
            cache_hits: AtomicU64::new(0),
            generated: AtomicU64::new(0),
            deduplicated_waiters: AtomicU64::new(0),
            corruption_recoveries: AtomicU64::new(0),
        })
    }

    /// Returns or generates one exact proxy.
    ///
    /// Identical requests join the in-flight result before the leader waits for
    /// a generation slot, so followers never consume scheduler capacity.
    ///
    /// # Errors
    ///
    /// Returns a scoped materialization, generation, cache, or validation error.
    pub fn request(
        &self,
        request: &ProxyRequest,
        materializer: &dyn RepresentationMaterializer,
    ) -> Result<ProxyArtifact, ProxyServiceError> {
        if request.project_id != self.project_id {
            return Err(ProxyServiceError::WrongProject {
                expected: self.project_id,
                actual: request.project_id,
            });
        }
        let generator_ref = self.generator.exact_ref();
        let cache_key = request
            .cache_key(&generator_ref)
            .map_err(|error| ProxyServiceError::Contract(error.to_string()))?;

        if let Some(artifact) = self.load_cached(request, &generator_ref, cache_key)? {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(artifact);
        }

        let (flight, leader) = {
            let mut flights = lock(&self.in_flight);
            if let Some(flight) = flights.get(&cache_key) {
                (Arc::clone(flight), false)
            } else {
                let flight = Arc::new(InFlight::default());
                flights.insert(cache_key, Arc::clone(&flight));
                (flight, true)
            }
        };

        if !leader {
            self.deduplicated_waiters.fetch_add(1, Ordering::Relaxed);
            let mut artifact = flight.wait()?;
            artifact.disposition = ProxyArtifactDisposition::SharedInFlight;
            return Ok(artifact);
        }

        let result = self.generate_leader(request, &generator_ref, cache_key, materializer);
        flight.complete(result.clone());
        let mut flights = lock(&self.in_flight);
        if flights
            .get(&cache_key)
            .is_some_and(|current| Arc::ptr_eq(current, &flight))
        {
            flights.remove(&cache_key);
        }
        result
    }

    #[must_use]
    pub fn metrics(&self) -> ProxyServiceMetrics {
        ProxyServiceMetrics {
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            generated: self.generated.load(Ordering::Relaxed),
            deduplicated_waiters: self.deduplicated_waiters.load(Ordering::Relaxed),
            corruption_recoveries: self.corruption_recoveries.load(Ordering::Relaxed),
            peak_active_generations: self.limiter.peak(),
        }
    }

    #[must_use]
    pub const fn max_concurrent_generations(&self) -> NonZeroUsize {
        self.config.max_concurrent_generations
    }

    /// Deletes only this project's derived proxy objects and staging files.
    /// Authoritative project state is outside this root and is untouched.
    ///
    /// # Errors
    ///
    /// Returns [`ProxyServiceError::CacheBusy`] while requests or artifact
    /// leases are live, or an I/O error if deletion cannot complete.
    pub fn clear_cache(&self) -> Result<(), ProxyServiceError> {
        if !lock(&self.in_flight).is_empty() {
            return Err(ProxyServiceError::CacheBusy);
        }
        let _cache_operation = lock(&self.cache_operations);
        if lock(&self.leases)
            .values()
            .any(|lease| lease.strong_count() > 0)
        {
            return Err(ProxyServiceError::CacheBusy);
        }
        let project_root = self.project_root();
        remove_dir_if_exists(&project_root)?;
        create_dir_all(&project_root.join("objects"))?;
        create_dir_all(&project_root.join("staging"))?;
        Ok(())
    }

    fn generate_leader(
        &self,
        request: &ProxyRequest,
        generator_ref: &ProxyGeneratorRef,
        cache_key: ProxyCacheKey,
        materializer: &dyn RepresentationMaterializer,
    ) -> Result<ProxyArtifact, ProxyServiceError> {
        if let Some(artifact) = self.load_cached(request, generator_ref, cache_key)? {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(artifact);
        }

        let _slot = self.limiter.acquire();
        let source = materializer
            .materialize(&RepresentationMaterializationRequest {
                asset_id: request.asset_id,
                representation_id: request.representation_id,
                expected_fingerprint: request.source_fingerprint,
            })
            .map_err(ProxyServiceError::Materialization)?;
        if source.fingerprint != request.source_fingerprint {
            return Err(ProxyServiceError::SourceFingerprintMismatch {
                expected: request.source_fingerprint,
                actual: source.fingerprint,
            });
        }
        if source.asset_id != request.asset_id
            || source.representation_id != request.representation_id
        {
            return Err(ProxyServiceError::MaterializedIdentityMismatch);
        }

        let staging = self
            .project_root()
            .join("staging")
            .join(format!("{cache_key}-{}", Uuid::new_v4()));
        create_dir_all(&staging)?;
        let staged_payload = staging.join("proxy");
        let generated = match self.generator.generate(request, &source, &staged_payload) {
            Ok(generated) => generated,
            Err(error) => {
                let _ = remove_dir_if_exists(&staging);
                return Err(ProxyServiceError::Generation(error));
            }
        };
        let result = self.publish(
            request,
            generator_ref,
            cache_key,
            generated,
            &staging,
            &staged_payload,
        );
        if result.is_err() {
            let _ = remove_dir_if_exists(&staging);
        }
        result
    }

    fn publish(
        &self,
        request: &ProxyRequest,
        generator_ref: &ProxyGeneratorRef,
        cache_key: ProxyCacheKey,
        generated: GeneratedProxyMetadata,
        staging: &Path,
        staged_payload: &Path,
    ) -> Result<ProxyArtifact, ProxyServiceError> {
        Self::validate_generated_profile(request, &generated)?;
        sync_file(staged_payload)?;
        let (content_fingerprint, byte_length) = fingerprint_file(staged_payload)?;
        if byte_length > self.config.quota_bytes {
            return Err(ProxyServiceError::ArtifactExceedsQuota {
                artifact_bytes: byte_length,
                quota_bytes: self.config.quota_bytes,
            });
        }
        let descriptor = ProxyDescriptor {
            cache_key,
            source_fingerprint: request.source_fingerprint,
            profile: request
                .profile
                .exact_ref()
                .map_err(|error| ProxyServiceError::Contract(error.to_string()))?,
            generator: generator_ref.clone(),
            pixel_width: generated.pixel_width,
            pixel_height: generated.pixel_height,
            channel_depth: generated.channel_depth,
            alpha: generated.alpha,
            encoding: generated.encoding,
            color_space: generated.color_space,
            embedded_icc_fingerprint: generated.embedded_icc_fingerprint,
            dynamic_range: generated.dynamic_range,
            orientation: generated.orientation,
            content_fingerprint,
            byte_length,
        };
        write_json_synced(&staging.join("descriptor.json"), &descriptor)?;
        write_access(&staging.join("access"))?;
        let staged_bytes = directory_bytes(staging)?;
        if staged_bytes > self.config.quota_bytes {
            return Err(ProxyServiceError::ArtifactExceedsQuota {
                artifact_bytes: staged_bytes,
                quota_bytes: self.config.quota_bytes,
            });
        }
        sync_directory(staging)?;

        let _cache_operation = lock(&self.cache_operations);
        let destination = self.entry_path(cache_key);
        if destination.exists()
            && let Some(artifact) = self.load_cached_locked(request, generator_ref, cache_key)?
        {
            remove_dir_if_exists(staging)?;
            return Ok(artifact);
        }
        if let Err(error) = fs::rename(staging, &destination) {
            if destination.exists()
                && let Some(artifact) =
                    self.load_cached_locked(request, generator_ref, cache_key)?
            {
                remove_dir_if_exists(staging)?;
                return Ok(artifact);
            }
            return Err(io_error("rename", &destination, error));
        }
        sync_directory(&self.project_root().join("objects"))?;
        self.generated.fetch_add(1, Ordering::Relaxed);
        self.enforce_quota(Some(cache_key))?;
        Ok(ProxyArtifact {
            descriptor,
            local_path: destination.join("proxy"),
            disposition: ProxyArtifactDisposition::Generated,
            lease: self.lease_for(cache_key),
        })
    }

    fn load_cached(
        &self,
        request: &ProxyRequest,
        generator: &ProxyGeneratorRef,
        cache_key: ProxyCacheKey,
    ) -> Result<Option<ProxyArtifact>, ProxyServiceError> {
        let _cache_operation = lock(&self.cache_operations);
        self.load_cached_locked(request, generator, cache_key)
    }

    fn load_cached_locked(
        &self,
        request: &ProxyRequest,
        generator: &ProxyGeneratorRef,
        cache_key: ProxyCacheKey,
    ) -> Result<Option<ProxyArtifact>, ProxyServiceError> {
        let entry = self.entry_path(cache_key);
        if !entry.exists() {
            return Ok(None);
        }
        match Self::validate_entry(request, generator, cache_key, &entry) {
            Ok(descriptor) => {
                write_access(&entry.join("access"))?;
                Ok(Some(ProxyArtifact {
                    descriptor,
                    local_path: entry.join("proxy"),
                    disposition: ProxyArtifactDisposition::CacheHit,
                    lease: self.lease_for(cache_key),
                }))
            }
            Err(ProxyServiceError::CorruptCache { .. }) => {
                remove_dir_if_exists(&entry)?;
                self.corruption_recoveries.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn validate_entry(
        request: &ProxyRequest,
        generator: &ProxyGeneratorRef,
        cache_key: ProxyCacheKey,
        entry: &Path,
    ) -> Result<ProxyDescriptor, ProxyServiceError> {
        let descriptor_path = entry.join("descriptor.json");
        let bytes = fs::read(&descriptor_path)
            .map_err(|error| corrupt(cache_key, format!("descriptor read failed: {error}")))?;
        let descriptor: ProxyDescriptor = serde_json::from_slice(&bytes)
            .map_err(|error| corrupt(cache_key, format!("descriptor decode failed: {error}")))?;
        let expected_profile = request
            .profile
            .exact_ref()
            .map_err(|error| ProxyServiceError::Contract(error.to_string()))?;
        if descriptor.cache_key != cache_key
            || descriptor.source_fingerprint != request.source_fingerprint
            || descriptor.profile != expected_profile
            || descriptor.generator != *generator
        {
            return Err(corrupt(cache_key, "descriptor identity mismatch"));
        }
        let payload = entry.join("proxy");
        let (actual_fingerprint, actual_bytes) = fingerprint_file(&payload)
            .map_err(|error| corrupt(cache_key, format!("payload verification failed: {error}")))?;
        if descriptor.content_fingerprint != actual_fingerprint
            || descriptor.byte_length != actual_bytes
        {
            return Err(corrupt(cache_key, "payload fingerprint or length mismatch"));
        }
        Ok(descriptor)
    }

    fn validate_generated_profile(
        request: &ProxyRequest,
        generated: &GeneratedProxyMetadata,
    ) -> Result<(), ProxyServiceError> {
        if generated.channel_depth != request.profile.channel_depth
            || generated.alpha != request.profile.alpha
            || generated.encoding != request.profile.encoding
        {
            return Err(ProxyServiceError::GeneratorContractViolation(
                "generated depth, alpha, or encoding differs from the exact profile".to_owned(),
            ));
        }
        let within_bounds = match request.profile.sizing {
            photara_core::ProxySizing::FitWithin {
                max_width,
                max_height,
            } => generated.pixel_width <= max_width && generated.pixel_height <= max_height,
            photara_core::ProxySizing::LongEdge { pixels } => {
                generated.pixel_width.max(generated.pixel_height) <= pixels
            }
        };
        if !within_bounds {
            return Err(ProxyServiceError::GeneratorContractViolation(
                "generated dimensions exceed the exact profile".to_owned(),
            ));
        }
        let expected_orientation = match request.profile.orientation {
            photara_core::ProxyOrientationPolicy::PreserveMetadata => {
                ProxyStoredOrientation::MetadataPreserved
            }
            photara_core::ProxyOrientationPolicy::NormalizePixels => {
                ProxyStoredOrientation::PixelsNormalized
            }
        };
        if generated.orientation != expected_orientation {
            return Err(ProxyServiceError::GeneratorContractViolation(
                "generated orientation differs from the exact profile".to_owned(),
            ));
        }
        if let photara_core::ProxyColorPolicy::Convert { destination, .. } = &request.profile.color
            && generated.color_space != *destination
        {
            return Err(ProxyServiceError::GeneratorContractViolation(
                "generated color space differs from the exact profile".to_owned(),
            ));
        }
        let dynamic_range_matches = matches!(
            (&request.profile.dynamic_range, generated.dynamic_range),
            (
                photara_core::ProxyDynamicRangePolicy::SdrCompatible { .. },
                ProxyDynamicRangeDescription::Sdr { .. }
            ) | (
                photara_core::ProxyDynamicRangePolicy::HdrCapable { .. },
                ProxyDynamicRangeDescription::Hdr { .. }
            ) | (
                photara_core::ProxyDynamicRangePolicy::PreserveSource,
                ProxyDynamicRangeDescription::Sdr { .. } | ProxyDynamicRangeDescription::Hdr { .. }
            )
        );
        if !dynamic_range_matches {
            return Err(ProxyServiceError::GeneratorContractViolation(
                "generated dynamic-range description differs from the exact profile".to_owned(),
            ));
        }
        Ok(())
    }

    fn enforce_quota(&self, preserve: Option<ProxyCacheKey>) -> Result<(), ProxyServiceError> {
        let objects = self.project_root().join("objects");
        let mut entries = Vec::new();
        let mut total = 0_u64;
        for item in fs::read_dir(&objects).map_err(|error| io_error("read_dir", &objects, error))? {
            let item = item.map_err(|error| io_error("read_dir entry", &objects, error))?;
            let path = item.path();
            if !path.is_dir() {
                continue;
            }
            let key = item.file_name().to_string_lossy().into_owned();
            let bytes = directory_bytes(&path)?;
            let accessed = read_access(&path.join("access")).unwrap_or(0);
            total = total.saturating_add(bytes);
            entries.push((accessed, key, bytes, path));
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.3.cmp(&right.3)));
        for (_, key, bytes, path) in entries {
            if total <= self.config.quota_bytes {
                break;
            }
            if preserve.is_some_and(|preserve| key == preserve.to_string()) {
                continue;
            }
            let leased = lock(&self.leases)
                .iter()
                .any(|(cache_key, lease)| cache_key.to_string() == key && lease.strong_count() > 0);
            if leased {
                continue;
            }
            remove_dir_if_exists(&path)?;
            total = total.saturating_sub(bytes);
        }
        Ok(())
    }

    fn lease_for(&self, cache_key: ProxyCacheKey) -> Arc<()> {
        let mut leases = lock(&self.leases);
        if let Some(lease) = leases.get(&cache_key).and_then(Weak::upgrade) {
            return lease;
        }
        let lease = Arc::new(());
        leases.insert(cache_key, Arc::downgrade(&lease));
        lease
    }

    fn project_root(&self) -> PathBuf {
        self.config.cache_root.join(self.project_id.to_string())
    }

    fn entry_path(&self, cache_key: ProxyCacheKey) -> PathBuf {
        self.project_root()
            .join("objects")
            .join(cache_key.to_string())
    }
}

#[derive(Default)]
struct InFlight {
    result: Mutex<Option<Result<ProxyArtifact, ProxyServiceError>>>,
    ready: Condvar,
}

impl InFlight {
    fn wait(&self) -> Result<ProxyArtifact, ProxyServiceError> {
        let mut result = lock(&self.result);
        while result.is_none() {
            result = self
                .ready
                .wait(result)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        result.clone().expect("in-flight result is ready")
    }

    fn complete(&self, result: Result<ProxyArtifact, ProxyServiceError>) {
        *lock(&self.result) = Some(result);
        self.ready.notify_all();
    }
}

struct GenerationLimiter {
    limit: usize,
    state: Mutex<LimiterState>,
    available: Condvar,
    peak: AtomicUsize,
}

#[derive(Default)]
struct LimiterState {
    active: usize,
}

impl GenerationLimiter {
    fn new(limit: NonZeroUsize) -> Self {
        Self {
            limit: limit.get(),
            state: Mutex::new(LimiterState::default()),
            available: Condvar::new(),
            peak: AtomicUsize::new(0),
        }
    }

    fn acquire(&self) -> GenerationSlot<'_> {
        let mut state = lock(&self.state);
        while state.active >= self.limit {
            state = self
                .available
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.active += 1;
        self.peak.fetch_max(state.active, Ordering::Relaxed);
        GenerationSlot { limiter: self }
    }

    fn peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }
}

struct GenerationSlot<'a> {
    limiter: &'a GenerationLimiter,
}

impl Drop for GenerationSlot<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.limiter.state);
        state.active -= 1;
        self.limiter.available.notify_one();
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProxyServiceError {
    #[error("project asset {0} is unknown")]
    UnknownAsset(AssetId),
    #[error("project asset {asset_id} has no representation with capability {required_capability}")]
    NoCompatibleRepresentation {
        asset_id: AssetId,
        required_capability: &'static str,
    },
    #[error("proxy request belongs to project {actual}, expected {expected}")]
    WrongProject {
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("proxy contract error: {0}")]
    Contract(String),
    #[error(transparent)]
    Materialization(RepresentationMaterializationError),
    #[error("materialized source fingerprint {actual:?} differs from expected {expected:?}")]
    SourceFingerprintMismatch {
        expected: RepresentationFingerprint,
        actual: RepresentationFingerprint,
    },
    #[error("materializer returned a different asset or representation identity")]
    MaterializedIdentityMismatch,
    #[error(transparent)]
    Generation(ProxyGenerationError),
    #[error("proxy generator violated its exact profile: {0}")]
    GeneratorContractViolation(String),
    #[error("proxy artifact is {artifact_bytes} bytes, exceeding quota {quota_bytes}")]
    ArtifactExceedsQuota {
        artifact_bytes: u64,
        quota_bytes: u64,
    },
    #[error("proxy cache cannot be cleared while requests or artifact leases are live")]
    CacheBusy,
    #[error("corrupt proxy cache entry {cache_key}: {reason}")]
    CorruptCache {
        cache_key: ProxyCacheKey,
        reason: String,
    },
    #[error("proxy cache I/O during {operation} at {path}: {message}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn create_dir_all(path: &Path) -> Result<(), ProxyServiceError> {
    fs::create_dir_all(path).map_err(|error| io_error("create_dir_all", path, error))
}

fn remove_dir_if_exists(path: &Path) -> Result<(), ProxyServiceError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("remove_dir_all", path, error)),
    }
}

fn write_json_synced<T: Serialize>(path: &Path, value: &T) -> Result<(), ProxyServiceError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| ProxyServiceError::Contract(error.to_string()))?;
    let mut file = File::create(path).map_err(|error| io_error("create", path, error))?;
    file.write_all(&bytes)
        .map_err(|error| io_error("write", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("sync_all", path, error))
}

fn write_access(path: &Path) -> Result<(), ProxyServiceError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error("open access marker", path, error))?;
    write!(file, "{now}").map_err(|error| io_error("write access marker", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("sync access marker", path, error))
}

fn read_access(path: &Path) -> Option<u128> {
    fs::read_to_string(path).ok()?.parse().ok()
}

fn sync_directory(path: &Path) -> Result<(), ProxyServiceError> {
    let directory = File::open(path).map_err(|error| io_error("open directory", path, error))?;
    directory
        .sync_all()
        .map_err(|error| io_error("sync directory", path, error))
}

fn sync_file(path: &Path) -> Result<(), ProxyServiceError> {
    let file = File::open(path).map_err(|error| io_error("open payload for sync", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("sync payload", path, error))
}

fn fingerprint_file(path: &Path) -> Result<(RepresentationFingerprint, u64), ProxyServiceError> {
    let file = File::open(path).map_err(|error| io_error("open", path, error))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    let mut byte_length = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| io_error("read", path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        byte_length = byte_length.saturating_add(read as u64);
    }
    Ok((
        RepresentationFingerprint::sha256(hasher.finalize().into()),
        byte_length,
    ))
}

fn directory_bytes(path: &Path) -> Result<u64, ProxyServiceError> {
    let mut total = 0_u64;
    for item in fs::read_dir(path).map_err(|error| io_error("read_dir", path, error))? {
        let item = item.map_err(|error| io_error("read_dir entry", path, error))?;
        let metadata = item
            .metadata()
            .map_err(|error| io_error("metadata", &item.path(), error))?;
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn corrupt(cache_key: ProxyCacheKey, reason: impl Into<String>) -> ProxyServiceError {
    ProxyServiceError::CorruptCache {
        cache_key,
        reason: reason.into(),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn io_error(operation: &'static str, path: &Path, error: io::Error) -> ProxyServiceError {
    ProxyServiceError::Io {
        operation,
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
