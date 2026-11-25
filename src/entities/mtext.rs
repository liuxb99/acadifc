//! Multi-line text entity

use super::{Entity, EntityCommon, LineSpacingStyle};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector3};

/// Attachment point for MText
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachmentPoint {
    /// Top left
    TopLeft = 1,
    /// Top center
    TopCenter = 2,
    /// Top right
    TopRight = 3,
    /// Middle left
    MiddleLeft = 4,
    /// Middle center
    MiddleCenter = 5,
    /// Middle right
    MiddleRight = 6,
    /// Bottom left
    BottomLeft = 7,
    /// Bottom center
    BottomCenter = 8,
    /// Bottom right
    BottomRight = 9,
}

/// Drawing direction for MText
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DrawingDirection {
    /// Left to right
    LeftToRight = 1,
    /// Top to bottom (DXF 72 = 3 — the TEXT-era code set skips 2 and 4)
    TopToBottom = 3,
    /// By style (DXF 72 = 5)
    ByStyle = 5,
}

/// Column layout for an [`MText`] entity (stored in R2018+ DWG, non-annotative).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MTextColumnData {
    /// Column type: 0 = no columns, 1 = static columns, 2 = dynamic columns.
    pub column_type: i16,
    /// Number of columns. For dynamic, non-auto-height columns this is the
    /// number of per-column [`heights`](Self::heights); the DWG writer derives
    /// the on-disk count from `heights.len()` in that case to keep the object
    /// stream in sync, so keep the two consistent for dynamic columns.
    pub column_count: i32,
    /// Whether the column flow is reversed.
    pub flow_reversed: bool,
    /// Whether the column height is computed automatically.
    pub auto_height: bool,
    /// Column width.
    pub width: f64,
    /// Gutter width between columns.
    pub gutter: f64,
    /// Per-column heights. Only stored for dynamic, non-auto-height columns.
    pub heights: Vec<f64>,
}

impl MTextColumnData {
    /// Create empty (no-columns) column data.
    pub fn new() -> Self {
        MTextColumnData {
            column_type: 0,
            column_count: 0,
            flow_reversed: false,
            auto_height: false,
            width: 0.0,
            gutter: 0.0,
            heights: Vec::new(),
        }
    }
}

impl Default for MTextColumnData {
    fn default() -> Self {
        Self::new()
    }
}

/// A multi-line text entity
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MText {
    /// Common entity data
    pub common: EntityCommon,
    /// Text content (may contain formatting codes)
    pub value: String,
    /// Insertion point
    pub insertion_point: Vector3,
    /// Text height
    pub height: f64,
    /// Reference rectangle width
    pub rectangle_width: f64,
    /// Reference rectangle height (optional)
    pub rectangle_height: Option<f64>,
    /// Rotation angle in radians
    pub rotation: f64,
    /// Exact DWG X-axis direction used to derive `rotation`. Kept only while
    /// it still describes the public rotation, avoiding needless last-bit
    /// drift from an `atan2` followed by `sin`/`cos`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dwg_x_direction: Option<Vector3>,
    /// Text style name
    pub style: String,
    /// Attachment point
    pub attachment_point: AttachmentPoint,
    /// Drawing direction
    pub drawing_direction: DrawingDirection,
    /// Line spacing factor
    pub line_spacing_factor: f64,
    /// Line spacing style (DXF 73): AtLeast (1) or Exactly (2).
    pub line_spacing_style: LineSpacingStyle,
    /// Normal vector
    pub normal: Vector3,
    /// Background fill flags (BL 90): bit 0x01 = use background fill color,
    /// 0x02 = use drawing window color, 0x10 = text frame (R2018+).
    pub background_fill_flags: i32,
    /// Background fill scale factor (BD 45). Default 1.5.
    pub background_scale: f64,
    /// Background fill color (CMC 63).
    pub background_color: Color,
    /// Background fill transparency (BL 441).
    pub background_transparency: i32,
    /// Whether this MTEXT is annotative (R2018+). When `false`, the DWG stores
    /// a block of redundant fields followed by column data.
    pub is_annotative: bool,
    /// Column layout data (R2018+).
    pub column_data: MTextColumnData,
    /// Horizontal extent of the laid-out text (DXF 42, output-only; DWG
    /// "extents width"). 0 when never laid out.
    #[cfg_attr(feature = "serde", serde(default))]
    pub extents_width: f64,
    /// Vertical extent of the laid-out text (DXF 43, output-only; DWG
    /// "extents height"). 0 when never laid out.
    #[cfg_attr(feature = "serde", serde(default))]
    pub extents_height: f64,
    /// Original DWG STYLE handle. Kept even when the style name cannot be
    /// resolved back to a table entry, so a no-op save never writes NULL.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub style_handle: Option<crate::types::Handle>,
    /// Set when the caller changes `style`; the writer then resolves the new
    /// name instead of reusing the original handle.
    #[cfg_attr(feature = "serde", serde(default, skip))]
    pub style_dirty: bool,
    /// Set when rotation or OCS normal changes; the writer must recompute the
    /// X direction in the (possibly non-default) OCS instead of reusing the
    /// direction captured from the source record.
    #[cfg_attr(feature = "serde", serde(default, skip))]
    pub direction_dirty: bool,
    /// Uninterpreted bit read from the R2000+ MTEXT record. Preserved verbatim
    /// on same-version writes; its semantics are intentionally not guessed.
    #[cfg_attr(feature = "serde", serde(default, skip))]
    pub dwg_unknown_bit: bool,
    /// Source semantic snapshot. These are set by the DWG reader and compared
    /// by the writer, so direct field assignment, serde and clone/copy cannot
    /// bypass change detection.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_rotation: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_normal: Option<Vector3>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_style: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_value: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_height: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_rect_width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_line_spacing: Option<f64>,
    /// True when text/layout changed and the cached extents no longer describe
    /// the authored text. The writer writes explicit zero (invalid) extents
    /// instead of guessing glyph sizes from an unavailable font.
    #[cfg_attr(feature = "serde", serde(default))]
    pub extents_dirty: bool,
}

impl PartialEq for MText {
    /// Compare the authored/semantic state. The original STYLE handle, its
    /// dirty flag and the cached X-direction dirty flag are round-trip
    /// bookkeeping, not authored values; asserting equality on them would
    /// make every freshly written MTEXT differ from the one read back.
    fn eq(&self, other: &Self) -> bool {
        self.common == other.common
            && self.value == other.value
            && self.insertion_point == other.insertion_point
            && self.height == other.height
            && self.rectangle_width == other.rectangle_width
            && self.rectangle_height == other.rectangle_height
            && self.rotation == other.rotation
            && self.dwg_x_direction == other.dwg_x_direction
            && self.style == other.style
            && self.attachment_point == other.attachment_point
            && self.drawing_direction == other.drawing_direction
            && self.line_spacing_factor == other.line_spacing_factor
            && self.line_spacing_style == other.line_spacing_style
            && self.normal == other.normal
            && self.background_fill_flags == other.background_fill_flags
            && self.background_scale == other.background_scale
            && self.background_color == other.background_color
            && self.background_transparency == other.background_transparency
            && self.is_annotative == other.is_annotative
            && self.column_data == other.column_data
            && self.extents_width == other.extents_width
            && self.extents_height == other.extents_height
            && self.dwg_unknown_bit == other.dwg_unknown_bit
    }
}

impl MText {
    /// Create a new MText entity
    pub fn new() -> Self {
        MText {
            common: EntityCommon::new(),
            value: String::new(),
            insertion_point: Vector3::ZERO,
            height: 1.0,
            rectangle_width: 10.0,
            rectangle_height: None,
            rotation: 0.0,
            dwg_x_direction: None,
            style: "Standard".to_string(),
            attachment_point: AttachmentPoint::TopLeft,
            drawing_direction: DrawingDirection::LeftToRight,
            line_spacing_factor: 1.0,
            line_spacing_style: LineSpacingStyle::AtLeast,
            normal: Vector3::UNIT_Z,
            background_fill_flags: 0,
            background_scale: 1.5,
            background_color: Color::ByLayer,
            background_transparency: 0,
            // Non-annotative by default. Real annotativeness is carried by the
            // annotation context / text style (and, from R2018 DWG on, an inline
            // entity bit the reader sets explicitly) — not by every fresh MTEXT.
            // DXF has no entity-level annotative flag at all, so without this
            // default it would mark every imported MTEXT annotative.
            is_annotative: false,
            column_data: MTextColumnData::new(),
            extents_width: 0.0,
            extents_height: 0.0,
            style_handle: None,
            style_dirty: false,
            direction_dirty: false,
            dwg_unknown_bit: false,
            source_rotation: None,
            source_normal: None,
            source_style: None,
            source_value: None,
            source_height: None,
            source_rect_width: None,
            source_line_spacing: None,
            extents_dirty: false,
        }
    }

    /// Create a new MText with value and position
    pub fn with_value(value: impl Into<String>, position: Vector3) -> Self {
        MText {
            value: value.into(),
            insertion_point: position,
            ..Self::new()
        }
    }

    /// Set the text height
    pub fn with_height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    /// Set the rectangle width
    pub fn with_width(mut self, width: f64) -> Self {
        self.rectangle_width = width;
        self
    }

    /// Change rotation and mark the cached DWG X direction stale.
    pub fn set_rotation(&mut self, rotation: f64) {
        self.rotation = rotation;
        self.direction_dirty = true;
    }

    /// Change the OCS normal and mark the cached DWG X direction stale.
    pub fn set_normal(&mut self, normal: Vector3) {
        self.normal = normal;
        self.direction_dirty = true;
    }

    /// Change the text style and mark the original STYLE handle stale.
    pub fn set_style(&mut self, style: impl Into<String>) {
        self.style = style.into();
        self.style_dirty = true;
        self.extents_dirty = true;
    }

    /// Change the authored text; cached extents become invalid.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.extents_dirty = true;
    }

    /// Change text height; cached extents become invalid.
    pub fn set_height(&mut self, height: f64) {
        self.height = height;
        self.extents_dirty = true;
    }

    /// Change reference width; cached extents become invalid.
    pub fn set_rectangle_width(&mut self, width: f64) {
        self.rectangle_width = width;
        self.extents_dirty = true;
    }

    /// Change line spacing; cached extents become invalid.
    pub fn set_line_spacing_factor(&mut self, factor: f64) {
        self.line_spacing_factor = factor;
        self.extents_dirty = true;
    }

    /// Whether the cached extents still describe the authored text.
    pub fn extents_are_valid(&self) -> bool {
        // Only an explicit, authorized mutation invalidates cached extents.
        !self.extents_dirty
    }
}

impl Default for MText {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for MText {
    fn handle(&self) -> Handle {
        self.common.handle
    }

    fn set_handle(&mut self, handle: Handle) {
        self.common.handle = handle;
    }

    fn layer(&self) -> &str {
        &self.common.layer
    }

    fn set_layer(&mut self, layer: String) {
        self.common.layer = layer;
    }

    fn color(&self) -> Color {
        self.common.color
    }

    fn set_color(&mut self, color: Color) {
        self.common.color = color;
    }

    fn line_weight(&self) -> LineWeight {
        self.common.line_weight
    }

    fn set_line_weight(&mut self, weight: LineWeight) {
        self.common.line_weight = weight;
    }

    fn transparency(&self) -> Transparency {
        self.common.transparency
    }

    fn set_transparency(&mut self, transparency: Transparency) {
        self.common.transparency = transparency;
    }

    fn is_invisible(&self) -> bool {
        self.common.invisible
    }

    fn set_invisible(&mut self, invisible: bool) {
        self.common.invisible = invisible;
    }

    fn bounding_box(&self) -> BoundingBox3D {
        let height = self.rectangle_height.unwrap_or(self.height * 2.0);
        BoundingBox3D::new(
            self.insertion_point,
            Vector3::new(
                self.insertion_point.x + self.rectangle_width,
                self.insertion_point.y + height,
                self.insertion_point.z,
            ),
        )
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_mtext(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "MTEXT"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_mtext(self, transform);
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_mtext(self, transform);
    }
}

