use battlement_types::Color;
use serde::{Deserialize, Serialize};

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
    pub background_color: Option<Color>,
    /// Bottom offset from normal flow or the containing block, depending on position mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<StyleValue<LengthOrAuto>>,
    /// Foreground color inherited by text unless a descendant overrides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
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
    pub font_size: Option<f32>,
    /// Border-box height in pixels, percentage, automatic size, or initial value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<StyleValue<LengthOrAuto>>,
    /// Main-axis packing and free-space distribution for this container's children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<StyleValue<Justify>>,
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
    /// Top offset from normal flow or the containing block, depending on position mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<StyleValue<LengthOrAuto>>,
    /// Border-box width in pixels, percentage, automatic size, or initial value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<StyleValue<LengthOrAuto>>,
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
            bottom,
            color,
            flex_basis,
            flex_direction,
            flex_grow,
            flex_shrink,
            flex_wrap,
            font_size,
            height,
            justify_content,
            left,
            margin_bottom,
            margin_left,
            margin_right,
            margin_top,
            max_height,
            max_width,
            min_height,
            min_width,
            padding_bottom,
            padding_left,
            padding_right,
            padding_top,
            position,
            right,
            top,
            width,
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

    /// Paints `value` behind the element's content and padding.
    #[must_use]
    pub fn background_color(mut self, value: Color) -> Self {
        self.background_color = Some(value);
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
    pub fn color(mut self, value: Color) -> Self {
        self.color = Some(value);
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
    pub fn font_size(mut self, value: f32) -> Self {
        self.font_size = Some(value);
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

    /// Sets the top offset used by relative or absolute positioning.
    #[must_use]
    pub fn top(mut self, value: impl Into<StyleValue<LengthOrAuto>>) -> Self {
        self.top = Some(value.into());
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
    FlexDirection,
    FlexWrap,
    Justify,
    Length,
    LengthOrAuto,
    Position,
);

impl From<Length> for StyleValue<LengthOrAuto> {
    fn from(value: Length) -> Self {
        Self::Value(value.into())
    }
}
