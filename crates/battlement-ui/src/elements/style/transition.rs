use serde::{Deserialize, Serialize};

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
