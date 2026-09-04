use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{
    PhotaraError, Result,
    config::{PhotaraConfig, validate_slug},
    project::ProjectRecord,
};

const FULL_FRAME_V1: &str = include_str!("../templates/full-frame/v1.json");
const STACKED_TWO_V1: &str = include_str!("../templates/stacked-two/v1.json");
const STACKED_THREE_V1: &str = include_str!("../templates/stacked-three/v1.json");
const STACKED_THREE_V2: &str = include_str!("../templates/stacked-three/v2.json");
const GRID_FOUR_V1: &str = include_str!("../templates/grid-four/v1.json");
const GRID_FOUR_THREADS_V1: &str = include_str!("../templates/grid-four-threads/v1.json");
const CONTINUOUS_PANORAMA_V1: &str = include_str!("../templates/continuous-panorama/v1.json");
const DYNAMIC_RANGE_COMPARISON_V1: &str =
    include_str!("../templates/dynamic-range-comparison/v1.json");
const DYNAMIC_RANGE_COMPARISON_V2: &str =
    include_str!("../templates/dynamic-range-comparison/v2.json");
const DYNAMIC_RANGE_COMPARISON_V3: &str =
    include_str!("../templates/dynamic-range-comparison/v3.json");
const EDIT_COMPARISON_V1: &str = include_str!("../templates/edit-comparison/v1.json");
const EDIT_COMPARISON_V2: &str = include_str!("../templates/edit-comparison/v2.json");
const LAYOUT_SCRIPT: &str = include_str!("../photoshop/Build Photara Layouts.psjs");
const LAYOUT_SCRIPT_NAME: &str = "Build Photara Layouts.psjs";
const LAYOUT_HANDOFF_NAME: &str = "Photara Layout Manifest.json";
const AUTHORING_SCRIPT: &str = include_str!("../photoshop/Author Photara Placement.psjs");
const AUTHORING_CAPTURE_SCRIPT: &str = include_str!("../photoshop/Capture Photara Placement.psjs");
const AUTHORING_SCRIPT_NAME: &str = "Author Photara Placement.psjs";
const AUTHORING_CAPTURE_SCRIPT_NAME: &str = "Capture Photara Placement.psjs";
const AUTHORING_MANIFEST_NAME: &str = "Photara Authoring Manifest.json";
const AUTHORING_REPORT_NAME: &str = "Photara Authoring Report.json";
const EDIT_SOURCE_HANDOFF_NAME: &str = "Photara Edit Comparison Source Manifest.json";
const EDIT_SOURCE_REPORT_NAME: &str = "Photara Edit Comparison Source Report.json";
const EDIT_SOURCE_REGISTRY_NAME: &str = "Photara Edit Comparison Source Registry.json";
const LAYOUT_MANIFEST_NAME: &str = "Photara Layout Manifest.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateRef {
    pub name: String,
    pub version: u32,
}

impl TemplateRef {
    pub fn parse(value: &str) -> Result<Self> {
        let (name, version) = value.split_once('@').ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "layout template reference {value:?} must use NAME@VERSION"
            ))
        })?;
        validate_slug(name).map_err(|message| {
            PhotaraError::Configuration(format!("invalid layout template name {name:?}: {message}"))
        })?;
        let version = version.parse::<u32>().map_err(|_| {
            PhotaraError::Configuration(format!(
                "layout template reference {value:?} has an invalid version"
            ))
        })?;
        if version == 0 {
            return Err(PhotaraError::Configuration(
                "layout template version must be greater than zero".into(),
            ));
        }
        Ok(Self {
            name: name.into(),
            version,
        })
    }

    fn display(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayoutTemplate {
    pub schema_version: u32,
    pub name: String,
    pub version: u32,
    pub display_name: String,
    pub kind: String,
    pub canvas: TemplateCanvas,
    pub slots: Vec<TemplateSlot>,
    pub decoration: TemplateDecoration,
    pub wsp: WspContract,
    #[serde(default)]
    pub custom_rows: bool,
    #[serde(default)]
    pub surface: Option<ContinuousSurface>,
    #[serde(default)]
    pub reference: Option<TemplateReferenceDocument>,
    #[serde(default)]
    pub comparison: Option<ComparisonContract>,
    #[serde(default)]
    pub edit_comparison: Option<EditComparisonContract>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplateReferenceDocument {
    pub filename: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_profile: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ComparisonContract {
    pub left_role: String,
    pub right_role: String,
    pub cells: ComparisonCells,
    pub hdr_headroom_ramp: NormalizedRect,
    pub hdr_headroom_sdr_base: String,
    pub hdr_headroom_hdr_top: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ComparisonCells {
    pub top_left: NormalizedRect,
    pub top_right: NormalizedRect,
    pub bottom_left: NormalizedRect,
    pub bottom_right: NormalizedRect,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditComparisonContract {
    pub before_role: String,
    pub after_role: String,
    pub before_rendering: String,
    pub cells: ComparisonCells,
    pub text_layers: EditComparisonTextLayers,
    pub text_style: EditComparisonTextStyle,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditComparisonTextLayers {
    pub top_camera: Vec<String>,
    pub top_capture: Vec<String>,
    pub bottom_camera: Vec<String>,
    pub bottom_capture: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditComparisonTextStyle {
    pub font_postscript_name: String,
    pub preferred_size_points: f64,
    pub minimum_size_points: f64,
    pub right_padding_pixels: u32,
    pub color_rgb: [f64; 3],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContinuousSurface {
    pub frame_aspect: String,
    pub frame_count: u32,
    pub flow: String,
    pub splitter: String,
    pub resolution_policy: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplateCanvas {
    pub sizing: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplateSlot {
    pub id: String,
    pub kind: String,
    pub bounds: NormalizedRect,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct NormalizedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Eq for NormalizedRect {}

impl StackedThreeParameters {
    fn validate(self) -> Result<()> {
        if self.row_percentages.contains(&0) {
            return Err(PhotaraError::Configuration(
                "stacked-three row percentages must all be greater than zero".into(),
            ));
        }
        let total: u16 = self
            .row_percentages
            .iter()
            .map(|value| u16::from(*value))
            .sum();
        if total > 100 {
            return Err(PhotaraError::Configuration(format!(
                "stacked-three row percentages total {total}%; the total cannot exceed 100%"
            )));
        }
        if total < 100 && self.underfill == StackedUnderfill::Error {
            return Err(PhotaraError::Configuration(format!(
                "stacked-three row percentages total {total}%; use --outer-letterbox to place the remaining {}% equally above and below the stack",
                100 - total
            )));
        }
        Ok(())
    }

    fn total(self) -> u16 {
        self.row_percentages
            .iter()
            .map(|value| u16::from(*value))
            .sum()
    }

    fn slots(self) -> [NormalizedRect; 3] {
        let total = self.total();
        let outer_padding = if total < 100 {
            f64::from(100 - total) / 200.0
        } else {
            0.0
        };
        let top_height = f64::from(self.row_percentages[0]) / 100.0;
        let middle_height = f64::from(self.row_percentages[1]) / 100.0;
        let bottom_height = f64::from(self.row_percentages[2]) / 100.0;
        [
            NormalizedRect {
                x: 0.0,
                y: outer_padding,
                width: 1.0,
                height: top_height,
            },
            NormalizedRect {
                x: 0.0,
                y: outer_padding + top_height,
                width: 1.0,
                height: middle_height,
            },
            NormalizedRect {
                x: 0.0,
                y: outer_padding + top_height + middle_height,
                width: 1.0,
                height: bottom_height,
            },
        ]
    }

    fn needs_background(self) -> bool {
        self.total() < 100 && self.underfill == StackedUnderfill::OuterLetterbox
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplateDecoration {
    pub background: bool,
    pub border: bool,
    pub text: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WspContract {
    pub mode: String,
    pub hdr_layer: String,
    pub sdr_layer: String,
    pub hdr_above_sdr: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TemplateInstallReport {
    pub schema_version: u32,
    pub installed: Vec<PathBuf>,
    pub verified: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TemplateReferenceInstallReport {
    pub schema_version: u32,
    pub template: String,
    pub source: PathBuf,
    pub installed_path: PathBuf,
    pub sha256: String,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostPlatform {
    Instagram,
    Threads,
}

impl PostPlatform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Instagram => "instagram",
            Self::Threads => "threads",
        }
    }

    fn profile(self) -> PlatformProfile {
        match self {
            Self::Instagram => PlatformProfile {
                name: "instagram-portrait".into(),
                width: 4500,
                height: 6000,
                minimum_delivery_frames: 1,
                maximum_delivery_frames: Some(20),
            },
            Self::Threads => PlatformProfile {
                name: "threads-portrait".into(),
                width: 4500,
                height: 8000,
                minimum_delivery_frames: 1,
                maximum_delivery_frames: None,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PostSpecification {
    pub schema_version: u32,
    pub project: String,
    pub name: String,
    pub platform: PostPlatform,
    pub items: Vec<PostItem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PostItem {
    pub id: String,
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacked_three: Option<StackedThreeParameters>,
    pub placements: Vec<PostPlacement>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StackedThreeParameters {
    pub row_percentages: [u8; 3],
    #[serde(default)]
    pub underfill: StackedUnderfill,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StackedUnderfill {
    #[default]
    Error,
    OuterLetterbox,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PostPlacement {
    pub slot: String,
    pub asset_id: Uuid,
    pub display_filename: String,
    pub fit: String,
    pub focal_point: FocalPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<NormalizedRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<PlacementTransform>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlacementTransform {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<NormalizedRect>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation_quarter_turns_cw: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoringManifest {
    pub schema_version: u32,
    pub session_id: Uuid,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub project_root: PathBuf,
    pub source_specification: PathBuf,
    pub source_specification_sha256: String,
    pub authoring_input_sha256: String,
    pub author_script: PathBuf,
    pub capture_script: PathBuf,
    pub placements: Vec<AuthoringPlacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<SecondaryAuthoring>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecondaryAuthoring {
    pub platform: PostPlatform,
    pub source_specification: PathBuf,
    pub source_specification_sha256: String,
    pub authoring_input_sha256: String,
    pub placements: Vec<AuthoringPlacement>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoringPlacement {
    pub item_id: String,
    pub slot: String,
    pub asset_id: Uuid,
    pub display_filename: String,
    pub template: String,
    pub target_bounds: PixelRect,
    pub source_relative_path: PathBuf,
    pub source_sha256: String,
    pub source_width: u32,
    pub source_height: u32,
    #[serde(default)]
    pub transform: PlacementTransform,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoringReport {
    pub schema_version: u32,
    pub session_id: Uuid,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub source_specification_sha256: String,
    pub authoring_input_sha256: String,
    pub placements: Vec<AuthoringResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoringResult {
    pub item_id: String,
    pub slot: String,
    pub asset_id: Uuid,
    pub source_sha256: String,
    pub document_width: u32,
    pub document_height: u32,
    pub transform: PlacementTransform,
}

#[derive(Serialize)]
struct AuthoringInput<'a> {
    schema_version: u32,
    project: &'a str,
    post: &'a str,
    platform: PostPlatform,
    placements: &'a [AuthoringPlacement],
}

fn is_zero(value: &u8) -> bool {
    *value == 0
}

fn authoring_input_sha256(
    project: &str,
    post: &str,
    platform: PostPlatform,
    placements: &[AuthoringPlacement],
) -> Result<String> {
    Ok(sha256(&canonical_json(&AuthoringInput {
        schema_version: 1,
        project,
        post,
        platform,
        placements,
    })?))
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct FocalPoint {
    pub x: f64,
    pub y: f64,
}

impl Eq for FocalPoint {}

#[derive(Clone, Debug, Serialize)]
pub struct PostWriteReport {
    pub schema_version: u32,
    pub path: PathBuf,
    pub post: PostSpecification,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedPost {
    pub schema_version: u32,
    pub project: String,
    pub name: String,
    pub platform: PostPlatform,
    pub platform_profile: PlatformProfile,
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub ready: bool,
    pub requirements: Vec<String>,
    pub delivery_frame_count: u32,
    pub items: Vec<ResolvedItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlatformProfile {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub minimum_delivery_frames: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_delivery_frames: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedItem {
    pub id: String,
    pub template: ResolvedTemplate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacked_three: Option<StackedThreeParameters>,
    pub placements: Vec<ResolvedPlacement>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedTemplate {
    pub reference: String,
    pub path: PathBuf,
    pub sha256: String,
    pub template: LayoutTemplate,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedPlacement {
    pub slot: String,
    pub asset_id: Uuid,
    pub original_filename: String,
    pub fit: String,
    pub focal_point: FocalPoint,
    pub crop: Option<NormalizedRect>,
    pub rotation_quarter_turns_cw: u8,
    pub layered_psb: ResolvedFile,
    pub hdr_tiff: ResolvedFile,
    pub sdr_tiff: Option<ResolvedFile>,
    pub sdr_state: String,
    pub camera_raw: ResolvedFile,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedFile {
    pub id: Uuid,
    pub representation: String,
    pub logical_location: String,
    pub path: PathBuf,
    pub sha256: String,
    pub byte_size: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutRenderManifest {
    pub schema_version: u32,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub project_root: PathBuf,
    pub photoshop_script: PathBuf,
    pub source_specification: PathBuf,
    pub source_sha256: String,
    pub items: Vec<LayoutRenderItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutRenderItem {
    pub id: String,
    pub template: String,
    pub template_sha256: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub bit_depth: u8,
    pub color_profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_rgb: Option<[u8; 3]>,
    pub placements: Vec<LayoutRenderPlacement>,
    pub output_relative_path: PathBuf,
    pub output_filename: String,
    pub hdr_layer: String,
    pub sdr_layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<ComparisonRenderContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_comparison: Option<EditComparisonRenderContract>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComparisonRenderContract {
    pub reference_relative_path: PathBuf,
    pub reference_sha256: String,
    pub top_left: PixelRect,
    pub top_right: PixelRect,
    pub bottom_left: PixelRect,
    pub bottom_right: PixelRect,
    pub hdr_headroom_ramp: PixelRect,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditComparisonRenderContract {
    pub reference_relative_path: PathBuf,
    pub reference_sha256: String,
    pub top_left: PixelRect,
    pub top_right: PixelRect,
    pub bottom_left: PixelRect,
    pub bottom_right: PixelRect,
    pub text_layers: EditComparisonTextLayers,
    pub text_style: EditComparisonTextStyle,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutRenderPlacement {
    pub slot: String,
    pub bounds: PixelRect,
    pub fit: String,
    pub focal_point: FocalPoint,
    pub rotation_quarter_turns_cw: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_crop: Option<PixelRect>,
    pub hdr_relative_path: PathBuf,
    pub hdr_sha256: String,
    pub sdr_relative_path: PathBuf,
    pub sdr_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_relative_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_metadata: Option<CaptureMetadata>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CaptureMetadata {
    pub make: String,
    pub model: String,
    pub lens: String,
    pub iso: u32,
    pub focal_length_mm: f64,
    pub aperture: f64,
    pub exposure_seconds: f64,
    pub camera_text: String,
    pub capture_text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditSourceManifest {
    pub schema_version: u32,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub project_root: PathBuf,
    pub source_specification: PathBuf,
    pub source_sha256: String,
    pub rendering: String,
    pub reused_items: Vec<EditSourceReportItem>,
    pub items: Vec<EditSourceManifestItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditSourceManifestItem {
    pub item_id: String,
    pub slot: String,
    pub asset_id: Uuid,
    pub original_filename: String,
    pub camera_raw_path: PathBuf,
    pub output_relative_path: PathBuf,
    pub output_filename: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EditSourceReport {
    schema_version: u32,
    project: String,
    post: String,
    platform: PostPlatform,
    source_sha256: String,
    items: Vec<EditSourceReportItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditSourceReportItem {
    pub item_id: String,
    pub slot: String,
    pub asset_id: Uuid,
    pub state: String,
    pub output_relative_path: PathBuf,
    pub output_sha256: String,
    pub output_byte_size: u64,
    pub profile: String,
    pub restored: bool,
    pub metadata: CaptureMetadataInput,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaptureMetadataInput {
    pub make: String,
    pub model: String,
    pub lens: String,
    pub iso: u32,
    pub focal_length_mm: f64,
    pub aperture: f64,
    pub exposure_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EditSourceRegistry {
    schema_version: u32,
    project: String,
    rendering: String,
    items: Vec<EditSourceReportItem>,
}

#[derive(Debug, Deserialize)]
struct LegacyLayoutManifest {
    project: String,
    items: Vec<LegacyLayoutItem>,
}

#[derive(Debug, Deserialize)]
struct LegacyLayoutItem {
    id: String,
    placements: Vec<LegacyLayoutPlacement>,
}

#[derive(Debug, Deserialize)]
struct LegacyLayoutPlacement {
    slot: String,
    hdr_sha256: String,
    before_relative_path: Option<PathBuf>,
    before_sha256: Option<String>,
    capture_metadata: Option<CaptureMetadataInput>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanoramaCropHandoff {
    pub schema_version: u32,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub item_id: String,
    pub project_root: PathBuf,
    pub source_specification: PathBuf,
    pub source_specification_sha256: String,
    pub source_relative_path: PathBuf,
    pub source_filename: String,
    pub source_sha256: String,
    pub author_script: PathBuf,
    pub capture_script: PathBuf,
    pub frame_aspect: String,
    pub frame_count: u32,
    pub crop_aspect_ratio: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanoramaCropReport {
    pub schema_version: u32,
    pub project: String,
    pub post: String,
    pub platform: PostPlatform,
    pub item_id: String,
    pub source_specification_sha256: String,
    pub source_sha256: String,
    pub document_width: u32,
    pub document_height: u32,
    pub crop_pixels: PixelRect,
    pub crop: NormalizedRect,
}

#[derive(Clone, Debug)]
struct MasterBinding {
    asset_id: Uuid,
    original_filename: String,
    layered_psb: ResolvedFile,
    hdr_tiff: ResolvedFile,
    sdr_tiff: Option<ResolvedFile>,
    camera_raw: ResolvedFile,
}

pub fn install_builtin_templates(root: &Path) -> Result<TemplateInstallReport> {
    let mut installed = Vec::new();
    let mut verified = Vec::new();
    for (reference, contents) in [
        ("full-frame@1", FULL_FRAME_V1),
        ("stacked-two@1", STACKED_TWO_V1),
        ("stacked-three@1", STACKED_THREE_V1),
        ("stacked-three@2", STACKED_THREE_V2),
        ("grid-four@1", GRID_FOUR_V1),
        ("grid-four-threads@1", GRID_FOUR_THREADS_V1),
        ("continuous-panorama@1", CONTINUOUS_PANORAMA_V1),
        ("dynamic-range-comparison@1", DYNAMIC_RANGE_COMPARISON_V1),
        ("dynamic-range-comparison@2", DYNAMIC_RANGE_COMPARISON_V2),
        ("dynamic-range-comparison@3", DYNAMIC_RANGE_COMPARISON_V3),
        ("edit-comparison@1", EDIT_COMPARISON_V1),
        ("edit-comparison@2", EDIT_COMPARISON_V2),
    ] {
        install_builtin_template(root, reference, contents, &mut installed, &mut verified)?;
    }
    Ok(TemplateInstallReport {
        schema_version: 1,
        installed,
        verified,
    })
}

fn install_builtin_template(
    root: &Path,
    value: &str,
    contents: &str,
    installed: &mut Vec<PathBuf>,
    verified: &mut Vec<PathBuf>,
) -> Result<()> {
    let reference = TemplateRef::parse(value)?;
    let path = root
        .join(&reference.name)
        .join(format!("v{}.json", reference.version));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            PhotaraError::filesystem("create layout template directory", parent, source)
        })?;
    }
    match fs::read_to_string(&path) {
        Ok(existing) => {
            let actual: LayoutTemplate = parse_json(&path, &existing)?;
            validate_template(&actual, &reference)?;
            let expected: LayoutTemplate = serde_json::from_str(contents)?;
            if canonical_json(&actual)? != canonical_json(&expected)? {
                return Err(PhotaraError::Configuration(format!(
                    "{} differs from Photara's immutable {} template; create a new template version instead of changing it",
                    path.display(),
                    reference.display()
                )));
            }
            verified.push(path);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_atomic(&path, contents.as_bytes())?;
            installed.push(path);
        }
        Err(source) => {
            return Err(PhotaraError::filesystem(
                "read layout template",
                path,
                source,
            ));
        }
    }
    Ok(())
}

pub fn load_template(config: &PhotaraConfig, value: &str) -> Result<ResolvedTemplate> {
    let reference = TemplateRef::parse(value)?;
    let path = config
        .settings
        .templates_root
        .join(&reference.name)
        .join(format!("v{}.json", reference.version));
    let text = fs::read_to_string(&path)
        .map_err(|source| PhotaraError::filesystem("read layout template", &path, source))?;
    let template: LayoutTemplate = parse_json(&path, &text)?;
    validate_template(&template, &reference)?;
    Ok(ResolvedTemplate {
        reference: reference.display(),
        path,
        sha256: sha256(text.as_bytes()),
        template,
    })
}

fn apply_stacked_three_parameters(
    template: &mut ResolvedTemplate,
    parameters: Option<StackedThreeParameters>,
) -> Result<()> {
    let Some(parameters) = parameters else {
        return Ok(());
    };
    parameters.validate()?;
    if template.template.kind != "stacked-three" || !template.template.custom_rows {
        return Err(PhotaraError::Configuration(format!(
            "template {} does not support custom stacked-three rows",
            template.reference
        )));
    }
    for (slot_id, bounds) in ["top", "middle", "bottom"]
        .into_iter()
        .zip(parameters.slots())
    {
        let slot = template
            .template
            .slots
            .iter_mut()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "template {} has no {slot_id:?} slot",
                    template.reference
                ))
            })?;
        slot.bounds = bounds;
    }
    Ok(())
}

pub fn install_template_reference(
    config: &PhotaraConfig,
    value: &str,
    source: &Path,
) -> Result<TemplateReferenceInstallReport> {
    let loaded = load_template(config, value)?;
    let reference = loaded.template.reference.as_ref().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "template {value:?} does not use a reference document"
        ))
    })?;
    let bytes = fs::read(source)
        .map_err(|error| PhotaraError::filesystem("read layout reference", source, error))?;
    let actual_sha256 = sha256(&bytes);
    if actual_sha256 != reference.sha256 {
        return Err(PhotaraError::Configuration(format!(
            "{} has SHA-256 {actual_sha256}, expected {} for immutable template {value}",
            source.display(),
            reference.sha256
        )));
    }
    let destination = loaded
        .path
        .parent()
        .expect("template path has a parent")
        .join(format!("v{}", loaded.template.version))
        .join(&reference.filename);
    let changed = match fs::read(&destination) {
        Ok(existing) => {
            if sha256(&existing) != reference.sha256 {
                return Err(PhotaraError::Configuration(format!(
                    "{} differs from the immutable reference for {value}; create a new template version instead",
                    destination.display()
                )));
            }
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_atomic(&destination, &bytes)?;
            true
        }
        Err(error) => {
            return Err(PhotaraError::filesystem(
                "read installed layout reference",
                &destination,
                error,
            ));
        }
    };
    Ok(TemplateReferenceInstallReport {
        schema_version: 1,
        template: value.into(),
        source: source.into(),
        installed_path: destination,
        sha256: actual_sha256,
        changed,
    })
}

pub fn initialize_post(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    name: &str,
    platform: PostPlatform,
) -> Result<PostWriteReport> {
    validate_post_identity(project, name)?;
    let path = post_path(config, &project.slug, name, platform)?;
    let post = PostSpecification {
        schema_version: 1,
        project: project.slug.clone(),
        name: name.into(),
        platform,
        items: Vec::new(),
    };
    let changed = write_or_verify_json(&path, &post)?;
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_full_frame(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    asset_reference: &str,
    fit: &str,
    template: Option<String>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    if asset_reference.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "asset reference must not be empty".into(),
        ));
    }
    validate_requested_fit(fit)?;
    if let Some(value) = &template {
        let loaded = load_template(config, value)?;
        if loaded.template.kind != "full-frame" {
            return Err(PhotaraError::Configuration(format!(
                "template {value:?} is not a full-frame template"
            )));
        }
    }
    let binding = find_master(database, config, project, asset_reference).await?;
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let item = PostItem {
        id: item_id.into(),
        template,
        stacked_three: None,
        placements: vec![PostPlacement {
            slot: "image".into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename,
            fit: fit.into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop: None,
            transform: None,
        }],
    };
    let changed = match post.items.iter().find(|existing| existing.id == item.id) {
        Some(existing) if existing == &item => false,
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_stacked_two(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    bottom_reference: &str,
    top_crop_from_item: Option<&str>,
    bottom_crop_from_item: Option<&str>,
    template: Option<String>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    if top_reference.trim().is_empty() || bottom_reference.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "stacked layout asset references must not be empty".into(),
        ));
    }
    let template_reference =
        template.unwrap_or_else(|| config.settings.layouts.defaults.stacked_two.clone());
    let loaded = load_template(config, &template_reference)?;
    if loaded.template.kind != "stacked-two" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not a stacked-two template"
        )));
    }
    let top = find_master(database, config, project, top_reference).await?;
    let bottom = find_master(database, config, project, bottom_reference).await?;
    if top.asset_id == bottom.asset_id {
        return Err(PhotaraError::Configuration(
            "stacked-two requires two different assets".into(),
        ));
    }
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let placement = |slot: &str,
                     binding: &MasterBinding,
                     crop_from_item: Option<&str>|
     -> Result<PostPlacement> {
        let crop = crop_from_item
            .map(|source_item_id| reuse_item_crop(&post, source_item_id, binding.asset_id, slot))
            .transpose()?;
        Ok(PostPlacement {
            slot: slot.into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename.clone(),
            fit: if crop.is_some() { "crop" } else { "fill" }.into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop,
            transform: None,
        })
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three: None,
        placements: vec![
            placement("top", &top, top_crop_from_item)?,
            placement("bottom", &bottom, bottom_crop_from_item)?,
        ],
    };
    let changed = match post.items.iter().find(|existing| existing.id == item.id) {
        Some(existing) if existing == &item => false,
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_stacked_three(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    middle_reference: &str,
    bottom_reference: &str,
    template: Option<String>,
    stacked_three: Option<StackedThreeParameters>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    if let Some(parameters) = stacked_three {
        parameters.validate()?;
    }
    let template_reference =
        template.unwrap_or_else(|| config.settings.layouts.defaults.stacked_three.clone());
    let loaded = load_template(config, &template_reference)?;
    if loaded.template.kind != "stacked-three" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not a stacked-three template"
        )));
    }
    if stacked_three.is_some() && !loaded.template.custom_rows {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} does not support custom row percentages; use stacked-three@2"
        )));
    }
    let bindings = [
        find_master(database, config, project, top_reference).await?,
        find_master(database, config, project, middle_reference).await?,
        find_master(database, config, project, bottom_reference).await?,
    ];
    let unique_assets: BTreeSet<_> = bindings.iter().map(|binding| binding.asset_id).collect();
    if unique_assets.len() != 3 {
        return Err(PhotaraError::Configuration(
            "stacked-three requires three different assets".into(),
        ));
    }
    let placement = |slot: &str, binding: &MasterBinding| PostPlacement {
        slot: slot.into(),
        asset_id: binding.asset_id,
        display_filename: binding.original_filename.clone(),
        fit: "crop".into(),
        focal_point: FocalPoint { x: 0.5, y: 0.5 },
        crop: None,
        transform: None,
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three,
        placements: vec![
            placement("top", &bindings[0]),
            placement("middle", &bindings[1]),
            placement("bottom", &bindings[2]),
        ],
    };
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let changed = match post.items.iter().find(|existing| existing.id == item.id) {
        Some(existing) if existing == &item => false,
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: post.schema_version,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_grid_four(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_left_reference: &str,
    top_right_reference: &str,
    bottom_left_reference: &str,
    bottom_right_reference: &str,
    fit: &str,
    template: Option<String>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = template.unwrap_or_else(|| match platform {
        PostPlatform::Instagram => "grid-four@1".into(),
        PostPlatform::Threads => "grid-four-threads@1".into(),
    });
    let loaded = load_template(config, &template_reference)?;
    if loaded.template.kind != "grid-four" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not a four-image grid template"
        )));
    }
    validate_requested_fit(fit)?;
    let bindings = [
        find_master(database, config, project, top_left_reference).await?,
        find_master(database, config, project, top_right_reference).await?,
        find_master(database, config, project, bottom_left_reference).await?,
        find_master(database, config, project, bottom_right_reference).await?,
    ];
    let unique_assets: BTreeSet<_> = bindings.iter().map(|binding| binding.asset_id).collect();
    if unique_assets.len() != 4 {
        return Err(PhotaraError::Configuration(
            "grid-four requires four different assets".into(),
        ));
    }
    let slots = ["top-left", "top-right", "bottom-left", "bottom-right"];
    let mut placements = Vec::with_capacity(4);
    for (slot, binding) in slots.into_iter().zip(&bindings) {
        placements.push(PostPlacement {
            slot: slot.into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename.clone(),
            fit: fit.into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop: None,
            transform: None,
        });
    }
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three: None,
        placements,
    };
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let changed = match post.items.iter().find(|existing| existing.id == item.id) {
        Some(existing) if existing == &item => false,
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: post.schema_version,
        path,
        post,
        changed,
    })
}

fn validate_requested_fit(fit: &str) -> Result<()> {
    if matches!(fit, "fill" | "contain" | "crop") {
        Ok(())
    } else {
        Err(PhotaraError::Configuration(format!(
            "unsupported placement fit {fit:?}; use fill, contain, or crop"
        )))
    }
}

fn reuse_item_crop(
    post: &PostSpecification,
    source_item_id: &str,
    asset_id: Uuid,
    target_slot: &str,
) -> Result<NormalizedRect> {
    let source_item = post
        .items
        .iter()
        .find(|item| item.id == source_item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "cannot reuse crop for {target_slot:?}: source item {source_item_id:?} was not found"
            ))
        })?;
    let mut matches = source_item
        .placements
        .iter()
        .filter(|placement| placement.asset_id == asset_id);
    let placement = matches.next().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "cannot reuse crop for {target_slot:?}: source item {source_item_id:?} does not place the same asset"
        ))
    })?;
    if matches.next().is_some() {
        return Err(PhotaraError::Configuration(format!(
            "cannot reuse crop for {target_slot:?}: source item {source_item_id:?} places the asset more than once"
        )));
    }
    placement_transform(post.schema_version, placement)?
        .crop
        .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "cannot reuse crop for {target_slot:?}: source item {source_item_id:?} has no authored crop"
        ))
    })
}

pub async fn add_continuous_panorama(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    asset_reference: &str,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = config.settings.layouts.defaults.continuous_panorama.clone();
    let template = load_template(config, &template_reference)?;
    if template.template.kind != "continuous-panorama" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not a continuous panorama template"
        )));
    }
    let binding = find_master(database, config, project, asset_reference).await?;
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three: None,
        placements: vec![PostPlacement {
            slot: "image".into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename,
            fit: "crop".into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop: None,
            transform: None,
        }],
    };
    let changed = match post.items.iter().find(|existing| existing.id == item.id) {
        Some(existing) if existing == &item => false,
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_dynamic_range_comparison(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    bottom_reference: &str,
    template_reference: Option<&str>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = template_reference
        .unwrap_or(&config.settings.layouts.defaults.dynamic_range_comparison)
        .to_string();
    let loaded = load_template(config, &template_reference)?;
    if loaded.template.kind != "dynamic-range-comparison" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not a dynamic range comparison template"
        )));
    }
    let top = find_master(database, config, project, top_reference).await?;
    let bottom = find_master(database, config, project, bottom_reference).await?;
    if top.asset_id == bottom.asset_id {
        return Err(PhotaraError::Configuration(
            "dynamic range comparison requires two different assets".into(),
        ));
    }
    let placement = |slot: &str, binding: &MasterBinding| PostPlacement {
        slot: slot.into(),
        asset_id: binding.asset_id,
        display_filename: binding.original_filename.clone(),
        fit: "contain".into(),
        focal_point: FocalPoint { x: 0.5, y: 0.5 },
        crop: None,
        transform: None,
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three: None,
        placements: vec![placement("top", &top), placement("bottom", &bottom)],
    };
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let changed = match post
        .items
        .iter()
        .position(|existing| existing.id == item.id)
    {
        Some(index) if post.items[index] == item => false,
        Some(index)
            if post.items[index]
                .template
                .as_deref()
                .and_then(|value| TemplateRef::parse(value).ok())
                .is_some_and(|reference| reference.name == "dynamic-range-comparison")
                && item
                    .template
                    .as_deref()
                    .and_then(|value| TemplateRef::parse(value).ok())
                    .is_some_and(|reference| reference.name == "dynamic-range-comparison")
                && post.items[index].placements.len() == item.placements.len()
                && post.items[index]
                    .placements
                    .iter()
                    .zip(&item.placements)
                    .all(|(existing, replacement)| {
                        existing.slot == replacement.slot
                            && existing.asset_id == replacement.asset_id
                    }) =>
        {
            post.items[index] = item;
            write_json_atomic(&path, &post)?;
            true
        }
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {item_id:?} already exists with different contents"
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_edit_comparison(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    bottom_reference: &str,
    template_reference: Option<&str>,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = template_reference
        .unwrap_or(&config.settings.layouts.defaults.edit_comparison)
        .to_string();
    let loaded = load_template(config, &template_reference)?;
    if loaded.template.kind != "edit-comparison" {
        return Err(PhotaraError::Configuration(format!(
            "template {template_reference:?} is not an edit comparison template"
        )));
    }
    let top = find_master(database, config, project, top_reference).await?;
    let bottom = find_master(database, config, project, bottom_reference).await?;
    if top.asset_id == bottom.asset_id {
        return Err(PhotaraError::Configuration(
            "edit comparison requires two different assets".into(),
        ));
    }
    let placement = |slot: &str, binding: &MasterBinding| PostPlacement {
        slot: slot.into(),
        asset_id: binding.asset_id,
        display_filename: binding.original_filename.clone(),
        fit: "contain".into(),
        focal_point: FocalPoint { x: 0.5, y: 0.5 },
        crop: None,
        transform: None,
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
        stacked_three: None,
        placements: vec![placement("top", &top), placement("bottom", &bottom)],
    };
    upsert_comparison_item(
        config,
        project,
        post_name,
        platform,
        item,
        "edit-comparison",
    )
}

pub async fn prepare_edit_comparison_sources(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: Option<PostPlatform>,
) -> Result<EditSourceManifest> {
    let project_root = config.settings.projects_root.join(&project.slug);
    let platforms = platform.map_or_else(
        || vec![PostPlatform::Instagram, PostPlatform::Threads],
        |platform| vec![platform],
    );
    let mut resolved_posts = Vec::new();
    for candidate in platforms {
        let source = project_root
            .join("posts")
            .join(candidate.as_str())
            .join(format!("{post_name}.json"));
        if source.is_file() {
            resolved_posts
                .push(resolve_post(database, config, project, post_name, candidate).await?);
        }
    }
    let resolved = resolved_posts.first().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "post {post_name:?} has no Instagram or Threads specification"
        ))
    })?;
    let output_root = PathBuf::from("sources")
        .join("edit-comparison")
        .join("before");
    fs::create_dir_all(project_root.join(&output_root)).map_err(|source| {
        PhotaraError::filesystem(
            "create edit comparison source directory",
            project_root.join(&output_root),
            source,
        )
    })?;
    let registry = relocate_edit_source_registry(
        &project_root,
        load_or_recover_edit_source_registry(&project_root, resolved)?,
        &output_root,
    )?;
    let registry_by_asset = registry
        .items
        .into_iter()
        .map(|item| (item.asset_id, item))
        .collect::<BTreeMap<_, _>>();
    let mut items = Vec::new();
    let mut reused_items = Vec::new();
    let mut seen_assets = BTreeSet::new();
    for resolved_post in &resolved_posts {
        for item in &resolved_post.items {
            if item.template.template.kind != "edit-comparison" {
                continue;
            }
            for placement in &item.placements {
                if !seen_assets.insert(placement.asset_id) {
                    continue;
                }
                verify_file_evidence(&placement.camera_raw)?;
                if let Some(source) = registry_by_asset.get(&placement.asset_id) {
                    let mut reused = source.clone();
                    reused.item_id = item.id.clone();
                    reused.slot = placement.slot.clone();
                    reused_items.push(reused);
                    continue;
                }
                let stem = placement
                    .original_filename
                    .rsplit_once('.')
                    .map(|(stem, _)| stem)
                    .unwrap_or(&placement.original_filename);
                let output_filename = format!("{stem}_RESET_ADOBE_COLOR.TIF");
                items.push(EditSourceManifestItem {
                    item_id: item.id.clone(),
                    slot: placement.slot.clone(),
                    asset_id: placement.asset_id,
                    original_filename: placement.original_filename.clone(),
                    camera_raw_path: placement.camera_raw.path.clone(),
                    output_relative_path: output_root.join(&output_filename),
                    output_filename,
                });
            }
        }
    }
    if items.is_empty() && reused_items.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "post {post_name:?} has no edit-comparison items"
        )));
    }
    let manifest = EditSourceManifest {
        schema_version: 1,
        project: project.slug.clone(),
        post: post_name.into(),
        platform: resolved.platform,
        project_root: project_root.clone(),
        source_specification: resolved.source_path.clone(),
        source_sha256: resolved.source_sha256.clone(),
        rendering: "lightroom-reset-adobe-color".into(),
        reused_items,
        items,
    };
    write_json_atomic(&project_root.join(EDIT_SOURCE_HANDOFF_NAME), &manifest)?;
    Ok(manifest)
}

pub fn verify_edit_comparison_sources(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<serde_json::Value> {
    let project_root = config.settings.projects_root.join(&project.slug);
    let path = project_root.join(EDIT_SOURCE_REPORT_NAME);
    let mut report: EditSourceReport = read_json(&path)?;
    if report.schema_version != 1
        || report.project != project.slug
        || report.post != post_name
        || report.platform != platform
    {
        return Err(PhotaraError::Configuration(
            "edit comparison source report identity does not match the requested post".into(),
        ));
    }
    for item in &mut report.items {
        if !item.restored || item.profile != "Adobe Color" {
            return Err(PhotaraError::Configuration(format!(
                "{} {} was not safely rendered with Adobe Color and restored",
                item.item_id, item.slot
            )));
        }
        let output = project_root.join(&item.output_relative_path);
        let metadata = fs::metadata(&output)
            .map_err(|source| PhotaraError::filesystem("inspect neutral TIFF", &output, source))?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(PhotaraError::Configuration(format!(
                "neutral TIFF {} is empty or not a regular file",
                output.display()
            )));
        }
        item.output_sha256 = sha256_file(&output)?;
        item.output_byte_size = metadata.len();
        item.state = "verified".into();
    }
    write_json_atomic(&path, &report)?;
    merge_edit_source_registry(&project_root, &report)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "project": project.slug,
        "post": post_name,
        "platform": platform,
        "verified": report.items.len(),
        "report": path,
    }))
}

fn upsert_comparison_item(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item: PostItem,
    template_name: &str,
) -> Result<PostWriteReport> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let changed = match post
        .items
        .iter()
        .position(|existing| existing.id == item.id)
    {
        Some(index) if post.items[index] == item => false,
        Some(index)
            if post.items[index]
                .template
                .as_deref()
                .and_then(|value| TemplateRef::parse(value).ok())
                .is_some_and(|reference| reference.name == template_name)
                && post.items[index].placements.len() == item.placements.len()
                && post.items[index]
                    .placements
                    .iter()
                    .zip(&item.placements)
                    .all(|(existing, replacement)| {
                        existing.slot == replacement.slot
                            && existing.asset_id == replacement.asset_id
                    }) =>
        {
            post.items[index] = item;
            write_json_atomic(&path, &post)?;
            true
        }
        Some(_) => {
            return Err(PhotaraError::Configuration(format!(
                "post item {:?} already exists with different contents",
                item.id
            )));
        }
        None => {
            post.items.push(item);
            write_json_atomic(&path, &post)?;
            true
        }
    };
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

pub fn set_item_crop(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    crop: NormalizedRect,
) -> Result<PostWriteReport> {
    set_item_transform(
        config,
        project,
        post_name,
        platform,
        item_id,
        None,
        PlacementTransform {
            crop: Some(crop),
            rotation_quarter_turns_cw: 0,
        },
    )
}

pub fn set_item_transform(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    slot: Option<&str>,
    transform: PlacementTransform,
) -> Result<PostWriteReport> {
    validate_placement_transform(transform)?;
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let original_post = post.clone();
    upgrade_post_to_v2(&mut post)?;
    let item = post
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let placement = if let Some(slot) = slot {
        item.placements
            .iter_mut()
            .find(|placement| placement.slot == slot)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "post item {item_id:?} has no placement in slot {slot:?}"
                ))
            })?
    } else if item.placements.len() == 1 {
        &mut item.placements[0]
    } else {
        return Err(PhotaraError::Configuration(format!(
            "post item {item_id:?} has multiple placements; select a slot"
        )));
    };
    placement.crop = None;
    placement.transform = Some(transform);
    let changed = post != original_post;
    if changed {
        write_json_atomic(&path, &post)?;
    }
    Ok(PostWriteReport {
        schema_version: post.schema_version,
        path,
        post,
        changed,
    })
}

pub fn set_item_fit(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    slot: Option<&str>,
    fit: &str,
) -> Result<PostWriteReport> {
    validate_requested_fit(fit)?;
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let original_post = post.clone();
    upgrade_post_to_v2(&mut post)?;
    let item = post
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let placement = if let Some(slot) = slot {
        item.placements
            .iter_mut()
            .find(|placement| placement.slot == slot)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "post item {item_id:?} has no placement in slot {slot:?}"
                ))
            })?
    } else if item.placements.len() == 1 {
        &mut item.placements[0]
    } else {
        return Err(PhotaraError::Configuration(format!(
            "post item {item_id:?} has multiple placements; select a slot"
        )));
    };
    if placement.fit != fit {
        let rotation_quarter_turns_cw = placement
            .transform
            .unwrap_or_default()
            .rotation_quarter_turns_cw;
        placement.fit = fit.into();
        placement.crop = None;
        placement.transform = (rotation_quarter_turns_cw != 0).then_some(PlacementTransform {
            crop: None,
            rotation_quarter_turns_cw,
        });
    }
    validate_post(&post, project, post_name, platform)?;
    let changed = post != original_post;
    if changed {
        write_json_atomic(&path, &post)?;
    }
    Ok(PostWriteReport {
        schema_version: post.schema_version,
        path,
        post,
        changed,
    })
}

fn upgrade_post_to_v2(post: &mut PostSpecification) -> Result<()> {
    let source_schema_version = post.schema_version;
    for item in &mut post.items {
        for placement in &mut item.placements {
            let transform = placement_transform(source_schema_version, placement)?;
            placement.crop = None;
            placement.transform = (transform != PlacementTransform::default()).then_some(transform);
        }
    }
    post.schema_version = 2;
    Ok(())
}

pub async fn prepare_panorama_crop(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
) -> Result<AuthoringManifest> {
    validate_panorama_item(config, project, post_name, platform, item_id)?;
    prepare_placement_authoring(
        database, config, project, post_name, platform, item_id, None,
    )
    .await
}

pub fn apply_panorama_crop(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
) -> Result<PostWriteReport> {
    validate_panorama_item(config, project, post_name, platform, item_id)?;
    let manifest: AuthoringManifest = read_json(
        &config
            .settings
            .projects_root
            .join(&project.slug)
            .join(AUTHORING_MANIFEST_NAME),
    )?;
    if manifest.placements.len() != 1 || manifest.placements[0].item_id != item_id {
        return Err(PhotaraError::Configuration(format!(
            "prepared authoring session is not for panorama item {item_id:?}"
        )));
    }
    apply_placement_authoring(config, project, post_name, platform)
}

fn validate_panorama_item(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
) -> Result<()> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let item = post
        .items
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let reference = item.template.as_deref().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "post item {item_id:?} does not explicitly select a panorama template"
        ))
    })?;
    if item.placements.len() != 1
        || load_template(config, reference)?.template.kind != "continuous-panorama"
    {
        return Err(PhotaraError::Configuration(format!(
            "post item {item_id:?} is not a single-source continuous panorama"
        )));
    }
    Ok(())
}

pub async fn prepare_placement_authoring(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    slot: Option<&str>,
) -> Result<AuthoringManifest> {
    prepare_authoring_session(
        database,
        config,
        project,
        post_name,
        platform,
        Some(item_id),
        slot,
        false,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_authoring_session(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_filter: Option<&str>,
    slot_filter: Option<&str>,
    reauthor: bool,
) -> Result<AuthoringManifest> {
    if slot_filter.is_some() && item_filter.is_none() {
        return Err(PhotaraError::Configuration(
            "an authoring slot filter requires an item filter".into(),
        ));
    }
    if reauthor && item_filter.is_none() {
        return Err(PhotaraError::Configuration(
            "reauthoring requires an explicit item filter".into(),
        ));
    }
    let source = collect_authoring_source(
        database,
        config,
        project,
        post_name,
        platform,
        item_filter,
        slot_filter,
        reauthor,
    )
    .await?;
    let source_specification = source.path;
    let specification_text = source.text;
    let placements = source.placements;
    if placements.is_empty() {
        return Err(PhotaraError::Configuration(match item_filter {
            Some(item_id) => {
                format!("post item {item_id:?} has no placement matching the authoring request")
            }
            None => "post has no unresolved crop placements".into(),
        }));
    }
    let authoring_input_sha256 =
        authoring_input_sha256(&project.slug, post_name, platform, &placements)?;
    let scripts_root = scripts_root(config)?;
    fs::create_dir_all(&scripts_root).map_err(|source| {
        PhotaraError::filesystem("create Photoshop scripts directory", &scripts_root, source)
    })?;
    let author_script = scripts_root.join(AUTHORING_SCRIPT_NAME);
    let capture_script = scripts_root.join(AUTHORING_CAPTURE_SCRIPT_NAME);
    write_atomic(&author_script, AUTHORING_SCRIPT.as_bytes())?;
    write_atomic(&capture_script, AUTHORING_CAPTURE_SCRIPT.as_bytes())?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let manifest = AuthoringManifest {
        schema_version: 1,
        session_id: Uuid::new_v4(),
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        project_root: project_root.clone(),
        source_specification,
        source_specification_sha256: sha256(specification_text.as_bytes()),
        authoring_input_sha256,
        author_script,
        capture_script,
        placements,
        secondary: None,
    };
    write_json_atomic(&project_root.join(AUTHORING_MANIFEST_NAME), &manifest)?;
    Ok(manifest)
}

struct CollectedAuthoringSource {
    path: PathBuf,
    text: String,
    placements: Vec<AuthoringPlacement>,
}

#[allow(clippy::too_many_arguments)]
async fn collect_authoring_source(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_filter: Option<&str>,
    slot_filter: Option<&str>,
    reauthor: bool,
) -> Result<CollectedAuthoringSource> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let text = fs::read_to_string(&path)
        .map_err(|source| PhotaraError::filesystem("read project post", &path, source))?;
    let post: PostSpecification = parse_json(&path, &text)?;
    validate_post(&post, project, post_name, platform)?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let mut placements = Vec::new();
    for item in post
        .items
        .iter()
        .filter(|item| item_filter.is_none_or(|filter| item.id == filter))
    {
        let template_reference = item
            .template
            .clone()
            .unwrap_or_else(|| config.settings.layouts.defaults.full_frame.clone());
        let mut template = load_template(config, &template_reference)?;
        apply_stacked_three_parameters(&mut template, item.stacked_three)?;
        if let Some(reference) = template.template.reference.as_ref() {
            let profile = platform.profile();
            if reference.width != profile.width || reference.height != profile.height {
                return Err(PhotaraError::Configuration(format!(
                    "template {} reference is {}x{}, but {} posts require {}x{}",
                    template.reference,
                    reference.width,
                    reference.height,
                    platform.as_str(),
                    profile.width,
                    profile.height
                )));
            }
        }
        for placement in item
            .placements
            .iter()
            .filter(|placement| slot_filter.is_none_or(|filter| placement.slot == filter))
        {
            let transform = placement_transform(post.schema_version, placement)?;
            if !placement_enters_authoring(&placement.fit, transform, reauthor) {
                continue;
            }
            let binding = find_master_by_id(database, config, project, placement.asset_id).await?;
            verify_file_evidence(&binding.hdr_tiff)?;
            let (source_width, source_height) = inspect_tiff_dimensions(&binding.hdr_tiff.path)?;
            placements.push(AuthoringPlacement {
                item_id: item.id.clone(),
                slot: placement.slot.clone(),
                asset_id: placement.asset_id,
                display_filename: binding.original_filename,
                template: template_reference.clone(),
                target_bounds: authoring_target_bounds(&template.template, placement, platform)?,
                source_relative_path: project_relative_path(&project_root, &binding.hdr_tiff.path)?,
                source_sha256: binding.hdr_tiff.sha256,
                source_width,
                source_height,
                transform,
            });
        }
    }
    Ok(CollectedAuthoringSource {
        path,
        text,
        placements,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_dual_platform_authoring_session(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    secondary_platform: Option<PostPlatform>,
    item_filter: Option<&str>,
    slot_filter: Option<&str>,
    reauthor: bool,
) -> Result<AuthoringManifest> {
    let Some(secondary_platform) = secondary_platform else {
        return prepare_authoring_session(
            database,
            config,
            project,
            post_name,
            platform,
            item_filter,
            slot_filter,
            reauthor,
        )
        .await;
    };
    if secondary_platform == platform {
        return Err(PhotaraError::Configuration(
            "secondary authoring platform must differ from the primary platform".into(),
        ));
    }
    if slot_filter.is_some() && item_filter.is_none() {
        return Err(PhotaraError::Configuration(
            "an authoring slot filter requires an item filter".into(),
        ));
    }
    if reauthor && item_filter.is_none() {
        return Err(PhotaraError::Configuration(
            "reauthoring requires an explicit item filter".into(),
        ));
    }
    let primary = collect_authoring_source(
        database,
        config,
        project,
        post_name,
        platform,
        item_filter,
        slot_filter,
        reauthor,
    )
    .await?;
    let secondary = collect_authoring_source(
        database,
        config,
        project,
        post_name,
        secondary_platform,
        item_filter,
        slot_filter,
        reauthor,
    )
    .await?;
    if primary.placements.is_empty() && secondary.placements.is_empty() {
        return Err(PhotaraError::Configuration(match item_filter {
            Some(item_id) => {
                format!("post item {item_id:?} has no unresolved placement on either platform")
            }
            None => "neither platform has unresolved crop placements".into(),
        }));
    }
    let primary_input_sha256 =
        authoring_input_sha256(&project.slug, post_name, platform, &primary.placements)?;
    let secondary_input_sha256 = authoring_input_sha256(
        &project.slug,
        post_name,
        secondary_platform,
        &secondary.placements,
    )?;
    let scripts_root = scripts_root(config)?;
    fs::create_dir_all(&scripts_root).map_err(|source| {
        PhotaraError::filesystem("create Photoshop scripts directory", &scripts_root, source)
    })?;
    let author_script = scripts_root.join(AUTHORING_SCRIPT_NAME);
    let capture_script = scripts_root.join(AUTHORING_CAPTURE_SCRIPT_NAME);
    write_atomic(&author_script, AUTHORING_SCRIPT.as_bytes())?;
    write_atomic(&capture_script, AUTHORING_CAPTURE_SCRIPT.as_bytes())?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let manifest = AuthoringManifest {
        schema_version: 2,
        session_id: Uuid::new_v4(),
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        project_root: project_root.clone(),
        source_specification: primary.path,
        source_specification_sha256: sha256(primary.text.as_bytes()),
        authoring_input_sha256: primary_input_sha256,
        author_script,
        capture_script,
        placements: primary.placements,
        secondary: Some(SecondaryAuthoring {
            platform: secondary_platform,
            source_specification: secondary.path,
            source_specification_sha256: sha256(secondary.text.as_bytes()),
            authoring_input_sha256: secondary_input_sha256,
            placements: secondary.placements,
        }),
    };
    write_json_atomic(&project_root.join(AUTHORING_MANIFEST_NAME), &manifest)?;
    Ok(manifest)
}

pub fn apply_placement_authoring(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<PostWriteReport> {
    let project_root = config.settings.projects_root.join(&project.slug);
    let manifest: AuthoringManifest = read_json(&project_root.join(AUTHORING_MANIFEST_NAME))?;
    let report: AuthoringReport = read_json(&project_root.join(AUTHORING_REPORT_NAME))?;
    if manifest.schema_version != 1
        || report.schema_version != 1
        || manifest.session_id != report.session_id
        || manifest.project != project.slug
        || manifest.post != post_name
        || manifest.platform != platform
        || report.project != manifest.project
        || report.post != manifest.post
        || report.platform != manifest.platform
        || report.source_specification_sha256 != manifest.source_specification_sha256
        || report.authoring_input_sha256 != manifest.authoring_input_sha256
        || manifest.placements.is_empty()
        || report.placements.len() != manifest.placements.len()
    {
        return Err(PhotaraError::Configuration(
            "authoring report does not match the prepared session".into(),
        ));
    }
    let expected_input = authoring_input_sha256(
        &manifest.project,
        &manifest.post,
        manifest.platform,
        &manifest.placements,
    )?;
    if expected_input != manifest.authoring_input_sha256 {
        return Err(PhotaraError::Configuration(
            "authoring manifest input fingerprint is invalid".into(),
        ));
    }
    for (expected, actual) in manifest.placements.iter().zip(&report.placements) {
        if actual.item_id != expected.item_id
            || actual.slot != expected.slot
            || actual.asset_id != expected.asset_id
            || actual.source_sha256 != expected.source_sha256
        {
            return Err(PhotaraError::Configuration(format!(
                "authoring report placement identity does not match {}/{}",
                expected.item_id, expected.slot
            )));
        }
        validate_placement_transform(actual.transform)?;
        let source_path = project_root.join(&expected.source_relative_path);
        if sha256_file(&source_path)? != expected.source_sha256 {
            return Err(PhotaraError::Configuration(format!(
                "authoring source changed for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let current_dimensions = inspect_tiff_dimensions(&source_path)?;
        if current_dimensions != (expected.source_width, expected.source_height) {
            return Err(PhotaraError::Configuration(format!(
                "authoring source dimensions changed for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let (rotated_dimensions, crop) = resolve_placement_transform(
            actual.transform,
            expected.source_width,
            expected.source_height,
        )?;
        if (actual.document_width, actual.document_height) != rotated_dimensions {
            return Err(PhotaraError::Configuration(format!(
                "authoring report dimensions do not match rotation for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let crop = crop.ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "authoring report has no crop for {}/{}",
                expected.item_id, expected.slot
            ))
        })?;
        let actual_ratio = f64::from(crop.width) / f64::from(crop.height);
        let target_ratio =
            f64::from(expected.target_bounds.width) / f64::from(expected.target_bounds.height);
        if (actual_ratio - target_ratio).abs() > 0.002 {
            return Err(PhotaraError::Configuration(format!(
                "authored crop ratio {actual_ratio:.6}:1 does not match target {target_ratio:.6}:1 for {}/{}",
                expected.item_id, expected.slot
            )));
        }
    }
    let specification_path = post_path(config, &project.slug, post_name, platform)?;
    let specification = fs::read(&specification_path).map_err(|source| {
        PhotaraError::filesystem("read project post", &specification_path, source)
    })?;
    let mut current_post: PostSpecification = parse_json(
        &specification_path,
        std::str::from_utf8(&specification)
            .map_err(|_| PhotaraError::Configuration("project post is not valid UTF-8".into()))?,
    )?;
    validate_post(&current_post, project, post_name, platform)?;
    let already_applied = report.placements.iter().all(|actual| {
        current_placement_transform(&current_post, &actual.item_id, &actual.slot)
            .is_ok_and(|transform| transform == actual.transform)
    });
    if already_applied {
        return Ok(PostWriteReport {
            schema_version: current_post.schema_version,
            path: specification_path,
            post: current_post,
            changed: false,
        });
    }
    if sha256(&specification) != manifest.source_specification_sha256 {
        return Err(PhotaraError::Configuration(
            "project post changed after authoring began; prepare a new session".into(),
        ));
    }
    let original_post = current_post.clone();
    upgrade_post_to_v2(&mut current_post)?;
    for actual in &report.placements {
        set_post_placement_transform(
            &mut current_post,
            &actual.item_id,
            &actual.slot,
            actual.transform,
        )?;
    }
    let changed = current_post != original_post;
    if changed {
        write_json_atomic(&specification_path, &current_post)?;
    }
    Ok(PostWriteReport {
        schema_version: current_post.schema_version,
        path: specification_path,
        post: current_post,
        changed,
    })
}

pub fn apply_dual_platform_authoring(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<PostWriteReport> {
    let project_root = config.settings.projects_root.join(&project.slug);
    let manifest: AuthoringManifest = read_json(&project_root.join(AUTHORING_MANIFEST_NAME))?;
    let Some(secondary) = manifest.secondary.clone() else {
        return apply_placement_authoring(config, project, post_name, platform);
    };
    let report: AuthoringReport = read_json(&project_root.join(AUTHORING_REPORT_NAME))?;
    if manifest.schema_version != 2
        || report.schema_version != 1
        || manifest.session_id != report.session_id
        || manifest.project != project.slug
        || manifest.post != post_name
        || manifest.platform != platform
        || secondary.platform == platform
        || report.project != manifest.project
        || report.post != manifest.post
        || report.platform != manifest.platform
        || report.source_specification_sha256 != manifest.source_specification_sha256
        || report.authoring_input_sha256 != manifest.authoring_input_sha256
        || report.placements.len() != manifest.placements.len() + secondary.placements.len()
    {
        return Err(PhotaraError::Configuration(
            "dual-platform authoring report does not match the prepared session".into(),
        ));
    }
    if authoring_input_sha256(
        &manifest.project,
        &manifest.post,
        manifest.platform,
        &manifest.placements,
    )? != manifest.authoring_input_sha256
        || authoring_input_sha256(
            &manifest.project,
            &manifest.post,
            secondary.platform,
            &secondary.placements,
        )? != secondary.authoring_input_sha256
    {
        return Err(PhotaraError::Configuration(
            "dual-platform authoring manifest fingerprint is invalid".into(),
        ));
    }
    let expected_secondary_path = post_path(config, &project.slug, post_name, secondary.platform)?;
    if secondary.source_specification != expected_secondary_path {
        return Err(PhotaraError::Configuration(
            "secondary authoring specification path is invalid".into(),
        ));
    }
    let (primary_results, secondary_results) =
        report.placements.split_at(manifest.placements.len());
    validate_authoring_result_group(&project_root, &manifest.placements, primary_results)?;
    validate_authoring_result_group(&project_root, &secondary.placements, secondary_results)?;
    validate_authoring_post_state(
        config,
        project,
        post_name,
        platform,
        &manifest.source_specification_sha256,
        primary_results,
    )?;
    validate_authoring_post_state(
        config,
        project,
        post_name,
        secondary.platform,
        &secondary.source_specification_sha256,
        secondary_results,
    )?;

    let (primary_path, primary_post, primary_changed) = apply_authoring_result_group(
        config,
        project,
        post_name,
        platform,
        &manifest.source_specification_sha256,
        primary_results,
    )?;
    let (_, _, secondary_changed) = apply_authoring_result_group(
        config,
        project,
        post_name,
        secondary.platform,
        &secondary.source_specification_sha256,
        secondary_results,
    )?;
    Ok(PostWriteReport {
        schema_version: primary_post.schema_version,
        path: primary_path,
        post: primary_post,
        changed: primary_changed || secondary_changed,
    })
}

fn validate_authoring_post_state(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    expected_sha256: &str,
    results: &[AuthoringResult],
) -> Result<()> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let bytes = fs::read(&path)
        .map_err(|source| PhotaraError::filesystem("read project post", &path, source))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| PhotaraError::Configuration("project post is not valid UTF-8".into()))?;
    let post: PostSpecification = parse_json(&path, text)?;
    validate_post(&post, project, post_name, platform)?;
    let already_applied = results.iter().all(|actual| {
        current_placement_transform(&post, &actual.item_id, &actual.slot)
            .is_ok_and(|transform| transform == actual.transform)
    });
    if !already_applied && sha256(&bytes) != expected_sha256 {
        return Err(PhotaraError::Configuration(format!(
            "{} project post changed after authoring began; prepare a new session",
            platform.as_str()
        )));
    }
    Ok(())
}

fn validate_authoring_result_group(
    project_root: &Path,
    expected: &[AuthoringPlacement],
    actual: &[AuthoringResult],
) -> Result<()> {
    if expected.len() != actual.len() {
        return Err(PhotaraError::Configuration(
            "authoring result group has an unexpected placement count".into(),
        ));
    }
    for (expected, actual) in expected.iter().zip(actual) {
        if actual.item_id != expected.item_id
            || actual.slot != expected.slot
            || actual.asset_id != expected.asset_id
            || actual.source_sha256 != expected.source_sha256
        {
            return Err(PhotaraError::Configuration(format!(
                "authoring report placement identity does not match {}/{}",
                expected.item_id, expected.slot
            )));
        }
        validate_placement_transform(actual.transform)?;
        let source_path = project_root.join(&expected.source_relative_path);
        if sha256_file(&source_path)? != expected.source_sha256 {
            return Err(PhotaraError::Configuration(format!(
                "authoring source changed for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let current_dimensions = inspect_tiff_dimensions(&source_path)?;
        if current_dimensions != (expected.source_width, expected.source_height) {
            return Err(PhotaraError::Configuration(format!(
                "authoring source dimensions changed for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let (rotated_dimensions, crop) = resolve_placement_transform(
            actual.transform,
            expected.source_width,
            expected.source_height,
        )?;
        if (actual.document_width, actual.document_height) != rotated_dimensions {
            return Err(PhotaraError::Configuration(format!(
                "authoring report dimensions do not match rotation for {}/{}",
                expected.item_id, expected.slot
            )));
        }
        let crop = crop.ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "authoring report has no crop for {}/{}",
                expected.item_id, expected.slot
            ))
        })?;
        let actual_ratio = f64::from(crop.width) / f64::from(crop.height);
        let target_ratio =
            f64::from(expected.target_bounds.width) / f64::from(expected.target_bounds.height);
        if (actual_ratio - target_ratio).abs() > 0.002 {
            return Err(PhotaraError::Configuration(format!(
                "authored crop ratio {actual_ratio:.6}:1 does not match target {target_ratio:.6}:1 for {}/{}",
                expected.item_id, expected.slot
            )));
        }
    }
    Ok(())
}

fn apply_authoring_result_group(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    expected_sha256: &str,
    results: &[AuthoringResult],
) -> Result<(PathBuf, PostSpecification, bool)> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let bytes = fs::read(&path)
        .map_err(|source| PhotaraError::filesystem("read project post", &path, source))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| PhotaraError::Configuration("project post is not valid UTF-8".into()))?;
    let mut post: PostSpecification = parse_json(&path, text)?;
    validate_post(&post, project, post_name, platform)?;
    let already_applied = results.iter().all(|actual| {
        current_placement_transform(&post, &actual.item_id, &actual.slot)
            .is_ok_and(|transform| transform == actual.transform)
    });
    if already_applied {
        return Ok((path, post, false));
    }
    if sha256(&bytes) != expected_sha256 {
        return Err(PhotaraError::Configuration(format!(
            "{} project post changed after authoring began; prepare a new session",
            platform.as_str()
        )));
    }
    let original = post.clone();
    upgrade_post_to_v2(&mut post)?;
    for actual in results {
        set_post_placement_transform(&mut post, &actual.item_id, &actual.slot, actual.transform)?;
    }
    let changed = post != original;
    if changed {
        write_json_atomic(&path, &post)?;
    }
    Ok((path, post, changed))
}

fn set_post_placement_transform(
    post: &mut PostSpecification,
    item_id: &str,
    slot: &str,
    transform: PlacementTransform,
) -> Result<()> {
    let item = post
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let placement = item
        .placements
        .iter_mut()
        .find(|placement| placement.slot == slot)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "post item {item_id:?} has no placement in slot {slot:?}"
            ))
        })?;
    placement.crop = None;
    placement.transform = Some(transform);
    Ok(())
}

fn select_placement<'a>(item: &'a PostItem, slot: Option<&str>) -> Result<&'a PostPlacement> {
    if let Some(slot) = slot {
        return item
            .placements
            .iter()
            .find(|placement| placement.slot == slot)
            .ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "post item {:?} has no placement in slot {slot:?}",
                    item.id
                ))
            });
    }
    match item.placements.as_slice() {
        [placement] => Ok(placement),
        _ => Err(PhotaraError::Configuration(format!(
            "post item {:?} has multiple placements; select a slot",
            item.id
        ))),
    }
}

fn current_placement_transform(
    post: &PostSpecification,
    item_id: &str,
    slot: &str,
) -> Result<PlacementTransform> {
    let item = post
        .items
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let placement = select_placement(item, Some(slot))?;
    placement_transform(post.schema_version, placement)
}

fn authoring_target_bounds(
    template: &LayoutTemplate,
    placement: &PostPlacement,
    platform: PostPlatform,
) -> Result<PixelRect> {
    if template.kind == "continuous-panorama" {
        let surface = template.surface.as_ref().ok_or_else(|| {
            PhotaraError::Configuration("continuous panorama has no surface contract".into())
        })?;
        let (width, height) = surface
            .frame_aspect
            .split_once(':')
            .ok_or_else(|| PhotaraError::Configuration("invalid frame aspect".into()))?;
        let width = width
            .parse::<u32>()
            .map_err(|_| PhotaraError::Configuration("invalid frame aspect width".into()))?;
        let height = height
            .parse::<u32>()
            .map_err(|_| PhotaraError::Configuration("invalid frame aspect height".into()))?;
        return Ok(PixelRect {
            x: 0,
            y: 0,
            width: width.checked_mul(surface.frame_count).ok_or_else(|| {
                PhotaraError::Configuration("continuous surface aspect overflowed".into())
            })?,
            height,
        });
    }
    let profile = platform.profile();
    if template.kind == "dynamic-range-comparison" {
        let cells = &template
            .comparison
            .as_ref()
            .ok_or_else(|| {
                PhotaraError::Configuration("comparison template has no cell contract".into())
            })?
            .cells;
        let bounds = match placement.slot.as_str() {
            "top" => cells.top_left,
            "bottom" => cells.bottom_left,
            _ => {
                return Err(PhotaraError::Configuration(
                    "comparison placement has an unsupported slot".into(),
                ));
            }
        };
        return resolve_bounds(bounds, profile.width, profile.height);
    }
    if template.kind == "edit-comparison" {
        let cells = &template
            .edit_comparison
            .as_ref()
            .ok_or_else(|| {
                PhotaraError::Configuration("edit comparison template has no cell contract".into())
            })?
            .cells;
        let bounds = match placement.slot.as_str() {
            "top" => cells.top_left,
            "bottom" => cells.bottom_left,
            _ => {
                return Err(PhotaraError::Configuration(
                    "edit comparison placement has an unsupported slot".into(),
                ));
            }
        };
        return resolve_bounds(bounds, profile.width, profile.height);
    }
    let slot = template
        .slots
        .iter()
        .find(|slot| slot.id == placement.slot)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "template {} does not define slot {:?}",
                template.name, placement.slot
            ))
        })?;
    resolve_bounds(slot.bounds, profile.width, profile.height)
}

pub fn show_post(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<PostWriteReport> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed: false,
    })
}

pub fn reorder_post(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_ids: &[String],
) -> Result<PostWriteReport> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let changed = reorder_items(&mut post, item_ids)?;
    if changed {
        write_json_atomic(&path, &post)?;
    }
    Ok(PostWriteReport {
        schema_version: 1,
        path,
        post,
        changed,
    })
}

fn reorder_items(post: &mut PostSpecification, item_ids: &[String]) -> Result<bool> {
    for item_id in item_ids {
        validate_slug(item_id).map_err(|message| {
            PhotaraError::Configuration(format!(
                "invalid reordered post item ID {item_id:?}: {message}"
            ))
        })?;
    }
    let existing_ids: BTreeSet<_> = post.items.iter().map(|item| item.id.clone()).collect();
    let requested_ids: BTreeSet<_> = item_ids.iter().cloned().collect();
    if item_ids.len() != requested_ids.len() {
        return Err(PhotaraError::Configuration(
            "reordered post item IDs must not contain duplicates".into(),
        ));
    }
    if existing_ids != requested_ids {
        let missing: Vec<_> = existing_ids.difference(&requested_ids).cloned().collect();
        let unknown: Vec<_> = requested_ids.difference(&existing_ids).cloned().collect();
        return Err(PhotaraError::Configuration(format!(
            "reordered post items must be an exact permutation (missing: {}; unknown: {})",
            missing.join(", "),
            unknown.join(", ")
        )));
    }
    let current_ids: Vec<_> = post.items.iter().map(|item| item.id.clone()).collect();
    if current_ids == item_ids {
        return Ok(false);
    }
    let mut items_by_id: BTreeMap<_, _> = post
        .items
        .drain(..)
        .map(|item| (item.id.clone(), item))
        .collect();
    post.items = item_ids
        .iter()
        .map(|item_id| items_by_id.remove(item_id).expect("validated item ID"))
        .collect();
    Ok(true)
}

pub async fn resolve_post(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<ResolvedPost> {
    resolve_post_item(database, config, project, post_name, platform, None).await
}

async fn resolve_post_item(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_filter: Option<&str>,
) -> Result<ResolvedPost> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let text = fs::read_to_string(&path)
        .map_err(|source| PhotaraError::filesystem("read project post", &path, source))?;
    let post: PostSpecification = parse_json(&path, &text)?;
    validate_post(&post, project, post_name, platform)?;
    if post.items.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project post {} has no editorial items",
            path.display()
        )));
    }
    let mut requirements = BTreeSet::new();
    let mut items = Vec::with_capacity(post.items.len());
    let selected_items: Vec<_> = post
        .items
        .into_iter()
        .filter(|item| item_filter.is_none_or(|filter| item.id == filter))
        .collect();
    if let Some(filter) = item_filter
        && selected_items.is_empty()
    {
        return Err(PhotaraError::Configuration(format!(
            "post {post_name:?} has no item {filter:?}"
        )));
    }
    let post_schema_version = post.schema_version;
    for item in selected_items {
        let template_reference = item
            .template
            .clone()
            .unwrap_or_else(|| config.settings.layouts.defaults.full_frame.clone());
        let mut template = load_template(config, &template_reference)?;
        apply_stacked_three_parameters(&mut template, item.stacked_three)?;
        let slot_ids: BTreeSet<_> = template
            .template
            .slots
            .iter()
            .map(|slot| slot.id.as_str())
            .collect();
        let mut placements = Vec::with_capacity(item.placements.len());
        for placement in item.placements {
            let transform = placement_transform(post_schema_version, &placement)?;
            if !slot_ids.contains(placement.slot.as_str()) {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses slot {:?}, which template {} does not define",
                    item.id, placement.slot, template.reference
                )));
            }
            validate_focal_point(placement.focal_point)?;
            let binding = find_master_by_id(database, config, project, placement.asset_id).await?;
            if template.template.kind == "continuous-panorama" && transform.crop.is_none() {
                requirements.insert(format!(
                    "author a 3:2 crop for panorama item {} ({})",
                    item.id, binding.original_filename
                ));
            }
            if placement_requires_authoring(&placement.fit, transform) {
                requirements.insert(format!(
                    "author the placement for item {} slot {} ({})",
                    item.id, placement.slot, binding.original_filename
                ));
            }
            if binding.original_filename != placement.display_filename {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} asset label changed from {:?} to {:?}",
                    item.id, placement.display_filename, binding.original_filename
                )));
            }
            if binding.sdr_tiff.is_none() {
                requirements.insert(format!(
                    "author and flatten the SDR rendition for {}",
                    binding.original_filename
                ));
            }
            let sdr_state = if binding.sdr_tiff.is_some() {
                "verified"
            } else {
                "authoring-required"
            };
            placements.push(ResolvedPlacement {
                slot: placement.slot,
                asset_id: binding.asset_id,
                original_filename: binding.original_filename,
                fit: placement.fit,
                focal_point: placement.focal_point,
                crop: transform.crop,
                rotation_quarter_turns_cw: transform.rotation_quarter_turns_cw,
                layered_psb: binding.layered_psb,
                hdr_tiff: binding.hdr_tiff,
                sdr_tiff: binding.sdr_tiff,
                sdr_state: sdr_state.into(),
                camera_raw: binding.camera_raw,
            });
        }
        items.push(ResolvedItem {
            id: item.id,
            template,
            stacked_three: item.stacked_three,
            placements,
        });
    }
    let delivery_frame_count = delivery_frame_count(&items);
    Ok(ResolvedPost {
        schema_version: 1,
        project: post.project,
        name: post.name,
        platform: post.platform,
        platform_profile: post.platform.profile(),
        source_path: path,
        source_sha256: sha256(text.as_bytes()),
        ready: requirements.is_empty(),
        requirements: requirements.into_iter().collect(),
        delivery_frame_count,
        items,
    })
}

pub async fn prepare_render(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
) -> Result<LayoutRenderManifest> {
    prepare_render_item(database, config, project, post_name, platform, None).await
}

pub async fn prepare_render_item(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_filter: Option<&str>,
) -> Result<LayoutRenderManifest> {
    let resolved =
        resolve_post_item(database, config, project, post_name, platform, item_filter).await?;
    if !resolved.ready {
        return Err(PhotaraError::Configuration(format!(
            "post is not ready to render: {}",
            resolved.requirements.join("; ")
        )));
    }
    if item_filter.is_none() {
        validate_delivery_frame_count(&resolved.platform_profile, resolved.delivery_frame_count)?;
    }
    let project_root = config.settings.projects_root.join(&project.slug);
    let edit_sources = load_edit_sources(&project_root, &resolved)?;
    let mut verified_renditions = BTreeSet::new();
    let render_root = PathBuf::from("posts")
        .join(platform.as_str())
        .join("renders")
        .join(post_name);
    fs::create_dir_all(project_root.join(&render_root)).map_err(|source| {
        PhotaraError::filesystem(
            "create layout render directory",
            project_root.join(&render_root),
            source,
        )
    })?;
    let mut items = Vec::with_capacity(resolved.items.len());
    for item in &resolved.items {
        if !matches!(
            item.template.template.kind.as_str(),
            "full-frame"
                | "stacked-two"
                | "stacked-three"
                | "grid-four"
                | "continuous-panorama"
                | "dynamic-range-comparison"
                | "edit-comparison"
        ) {
            return Err(PhotaraError::Configuration(format!(
                "layout compositor does not support template kind {:?} for item {:?}",
                item.template.template.kind, item.id
            )));
        }
        let panorama = if item.template.template.kind == "continuous-panorama" {
            let placement = item.placements.first().ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "continuous panorama item {:?} has no placement",
                    item.id
                ))
            })?;
            let sdr = placement.sdr_tiff.as_ref().ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "{} has no verified SDR TIFF",
                    placement.original_filename
                ))
            })?;
            let hdr_dimensions = inspect_tiff_dimensions(&placement.hdr_tiff.path)?;
            let sdr_dimensions = inspect_tiff_dimensions(&sdr.path)?;
            if hdr_dimensions != sdr_dimensions {
                return Err(PhotaraError::Configuration(format!(
                    "paired panorama TIFF dimensions differ for {}",
                    placement.original_filename
                )));
            }
            let transform = PlacementTransform {
                crop: placement.crop,
                rotation_quarter_turns_cw: placement.rotation_quarter_turns_cw,
            };
            let (_, crop) =
                resolve_placement_transform(transform, hdr_dimensions.0, hdr_dimensions.1)?;
            let crop = crop.ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "continuous panorama item {:?} has no authored crop",
                    item.id
                ))
            })?;
            let ratio = f64::from(crop.width) / f64::from(crop.height);
            if (ratio - 1.5).abs() > 0.002 {
                return Err(PhotaraError::Configuration(format!(
                    "resolved panorama crop for {:?} is not 3:2 ({ratio:.6}:1)",
                    item.id
                )));
            }
            Some(crop)
        } else {
            None
        };
        let (canvas_width, canvas_height) =
            panorama.map(|crop| (crop.width, crop.height)).unwrap_or((
                resolved.platform_profile.width,
                resolved.platform_profile.height,
            ));
        let comparison = if item.template.template.kind == "dynamic-range-comparison" {
            let contract = item.template.template.comparison.as_ref().ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "template {} has no comparison contract",
                    item.template.reference
                ))
            })?;
            let reference = item.template.template.reference.as_ref().ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "template {} has no reference document",
                    item.template.reference
                ))
            })?;
            let versioned_reference_path = item
                .template
                .path
                .parent()
                .expect("template path has a parent")
                .join(format!("v{}", item.template.template.version))
                .join(&reference.filename);
            let legacy_reference_path = item
                .template
                .path
                .parent()
                .expect("template path has a parent")
                .join(&reference.filename);
            let reference_path = if versioned_reference_path.is_file() {
                versioned_reference_path
            } else {
                legacy_reference_path
            };
            let bytes = fs::read(&reference_path).map_err(|error| {
                PhotaraError::filesystem(
                    "read comparison template reference",
                    &reference_path,
                    error,
                )
            })?;
            if sha256(&bytes) != reference.sha256 {
                return Err(PhotaraError::Configuration(format!(
                    "comparison reference {} does not match immutable template {}",
                    reference_path.display(),
                    item.template.reference
                )));
            }
            let cached_reference_path = config
                .settings
                .templates_cache
                .join(&item.template.template.name)
                .join(format!("v{}", item.template.template.version))
                .join(&reference.filename);
            let cache_matches = fs::read(&cached_reference_path)
                .map(|existing| sha256(&existing) == reference.sha256)
                .unwrap_or(false);
            if !cache_matches {
                write_atomic(&cached_reference_path, &bytes)?;
            }
            let bytes = fs::read(&cached_reference_path).map_err(|error| {
                PhotaraError::filesystem(
                    "read cached comparison template reference",
                    &cached_reference_path,
                    error,
                )
            })?;
            let reference_relative_path = PathBuf::from("posts")
                .join("_templates")
                .join("dynamic-range-comparison")
                .join(format!("v{}", item.template.template.version))
                .join(&reference.filename);
            let materialized_reference = project_root.join(&reference_relative_path);
            let reference_matches = fs::read(&materialized_reference)
                .map(|existing| sha256(&existing) == reference.sha256)
                .unwrap_or(false);
            if !reference_matches {
                if let Some(parent) = materialized_reference.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        PhotaraError::filesystem("create project template cache", parent, error)
                    })?;
                }
                write_atomic(&materialized_reference, &bytes)?;
            }
            Some(ComparisonRenderContract {
                reference_relative_path,
                reference_sha256: reference.sha256.clone(),
                top_left: resolve_bounds(contract.cells.top_left, canvas_width, canvas_height)?,
                top_right: resolve_bounds(contract.cells.top_right, canvas_width, canvas_height)?,
                bottom_left: resolve_bounds(
                    contract.cells.bottom_left,
                    canvas_width,
                    canvas_height,
                )?,
                bottom_right: resolve_bounds(
                    contract.cells.bottom_right,
                    canvas_width,
                    canvas_height,
                )?,
                hdr_headroom_ramp: resolve_bounds(
                    contract.hdr_headroom_ramp,
                    canvas_width,
                    canvas_height,
                )?,
            })
        } else {
            None
        };
        let edit_comparison = if item.template.template.kind == "edit-comparison" {
            let contract = item
                .template
                .template
                .edit_comparison
                .as_ref()
                .ok_or_else(|| {
                    PhotaraError::Configuration(format!(
                        "template {} has no edit comparison contract",
                        item.template.reference
                    ))
                })?;
            let (reference_relative_path, reference_sha256) =
                materialize_reference(config, &project_root, &item.template, "edit-comparison")?;
            Some(EditComparisonRenderContract {
                reference_relative_path,
                reference_sha256,
                top_left: resolve_bounds(contract.cells.top_left, canvas_width, canvas_height)?,
                top_right: resolve_bounds(contract.cells.top_right, canvas_width, canvas_height)?,
                bottom_left: resolve_bounds(
                    contract.cells.bottom_left,
                    canvas_width,
                    canvas_height,
                )?,
                bottom_right: resolve_bounds(
                    contract.cells.bottom_right,
                    canvas_width,
                    canvas_height,
                )?,
                text_layers: contract.text_layers.clone(),
                text_style: contract.text_style.clone(),
            })
        } else {
            None
        };
        let mut render_placements = Vec::with_capacity(item.placements.len());
        for placement in &item.placements {
            let slot = item
                .template
                .template
                .slots
                .iter()
                .find(|slot| slot.id == placement.slot)
                .ok_or_else(|| {
                    PhotaraError::Configuration(format!(
                        "template {} does not define slot {:?}",
                        item.template.reference, placement.slot
                    ))
                })?;
            let sdr = placement.sdr_tiff.as_ref().ok_or_else(|| {
                PhotaraError::Configuration(format!(
                    "{} has no verified SDR TIFF",
                    placement.original_filename
                ))
            })?;
            if verified_renditions.insert(placement.hdr_tiff.id) {
                verify_file_evidence(&placement.hdr_tiff)?;
            }
            if verified_renditions.insert(sdr.id) {
                verify_file_evidence(sdr)?;
            }
            let edit_source = if item.template.template.kind == "edit-comparison" {
                Some(
                    edit_sources
                        .get(&placement.asset_id)
                        .ok_or_else(|| {
                            PhotaraError::Configuration(format!(
                                "prepare neutral Adobe Color source for edit comparison item {:?} slot {:?}",
                                item.id, placement.slot
                            ))
                        })?,
                )
            } else {
                None
            };
            let hdr_dimensions = inspect_tiff_dimensions(&placement.hdr_tiff.path)?;
            let sdr_dimensions = inspect_tiff_dimensions(&sdr.path)?;
            if hdr_dimensions != sdr_dimensions {
                return Err(PhotaraError::Configuration(format!(
                    "paired TIFF dimensions differ for transformed placement {} in item {:?}",
                    placement.original_filename, item.id
                )));
            }
            let (_, source_crop) = resolve_placement_transform(
                PlacementTransform {
                    crop: placement.crop,
                    rotation_quarter_turns_cw: placement.rotation_quarter_turns_cw,
                },
                hdr_dimensions.0,
                hdr_dimensions.1,
            )?;
            render_placements.push(LayoutRenderPlacement {
                slot: placement.slot.clone(),
                bounds: resolve_bounds(slot.bounds, canvas_width, canvas_height)?,
                fit: placement.fit.clone(),
                focal_point: placement.focal_point,
                rotation_quarter_turns_cw: placement.rotation_quarter_turns_cw,
                source_crop,
                hdr_relative_path: project_relative_path(&project_root, &placement.hdr_tiff.path)?,
                hdr_sha256: placement.hdr_tiff.sha256.clone(),
                sdr_relative_path: project_relative_path(&project_root, &sdr.path)?,
                sdr_sha256: sdr.sha256.clone(),
                before_relative_path: edit_source.map(|source| source.output_relative_path.clone()),
                before_sha256: edit_source.map(|source| source.output_sha256.clone()),
                capture_metadata: edit_source.map(|source| capture_metadata(&source.metadata)),
            });
        }
        let output_filename = format!("{}.PSB", item.id);
        items.push(LayoutRenderItem {
            id: item.id.clone(),
            template: item.template.reference.clone(),
            template_sha256: item.template.sha256.clone(),
            canvas_width,
            canvas_height,
            bit_depth: 32,
            color_profile: "Display P3 Linear".into(),
            background_rgb: item
                .stacked_three
                .filter(|parameters| parameters.needs_background())
                .map(|_| [0, 0, 0]),
            placements: render_placements,
            output_relative_path: render_root.join(&output_filename),
            output_filename,
            hdr_layer: item.template.template.wsp.hdr_layer.clone(),
            sdr_layer: item.template.template.wsp.sdr_layer.clone(),
            comparison,
            edit_comparison,
        });
    }
    let scripts_root = scripts_root(config)?;
    fs::create_dir_all(&scripts_root).map_err(|source| {
        PhotaraError::filesystem("create Photoshop scripts directory", &scripts_root, source)
    })?;
    let photoshop_script = scripts_root.join(LAYOUT_SCRIPT_NAME);
    write_atomic(&photoshop_script, LAYOUT_SCRIPT.as_bytes())?;
    let manifest = LayoutRenderManifest {
        schema_version: 1,
        project: resolved.project,
        post: resolved.name,
        platform: resolved.platform,
        project_root: project_root.clone(),
        photoshop_script,
        source_specification: resolved.source_path,
        source_sha256: resolved.source_sha256,
        items,
    };
    write_json_atomic(&project_root.join(LAYOUT_HANDOFF_NAME), &manifest)?;
    Ok(manifest)
}

fn delivery_frame_count(items: &[ResolvedItem]) -> u32 {
    items
        .iter()
        .map(|item| {
            item.template
                .template
                .surface
                .as_ref()
                .map(|surface| surface.frame_count)
                .unwrap_or(1)
        })
        .sum()
}

fn validate_delivery_frame_count(profile: &PlatformProfile, count: u32) -> Result<()> {
    if count < profile.minimum_delivery_frames {
        return Err(PhotaraError::Configuration(format!(
            "{} render manifest must contain at least {} delivery frame; resolved {count}",
            profile.name, profile.minimum_delivery_frames
        )));
    }
    if let Some(maximum) = profile.maximum_delivery_frames
        && count > maximum
    {
        return Err(PhotaraError::Configuration(format!(
            "{} render manifest exceeds the maximum of {maximum} delivery frames; resolved {count}",
            profile.name
        )));
    }
    Ok(())
}

fn resolve_bounds(
    bounds: NormalizedRect,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<PixelRect> {
    let x = (bounds.x * f64::from(canvas_width)).round();
    let y = (bounds.y * f64::from(canvas_height)).round();
    let width = (bounds.width * f64::from(canvas_width)).round();
    let height = (bounds.height * f64::from(canvas_height)).round();
    if x < 0.0
        || y < 0.0
        || width <= 0.0
        || height <= 0.0
        || x + width > f64::from(canvas_width)
        || y + height > f64::from(canvas_height)
    {
        return Err(PhotaraError::Configuration(
            "layout slot resolves outside the platform canvas".into(),
        ));
    }
    Ok(PixelRect {
        x: x as u32,
        y: y as u32,
        width: width as u32,
        height: height as u32,
    })
}

fn resolve_placement_transform(
    transform: PlacementTransform,
    source_width: u32,
    source_height: u32,
) -> Result<((u32, u32), Option<PixelRect>)> {
    validate_placement_transform(transform)?;
    if source_width == 0 || source_height == 0 {
        return Err(PhotaraError::Configuration(
            "placement transform source dimensions must be greater than zero".into(),
        ));
    }
    let rotated_dimensions = if transform.rotation_quarter_turns_cw.is_multiple_of(2) {
        (source_width, source_height)
    } else {
        (source_height, source_width)
    };
    let crop = transform
        .crop
        .map(|crop| resolve_bounds(crop, rotated_dimensions.0, rotated_dimensions.1))
        .transpose()?;
    Ok((rotated_dimensions, crop))
}

fn load_edit_sources(
    project_root: &Path,
    resolved: &ResolvedPost,
) -> Result<BTreeMap<Uuid, EditSourceReportItem>> {
    if !resolved
        .items
        .iter()
        .any(|item| item.template.template.kind == "edit-comparison")
    {
        return Ok(BTreeMap::new());
    }
    let registry = load_or_recover_edit_source_registry(project_root, resolved)?;
    let mut sources = BTreeMap::new();
    for item in registry.items {
        if sources.insert(item.asset_id, item).is_some() {
            return Err(PhotaraError::Configuration(
                "edit comparison source registry contains duplicate assets".into(),
            ));
        }
    }
    Ok(sources)
}

fn validate_edit_source_registry_item(
    project_root: &Path,
    item: &EditSourceReportItem,
) -> Result<()> {
    if item.state != "verified" || !item.restored {
        return Err(PhotaraError::Configuration(format!(
            "neutral source for asset {} was not verified and safely restored",
            item.asset_id
        )));
    }
    if !item.profile.eq_ignore_ascii_case("Adobe Color") {
        return Err(PhotaraError::Configuration(format!(
            "neutral source for asset {} used profile {:?}, not Adobe Color",
            item.asset_id, item.profile
        )));
    }
    let output = project_root.join(&item.output_relative_path);
    let metadata = fs::metadata(&output)
        .map_err(|source| PhotaraError::filesystem("inspect neutral source", &output, source))?;
    if !metadata.is_file()
        || metadata.len() != item.output_byte_size
        || sha256_file(&output)? != item.output_sha256
    {
        return Err(PhotaraError::Configuration(format!(
            "neutral source {} changed after Lightroom verification",
            output.display()
        )));
    }
    Ok(())
}

fn load_or_recover_edit_source_registry(
    project_root: &Path,
    resolved: &ResolvedPost,
) -> Result<EditSourceRegistry> {
    let registry_path = project_root.join(EDIT_SOURCE_REGISTRY_NAME);
    if registry_path.is_file() {
        let registry: EditSourceRegistry = read_json(&registry_path)?;
        if registry.schema_version != 1
            || registry.project != resolved.project
            || registry.rendering != "lightroom-reset-adobe-color"
        {
            return Err(PhotaraError::Configuration(format!(
                "edit comparison source registry {} has an unsupported identity or rendering contract",
                registry_path.display()
            )));
        }
        for item in &registry.items {
            validate_edit_source_registry_item(project_root, item)?;
        }
        return Ok(registry);
    }

    let mut recovered = BTreeMap::<Uuid, EditSourceReportItem>::new();
    let legacy_manifest_path = project_root.join(LAYOUT_MANIFEST_NAME);
    if legacy_manifest_path.is_file() {
        let legacy: LegacyLayoutManifest = read_json(&legacy_manifest_path)?;
        if legacy.project == resolved.project {
            for legacy_item in legacy.items {
                for placement in legacy_item.placements {
                    let (Some(output_relative_path), Some(output_sha256), Some(metadata)) = (
                        placement.before_relative_path,
                        placement.before_sha256,
                        placement.capture_metadata,
                    ) else {
                        continue;
                    };
                    let mut matches = resolved
                        .items
                        .iter()
                        .flat_map(|item| &item.placements)
                        .filter(|candidate| candidate.hdr_tiff.sha256 == placement.hdr_sha256);
                    let Some(candidate) = matches.next() else {
                        continue;
                    };
                    if matches.any(|other| other.asset_id != candidate.asset_id) {
                        continue;
                    }
                    let output = project_root.join(&output_relative_path);
                    let Ok(file_metadata) = fs::metadata(&output) else {
                        continue;
                    };
                    if !file_metadata.is_file()
                        || file_metadata.len() == 0
                        || sha256_file(&output)? != output_sha256
                    {
                        continue;
                    }
                    recovered.insert(
                        candidate.asset_id,
                        EditSourceReportItem {
                            item_id: legacy_item.id.clone(),
                            slot: placement.slot,
                            asset_id: candidate.asset_id,
                            state: "verified".into(),
                            output_relative_path,
                            output_sha256,
                            output_byte_size: file_metadata.len(),
                            profile: "Adobe Color".into(),
                            restored: true,
                            metadata,
                        },
                    );
                }
            }
        }
    }

    if recovered.is_empty() {
        let report_path = project_root.join(EDIT_SOURCE_REPORT_NAME);
        if report_path.is_file() {
            let report: EditSourceReport = read_json(&report_path)?;
            if report.schema_version == 1 && report.project == resolved.project {
                for item in report.items {
                    if validate_edit_source_registry_item(project_root, &item).is_ok() {
                        recovered.insert(item.asset_id, item);
                    }
                }
            }
        }
    }

    let registry = EditSourceRegistry {
        schema_version: 1,
        project: resolved.project.clone(),
        rendering: "lightroom-reset-adobe-color".into(),
        items: recovered.into_values().collect(),
    };
    if !registry.items.is_empty() {
        write_json_atomic(&registry_path, &registry)?;
    }
    Ok(registry)
}

fn relocate_edit_source_registry(
    project_root: &Path,
    mut registry: EditSourceRegistry,
    output_root: &Path,
) -> Result<EditSourceRegistry> {
    let mut changed = false;
    for item in &mut registry.items {
        if item.output_relative_path.starts_with(output_root) {
            continue;
        }
        let filename = item.output_relative_path.file_name().ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "neutral source path {} has no filename",
                item.output_relative_path.display()
            ))
        })?;
        let new_relative_path = output_root.join(filename);
        let old_path = project_root.join(&item.output_relative_path);
        let new_path = project_root.join(&new_relative_path);
        if new_path.is_file() {
            let metadata = fs::metadata(&new_path).map_err(|source| {
                PhotaraError::filesystem("inspect shared neutral source", &new_path, source)
            })?;
            if metadata.len() != item.output_byte_size
                || sha256_file(&new_path)? != item.output_sha256
            {
                return Err(PhotaraError::Configuration(format!(
                    "shared neutral source destination {} contains different content",
                    new_path.display()
                )));
            }
            fs::remove_file(&old_path).map_err(|source| {
                PhotaraError::filesystem(
                    "remove duplicate legacy neutral source",
                    &old_path,
                    source,
                )
            })?;
        } else {
            fs::rename(&old_path, &new_path).map_err(|source| {
                PhotaraError::filesystem(
                    "move neutral source into shared directory",
                    &old_path,
                    source,
                )
            })?;
        }
        item.output_relative_path = new_relative_path;
        changed = true;
    }
    if changed {
        write_json_atomic(&project_root.join(EDIT_SOURCE_REGISTRY_NAME), &registry)?;
    }
    Ok(registry)
}

fn merge_edit_source_registry(project_root: &Path, report: &EditSourceReport) -> Result<()> {
    let path = project_root.join(EDIT_SOURCE_REGISTRY_NAME);
    let mut items = if path.is_file() {
        let existing: EditSourceRegistry = read_json(&path)?;
        if existing.schema_version != 1
            || existing.project != report.project
            || existing.rendering != "lightroom-reset-adobe-color"
        {
            return Err(PhotaraError::Configuration(format!(
                "edit comparison source registry {} has an unsupported identity or rendering contract",
                path.display()
            )));
        }
        existing
            .items
            .into_iter()
            .map(|item| (item.asset_id, item))
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::new()
    };
    for item in &report.items {
        validate_edit_source_registry_item(project_root, item)?;
        items.insert(item.asset_id, item.clone());
    }
    write_json_atomic(
        &path,
        &EditSourceRegistry {
            schema_version: 1,
            project: report.project.clone(),
            rendering: "lightroom-reset-adobe-color".into(),
            items: items.into_values().collect(),
        },
    )
}

fn capture_metadata(input: &CaptureMetadataInput) -> CaptureMetadata {
    let model = friendly_camera_model(&input.make, &input.model);
    let camera_text = format!("{model} · {}", input.lens);
    let shutter = if input.exposure_seconds > 0.0 && input.exposure_seconds < 1.0 {
        format!("1/{}", (1.0 / input.exposure_seconds).round() as u64)
    } else {
        format_number(input.exposure_seconds)
    };
    let capture_text = format!(
        "ISO {} · {}mm · ƒ/{} · {}",
        input.iso,
        format_number(input.focal_length_mm),
        format_number(input.aperture),
        shutter
    );
    CaptureMetadata {
        make: input.make.clone(),
        model: input.model.clone(),
        lens: input.lens.clone(),
        iso: input.iso,
        focal_length_mm: input.focal_length_mm,
        aperture: input.aperture,
        exposure_seconds: input.exposure_seconds,
        camera_text,
        capture_text,
    }
}

fn friendly_camera_model(make: &str, model: &str) -> String {
    match (make.trim().to_ascii_uppercase().as_str(), model.trim()) {
        ("SONY", "ILCE-7RM3") => "Sony α7R III".into(),
        ("SONY", "ILCE-7RM5") => "Sony α7R V".into(),
        (_, model)
            if model
                .to_ascii_lowercase()
                .starts_with(&make.to_ascii_lowercase()) =>
        {
            model.into()
        }
        (_, model) => format!("{} {model}", title_case_ascii(make)),
    }
}

fn title_case_ascii(value: &str) -> String {
    let mut chars = value.trim().chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase())
        .unwrap_or_default()
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 0.000_001 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn materialize_reference(
    config: &PhotaraConfig,
    project_root: &Path,
    template: &ResolvedTemplate,
    directory_name: &str,
) -> Result<(PathBuf, String)> {
    let reference = template.template.reference.as_ref().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "template {} has no reference document",
            template.reference
        ))
    })?;
    let versioned = template
        .path
        .parent()
        .expect("template path has a parent")
        .join(format!("v{}", template.template.version))
        .join(&reference.filename);
    let legacy = template
        .path
        .parent()
        .expect("template path has a parent")
        .join(&reference.filename);
    let source = if versioned.is_file() {
        versioned
    } else {
        legacy
    };
    let bytes = fs::read(&source)
        .map_err(|error| PhotaraError::filesystem("read template reference", &source, error))?;
    if sha256(&bytes) != reference.sha256 {
        return Err(PhotaraError::Configuration(format!(
            "reference {} does not match immutable template {}",
            source.display(),
            template.reference
        )));
    }
    let cached = config
        .settings
        .templates_cache
        .join(&template.template.name)
        .join(format!("v{}", template.template.version))
        .join(&reference.filename);
    if fs::read(&cached)
        .map(|existing| sha256(&existing) != reference.sha256)
        .unwrap_or(true)
    {
        write_atomic(&cached, &bytes)?;
    }
    let relative = PathBuf::from("posts")
        .join("_templates")
        .join(directory_name)
        .join(format!("v{}", template.template.version))
        .join(&reference.filename);
    let destination = project_root.join(&relative);
    if fs::read(&destination)
        .map(|existing| sha256(&existing) != reference.sha256)
        .unwrap_or(true)
    {
        write_atomic(&destination, &bytes)?;
    }
    Ok((relative, reference.sha256.clone()))
}

fn verify_file_evidence(file: &ResolvedFile) -> Result<()> {
    let metadata = fs::metadata(&file.path)
        .map_err(|source| PhotaraError::filesystem("inspect layout source", &file.path, source))?;
    if metadata.len() != file.byte_size as u64 {
        return Err(PhotaraError::Configuration(format!(
            "layout source {} changed size after registration",
            file.path.display()
        )));
    }
    let actual = sha256_file(&file.path)?;
    if actual != file.sha256 {
        return Err(PhotaraError::Configuration(format!(
            "layout source {} no longer matches its registered SHA-256",
            file.path.display()
        )));
    }
    Ok(())
}

fn project_relative_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            PhotaraError::Configuration(format!(
                "layout source {} is outside project root {}",
                path.display(),
                project_root.display()
            ))
        })
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .map_err(|source| PhotaraError::filesystem("open layout source", path, source))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|source| PhotaraError::filesystem("hash layout source", path, source))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn inspect_tiff_dimensions(path: &Path) -> Result<(u32, u32)> {
    let mut file = fs::File::open(path)
        .map_err(|source| PhotaraError::filesystem("open layout TIFF", path, source))?;
    let mut header = [0_u8; 8];
    file.read_exact(&mut header)
        .map_err(|source| PhotaraError::filesystem("read layout TIFF header", path, source))?;
    let little = match &header[..2] {
        b"II" => true,
        b"MM" => false,
        _ => {
            return Err(PhotaraError::Configuration(format!(
                "{} has no TIFF byte-order marker",
                path.display()
            )));
        }
    };
    let decode_u16 = |value: [u8; 2]| {
        if little {
            u16::from_le_bytes(value)
        } else {
            u16::from_be_bytes(value)
        }
    };
    let decode_u32 = |value: [u8; 4]| {
        if little {
            u32::from_le_bytes(value)
        } else {
            u32::from_be_bytes(value)
        }
    };
    if decode_u16([header[2], header[3]]) != 42 {
        return Err(PhotaraError::Configuration(format!(
            "{} is not a classic TIFF file",
            path.display()
        )));
    }
    let ifd = u64::from(decode_u32([header[4], header[5], header[6], header[7]]));
    file.seek(SeekFrom::Start(ifd))
        .map_err(|source| PhotaraError::filesystem("seek layout TIFF IFD", path, source))?;
    let mut count_bytes = [0_u8; 2];
    file.read_exact(&mut count_bytes)
        .map_err(|source| PhotaraError::filesystem("read layout TIFF IFD count", path, source))?;
    let mut width = None;
    let mut height = None;
    for index in 0..usize::from(decode_u16(count_bytes)) {
        file.seek(SeekFrom::Start(ifd + 2 + (index as u64) * 12))
            .map_err(|source| PhotaraError::filesystem("seek layout TIFF entry", path, source))?;
        let mut entry = [0_u8; 12];
        file.read_exact(&mut entry)
            .map_err(|source| PhotaraError::filesystem("read layout TIFF entry", path, source))?;
        let tag = decode_u16([entry[0], entry[1]]);
        if !matches!(tag, 256 | 257) {
            continue;
        }
        let field_type = decode_u16([entry[2], entry[3]]);
        let count = decode_u32([entry[4], entry[5], entry[6], entry[7]]);
        if count != 1 || !matches!(field_type, 3 | 4) {
            continue;
        }
        let value = if field_type == 3 {
            u32::from(decode_u16([entry[8], entry[9]]))
        } else {
            decode_u32([entry[8], entry[9], entry[10], entry[11]])
        };
        match tag {
            256 => width = Some(value),
            257 => height = Some(value),
            _ => unreachable!(),
        }
    }
    match (width, height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => Err(PhotaraError::Configuration(format!(
            "{} has no readable TIFF dimensions",
            path.display()
        ))),
    }
}

fn validate_template(template: &LayoutTemplate, reference: &TemplateRef) -> Result<()> {
    if template.schema_version != 1
        || template.name != reference.name
        || template.version != reference.version
    {
        return Err(PhotaraError::Configuration(format!(
            "layout template {} identity does not match its requested reference",
            reference.display()
        )));
    }
    let valid_sizing = match template.kind.as_str() {
        "continuous-panorama" => template.canvas.sizing == "source-crop",
        _ => template.canvas.sizing == "platform-profile",
    };
    if !valid_sizing {
        return Err(PhotaraError::Configuration(format!(
            "layout template {} uses unsupported kind or canvas sizing",
            reference.display()
        )));
    }
    if template.custom_rows && template.kind != "stacked-three" {
        return Err(PhotaraError::Configuration(format!(
            "layout template {} enables custom rows for a non-stacked-three layout",
            reference.display()
        )));
    }
    if !matches!(
        template.kind.as_str(),
        "dynamic-range-comparison" | "edit-comparison"
    ) && (template.decoration.background
        || template.decoration.border
        || template.decoration.text)
    {
        return Err(PhotaraError::Configuration(format!(
            "clean template {} must not add background, border, or text",
            reference.display()
        )));
    }
    if template.wsp.mode != "hdr-sdr-pair"
        || template.wsp.hdr_layer != "HDR"
        || template.wsp.sdr_layer != "SDR"
        || !template.wsp.hdr_above_sdr
    {
        return Err(PhotaraError::Configuration(format!(
            "template {} does not satisfy the WSP HDR-over-SDR contract",
            reference.display()
        )));
    }
    let slot_matches = |id: &str, expected: NormalizedRect| {
        template.slots.iter().any(|slot| {
            slot.id == id
                && slot.kind == "image"
                && slot.bounds.x == expected.x
                && slot.bounds.y == expected.y
                && slot.bounds.width == expected.width
                && slot.bounds.height == expected.height
        })
    };
    let valid = match template.kind.as_str() {
        "full-frame" => {
            template.slots.len() == 1
                && slot_matches(
                    "image",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    },
                )
        }
        "stacked-two" => {
            template.slots.len() == 2
                && slot_matches(
                    "top",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 0.5,
                    },
                )
                && slot_matches(
                    "bottom",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.5,
                        width: 1.0,
                        height: 0.5,
                    },
                )
        }
        "stacked-three" => {
            template.slots.len() == 3
                && slot_matches(
                    "top",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 0.333375,
                    },
                )
                && slot_matches(
                    "middle",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.333375,
                        width: 1.0,
                        height: 0.33325,
                    },
                )
                && slot_matches(
                    "bottom",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.666625,
                        width: 1.0,
                        height: 0.333375,
                    },
                )
        }
        "grid-four" => {
            if !matches!(template.name.as_str(), "grid-four" | "grid-four-threads") {
                return Err(PhotaraError::Configuration(format!(
                    "unsupported four-image grid template name {:?}",
                    template.name
                )));
            }
            let expected_slots = [
                (
                    "top-left",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.5,
                        height: 0.5,
                    },
                ),
                (
                    "top-right",
                    NormalizedRect {
                        x: 0.5,
                        y: 0.0,
                        width: 0.5,
                        height: 0.5,
                    },
                ),
                (
                    "bottom-left",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.5,
                        width: 0.5,
                        height: 0.5,
                    },
                ),
                (
                    "bottom-right",
                    NormalizedRect {
                        x: 0.5,
                        y: 0.5,
                        width: 0.5,
                        height: 0.5,
                    },
                ),
            ];
            template.slots.len() == 4
                && expected_slots
                    .iter()
                    .all(|(id, bounds)| slot_matches(id, *bounds))
                && !template.decoration.background
                && !template.decoration.border
                && !template.decoration.text
        }
        "continuous-panorama" => {
            template.slots.len() == 1
                && slot_matches(
                    "image",
                    NormalizedRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    },
                )
                && matches!(
                    template.surface.as_ref(),
                    Some(surface)
                        if surface.frame_aspect == "3:4"
                            && surface.frame_count == 2
                            && surface.flow == "horizontal"
                            && surface.splitter == "web-sharp-pro"
                            && surface.resolution_policy == "no-upscale"
                )
        }
        "dynamic-range-comparison" => {
            let reference_ok = matches!(
                template.reference.as_ref(),
                Some(reference)
                    if reference.filename == "reference.psd"
                        && reference.sha256.len() == 64
                        && reference.width == 4500
                        && matches!(reference.height, 6000 | 8000)
                        && reference.bit_depth == 32
                        && reference.color_profile == "Display P3 Linear"
            );
            let comparison_ok = matches!(
                template.comparison.as_ref(),
                Some(comparison)
                    if comparison.left_role == "SDR"
                        && comparison.right_role == "HDR"
                        && comparison.hdr_headroom_sdr_base == "flat-white-1.0"
                        && comparison.hdr_headroom_hdr_top == "reference-gradient-1.0-to-10.0"
            );
            template.slots.len() == 2
                && template
                    .slots
                    .iter()
                    .any(|slot| slot.id == "top" && slot.kind == "image-pair")
                && template
                    .slots
                    .iter()
                    .any(|slot| slot.id == "bottom" && slot.kind == "image-pair")
                && template.decoration.background
                && template.decoration.border
                && template.decoration.text
                && reference_ok
                && comparison_ok
        }
        "edit-comparison" => {
            let reference_ok = matches!(
                template.reference.as_ref(),
                Some(reference)
                    if reference.filename == "reference.psd"
                        && reference.sha256.len() == 64
                        && reference.width == 4500
                        && matches!(reference.height, 6000 | 8000)
                        && reference.bit_depth == 32
                        && reference.color_profile == "Display P3 Linear"
            );
            let contract_ok = matches!(
                template.edit_comparison.as_ref(),
                Some(contract)
                    if contract.before_role == "neutral-raw-sdr"
                        && contract.after_role == "authored"
                        && contract.before_rendering == "lightroom-reset-adobe-color"
                        && [
                            &contract.text_layers.top_camera,
                            &contract.text_layers.top_capture,
                            &contract.text_layers.bottom_camera,
                            &contract.text_layers.bottom_capture,
                        ]
                        .iter()
                        .all(|path| !path.is_empty() && path.iter().all(|part| !part.is_empty()))
            );
            template.slots.len() == 2
                && template
                    .slots
                    .iter()
                    .any(|slot| slot.id == "top" && slot.kind == "edit-pair")
                && template
                    .slots
                    .iter()
                    .any(|slot| slot.id == "bottom" && slot.kind == "edit-pair")
                && template.decoration.background
                && template.decoration.border
                && template.decoration.text
                && reference_ok
                && contract_ok
        }
        _ => false,
    };
    if !valid {
        return Err(PhotaraError::Configuration(format!(
            "layout template {} does not match its supported geometry contract",
            reference.display()
        )));
    }
    Ok(())
}

fn validate_post_identity(project: &ProjectRecord, name: &str) -> Result<()> {
    validate_slug(name).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post name {name:?}: {message}"))
    })?;
    validate_slug(&project.slug).map_err(|message| {
        PhotaraError::Configuration(format!(
            "invalid project slug {:?}: {message}",
            project.slug
        ))
    })
}

fn validate_post(
    post: &PostSpecification,
    project: &ProjectRecord,
    name: &str,
    platform: PostPlatform,
) -> Result<()> {
    validate_post_identity(project, name)?;
    if !matches!(post.schema_version, 1 | 2)
        || post.project != project.slug
        || post.name != name
        || post.platform != platform
    {
        return Err(PhotaraError::Configuration(format!(
            "project post identity does not match {}/{}",
            project.slug, name
        )));
    }
    let mut ids = BTreeSet::new();
    for item in &post.items {
        validate_slug(&item.id).map_err(|message| {
            PhotaraError::Configuration(format!("invalid post item ID {:?}: {message}", item.id))
        })?;
        if !ids.insert(&item.id) {
            return Err(PhotaraError::Configuration(format!(
                "duplicate post item ID {:?}",
                item.id
            )));
        }
        if item.placements.is_empty() || item.placements.len() > 4 {
            return Err(PhotaraError::Configuration(format!(
                "post item {:?} must have one, two, three, or four placements",
                item.id
            )));
        }
        let template_name = item
            .template
            .as_deref()
            .and_then(|reference| TemplateRef::parse(reference).ok())
            .map(|reference| reference.name);
        let is_dynamic_range_comparison =
            template_name.as_deref() == Some("dynamic-range-comparison");
        let is_edit_comparison = template_name.as_deref() == Some("edit-comparison");
        let is_continuous_panorama = template_name.as_deref() == Some("continuous-panorama");
        if let Some(parameters) = item.stacked_three {
            if template_name.as_deref() != Some("stacked-three") {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} has stacked-three parameters but does not use a stacked-three template",
                    item.id
                )));
            }
            parameters.validate()?;
        }
        let mut slots = BTreeSet::new();
        for placement in &item.placements {
            if !slots.insert(&placement.slot) {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses slot {:?} more than once",
                    item.id, placement.slot
                )));
            }
            validate_focal_point(placement.focal_point)?;
            let supports_contain = !is_continuous_panorama
                && (!(is_dynamic_range_comparison || is_edit_comparison)
                    || matches!(placement.slot.as_str(), "top" | "bottom"));
            if !(matches!(placement.fit.as_str(), "fill" | "crop")
                || placement.fit == "contain" && supports_contain)
            {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses unsupported fit {:?}",
                    item.id, placement.fit
                )));
            }
            placement_transform(post.schema_version, placement)?;
        }
    }
    Ok(())
}

fn placement_transform(
    post_schema_version: u32,
    placement: &PostPlacement,
) -> Result<PlacementTransform> {
    let transform = match post_schema_version {
        1 => {
            if placement.transform.is_some() {
                return Err(PhotaraError::Configuration(
                    "post schema v1 placements cannot contain a structured transform".into(),
                ));
            }
            PlacementTransform {
                crop: placement.crop,
                rotation_quarter_turns_cw: 0,
            }
        }
        2 => {
            if placement.crop.is_some() && placement.transform.is_some() {
                return Err(PhotaraError::Configuration(
                    "post placement cannot contain both legacy crop and structured transform"
                        .into(),
                ));
            }
            placement.transform.unwrap_or(PlacementTransform {
                crop: placement.crop,
                rotation_quarter_turns_cw: 0,
            })
        }
        version => {
            return Err(PhotaraError::Configuration(format!(
                "unsupported post schema version {version}"
            )));
        }
    };
    validate_placement_transform(transform)?;
    Ok(transform)
}

fn validate_placement_transform(transform: PlacementTransform) -> Result<()> {
    if transform.rotation_quarter_turns_cw > 3 {
        return Err(PhotaraError::Configuration(format!(
            "placement rotation must be 0, 1, 2, or 3 clockwise quarter turns; received {}",
            transform.rotation_quarter_turns_cw
        )));
    }
    if let Some(crop) = transform.crop {
        validate_normalized_rect(crop, "placement transform crop")?;
    }
    Ok(())
}

fn placement_requires_authoring(fit: &str, transform: PlacementTransform) -> bool {
    transform.crop.is_none() && fit == "crop"
}

fn placement_enters_authoring(fit: &str, transform: PlacementTransform, reauthor: bool) -> bool {
    fit == "crop" && (transform.crop.is_none() || reauthor)
}

fn validate_focal_point(point: FocalPoint) -> Result<()> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || !(0.0..=1.0).contains(&point.x)
        || !(0.0..=1.0).contains(&point.y)
    {
        return Err(PhotaraError::Configuration(
            "layout focal point must contain finite normalized values from 0 through 1".into(),
        ));
    }
    Ok(())
}

fn validate_normalized_rect(rect: NormalizedRect, name: &str) -> Result<()> {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || rect.x < 0.0
        || rect.y < 0.0
        || rect.width <= 0.0
        || rect.height <= 0.0
        || rect.x + rect.width > 1.0 + f64::EPSILON
        || rect.y + rect.height > 1.0 + f64::EPSILON
    {
        return Err(PhotaraError::Configuration(format!(
            "layout {name} must be a positive normalized rectangle inside the source"
        )));
    }
    Ok(())
}

fn post_path(
    config: &PhotaraConfig,
    project: &str,
    name: &str,
    platform: PostPlatform,
) -> Result<PathBuf> {
    validate_slug(project).map_err(|message| {
        PhotaraError::Configuration(format!("invalid project slug {project:?}: {message}"))
    })?;
    validate_slug(name).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post name {name:?}: {message}"))
    })?;
    Ok(config
        .settings
        .projects_root
        .join(project)
        .join("posts")
        .join(platform.as_str())
        .join(format!("{name}.json")))
}

fn scripts_root(config: &PhotaraConfig) -> Result<PathBuf> {
    Ok(config
        .settings
        .lightroom_inbox
        .parent()
        .ok_or_else(|| {
            PhotaraError::Configuration(
                "lightroom_inbox must have a parent directory for Photoshop scripts".into(),
            )
        })?
        .join("Scripts"))
}

fn read_post(path: &Path) -> Result<PostSpecification> {
    let text = fs::read_to_string(path)
        .map_err(|source| PhotaraError::filesystem("read project post", path, source))?;
    parse_json(path, &text)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .map_err(|source| PhotaraError::filesystem("read JSON document", path, source))?;
    parse_json(path, &text)
}

async fn find_master(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    reference: &str,
) -> Result<MasterBinding> {
    let rows = sqlx::query(MASTER_SELECT_BY_REFERENCE)
        .bind(project.id)
        .bind(reference)
        .fetch_all(database.pool())
        .await?;
    match rows.as_slice() {
        [row] => binding_from_row(config, row),
        [] => Err(PhotaraError::Configuration(format!(
            "no verified layered PSB and flattened HDR TIFF in project {:?} match asset reference {reference:?}",
            project.slug
        ))),
        _ => Err(PhotaraError::Configuration(format!(
            "asset reference {reference:?} is ambiguous in project {:?}; use the stable asset UUID",
            project.slug
        ))),
    }
}

async fn find_master_by_id(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    asset_id: Uuid,
) -> Result<MasterBinding> {
    let row = sqlx::query(MASTER_SELECT_BY_ID)
        .bind(project.id)
        .bind(asset_id)
        .fetch_optional(database.pool())
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "asset {asset_id} no longer has a verified PSB and flattened HDR TIFF for project {:?}",
                project.slug
            ))
        })?;
    binding_from_row(config, &row)
}

const MASTER_SELECT_BY_REFERENCE: &str = concat!(
    "SELECT asset.id AS asset_id, asset.original_filename, ",
    "raw.id AS raw_id, raw.location AS raw_location, raw.sha256 AS raw_sha256, raw.byte_size AS raw_byte_size, ",
    "psb.id AS psb_id, psb.location AS psb_location, psb.sha256 AS psb_sha256, ",
    "psb.byte_size AS psb_byte_size, hdr_tiff.id AS hdr_tiff_id, ",
    "hdr_tiff.location AS hdr_tiff_location, hdr_tiff.sha256 AS hdr_tiff_sha256, ",
    "hdr_tiff.byte_size AS hdr_tiff_byte_size, sdr_tiff.id AS sdr_tiff_id, ",
    "sdr_tiff.location AS sdr_tiff_location, sdr_tiff.sha256 AS sdr_tiff_sha256, ",
    "sdr_tiff.byte_size AS sdr_tiff_byte_size ",
    "FROM project_assets AS membership ",
    "JOIN assets AS asset ON asset.id = membership.asset_id ",
    "JOIN asset_files AS raw ON raw.asset_id = asset.id AND raw.representation = 'camera-raw' ",
    "AND raw.authoritative AND raw.state = 'current' ",
    "JOIN layered_master_documents AS layered ON layered.project_id = membership.project_id ",
    "JOIN asset_files AS psb ON psb.id = layered.asset_file_id AND psb.asset_id = asset.id ",
    "AND psb.representation = 'layered-psb' AND psb.authoritative AND psb.state = 'current' ",
    "JOIN flattened_master_documents AS hdr ON hdr.project_id = membership.project_id ",
    "AND hdr.source_file_id = psb.id AND hdr.rendition_role = 'hdr' ",
    "JOIN asset_files AS hdr_tiff ON hdr_tiff.id = hdr.asset_file_id AND hdr_tiff.asset_id = asset.id ",
    "AND hdr_tiff.representation = 'flattened-hdr-tiff' AND hdr_tiff.authoritative AND hdr_tiff.state = 'current' ",
    "LEFT JOIN flattened_master_documents AS sdr ON sdr.project_id = membership.project_id ",
    "AND sdr.source_file_id = psb.id AND sdr.rendition_role = 'sdr' ",
    "AND EXISTS (SELECT 1 FROM asset_files AS current_sdr ",
    "WHERE current_sdr.id = sdr.asset_file_id AND current_sdr.asset_id = asset.id ",
    "AND current_sdr.representation = 'flattened-sdr-tiff' ",
    "AND current_sdr.authoritative AND current_sdr.state = 'current') ",
    "LEFT JOIN asset_files AS sdr_tiff ON sdr_tiff.id = sdr.asset_file_id AND sdr_tiff.asset_id = asset.id ",
    "AND sdr_tiff.representation = 'flattened-sdr-tiff' AND sdr_tiff.authoritative AND sdr_tiff.state = 'current' ",
    "WHERE membership.project_id = $1 AND (lower(asset.id::text) = lower($2) ",
    "OR lower(asset.original_filename) = lower($2) OR lower(asset.original_stem) = lower($2) ",
    "OR lower(regexp_replace(psb.location, '^.*/', '')) = lower($2) ",
    "OR lower(regexp_replace(hdr_tiff.location, '^.*/', '')) = lower($2) ",
    "OR lower(regexp_replace(sdr_tiff.location, '^.*/', '')) = lower($2))"
);

const MASTER_SELECT_BY_ID: &str = concat!(
    "SELECT asset.id AS asset_id, asset.original_filename, ",
    "raw.id AS raw_id, raw.location AS raw_location, raw.sha256 AS raw_sha256, raw.byte_size AS raw_byte_size, ",
    "psb.id AS psb_id, psb.location AS psb_location, psb.sha256 AS psb_sha256, ",
    "psb.byte_size AS psb_byte_size, hdr_tiff.id AS hdr_tiff_id, ",
    "hdr_tiff.location AS hdr_tiff_location, hdr_tiff.sha256 AS hdr_tiff_sha256, ",
    "hdr_tiff.byte_size AS hdr_tiff_byte_size, sdr_tiff.id AS sdr_tiff_id, ",
    "sdr_tiff.location AS sdr_tiff_location, sdr_tiff.sha256 AS sdr_tiff_sha256, ",
    "sdr_tiff.byte_size AS sdr_tiff_byte_size ",
    "FROM project_assets AS membership ",
    "JOIN assets AS asset ON asset.id = membership.asset_id ",
    "JOIN asset_files AS raw ON raw.asset_id = asset.id AND raw.representation = 'camera-raw' ",
    "AND raw.authoritative AND raw.state = 'current' ",
    "JOIN layered_master_documents AS layered ON layered.project_id = membership.project_id ",
    "JOIN asset_files AS psb ON psb.id = layered.asset_file_id AND psb.asset_id = asset.id ",
    "AND psb.representation = 'layered-psb' AND psb.authoritative AND psb.state = 'current' ",
    "JOIN flattened_master_documents AS hdr ON hdr.project_id = membership.project_id ",
    "AND hdr.source_file_id = psb.id AND hdr.rendition_role = 'hdr' ",
    "JOIN asset_files AS hdr_tiff ON hdr_tiff.id = hdr.asset_file_id AND hdr_tiff.asset_id = asset.id ",
    "AND hdr_tiff.representation = 'flattened-hdr-tiff' AND hdr_tiff.authoritative AND hdr_tiff.state = 'current' ",
    "LEFT JOIN flattened_master_documents AS sdr ON sdr.project_id = membership.project_id ",
    "AND sdr.source_file_id = psb.id AND sdr.rendition_role = 'sdr' ",
    "AND EXISTS (SELECT 1 FROM asset_files AS current_sdr ",
    "WHERE current_sdr.id = sdr.asset_file_id AND current_sdr.asset_id = asset.id ",
    "AND current_sdr.representation = 'flattened-sdr-tiff' ",
    "AND current_sdr.authoritative AND current_sdr.state = 'current') ",
    "LEFT JOIN asset_files AS sdr_tiff ON sdr_tiff.id = sdr.asset_file_id AND sdr_tiff.asset_id = asset.id ",
    "AND sdr_tiff.representation = 'flattened-sdr-tiff' AND sdr_tiff.authoritative AND sdr_tiff.state = 'current' ",
    "WHERE membership.project_id = $1 AND asset.id = $2"
);

fn binding_from_row(config: &PhotaraConfig, row: &sqlx::postgres::PgRow) -> Result<MasterBinding> {
    let asset_id: Uuid = row.try_get("asset_id")?;
    Ok(MasterBinding {
        asset_id,
        original_filename: row.try_get("original_filename")?,
        layered_psb: resolved_file(config, row, "psb", "layered-psb")?,
        hdr_tiff: resolved_file(config, row, "hdr_tiff", "flattened-hdr-tiff")?,
        sdr_tiff: resolved_optional_file(config, row, "sdr_tiff", "flattened-sdr-tiff")?,
        camera_raw: resolved_file(config, row, "raw", "camera-raw")?,
    })
}

fn resolved_optional_file(
    config: &PhotaraConfig,
    row: &sqlx::postgres::PgRow,
    prefix: &str,
    representation: &str,
) -> Result<Option<ResolvedFile>> {
    let id: Option<Uuid> = row.try_get(format!("{prefix}_id").as_str())?;
    if id.is_none() {
        return Ok(None);
    }
    resolved_file(config, row, prefix, representation).map(Some)
}

fn resolved_file(
    config: &PhotaraConfig,
    row: &sqlx::postgres::PgRow,
    prefix: &str,
    representation: &str,
) -> Result<ResolvedFile> {
    let logical_location: String = row.try_get(format!("{prefix}_location").as_str())?;
    let path = resolve_location(config, &logical_location)?;
    if !path.is_file() {
        return Err(PhotaraError::Configuration(format!(
            "registered {representation} {} is not available at {}",
            logical_location,
            path.display()
        )));
    }
    let sha256: Option<String> = row.try_get(format!("{prefix}_sha256").as_str())?;
    let byte_size: Option<i64> = row.try_get(format!("{prefix}_byte_size").as_str())?;
    Ok(ResolvedFile {
        id: row.try_get(format!("{prefix}_id").as_str())?,
        representation: representation.into(),
        logical_location,
        path,
        sha256: sha256.ok_or_else(|| {
            PhotaraError::Configuration(format!("registered {representation} has no SHA-256"))
        })?,
        byte_size: byte_size.ok_or_else(|| {
            PhotaraError::Configuration(format!("registered {representation} has no byte size"))
        })?,
    })
}

fn resolve_location(config: &PhotaraConfig, value: &str) -> Result<PathBuf> {
    let (root, relative) = if let Some(relative) = value.strip_prefix("images:") {
        (&config.settings.images_root, relative)
    } else if let Some(relative) = value.strip_prefix("projects:") {
        (&config.settings.projects_root, relative)
    } else {
        return Err(PhotaraError::Configuration(format!(
            "unsupported layout source location {value:?}"
        )));
    };
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(PhotaraError::Configuration(format!(
            "unsafe layout source location {value:?}"
        )));
    }
    Ok(root.join(relative))
}

fn parse_json<T: for<'de> Deserialize<'de>>(path: &Path, text: &str) -> Result<T> {
    serde_json::from_str(text).map_err(|error| {
        PhotaraError::Configuration(format!("could not parse {}: {error}", path.display()))
    })
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_or_verify_json(path: &Path, value: &impl Serialize) -> Result<bool> {
    let expected = pretty_json(value)?;
    match fs::read(path) {
        Ok(existing) if existing == expected => Ok(false),
        Ok(_) => Err(PhotaraError::Configuration(format!(
            "{} already exists with different contents",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_atomic(path, &expected)?;
            Ok(true)
        }
        Err(source) => Err(PhotaraError::filesystem("read project post", path, source)),
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    write_atomic(path, &pretty_json(value)?)
}

fn pretty_json(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        PhotaraError::Configuration(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|source| {
        PhotaraError::filesystem("create layout configuration directory", parent, source)
    })?;
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("layout"),
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| {
            PhotaraError::filesystem("create layout temporary file", &temporary, source)
        })?;
    file.write_all(contents).map_err(|source| {
        PhotaraError::filesystem("write layout temporary file", &temporary, source)
    })?;
    if let Err(source) = file.sync_all()
        && source.kind() != std::io::ErrorKind::Unsupported
        && source.raw_os_error() != Some(45)
    {
        return Err(PhotaraError::filesystem(
            "sync layout temporary file",
            &temporary,
            source,
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|source| PhotaraError::filesystem("replace layout configuration", path, source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn parses_template_references() {
        assert_eq!(
            TemplateRef::parse("full-frame@1").unwrap(),
            TemplateRef {
                name: "full-frame".into(),
                version: 1
            }
        );
        assert!(TemplateRef::parse("full-frame").is_err());
        assert!(TemplateRef::parse("Full-Frame@1").is_err());
        assert!(TemplateRef::parse("full-frame@0").is_err());
    }

    #[test]
    fn installs_and_refuses_mutated_immutable_template() {
        let temporary = tempfile::tempdir().unwrap();
        let first = install_builtin_templates(temporary.path()).unwrap();
        assert_eq!(first.installed.len(), 12);
        let second = install_builtin_templates(temporary.path()).unwrap();
        assert_eq!(second.verified.len(), 12);
        fs::write(temporary.path().join("full-frame/v1.json"), "{}\n").unwrap();
        assert!(install_builtin_templates(temporary.path()).is_err());
    }

    #[test]
    fn stacked_slots_resolve_to_two_exact_instagram_halves() {
        let top = resolve_bounds(
            NormalizedRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 0.5,
            },
            4500,
            6000,
        )
        .unwrap();
        let bottom = resolve_bounds(
            NormalizedRect {
                x: 0.0,
                y: 0.5,
                width: 1.0,
                height: 0.5,
            },
            4500,
            6000,
        )
        .unwrap();
        assert_eq!((top.x, top.y, top.width, top.height), (0, 0, 4500, 3000));
        assert_eq!(
            (bottom.x, bottom.y, bottom.width, bottom.height),
            (0, 3000, 4500, 3000)
        );
    }

    #[test]
    fn stacked_three_resolves_to_exact_threads_rows() {
        let template: LayoutTemplate = serde_json::from_str(STACKED_THREE_V1).unwrap();
        validate_template(
            &template,
            &TemplateRef {
                name: "stacked-three".into(),
                version: 1,
            },
        )
        .unwrap();
        let rows: Vec<_> = template
            .slots
            .iter()
            .map(|slot| resolve_bounds(slot.bounds, 4500, 8000).unwrap())
            .collect();
        assert_eq!(
            rows,
            [
                PixelRect {
                    x: 0,
                    y: 0,
                    width: 4500,
                    height: 2667,
                },
                PixelRect {
                    x: 0,
                    y: 2667,
                    width: 4500,
                    height: 2666,
                },
                PixelRect {
                    x: 0,
                    y: 5333,
                    width: 4500,
                    height: 2667,
                },
            ]
        );
    }

    #[test]
    fn flexible_stacked_three_resolves_independently_for_both_platforms() {
        let parameters = StackedThreeParameters {
            row_percentages: [30, 40, 30],
            underfill: StackedUnderfill::Error,
        };
        parameters.validate().unwrap();
        let bounds = parameters.slots();
        let instagram: Vec<_> = bounds
            .iter()
            .map(|bounds| resolve_bounds(*bounds, 4500, 6000).unwrap())
            .collect();
        let threads: Vec<_> = bounds
            .iter()
            .map(|bounds| resolve_bounds(*bounds, 4500, 8000).unwrap())
            .collect();
        assert_eq!(
            instagram,
            [
                PixelRect {
                    x: 0,
                    y: 0,
                    width: 4500,
                    height: 1800
                },
                PixelRect {
                    x: 0,
                    y: 1800,
                    width: 4500,
                    height: 2400
                },
                PixelRect {
                    x: 0,
                    y: 4200,
                    width: 4500,
                    height: 1800
                },
            ]
        );
        assert_eq!(
            threads,
            [
                PixelRect {
                    x: 0,
                    y: 0,
                    width: 4500,
                    height: 2400
                },
                PixelRect {
                    x: 0,
                    y: 2400,
                    width: 4500,
                    height: 3200
                },
                PixelRect {
                    x: 0,
                    y: 5600,
                    width: 4500,
                    height: 2400
                },
            ]
        );
    }

    #[test]
    fn flexible_stacked_three_requires_explicit_centered_underfill() {
        let rejected = StackedThreeParameters {
            row_percentages: [25, 40, 25],
            underfill: StackedUnderfill::Error,
        };
        assert!(rejected.validate().is_err());

        let letterboxed = StackedThreeParameters {
            row_percentages: [25, 40, 25],
            underfill: StackedUnderfill::OuterLetterbox,
        };
        letterboxed.validate().unwrap();
        assert!(letterboxed.needs_background());
        let rows: Vec<_> = letterboxed
            .slots()
            .iter()
            .map(|bounds| resolve_bounds(*bounds, 4500, 6000).unwrap())
            .collect();
        assert_eq!(rows[0].y, 300);
        assert_eq!(rows[2].y + rows[2].height, 5700);
    }

    #[test]
    fn flexible_stacked_three_rejects_zero_or_overfilled_rows() {
        for row_percentages in [[0, 50, 50], [40, 40, 30]] {
            assert!(
                StackedThreeParameters {
                    row_percentages,
                    underfill: StackedUnderfill::OuterLetterbox,
                }
                .validate()
                .is_err()
            );
        }
    }

    #[test]
    fn four_image_grid_uses_each_platform_aspect_without_gaps() {
        let instagram: LayoutTemplate = serde_json::from_str(GRID_FOUR_V1).unwrap();
        let threads: LayoutTemplate = serde_json::from_str(GRID_FOUR_THREADS_V1).unwrap();
        validate_template(
            &instagram,
            &TemplateRef {
                name: "grid-four".into(),
                version: 1,
            },
        )
        .unwrap();
        validate_template(
            &threads,
            &TemplateRef {
                name: "grid-four-threads".into(),
                version: 1,
            },
        )
        .unwrap();
        let instagram_cells: Vec<_> = instagram
            .slots
            .iter()
            .map(|slot| resolve_bounds(slot.bounds, 4500, 6000).unwrap())
            .collect();
        let threads_cells: Vec<_> = threads
            .slots
            .iter()
            .map(|slot| resolve_bounds(slot.bounds, 4500, 8000).unwrap())
            .collect();
        for cell in &instagram_cells {
            assert_eq!((cell.width, cell.height), (2250, 3000));
        }
        for cell in &threads_cells {
            assert_eq!((cell.width, cell.height), (2250, 4000));
        }
        assert_eq!(threads_cells[0].y, 0);
        assert_eq!(threads_cells[2].y + threads_cells[2].height, 8000);
    }

    #[test]
    fn comparison_cells_resolve_to_inspected_reference_geometry() {
        let template: LayoutTemplate = serde_json::from_str(DYNAMIC_RANGE_COMPARISON_V1).unwrap();
        let comparison = template.comparison.unwrap();
        let cells = [
            (
                comparison.cells.top_left,
                PixelRect {
                    x: 226,
                    y: 1475,
                    width: 2000,
                    height: 2000,
                },
            ),
            (
                comparison.cells.top_right,
                PixelRect {
                    x: 2276,
                    y: 1475,
                    width: 2000,
                    height: 2000,
                },
            ),
            (
                comparison.cells.bottom_left,
                PixelRect {
                    x: 226,
                    y: 3706,
                    width: 2000,
                    height: 2000,
                },
            ),
            (
                comparison.cells.bottom_right,
                PixelRect {
                    x: 2276,
                    y: 3706,
                    width: 2000,
                    height: 2000,
                },
            ),
        ];
        for (normalized, expected) in cells {
            let actual = resolve_bounds(normalized, 4500, 6000).unwrap();
            assert_eq!(
                (actual.x, actual.y, actual.width, actual.height),
                (expected.x, expected.y, expected.width, expected.height)
            );
        }
    }

    #[test]
    fn edit_comparison_cells_resolve_to_inspected_reference_geometry() {
        let template: LayoutTemplate = serde_json::from_str(EDIT_COMPARISON_V1).unwrap();
        validate_template(
            &template,
            &TemplateRef {
                name: "edit-comparison".into(),
                version: 1,
            },
        )
        .unwrap();
        let comparison = template.edit_comparison.unwrap();
        assert_eq!(
            resolve_bounds(comparison.cells.top_left, 4500, 6000).unwrap(),
            PixelRect {
                x: 226,
                y: 779,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.cells.top_right, 4500, 6000).unwrap(),
            PixelRect {
                x: 2276,
                y: 779,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.cells.bottom_left, 4500, 6000).unwrap(),
            PixelRect {
                x: 226,
                y: 3331,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.cells.bottom_right, 4500, 6000).unwrap(),
            PixelRect {
                x: 2276,
                y: 3331,
                width: 2000,
                height: 2000
            }
        );
    }

    #[test]
    fn threads_dynamic_range_comparison_resolves_to_inspected_geometry() {
        let template: LayoutTemplate = serde_json::from_str(DYNAMIC_RANGE_COMPARISON_V3).unwrap();
        validate_template(
            &template,
            &TemplateRef::parse("dynamic-range-comparison@3").unwrap(),
        )
        .unwrap();
        let comparison = template.comparison.unwrap();
        assert_eq!(
            resolve_bounds(comparison.cells.top_left, 4500, 8000).unwrap(),
            PixelRect {
                x: 226,
                y: 1903,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.cells.bottom_right, 4500, 8000).unwrap(),
            PixelRect {
                x: 2276,
                y: 4134,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.hdr_headroom_ramp, 4500, 8000).unwrap(),
            PixelRect {
                x: 2276,
                y: 1337,
                width: 2000,
                height: 189
            }
        );
    }

    #[test]
    fn threads_edit_comparison_resolves_to_inspected_geometry() {
        let template: LayoutTemplate = serde_json::from_str(EDIT_COMPARISON_V2).unwrap();
        validate_template(&template, &TemplateRef::parse("edit-comparison@2").unwrap()).unwrap();
        let comparison = template.edit_comparison.unwrap();
        assert_eq!(
            resolve_bounds(comparison.cells.top_left, 4500, 8000).unwrap(),
            PixelRect {
                x: 226,
                y: 1125,
                width: 2000,
                height: 2000
            }
        );
        assert_eq!(
            resolve_bounds(comparison.cells.bottom_right, 4500, 8000).unwrap(),
            PixelRect {
                x: 2276,
                y: 3677,
                width: 2000,
                height: 2000
            }
        );
    }

    #[test]
    fn platform_profiles_are_explicit() {
        let instagram = PostPlatform::Instagram.profile();
        let threads = PostPlatform::Threads.profile();
        assert_eq!((instagram.width, instagram.height), (4500, 6000));
        assert_eq!((threads.width, threads.height), (4500, 8000));
        assert_eq!(instagram.minimum_delivery_frames, 1);
        assert_eq!(instagram.maximum_delivery_frames, Some(20));
        assert_eq!(threads.minimum_delivery_frames, 1);
        assert_eq!(threads.maximum_delivery_frames, None);
    }

    #[test]
    fn instagram_delivery_frames_accept_positive_packages_up_to_the_platform_maximum() {
        let profile = PostPlatform::Instagram.profile();
        assert!(validate_delivery_frame_count(&profile, 1).is_ok());
        assert!(validate_delivery_frame_count(&profile, 10).is_ok());
        assert!(validate_delivery_frame_count(&profile, 20).is_ok());
        assert!(validate_delivery_frame_count(&profile, 0).is_err());
        assert!(validate_delivery_frame_count(&profile, 21).is_err());
    }

    fn test_placement(crop: Option<NormalizedRect>) -> PostPlacement {
        PostPlacement {
            slot: "image".into(),
            asset_id: Uuid::nil(),
            display_filename: "source.ARW".into(),
            fit: "crop".into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop,
            transform: None,
        }
    }

    #[test]
    fn placement_transform_identity_is_the_stable_default() {
        let transform: PlacementTransform = serde_json::from_str("{}").unwrap();
        assert_eq!(transform, PlacementTransform::default());
        assert_eq!(serde_json::to_string(&transform).unwrap(), "{}");
    }

    #[test]
    fn only_unresolved_crop_fit_requires_authoring() {
        let identity = PlacementTransform::default();
        assert!(!placement_requires_authoring("contain", identity));
        assert!(!placement_requires_authoring("fill", identity));
        assert!(placement_requires_authoring("crop", identity));
        assert!(!placement_requires_authoring(
            "crop",
            PlacementTransform {
                crop: Some(NormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                }),
                rotation_quarter_turns_cw: 0,
            }
        ));
    }

    #[test]
    fn authoring_selection_excludes_automatic_fits_and_resolved_crops() {
        let identity = PlacementTransform::default();
        let authored = PlacementTransform {
            crop: Some(NormalizedRect {
                x: 0.1,
                y: 0.1,
                width: 0.8,
                height: 0.8,
            }),
            rotation_quarter_turns_cw: 0,
        };
        let mixed = [("fill", identity), ("crop", identity), ("fill", identity)];
        assert_eq!(
            mixed
                .iter()
                .filter(|(fit, transform)| placement_enters_authoring(fit, *transform, false))
                .count(),
            1
        );
        assert!(!placement_enters_authoring("contain", identity, false));
        assert!(!placement_enters_authoring("crop", authored, false));
        assert!(placement_enters_authoring("crop", authored, true));
        assert!(!placement_enters_authoring("fill", authored, true));
        assert!(!placement_enters_authoring("contain", authored, true));
    }

    #[test]
    fn placement_transform_resolves_crop_after_rotation() {
        let transform = PlacementTransform {
            crop: Some(NormalizedRect {
                x: 0.25,
                y: 0.1,
                width: 0.5,
                height: 0.75,
            }),
            rotation_quarter_turns_cw: 1,
        };
        let (rotated_dimensions, crop) =
            resolve_placement_transform(transform, 6000, 4000).unwrap();
        assert_eq!(rotated_dimensions, (4000, 6000));
        assert_eq!(
            crop,
            Some(PixelRect {
                x: 1000,
                y: 600,
                width: 2000,
                height: 4500,
            })
        );
    }

    #[test]
    fn placement_transform_rejects_non_quarter_turn_values() {
        assert!(
            validate_placement_transform(PlacementTransform {
                crop: None,
                rotation_quarter_turns_cw: 4,
            })
            .is_err()
        );
    }

    #[test]
    fn post_schema_v1_translates_legacy_crop_without_mutation() {
        let crop = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.75,
            height: 0.5,
        };
        let placement = test_placement(Some(crop));
        assert_eq!(
            placement_transform(1, &placement).unwrap(),
            PlacementTransform {
                crop: Some(crop),
                rotation_quarter_turns_cw: 0,
            }
        );
        assert_eq!(placement.crop, Some(crop));
        assert!(placement.transform.is_none());
    }

    #[test]
    fn post_schema_rejects_conflicting_or_misversioned_transforms() {
        let crop = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.75,
            height: 0.5,
        };
        let mut placement = test_placement(Some(crop));
        placement.transform = Some(PlacementTransform::default());
        assert!(placement_transform(1, &placement).is_err());
        assert!(placement_transform(2, &placement).is_err());
        assert!(placement_transform(3, &test_placement(None)).is_err());
    }

    #[test]
    fn post_upgrade_moves_legacy_crop_into_structured_transform() {
        let crop = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.75,
            height: 0.5,
        };
        let mut post = PostSpecification {
            schema_version: 1,
            project: "project".into(),
            name: "post".into(),
            platform: PostPlatform::Instagram,
            items: vec![PostItem {
                id: "item".into(),
                template: Some("full-frame@1".into()),
                stacked_three: None,
                placements: vec![test_placement(Some(crop))],
            }],
        };
        upgrade_post_to_v2(&mut post).unwrap();
        assert_eq!(post.schema_version, 2);
        assert!(post.items[0].placements[0].crop.is_none());
        assert_eq!(
            post.items[0].placements[0].transform,
            Some(PlacementTransform {
                crop: Some(crop),
                rotation_quarter_turns_cw: 0,
            })
        );
    }

    #[test]
    fn authoring_input_fingerprint_is_deterministic_and_order_sensitive() {
        let placement = |item_id: &str| AuthoringPlacement {
            item_id: item_id.into(),
            slot: "image".into(),
            asset_id: Uuid::nil(),
            display_filename: "source.ARW".into(),
            template: "full-frame@1".into(),
            target_bounds: PixelRect {
                x: 0,
                y: 0,
                width: 4500,
                height: 6000,
            },
            source_relative_path: "masters/source_HDR.TIF".into(),
            source_sha256: "a".repeat(64),
            source_width: 6000,
            source_height: 4000,
            transform: PlacementTransform::default(),
        };
        let ordered = vec![placement("first"), placement("second")];
        let reversed = vec![placement("second"), placement("first")];
        let first =
            authoring_input_sha256("project", "post", PostPlatform::Instagram, &ordered).unwrap();
        let second =
            authoring_input_sha256("project", "post", PostPlatform::Instagram, &ordered).unwrap();
        assert_eq!(first, second);
        assert_ne!(
            first,
            authoring_input_sha256("project", "post", PostPlatform::Instagram, &reversed,).unwrap()
        );
    }

    #[test]
    fn panorama_crop_is_normalized_inside_its_source() {
        let crop = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.75,
            height: 0.5,
        };
        validate_normalized_rect(crop, "crop").unwrap();
        assert!(
            validate_normalized_rect(
                NormalizedRect {
                    x: 0.5,
                    y: 0.5,
                    width: 0.6,
                    height: 0.4,
                },
                "crop"
            )
            .is_err()
        );
    }

    #[test]
    fn accepted_dsc05417_panorama_crop_keeps_exact_instagram_pixels() {
        let transform = PlacementTransform {
            crop: Some(NormalizedRect {
                x: 0.050686987646,
                y: 0.114132317284,
                width: 0.88592541277,
                height: 0.885867682716,
            }),
            rotation_quarter_turns_cw: 0,
        };
        let (_, crop) = resolve_placement_transform(transform, 8661, 5774).unwrap();
        assert_eq!(
            crop,
            Some(PixelRect {
                x: 439,
                y: 659,
                width: 7673,
                height: 5115,
            })
        );
    }

    #[test]
    fn instagram_regression_fixture_freezes_accepted_package() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/red-meridian-instagram-package-a.json"
        ))
        .unwrap();
        assert_eq!(fixture["editorial_item_count"], 18);
        assert_eq!(fixture["delivery_frame_count"], 20);
        let items = fixture["items"].as_array().unwrap();
        let ids: Vec<_> = items
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "hero",
                "stacked-01",
                "full-frame-05217",
                "full-frame-05406",
                "panorama-05382",
                "full-frame-05409",
                "stacked-02",
                "full-frame-05421-a",
                "stacked-03",
                "full-frame-05382",
                "full-frame-05372",
                "full-frame-05421-b",
                "dynamic-range-01",
                "edit-comparison-01",
                "panorama-05417",
                "dynamic-range-02",
                "edit-comparison-02",
                "full-frame-05250-repeat",
            ]
        );
        let panorama = items
            .iter()
            .find(|item| item["id"] == "panorama-05417")
            .unwrap();
        assert_eq!(panorama["canvas_width"], 7673);
        assert_eq!(panorama["canvas_height"], 5115);
        assert_eq!(panorama["placements"][0]["source_crop"]["x"], 439);
        assert_eq!(panorama["placements"][0]["source_crop"]["y"], 659);
    }

    #[test]
    fn stacked_crop_reuse_requires_the_same_asset() {
        let asset_id = Uuid::nil();
        let crop = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.75,
            height: 0.5,
        };
        let post = PostSpecification {
            schema_version: 1,
            project: "project".into(),
            name: "post".into(),
            platform: PostPlatform::Instagram,
            items: vec![PostItem {
                id: "panorama".into(),
                template: Some("continuous-panorama@1".into()),
                stacked_three: None,
                placements: vec![PostPlacement {
                    slot: "image".into(),
                    asset_id,
                    display_filename: "source.ARW".into(),
                    fit: "crop".into(),
                    focal_point: FocalPoint { x: 0.5, y: 0.5 },
                    crop: Some(crop),
                    transform: None,
                }],
            }],
        };

        assert_eq!(
            reuse_item_crop(&post, "panorama", asset_id, "bottom").unwrap(),
            crop
        );
        assert!(reuse_item_crop(&post, "panorama", Uuid::from_u128(1), "bottom").is_err());
    }

    #[test]
    fn post_reorder_requires_an_exact_permutation() {
        let make_item = |id: &str| PostItem {
            id: id.into(),
            template: None,
            stacked_three: None,
            placements: vec![PostPlacement {
                slot: "image".into(),
                asset_id: Uuid::nil(),
                display_filename: "source.ARW".into(),
                fit: "fill".into(),
                focal_point: FocalPoint { x: 0.5, y: 0.5 },
                crop: None,
                transform: None,
            }],
        };
        let mut post = PostSpecification {
            schema_version: 1,
            project: "project".into(),
            name: "post".into(),
            platform: PostPlatform::Instagram,
            items: vec![make_item("first"), make_item("second")],
        };

        assert!(reorder_items(&mut post, &["second".into(), "first".into()]).unwrap());
        assert_eq!(post.items[0].id, "second");
        assert!(!reorder_items(&mut post, &["second".into(), "first".into()]).unwrap());
        assert!(reorder_items(&mut post, &["first".into()]).is_err());
        assert!(reorder_items(&mut post, &["first".into(), "first".into()]).is_err());
    }
}
