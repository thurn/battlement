use battlement_types::{Color, ObjectId, ScreenSize};
use serde::{Deserialize, Serialize};

use crate::{LanguageDirection, PickingMode, Style, UiNode, VisualElement};

/// A logical UI document authored in Rust and rendered by a Unity `UIDocument`.
///
/// The document owns the root of a [`UiNode`] hierarchy and identifies the
/// Unity GameObject whose [`UiDocumentState`] supplies host-side panel and
/// placement settings. Root name, style, and children are applied to the
/// `UIDocument.rootVisualElement`; they do not describe the host GameObject.
///
/// All documents in a snapshot share one identity namespace with their host
/// objects, roots, and descendants. Preserve `document_id`, `root_id`, and node
/// identities across snapshots when they represent the same logical objects.
///
/// See Unity's [`UIDocument` reference](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.UIDocument.html)
/// for the native component and root visual element it owns.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Label, UiDocument, UiNode};
///
/// let document_id = ObjectId::new_v4();
/// let document = UiDocument::new(document_id)
///     .name("hud")
///     .class("game-hud")
///     .child(UiNode::new(ObjectId::new_v4(), Label::new("Score: 0")));
///
/// assert_eq!(document.document_id, document_id);
/// assert_eq!(document.children.len(), 1);
/// ```
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiDocument {
    /// Unity GameObject that hosts the matching `UIDocument` component.
    pub document_id: ObjectId,
    /// Stable identity assigned to `UIDocument.rootVisualElement`.
    pub root_id: ObjectId,
    /// Name, enabled state, classes, style, and subscriptions for the native root.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Logical root children in native insertion and layout order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiNode>,
}

impl UiDocument {
    /// Creates a document for `document_id` with a newly generated root identity.
    ///
    /// Use [`Self::with_root_id`] when deterministic fixtures or persisted state
    /// already own the root identity.
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

    /// Sets the root name used by Unity queries and `#name` USS selectors.
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.element.name = Some(value.into());
        self
    }

    /// Sets the root's local enabled state, thereby gating its complete hierarchy.
    #[must_use]
    pub fn enabled(mut self, value: bool) -> Self {
        self.element.enabled = Some(value);
        self
    }

    /// Sets whether pointer hit testing may select the document root.
    ///
    /// Ignoring the root does not prevent independently pickable descendants
    /// from receiving pointer events.
    #[must_use]
    pub fn picking_mode(mut self, value: PickingMode) -> Self {
        self.element.picking_mode = Some(value);
        self
    }

    /// Sets text directionality inherited by the document hierarchy.
    ///
    /// This affects text direction rather than flex layout order.
    #[must_use]
    pub fn language_direction(mut self, value: LanguageDirection) -> Self {
        self.element.language_direction = Some(value);
        self
    }

    /// Sets whether the document root may receive focus.
    ///
    /// The root must also be enabled and accepted by Unity's focus controller.
    #[must_use]
    pub fn focusable(mut self, value: bool) -> Self {
        self.element.focusable = Some(value);
        self
    }

    /// Sets the document root's position in keyboard focus-ring ordering.
    ///
    /// Negative values remove the root from tab navigation without disabling
    /// programmatic focus eligibility.
    #[must_use]
    pub fn tab_index(mut self, value: i32) -> Self {
        self.element.tab_index = Some(value);
        self
    }

    /// Sets whether focus requested on the root transfers to a descendant.
    ///
    /// Unity selects the first eligible descendant in focus-ring order.
    #[must_use]
    pub fn delegates_focus(mut self, value: bool) -> Self {
        self.element.delegates_focus = Some(value);
        self
    }

    /// Appends one USS class name used to style the document root.
    ///
    /// Empty or duplicate class names make the document invalid.
    #[must_use]
    pub fn class(mut self, value: impl Into<String>) -> Self {
        self.element
            .classes
            .get_or_insert_with(Vec::new)
            .push(value.into());
        self
    }

    /// Subscribes Rust to supplied native event kinds reaching the document root.
    ///
    /// Repeated calls append subscriptions; duplicate kinds make the document
    /// invalid.
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

    /// Appends one logical child after the root's existing children.
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

    /// Appends `value` when present and otherwise leaves the hierarchy unchanged.
    #[must_use]
    pub fn optional_child(mut self, value: Option<UiNode>) -> Self {
        if let Some(value) = value {
            self.children.push(value);
        }
        self
    }

    /// Converts the visual root and its children into a canonical [`UiNode`].
    ///
    /// The returned node uses `root_id`; host-only `document_id` is intentionally
    /// not represented.
    #[must_use]
    pub fn into_root_node(self) -> UiNode {
        UiNode {
            object_id: self.root_id,
            element: self.element.into(),
            children: self.children,
        }
    }

    /// Appends `values` in iterator order only when `condition` is true.
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
/// Changing this create-time state requires recreating the host GameObject;
/// visual-element update commands do not mutate document host settings.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiDocumentState {
    /// Identity linking this host to its matching [`UiDocument`] root.
    pub(crate) root_id: ObjectId,
    /// Rendering and scaling configuration copied to the document's private
    /// runtime panel, preventing unrelated documents from sharing mutable state.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub panel_settings: PanelSettings,
    /// Selects layout-relative or independently positioned document placement.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub position: DocumentPosition,
    /// Selects fixed or content-derived dimensions for a world-space document.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub world_space_size_mode: WorldSpaceSizeMode,
    /// Width and height used when [`WorldSpaceSizeMode::Fixed`] controls a
    /// world-space document.
    #[serde(
        default = "default_world_size",
        skip_serializing_if = "is_default_world_size"
    )]
    pub world_space_size: ScreenSize,
    /// Geometry Unity uses as the frame for locating a world-space pivot.
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
    /// Creates host state linked to `root_id` with screen-space-compatible defaults.
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

    /// Selects whether the document participates in layout-relative positioning.
    #[must_use]
    pub fn position(mut self, value: DocumentPosition) -> Self {
        self.position = value;
        self
    }

    /// Selects whether world-space dimensions are fixed or derived from content.
    #[must_use]
    pub fn world_space_size_mode(mut self, value: WorldSpaceSizeMode) -> Self {
        self.world_space_size_mode = value;
        self
    }

    /// Sets the pixel width and height used by fixed-size world-space documents.
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

    /// Selects the point on the world-space document aligned with its host transform.
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
///
/// See Unity's [`PanelSettings` reference](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.PanelSettings.html)
/// for the corresponding runtime panel settings.
///
/// # Example
///
/// ```
/// use battlement_types::ScreenSize;
/// use battlement_ui::{PanelScaleMode, PanelScreenMatchMode, PanelSettings};
///
/// let settings = PanelSettings::new()
///     .scale_mode(PanelScaleMode::ScaleWithScreenSize)
///     .reference_resolution(ScreenSize::new(1920, 1080))
///     .screen_match_mode(PanelScreenMatchMode::MatchWidthOrHeight)
///     .match_factor(0.5);
///
/// assert!(battlement_ui::validate_panel_settings(&settings).is_ok());
/// ```
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelSettings {
    /// Determines whether the panel is composited over a display or rendered in
    /// world space from the document transform.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub render_mode: PanelRenderMode,
    /// Determines how authored UI lengths are converted to display pixels.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub scale_mode: PanelScaleMode,
    /// Sprite pixels corresponding to one UI unit.
    ///
    /// A sprite with the same Pixels Per Unit value renders at one source pixel
    /// per UI pixel before panel scaling.
    #[serde(default = "default_hundred", skip_serializing_if = "is_hundred")]
    pub reference_sprite_pixels_per_unit: f32,
    /// Uniform panel multiplier used only by [`PanelScaleMode::ConstantPixelSize`].
    #[serde(default = "default_one", skip_serializing_if = "is_one")]
    pub scale: f32,
    /// Design density, in dots per inch, used to convert physical units when
    /// [`PanelScaleMode::ConstantPhysicalSize`] is active.
    #[serde(default = "default_dpi", skip_serializing_if = "is_dpi")]
    pub reference_dpi: f32,
    /// Density, in dots per inch, used when the target display does not
    /// report a usable DPI.
    #[serde(default = "default_dpi", skip_serializing_if = "is_dpi")]
    pub fallback_dpi: f32,
    /// Design resolution, in pixels, compared with the target display when
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
    /// Zero-based Unity display index for a screen-space overlay panel.
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
    /// Creates settings with Battlement's Unity-compatible runtime defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects display-overlay or transform-based world-space rendering.
    ///
    /// World-space rendering uses the host transform and the world-space fields
    /// on [`UiDocumentState`].
    #[must_use]
    pub fn render_mode(mut self, value: PanelRenderMode) -> Self {
        self.render_mode = value;
        self
    }

    /// Selects how authored UI lengths are converted to display pixels.
    ///
    /// Fields belonging to another mode must remain at their defaults; call
    /// [`validate_panel_settings`](crate::validate_panel_settings) before use.
    #[must_use]
    pub fn scale_mode(mut self, value: PanelScaleMode) -> Self {
        self.scale_mode = value;
        self
    }

    /// Sets how many sprite pixels correspond to one UI unit.
    ///
    /// The value must be finite and greater than zero.
    #[must_use]
    pub fn reference_sprite_pixels_per_unit(mut self, value: f32) -> Self {
        self.reference_sprite_pixels_per_unit = value;
        self
    }

    /// Sets the positive uniform multiplier for constant-pixel-size scaling.
    #[must_use]
    pub fn scale(mut self, value: f32) -> Self {
        self.scale = value;
        self
    }

    /// Sets the positive design density, in DPI, for physical-size scaling.
    #[must_use]
    pub fn reference_dpi(mut self, value: f32) -> Self {
        self.reference_dpi = value;
        self
    }

    /// Sets the positive fallback DPI used when a display reports no usable density.
    #[must_use]
    pub fn fallback_dpi(mut self, value: f32) -> Self {
        self.fallback_dpi = value;
        self
    }

    /// Sets the nonzero design resolution for scale-with-screen-size mode.
    #[must_use]
    pub fn reference_resolution(mut self, value: ScreenSize) -> Self {
        self.reference_resolution = value;
        self
    }

    /// Selects how target width and height determine screen-size scaling.
    ///
    /// This setting is valid only with [`PanelScaleMode::ScaleWithScreenSize`].
    #[must_use]
    pub fn screen_match_mode(mut self, value: PanelScreenMatchMode) -> Self {
        self.screen_match_mode = value;
        self
    }

    /// Sets the width-to-height interpolation factor used by
    /// [`PanelScreenMatchMode::MatchWidthOrHeight`].
    ///
    /// `0` follows the width ratio, `1` follows the height ratio, and values
    /// between them blend the two. The value must be finite and in `0..=1`.
    #[must_use]
    pub fn match_factor(mut self, value: f32) -> Self {
        self.match_factor = value;
        self
    }

    /// Selects the zero-based Unity display for a screen-space overlay panel.
    ///
    /// Battlement accepts display indices from `0` through `7`.
    #[must_use]
    pub fn target_display(mut self, value: u32) -> Self {
        self.target_display = value;
        self
    }

    /// Sets whether Unity clears depth and stencil before rendering the panel.
    #[must_use]
    pub fn clear_depth_stencil(mut self, value: bool) -> Self {
        self.clear_depth_stencil = value;
        self
    }

    /// Sets whether Unity clears the color buffer before rendering the panel.
    #[must_use]
    pub fn clear_color(mut self, value: bool) -> Self {
        self.clear_color = value;
        self
    }

    /// Sets the finite, normalized RGBA value written when color clearing is enabled.
    #[must_use]
    pub fn color_clear_value(mut self, value: Color) -> Self {
        self.color_clear_value = value;
        self
    }

    /// Replaces the panel's dynamic-atlas allocation limits and exclusion filters.
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
/// All three sizes must be nonzero powers of two. The minimum cannot exceed the
/// maximum, the maximum sub-texture size cannot exceed the maximum atlas size,
/// and each [`DynamicAtlasFilter`] may appear at most once.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DynamicAtlasSettings {
    /// Minimum width and height, in pixels, allocated for a new dynamic atlas.
    pub min_atlas_size: u32,
    /// Maximum width and height, in pixels, to which the dynamic atlas may grow.
    pub max_atlas_size: u32,
    /// Maximum width or height, in pixels, of a texture eligible for insertion.
    pub max_sub_texture_size: u32,
    /// Ordered, duplicate-free filters Unity evaluates to exclude textures.
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

/// Where Unity renders a panel and how it interprets document geometry.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelRenderMode {
    /// Composites UI over the selected display in screen space.
    #[default]
    ScreenSpaceOverlay,
    /// Renders UI in the scene using the document GameObject's transform.
    WorldSpace,
}
/// Strategy for converting authored UI dimensions to display pixels.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelScaleMode {
    /// Multiplies authored pixel sizes uniformly by [`PanelSettings::scale`].
    ConstantPixelSize,
    /// Preserves physical size using the display DPI or fallback DPI.
    #[default]
    ConstantPhysicalSize,
    /// Scales relative to [`PanelSettings::reference_resolution`].
    ScaleWithScreenSize,
}
/// Strategy for reconciling target and reference aspect ratios.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PanelScreenMatchMode {
    /// Blends width- and height-based ratios using [`PanelSettings::match_factor`].
    #[default]
    MatchWidthOrHeight,
    /// Uses the smaller ratio so the reference resolution fits within the target.
    Shrink,
    /// Uses the larger ratio so the reference resolution covers the target.
    Expand,
}
/// A condition that prevents a texture from entering the dynamic atlas.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum DynamicAtlasFilter {
    /// Excludes textures whose CPU-readable flag is enabled.
    Readability,
    /// Excludes textures outside the configured sub-texture size limit.
    Size,
    /// Excludes textures whose format is unsuitable for atlas storage.
    Format,
    /// Excludes textures whose color space differs from the atlas.
    ColorSpace,
    /// Excludes textures whose filtering mode differs from the atlas.
    FilterMode,
}
/// How a document root participates in UI Toolkit positioning.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DocumentPosition {
    /// Participates in the ordinary layout flow relative to surrounding content.
    #[default]
    Relative,
    /// Is positioned independently of ordinary layout flow.
    Absolute,
}
/// How Unity determines the dimensions of a world-space document.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum WorldSpaceSizeMode {
    /// Uses [`UiDocumentState::world_space_size`] as the document dimensions.
    #[default]
    Fixed,
    /// Derives the document dimensions dynamically from its visual content.
    Dynamic,
}
/// Geometry used as the reference rectangle for a world-space pivot.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PivotReferenceSize {
    /// Uses the document's rendered bounding box.
    #[default]
    BoundingBox,
    /// Uses the document's resolved layout rectangle.
    Layout,
}
/// Point on a world-space document aligned with its host transform.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DocumentPivot {
    /// Aligns the transform with the top-left corner.
    TopLeft,
    /// Aligns the transform with the midpoint of the top edge.
    TopCenter,
    /// Aligns the transform with the top-right corner.
    TopRight,
    /// Aligns the transform with the midpoint of the left edge.
    MiddleLeft,
    /// Aligns the transform with the center of the document.
    #[default]
    Center,
    /// Aligns the transform with the midpoint of the right edge.
    MiddleRight,
    /// Aligns the transform with the bottom-left corner.
    BottomLeft,
    /// Aligns the transform with the midpoint of the bottom edge.
    BottomCenter,
    /// Aligns the transform with the bottom-right corner.
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
