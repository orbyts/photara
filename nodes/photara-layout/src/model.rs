use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    num::NonZeroU32,
};

use photara_core::{
    AssetId, AssetSet, CanonicalDigest, ColorSpaceId, Diagnostic, DiagnosticSeverity,
    NodeInstanceId, ProjectId, ProxyProfile, RequestId, SchemaId, SchemaRef, SchemaValue,
    SchemaVersion, TypedValue, ValueTypeDescriptor, ValueTypeId, ValueTypeRef, ValueTypeVersion,
    canonical_digest,
};
use photara_proxy::{
    ProjectVisualProxyRequest, ProjectVisualProxyService, ProxyArtifact, ProxyServiceError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::LAYOUT_PLAN_TYPE_ID;

pub const LAYOUT_STATE_SCHEMA_ID: &str = "photara.layout.state";
pub const LAYOUT_PLAN_SCHEMA_ID: &str = "photara.layout-plan.value";
pub const NORMALIZED_SCALE: u32 = 1_000_000;

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

uuid_id!(LayoutFrameId);
uuid_id!(LayoutCellId);

/// Fixed-point normalized coordinate in `[0, 1]`, with one-millionth precision.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NormalizedUnit(u32);

impl NormalizedUnit {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(NORMALIZED_SCALE);
    pub const CENTER: Self = Self(NORMALIZED_SCALE / 2);

    /// Creates a fixed-point normalized value.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutValidationError::NormalizedOutOfRange`] above one.
    pub const fn new(value: u32) -> Result<Self, LayoutValidationError> {
        if value > NORMALIZED_SCALE {
            return Err(LayoutValidationError::NormalizedOutOfRange(value));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedPoint {
    pub x: NormalizedUnit,
    pub y: NormalizedUnit,
}

impl NormalizedPoint {
    pub const CENTER: Self = Self {
        x: NormalizedUnit::CENTER,
        y: NormalizedUnit::CENTER,
    };
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedRect {
    pub x: NormalizedUnit,
    pub y: NormalizedUnit,
    pub width: NormalizedUnit,
    pub height: NormalizedUnit,
}

impl NormalizedRect {
    pub const FULL: Self = Self {
        x: NormalizedUnit::ZERO,
        y: NormalizedUnit::ZERO,
        width: NormalizedUnit::ONE,
        height: NormalizedUnit::ONE,
    };

    fn validate(self) -> Result<(), LayoutValidationError> {
        if self.width == NormalizedUnit::ZERO || self.height == NormalizedUnit::ZERO {
            return Err(LayoutValidationError::EmptyNormalizedRect);
        }
        if self.x.get().saturating_add(self.width.get()) > NORMALIZED_SCALE
            || self.y.get().saturating_add(self.height.get()) > NORMALIZED_SCALE
        {
            return Err(LayoutValidationError::NormalizedRectEscapesBounds);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedInsets {
    pub top: NormalizedUnit,
    pub right: NormalizedUnit,
    pub bottom: NormalizedUnit,
    pub left: NormalizedUnit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundledCanvasProfile {
    Portrait3x4,
    Vertical9x16,
}

impl BundledCanvasProfile {
    const fn aspect(self) -> (u32, u32) {
        match self {
            Self::Portrait3x4 => (3, 4),
            Self::Vertical9x16 => (9, 16),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LayoutCanvas {
    Bundled {
        profile: BundledCanvasProfile,
        profile_version: NonZeroU32,
        long_edge_pixels: NonZeroU32,
    },
    CustomPixels {
        width: NonZeroU32,
        height: NonZeroU32,
    },
    CustomAspect {
        horizontal_units: NonZeroU32,
        vertical_units: NonZeroU32,
        long_edge_pixels: NonZeroU32,
    },
}

impl LayoutCanvas {
    #[must_use]
    pub fn portrait_3x4(long_edge_pixels: NonZeroU32) -> Self {
        Self::Bundled {
            profile: BundledCanvasProfile::Portrait3x4,
            profile_version: NonZeroU32::MIN,
            long_edge_pixels,
        }
    }

    #[must_use]
    pub fn vertical_9x16(long_edge_pixels: NonZeroU32) -> Self {
        Self::Bundled {
            profile: BundledCanvasProfile::Vertical9x16,
            profile_version: NonZeroU32::MIN,
            long_edge_pixels,
        }
    }

    /// Resolves exact output pixels from a versioned ratio or custom dimensions.
    ///
    /// # Errors
    ///
    /// Returns a validation error for unsupported bundled versions or arithmetic
    /// overflow.
    pub fn pixel_size(&self) -> Result<PixelSize, LayoutValidationError> {
        match *self {
            Self::Bundled {
                profile,
                profile_version,
                long_edge_pixels,
            } => {
                if profile_version != NonZeroU32::MIN {
                    return Err(LayoutValidationError::UnsupportedCanvasProfileVersion(
                        profile_version.get(),
                    ));
                }
                aspect_size(profile.aspect(), long_edge_pixels)
            }
            Self::CustomPixels { width, height } => Ok(PixelSize { width, height }),
            Self::CustomAspect {
                horizontal_units,
                vertical_units,
                long_edge_pixels,
            } => aspect_size(
                (horizontal_units.get(), vertical_units.get()),
                long_edge_pixels,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PixelSize {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutColor {
    pub color_space: ColorSpaceId,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrameDecoration {
    pub insets: NormalizedInsets,
    pub gap: NormalizedUnit,
    pub corner_radius: NormalizedUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<LayoutColor>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CellArrangement {
    One,
    HorizontalStack,
    VerticalStack,
    UniformGrid { columns: NonZeroU32 },
    Custom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuarterTurn {
    Zero,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CellContentMode {
    Fit { alignment: NormalizedPoint },
    Fill { focal_point: NormalizedPoint },
    Crop { source_rect: NormalizedRect },
}

impl Default for CellContentMode {
    fn default() -> Self {
        Self::Fit {
            alignment: NormalizedPoint::CENTER,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutCell {
    pub id: LayoutCellId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<AssetId>,
    pub content_mode: CellContentMode,
    pub quarter_turn: QuarterTurn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_rect: Option<NormalizedRect>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl LayoutCell {
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: LayoutCellId::new(),
            asset_id: None,
            content_mode: CellContentMode::default(),
            quarter_turn: QuarterTurn::Zero,
            custom_rect: None,
            extensions: BTreeMap::new(),
        }
    }
}

impl Default for LayoutCell {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutFrame {
    pub id: LayoutFrameId,
    pub arrangement: CellArrangement,
    pub decoration: FrameDecoration,
    pub cells: Vec<LayoutCell>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl LayoutFrame {
    #[must_use]
    pub fn one_cell() -> Self {
        Self {
            id: LayoutFrameId::new(),
            arrangement: CellArrangement::One,
            decoration: FrameDecoration::default(),
            cells: vec![LayoutCell::new()],
            extensions: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutState {
    pub schema_version: SchemaVersion,
    pub canvas: LayoutCanvas,
    pub frames: Vec<LayoutFrame>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl LayoutState {
    #[must_use]
    pub fn new(canvas: LayoutCanvas) -> Self {
        Self {
            schema_version: SchemaVersion::first(),
            canvas,
            frames: vec![LayoutFrame::one_cell()],
            extensions: BTreeMap::new(),
        }
    }

    /// Validates authored Layout semantics without consulting runtime services.
    ///
    /// # Errors
    ///
    /// Returns a structured error for invalid canvas, frames, cells, geometry,
    /// identities, or forbidden proxy/cache state.
    pub fn validate(&self) -> Result<(), LayoutValidationError> {
        if self.schema_version != SchemaVersion::first() {
            return Err(LayoutValidationError::UnsupportedStateSchema(
                self.schema_version.get(),
            ));
        }
        self.canvas.pixel_size()?;
        validate_extensions(&self.extensions)?;
        if self.frames.is_empty() {
            return Err(LayoutValidationError::NoFrames);
        }
        let mut frame_ids = BTreeSet::new();
        let mut cell_ids = BTreeSet::new();
        for frame in &self.frames {
            if !frame_ids.insert(frame.id) {
                return Err(LayoutValidationError::DuplicateFrame(frame.id));
            }
            validate_extensions(&frame.extensions)?;
            validate_extensions(&frame.decoration.extensions)?;
            validate_frame(frame, &mut cell_ids)?;
        }
        Ok(())
    }

    /// Encodes exact authored state for the node's ordinary schema boundary.
    ///
    /// # Errors
    ///
    /// Returns a validation or JSON serialization error.
    pub fn to_schema_value(&self) -> Result<SchemaValue, LayoutStateCodecError> {
        self.validate()?;
        Ok(SchemaValue {
            schema: layout_state_schema(),
            value: serde_json::to_value(self)?,
        })
    }

    /// Decodes and validates authored state from a node instance.
    ///
    /// # Errors
    ///
    /// Returns a schema, JSON, or semantic validation error.
    pub fn from_schema_value(value: &SchemaValue) -> Result<Self, LayoutStateCodecError> {
        if value.schema != layout_state_schema() {
            return Err(LayoutStateCodecError::WrongSchema {
                expected: layout_state_schema(),
                actual: value.schema.clone(),
            });
        }
        let state: Self = serde_json::from_value(value.value.clone())?;
        state.validate()?;
        Ok(state)
    }

    /// Canonical digest of authoritative Layout state only.
    ///
    /// # Errors
    ///
    /// Returns a validation or canonical JSON error.
    pub fn digest(&self) -> Result<CanonicalDigest, LayoutStateCodecError> {
        self.validate()?;
        Ok(canonical_digest(self)?)
    }

    #[must_use]
    pub fn diagnostics(&self, node_instance_id: NodeInstanceId) -> Vec<Diagnostic> {
        self.validate().err().map_or_else(Vec::new, |error| {
            vec![Diagnostic {
                code: error.code().to_owned(),
                severity: DiagnosticSeverity::Error,
                message: error.to_string(),
                node_instance_id: Some(node_instance_id),
                port_id: None,
            }]
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LayoutPlan {
    pub schema_version: SchemaVersion,
    pub canvas: PixelSize,
    pub frames: Vec<ResolvedFrame>,
}

impl LayoutPlan {
    /// Encodes this deterministic semantic plan as the node output value.
    ///
    /// # Errors
    ///
    /// Returns a JSON serialization error.
    pub fn to_typed_value(&self) -> Result<TypedValue, serde_json::Error> {
        Ok(TypedValue {
            value_type: layout_plan_value_type_ref(),
            value: serde_json::to_value(self)?,
        })
    }

    /// Canonical plan digest, independent of proxy/cache availability.
    ///
    /// # Errors
    ///
    /// Returns a canonical JSON error.
    pub fn digest(&self) -> Result<CanonicalDigest, serde_json::Error> {
        canonical_digest(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedFrame {
    pub id: LayoutFrameId,
    pub index: usize,
    pub decoration: FrameDecoration,
    pub cells: Vec<ResolvedCell>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedCell {
    pub id: LayoutCellId,
    pub index: usize,
    pub asset_id: Option<AssetId>,
    pub normalized_rect: NormalizedRect,
    pub pixel_rect: PixelRect,
    pub content_mode: CellContentMode,
    pub quarter_turn: QuarterTurn,
}

/// Resolves authored Layout and explicit `AssetSet` input without runtime I/O.
///
/// # Errors
///
/// Returns a validation error when state is invalid or a placement references
/// an asset absent from the explicit input.
pub fn resolve_layout(
    state: &LayoutState,
    assets: &AssetSet,
) -> Result<LayoutPlan, LayoutValidationError> {
    state.validate()?;
    let allowed_assets: BTreeSet<_> = assets.assets.iter().copied().collect();
    let canvas = state.canvas.pixel_size()?;
    let mut frames = Vec::with_capacity(state.frames.len());
    for (frame_index, frame) in state.frames.iter().enumerate() {
        let normalized = resolve_frame_cells(frame)?;
        let mut cells = Vec::with_capacity(frame.cells.len());
        for (cell_index, (cell, rect)) in frame.cells.iter().zip(normalized).enumerate() {
            if let Some(asset_id) = cell.asset_id
                && !allowed_assets.contains(&asset_id)
            {
                return Err(LayoutValidationError::AssetOutsideInput {
                    cell_id: cell.id,
                    asset_id,
                });
            }
            cells.push(ResolvedCell {
                id: cell.id,
                index: cell_index,
                asset_id: cell.asset_id,
                normalized_rect: rect,
                pixel_rect: pixel_rect(rect, canvas)?,
                content_mode: cell.content_mode,
                quarter_turn: cell.quarter_turn,
            });
        }
        frames.push(ResolvedFrame {
            id: frame.id,
            index: frame_index,
            decoration: frame.decoration.clone(),
            cells,
        });
    }
    Ok(LayoutPlan {
        schema_version: SchemaVersion::first(),
        canvas,
        frames,
    })
}

/// Ephemeral preview proxies keyed by semantic asset identity.
///
/// This runtime record is deliberately not serializable and cannot enter
/// `LayoutState` or `LayoutPlan`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutProxySet {
    pub artifacts: BTreeMap<AssetId, ProxyArtifact>,
}

/// Requests one shared proxy per distinct placed asset through project services.
///
/// # Errors
///
/// Returns a project proxy service error. Layout state and plan are unchanged.
pub fn request_layout_proxies(
    plan: &LayoutPlan,
    project_id: ProjectId,
    profile: &ProxyProfile,
    services: &dyn ProjectVisualProxyService,
) -> Result<LayoutProxySet, ProxyServiceError> {
    let asset_ids: BTreeSet<_> = plan
        .frames
        .iter()
        .flat_map(|frame| frame.cells.iter().filter_map(|cell| cell.asset_id))
        .collect();
    let mut artifacts = BTreeMap::new();
    for asset_id in asset_ids {
        let artifact = services.request_visual_proxy(&ProjectVisualProxyRequest {
            request_id: RequestId::new(),
            project_id,
            asset_id,
            profile: profile.clone(),
        })?;
        artifacts.insert(asset_id, artifact);
    }
    Ok(LayoutProxySet { artifacts })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutCommand {
    SetCanvas {
        canvas: LayoutCanvas,
    },
    InsertFrame {
        index: usize,
        frame: LayoutFrame,
    },
    RemoveFrame {
        frame_id: LayoutFrameId,
    },
    MoveFrame {
        frame_id: LayoutFrameId,
        to_index: usize,
    },
    ReplaceFrame {
        frame_id: LayoutFrameId,
        frame: LayoutFrame,
    },
    SetFrameArrangement {
        frame_id: LayoutFrameId,
        arrangement: CellArrangement,
    },
    InsertCell {
        frame_id: LayoutFrameId,
        index: usize,
        cell: LayoutCell,
    },
    RemoveCell {
        frame_id: LayoutFrameId,
        cell_id: LayoutCellId,
    },
    ReplaceCell {
        frame_id: LayoutFrameId,
        cell_id: LayoutCellId,
        cell: LayoutCell,
    },
    ReplaceDecoration {
        frame_id: LayoutFrameId,
        decoration: FrameDecoration,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutCommandResult {
    pub state: LayoutState,
    pub inverse: LayoutCommand,
}

/// Applies one semantic Layout edit and returns its exact inverse for undo.
///
/// # Errors
///
/// Returns an edit or validation error. The input state is never mutated.
#[allow(clippy::too_many_lines)]
pub fn apply_layout_command(
    state: &LayoutState,
    command: LayoutCommand,
) -> Result<LayoutCommandResult, LayoutCommandError> {
    state.validate()?;
    let mut updated = state.clone();
    let inverse = match command {
        LayoutCommand::SetCanvas { canvas } => {
            let old = std::mem::replace(&mut updated.canvas, canvas);
            LayoutCommand::SetCanvas { canvas: old }
        }
        LayoutCommand::InsertFrame { index, frame } => {
            if index > updated.frames.len() {
                return Err(LayoutCommandError::IndexOutOfBounds(index));
            }
            let frame_id = frame.id;
            updated.frames.insert(index, frame);
            LayoutCommand::RemoveFrame { frame_id }
        }
        LayoutCommand::RemoveFrame { frame_id } => {
            let index = frame_index(&updated, frame_id)?;
            let frame = updated.frames.remove(index);
            LayoutCommand::InsertFrame { index, frame }
        }
        LayoutCommand::MoveFrame { frame_id, to_index } => {
            if to_index >= updated.frames.len() {
                return Err(LayoutCommandError::IndexOutOfBounds(to_index));
            }
            let from_index = frame_index(&updated, frame_id)?;
            let frame = updated.frames.remove(from_index);
            updated.frames.insert(to_index, frame);
            LayoutCommand::MoveFrame {
                frame_id,
                to_index: from_index,
            }
        }
        LayoutCommand::ReplaceFrame { frame_id, frame } => {
            if frame.id != frame_id {
                return Err(LayoutCommandError::ReplacementFrameIdentity {
                    expected: frame_id,
                    actual: frame.id,
                });
            }
            let index = frame_index(&updated, frame_id)?;
            let old = std::mem::replace(&mut updated.frames[index], frame);
            LayoutCommand::ReplaceFrame {
                frame_id,
                frame: old,
            }
        }
        LayoutCommand::SetFrameArrangement {
            frame_id,
            arrangement,
        } => {
            let frame = frame_mut(&mut updated, frame_id)?;
            let old = std::mem::replace(&mut frame.arrangement, arrangement);
            LayoutCommand::SetFrameArrangement {
                frame_id,
                arrangement: old,
            }
        }
        LayoutCommand::InsertCell {
            frame_id,
            index,
            cell,
        } => {
            let frame = frame_mut(&mut updated, frame_id)?;
            if index > frame.cells.len() {
                return Err(LayoutCommandError::IndexOutOfBounds(index));
            }
            let cell_id = cell.id;
            frame.cells.insert(index, cell);
            LayoutCommand::RemoveCell { frame_id, cell_id }
        }
        LayoutCommand::RemoveCell { frame_id, cell_id } => {
            let frame = frame_mut(&mut updated, frame_id)?;
            let index = frame
                .cells
                .iter()
                .position(|cell| cell.id == cell_id)
                .ok_or(LayoutCommandError::UnknownCell(cell_id))?;
            let cell = frame.cells.remove(index);
            LayoutCommand::InsertCell {
                frame_id,
                index,
                cell,
            }
        }
        LayoutCommand::ReplaceCell {
            frame_id,
            cell_id,
            cell,
        } => {
            if cell.id != cell_id {
                return Err(LayoutCommandError::ReplacementCellIdentity {
                    expected: cell_id,
                    actual: cell.id,
                });
            }
            let frame = frame_mut(&mut updated, frame_id)?;
            let current = frame
                .cells
                .iter_mut()
                .find(|current| current.id == cell_id)
                .ok_or(LayoutCommandError::UnknownCell(cell_id))?;
            let old = std::mem::replace(current, cell);
            LayoutCommand::ReplaceCell {
                frame_id,
                cell_id,
                cell: old,
            }
        }
        LayoutCommand::ReplaceDecoration {
            frame_id,
            decoration,
        } => {
            let frame = frame_mut(&mut updated, frame_id)?;
            let old = std::mem::replace(&mut frame.decoration, decoration);
            LayoutCommand::ReplaceDecoration {
                frame_id,
                decoration: old,
            }
        }
    };
    updated.validate()?;
    Ok(LayoutCommandResult {
        state: updated,
        inverse,
    })
}

#[must_use]
/// Returns the built-in authored-state schema.
///
/// # Panics
///
/// Panics only if the compile-time built-in schema identifier is invalid.
pub fn layout_state_schema() -> SchemaRef {
    SchemaRef {
        id: SchemaId::parse(LAYOUT_STATE_SCHEMA_ID).expect("built-in schema ID is valid"),
        version: SchemaVersion::first(),
    }
}

#[must_use]
/// Returns the built-in Layout plan value type.
///
/// # Panics
///
/// Panics only if the compile-time built-in value-type identifier is invalid.
pub fn layout_plan_value_type_ref() -> ValueTypeRef {
    ValueTypeRef {
        id: ValueTypeId::parse(LAYOUT_PLAN_TYPE_ID).expect("built-in value type ID is valid"),
        version: ValueTypeVersion::first(),
    }
}

#[must_use]
/// Returns the built-in Layout plan descriptor.
///
/// # Panics
///
/// Panics only if a compile-time built-in identifier is invalid.
pub fn layout_plan_value_type_descriptor() -> ValueTypeDescriptor {
    ValueTypeDescriptor {
        value_type: layout_plan_value_type_ref(),
        display_name: "Layout Plan".to_owned(),
        schema: SchemaRef {
            id: SchemaId::parse(LAYOUT_PLAN_SCHEMA_ID).expect("built-in schema ID is valid"),
            version: SchemaVersion::first(),
        },
    }
}

fn validate_frame(
    frame: &LayoutFrame,
    cell_ids: &mut BTreeSet<LayoutCellId>,
) -> Result<(), LayoutValidationError> {
    if frame.cells.is_empty() {
        return Err(LayoutValidationError::FrameHasNoCells(frame.id));
    }
    if frame.arrangement == CellArrangement::One && frame.cells.len() != 1 {
        return Err(LayoutValidationError::OneArrangementCellCount {
            frame_id: frame.id,
            actual: frame.cells.len(),
        });
    }
    let horizontal_insets = frame
        .decoration
        .insets
        .left
        .get()
        .saturating_add(frame.decoration.insets.right.get());
    let vertical_insets = frame
        .decoration
        .insets
        .top
        .get()
        .saturating_add(frame.decoration.insets.bottom.get());
    if horizontal_insets >= NORMALIZED_SCALE || vertical_insets >= NORMALIZED_SCALE {
        return Err(LayoutValidationError::InsetsConsumeFrame(frame.id));
    }
    if frame.arrangement == CellArrangement::Custom && frame.decoration.gap != NormalizedUnit::ZERO
    {
        return Err(LayoutValidationError::CustomArrangementHasGap(frame.id));
    }
    for cell in &frame.cells {
        if !cell_ids.insert(cell.id) {
            return Err(LayoutValidationError::DuplicateCell(cell.id));
        }
        validate_extensions(&cell.extensions)?;
        match (frame.arrangement, cell.custom_rect) {
            (CellArrangement::Custom, Some(rect)) => rect.validate()?,
            (CellArrangement::Custom, None) => {
                return Err(LayoutValidationError::MissingCustomRect(cell.id));
            }
            (_, Some(_)) => return Err(LayoutValidationError::UnexpectedCustomRect(cell.id)),
            (_, None) => {}
        }
        if let CellContentMode::Crop { source_rect } = cell.content_mode {
            source_rect.validate()?;
        }
    }
    resolve_frame_cells(frame)?;
    Ok(())
}

fn resolve_frame_cells(frame: &LayoutFrame) -> Result<Vec<NormalizedRect>, LayoutValidationError> {
    let left = frame.decoration.insets.left.get();
    let top = frame.decoration.insets.top.get();
    let right = NORMALIZED_SCALE - frame.decoration.insets.right.get();
    let bottom = NORMALIZED_SCALE - frame.decoration.insets.bottom.get();
    let width = right - left;
    let height = bottom - top;
    let gap = frame.decoration.gap.get();
    let count = frame.cells.len();
    match frame.arrangement {
        CellArrangement::One => Ok(vec![raw_rect(left, top, width, height)?]),
        CellArrangement::HorizontalStack => (0..count)
            .map(|index| {
                let (start, extent) = partition(left, width, count, gap, index, frame.id)?;
                raw_rect(start, top, extent, height)
            })
            .collect(),
        CellArrangement::VerticalStack => (0..count)
            .map(|index| {
                let (start, extent) = partition(top, height, count, gap, index, frame.id)?;
                raw_rect(left, start, width, extent)
            })
            .collect(),
        CellArrangement::UniformGrid { columns } => {
            let columns =
                usize::try_from(columns.get()).expect("u32 fits usize on supported hosts");
            let rows = count.div_ceil(columns);
            (0..count)
                .map(|index| {
                    let column = index % columns;
                    let row = index / columns;
                    let (x, cell_width) = partition(left, width, columns, gap, column, frame.id)?;
                    let (y, cell_height) = partition(top, height, rows, gap, row, frame.id)?;
                    raw_rect(x, y, cell_width, cell_height)
                })
                .collect()
        }
        CellArrangement::Custom => frame
            .cells
            .iter()
            .map(|cell| {
                let rect = cell
                    .custom_rect
                    .ok_or(LayoutValidationError::MissingCustomRect(cell.id))?;
                raw_rect(
                    left + scale(rect.x.get(), width),
                    top + scale(rect.y.get(), height),
                    scale(rect.width.get(), width),
                    scale(rect.height.get(), height),
                )
            })
            .collect(),
    }
}

fn partition(
    start: u32,
    extent: u32,
    count: usize,
    gap: u32,
    index: usize,
    frame_id: LayoutFrameId,
) -> Result<(u32, u32), LayoutValidationError> {
    let gaps = u64::from(gap)
        * u64::try_from(count.saturating_sub(1)).expect("usize fits u64 on supported hosts");
    if gaps >= u64::from(extent) {
        return Err(LayoutValidationError::GapsConsumeFrame(frame_id));
    }
    let available = u64::from(extent) - gaps;
    let count = u64::try_from(count).expect("usize fits u64 on supported hosts");
    let index = u64::try_from(index).expect("usize fits u64 on supported hosts");
    let cell_start = u64::from(start) + index * u64::from(gap) + available * index / count;
    let cell_end = u64::from(start) + index * u64::from(gap) + available * (index + 1) / count;
    let cell_start = u32::try_from(cell_start).map_err(|_| LayoutValidationError::Overflow)?;
    let cell_extent = u32::try_from(cell_end - u64::from(cell_start))
        .map_err(|_| LayoutValidationError::Overflow)?;
    if cell_extent == 0 {
        return Err(LayoutValidationError::ZeroAreaCell);
    }
    Ok((cell_start, cell_extent))
}

fn raw_rect(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<NormalizedRect, LayoutValidationError> {
    let rect = NormalizedRect {
        x: NormalizedUnit::new(x)?,
        y: NormalizedUnit::new(y)?,
        width: NormalizedUnit::new(width)?,
        height: NormalizedUnit::new(height)?,
    };
    rect.validate()?;
    Ok(rect)
}

fn pixel_rect(rect: NormalizedRect, canvas: PixelSize) -> Result<PixelRect, LayoutValidationError> {
    let x = scale(rect.x.get(), canvas.width.get());
    let y = scale(rect.y.get(), canvas.height.get());
    let end_x = scale(
        rect.x.get().saturating_add(rect.width.get()),
        canvas.width.get(),
    );
    let end_y = scale(
        rect.y.get().saturating_add(rect.height.get()),
        canvas.height.get(),
    );
    let width =
        NonZeroU32::new(end_x.saturating_sub(x)).ok_or(LayoutValidationError::ZeroPixelCell)?;
    let height =
        NonZeroU32::new(end_y.saturating_sub(y)).ok_or(LayoutValidationError::ZeroPixelCell)?;
    Ok(PixelRect {
        x,
        y,
        width,
        height,
    })
}

fn scale(value: u32, extent: u32) -> u32 {
    u32::try_from(u64::from(value) * u64::from(extent) / u64::from(NORMALIZED_SCALE))
        .expect("normalized scaling fits u32")
}

fn aspect_size(
    aspect: (u32, u32),
    long_edge: NonZeroU32,
) -> Result<PixelSize, LayoutValidationError> {
    let (horizontal, vertical) = aspect;
    let long = u64::from(long_edge.get());
    let (width, height) = if horizontal >= vertical {
        (
            long,
            (long * u64::from(vertical) + u64::from(horizontal) / 2) / u64::from(horizontal),
        )
    } else {
        (
            (long * u64::from(horizontal) + u64::from(vertical) / 2) / u64::from(vertical),
            long,
        )
    };
    Ok(PixelSize {
        width: NonZeroU32::new(u32::try_from(width).map_err(|_| LayoutValidationError::Overflow)?)
            .ok_or(LayoutValidationError::Overflow)?,
        height: NonZeroU32::new(
            u32::try_from(height).map_err(|_| LayoutValidationError::Overflow)?,
        )
        .ok_or(LayoutValidationError::Overflow)?,
    })
}

fn frame_index(state: &LayoutState, frame_id: LayoutFrameId) -> Result<usize, LayoutCommandError> {
    state
        .frames
        .iter()
        .position(|frame| frame.id == frame_id)
        .ok_or(LayoutCommandError::UnknownFrame(frame_id))
}

fn frame_mut(
    state: &mut LayoutState,
    frame_id: LayoutFrameId,
) -> Result<&mut LayoutFrame, LayoutCommandError> {
    state
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or(LayoutCommandError::UnknownFrame(frame_id))
}

fn validate_extensions(extensions: &BTreeMap<String, Value>) -> Result<(), LayoutValidationError> {
    const FORBIDDEN: &[&str] = &[
        "proxy",
        "proxies",
        "proxy-cache-key",
        "proxy_cache_key",
        "proxy-path",
        "proxy_path",
        "cache",
        "thumbnail",
        "preview-path",
        "preview_path",
        "local-path",
        "local_path",
    ];
    if let Some(key) = extensions
        .keys()
        .find(|key| FORBIDDEN.contains(&key.as_str()))
    {
        return Err(LayoutValidationError::ForbiddenRuntimeState(key.clone()));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum LayoutValidationError {
    #[error("unsupported Layout state schema version {0}")]
    UnsupportedStateSchema(u32),
    #[error("unsupported bundled canvas profile version {0}")]
    UnsupportedCanvasProfileVersion(u32),
    #[error("normalized value {0} exceeds {NORMALIZED_SCALE}")]
    NormalizedOutOfRange(u32),
    #[error("normalized rectangle must have positive width and height")]
    EmptyNormalizedRect,
    #[error("normalized rectangle escapes its coordinate bounds")]
    NormalizedRectEscapesBounds,
    #[error("Layout must contain at least one frame")]
    NoFrames,
    #[error("duplicate Layout frame {0}")]
    DuplicateFrame(LayoutFrameId),
    #[error("duplicate Layout cell {0}")]
    DuplicateCell(LayoutCellId),
    #[error("Layout frame {0} must contain at least one cell")]
    FrameHasNoCells(LayoutFrameId),
    #[error("one-cell arrangement in frame {frame_id} has {actual} cells")]
    OneArrangementCellCount {
        frame_id: LayoutFrameId,
        actual: usize,
    },
    #[error("frame {0} insets consume the complete frame")]
    InsetsConsumeFrame(LayoutFrameId),
    #[error("frame {0} gaps consume the available arrangement extent")]
    GapsConsumeFrame(LayoutFrameId),
    #[error("custom arrangement frame {0} must use zero automatic gap")]
    CustomArrangementHasGap(LayoutFrameId),
    #[error("custom cell {0} has no normalized rectangle")]
    MissingCustomRect(LayoutCellId),
    #[error("non-custom cell {0} contains a custom rectangle")]
    UnexpectedCustomRect(LayoutCellId),
    #[error("cell {cell_id} references asset {asset_id} outside explicit AssetSet input")]
    AssetOutsideInput {
        cell_id: LayoutCellId,
        asset_id: AssetId,
    },
    #[error("normalized arrangement produced a zero-area cell")]
    ZeroAreaCell,
    #[error("canvas resolution produced a zero-pixel cell")]
    ZeroPixelCell,
    #[error("Layout geometry arithmetic overflow")]
    Overflow,
    #[error("authoritative Layout state contains forbidden runtime/cache field {0:?}")]
    ForbiddenRuntimeState(String),
}

impl LayoutValidationError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedStateSchema(_) => "photara.layout.unsupported-state-schema",
            Self::UnsupportedCanvasProfileVersion(_) => {
                "photara.layout.unsupported-canvas-profile-version"
            }
            Self::NormalizedOutOfRange(_)
            | Self::EmptyNormalizedRect
            | Self::NormalizedRectEscapesBounds
            | Self::InsetsConsumeFrame(_)
            | Self::GapsConsumeFrame(_)
            | Self::CustomArrangementHasGap(_)
            | Self::MissingCustomRect(_)
            | Self::UnexpectedCustomRect(_)
            | Self::ZeroAreaCell
            | Self::ZeroPixelCell
            | Self::Overflow => "photara.layout.invalid-geometry",
            Self::NoFrames
            | Self::DuplicateFrame(_)
            | Self::DuplicateCell(_)
            | Self::FrameHasNoCells(_)
            | Self::OneArrangementCellCount { .. } => "photara.layout.invalid-structure",
            Self::AssetOutsideInput { .. } => "photara.layout.asset-outside-input",
            Self::ForbiddenRuntimeState(_) => "photara.layout.forbidden-runtime-state",
        }
    }
}

#[derive(Debug, Error)]
pub enum LayoutStateCodecError {
    #[error(transparent)]
    Validation(#[from] LayoutValidationError),
    #[error("wrong Layout authored-state schema: expected {expected:?}, got {actual:?}")]
    WrongSchema {
        expected: SchemaRef,
        actual: SchemaRef,
    },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum LayoutCommandError {
    #[error(transparent)]
    Validation(#[from] LayoutValidationError),
    #[error("unknown Layout frame {0}")]
    UnknownFrame(LayoutFrameId),
    #[error("unknown Layout cell {0}")]
    UnknownCell(LayoutCellId),
    #[error("Layout index {0} is out of bounds")]
    IndexOutOfBounds(usize),
    #[error("replacement cell identity {actual} does not match target {expected}")]
    ReplacementCellIdentity {
        expected: LayoutCellId,
        actual: LayoutCellId,
    },
    #[error("replacement frame identity {actual} does not match target {expected}")]
    ReplacementFrameIdentity {
        expected: LayoutFrameId,
        actual: LayoutFrameId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assigned_cell(asset_id: AssetId) -> LayoutCell {
        LayoutCell {
            asset_id: Some(asset_id),
            ..LayoutCell::new()
        }
    }

    #[test]
    fn bundled_grid_resolves_exact_normalized_and_pixel_geometry() {
        let assets = [AssetId::new(), AssetId::new()];
        let mut frame = LayoutFrame {
            id: LayoutFrameId::new(),
            arrangement: CellArrangement::UniformGrid {
                columns: NonZeroU32::new(2).unwrap(),
            },
            decoration: FrameDecoration::default(),
            cells: vec![
                assigned_cell(assets[0]),
                assigned_cell(assets[1]),
                assigned_cell(assets[0]),
                assigned_cell(assets[1]),
            ],
            extensions: BTreeMap::new(),
        };
        frame.decoration.gap = NormalizedUnit::new(10_000).unwrap();
        let state = LayoutState {
            schema_version: SchemaVersion::first(),
            canvas: LayoutCanvas::portrait_3x4(NonZeroU32::new(4000).unwrap()),
            frames: vec![frame],
            extensions: BTreeMap::new(),
        };
        let asset_set = AssetSet {
            assets: assets.to_vec(),
        };

        let plan = resolve_layout(&state, &asset_set).unwrap();
        assert_eq!(plan.canvas.width.get(), 3000);
        assert_eq!(plan.canvas.height.get(), 4000);
        assert_eq!(plan.frames[0].cells[0].normalized_rect.width.get(), 495_000);
        assert_eq!(plan.frames[0].cells[1].normalized_rect.x.get(), 505_000);
        assert_eq!(plan.frames[0].cells[0].pixel_rect.width.get(), 1485);
        assert_eq!(plan.frames[0].cells[1].pixel_rect.x, 1515);
        assert_eq!(plan.frames[0].cells[2].asset_id, Some(assets[0]));
        assert_eq!(
            plan.digest().unwrap(),
            resolve_layout(&state, &asset_set)
                .unwrap()
                .digest()
                .unwrap()
        );
    }

    #[test]
    fn authored_commands_are_exactly_undoable_and_crops_are_independent() {
        let asset_id = AssetId::new();
        let mut state =
            LayoutState::new(LayoutCanvas::portrait_3x4(NonZeroU32::new(4000).unwrap()));
        let original = state.clone();
        let frame_id = state.frames[0].id;
        let cell_id = state.frames[0].cells[0].id;
        let mut cell = state.frames[0].cells[0].clone();
        cell.asset_id = Some(asset_id);
        cell.content_mode = CellContentMode::Crop {
            source_rect: NormalizedRect {
                x: NormalizedUnit::new(100_000).unwrap(),
                y: NormalizedUnit::new(50_000).unwrap(),
                width: NormalizedUnit::new(800_000).unwrap(),
                height: NormalizedUnit::new(900_000).unwrap(),
            },
        };
        cell.quarter_turn = QuarterTurn::Clockwise90;

        let edited = apply_layout_command(
            &state,
            LayoutCommand::ReplaceCell {
                frame_id,
                cell_id,
                cell,
            },
        )
        .unwrap();
        state = edited.state;
        assert_ne!(state.digest().unwrap(), original.digest().unwrap());
        let undone = apply_layout_command(&state, edited.inverse).unwrap();
        assert_eq!(undone.state, original);
    }

    #[test]
    fn explicit_asset_input_and_proxy_cache_fields_are_enforced() {
        let asset_id = AssetId::new();
        let mut state =
            LayoutState::new(LayoutCanvas::vertical_9x16(NonZeroU32::new(3840).unwrap()));
        state.frames[0].cells[0].asset_id = Some(asset_id);
        assert!(matches!(
            resolve_layout(&state, &AssetSet::default()),
            Err(LayoutValidationError::AssetOutsideInput { .. })
        ));

        state.extensions.insert(
            "proxy_cache_key".to_owned(),
            Value::String("derived".to_owned()),
        );
        assert!(matches!(
            state.validate(),
            Err(LayoutValidationError::ForbiddenRuntimeState(_))
        ));
    }
}
