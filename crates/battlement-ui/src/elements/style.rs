use std::{fmt, marker::PhantomData};

use battlement_types::{Color, MaterialAddress, UiFontAddress};
use serde::{
  Deserialize, Deserializer, Serialize, Serializer,
  de::{self, SeqAccess, Visitor},
  ser::SerializeTuple,
};

use crate::Prop;
use crate::elements::background::BackgroundSource;

mod background;
mod filter;
mod layout;
mod text;
mod transform;
mod transition;

pub use background::*;
pub use filter::*;
pub use layout::*;
pub use text::*;
pub use transform::*;
pub use transition::*;

/// Explicit USS keyword accepted by every inline style property.
///
/// Use this when an update must explicitly author Unity's initial keyword.
/// Use [`Prop::Reset`] to remove the inline declaration; leaving a [`Style`]
/// field absent preserves the current inline value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InlineKeyword {
  /// Authors the property's Unity initial keyword.
  Initial,
}

/// One concrete inline value or an explicit USS keyword.
///
/// Values serialize as `[0, value]`; keywords serialize as `[1, keyword]`.
/// [`Prop::Reset`] serializes as `null`, while an omitted [`Style`] field leaves
/// the current value unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StyleValue<T> {
  /// Assigns a concrete property value.
  Value(T),
  /// Assigns an explicit USS keyword.
  Keyword {
    /// Keyword sent to Unity's inline style.
    value: InlineKeyword,
  },
}

impl<T: Serialize> Serialize for StyleValue<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut tuple = serializer.serialize_tuple(2)?;
    match self {
      Self::Value(value) => {
        tuple.serialize_element(&0_u8)?;
        tuple.serialize_element(value)?;
      }
      Self::Keyword { value } => {
        tuple.serialize_element(&1_u8)?;
        tuple.serialize_element(value)?;
      }
    }
    tuple.end()
  }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for StyleValue<T> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct StyleValueVisitor<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for StyleValueVisitor<T> {
      type Value = StyleValue<T>;

      fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a two-item UI style value array")
      }

      fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
      where
        A: SeqAccess<'de>,
      {
        let kind = sequence
          .next_element::<u8>()?
          .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let value = match kind {
          0 => StyleValue::Value(
            sequence
              .next_element()?
              .ok_or_else(|| de::Error::invalid_length(1, &self))?,
          ),
          1 => StyleValue::Keyword {
            value: sequence
              .next_element()?
              .ok_or_else(|| de::Error::invalid_length(1, &self))?,
          },
          _ => return Err(de::Error::custom("unknown UI style value kind")),
        };
        if sequence.next_element::<de::IgnoredAny>()?.is_some() {
          return Err(de::Error::invalid_length(3, &self));
        }
        Ok(value)
      }
    }

    deserializer.deserialize_tuple(2, StyleValueVisitor(PhantomData))
  }
}

impl<T> From<InlineKeyword> for StyleValue<T> {
  fn from(value: InlineKeyword) -> Self {
    Self::Keyword { value }
  }
}

/// Converts an ordinary inline value or explicit property operation into a style property.
pub trait IntoStyleProp<T> {
  /// Produces the sparse property operation used by [`Style`].
  fn into_style_prop(self) -> Prop<StyleValue<T>>;
}

impl<T, V> IntoStyleProp<T> for V
where
  V: Into<StyleValue<T>>,
{
  fn into_style_prop(self) -> Prop<StyleValue<T>> {
    Prop::Set(self.into())
  }
}

impl<T, V> IntoStyleProp<T> for Option<V>
where
  V: Into<StyleValue<T>>,
{
  fn into_style_prop(self) -> Prop<StyleValue<T>> {
    self.map_or(Prop::Unset, |value| Prop::Set(value.into()))
  }
}

impl<T> IntoStyleProp<T> for Prop<StyleValue<T>> {
  fn into_style_prop(self) -> Prop<StyleValue<T>> {
    self
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
/// Resettable fields accept [`Prop::Reset`], serialize it as `null`, and
/// remove the live inline declaration so USS or Unity's initial style applies.
/// [`Prop::Unset`] omits the field and preserves the live inline declaration.
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
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_content: Prop<StyleValue<Align>>,
  /// Default cross-axis alignment applied to this flex container's children.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_items: Prop<StyleValue<Align>>,
  /// Cross-axis alignment of this item, overriding its container's alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_self: Prop<StyleValue<Align>>,
  /// Preferred width-to-height ratio used when at least one dimension is automatic.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub aspect_ratio: Prop<StyleValue<AspectRatio>>,
  /// Color painted behind the element's content and padding, inside its border.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_color: Prop<StyleValue<Color>>,
  /// Prepared image painted behind content and affected by background tint and slicing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_image: Prop<StyleValue<BackgroundSource>>,
  /// Horizontal background anchor and offset after image sizing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_position_x: Prop<StyleValue<BackgroundPosition>>,
  /// Vertical background anchor and offset after image sizing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_position_y: Prop<StyleValue<BackgroundPosition>>,
  /// Independent horizontal and vertical background tiling behavior.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_repeat: Prop<StyleValue<BackgroundRepeat>>,
  /// Intrinsic, fitted, covering, or explicit background-image dimensions.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub background_size: Prop<StyleValue<BackgroundSize>>,
  /// Color of the bottom border; it is visible only when the bottom width is positive.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_bottom_color: Prop<StyleValue<Color>>,
  /// Radius of the bottom-left corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_bottom_left_radius: Prop<StyleValue<Length>>,
  /// Radius of the bottom-right corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_bottom_right_radius: Prop<StyleValue<Length>>,
  /// Layout space, in pixels, reserved for the bottom border edge.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_bottom_width: Prop<StyleValue<FloatValue>>,
  /// Color of the left border; it is visible only when the left width is positive.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_left_color: Prop<StyleValue<Color>>,
  /// Layout space, in pixels, reserved for the left border edge.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_left_width: Prop<StyleValue<FloatValue>>,
  /// Color of the right border; it is visible only when the right width is positive.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_right_color: Prop<StyleValue<Color>>,
  /// Layout space, in pixels, reserved for the right border edge.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_right_width: Prop<StyleValue<FloatValue>>,
  /// Color of the top border; it is visible only when the top width is positive.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_top_color: Prop<StyleValue<Color>>,
  /// Radius of the top-left corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_top_left_radius: Prop<StyleValue<Length>>,
  /// Radius of the top-right corner, resolved against the element size and clamped by Unity.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_top_right_radius: Prop<StyleValue<Length>>,
  /// Layout space, in pixels, reserved for the top border edge.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub border_top_width: Prop<StyleValue<FloatValue>>,
  /// Bottom offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub bottom: Prop<StyleValue<LengthOrAuto>>,
  /// Foreground color inherited by text unless a descendant overrides it.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub color: Prop<StyleValue<Color>>,
  /// Runtime mouse cursor used while a pointer hovers this element.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub cursor: Prop<StyleValue<Cursor>>,
  /// Whether this element and its descendants participate in layout and rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub display: Prop<StyleValue<Display>>,
  /// Initial main-axis size before flex grow and shrink distribute free space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub flex_basis: Prop<StyleValue<LengthOrAuto>>,
  /// Direction and ordering of this flex container's main axis.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub flex_direction: Prop<StyleValue<FlexDirection>>,
  /// Nonnegative share of remaining main-axis space assigned to this item.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub flex_grow: Prop<StyleValue<FloatValue>>,
  /// Nonnegative shrink factor used when siblings exceed the main-axis space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub flex_shrink: Prop<StyleValue<FloatValue>>,
  /// Whether children remain on one line or wrap across the cross axis.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub flex_wrap: Prop<StyleValue<FlexWrap>>,
  /// Font size, in pixels, inherited by descendant text unless overridden.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub font_size: Prop<StyleValue<Length>>,
  /// Border-box height in pixels, percentage, automatic size, or initial value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub height: Prop<StyleValue<LengthOrAuto>>,
  /// Main-axis packing and free-space distribution for this container's children.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub justify_content: Prop<StyleValue<Justify>>,
  /// Inherited additional logical-pixel advance between glyphs; percentages use font size.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub letter_spacing: Prop<StyleValue<Length>>,
  /// Left offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub left: Prop<StyleValue<LengthOrAuto>>,
  /// Space outside the bottom border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub margin_bottom: Prop<StyleValue<LengthOrAuto>>,
  /// Space outside the left border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub margin_left: Prop<StyleValue<LengthOrAuto>>,
  /// Space outside the right border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub margin_right: Prop<StyleValue<LengthOrAuto>>,
  /// Space outside the top border; automatic values can absorb available space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub margin_top: Prop<StyleValue<LengthOrAuto>>,
  /// Maximum border-box height applied after preferred size and flex calculations.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub max_height: Prop<StyleValue<LengthOrAuto>>,
  /// Maximum border-box width applied after preferred size and flex calculations.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub max_width: Prop<StyleValue<LengthOrAuto>>,
  /// Minimum border-box height that constrains shrinking and automatic sizing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub min_height: Prop<StyleValue<LengthOrAuto>>,
  /// Minimum border-box width that constrains shrinking and automatic sizing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub min_width: Prop<StyleValue<LengthOrAuto>>,
  /// Element opacity multiplied through its rendered subtree, from transparent zero to opaque one.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub opacity: Prop<StyleValue<FloatValue>>,
  /// Whether descendant painting is clipped at this element's selected clip box.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub overflow: Prop<StyleValue<Overflow>>,
  /// Space between the bottom border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub padding_bottom: Prop<StyleValue<Length>>,
  /// Space between the left border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub padding_left: Prop<StyleValue<Length>>,
  /// Space between the right border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub padding_right: Prop<StyleValue<Length>>,
  /// Space between the top border and content; values must be nonnegative.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub padding_top: Prop<StyleValue<Length>>,
  /// Selects normal flex flow or independent placement against the parent box.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub position: Prop<StyleValue<Position>>,
  /// Right offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub right: Prop<StyleValue<LengthOrAuto>>,
  /// Paint-time rotation around the authored transform origin.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub rotate: Prop<StyleValue<Rotate>>,
  /// Paint-time horizontal and vertical size multipliers.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub scale: Prop<StyleValue<Scale>>,
  /// Whether overflowing text is clipped or replaced with an ellipsis.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text_overflow: Prop<StyleValue<TextOverflow>>,
  /// Shadow rendered behind each glyph without affecting layout.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text_shadow: Prop<StyleValue<TextShadow>>,
  /// Top offset from normal flow or the containing block, depending on position mode.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub top: Prop<StyleValue<LengthOrAuto>>,
  /// Pivot against which scale and rotation are resolved.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub transform_origin: Prop<StyleValue<TransformOrigin>>,
  /// Per-property delays in milliseconds; negative values begin partway through a transition.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub transition_delay: Prop<StyleValue<TransitionList<TimeValue>>>,
  /// Nonnegative per-property transition durations in milliseconds.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub transition_duration: Prop<StyleValue<TransitionList<TimeValue>>>,
  /// Properties whose value changes should be interpolated.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub transition_property: Prop<StyleValue<TransitionList<TransitionProperty>>>,
  /// Per-property interpolation curves repeated across the transition-property list.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub transition_timing_function: Prop<StyleValue<TransitionList<EasingFunction>>>,
  /// Paint-time offset applied after scale and rotation without affecting layout.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub translate: Prop<StyleValue<Translate>>,
  /// Color multiplied with pixels from a background image before compositing.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_background_image_tint_color: Prop<StyleValue<Color>>,
  /// Selects signed-distance-field or bitmap editor text rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_editor_text_rendering_mode: Prop<StyleValue<EditorTextRenderingMode>>,
  /// Prepared TextCore font asset inherited by descendant text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_font_definition: Prop<StyleValue<UiFontAddress>>,
  /// Bold and italic selection inherited by descendant text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_font_style_and_weight: Prop<StyleValue<FontStyle>>,
  /// Prepared custom material used to render this element and inherited by descendants.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_material: Prop<StyleValue<MaterialAddress>>,
  /// Selects the padding or content box as the boundary for hidden overflow.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_overflow_clip_box: Prop<StyleValue<OverflowClipBox>>,
  /// Extra vertical advance inserted after each paragraph.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_paragraph_spacing: Prop<StyleValue<Length>>,
  /// Bottom inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_bottom: Prop<StyleValue<i32>>,
  /// Left inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_left: Prop<StyleValue<i32>>,
  /// Right inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_right: Prop<StyleValue<i32>>,
  /// Positive multiplier applied to nine-slice inset sizes.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_scale: Prop<StyleValue<FloatValue>>,
  /// Top inset, in source pixels, preserved by nine-slice background rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_top: Prop<StyleValue<i32>>,
  /// Selects stretched or repeated center and edge regions for nine-slice backgrounds.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_slice_type: Prop<StyleValue<SliceType>>,
  /// Alignment of text within the content rectangle.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_align: Prop<StyleValue<TextAnchor>>,
  /// Optional best-fit font sizing within positive pixel bounds.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_auto_size: Prop<StyleValue<TextAutoSize>>,
  /// Text generation backend used for glyph layout and rendering.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_generator: Prop<StyleValue<TextGenerator>>,
  /// Color of the stroke painted around every text glyph.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_outline_color: Prop<StyleValue<Color>>,
  /// Nonnegative text outline width in panel pixels.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_outline_width: Prop<StyleValue<FloatValue>>,
  /// Portion of an overflowing string preserved around its ellipsis.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub unity_text_overflow_position: Prop<StyleValue<TextOverflowPosition>>,
  /// Whether the element is drawn while retaining its layout space.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub visibility: Prop<StyleValue<Visibility>>,
  /// Controls newline preservation, space collapsing, and automatic wrapping.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub white_space: Prop<StyleValue<WhiteSpace>>,
  /// Border-box width in pixels, percentage, automatic size, or initial value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub width: Prop<StyleValue<LengthOrAuto>>,
  /// Additional advance inserted at word boundaries; negative values tighten text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub word_spacing: Prop<StyleValue<Length>>,
}

impl Style {
  /// Creates an empty set of inline declarations.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets width and height together.
  #[must_use]
  pub fn size(
    self,
    width: impl IntoStyleProp<LengthOrAuto>,
    height: impl IntoStyleProp<LengthOrAuto>,
  ) -> Self {
    self.width(width).height(height)
  }

  /// Fills the parent's content box in both dimensions.
  #[must_use]
  pub fn full_size(self) -> Self {
    self.size(Length::percent(100.0), Length::percent(100.0))
  }

  /// Pins an absolutely positioned element to every parent edge.
  #[must_use]
  pub fn absolute_fill(self) -> Self {
    self
      .position(Position::Absolute)
      .top(0)
      .right(0)
      .bottom(0)
      .left(0)
  }

  /// Sets CSS-order offsets on all four positioned edges.
  #[must_use]
  pub fn inset(mut self, value: impl IntoStyleSides<LengthOrAuto>) -> Self {
    [self.top, self.right, self.bottom, self.left] = value.into_style_sides().map(Prop::Set);
    self
  }

  /// Centers children on both flex axes.
  #[must_use]
  pub fn center_content(self) -> Self {
    self
      .align_items(Align::Center)
      .justify_content(Justify::Center)
  }

  /// Sets a vertical-only paint translation.
  #[must_use]
  pub fn translate_y(self, value: impl Into<Length>) -> Self {
    self.translate(Translate::two_dimensional(Length::px(0.0), value.into()))
  }

  /// Overlays populated declarations from `value`, preserving absent fields.
  #[must_use]
  pub fn merge(mut self, value: Self) -> Self {
    macro_rules! merge_fields {
            ($($field:ident),+ $(,)?) => {$(
                self.$field = SparseStyleField::overlay(value.$field, self.$field);
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
  pub fn align_content(mut self, value: impl IntoStyleProp<Align>) -> Self {
    self.align_content = value.into_style_prop();
    self
  }

  /// Sets the default cross-axis alignment of direct children.
  #[must_use]
  pub fn align_items(mut self, value: impl IntoStyleProp<Align>) -> Self {
    self.align_items = value.into_style_prop();
    self
  }

  /// Overrides this item's cross-axis alignment within its flex container.
  #[must_use]
  pub fn align_self(mut self, value: impl IntoStyleProp<Align>) -> Self {
    self.align_self = value.into_style_prop();
    self
  }

  /// Sets the preferred ratio used while resolving automatic dimensions.
  #[must_use]
  pub fn aspect_ratio(mut self, value: impl IntoStyleProp<AspectRatio>) -> Self {
    self.aspect_ratio = value.into_style_prop();
    self
  }

  /// Paints a color behind the element's content and padding.
  #[must_use]
  pub fn background_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.background_color = value.into_style_prop();
    self
  }

  /// Paints a prepared image behind the element so background sizing, tinting, and slicing can affect it.
  #[must_use]
  pub fn background_image(mut self, value: impl IntoStyleProp<BackgroundSource>) -> Self {
    self.background_image = value.into_style_prop();
    self
  }

  /// Positions the background horizontally from left, center, or right.
  #[must_use]
  pub fn background_position_x(mut self, value: impl IntoStyleProp<BackgroundPosition>) -> Self {
    self.background_position_x = value.into_style_prop();
    self
  }

  /// Positions the background vertically from top, center, or bottom.
  #[must_use]
  pub fn background_position_y(mut self, value: impl IntoStyleProp<BackgroundPosition>) -> Self {
    self.background_position_y = value.into_style_prop();
    self
  }

  /// Selects independent horizontal and vertical background tiling modes.
  #[must_use]
  pub fn background_repeat(mut self, value: impl IntoStyleProp<BackgroundRepeat>) -> Self {
    self.background_repeat = value.into_style_prop();
    self
  }

  /// Selects intrinsic, fitted, covering, or explicit background dimensions.
  #[must_use]
  pub fn background_size(mut self, value: impl IntoStyleProp<BackgroundSize>) -> Self {
    self.background_size = value.into_style_prop();
    self
  }

  /// Sets the bottom border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_bottom_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.border_bottom_color = value.into_style_prop();
    self
  }

  /// Rounds the bottom-left corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_bottom_left_radius(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.border_bottom_left_radius = value.into_style_prop();
    self
  }

  /// Rounds the bottom-right corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_bottom_right_radius(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.border_bottom_right_radius = value.into_style_prop();
    self
  }

  /// Reserves a nonnegative pixel width for the bottom border.
  #[must_use]
  pub fn border_bottom_width(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.border_bottom_width = value.into_style_prop();
    self
  }

  /// Sets the left border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_left_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.border_left_color = value.into_style_prop();
    self
  }

  /// Reserves a nonnegative pixel width for the left border.
  #[must_use]
  pub fn border_left_width(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.border_left_width = value.into_style_prop();
    self
  }

  /// Sets the right border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_right_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.border_right_color = value.into_style_prop();
    self
  }

  /// Reserves a nonnegative pixel width for the right border.
  #[must_use]
  pub fn border_right_width(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.border_right_width = value.into_style_prop();
    self
  }

  /// Sets the top border color; a positive width is required to draw it.
  #[must_use]
  pub fn border_top_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.border_top_color = value.into_style_prop();
    self
  }

  /// Rounds the top-left corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_top_left_radius(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.border_top_left_radius = value.into_style_prop();
    self
  }

  /// Rounds the top-right corner by a nonnegative pixel or percentage radius.
  #[must_use]
  pub fn border_top_right_radius(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.border_top_right_radius = value.into_style_prop();
    self
  }

  /// Reserves a nonnegative pixel width for the top border.
  #[must_use]
  pub fn border_top_width(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.border_top_width = value.into_style_prop();
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
    ] = value.into_style_sides().map(Prop::Set);
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
    ] = value.into_style_corners().map(Prop::Set);
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
    ] = value.into_style_sides().map(Prop::Set);
    self
  }

  /// Sets the bottom offset used by relative or absolute positioning.
  #[must_use]
  pub fn bottom(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.bottom = value.into_style_prop();
    self
  }

  /// Sets the text color inherited by this element's descendants.
  #[must_use]
  pub fn color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.color = value.into_style_prop();
    self
  }

  /// Sets the runtime mouse cursor shown while this element is hovered.
  #[must_use]
  pub fn cursor(mut self, value: impl IntoStyleProp<Cursor>) -> Self {
    self.cursor = value.into_style_prop();
    self
  }

  /// Selects flex participation or removes the subtree from layout and rendering.
  #[must_use]
  pub fn display(mut self, value: impl IntoStyleProp<Display>) -> Self {
    self.display = value.into_style_prop();
    self
  }

  /// Sets the item's initial main-axis size before flex distribution.
  #[must_use]
  pub fn flex_basis(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.flex_basis = value.into_style_prop();
    self
  }

  /// Selects the main-axis direction and child order.
  #[must_use]
  pub fn flex_direction(mut self, value: impl IntoStyleProp<FlexDirection>) -> Self {
    self.flex_direction = value.into_style_prop();
    self
  }

  /// Sets this item's nonnegative share of remaining main-axis space.
  #[must_use]
  pub fn flex_grow(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.flex_grow = value.into_style_prop();
    self
  }

  /// Sets this item's nonnegative shrink factor when space is insufficient.
  #[must_use]
  pub fn flex_shrink(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.flex_shrink = value.into_style_prop();
    self
  }

  /// Selects single-line or multi-line child placement.
  #[must_use]
  pub fn flex_wrap(mut self, value: impl IntoStyleProp<FlexWrap>) -> Self {
    self.flex_wrap = value.into_style_prop();
    self
  }

  /// Sets the inherited text size in pixels.
  #[must_use]
  pub fn font_size(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.font_size = value.into_style_prop();
    self
  }

  /// Sets additional advance between glyphs in pixels or font-relative percentage.
  #[must_use]
  pub fn letter_spacing(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.letter_spacing = value.into_style_prop();
    self
  }
  /// Chooses clipping or ellipsis when text exceeds its box.
  #[must_use]
  pub fn text_overflow(mut self, value: impl IntoStyleProp<TextOverflow>) -> Self {
    self.text_overflow = value.into_style_prop();
    self
  }
  /// Paints a shadow behind glyphs without changing layout.
  #[must_use]
  pub fn text_shadow(mut self, value: impl IntoStyleProp<TextShadow>) -> Self {
    self.text_shadow = value.into_style_prop();
    self
  }
  /// Selects the editor text rasterization mode.
  #[must_use]
  pub fn unity_editor_text_rendering_mode(
    mut self,
    value: impl IntoStyleProp<EditorTextRenderingMode>,
  ) -> Self {
    self.unity_editor_text_rendering_mode = value.into_style_prop();
    self
  }
  /// Assigns a prepared TextCore UI font asset.
  #[must_use]
  pub fn unity_font_definition(mut self, value: impl IntoStyleProp<UiFontAddress>) -> Self {
    self.unity_font_definition = value.into_style_prop();
    self
  }
  /// Selects normal, bold, italic, or bold-italic glyph styling.
  #[must_use]
  pub fn unity_font_style_and_weight(mut self, value: impl IntoStyleProp<FontStyle>) -> Self {
    self.unity_font_style_and_weight = value.into_style_prop();
    self
  }
  /// Adds vertical spacing after paragraphs.
  #[must_use]
  pub fn unity_paragraph_spacing(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.unity_paragraph_spacing = value.into_style_prop();
    self
  }
  /// Aligns text inside its content rectangle.
  #[must_use]
  pub fn unity_text_align(mut self, value: impl IntoStyleProp<TextAnchor>) -> Self {
    self.unity_text_align = value.into_style_prop();
    self
  }
  /// Enables or disables best-fit font sizing.
  #[must_use]
  pub fn unity_text_auto_size(mut self, value: impl IntoStyleProp<TextAutoSize>) -> Self {
    self.unity_text_auto_size = value.into_style_prop();
    self
  }
  /// Selects the standard or advanced text generator.
  #[must_use]
  pub fn unity_text_generator(mut self, value: impl IntoStyleProp<TextGenerator>) -> Self {
    self.unity_text_generator = value.into_style_prop();
    self
  }
  /// Sets the text-glyph outline color.
  #[must_use]
  pub fn unity_text_outline_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.unity_text_outline_color = value.into_style_prop();
    self
  }
  /// Sets the nonnegative text-glyph outline width in pixels.
  #[must_use]
  pub fn unity_text_outline_width(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.unity_text_outline_width = value.into_style_prop();
    self
  }
  /// Chooses which portion of an elided string remains visible.
  #[must_use]
  pub fn unity_text_overflow_position(
    mut self,
    value: impl IntoStyleProp<TextOverflowPosition>,
  ) -> Self {
    self.unity_text_overflow_position = value.into_style_prop();
    self
  }
  /// Chooses whitespace preservation and wrapping behavior.
  #[must_use]
  pub fn white_space(mut self, value: impl IntoStyleProp<WhiteSpace>) -> Self {
    self.white_space = value.into_style_prop();
    self
  }
  /// Sets additional advance at word boundaries.
  #[must_use]
  pub fn word_spacing(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.word_spacing = value.into_style_prop();
    self
  }

  /// Sets the preferred border-box height.
  #[must_use]
  pub fn height(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.height = value.into_style_prop();
    self
  }

  /// Distributes children and free space along the main axis.
  #[must_use]
  pub fn justify_content(mut self, value: impl IntoStyleProp<Justify>) -> Self {
    self.justify_content = value.into_style_prop();
    self
  }

  /// Sets the left offset used by relative or absolute positioning.
  #[must_use]
  pub fn left(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.left = value.into_style_prop();
    self
  }

  /// Sets bottom outer spacing for this element.
  #[must_use]
  pub fn margin_bottom(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.margin_bottom = value.into_style_prop();
    self
  }

  /// Sets left outer spacing for this element.
  #[must_use]
  pub fn margin_left(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.margin_left = value.into_style_prop();
    self
  }

  /// Sets right outer spacing for this element.
  #[must_use]
  pub fn margin_right(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.margin_right = value.into_style_prop();
    self
  }

  /// Sets top outer spacing for this element.
  #[must_use]
  pub fn margin_top(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.margin_top = value.into_style_prop();
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
    ] = value.into_style_sides().map(Prop::Set);
    self
  }

  /// Sets the maximum border-box height after flex sizing.
  #[must_use]
  pub fn max_height(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.max_height = value.into_style_prop();
    self
  }

  /// Sets the maximum border-box width after flex sizing.
  #[must_use]
  pub fn max_width(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.max_width = value.into_style_prop();
    self
  }

  /// Sets the minimum border-box height constraining automatic or flex sizing.
  #[must_use]
  pub fn min_height(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.min_height = value.into_style_prop();
    self
  }

  /// Sets the minimum border-box width constraining automatic or flex sizing.
  #[must_use]
  pub fn min_width(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.min_width = value.into_style_prop();
    self
  }

  /// Sets subtree opacity from transparent zero through opaque one.
  #[must_use]
  pub fn opacity(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.opacity = value.into_style_prop();
    self
  }

  /// Allows descendant painting outside bounds or clips it at the selected box.
  #[must_use]
  pub fn overflow(mut self, value: impl IntoStyleProp<Overflow>) -> Self {
    self.overflow = value.into_style_prop();
    self
  }

  /// Sets nonnegative bottom inner spacing.
  #[must_use]
  pub fn padding_bottom(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.padding_bottom = value.into_style_prop();
    self
  }

  /// Sets nonnegative left inner spacing.
  #[must_use]
  pub fn padding_left(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.padding_left = value.into_style_prop();
    self
  }

  /// Sets nonnegative right inner spacing.
  #[must_use]
  pub fn padding_right(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.padding_right = value.into_style_prop();
    self
  }

  /// Sets nonnegative top inner spacing.
  #[must_use]
  pub fn padding_top(mut self, value: impl IntoStyleProp<Length>) -> Self {
    self.padding_top = value.into_style_prop();
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
    ] = value.into_style_sides().map(Prop::Set);
    self
  }

  /// Selects relative flex-flow positioning or parent-relative absolute positioning.
  #[must_use]
  pub fn position(mut self, value: impl IntoStyleProp<Position>) -> Self {
    self.position = value.into_style_prop();
    self
  }

  /// Sets the right offset used by relative or absolute positioning.
  #[must_use]
  pub fn right(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.right = value.into_style_prop();
    self
  }

  /// Sets a paint-time rotation around the element's transform origin.
  ///
  /// Rotation does not affect flex layout and follows scale in Unity's fixed
  /// transform order.
  #[must_use]
  pub fn rotate(mut self, value: impl IntoStyleProp<Rotate>) -> Self {
    self.rotate = value.into_style_prop();
    self
  }

  /// Sets paint-time horizontal and vertical size multipliers.
  ///
  /// Scale affects the element and descendants without reserving different
  /// layout space. Negative factors mirror along the corresponding axis.
  #[must_use]
  pub fn scale(mut self, value: impl IntoStyleProp<Scale>) -> Self {
    self.scale = value.into_style_prop();
    self
  }

  /// Sets the top offset used by relative or absolute positioning.
  #[must_use]
  pub fn top(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.top = value.into_style_prop();
    self
  }

  /// Sets the pivot used by scale and rotation.
  ///
  /// Percentage coordinates resolve against this element's bounds; pixel and
  /// percentage values may intentionally place the pivot outside the element.
  #[must_use]
  pub fn transform_origin(mut self, value: impl IntoStyleProp<TransformOrigin>) -> Self {
    self.transform_origin = value.into_style_prop();
    self
  }

  /// Sets transition delays in milliseconds.
  ///
  /// Values repeat across the transition-property list. Negative delays skip
  /// the corresponding amount of the animation when the transition begins.
  #[must_use]
  pub fn transition_delay(mut self, value: impl IntoStyleProp<TransitionList<TimeValue>>) -> Self {
    self.transition_delay = value.into_style_prop();
    self
  }

  /// Sets nonnegative transition durations in milliseconds.
  ///
  /// Values repeat cyclically when fewer durations than properties are
  /// authored. A zero duration changes the property without interpolation.
  #[must_use]
  pub fn transition_duration(
    mut self,
    value: impl IntoStyleProp<TransitionList<TimeValue>>,
  ) -> Self {
    self.transition_duration = value.into_style_prop();
    self
  }

  /// Selects the closed set of inline properties whose changes may transition.
  ///
  /// Each property has an independent lifecycle. [`TransitionProperty::All`]
  /// requests every animatable property supported by Unity.
  #[must_use]
  pub fn transition_property(
    mut self,
    value: impl IntoStyleProp<TransitionList<TransitionProperty>>,
  ) -> Self {
    self.transition_property = value.into_style_prop();
    self
  }

  /// Sets interpolation curves repeated across the transition-property list.
  #[must_use]
  pub fn transition_timing_function(
    mut self,
    value: impl IntoStyleProp<TransitionList<EasingFunction>>,
  ) -> Self {
    self.transition_timing_function = value.into_style_prop();
    self
  }

  /// Sets a paint-time offset relative to the element's own bounds.
  ///
  /// Translation follows scale and rotation and does not change sibling
  /// layout. Percentage x and y values resolve against this element.
  #[must_use]
  pub fn translate(mut self, value: impl IntoStyleProp<Translate>) -> Self {
    self.translate = value.into_style_prop();
    self
  }

  /// Multiplies background-image pixels by a tint before compositing.
  #[must_use]
  pub fn unity_background_image_tint_color(mut self, value: impl IntoStyleProp<Color>) -> Self {
    self.unity_background_image_tint_color = value.into_style_prop();
    self
  }

  /// Assigns a prepared custom material to this element's renderer.
  #[must_use]
  pub fn unity_material(mut self, value: impl IntoStyleProp<MaterialAddress>) -> Self {
    self.unity_material = value.into_style_prop();
    self
  }

  /// Chooses whether hidden overflow clips at the padding or content box.
  #[must_use]
  pub fn unity_overflow_clip_box(mut self, value: impl IntoStyleProp<OverflowClipBox>) -> Self {
    self.unity_overflow_clip_box = value.into_style_prop();
    self
  }

  /// Sets the nonnegative bottom nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_bottom(mut self, value: impl IntoStyleProp<i32>) -> Self {
    self.unity_slice_bottom = value.into_style_prop();
    self
  }

  /// Sets the nonnegative left nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_left(mut self, value: impl IntoStyleProp<i32>) -> Self {
    self.unity_slice_left = value.into_style_prop();
    self
  }

  /// Sets the nonnegative right nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_right(mut self, value: impl IntoStyleProp<i32>) -> Self {
    self.unity_slice_right = value.into_style_prop();
    self
  }

  /// Scales all nine-slice insets by a positive multiplier.
  #[must_use]
  pub fn unity_slice_scale(mut self, value: impl IntoStyleProp<FloatValue>) -> Self {
    self.unity_slice_scale = value.into_style_prop();
    self
  }

  /// Sets the nonnegative top nine-slice inset in source pixels.
  #[must_use]
  pub fn unity_slice_top(mut self, value: impl IntoStyleProp<i32>) -> Self {
    self.unity_slice_top = value.into_style_prop();
    self
  }

  /// Selects stretched or repeated nine-slice center and edge regions.
  #[must_use]
  pub fn unity_slice_type(mut self, value: impl IntoStyleProp<SliceType>) -> Self {
    self.unity_slice_type = value.into_style_prop();
    self
  }

  /// Shows the element or hides it while preserving layout space.
  #[must_use]
  pub fn visibility(mut self, value: impl IntoStyleProp<Visibility>) -> Self {
    self.visibility = value.into_style_prop();
    self
  }

  /// Sets the preferred border-box width.
  #[must_use]
  pub fn width(mut self, value: impl IntoStyleProp<LengthOrAuto>) -> Self {
    self.width = value.into_style_prop();
    self
  }

  /// Returns whether this value contributes no inline declarations.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self == &Self::default()
  }
}

trait SparseStyleField: Sized {
  fn overlay(self, previous: Self) -> Self;
}

impl<T> SparseStyleField for Option<T> {
  fn overlay(self, previous: Self) -> Self {
    self.or(previous)
  }
}

impl<T> SparseStyleField for Prop<T> {
  fn overlay(self, previous: Self) -> Self {
    if self.is_unset() { previous } else { self }
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
  EditorTextRenderingMode,
  EasingFunction,
  FlexDirection,
  FlexWrap,
  FloatValue,
  FontStyle,
  Justify,
  Length,
  LengthOrAuto,
  Overflow,
  OverflowClipBox,
  Position,
  Rotate,
  Scale,
  SliceType,
  TextAnchor,
  TextAutoSize,
  TextGenerator,
  TextOverflow,
  TextOverflowPosition,
  TextShadow,
  TimeValue,
  TransformOrigin,
  TransitionList<TimeValue>,
  TransitionList<TransitionProperty>,
  TransitionList<EasingFunction>,
  Translate,
  Visibility,
  WhiteSpace,
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
