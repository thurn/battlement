use battlement_types::{Color, MaterialAddress, TextureAddress, UiFontAddress};
use serde::{Deserialize, Serialize};

use crate::elements::background::BackgroundSource;

/// Explicit USS keyword accepted by every inline style property.
///
/// Use this when an update must clear a previously authored inline value back
/// to Unity's initial value. Leaving a [`Style`] field absent instead preserves
/// the current inline value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InlineKeyword {
  /// Replaces the inline declaration with the property's Unity initial value.
  Initial,
}

/// One concrete inline value or an explicit USS keyword.
///
/// Concrete values serialize directly. [`InlineKeyword::Initial`] serializes as
/// `{ "Keyword": "Initial" }`, keeping reset distinct from an omitted field.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StyleValue<T> {
  /// Assigns a concrete property value.
  Value(T),
  /// Assigns an explicit USS keyword.
  Keyword {
    /// Keyword sent to Unity's inline style.
    #[serde(rename = "Keyword")]
    value: InlineKeyword,
  },
}

impl<T> From<InlineKeyword> for StyleValue<T> {
  fn from(value: InlineKeyword) -> Self {
    Self::Keyword { value }
  }
}

impl From<Color> for StyleValue<Color> {
  fn from(value: Color) -> Self {
    Self::Value(value)
  }
}

impl From<MaterialAddress> for StyleValue<MaterialAddress> {
  fn from(value: MaterialAddress) -> Self {
    Self::Value(value)
  }
}

impl From<UiFontAddress> for StyleValue<UiFontAddress> {
  fn from(value: UiFontAddress) -> Self {
    Self::Value(value)
  }
}

impl From<BackgroundSource> for StyleValue<BackgroundSource> {
  fn from(value: BackgroundSource) -> Self {
    Self::Value(value)
  }
}

impl From<Cursor> for StyleValue<Cursor> {
  fn from(value: Cursor) -> Self {
    Self::Value(value)
  }
}

/// A finite UI Toolkit length in pixels or as a parent-relative percentage.
///
/// Percentages are not clamped to `0..=100`; oversize dimensions and offsets
/// are useful layout inputs. Property-specific validation can still reject
/// negative values where Unity expects a nonnegative size or spacing value.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Length {
  /// A device-independent UI Toolkit pixel length.
  Px(f32),
  /// A percentage of the property's containing dimension.
  Percent(f32),
}

impl From<i32> for Length {
  fn from(value: i32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<u32> for Length {
  fn from(value: u32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<f32> for Length {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

/// A finite UI Toolkit length that can also request automatic layout sizing.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum LengthOrAuto {
  /// A device-independent UI Toolkit pixel length.
  Px(f32),
  /// A percentage of the property's containing dimension.
  Percent(f32),
  /// Lets the UI Toolkit layout engine derive the value.
  Auto,
}

impl From<i32> for LengthOrAuto {
  fn from(value: i32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<u32> for LengthOrAuto {
  fn from(value: u32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<f32> for LengthOrAuto {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

impl From<Length> for LengthOrAuto {
  fn from(value: Length) -> Self {
    match value {
      Length::Px(value) => Self::Px(value),
      Length::Percent(value) => Self::Percent(value),
    }
  }
}

/// Extension methods for explicitly authored pixel and percentage lengths.
pub trait LengthUnits {
  /// Converts this number to a pixel [`Length`].
  fn px(self) -> Length;
  /// Converts this number to a percentage [`Length`].
  fn pct(self) -> Length;
}

impl LengthUnits for i32 {
  fn px(self) -> Length {
    Length::Px(self as f32)
  }

  fn pct(self) -> Length {
    Length::Percent(self as f32)
  }
}

impl LengthUnits for u32 {
  fn px(self) -> Length {
    Length::Px(self as f32)
  }

  fn pct(self) -> Length {
    Length::Percent(self as f32)
  }
}

impl LengthUnits for f32 {
  fn px(self) -> Length {
    Length::Px(self)
  }

  fn pct(self) -> Length {
    Length::Percent(self)
  }
}

/// A finite scalar used by numeric UI Toolkit style properties.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FloatValue(pub f32);

impl From<i32> for FloatValue {
  fn from(value: i32) -> Self {
    Self(value as f32)
  }
}

impl From<u32> for FloatValue {
  fn from(value: u32) -> Self {
    Self(value as f32)
  }
}

impl From<f32> for FloatValue {
  fn from(value: f32) -> Self {
    Self(value)
  }
}

/// Preferred width-to-height relationship used while resolving automatic size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum AspectRatio {
  /// Leaves the preferred ratio automatic.
  Auto,
  /// Uses the finite positive quotient `width / height` during layout.
  Ratio {
    /// Relative width component.
    width: f32,
    /// Relative height component.
    height: f32,
  },
}

impl AspectRatio {
  /// Creates a preferred ratio from positive width and height components.
  #[must_use]
  pub const fn new(width: f32, height: f32) -> Self {
    Self::Ratio { width, height }
  }
}

/// Cross-axis alignment for a flex container or item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Align {
  /// Defers item alignment to its container's alignment behavior.
  Auto,
  /// Packs content at the cross-axis start.
  FlexStart,
  /// Centers content on the cross axis.
  Center,
  /// Packs content at the cross-axis end.
  FlexEnd,
  /// Expands auto-sized content across the available cross axis.
  Stretch,
}

/// Main-axis direction used by a flex container to lay out its children.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FlexDirection {
  /// Lays out children from top to bottom.
  Column,
  /// Lays out children from bottom to top.
  ColumnReverse,
  /// Lays out children from left to right.
  Row,
  /// Lays out children from right to left.
  RowReverse,
}

/// Multi-line placement behavior for a flex container.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FlexWrap {
  /// Keeps children on one line even when they exceed available space.
  NoWrap,
  /// Moves overflowing children onto additional lines.
  Wrap,
  /// Wraps onto additional lines in the reverse cross-axis direction.
  WrapReverse,
}

/// Main-axis distribution of children inside a flex container.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Justify {
  /// Packs children at the main-axis start.
  FlexStart,
  /// Centers children along the main axis.
  Center,
  /// Packs children at the main-axis end.
  FlexEnd,
  /// Distributes free space between adjacent children.
  SpaceBetween,
  /// Distributes free space around every child.
  SpaceAround,
  /// Distributes equal free space between children and container edges.
  SpaceEvenly,
}

/// Whether an element participates in flex flow or is positioned independently.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Position {
  /// Keeps the element in normal flex layout and applies offsets relative to it.
  Relative,
  /// Removes the element from flex flow and resolves offsets against its parent.
  Absolute,
}

/// Whether an element participates in layout and rendering.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Display {
  /// Keeps the element in UI Toolkit's flex layout and renders it.
  Flex,
  /// Removes the element and its descendants from layout and rendering.
  None,
}

/// Whether an element is drawn while retaining its layout space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Visibility {
  /// Draws the element normally.
  Visible,
  /// Suppresses drawing while preserving the element's layout contribution.
  Hidden,
}

/// Whether descendants may paint outside an element's clipping boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Overflow {
  /// Allows descendant content to render beyond the element's bounds.
  Visible,
  /// Clips descendant content to the selected [`OverflowClipBox`].
  Hidden,
}

/// Box edge used when [`Overflow::Hidden`] clips descendant content.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OverflowClipBox {
  /// Clips at the outer edge of the padding box.
  PaddingBox,
  /// Clips at the inner content box, excluding padding.
  ContentBox,
}

/// How the center and edges of a nine-sliced background are painted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SliceType {
  /// Stretches the center and edge regions between fixed corners.
  Sliced,
  /// Repeats the center and edge regions between fixed corners.
  Tiled,
}

/// Font face style and weight selected from the active UI font.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FontStyle {
  /// Uses the font's regular face.
  Normal,
  /// Uses the font's bold face or synthesized bold weight.
  Bold,
  /// Uses the font's italic face or synthesized slant.
  Italic,
  /// Combines bold weight and italic slant.
  BoldAndItalic,
}

/// Alignment of text within the element's content rectangle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextAnchor {
  /// Aligns to the top-left corner.
  UpperLeft,
  /// Centers horizontally at the top edge.
  UpperCenter,
  /// Aligns to the top-right corner.
  UpperRight,
  /// Centers vertically at the left edge.
  MiddleLeft,
  /// Centers on both axes.
  MiddleCenter,
  /// Centers vertically at the right edge.
  MiddleRight,
  /// Aligns to the bottom-left corner.
  LowerLeft,
  /// Centers horizontally at the bottom edge.
  LowerCenter,
  /// Aligns to the bottom-right corner.
  LowerRight,
}

/// Whether UI Toolkit automatically chooses a font size that fits the box.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum TextAutoSize {
  /// Uses the separately authored font size.
  None,
  /// Chooses a fitting size between the inclusive pixel bounds.
  BestFit {
    /// Smallest font size Unity may choose, in pixels.
    min_size: f32,
    /// Largest font size Unity may choose, in pixels.
    max_size: f32,
  },
}

impl TextAutoSize {
  /// Creates ordered positive pixel bounds for best-fit text.
  #[must_use]
  pub const fn best_fit(min_size: f32, max_size: f32) -> Self {
    Self::BestFit { min_size, max_size }
  }
}

/// Text layout behavior when content exceeds the available width.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextOverflow {
  /// Cuts glyphs at the element's overflow boundary.
  Clip,
  /// Replaces hidden text with an ellipsis.
  Ellipsis,
}

/// Which portion of an elided string UI Toolkit preserves.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextOverflowPosition {
  /// Preserves the end of the string and elides its start.
  Start,
  /// Preserves both ends and elides the middle.
  Middle,
  /// Preserves the start and elides the end.
  End,
}

/// Whitespace preservation and wrapping behavior for rendered text.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WhiteSpace {
  /// Collapses spaces and wraps lines to fit.
  Normal,
  /// Collapses spaces without automatic line wrapping.
  NoWrap,
  /// Preserves spaces and newlines without automatic wrapping.
  Pre,
  /// Preserves spaces and newlines while allowing wrapping.
  PreWrap,
}

/// Text rendering backend selected for the element.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextGenerator {
  /// Uses Unity's standard text generator.
  Standard,
  /// Uses Unity's advanced generator for complex scripts when available.
  Advanced,
}

/// Glyph rasterization mode used by Unity's editor text renderer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EditorTextRenderingMode {
  /// Uses signed-distance-field glyph rendering.
  Sdf,
  /// Uses bitmap glyph rendering.
  Bitmap,
}

/// Shadow painted behind every rendered text glyph.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextShadow {
  /// Horizontal offset in panel pixels.
  pub x: f32,
  /// Vertical offset in panel pixels.
  pub y: f32,
  /// Nonnegative blur radius in panel pixels.
  pub blur_radius: f32,
  /// Shadow color multiplied with glyph coverage.
  pub color: Color,
}

impl TextShadow {
  /// Creates a text shadow from its pixel offset, blur radius, and color.
  #[must_use]
  pub const fn new(x: f32, y: f32, blur_radius: f32, color: Color) -> Self {
    Self {
      x,
      y,
      blur_radius,
      color,
    }
  }
}

/// Anchor used to position a background image along one element axis.
///
/// [`Center`](Self::Center) is valid on either axis. Left and right are valid
/// only for [`Style::background_position_x`], while top and bottom are valid
/// only for [`Style::background_position_y`]. The paired offset moves inward
/// from the selected edge; percentages resolve against the remaining space
/// after the background image has been sized.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BackgroundPositionKeyword {
  /// Centers the image on the selected axis before applying its offset.
  Center,
  /// Anchors the image to the top edge of the element.
  Top,
  /// Anchors the image to the bottom edge of the element.
  Bottom,
  /// Anchors the image to the left edge of the element.
  Left,
  /// Anchors the image to the right edge of the element.
  Right,
}

/// One axis of a background image's position inside an element.
///
/// The offset is a UI Toolkit pixel or percentage [`Length`]. Negative and
/// greater-than-100% offsets are supported for deliberately placing a
/// background beyond the element box. The property receiving this value
/// determines whether horizontal or vertical keywords are valid.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct BackgroundPosition {
  /// Edge or center used as the offset's origin.
  pub keyword: BackgroundPositionKeyword,
  /// Pixel or percentage displacement from the selected origin.
  pub offset: Length,
}

impl BackgroundPosition {
  /// Creates an anchored background position with the supplied offset.
  #[must_use]
  pub fn new(keyword: BackgroundPositionKeyword, offset: impl Into<Length>) -> Self {
    Self {
      keyword,
      offset: offset.into(),
    }
  }
}

/// Repetition mode for one axis of a background image.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BackgroundRepeatMode {
  /// Draws one image and leaves remaining space unpainted.
  NoRepeat,
  /// Tiles the image and clips the final tile when it does not fully fit.
  Repeat,
  /// Rescales tiles so a whole number fills the axis without gaps.
  Round,
  /// Draws only whole tiles and distributes remaining space between them.
  Space,
}

/// Independent horizontal and vertical repetition for a background image.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackgroundRepeat {
  /// Horizontal tiling behavior.
  pub x: BackgroundRepeatMode,
  /// Vertical tiling behavior.
  pub y: BackgroundRepeatMode,
}

impl BackgroundRepeat {
  /// Creates a background repeat pair without collapsing the two axes.
  #[must_use]
  pub const fn new(x: BackgroundRepeatMode, y: BackgroundRepeatMode) -> Self {
    Self { x, y }
  }
}

/// How UI Toolkit resolves a background image's painted dimensions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum BackgroundSize {
  /// Uses the source's intrinsic dimensions on both axes.
  Auto,
  /// Preserves aspect ratio while covering the element, cropping overflow.
  Cover,
  /// Preserves aspect ratio while keeping the complete source visible.
  Contain,
  /// Resolves each axis independently from a nonnegative length or automatic size.
  Axes {
    /// Horizontal image size; percentages resolve against element width.
    x: LengthOrAuto,
    /// Vertical image size; percentages resolve against element height.
    y: LengthOrAuto,
  },
}

impl BackgroundSize {
  /// Creates an explicit two-axis background size.
  #[must_use]
  pub fn axes(x: impl Into<LengthOrAuto>, y: impl Into<LengthOrAuto>) -> Self {
    Self::Axes {
      x: x.into(),
      y: y.into(),
    }
  }
}

/// Pixel selected as the active point of a custom cursor texture.
///
/// Coordinates start at the texture's top-left corner, increase right and
/// downward, and must fall inside the acquired texture's dimensions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CursorHotspot {
  /// Horizontal pixel offset from the texture's left edge.
  pub x: f32,
  /// Vertical pixel offset from the texture's top edge.
  pub y: f32,
}

impl CursorHotspot {
  /// Creates a cursor hotspot in top-left-origin texture pixels.
  #[must_use]
  pub const fn new(x: f32, y: f32) -> Self {
    Self { x, y }
  }
}

/// Runtime mouse cursor shown while a pointer hovers an element.
///
/// Custom cursors use one prepared texture imported with Unity's Cursor
/// defaults. UI Toolkit does not support runtime named cursors or fallback
/// chains through its public style API.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Cursor {
  /// Restores the platform's default cursor.
  Default,
  /// Uses a prepared cursor texture and its active pixel.
  Texture {
    /// Address of the prepared `Texture2D`.
    address: TextureAddress,
    /// Active pixel measured from the texture's top-left corner.
    hotspot: CursorHotspot,
  },
}

impl Cursor {
  /// Creates a custom cursor from a prepared texture and active pixel.
  #[must_use]
  pub fn texture(address: TextureAddress, hotspot: CursorHotspot) -> Self {
    Self::Texture { address, hotspot }
  }

  /// Returns the prepared texture retained by a custom cursor.
  #[must_use]
  pub const fn texture_address(&self) -> Option<&TextureAddress> {
    match self {
      Self::Default => None,
      Self::Texture { address, .. } => Some(address),
    }
  }
}

/// A rotation in degrees around a finite three-dimensional axis.
///
/// UI Toolkit applies this after scale and before translation without changing
/// layout. Positive angles rotate clockwise in panel space. The axis may point
/// in any direction but must not be the zero vector.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Rotate {
  /// Horizontal axis component.
  pub x: f32,
  /// Vertical axis component.
  pub y: f32,
  /// Depth axis component.
  pub z: f32,
  /// Clockwise rotation angle in degrees.
  pub degrees: f32,
}

impl Rotate {
  /// Creates a rotation around the supplied axis in degrees.
  #[must_use]
  pub const fn new(x: f32, y: f32, z: f32, degrees: f32) -> Self {
    Self { x, y, z, degrees }
  }

  /// Creates a panel-plane rotation around the positive z axis.
  #[must_use]
  pub const fn degrees(value: f32) -> Self {
    Self::new(0.0, 0.0, 1.0, value)
  }
}

/// Apparent horizontal and vertical size multipliers for an element.
///
/// Scale affects painting rather than flex layout. Negative values mirror the
/// element on that axis, and descendants are transformed with their parent.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Scale {
  /// Horizontal size multiplier.
  pub x: f32,
  /// Vertical size multiplier.
  pub y: f32,
}

impl Scale {
  /// Creates independent horizontal and vertical scale factors.
  #[must_use]
  pub const fn new(x: f32, y: f32) -> Self {
    Self { x, y }
  }

  /// Creates a uniform scale factor for both axes.
  #[must_use]
  pub const fn uniform(value: f32) -> Self {
    Self::new(value, value)
  }
}

/// A paint-time offset relative to an element's own size and position.
///
/// Percentage x and y values resolve against the element itself, not its
/// parent. The z component is measured in panel pixels. Translation does not
/// reserve layout space and is applied after scale and rotation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Translate {
  /// Horizontal pixel or self-relative percentage offset.
  pub x: Length,
  /// Vertical pixel or self-relative percentage offset.
  pub y: Length,
  /// Depth offset in panel pixels.
  pub z: f32,
}

impl Translate {
  /// Creates a translation with an explicit depth offset.
  #[must_use]
  pub const fn new(x: Length, y: Length, z: f32) -> Self {
    Self { x, y, z }
  }

  /// Creates a two-dimensional translation at zero depth.
  #[must_use]
  pub const fn two_dimensional(x: Length, y: Length) -> Self {
    Self::new(x, y, 0.0)
  }
}

/// Pivot used by an element's scale and rotation transforms.
///
/// Percentage x and y values resolve against the element bounds. Values may
/// lie outside those bounds, allowing an element to orbit an external pivot.
/// The z component is measured in panel pixels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransformOrigin {
  /// Horizontal pixel or self-relative percentage pivot.
  pub x: Length,
  /// Vertical pixel or self-relative percentage pivot.
  pub y: Length,
  /// Depth pivot in panel pixels.
  pub z: f32,
}

impl TransformOrigin {
  /// Creates a transform pivot with an explicit depth coordinate.
  #[must_use]
  pub const fn new(x: Length, y: Length, z: f32) -> Self {
    Self { x, y, z }
  }

  /// Creates a two-dimensional transform pivot at zero depth.
  #[must_use]
  pub const fn two_dimensional(x: Length, y: Length) -> Self {
    Self::new(x, y, 0.0)
  }
}

/// One standard UI Toolkit post-processing filter.
///
/// Filters are evaluated in authored order. Tint takes a color; opacity,
/// invert, grayscale, sepia, and contrast take unitless factors; blur is in
/// pixels; and hue rotation is in degrees. Battlement does not expose Unity's
/// custom filter definitions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum FilterFunction {
  /// Multiplies rendered color by the supplied tint.
  Tint(Color),
  /// Multiplies rendered alpha by a unitless factor.
  Opacity(f32),
  /// Blends rendered color toward its inverse by a unitless factor.
  Invert(f32),
  /// Blends rendered color toward grayscale by a unitless factor.
  Grayscale(f32),
  /// Blends rendered color toward sepia by a unitless factor.
  Sepia(f32),
  /// Applies a blur radius in panel pixels.
  Blur(f32),
  /// Adjusts contrast by a unitless factor.
  Contrast(f32),
  /// Rotates rendered hue by degrees.
  HueRotate(f32),
}

/// Ordered standard filter functions applied after an element is rendered.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FilterList(Vec<FilterFunction>);

impl FilterList {
  /// Creates a filter list in evaluation order.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = FilterFunction>) -> Self {
    Self(values.into_iter().collect())
  }

  /// Returns filter functions in evaluation order.
  #[must_use]
  pub fn as_slice(&self) -> &[FilterFunction] {
    &self.0
  }
}

/// One time value carried on the wire in milliseconds.
///
/// Unity receives the equivalent duration in seconds. Transition durations
/// must be nonnegative; transition delays may be negative to begin partway
/// through an animation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TimeValue(pub f32);

impl TimeValue {
  /// Creates a transition time in milliseconds.
  #[must_use]
  pub const fn milliseconds(value: f32) -> Self {
    Self(value)
  }
}

impl From<i32> for TimeValue {
  fn from(value: i32) -> Self {
    Self(value as f32)
  }
}

impl From<u32> for TimeValue {
  fn from(value: u32) -> Self {
    Self(value as f32)
  }
}

impl From<f32> for TimeValue {
  fn from(value: f32) -> Self {
    Self(value)
  }
}

/// UI Toolkit easing curve used to interpolate a transition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EasingFunction {
  /// CSS-like ease curve.
  Ease,
  /// Accelerating ease curve.
  EaseIn,
  /// Decelerating ease curve.
  EaseOut,
  /// Accelerating then decelerating ease curve.
  EaseInOut,
  /// Constant-speed interpolation.
  Linear,
  /// Sine acceleration.
  EaseInSine,
  /// Sine deceleration.
  EaseOutSine,
  /// Sine acceleration and deceleration.
  EaseInOutSine,
  /// Cubic acceleration.
  EaseInCubic,
  /// Cubic deceleration.
  EaseOutCubic,
  /// Cubic acceleration and deceleration.
  EaseInOutCubic,
  /// Circular acceleration.
  EaseInCirc,
  /// Circular deceleration.
  EaseOutCirc,
  /// Circular acceleration and deceleration.
  EaseInOutCirc,
  /// Elastic acceleration.
  EaseInElastic,
  /// Elastic deceleration.
  EaseOutElastic,
  /// Elastic acceleration and deceleration.
  EaseInOutElastic,
  /// Overshooting acceleration.
  EaseInBack,
  /// Overshooting deceleration.
  EaseOutBack,
  /// Overshooting acceleration and deceleration.
  EaseInOutBack,
  /// Bouncing acceleration.
  EaseInBounce,
  /// Bouncing deceleration.
  EaseOutBounce,
  /// Bouncing acceleration and deceleration.
  EaseInOutBounce,
}

/// Closed set of Battlement inline properties that a transition can target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TransitionProperty {
  /// Every animatable property.
  All,
  /// `align-content`.
  AlignContent,
  /// `align-items`.
  AlignItems,
  /// `align-self`.
  AlignSelf,
  /// `aspect-ratio`.
  AspectRatio,
  /// `background-color`.
  BackgroundColor,
  /// `background-image`.
  BackgroundImage,
  /// `background-position-x`.
  BackgroundPositionX,
  /// `background-position-y`.
  BackgroundPositionY,
  /// `background-repeat`.
  BackgroundRepeat,
  /// `background-size`.
  BackgroundSize,
  /// `border-bottom-color`.
  BorderBottomColor,
  /// `border-bottom-left-radius`.
  BorderBottomLeftRadius,
  /// `border-bottom-right-radius`.
  BorderBottomRightRadius,
  /// `border-bottom-width`.
  BorderBottomWidth,
  /// `border-left-color`.
  BorderLeftColor,
  /// `border-left-width`.
  BorderLeftWidth,
  /// `border-right-color`.
  BorderRightColor,
  /// `border-right-width`.
  BorderRightWidth,
  /// `border-top-color`.
  BorderTopColor,
  /// `border-top-left-radius`.
  BorderTopLeftRadius,
  /// `border-top-right-radius`.
  BorderTopRightRadius,
  /// `border-top-width`.
  BorderTopWidth,
  /// `bottom`.
  Bottom,
  /// `color`.
  Color,
  /// `cursor`.
  Cursor,
  /// `display`.
  Display,
  /// `filter`.
  Filter,
  /// `flex-basis`.
  FlexBasis,
  /// `flex-direction`.
  FlexDirection,
  /// `flex-grow`.
  FlexGrow,
  /// `flex-shrink`.
  FlexShrink,
  /// `flex-wrap`.
  FlexWrap,
  /// `font-size`.
  FontSize,
  /// `height`.
  Height,
  /// `justify-content`.
  JustifyContent,
  /// `left`.
  Left,
  /// `letter-spacing`.
  LetterSpacing,
  /// `margin-bottom`.
  MarginBottom,
  /// `margin-left`.
  MarginLeft,
  /// `margin-right`.
  MarginRight,
  /// `margin-top`.
  MarginTop,
  /// `max-height`.
  MaxHeight,
  /// `max-width`.
  MaxWidth,
  /// `min-height`.
  MinHeight,
  /// `min-width`.
  MinWidth,
  /// `opacity`.
  Opacity,
  /// `overflow`.
  Overflow,
  /// `padding-bottom`.
  PaddingBottom,
  /// `padding-left`.
  PaddingLeft,
  /// `padding-right`.
  PaddingRight,
  /// `padding-top`.
  PaddingTop,
  /// `position`.
  Position,
  /// `right`.
  Right,
  /// `rotate`.
  Rotate,
  /// `scale`.
  Scale,
  /// `text-overflow`.
  TextOverflow,
  /// `text-shadow`.
  TextShadow,
  /// `top`.
  Top,
  /// `transform-origin`.
  TransformOrigin,
  /// `transition-delay`.
  TransitionDelay,
  /// `transition-duration`.
  TransitionDuration,
  /// `transition-property`.
  TransitionProperty,
  /// `transition-timing-function`.
  TransitionTimingFunction,
  /// `translate`.
  Translate,
  /// `-unity-background-image-tint-color`.
  UnityBackgroundImageTintColor,
  /// `-unity-editor-text-rendering-mode`.
  UnityEditorTextRenderingMode,
  /// `-unity-font-definition`.
  UnityFontDefinition,
  /// `-unity-font-style`.
  UnityFontStyleAndWeight,
  /// `-unity-material`.
  UnityMaterial,
  /// `-unity-overflow-clip-box`.
  UnityOverflowClipBox,
  /// `-unity-paragraph-spacing`.
  UnityParagraphSpacing,
  /// `-unity-slice-bottom`.
  UnitySliceBottom,
  /// `-unity-slice-left`.
  UnitySliceLeft,
  /// `-unity-slice-right`.
  UnitySliceRight,
  /// `-unity-slice-scale`.
  UnitySliceScale,
  /// `-unity-slice-top`.
  UnitySliceTop,
  /// `-unity-slice-type`.
  UnitySliceType,
  /// `-unity-text-align`.
  UnityTextAlign,
  /// `-unity-text-auto-size`.
  UnityTextAutoSize,
  /// `-unity-text-generator`.
  UnityTextGenerator,
  /// `-unity-text-outline-color`.
  UnityTextOutlineColor,
  /// `-unity-text-outline-width`.
  UnityTextOutlineWidth,
  /// `-unity-text-overflow-position`.
  UnityTextOverflowPosition,
  /// `visibility`.
  Visibility,
  /// `white-space`.
  WhiteSpace,
  /// `width`.
  Width,
  /// `word-spacing`.
  WordSpacing,
}

/// One authored list whose shorter values repeat across transition properties.
///
/// UI Toolkit repeats each nonempty parallel list cyclically until every
/// transition property has a duration, delay, and easing function. An empty
/// list leaves Unity with no authored entries for that inline property.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TransitionList<T>(Vec<T>);

impl<T> TransitionList<T> {
  /// Creates a transition list in authored order.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = T>) -> Self {
    Self(values.into_iter().collect())
  }

  /// Returns values in authored order.
  #[must_use]
  pub fn as_slice(&self) -> &[T] {
    &self.0
  }

  /// Returns the value UI Toolkit repeats at `index`, or `None` for an empty list.
  #[must_use]
  pub fn repeated(&self, index: usize) -> Option<&T> {
    (!self.0.is_empty()).then(|| &self.0[index % self.0.len()])
  }
}

/// Converts one CSS-order shorthand into top, right, bottom, and left values.
///
/// One value applies to every side, two apply vertically then horizontally,
/// three apply top, horizontal, then bottom, and four apply clockwise.
pub trait IntoStyleSides<T> {
  /// Expands this shorthand to four concrete side values.
  fn into_style_sides(self) -> [StyleValue<T>; 4];
}

macro_rules! style_sides_for {
  ($target:ty, $source:ty) => {
    impl IntoStyleSides<$target> for $source {
      fn into_style_sides(self) -> [StyleValue<$target>; 4] {
        let value = self.into();
        [value; 4]
      }
    }

    impl IntoStyleSides<$target> for ($source, $source) {
      fn into_style_sides(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.0.into(), self.1.into()]
      }
    }

    impl IntoStyleSides<$target> for ($source, $source, $source) {
      fn into_style_sides(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.1.into()]
      }
    }

    impl IntoStyleSides<$target> for ($source, $source, $source, $source) {
      fn into_style_sides(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.3.into()]
      }
    }
  };
}

style_sides_for!(Length, i32);
style_sides_for!(Length, u32);
style_sides_for!(Length, f32);
style_sides_for!(Length, Length);
style_sides_for!(Length, InlineKeyword);
style_sides_for!(LengthOrAuto, i32);
style_sides_for!(LengthOrAuto, u32);
style_sides_for!(LengthOrAuto, f32);
style_sides_for!(LengthOrAuto, Length);
style_sides_for!(LengthOrAuto, LengthOrAuto);
style_sides_for!(LengthOrAuto, InlineKeyword);
style_sides_for!(FloatValue, i32);
style_sides_for!(FloatValue, u32);
style_sides_for!(FloatValue, f32);
style_sides_for!(FloatValue, FloatValue);
style_sides_for!(FloatValue, InlineKeyword);
style_sides_for!(Color, Color);
style_sides_for!(Color, InlineKeyword);

/// Converts a CSS-order shorthand into four corner values.
///
/// One value applies to every corner. Two alternate diagonal pairs; three
/// apply top-left, the other diagonal, then bottom-right; and four proceed
/// clockwise from top-left.
pub trait IntoStyleCorners<T> {
  /// Expands this shorthand to top-left, top-right, bottom-right, and bottom-left.
  fn into_style_corners(self) -> [StyleValue<T>; 4];
}

macro_rules! style_corners_for {
  ($target:ty, $source:ty) => {
    impl IntoStyleCorners<$target> for $source {
      fn into_style_corners(self) -> [StyleValue<$target>; 4] {
        let value = self.into();
        [value; 4]
      }
    }

    impl IntoStyleCorners<$target> for ($source, $source) {
      fn into_style_corners(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.0.into(), self.1.into()]
      }
    }

    impl IntoStyleCorners<$target> for ($source, $source, $source) {
      fn into_style_corners(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.1.into()]
      }
    }

    impl IntoStyleCorners<$target> for ($source, $source, $source, $source) {
      fn into_style_corners(self) -> [StyleValue<$target>; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.3.into()]
      }
    }
  };
}

style_corners_for!(Length, i32);
style_corners_for!(Length, u32);
style_corners_for!(Length, f32);
style_corners_for!(Length, Length);
style_corners_for!(Length, InlineKeyword);

/// Inline Unity Style Sheet declarations applied directly to one element.
///
/// Layout properties are not inherited. Present fields replace their matching
/// inline declaration, omitted fields leave it unchanged, and
/// [`InlineKeyword::Initial`] explicitly clears it to Unity's initial value.
/// UI Toolkit uses a border-box model: authored width and height include
/// padding and borders.
///
/// See Unity's [USS properties reference](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-USS-Properties-Reference.html)
/// and [layout engine guide](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-LayoutEngine.html).
///
/// # Example
///
/// ```
/// use battlement_ui::{FlexDirection, FlexWrap, LengthUnits, Style};
///
/// let toolbar = Style::new()
///     .flex_direction(FlexDirection::Row)
///     .flex_wrap(FlexWrap::Wrap)
///     .width(100.pct())
///     .padding((12, 20));
///
/// assert!(!toolbar.is_empty());
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Style {
  /// Cross-axis alignment of wrapped lines inside this flex container.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub align_content: Option<StyleValue<Align>>,
  /// Default cross-axis alignment applied to this flex container's children.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub align_items: Option<StyleValue<Align>>,
  /// Cross-axis alignment of this item, overriding its container's alignment.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub align_self: Option<StyleValue<Align>>,
  /// Preferred width-to-height ratio used when at least one dimension is automatic.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<StyleValue<AspectRatio>>,
  /// Color painted behind the element's content and padding, inside its border.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_color: Option<StyleValue<Color>>,
  /// Prepared image painted behind content and affected by background tint and slicing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_image: Option<StyleValue<BackgroundSource>>,
  /// Horizontal background anchor and offset after image sizing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_position_x: Option<StyleValue<BackgroundPosition>>,
  /// Vertical background anchor and offset after image sizing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_position_y: Option<StyleValue<BackgroundPosition>>,
  /// Independent horizontal and vertical background tiling behavior.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_repeat: Option<StyleValue<BackgroundRepeat>>,
  /// Intrinsic, fitted, covering, or explicit background-image dimensions.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background_size: Option<StyleValue<BackgroundSize>>,
  /// Color of the bottom border; it is visible only when the bottom width is positive.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_bottom_color: Option<StyleValue<Color>>,
  /// Radius of the bottom-left corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_bottom_left_radius: Option<StyleValue<Length>>,
  /// Radius of the bottom-right corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_bottom_right_radius: Option<StyleValue<Length>>,
  /// Layout space, in pixels, reserved for the bottom border edge.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_bottom_width: Option<StyleValue<FloatValue>>,
  /// Color of the left border; it is visible only when the left width is positive.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_left_color: Option<StyleValue<Color>>,
  /// Layout space, in pixels, reserved for the left border edge.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_left_width: Option<StyleValue<FloatValue>>,
  /// Color of the right border; it is visible only when the right width is positive.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_right_color: Option<StyleValue<Color>>,
  /// Layout space, in pixels, reserved for the right border edge.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_right_width: Option<StyleValue<FloatValue>>,
  /// Color of the top border; it is visible only when the top width is positive.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_top_color: Option<StyleValue<Color>>,
  /// Radius of the top-left corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_top_left_radius: Option<StyleValue<Length>>,
  /// Radius of the top-right corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_top_right_radius: Option<StyleValue<Length>>,
  /// Layout space, in pixels, reserved for the top border edge.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub border_top_width: Option<StyleValue<FloatValue>>,
  /// Bottom offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bottom: Option<StyleValue<LengthOrAuto>>,
  /// Foreground color inherited by text unless a descendant overrides it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub color: Option<StyleValue<Color>>,
  /// Runtime mouse cursor used while a pointer hovers this element.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cursor: Option<StyleValue<Cursor>>,
  /// Whether this element and its descendants participate in layout and rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub display: Option<StyleValue<Display>>,
  /// Ordered post-processing functions applied to the rendered element subtree.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub filter: Option<StyleValue<FilterList>>,
  /// Initial main-axis size before flex grow and shrink distribute free space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flex_basis: Option<StyleValue<LengthOrAuto>>,
  /// Direction and ordering of this flex container's main axis.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flex_direction: Option<StyleValue<FlexDirection>>,
  /// Nonnegative share of remaining main-axis space assigned to this item.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flex_grow: Option<StyleValue<FloatValue>>,
  /// Nonnegative shrink factor used when siblings exceed the main-axis space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flex_shrink: Option<StyleValue<FloatValue>>,
  /// Whether children remain on one line or wrap across the cross axis.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub flex_wrap: Option<StyleValue<FlexWrap>>,
  /// Font size, in pixels, inherited by descendant text unless overridden.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub font_size: Option<StyleValue<Length>>,
  /// Border-box height in pixels, percentage, automatic size, or initial value.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub height: Option<StyleValue<LengthOrAuto>>,
  /// Main-axis packing and free-space distribution for this container's children.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub justify_content: Option<StyleValue<Justify>>,
  /// Additional advance inserted between adjacent glyphs; negative values tighten text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub letter_spacing: Option<StyleValue<Length>>,
  /// Left offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub left: Option<StyleValue<LengthOrAuto>>,
  /// Space outside the bottom border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin_bottom: Option<StyleValue<LengthOrAuto>>,
  /// Space outside the left border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin_left: Option<StyleValue<LengthOrAuto>>,
  /// Space outside the right border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin_right: Option<StyleValue<LengthOrAuto>>,
  /// Space outside the top border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin_top: Option<StyleValue<LengthOrAuto>>,
  /// Maximum border-box height applied after preferred size and flex calculations.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_height: Option<StyleValue<LengthOrAuto>>,
  /// Maximum border-box width applied after preferred size and flex calculations.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_width: Option<StyleValue<LengthOrAuto>>,
  /// Minimum border-box height that constrains shrinking and automatic sizing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min_height: Option<StyleValue<LengthOrAuto>>,
  /// Minimum border-box width that constrains shrinking and automatic sizing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min_width: Option<StyleValue<LengthOrAuto>>,
  /// Element opacity multiplied through its rendered subtree, from transparent zero to opaque one.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub opacity: Option<StyleValue<FloatValue>>,
  /// Whether descendant painting is clipped at this element's selected clip box.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub overflow: Option<StyleValue<Overflow>>,
  /// Space between the bottom border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub padding_bottom: Option<StyleValue<Length>>,
  /// Space between the left border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub padding_left: Option<StyleValue<Length>>,
  /// Space between the right border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub padding_right: Option<StyleValue<Length>>,
  /// Space between the top border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub padding_top: Option<StyleValue<Length>>,
  /// Selects normal flex flow or independent placement against the parent box.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub position: Option<StyleValue<Position>>,
  /// Right offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub right: Option<StyleValue<LengthOrAuto>>,
  /// Paint-time rotation around the authored transform origin.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub rotate: Option<StyleValue<Rotate>>,
  /// Paint-time horizontal and vertical size multipliers.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scale: Option<StyleValue<Scale>>,
  /// Whether overflowing text is clipped or replaced with an ellipsis.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text_overflow: Option<StyleValue<TextOverflow>>,
  /// Shadow rendered behind each glyph without affecting layout.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text_shadow: Option<StyleValue<TextShadow>>,
  /// Top offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub top: Option<StyleValue<LengthOrAuto>>,
  /// Pivot against which scale and rotation are resolved.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transform_origin: Option<StyleValue<TransformOrigin>>,
  /// Per-property delays in milliseconds; negative values begin partway through a transition.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transition_delay: Option<StyleValue<TransitionList<TimeValue>>>,
  /// Nonnegative per-property transition durations in milliseconds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transition_duration: Option<StyleValue<TransitionList<TimeValue>>>,
  /// Properties whose value changes should be interpolated.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transition_property: Option<StyleValue<TransitionList<TransitionProperty>>>,
  /// Per-property interpolation curves repeated across the transition-property list.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub transition_timing_function: Option<StyleValue<TransitionList<EasingFunction>>>,
  /// Paint-time offset applied after scale and rotation without affecting layout.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub translate: Option<StyleValue<Translate>>,
  /// Color multiplied with pixels from a background image before compositing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_background_image_tint_color: Option<StyleValue<Color>>,
  /// Selects signed-distance-field or bitmap editor text rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_editor_text_rendering_mode: Option<StyleValue<EditorTextRenderingMode>>,
  /// Prepared TextCore font asset inherited by descendant text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_font_definition: Option<StyleValue<UiFontAddress>>,
  /// Bold and italic selection inherited by descendant text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_font_style_and_weight: Option<StyleValue<FontStyle>>,
  /// Prepared custom material used to render this element and inherited by descendants.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_material: Option<StyleValue<MaterialAddress>>,
  /// Selects the padding or content box as the boundary for hidden overflow.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_overflow_clip_box: Option<StyleValue<OverflowClipBox>>,
  /// Extra vertical advance inserted after each paragraph.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_paragraph_spacing: Option<StyleValue<Length>>,
  /// Bottom inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_bottom: Option<StyleValue<i32>>,
  /// Left inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_left: Option<StyleValue<i32>>,
  /// Right inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_right: Option<StyleValue<i32>>,
  /// Positive multiplier applied to nine-slice inset sizes.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_scale: Option<StyleValue<FloatValue>>,
  /// Top inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_top: Option<StyleValue<i32>>,
  /// Selects stretched or repeated center and edge regions for nine-slice backgrounds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_slice_type: Option<StyleValue<SliceType>>,
  /// Alignment of text within the content rectangle.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_align: Option<StyleValue<TextAnchor>>,
  /// Optional best-fit font sizing within positive pixel bounds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_auto_size: Option<StyleValue<TextAutoSize>>,
  /// Text generation backend used for glyph layout and rendering.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_generator: Option<StyleValue<TextGenerator>>,
  /// Color of the stroke painted around every text glyph.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_outline_color: Option<StyleValue<Color>>,
  /// Nonnegative text outline width in panel pixels.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_outline_width: Option<StyleValue<FloatValue>>,
  /// Portion of an overflowing string preserved around its ellipsis.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unity_text_overflow_position: Option<StyleValue<TextOverflowPosition>>,
  /// Whether the element is drawn while retaining its layout space.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub visibility: Option<StyleValue<Visibility>>,
  /// Controls newline preservation, space collapsing, and automatic wrapping.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub white_space: Option<StyleValue<WhiteSpace>>,
  /// Border-box width in pixels, percentage, automatic size, or initial value.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub width: Option<StyleValue<LengthOrAuto>>,
  /// Additional advance inserted at word boundaries; negative values tighten text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub word_spacing: Option<StyleValue<Length>>,
}

impl Style {
  /// Creates an empty set of inline declarations.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Overlays populated declarations from `value`, preserving absent fields.
  #[must_use]
  pub fn merge(mut self, value: Self) -> Self {
    macro_rules! merge_fields {
            ($($field:ident),+ $(,)?) => {$(
                self.$field = value.$field.or(self.$field);
            )+};
        }
    merge_fields!(
      align_content,
      align_items,
      align_self,
      aspect_ratio,
      background_color,
      background_image,
      background_position_x,
      background_position_y,
      background_repeat,
      background_size,
      border_bottom_color,
      border_bottom_left_radius,
      border_bottom_right_radius,
      border_bottom_width,
      border_left_color,
      border_left_width,
      border_right_color,
      border_right_width,
      border_top_color,
      border_top_left_radius,
      border_top_right_radius,
      border_top_width,
      bottom,
      color,
      cursor,
      display,
      filter,
      flex_basis,
      flex_direction,
      flex_grow,
      flex_shrink,
      flex_wrap,
      font_size,
      height,
      justify_content,
      letter_spacing,
      left,
      margin_bottom,
      margin_left,
      margin_right,
      margin_top,
      max_height,
      max_width,
      min_height,
      min_width,
      opacity,
      overflow,
      padding_bottom,
      padding_left,
      padding_right,
      padding_top,
      position,
      right,
      rotate,
      scale,
      text_overflow,
      text_shadow,
      top,
      transform_origin,
      transition_delay,
      transition_duration,
      transition_property,
      transition_timing_function,
      translate,
      unity_background_image_tint_color,
      unity_editor_text_rendering_mode,
      unity_font_definition,
      unity_font_style_and_weight,
      unity_material,
      unity_overflow_clip_box,
      unity_paragraph_spacing,
      unity_slice_bottom,
      unity_slice_left,
      unity_slice_right,
      unity_slice_scale,
      unity_slice_top,
      unity_slice_type,
      unity_text_align,
      unity_text_auto_size,
      unity_text_generator,
      unity_text_outline_color,
      unity_text_outline_width,
      unity_text_overflow_position,
      visibility,
      white_space,
      width,
      word_spacing,
    );
    self
  }

  /// Aligns wrapped lines on the container's cross axis.
  #[must_use]
  pub fn align_content(mut self, value: impl Into<StyleValue<Align>>) -> Self {
    self.align_content = Some(value.into());
    self
  }

  /// Sets the default cross-axis alignment of direct children.
  #[must_use]
  pub fn align_items(mut self, value: impl Into<StyleValue<Align>>) -> Self {
    self.align_items = Some(value.into());
    self
  }

  /// Overrides this item's cross-axis alignment within its flex container.
  #[must_use]
  pub fn align_self(mut self, value: impl Into<StyleValue<Align>>) -> Self {
    self.align_self = Some(value.into());
    self
  }

  /// Sets the preferred ratio used while resolving automatic dimensions.
  #[must_use]
  pub fn aspect_ratio(mut self, value: impl Into<StyleValue<AspectRatio>>) -> Self {
    self.aspect_ratio = Some(value.into());
    self
  }

  /// Paints a color behind the element's content and padding.
  #[must_use]
  pub fn background_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.background_color = Some(value.into());
    self
  }

  /// Paints a prepared image behind the element so background sizing, tinting, and slicing can affect it.
  #[must_use]
  pub fn background_image(mut self, value: impl Into<StyleValue<BackgroundSource>>) -> Self {
    self.background_image = Some(value.into());
    self
  }

  /// Positions the background horizontally from left, center, or right.
  #[must_use]
  pub fn background_position_x(mut self, value: impl Into<StyleValue<BackgroundPosition>>) -> Self {
    self.background_position_x = Some(value.into());
    self
  }

  /// Positions the background vertically from top, center, or bottom.
  #[must_use]
  pub fn background_position_y(mut self, value: impl Into<StyleValue<BackgroundPosition>>) -> Self {
    self.background_position_y = Some(value.into());
    self
  }

  /// Selects independent horizontal and vertical background tiling modes.
  #[must_use]
  pub fn background_repeat(mut self, value: impl Into<StyleValue<BackgroundRepeat>>) -> Self {
    self.background_repeat = Some(value.into());
    self
  }

  /// Selects intrinsic, fitted, covering, or explicit background dimensions.
  #[must_use]
  pub fn background_size(mut self, value: impl Into<StyleValue<BackgroundSize>>) -> Self {
    self.background_size = Some(value.into());
    self
  }

  /// Sets the bottom border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_bottom_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.border_bottom_color = Some(value.into());
    self
  }

  /// Rounds the bottom-left corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_bottom_left_radius(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.border_bottom_left_radius = Some(value.into());
    self
  }

  /// Rounds the bottom-right corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_bottom_right_radius(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.border_bottom_right_radius = Some(value.into());
    self
  }

  /// Reserves a nonnegative pixel width for the bottom border.
  #[must_use]
  pub fn border_bottom_width(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.border_bottom_width = Some(value.into());
    self
  }

  /// Sets the left border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_left_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.border_left_color = Some(value.into());
    self
  }

  /// Reserves a nonnegative pixel width for the left border.
  #[must_use]
  pub fn border_left_width(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.border_left_width = Some(value.into());
    self
  }

  /// Sets the right border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_right_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.border_right_color = Some(value.into());
    self
  }

  /// Reserves a nonnegative pixel width for the right border.
  #[must_use]
  pub fn border_right_width(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.border_right_width = Some(value.into());
    self
  }

  /// Sets the top border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_top_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.border_top_color = Some(value.into());
    self
  }

  /// Rounds the top-left corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_top_left_radius(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.border_top_left_radius = Some(value.into());
    self
  }

  /// Rounds the top-right corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_top_right_radius(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.border_top_right_radius = Some(value.into());
    self
  }

  /// Reserves a nonnegative pixel width for the top border.
  #[must_use]
  pub fn border_top_width(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.border_top_width = Some(value.into());
    self
  }

  /// Expands CSS-order colors into the top, right, bottom, and left border fields.
  #[must_use]
  pub fn border_color(mut self, value: impl IntoStyleSides<Color>) -> Self {
    [
      self.border_top_color,
      self.border_right_color,
      self.border_bottom_color,
      self.border_left_color,
    ] = value.into_style_sides().map(Some);
    self
  }

  /// Expands corner radii clockwise from top-left using CSS shorthand rules.
  #[must_use]
  pub fn border_radius(mut self, value: impl IntoStyleCorners<Length>) -> Self {
    [
      self.border_top_left_radius,
      self.border_top_right_radius,
      self.border_bottom_right_radius,
      self.border_bottom_left_radius,
    ] = value.into_style_corners().map(Some);
    self
  }

  /// Expands CSS-order widths into the top, right, bottom, and left border fields.
  #[must_use]
  pub fn border_width(mut self, value: impl IntoStyleSides<FloatValue>) -> Self {
    [
      self.border_top_width,
      self.border_right_width,
      self.border_bottom_width,
      self.border_left_width,
    ] = value.into_style_sides().map(Some);
    self
  }

  /// Sets the bottom offset used by relative or absolute positioning.
  #[must_use]
  pub fn bottom(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.bottom = Some(value.into());
    self
  }

  /// Sets the text color inherited by this element's descendants.
  #[must_use]
  pub fn color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.color = Some(value.into());
    self
  }

  /// Sets the runtime mouse cursor shown while this element is hovered.
  #[must_use]
  pub fn cursor(mut self, value: impl Into<StyleValue<Cursor>>) -> Self {
    self.cursor = Some(value.into());
    self
  }

  /// Selects flex participation or removes the subtree from layout and rendering.
  #[must_use]
  pub fn display(mut self, value: impl Into<StyleValue<Display>>) -> Self {
    self.display = Some(value.into());
    self
  }

  /// Sets ordered post-processing filters for this rendered subtree.
  ///
  /// Functions run in list order. An empty list removes concrete filter
  /// functions, while an omitted field preserves the current inline value.
  #[must_use]
  pub fn filter(mut self, value: impl Into<StyleValue<FilterList>>) -> Self {
    self.filter = Some(value.into());
    self
  }

  /// Sets the item's initial main-axis size before flex distribution.
  #[must_use]
  pub fn flex_basis(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.flex_basis = Some(value.into());
    self
  }

  /// Selects the main-axis direction and child order.
  #[must_use]
  pub fn flex_direction(mut self, value: impl Into<StyleValue<FlexDirection>>) -> Self {
    self.flex_direction = Some(value.into());
    self
  }

  /// Sets this item's nonnegative share of remaining main-axis space.
  #[must_use]
  pub fn flex_grow(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.flex_grow = Some(value.into());
    self
  }

  /// Sets this item's nonnegative shrink factor when space is insufficient.
  #[must_use]
  pub fn flex_shrink(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.flex_shrink = Some(value.into());
    self
  }

  /// Selects single-line or multi-line child placement.
  #[must_use]
  pub fn flex_wrap(mut self, value: impl Into<StyleValue<FlexWrap>>) -> Self {
    self.flex_wrap = Some(value.into());
    self
  }

  /// Sets the inherited text size in pixels.
  #[must_use]
  pub fn font_size(mut self, value: impl Into<Length>) -> Self {
    self.font_size = Some(StyleValue::Value(value.into()));
    self
  }

  /// Sets additional advance between glyphs in pixels or font-relative percentage.
  #[must_use]
  pub fn letter_spacing(mut self, value: impl Into<Length>) -> Self {
    self.letter_spacing = Some(StyleValue::Value(value.into()));
    self
  }
  /// Chooses clipping or ellipsis when text exceeds its box.
  #[must_use]
  pub fn text_overflow(mut self, value: TextOverflow) -> Self {
    self.text_overflow = Some(StyleValue::Value(value));
    self
  }
  /// Paints a shadow behind glyphs without changing layout.
  #[must_use]
  pub fn text_shadow(mut self, value: TextShadow) -> Self {
    self.text_shadow = Some(StyleValue::Value(value));
    self
  }
  /// Selects the editor text rasterization mode.
  #[must_use]
  pub fn unity_editor_text_rendering_mode(mut self, value: EditorTextRenderingMode) -> Self {
    self.unity_editor_text_rendering_mode = Some(StyleValue::Value(value));
    self
  }
  /// Assigns a prepared TextCore UI font asset.
  #[must_use]
  pub fn unity_font_definition(mut self, value: UiFontAddress) -> Self {
    self.unity_font_definition = Some(StyleValue::Value(value));
    self
  }
  /// Selects normal, bold, italic, or bold-italic glyph styling.
  #[must_use]
  pub fn unity_font_style_and_weight(mut self, value: FontStyle) -> Self {
    self.unity_font_style_and_weight = Some(StyleValue::Value(value));
    self
  }
  /// Adds vertical spacing after paragraphs.
  #[must_use]
  pub fn unity_paragraph_spacing(mut self, value: impl Into<Length>) -> Self {
    self.unity_paragraph_spacing = Some(StyleValue::Value(value.into()));
    self
  }
  /// Aligns text inside its content rectangle.
  #[must_use]
  pub fn unity_text_align(mut self, value: TextAnchor) -> Self {
    self.unity_text_align = Some(StyleValue::Value(value));
    self
  }
  /// Enables or disables best-fit font sizing.
  #[must_use]
  pub fn unity_text_auto_size(mut self, value: TextAutoSize) -> Self {
    self.unity_text_auto_size = Some(StyleValue::Value(value));
    self
  }
  /// Selects the standard or advanced text generator.
  #[must_use]
  pub fn unity_text_generator(mut self, value: TextGenerator) -> Self {
    self.unity_text_generator = Some(StyleValue::Value(value));
    self
  }
  /// Sets the text-glyph outline color.
  #[must_use]
  pub fn unity_text_outline_color(mut self, value: Color) -> Self {
    self.unity_text_outline_color = Some(StyleValue::Value(value));
    self
  }
  /// Sets the nonnegative text-glyph outline width in pixels.
  #[must_use]
  pub fn unity_text_outline_width(mut self, value: impl Into<FloatValue>) -> Self {
    self.unity_text_outline_width = Some(StyleValue::Value(value.into()));
    self
  }
  /// Chooses which portion of an elided string remains visible.
  #[must_use]
  pub fn unity_text_overflow_position(mut self, value: TextOverflowPosition) -> Self {
    self.unity_text_overflow_position = Some(StyleValue::Value(value));
    self
  }
  /// Chooses whitespace preservation and wrapping behavior.
  #[must_use]
  pub fn white_space(mut self, value: WhiteSpace) -> Self {
    self.white_space = Some(StyleValue::Value(value));
    self
  }
  /// Sets additional advance at word boundaries.
  #[must_use]
  pub fn word_spacing(mut self, value: impl Into<Length>) -> Self {
    self.word_spacing = Some(StyleValue::Value(value.into()));
    self
  }

  /// Sets the preferred border-box height.
  #[must_use]
  pub fn height(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.height = Some(value.into());
    self
  }

  /// Distributes children and free space along the main axis.
  #[must_use]
  pub fn justify_content(mut self, value: impl Into<StyleValue<Justify>>) -> Self {
    self.justify_content = Some(value.into());
    self
  }

  /// Sets the left offset used by relative or absolute positioning.
  #[must_use]
  pub fn left(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.left = Some(value.into());
    self
  }

  /// Sets bottom outer spacing for this element.
  #[must_use]
  pub fn margin_bottom(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.margin_bottom = Some(value.into());
    self
  }

  /// Sets left outer spacing for this element.
  #[must_use]
  pub fn margin_left(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.margin_left = Some(value.into());
    self
  }

  /// Sets right outer spacing for this element.
  #[must_use]
  pub fn margin_right(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.margin_right = Some(value.into());
    self
  }

  /// Sets top outer spacing for this element.
  #[must_use]
  pub fn margin_top(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.margin_top = Some(value.into());
    self
  }

  /// Expands CSS-order outer spacing into the four margin fields.
  #[must_use]
  pub fn margin(mut self, value: impl IntoStyleSides<LengthOrAuto>) -> Self {
    [
      self.margin_top,
      self.margin_right,
      self.margin_bottom,
      self.margin_left,
    ] = value.into_style_sides().map(Some);
    self
  }

  /// Sets the maximum border-box height after flex sizing.
  #[must_use]
  pub fn max_height(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.max_height = Some(value.into());
    self
  }

  /// Sets the maximum border-box width after flex sizing.
  #[must_use]
  pub fn max_width(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.max_width = Some(value.into());
    self
  }

  /// Sets the minimum border-box height constraining automatic or flex sizing.
  #[must_use]
  pub fn min_height(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.min_height = Some(value.into());
    self
  }

  /// Sets the minimum border-box width constraining automatic or flex sizing.
  #[must_use]
  pub fn min_width(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.min_width = Some(value.into());
    self
  }

  /// Sets subtree opacity from transparent zero through opaque one.
  #[must_use]
  pub fn opacity(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.opacity = Some(value.into());
    self
  }

  /// Allows descendant painting outside bounds or clips it at the selected box.
  #[must_use]
  pub fn overflow(mut self, value: impl Into<StyleValue<Overflow>>) -> Self {
    self.overflow = Some(value.into());
    self
  }

  /// Sets nonnegative bottom inner spacing.
  #[must_use]
  pub fn padding_bottom(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.padding_bottom = Some(value.into());
    self
  }

  /// Sets nonnegative left inner spacing.
  #[must_use]
  pub fn padding_left(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.padding_left = Some(value.into());
    self
  }

  /// Sets nonnegative right inner spacing.
  #[must_use]
  pub fn padding_right(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.padding_right = Some(value.into());
    self
  }

  /// Sets nonnegative top inner spacing.
  #[must_use]
  pub fn padding_top(mut self, value: impl Into<StyleValue<Length>>) -> Self {
    self.padding_top = Some(value.into());
    self
  }

  /// Expands CSS-order inner spacing into the four padding fields.
  #[must_use]
  pub fn padding(mut self, value: impl IntoStyleSides<Length>) -> Self {
    [
      self.padding_top,
      self.padding_right,
      self.padding_bottom,
      self.padding_left,
    ] = value.into_style_sides().map(Some);
    self
  }

  /// Selects relative flex-flow positioning or parent-relative absolute positioning.
  #[must_use]
  pub fn position(mut self, value: impl Into<StyleValue<Position>>) -> Self {
    self.position = Some(value.into());
    self
  }

  /// Sets the right offset used by relative or absolute positioning.
  #[must_use]
  pub fn right(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.right = Some(value.into());
    self
  }

  /// Sets a paint-time rotation around the element's transform origin.
  ///
  /// Rotation does not affect flex layout and follows scale in Unity's fixed
  /// transform order.
  #[must_use]
  pub fn rotate(mut self, value: impl Into<StyleValue<Rotate>>) -> Self {
    self.rotate = Some(value.into());
    self
  }

  /// Sets paint-time horizontal and vertical size multipliers.
  ///
  /// Scale affects the element and descendants without reserving different
  /// layout space. Negative factors mirror along the corresponding axis.
  #[must_use]
  pub fn scale(mut self, value: impl Into<StyleValue<Scale>>) -> Self {
    self.scale = Some(value.into());
    self
  }

  /// Sets the top offset used by relative or absolute positioning.
  #[must_use]
  pub fn top(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.top = Some(value.into());
    self
  }

  /// Sets the pivot used by scale and rotation.
  ///
  /// Percentage coordinates resolve against this element's bounds; pixel and
  /// percentage values may intentionally place the pivot outside the element.
  #[must_use]
  pub fn transform_origin(mut self, value: impl Into<StyleValue<TransformOrigin>>) -> Self {
    self.transform_origin = Some(value.into());
    self
  }

  /// Sets transition delays in milliseconds.
  ///
  /// Values repeat across the transition-property list. Negative delays skip
  /// the corresponding amount of the animation when the transition begins.
  #[must_use]
  pub fn transition_delay(
    mut self,
    value: impl Into<StyleValue<TransitionList<TimeValue>>>,
  ) -> Self {
    self.transition_delay = Some(value.into());
    self
  }

  /// Sets nonnegative transition durations in milliseconds.
  ///
  /// Values repeat cyclically when fewer durations than properties are
  /// authored. A zero duration changes the property without interpolation.
  #[must_use]
  pub fn transition_duration(
    mut self,
    value: impl Into<StyleValue<TransitionList<TimeValue>>>,
  ) -> Self {
    self.transition_duration = Some(value.into());
    self
  }

  /// Selects the closed set of inline properties whose changes may transition.
  ///
  /// Each property has an independent lifecycle. [`TransitionProperty::All`]
  /// requests every animatable property supported by Unity.
  #[must_use]
  pub fn transition_property(
    mut self,
    value: impl Into<StyleValue<TransitionList<TransitionProperty>>>,
  ) -> Self {
    self.transition_property = Some(value.into());
    self
  }

  /// Sets interpolation curves repeated across the transition-property list.
  #[must_use]
  pub fn transition_timing_function(
    mut self,
    value: impl Into<StyleValue<TransitionList<EasingFunction>>>,
  ) -> Self {
    self.transition_timing_function = Some(value.into());
    self
  }

  /// Sets a paint-time offset relative to the element's own bounds.
  ///
  /// Translation follows scale and rotation and does not change sibling
  /// layout. Percentage x and y values resolve against this element.
  #[must_use]
  pub fn translate(mut self, value: impl Into<StyleValue<Translate>>) -> Self {
    self.translate = Some(value.into());
    self
  }

  /// Multiplies background-image pixels by a tint before compositing.
  #[must_use]
  pub fn unity_background_image_tint_color(mut self, value: impl Into<StyleValue<Color>>) -> Self {
    self.unity_background_image_tint_color = Some(value.into());
    self
  }

  /// Assigns a prepared custom material to this element's renderer.
  #[must_use]
  pub fn unity_material(mut self, value: impl Into<StyleValue<MaterialAddress>>) -> Self {
    self.unity_material = Some(value.into());
    self
  }

  /// Chooses whether hidden overflow clips at the padding or content box.
  #[must_use]
  pub fn unity_overflow_clip_box(mut self, value: impl Into<StyleValue<OverflowClipBox>>) -> Self {
    self.unity_overflow_clip_box = Some(value.into());
    self
  }

  /// Sets the nonnegative bottom nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_bottom(mut self, value: impl Into<StyleValue<i32>>) -> Self {
    self.unity_slice_bottom = Some(value.into());
    self
  }

  /// Sets the nonnegative left nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_left(mut self, value: impl Into<StyleValue<i32>>) -> Self {
    self.unity_slice_left = Some(value.into());
    self
  }

  /// Sets the nonnegative right nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_right(mut self, value: impl Into<StyleValue<i32>>) -> Self {
    self.unity_slice_right = Some(value.into());
    self
  }

  /// Scales all nine-slice insets by a positive multiplier.
  #[must_use]
  pub fn unity_slice_scale(mut self, value: impl Into<StyleValue<FloatValue>>) -> Self {
    self.unity_slice_scale = Some(value.into());
    self
  }

  /// Sets the nonnegative top nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_top(mut self, value: impl Into<StyleValue<i32>>) -> Self {
    self.unity_slice_top = Some(value.into());
    self
  }

  /// Selects stretched or repeated nine-slice center and edge regions.
  #[must_use]
  pub fn unity_slice_type(mut self, value: impl Into<StyleValue<SliceType>>) -> Self {
    self.unity_slice_type = Some(value.into());
    self
  }

  /// Shows the element or hides it while preserving layout space.
  #[must_use]
  pub fn visibility(mut self, value: impl Into<StyleValue<Visibility>>) -> Self {
    self.visibility = Some(value.into());
    self
  }

  /// Sets the preferred border-box width.
  #[must_use]
  pub fn width(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
    self.width = Some(value.into());
    self
  }

  /// Returns whether this value contributes no inline declarations.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self == &Self::default()
  }
}

macro_rules! style_value_from_numeric {
  ($target:ty) => {
    impl From<i32> for StyleValue<$target> {
      fn from(value: i32) -> Self {
        Self::Value(value.into())
      }
    }

    impl From<u32> for StyleValue<$target> {
      fn from(value: u32) -> Self {
        Self::Value(value.into())
      }
    }

    impl From<f32> for StyleValue<$target> {
      fn from(value: f32) -> Self {
        Self::Value(value.into())
      }
    }
  };
}

style_value_from_numeric!(Length);
style_value_from_numeric!(LengthOrAuto);
style_value_from_numeric!(FloatValue);

macro_rules! style_value_from_concrete {
    ($($target:ty),+ $(,)?) => {$(
        impl From<$target> for StyleValue<$target> {
            fn from(value: $target) -> Self {
                Self::Value(value)
            }
        }
    )+};
}

style_value_from_concrete!(
  Align,
  AspectRatio,
  BackgroundPosition,
  BackgroundRepeat,
  BackgroundSize,
  Display,
  EasingFunction,
  FilterList,
  FlexDirection,
  FlexWrap,
  FloatValue,
  Justify,
  Length,
  LengthOrAuto,
  Overflow,
  OverflowClipBox,
  Position,
  Rotate,
  Scale,
  SliceType,
  TimeValue,
  TransformOrigin,
  TransitionList<TimeValue>,
  TransitionList<TransitionProperty>,
  TransitionList<EasingFunction>,
  Translate,
  Visibility,
);

impl From<i32> for StyleValue<i32> {
  fn from(value: i32) -> Self {
    Self::Value(value)
  }
}

impl From<Length> for StyleValue<LengthOrAuto> {
  fn from(value: Length) -> Self {
    Self::Value(value.into())
  }
}
