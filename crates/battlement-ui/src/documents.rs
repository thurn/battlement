use battlement_types::{Color, ObjectId, ScreenSize};
use serde::{Deserialize, Serialize};

use crate::{Style, UiNode, VisualElement};

/// A logical UI document authored in Rust and rendered by a Unity `UIDocument`.
///
/// The document owns the root of a [`UiNode`] hierarchy and identifies the
/// Unity GameObject whose [`UiDocumentState`] supplies host-side panel and
/// placement settings. Root name, style, and children are applied to the
/// `UIDocument.rootVisualElement`; they do not describe the host GameObject.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiDocument {
    /// Unity GameObject hosting the `UIDocument`.
    pub document_id: ObjectId,
    /// Stable identity of the native document root.
    pub root_id: ObjectId,
    /// Sparse visual properties applied to the native document root.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Logical root children in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiNode>,
}

impl UiDocument {
    /// Creates a document for `document_id` with a newly generated root identity.
    #[must_use]
    pub fn new(document_id: ObjectId) -> Self {
        Self::with_root_id(document_id, ObjectId::new_v4())
    }

    /// Creates a document with explicit host GameObject and visual-root identities.
    ///
    /// Preserve both identities across snapshots so Unity can reconcile the
    /// logical document and its root without replacing them unnecessarily.
    #[must_use]
    pub fn with_root_id(document_id: ObjectId, root_id: ObjectId) -> Self {
        Self {
            document_id,
            root_id,
            element: VisualElement::default(),
            children: Vec::new(),
        }
    }

    /// Assigns the root element name used by name-based queries and USS ID selectors.
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.element.name = Some(value.into());
        self
    }

    /// Enables or disables interaction for the complete document hierarchy.
    #[must_use]
    pub fn enabled(mut self, value: bool) -> Self {
        self.element.enabled = Some(value);
        self
    }

    /// Appends one USS class to the root element.
    #[must_use]
    pub fn class(mut self, value: impl Into<String>) -> Self {
        self.element
            .classes
            .get_or_insert_with(Vec::new)
            .push(value.into());
        self
    }

    /// Requests forwarding for each supplied event kind on the document root.
    #[must_use]
    pub fn events(mut self, values: impl IntoIterator<Item = crate::UiEventKind>) -> Self {
        self.element
            .events
            .get_or_insert_with(Vec::new)
            .extend(values);
        self
    }

    /// Replaces the root element's inline style overrides.
    ///
    /// Unset style fields remain controlled by USS, inheritance, or Unity defaults.
    #[must_use]
    pub fn style(mut self, value: Style) -> Self {
        self.element.style = value;
        self
    }

    /// Appends one logical child after the document root's existing children.
    #[must_use]
    pub fn child(mut self, value: UiNode) -> Self {
        self.children.push(value);
        self
    }

    /// Appends logical children in iterator order after existing root children.
    #[must_use]
    pub fn children(mut self, values: impl IntoIterator<Item = UiNode>) -> Self {
        self.children.extend(values);
        self
    }

    /// Appends a logical child when `value` is present.
    #[must_use]
    pub fn optional_child(mut self, value: Option<UiNode>) -> Self {
        if let Some(value) = value {
            self.children.push(value);
        }
        self
    }

    /// Converts this document root and its hierarchy into the canonical node value.
    #[must_use]
    pub fn into_root_node(self) -> UiNode {
        UiNode {
            object_id: self.root_id,
            element: self.element.into(),
            children: self.children,
        }
    }

    /// Appends logical children in iterator order when `condition` is true.
    #[must_use]
    pub fn children_if(
        mut self,
        condition: bool,
        values: impl IntoIterator<Item = UiNode>,
    ) -> Self {
        if condition {
            self.children.extend(values);
        }
        self
    }
}

/// Host configuration for a Battlement-created Unity `UIDocument` GameObject.
///
/// This state controls the native panel, placement, world-space geometry, and
/// draw order of the host. It is intentionally distinct from [`UiDocument`],
/// which contains the logical visual hierarchy rendered inside that host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiDocumentState {
    /// Identity that links this host state to the matching logical document root.
    pub(crate) root_id: ObjectId,
    /// Rendering and scaling configuration copied to the document's private
    /// runtime panel, preventing unrelated documents from sharing mutable state.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub panel_settings: PanelSettings,
    /// Determines whether the document participates in normal layout positioning
    /// or is positioned independently.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub position: DocumentPosition,
    /// Determines whether world-space dimensions are fixed or derived dynamically.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub world_space_size_mode: WorldSpaceSizeMode,
    /// Width and height used when [`WorldSpaceSizeMode::Fixed`] controls a
    /// world-space document.
    #[serde(
        default = "default_world_size",
        skip_serializing_if = "is_default_world_size"
    )]
    pub world_space_size: ScreenSize,
    /// Geometry Unity uses as the reference frame when locating a world-space pivot.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub pivot_reference_size: PivotReferenceSize,
    /// Anchor point placed at the host transform for a world-space document.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub pivot: DocumentPivot,
    /// Draw-order priority among panels in the same rendering context; larger
    /// values render above smaller values.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub sorting_order: i32,
}

impl UiDocumentState {
    /// Creates screen-space host state with the protocol's default panel settings.
    #[must_use]
    pub fn new(root_id: ObjectId) -> Self {
        Self {
            root_id,
            panel_settings: PanelSettings::default(),
            position: DocumentPosition::default(),
            world_space_size_mode: WorldSpaceSizeMode::default(),
            world_space_size: default_world_size(),
            pivot_reference_size: PivotReferenceSize::default(),
            pivot: DocumentPivot::default(),
            sorting_order: 0,
        }
    }

    /// Returns the identity of the document's visual root.
    #[must_use]
    pub fn root_id(&self) -> ObjectId {
        self.root_id
    }

    /// Replaces the rendering and scaling configuration of the private runtime panel.
    #[must_use]
    pub fn panel_settings(mut self, value: PanelSettings) -> Self {
        self.panel_settings = value;
        self
    }

    /// Selects whether the document uses relative or absolute positioning.
    #[must_use]
    pub fn position(mut self, value: DocumentPosition) -> Self {
        self.position = value;
        self
    }

    /// Selects whether world-space dimensions are fixed or dynamically derived.
    #[must_use]
    pub fn world_space_size_mode(mut self, value: WorldSpaceSizeMode) -> Self {
        self.world_space_size_mode = value;
        self
    }

    /// Sets the width and height used by fixed-size world-space documents.
    #[must_use]
    pub fn world_space_size(mut self, value: ScreenSize) -> Self {
        self.world_space_size = value;
        self
    }

    /// Selects which document geometry is used to calculate the world-space pivot.
    #[must_use]
    pub fn pivot_reference_size(mut self, value: PivotReferenceSize) -> Self {
        self.pivot_reference_size = value;
        self
    }

    /// Selects the point on the document aligned with its host transform.
    #[must_use]
    pub fn pivot(mut self, value: DocumentPivot) -> Self {
        self.pivot = value;
        self
    }

    /// Sets draw-order priority relative to other panels in the same context.
    #[must_use]
    pub fn sorting_order(mut self, value: i32) -> Self {
        self.sorting_order = value;
        self
    }
}

/// Configures how a Unity UI Toolkit panel is rendered, scaled, and cleared.
///
/// Each [`UiDocumentState`] receives a private runtime copy of these settings,
/// so applying a snapshot cannot mutate a shared project asset. Scaling fields
/// are interpreted according to [`PanelScaleMode`], and screen matching fields
/// apply only when scaling with screen size.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelSettings {
    /// Determines whether the panel is composited over a display or rendered in
    /// world space from the document transform.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub render_mode: PanelRenderMode,
    /// Determines how authored UI lengths are converted to rendered pixels.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub scale_mode: PanelScaleMode,
    /// Number of sprite pixels corresponding to one UI unit when Unity resolves
    /// sprite dimensions.
    #[serde(default = "default_hundred", skip_serializing_if = "is_hundred")]
    pub reference_sprite_pixels_per_unit: f32,
    /// Uniform multiplier applied when [`PanelScaleMode::ConstantPixelSize`] is active.
    #[serde(default = "default_one", skip_serializing_if = "is_one")]
    pub scale: f32,
    /// Design density used to convert physical units when
    /// [`PanelScaleMode::ConstantPhysicalSize`] is active.
    #[serde(default = "default_dpi", skip_serializing_if = "is_dpi")]
    pub reference_dpi: f32,
    /// Density used for physical-size scaling when the target display does not
    /// report a usable DPI.
    #[serde(default = "default_dpi", skip_serializing_if = "is_dpi")]
    pub fallback_dpi: f32,
    /// Design resolution compared with the target display when
    /// [`PanelScaleMode::ScaleWithScreenSize`] is active.
    #[serde(
        default = "default_reference_resolution",
        skip_serializing_if = "is_default_reference_resolution"
    )]
    pub reference_resolution: ScreenSize,
    /// Chooses how target width and height contribute to screen-size scaling.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub screen_match_mode: PanelScreenMatchMode,
    /// Interpolation between width-based scaling (`0`) and height-based scaling
    /// (`1`) for [`PanelScreenMatchMode::MatchWidthOrHeight`].
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub match_factor: f32,
    /// Zero-based Unity display index on which a screen-space overlay panel renders.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub target_display: u32,
    /// Whether Unity clears the panel's depth and stencil buffers before rendering.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub clear_depth_stencil: bool,
    /// Whether Unity clears the panel color buffer before rendering UI content.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub clear_color: bool,
    /// Color written by the clear operation when [`Self::clear_color`] is enabled.
    #[serde(default = "transparent", skip_serializing_if = "is_transparent")]
    pub color_clear_value: Color,
    /// Allocation limits and eligibility filters for textures cached in the
    /// panel's dynamic atlas.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub dynamic_atlas: DynamicAtlasSettings,
}

impl PanelSettings {
    /// Creates panel settings with the protocol's Unity-compatible defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects display-overlay or transform-based world-space rendering.
    #[must_use]
    pub fn render_mode(mut self, value: PanelRenderMode) -> Self {
        self.render_mode = value;
        self
    }

    /// Selects how authored UI lengths are converted to rendered pixels.
    #[must_use]
    pub fn scale_mode(mut self, value: PanelScaleMode) -> Self {
        self.scale_mode = value;
        self
    }

    /// Sets how many sprite pixels correspond to one UI unit.
    #[must_use]
    pub fn reference_sprite_pixels_per_unit(mut self, value: f32) -> Self {
        self.reference_sprite_pixels_per_unit = value;
        self
    }

    /// Sets the uniform multiplier used by constant-pixel-size scaling.
    #[must_use]
    pub fn scale(mut self, value: f32) -> Self {
        self.scale = value;
        self
    }

    /// Sets the design density used by constant-physical-size scaling.
    #[must_use]
    pub fn reference_dpi(mut self, value: f32) -> Self {
        self.reference_dpi = value;
        self
    }

    /// Sets the density used when the target display does not report a usable DPI.
    #[must_use]
    pub fn fallback_dpi(mut self, value: f32) -> Self {
        self.fallback_dpi = value;
        self
    }

    /// Sets the design resolution used by scale-with-screen-size mode.
    #[must_use]
    pub fn reference_resolution(mut self, value: ScreenSize) -> Self {
        self.reference_resolution = value;
        self
    }

    /// Selects how target width and height determine screen-size scaling.
    #[must_use]
    pub fn screen_match_mode(mut self, value: PanelScreenMatchMode) -> Self {
        self.screen_match_mode = value;
        self
    }

    /// Sets the width-to-height interpolation factor used by
    /// [`PanelScreenMatchMode::MatchWidthOrHeight`].
    #[must_use]
    pub fn match_factor(mut self, value: f32) -> Self {
        self.match_factor = value;
        self
    }

    /// Selects the zero-based Unity display for a screen-space overlay panel.
    #[must_use]
    pub fn target_display(mut self, value: u32) -> Self {
        self.target_display = value;
        self
    }

    /// Enables or disables clearing the panel's depth and stencil buffers.
    #[must_use]
    pub fn clear_depth_stencil(mut self, value: bool) -> Self {
        self.clear_depth_stencil = value;
        self
    }

    /// Enables or disables clearing the panel color buffer before rendering.
    #[must_use]
    pub fn clear_color(mut self, value: bool) -> Self {
        self.clear_color = value;
        self
    }

    /// Sets the color written when panel color clearing is enabled.
    #[must_use]
    pub fn color_clear_value(mut self, value: Color) -> Self {
        self.color_clear_value = value;
        self
    }

    /// Replaces the panel's dynamic-atlas allocation limits and filters.
    #[must_use]
    pub fn dynamic_atlas(mut self, value: DynamicAtlasSettings) -> Self {
        self.dynamic_atlas = value;
        self
    }
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            render_mode: PanelRenderMode::default(),
            scale_mode: PanelScaleMode::default(),
            reference_sprite_pixels_per_unit: default_hundred(),
            scale: default_one(),
            reference_dpi: default_dpi(),
            fallback_dpi: default_dpi(),
            reference_resolution: default_reference_resolution(),
            screen_match_mode: PanelScreenMatchMode::default(),
            match_factor: 0.0,
            target_display: 0,
            clear_depth_stencil: true,
            clear_color: false,
            color_clear_value: transparent(),
            dynamic_atlas: DynamicAtlasSettings::default(),
        }
    }
}

/// Controls allocation and texture eligibility for a panel's dynamic atlas.
///
/// UI Toolkit can batch eligible textures into an atlas to reduce state changes
/// while rendering. These limits bound atlas growth and prevent unsuitable
/// textures from being inserted; excluded textures remain independently bound.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DynamicAtlasSettings {
    /// Minimum width and height, in pixels, allocated for a new dynamic atlas.
    pub min_atlas_size: u32,
    /// Maximum width and height, in pixels, to which the dynamic atlas may grow.
    pub max_atlas_size: u32,
    /// Maximum width or height, in pixels, of a texture eligible for insertion.
    pub max_sub_texture_size: u32,
    /// Filters Unity evaluates to exclude textures that should not enter the atlas.
    pub filters: Vec<DynamicAtlasFilter>,
}

impl Default for DynamicAtlasSettings {
    fn default() -> Self {
        Self {
            min_atlas_size: 64,
            max_atlas_size: 4096,
            max_sub_texture_size: 64,
            filters: vec![
                DynamicAtlasFilter::Readability,
                DynamicAtlasFilter::Size,
                DynamicAtlasFilter::Format,
                DynamicAtlasFilter::ColorSpace,
                DynamicAtlasFilter::FilterMode,
            ],
        }
    }
}

/// Panel rendering mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelRenderMode {
    /// Renders over the selected display.
    #[default]
    ScreenSpaceOverlay,
    /// Renders in the scene from the document transform.
    WorldSpace,
}
/// Panel scaling strategy.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelScaleMode {
    /// Uses authored pixel sizes and a constant scale.
    ConstantPixelSize,
    /// Converts physical measurements using display DPI.
    #[default]
    ConstantPhysicalSize,
    /// Scales relative to a reference resolution.
    ScaleWithScreenSize,
}
/// Reference-resolution matching strategy.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelScreenMatchMode {
    /// Interpolates between reference width and height.
    #[default]
    MatchWidthOrHeight,
    /// Uses the smaller scale factor.
    Shrink,
    /// Uses the larger scale factor.
    Expand,
}
/// Dynamic atlas exclusion filter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum DynamicAtlasFilter {
    /// Excludes readable textures.
    Readability,
    /// Excludes textures outside atlas size limits.
    Size,
    /// Excludes unsupported texture formats.
    Format,
    /// Excludes textures with incompatible color space.
    ColorSpace,
    /// Excludes textures with incompatible filtering.
    FilterMode,
}
/// UI document position mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DocumentPosition {
    /// Participates in ordinary layout positioning.
    #[default]
    Relative,
    /// Uses absolute positioning.
    Absolute,
}
/// World-space sizing strategy.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum WorldSpaceSizeMode {
    /// Uses the authored world-space size.
    #[default]
    Fixed,
    /// Derives the world-space size dynamically.
    Dynamic,
}
/// Reference used for world-space pivot calculations.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PivotReferenceSize {
    /// Uses the document bounding box.
    #[default]
    BoundingBox,
    /// Uses the layout size.
    Layout,
}
/// World-space document pivot.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DocumentPivot {
    /// Top-left pivot.
    TopLeft,
    /// Top-center pivot.
    TopCenter,
    /// Top-right pivot.
    TopRight,
    /// Middle-left pivot.
    MiddleLeft,
    /// Center pivot.
    #[default]
    Center,
    /// Middle-right pivot.
    MiddleRight,
    /// Bottom-left pivot.
    BottomLeft,
    /// Bottom-center pivot.
    BottomCenter,
    /// Bottom-right pivot.
    BottomRight,
}

fn default_world_size() -> ScreenSize {
    ScreenSize::new(1920, 1080)
}
fn is_default_world_size(value: &ScreenSize) -> bool {
    *value == default_world_size()
}
fn default_reference_resolution() -> ScreenSize {
    ScreenSize::new(1200, 800)
}
fn is_default_reference_resolution(value: &ScreenSize) -> bool {
    *value == default_reference_resolution()
}
fn default_hundred() -> f32 {
    100.0
}
fn is_hundred(value: &f32) -> bool {
    *value == default_hundred()
}
fn default_one() -> f32 {
    1.0
}
fn is_one(value: &f32) -> bool {
    *value == 1.0
}
fn default_dpi() -> f32 {
    96.0
}
fn is_dpi(value: &f32) -> bool {
    *value == default_dpi()
}
fn default_true() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
fn transparent() -> Color {
    Color::rgba(0.0, 0.0, 0.0, 0.0)
}
fn is_transparent(value: &Color) -> bool {
    *value == transparent()
}
