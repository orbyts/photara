use std::{num::NonZeroU32, path::PathBuf, process::Command};

use photara_core::{
    ColorSpaceId, MaterializedRepresentation, ProxyAlphaPolicy, ProxyChannelDepth,
    ProxyColorPolicy, ProxyDynamicRangeDescription, ProxyDynamicRangePolicy, ProxyGeneratorId,
    ProxyGeneratorRef, ProxyGeneratorVersion, ProxyOrientationPolicy, ProxyProfile, ProxyPurpose,
    ProxyRenderingIntent, ProxyRequest, ProxyResamplingFilter, ProxySizing, ProxyStoredOrientation,
    RepresentationFingerprint, ToneMapVersion,
};
use serde::Deserialize;

use crate::{GeneratedProxyMetadata, ProxyGenerationError, ProxyGenerator};

const GENERATOR_ID: &str = "photara.proxy-generator.apple-imageio-core-image";
const PNG_ENCODING_ID: &str = "photara.proxy-encoding.png";
const TIFF_ENCODING_ID: &str = "photara.proxy-encoding.tiff";

/// Runtime configuration for the process-isolated Apple imaging adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageIoGeneratorConfig {
    pub helper_executable: PathBuf,
}

/// First production macOS generator selected by the Stage 6A measurements.
///
/// The helper uses `ImageIO` and Core Image without macOS 27 APIs. A short-lived
/// process contains each job's large decoder working set and returns only
/// backend-neutral metadata to Rust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageIoCoreImageGenerator {
    config: ImageIoGeneratorConfig,
}

impl ImageIoCoreImageGenerator {
    #[must_use]
    pub const fn new(config: ImageIoGeneratorConfig) -> Self {
        Self { config }
    }
}

impl ProxyGenerator for ImageIoCoreImageGenerator {
    fn exact_ref(&self) -> ProxyGeneratorRef {
        ProxyGeneratorRef {
            id: ProxyGeneratorId::parse(GENERATOR_ID).expect("built-in generator ID is valid"),
            version: ProxyGeneratorVersion::new(2).expect("built-in generator version is valid"),
        }
    }

    fn generate(
        &self,
        request: &ProxyRequest,
        source: &MaterializedRepresentation,
        destination: &std::path::Path,
    ) -> Result<GeneratedProxyMetadata, ProxyGenerationError> {
        let (mode, long_edge) = supported_mode(&request.profile)?;
        let metadata_path = destination.with_extension("metadata.json");
        let output = Command::new(&self.config.helper_executable)
            .arg(mode)
            .arg(&source.local_path)
            .arg(destination)
            .arg(long_edge.get().to_string())
            .arg(&metadata_path)
            .output()
            .map_err(|error| {
                ProxyGenerationError::new(format!(
                    "could not launch {}: {error}",
                    self.config.helper_executable.display()
                ))
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(ProxyGenerationError::new(format!(
                "Apple proxy helper exited with {}: {stderr}",
                output.status
            )));
        }
        let metadata_bytes = std::fs::read(&metadata_path).map_err(|error| {
            ProxyGenerationError::new(format!("could not read helper metadata: {error}"))
        })?;
        let _ = std::fs::remove_file(&metadata_path);
        let metadata: HelperMetadata =
            serde_json::from_slice(&metadata_bytes).map_err(|error| {
                ProxyGenerationError::new(format!("invalid helper metadata: {error}"))
            })?;
        let color_space = ColorSpaceId::parse(metadata.color_space_id).map_err(|error| {
            ProxyGenerationError::new(format!("invalid helper color-space identity: {error}"))
        })?;
        let embedded_icc_fingerprint = metadata
            .embedded_icc_fingerprint
            .as_deref()
            .map(parse_sha256_fingerprint)
            .transpose()?;
        let dynamic_range = match mode {
            "thumbnail-sdr" => ProxyDynamicRangeDescription::Sdr {
                reference_white_nits: None,
            },
            "authoring-hdr" => ProxyDynamicRangeDescription::Hdr {
                reference_white_nits: None,
                headroom_millistops: metadata.headroom_millistops,
            },
            _ => unreachable!("supported_mode returns a known mode"),
        };
        Ok(GeneratedProxyMetadata {
            pixel_width: metadata.pixel_width,
            pixel_height: metadata.pixel_height,
            channel_depth: request.profile.channel_depth,
            alpha: request.profile.alpha,
            encoding: request.profile.encoding.clone(),
            color_space,
            embedded_icc_fingerprint,
            dynamic_range,
            orientation: ProxyStoredOrientation::PixelsNormalized,
        })
    }
}

#[derive(Deserialize)]
struct HelperMetadata {
    pixel_width: NonZeroU32,
    pixel_height: NonZeroU32,
    color_space_id: String,
    embedded_icc_fingerprint: Option<String>,
    headroom_millistops: Option<u32>,
}

fn parse_sha256_fingerprint(
    value: &str,
) -> Result<RepresentationFingerprint, ProxyGenerationError> {
    if value.len() != 64 {
        return Err(ProxyGenerationError::new(
            "helper ICC fingerprint must contain 64 hexadecimal characters",
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| ProxyGenerationError::new("helper ICC fingerprint is not hexadecimal"))?;
    }
    Ok(RepresentationFingerprint::sha256(bytes))
}

fn supported_mode(
    profile: &ProxyProfile,
) -> Result<(&'static str, NonZeroU32), ProxyGenerationError> {
    let ProxySizing::LongEdge { pixels } = profile.sizing else {
        return Err(unsupported("only long-edge sizing is implemented"));
    };
    if profile.allow_upscale
        || profile.resampling != ProxyResamplingFilter::Lanczos3
        || profile.orientation != ProxyOrientationPolicy::NormalizePixels
        || profile.alpha != ProxyAlphaPolicy::Opaque
    {
        return Err(unsupported(
            "the initial Apple adapter requires no upscale, Lanczos3, normalized pixels, and opaque output",
        ));
    }
    let encoding_id = profile.encoding.id.as_str();
    let thumbnail_color_is_exact = matches!(
        &profile.color,
        ProxyColorPolicy::Convert {
            destination,
            rendering_intent: ProxyRenderingIntent::RelativeColorimetric,
            black_point_compensation: true,
        } if destination.as_str() == "photara.color.srgb"
    );
    let thumbnail_dynamic_range_is_exact = matches!(
        &profile.dynamic_range,
        ProxyDynamicRangePolicy::SdrCompatible { tone_map }
            if tone_map.operator.as_str() == "photara.tonemap.reinhard-global"
                && tone_map.version == ToneMapVersion::first()
                && tone_map.parameters.is_empty()
    );
    match (
        profile.purpose,
        profile.channel_depth,
        &profile.dynamic_range,
        encoding_id,
    ) {
        (
            ProxyPurpose::Thumbnail,
            ProxyChannelDepth::U8,
            ProxyDynamicRangePolicy::SdrCompatible { .. },
            PNG_ENCODING_ID,
        ) if thumbnail_color_is_exact
            && thumbnail_dynamic_range_is_exact
            && profile.encoding.version == photara_core::ProxyEncodingVersion::first() =>
        {
            Ok(("thumbnail-sdr", pixels))
        }
        (
            ProxyPurpose::AuthoringPreview | ProxyPurpose::Thumbnail,
            ProxyChannelDepth::F16,
            ProxyDynamicRangePolicy::PreserveSource,
            TIFF_ENCODING_ID,
        ) if profile.color == ProxyColorPolicy::PreserveEmbedded
            && profile.encoding.version == photara_core::ProxyEncodingVersion::first() =>
        {
            Ok(("authoring-hdr", pixels))
        }
        _ => Err(unsupported(
            "profile is outside the measured SDR PNG thumbnail and HDR F16 TIFF preview paths",
        )),
    }
}

fn unsupported(message: &str) -> ProxyGenerationError {
    ProxyGenerationError::new(format!("unsupported Apple proxy profile: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_accepts_only_the_measured_exact_profile_policies() {
        let generator = ImageIoCoreImageGenerator::new(ImageIoGeneratorConfig {
            helper_executable: PathBuf::from("unused-test-helper"),
        });
        assert_eq!(generator.exact_ref().version.get(), 2);
        let thumbnail = crate::standard_sdr_thumbnail_profile();
        let gallery = crate::standard_gallery_preview_profile();
        let layout = crate::standard_layout_interaction_preview_profile();
        let hdr = crate::standard_hdr_authoring_preview_profile();
        assert_eq!(supported_mode(&thumbnail).unwrap().0, "thumbnail-sdr");
        assert_eq!(supported_mode(&gallery).unwrap().0, "authoring-hdr");
        assert_eq!(supported_mode(&layout).unwrap().0, "authoring-hdr");
        assert_eq!(supported_mode(&hdr).unwrap().0, "authoring-hdr");

        let mut changed_intent = thumbnail;
        let ProxyColorPolicy::Convert {
            rendering_intent, ..
        } = &mut changed_intent.color
        else {
            unreachable!();
        };
        *rendering_intent = ProxyRenderingIntent::Perceptual;
        assert!(supported_mode(&changed_intent).is_err());
    }

    #[test]
    fn helper_icc_fingerprints_are_strict_sha256_hex() {
        let value = "42".repeat(32);
        assert_eq!(
            parse_sha256_fingerprint(&value).unwrap(),
            RepresentationFingerprint::sha256([0x42; 32])
        );
        assert!(parse_sha256_fingerprint("42").is_err());
        assert!(parse_sha256_fingerprint(&"zz".repeat(32)).is_err());
    }
}
