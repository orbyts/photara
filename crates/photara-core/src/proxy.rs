use std::{collections::BTreeMap, num::NonZeroU32};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AssetId, AssetRepresentationId, CanonicalDigest, ColorSpaceId, ProjectId, ProxyEncodingId,
    ProxyEncodingVersion, ProxyGeneratorId, ProxyGeneratorVersion, ProxyProfileId,
    ProxyProfileVersion, RepresentationFingerprint, RequestId, ToneMapOperatorId, ToneMapVersion,
    canonical_digest,
};

/// Why a reusable proxy is being produced. Purpose does not identify a UI owner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyPurpose {
    Thumbnail,
    AuthoringPreview,
}

/// Backend-neutral output sizing. Both variants preserve source aspect ratio.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProxySizing {
    FitWithin {
        max_width: NonZeroU32,
        max_height: NonZeroU32,
    },
    LongEdge {
        pixels: NonZeroU32,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyOrientationPolicy {
    PreserveMetadata,
    NormalizePixels,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyResamplingFilter {
    Box,
    Triangle,
    CatmullRom,
    Lanczos3,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyChannelDepth {
    U8,
    U16,
    F16,
    F32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyAlphaPolicy {
    Preserve,
    Opaque,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyRenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

/// Color conversion requested by a profile. The destination is a semantic,
/// namespaced color-space identifier rather than a platform color object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProxyColorPolicy {
    PreserveEmbedded,
    Convert {
        destination: ColorSpaceId,
        rendering_intent: ProxyRenderingIntent,
        black_point_compensation: bool,
    },
}

/// A versioned tone-map recipe. Parameters are part of cache identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToneMapPolicy {
    pub operator: ToneMapOperatorId,
    pub version: ToneMapVersion,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
}

/// Exact encoder recipe. Its version covers compression and metadata-writing
/// behavior that can change output bytes without changing image semantics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyEncodingRef {
    pub id: ProxyEncodingId,
    pub version: ProxyEncodingVersion,
}

/// Exact derived-byte implementation selected by the project proxy service.
/// This is cache identity, not a platform type exposed to graph consumers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyGeneratorRef {
    pub id: ProxyGeneratorId,
    pub version: ProxyGeneratorVersion,
}

/// Dynamic-range behavior is explicit and independent from the color-space
/// conversion. No backend-specific EDR, gain-map, or pixel-buffer type leaks in.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProxyDynamicRangePolicy {
    PreserveSource,
    SdrCompatible { tone_map: ToneMapPolicy },
    HdrCapable { sdr_fallback: ToneMapPolicy },
}

/// Every field that can affect encoded proxy bytes belongs in this profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyProfile {
    pub id: ProxyProfileId,
    pub version: ProxyProfileVersion,
    pub purpose: ProxyPurpose,
    pub sizing: ProxySizing,
    pub allow_upscale: bool,
    pub resampling: ProxyResamplingFilter,
    pub orientation: ProxyOrientationPolicy,
    pub color: ProxyColorPolicy,
    pub dynamic_range: ProxyDynamicRangePolicy,
    pub channel_depth: ProxyChannelDepth,
    pub alpha: ProxyAlphaPolicy,
    pub encoding: ProxyEncodingRef,
}

impl ProxyProfile {
    /// Produces an exact profile reference including a digest of all policy.
    ///
    /// # Errors
    ///
    /// Returns a JSON serialization error if a custom tone-map parameter cannot
    /// be represented canonically.
    pub fn exact_ref(&self) -> Result<ProxyProfileRef, serde_json::Error> {
        Ok(ProxyProfileRef {
            id: self.id.clone(),
            version: self.version,
            digest: canonical_digest(self)?,
        })
    }
}

/// Stable profile coordinate plus exact policy digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyProfileRef {
    pub id: ProxyProfileId,
    pub version: ProxyProfileVersion,
    pub digest: CanonicalDigest,
}

/// One project-scoped request. Consumer identity is intentionally absent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyRequest {
    pub request_id: RequestId,
    pub project_id: ProjectId,
    pub asset_id: AssetId,
    pub representation_id: AssetRepresentationId,
    pub source_fingerprint: RepresentationFingerprint,
    pub profile: ProxyProfile,
}

impl ProxyRequest {
    /// Computes content-addressed cache identity. Project and semantic IDs are
    /// not included because they do not affect the derived bytes.
    ///
    /// # Errors
    ///
    /// Returns a JSON serialization error if the profile cannot be represented
    /// canonically.
    pub fn cache_key(
        &self,
        generator: &ProxyGeneratorRef,
    ) -> Result<ProxyCacheKey, serde_json::Error> {
        ProxyCacheKey::derive(self.source_fingerprint, &self.profile, generator)
    }
}

/// Content-addressed identity for a derived proxy object.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ProxyCacheKey(CanonicalDigest);

impl ProxyCacheKey {
    /// Derives the key from source bytes and the complete output profile.
    ///
    /// # Errors
    ///
    /// Returns a JSON serialization error if the key input cannot be encoded.
    pub fn derive(
        source_fingerprint: RepresentationFingerprint,
        profile: &ProxyProfile,
        generator: &ProxyGeneratorRef,
    ) -> Result<Self, serde_json::Error> {
        #[derive(Serialize)]
        struct KeyInput<'a> {
            contract_version: u32,
            source_fingerprint: RepresentationFingerprint,
            profile: &'a ProxyProfile,
            generator: &'a ProxyGeneratorRef,
        }

        canonical_digest(&KeyInput {
            contract_version: 1,
            source_fingerprint,
            profile,
            generator,
        })
        .map(Self)
    }

    #[must_use]
    pub const fn digest(self) -> CanonicalDigest {
        self.0
    }
}

impl std::fmt::Display for ProxyCacheKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProxyDynamicRangeDescription {
    Sdr {
        reference_white_nits: Option<NonZeroU32>,
    },
    Hdr {
        reference_white_nits: Option<NonZeroU32>,
        headroom_millistops: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyStoredOrientation {
    MetadataPreserved,
    PixelsNormalized,
}

/// Derived cache metadata. It deliberately contains no authoritative project
/// state, consumer ownership, or machine path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyDescriptor {
    pub cache_key: ProxyCacheKey,
    pub source_fingerprint: RepresentationFingerprint,
    pub profile: ProxyProfileRef,
    pub generator: ProxyGeneratorRef,
    pub pixel_width: NonZeroU32,
    pub pixel_height: NonZeroU32,
    pub channel_depth: ProxyChannelDepth,
    pub alpha: ProxyAlphaPolicy,
    pub encoding: ProxyEncodingRef,
    pub color_space: ColorSpaceId,
    pub embedded_icc_fingerprint: Option<RepresentationFingerprint>,
    pub dynamic_range: ProxyDynamicRangeDescription,
    pub orientation: ProxyStoredOrientation,
    pub content_fingerprint: RepresentationFingerprint,
    pub byte_length: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_map(exposure_bias_millistops: i64) -> ToneMapPolicy {
        ToneMapPolicy {
            operator: ToneMapOperatorId::parse("photara.tonemap.reference").unwrap(),
            version: ToneMapVersion::first(),
            parameters: BTreeMap::from([(
                "exposure_bias_millistops".to_owned(),
                Value::from(exposure_bias_millistops),
            )]),
        }
    }

    fn profile() -> ProxyProfile {
        ProxyProfile {
            id: ProxyProfileId::parse("photara.proxy.authoring-sdr").unwrap(),
            version: ProxyProfileVersion::first(),
            purpose: ProxyPurpose::AuthoringPreview,
            sizing: ProxySizing::LongEdge {
                pixels: NonZeroU32::new(2048).unwrap(),
            },
            allow_upscale: false,
            resampling: ProxyResamplingFilter::Lanczos3,
            orientation: ProxyOrientationPolicy::NormalizePixels,
            color: ProxyColorPolicy::Convert {
                destination: ColorSpaceId::parse("photara.color.display-p3").unwrap(),
                rendering_intent: ProxyRenderingIntent::RelativeColorimetric,
                black_point_compensation: true,
            },
            dynamic_range: ProxyDynamicRangePolicy::SdrCompatible {
                tone_map: tone_map(0),
            },
            channel_depth: ProxyChannelDepth::U16,
            alpha: ProxyAlphaPolicy::Opaque,
            encoding: ProxyEncodingRef {
                id: ProxyEncodingId::parse("photara.proxy-encoding.tiff").unwrap(),
                version: ProxyEncodingVersion::first(),
            },
        }
    }

    fn request(profile: ProxyProfile) -> ProxyRequest {
        ProxyRequest {
            request_id: RequestId::new(),
            project_id: ProjectId::new(),
            asset_id: AssetId::new(),
            representation_id: AssetRepresentationId::new(),
            source_fingerprint: RepresentationFingerprint::sha256([7; 32]),
            profile,
        }
    }

    fn generator() -> ProxyGeneratorRef {
        ProxyGeneratorRef {
            id: ProxyGeneratorId::parse("photara.proxy-generator.reference").unwrap(),
            version: ProxyGeneratorVersion::first(),
        }
    }

    #[test]
    fn cache_key_ignores_request_and_semantic_identity() {
        let left = request(profile());
        let right = request(profile());
        assert_eq!(
            left.cache_key(&generator()).unwrap(),
            right.cache_key(&generator()).unwrap()
        );
    }

    #[test]
    fn source_color_hdr_and_tone_map_policy_change_cache_identity() {
        let base = request(profile());

        let mut changed_source = base.clone();
        changed_source.source_fingerprint = RepresentationFingerprint::sha256([8; 32]);
        assert_ne!(
            base.cache_key(&generator()).unwrap(),
            changed_source.cache_key(&generator()).unwrap()
        );

        let mut changed_color = base.clone();
        changed_color.profile.color = ProxyColorPolicy::PreserveEmbedded;
        assert_ne!(
            base.cache_key(&generator()).unwrap(),
            changed_color.cache_key(&generator()).unwrap()
        );

        let mut changed_hdr = base.clone();
        changed_hdr.profile.dynamic_range = ProxyDynamicRangePolicy::PreserveSource;
        assert_ne!(
            base.cache_key(&generator()).unwrap(),
            changed_hdr.cache_key(&generator()).unwrap()
        );

        let mut changed_tone_map = base.clone();
        changed_tone_map.profile.dynamic_range = ProxyDynamicRangePolicy::SdrCompatible {
            tone_map: tone_map(125),
        };
        assert_ne!(
            base.cache_key(&generator()).unwrap(),
            changed_tone_map.cache_key(&generator()).unwrap()
        );

        let mut changed_generator = generator();
        changed_generator.version = ProxyGeneratorVersion::new(2).unwrap();
        assert_ne!(
            base.cache_key(&generator()).unwrap(),
            base.cache_key(&changed_generator).unwrap()
        );
    }

    #[test]
    fn exact_profile_reference_covers_encoding_and_policy() {
        let base = profile();
        let mut changed = base.clone();
        changed.encoding.id = ProxyEncodingId::parse("photara.proxy-encoding.png").unwrap();
        assert_ne!(base.exact_ref().unwrap(), changed.exact_ref().unwrap());
    }
}
