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
const CONTINUOUS_PANORAMA_V1: &str = include_str!("../templates/continuous-panorama/v1.json");
const DYNAMIC_RANGE_COMPARISON_V1: &str =
    include_str!("../templates/dynamic-range-comparison/v1.json");
const DYNAMIC_RANGE_COMPARISON_V2: &str =
    include_str!("../templates/dynamic-range-comparison/v2.json");
const EDIT_COMPARISON_V1: &str = include_str!("../templates/edit-comparison/v1.json");
const LAYOUT_SCRIPT: &str = include_str!("../photoshop/Build Photara Layouts.psjs");
const LAYOUT_SCRIPT_NAME: &str = "Build Photara Layouts.psjs";
const LAYOUT_HANDOFF_NAME: &str = "Photara Layout Manifest.json";
const PANORAMA_CROP_SCRIPT: &str = include_str!("../photoshop/Author Photara Panorama Crop.psjs");
const PANORAMA_CAPTURE_SCRIPT: &str =
    include_str!("../photoshop/Capture Photara Panorama Crop.psjs");
const PANORAMA_CROP_SCRIPT_NAME: &str = "Author Photara Panorama Crop.psjs";
const PANORAMA_CAPTURE_SCRIPT_NAME: &str = "Capture Photara Panorama Crop.psjs";
const PANORAMA_CROP_HANDOFF_NAME: &str = "Photara Panorama Crop Manifest.json";
const PANORAMA_CROP_REPORT_NAME: &str = "Photara Panorama Crop Report.json";
const EDIT_SOURCE_HANDOFF_NAME: &str = "Photara Edit Comparison Source Manifest.json";
const EDIT_SOURCE_REPORT_NAME: &str = "Photara Edit Comparison Source Report.json";

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
            },
            Self::Threads => PlatformProfile {
                name: "threads-portrait".into(),
                width: 4500,
                height: 8000,
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
    pub placements: Vec<PostPlacement>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PostPlacement {
    pub slot: String,
    pub asset_id: Uuid,
    pub display_filename: String,
    pub fit: String,
    pub focal_point: FocalPoint,
    #[serde(default)]
    pub crop: Option<NormalizedRect>,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedItem {
    pub id: String,
    pub template: ResolvedTemplate,
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
struct EditSourceReportItem {
    item_id: String,
    slot: String,
    asset_id: Uuid,
    state: String,
    output_relative_path: PathBuf,
    output_sha256: String,
    output_byte_size: u64,
    profile: String,
    restored: bool,
    metadata: CaptureMetadataInput,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CaptureMetadataInput {
    make: String,
    model: String,
    lens: String,
    iso: u32,
    focal_length_mm: f64,
    aperture: f64,
    exposure_seconds: f64,
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
        ("continuous-panorama@1", CONTINUOUS_PANORAMA_V1),
        ("dynamic-range-comparison@1", DYNAMIC_RANGE_COMPARISON_V1),
        ("dynamic-range-comparison@2", DYNAMIC_RANGE_COMPARISON_V2),
        ("edit-comparison@1", EDIT_COMPARISON_V1),
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

pub async fn add_full_frame(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    asset_reference: &str,
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
        placements: vec![PostPlacement {
            slot: "image".into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename,
            fit: "fill".into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop: None,
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
        })
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
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
    placement.crop.ok_or_else(|| {
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
        placements: vec![PostPlacement {
            slot: "image".into(),
            asset_id: binding.asset_id,
            display_filename: binding.original_filename,
            fit: "crop".into(),
            focal_point: FocalPoint { x: 0.5, y: 0.5 },
            crop: None,
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

pub async fn add_dynamic_range_comparison(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    bottom_reference: &str,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = config
        .settings
        .layouts
        .defaults
        .dynamic_range_comparison
        .clone();
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
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
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

pub async fn add_edit_comparison(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
    top_reference: &str,
    bottom_reference: &str,
) -> Result<PostWriteReport> {
    validate_post_identity(project, post_name)?;
    validate_slug(item_id).map_err(|message| {
        PhotaraError::Configuration(format!("invalid post item ID {item_id:?}: {message}"))
    })?;
    let template_reference = config.settings.layouts.defaults.edit_comparison.clone();
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
    };
    let item = PostItem {
        id: item_id.into(),
        template: Some(template_reference),
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
    platform: PostPlatform,
) -> Result<EditSourceManifest> {
    let resolved = resolve_post(database, config, project, post_name, platform).await?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let output_root = PathBuf::from("posts")
        .join(platform.as_str())
        .join("sources")
        .join(post_name)
        .join("before");
    fs::create_dir_all(project_root.join(&output_root)).map_err(|source| {
        PhotaraError::filesystem(
            "create edit comparison source directory",
            project_root.join(&output_root),
            source,
        )
    })?;
    let mut items = Vec::new();
    for item in resolved.items {
        if item.template.template.kind != "edit-comparison" {
            continue;
        }
        for placement in item.placements {
            verify_file_evidence(&placement.camera_raw)?;
            let stem = placement
                .original_filename
                .rsplit_once('.')
                .map(|(stem, _)| stem)
                .unwrap_or(&placement.original_filename);
            let output_filename = format!("{stem}_RESET_ADOBE_COLOR.TIF");
            items.push(EditSourceManifestItem {
                item_id: item.id.clone(),
                slot: placement.slot,
                asset_id: placement.asset_id,
                original_filename: placement.original_filename,
                camera_raw_path: placement.camera_raw.path,
                output_relative_path: output_root.join(&output_filename),
                output_filename,
            });
        }
    }
    if items.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "post {post_name:?} has no edit-comparison items"
        )));
    }
    let manifest = EditSourceManifest {
        schema_version: 1,
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        project_root: project_root.clone(),
        source_specification: resolved.source_path,
        source_sha256: resolved.source_sha256,
        rendering: "lightroom-reset-adobe-color".into(),
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
    validate_normalized_rect(crop, "crop")?;
    let path = post_path(config, &project.slug, post_name, platform)?;
    let mut post = read_post(&path)?;
    validate_post(&post, project, post_name, platform)?;
    let item = post
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    if item.placements.len() != 1 {
        return Err(PhotaraError::Configuration(format!(
            "post item {item_id:?} cannot accept a single panorama crop"
        )));
    }
    let changed = item.placements[0].crop != Some(crop);
    item.placements[0].crop = Some(crop);
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

pub async fn prepare_panorama_crop(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
) -> Result<PanoramaCropHandoff> {
    let path = post_path(config, &project.slug, post_name, platform)?;
    let text = fs::read_to_string(&path)
        .map_err(|source| PhotaraError::filesystem("read project post", &path, source))?;
    let post: PostSpecification = parse_json(&path, &text)?;
    validate_post(&post, project, post_name, platform)?;
    let item = post
        .items
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("post item {item_id:?} was not found"))
        })?;
    let template_reference = item.template.as_deref().ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "post item {item_id:?} does not explicitly select a panorama template"
        ))
    })?;
    let template = load_template(config, template_reference)?;
    if template.template.kind != "continuous-panorama" || item.placements.len() != 1 {
        return Err(PhotaraError::Configuration(format!(
            "post item {item_id:?} is not a single-source continuous panorama"
        )));
    }
    let placement = &item.placements[0];
    let binding = find_master_by_id(database, config, project, placement.asset_id).await?;
    verify_file_evidence(&binding.hdr_tiff)?;
    let project_root = config.settings.projects_root.join(&project.slug);
    let source_relative_path = project_relative_path(&project_root, &binding.hdr_tiff.path)?;
    let scripts_root = scripts_root(config)?;
    fs::create_dir_all(&scripts_root).map_err(|source| {
        PhotaraError::filesystem("create Photoshop scripts directory", &scripts_root, source)
    })?;
    let author_script = scripts_root.join(PANORAMA_CROP_SCRIPT_NAME);
    let capture_script = scripts_root.join(PANORAMA_CAPTURE_SCRIPT_NAME);
    write_atomic(&author_script, PANORAMA_CROP_SCRIPT.as_bytes())?;
    write_atomic(&capture_script, PANORAMA_CAPTURE_SCRIPT.as_bytes())?;
    let handoff = PanoramaCropHandoff {
        schema_version: 1,
        project: project.slug.clone(),
        post: post_name.into(),
        platform,
        item_id: item_id.into(),
        project_root: project_root.clone(),
        source_specification: path,
        source_specification_sha256: sha256(text.as_bytes()),
        source_relative_path,
        source_filename: binding
            .hdr_tiff
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                PhotaraError::Configuration("panorama source filename is not UTF-8".into())
            })?
            .into(),
        source_sha256: binding.hdr_tiff.sha256,
        author_script,
        capture_script,
        frame_aspect: "3:4".into(),
        frame_count: 2,
        crop_aspect_ratio: "3:2".into(),
    };
    write_json_atomic(&project_root.join(PANORAMA_CROP_HANDOFF_NAME), &handoff)?;
    Ok(handoff)
}

pub fn apply_panorama_crop(
    config: &PhotaraConfig,
    project: &ProjectRecord,
    post_name: &str,
    platform: PostPlatform,
    item_id: &str,
) -> Result<PostWriteReport> {
    let project_root = config.settings.projects_root.join(&project.slug);
    let handoff_path = project_root.join(PANORAMA_CROP_HANDOFF_NAME);
    let handoff_text = fs::read_to_string(&handoff_path).map_err(|source| {
        PhotaraError::filesystem("read panorama crop manifest", &handoff_path, source)
    })?;
    let handoff: PanoramaCropHandoff = parse_json(&handoff_path, &handoff_text)?;
    let report_path = project_root.join(PANORAMA_CROP_REPORT_NAME);
    let report_text = fs::read_to_string(&report_path).map_err(|source| {
        PhotaraError::filesystem("read panorama crop report", &report_path, source)
    })?;
    let report: PanoramaCropReport = parse_json(&report_path, &report_text)?;
    if report.schema_version != 1
        || report.project != project.slug
        || report.post != post_name
        || report.platform != platform
        || report.item_id != item_id
        || handoff.project != report.project
        || handoff.post != report.post
        || handoff.platform != report.platform
        || handoff.item_id != report.item_id
        || handoff.source_specification_sha256 != report.source_specification_sha256
        || handoff.source_sha256 != report.source_sha256
    {
        return Err(PhotaraError::Configuration(
            "panorama crop report does not match the requested project post item".into(),
        ));
    }
    let specification_path = post_path(config, &project.slug, post_name, platform)?;
    let specification = fs::read(&specification_path).map_err(|source| {
        PhotaraError::filesystem("read project post", &specification_path, source)
    })?;
    if sha256(&specification) != report.source_specification_sha256 {
        return Err(PhotaraError::Configuration(
            "project post changed after panorama crop authoring began; prepare the crop again"
                .into(),
        ));
    }
    let source_path = project_root.join(&handoff.source_relative_path);
    if sha256_file(&source_path)? != report.source_sha256 {
        return Err(PhotaraError::Configuration(
            "panorama source changed after crop authoring began; prepare the crop again".into(),
        ));
    }
    if report.document_width == 0 || report.document_height == 0 {
        return Err(PhotaraError::Configuration(
            "panorama crop report has invalid document dimensions".into(),
        ));
    }
    let pixel_ratio = f64::from(report.crop_pixels.width) / f64::from(report.crop_pixels.height);
    if (pixel_ratio - 1.5).abs() > 0.002 {
        return Err(PhotaraError::Configuration(format!(
            "two-frame panorama crop must be 3:2; received {pixel_ratio:.6}:1"
        )));
    }
    let expected = NormalizedRect {
        x: f64::from(report.crop_pixels.x) / f64::from(report.document_width),
        y: f64::from(report.crop_pixels.y) / f64::from(report.document_height),
        width: f64::from(report.crop_pixels.width) / f64::from(report.document_width),
        height: f64::from(report.crop_pixels.height) / f64::from(report.document_height),
    };
    for (actual, expected) in [
        (report.crop.x, expected.x),
        (report.crop.y, expected.y),
        (report.crop.width, expected.width),
        (report.crop.height, expected.height),
    ] {
        if (actual - expected).abs() > 1e-9 {
            return Err(PhotaraError::Configuration(
                "panorama crop report pixel and normalized coordinates disagree".into(),
            ));
        }
    }
    set_item_crop(config, project, post_name, platform, item_id, report.crop)
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
    if let Some(filter) = item_filter {
        if selected_items.is_empty() {
            return Err(PhotaraError::Configuration(format!(
                "post {post_name:?} has no item {filter:?}"
            )));
        }
    }
    for item in selected_items {
        let template_reference = item
            .template
            .clone()
            .unwrap_or_else(|| config.settings.layouts.defaults.full_frame.clone());
        let template = load_template(config, &template_reference)?;
        let slot_ids: BTreeSet<_> = template
            .template
            .slots
            .iter()
            .map(|slot| slot.id.as_str())
            .collect();
        let mut placements = Vec::with_capacity(item.placements.len());
        for placement in item.placements {
            if !slot_ids.contains(placement.slot.as_str()) {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses slot {:?}, which template {} does not define",
                    item.id, placement.slot, template.reference
                )));
            }
            validate_focal_point(placement.focal_point)?;
            if let Some(crop) = placement.crop {
                validate_normalized_rect(crop, "crop")?;
            }
            let binding = find_master_by_id(database, config, project, placement.asset_id).await?;
            if template.template.kind == "continuous-panorama" && placement.crop.is_none() {
                requirements.insert(format!(
                    "author a 3:2 crop for panorama item {} ({})",
                    item.id, binding.original_filename
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
                crop: placement.crop,
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
    if item_filter.is_none()
        && resolved.platform == PostPlatform::Instagram
        && resolved.delivery_frame_count != 20
    {
        return Err(PhotaraError::Configuration(format!(
            "final Instagram render manifest must expand to exactly 20 ordered delivery frames; resolved {}",
            resolved.delivery_frame_count
        )));
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
            let crop = resolve_bounds(
                placement.crop.ok_or_else(|| {
                    PhotaraError::Configuration(format!(
                        "continuous panorama item {:?} has no authored crop",
                        item.id
                    ))
                })?,
                hdr_dimensions.0,
                hdr_dimensions.1,
            )?;
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
                        .get(&(item.id.clone(), placement.slot.clone()))
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
            let source_crop = if let Some(crop) = placement.crop {
                let hdr_dimensions = inspect_tiff_dimensions(&placement.hdr_tiff.path)?;
                let sdr_dimensions = inspect_tiff_dimensions(&sdr.path)?;
                if hdr_dimensions != sdr_dimensions {
                    return Err(PhotaraError::Configuration(format!(
                        "paired TIFF dimensions differ for cropped placement {} in item {:?}",
                        placement.original_filename, item.id
                    )));
                }
                Some(resolve_bounds(crop, hdr_dimensions.0, hdr_dimensions.1)?)
            } else {
                None
            };
            render_placements.push(LayoutRenderPlacement {
                slot: placement.slot.clone(),
                bounds: resolve_bounds(slot.bounds, canvas_width, canvas_height)?,
                fit: placement.fit.clone(),
                focal_point: placement.focal_point,
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

fn load_edit_sources(
    project_root: &Path,
    resolved: &ResolvedPost,
) -> Result<BTreeMap<(String, String), EditSourceReportItem>> {
    if !resolved
        .items
        .iter()
        .any(|item| item.template.template.kind == "edit-comparison")
    {
        return Ok(BTreeMap::new());
    }
    let path = project_root.join(EDIT_SOURCE_REPORT_NAME);
    let report: EditSourceReport = read_json(&path)?;
    if report.schema_version != 1
        || report.project != resolved.project
        || report.post != resolved.name
        || report.platform != resolved.platform
        || report.source_sha256 != resolved.source_sha256
    {
        return Err(PhotaraError::Configuration(format!(
            "edit comparison source report {} is stale or belongs to another post",
            path.display()
        )));
    }
    let mut sources = BTreeMap::new();
    for item in report.items {
        if item.state != "verified" || !item.restored {
            return Err(PhotaraError::Configuration(format!(
                "neutral source for item {:?} slot {:?} was not verified and safely restored",
                item.item_id, item.slot
            )));
        }
        if !item.profile.eq_ignore_ascii_case("Adobe Color") {
            return Err(PhotaraError::Configuration(format!(
                "neutral source for item {:?} slot {:?} used profile {:?}, not Adobe Color",
                item.item_id, item.slot, item.profile
            )));
        }
        let output = project_root.join(&item.output_relative_path);
        let metadata = fs::metadata(&output).map_err(|source| {
            PhotaraError::filesystem("inspect neutral source", &output, source)
        })?;
        if metadata.len() != item.output_byte_size || sha256_file(&output)? != item.output_sha256 {
            return Err(PhotaraError::Configuration(format!(
                "neutral source {} changed after Lightroom verification",
                output.display()
            )));
        }
        if sources
            .insert((item.item_id.clone(), item.slot.clone()), item)
            .is_some()
        {
            return Err(PhotaraError::Configuration(
                "edit comparison source report contains duplicate slots".into(),
            ));
        }
    }
    Ok(sources)
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
                        && reference.height == 6000
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
                        && reference.height == 6000
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
    if post.schema_version != 1
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
        if item.placements.is_empty() || item.placements.len() > 2 {
            return Err(PhotaraError::Configuration(format!(
                "post item {:?} must have one or two placements",
                item.id
            )));
        }
        let comparison_name = item
            .template
            .as_deref()
            .and_then(|reference| TemplateRef::parse(reference).ok())
            .map(|reference| reference.name);
        let is_dynamic_range_comparison =
            comparison_name.as_deref() == Some("dynamic-range-comparison");
        let is_edit_comparison = comparison_name.as_deref() == Some("edit-comparison");
        let mut slots = BTreeSet::new();
        for placement in &item.placements {
            if !slots.insert(&placement.slot) {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses slot {:?} more than once",
                    item.id, placement.slot
                )));
            }
            validate_focal_point(placement.focal_point)?;
            let supports_contain = (is_dynamic_range_comparison || is_edit_comparison)
                && matches!(placement.slot.as_str(), "top" | "bottom");
            if !matches!(placement.fit.as_str(), "fill" | "crop")
                && !(placement.fit == "contain" && supports_contain)
            {
                return Err(PhotaraError::Configuration(format!(
                    "post item {:?} uses unsupported fit {:?}",
                    item.id, placement.fit
                )));
            }
            if let Some(crop) = placement.crop {
                validate_normalized_rect(crop, "crop")?;
            }
        }
    }
    Ok(())
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
        assert_eq!(first.installed.len(), 6);
        let second = install_builtin_templates(temporary.path()).unwrap();
        assert_eq!(second.verified.len(), 6);
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
    fn platform_profiles_are_explicit() {
        let instagram = PostPlatform::Instagram.profile();
        let threads = PostPlatform::Threads.profile();
        assert_eq!((instagram.width, instagram.height), (4500, 6000));
        assert_eq!((threads.width, threads.height), (4500, 8000));
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
                placements: vec![PostPlacement {
                    slot: "image".into(),
                    asset_id,
                    display_filename: "source.ARW".into(),
                    fit: "crop".into(),
                    focal_point: FocalPoint { x: 0.5, y: 0.5 },
                    crop: Some(crop),
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
            placements: vec![PostPlacement {
                slot: "image".into(),
                asset_id: Uuid::nil(),
                display_filename: "source.ARW".into(),
                fit: "fill".into(),
                focal_point: FocalPoint { x: 0.5, y: 0.5 },
                crop: None,
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
