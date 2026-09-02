//! Reusable scalar, mathematical, animation, and input values shared by protocol domains.

use serde::{Deserialize, Serialize};

/// A three-dimensional value in Unity world units.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Vector3 {
  /// The X component.
  pub x: f64,
  /// The Y component.
  pub y: f64,
  /// The Z component.
  pub z: f64,
}

impl Vector3 {
  /// The zero vector.
  pub const ZERO: Self = Self {
    x: 0.0,
    y: 0.0,
    z: 0.0,
  };

  /// The vector whose components are all one.
  pub const ONE: Self = Self {
    x: 1.0,
    y: 1.0,
    z: 1.0,
  };

  /// Creates a vector from its components.
  #[must_use]
  pub const fn new(x: f64, y: f64, z: f64) -> Self {
    Self { x, y, z }
  }
}

/// A rectangular grid embedded in three-dimensional space.
///
/// The origin is the center of cell `(0, 0)`. Column and row steps can point
/// along any axes, so the grid may lie on a floor, wall, or tilted surface.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GridLayout {
  /// Center of cell `(0, 0)`.
  pub origin: Vector3,
  /// Offset from one column to the next.
  pub column_step: Vector3,
  /// Offset from one row to the next.
  pub row_step: Vector3,
}

impl GridLayout {
  /// Creates a grid from its first cell and per-axis steps.
  #[must_use]
  pub const fn new(origin: Vector3, column_step: Vector3, row_step: Vector3) -> Self {
    Self {
      origin,
      column_step,
      row_step,
    }
  }

  /// Creates a grid centered around a point.
  ///
  /// Both dimensions must contain at least one cell.
  #[must_use]
  pub fn centered(
    center: Vector3,
    columns: u32,
    rows: u32,
    column_step: Vector3,
    row_step: Vector3,
  ) -> Self {
    assert!(columns > 0, "a centered grid requires at least one column");
    assert!(rows > 0, "a centered grid requires at least one row");
    let column_offset = (f64::from(columns) - 1.0) / 2.0;
    let row_offset = (f64::from(rows) - 1.0) / 2.0;
    Self::new(
      Vector3::new(
        center.x - column_step.x * column_offset - row_step.x * row_offset,
        center.y - column_step.y * column_offset - row_step.y * row_offset,
        center.z - column_step.z * column_offset - row_step.z * row_offset,
      ),
      column_step,
      row_step,
    )
  }

  /// Returns the center of a cell.
  #[must_use]
  pub fn position(self, column: u32, row: u32) -> Vector3 {
    let column = f64::from(column);
    let row = f64::from(row);
    Vector3::new(
      self.origin.x + self.column_step.x * column + self.row_step.x * row,
      self.origin.y + self.column_step.y * column + self.row_step.y * row,
      self.origin.z + self.column_step.z * column + self.row_step.z * row,
    )
  }
}

/// A two-dimensional screen position measured in pixels from the bottom-left.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ScreenPosition {
  /// The horizontal coordinate.
  pub x: f64,
  /// The vertical coordinate.
  pub y: f64,
}

impl ScreenPosition {
  /// Creates a screen position from its components.
  #[must_use]
  pub const fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }
}

/// A screen size in physical pixels.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScreenSize {
  /// Screen width in pixels.
  pub width: u32,
  /// Screen height in pixels.
  pub height: u32,
}

impl ScreenSize {
  /// Creates a screen size from its dimensions.
  #[must_use]
  pub const fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }
}

/// A Unity quaternion in `{x, y, z, w}` order.
///
/// The value must have nonzero length. Battlement normalizes it before use.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Quaternion {
  /// The X component.
  pub x: f64,
  /// The Y component.
  pub y: f64,
  /// The Z component.
  pub z: f64,
  /// The scalar component.
  pub w: f64,
}

impl Quaternion {
  /// The identity rotation.
  pub const IDENTITY: Self = Self {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 1.0,
  };

  /// Creates a quaternion from its components.
  #[must_use]
  pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
    Self { x, y, z, w }
  }
}

impl Default for Quaternion {
  fn default() -> Self {
    Self::IDENTITY
  }
}

/// A linear RGB color without alpha.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RgbColor {
  /// Red intensity in the inclusive range `[0, 1]`.
  pub r: f64,
  /// Green intensity in the inclusive range `[0, 1]`.
  pub g: f64,
  /// Blue intensity in the inclusive range `[0, 1]`.
  pub b: f64,
}

impl RgbColor {
  /// White in linear color space.
  pub const WHITE: Self = Self {
    r: 1.0,
    g: 1.0,
    b: 1.0,
  };

  /// Black in linear color space.
  pub const BLACK: Self = Self {
    r: 0.0,
    g: 0.0,
    b: 0.0,
  };

  /// Creates a linear RGB color from its components.
  #[must_use]
  pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
    Self { r, g, b }
  }
}

impl Default for RgbColor {
  fn default() -> Self {
    Self::WHITE
  }
}

/// A linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Color {
  /// Red intensity in the inclusive range `[0, 1]`.
  pub r: f64,
  /// Green intensity in the inclusive range `[0, 1]`.
  pub g: f64,
  /// Blue intensity in the inclusive range `[0, 1]`.
  pub b: f64,
  /// Alpha in the inclusive range `[0, 1]`.
  #[serde(default = "crate::default_one", skip_serializing_if = "crate::is_one")]
  pub a: f64,
}

impl Color {
  /// Opaque white in linear color space.
  pub const WHITE: Self = Self {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
  };

  /// Opaque black in linear color space.
  pub const BLACK: Self = Self {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
  };

  /// Creates an opaque linear RGB color.
  #[must_use]
  pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
    Self::rgba(r, g, b, 1.0)
  }

  /// Creates a linear RGBA color from its components.
  #[must_use]
  pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
    Self { r, g, b, a }
  }
}

impl Default for Color {
  fn default() -> Self {
    Self::WHITE
  }
}

/// A finite axis-aligned rectangle represented by its origin and size.
///
/// The coordinate origin depends on the consuming Unity API. UI image source
/// rectangles use upper-left-origin pixels, while image UV rectangles use
/// lower-left-origin normalized texture coordinates.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Rect {
  /// Horizontal coordinate of the rectangle origin.
  pub x: f64,
  /// Vertical coordinate of the rectangle origin.
  pub y: f64,
  /// Horizontal extent in the consuming coordinate system.
  pub width: f64,
  /// Vertical extent in the consuming coordinate system.
  pub height: f64,
}

impl Rect {
  /// Creates a rectangle from an origin and size.
  #[must_use]
  pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
    Self {
      x,
      y,
      width,
      height,
    }
  }
}

/// An object's local transform relative to its parent or scene container.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LocalTransform {
  /// Local position. Omission means [`Vector3::ZERO`].
  pub position: Vector3,
  /// Local rotation. Omission means [`Quaternion::IDENTITY`].
  pub rotation: Quaternion,
  /// Local scale. Omission means [`Vector3::ONE`].
  pub scale: Vector3,
}

impl LocalTransform {
  /// Creates an identity local transform.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }
}

impl Default for LocalTransform {
  fn default() -> Self {
    Self {
      position: Vector3::ZERO,
      rotation: Quaternion::IDENTITY,
      scale: Vector3::ONE,
    }
  }
}

/// The event kinds an object may emit after pointer raycasting.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PointerEvent {
  /// The pointer began hovering the object.
  Enter,
  /// The pointer stopped hovering the object.
  Exit,
  /// A pointer button was pressed over the object.
  Down,
  /// A pointer button was released over the object.
  Up,
  /// A press and release resolved to the same object.
  Click,
}

/// How a draggable object's position relates to the pointer at pickup.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DragMode {
  /// Move the object's center to the pointer immediately.
  SnapToPointer,
  /// Preserve the world-space offset between the pointer and object.
  PreserveOffset,
}

/// A mouse-style button reported with pointer button actions.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PointerButton {
  /// The primary mouse button, also used for touch.
  #[default]
  Left,
  /// The middle mouse button.
  Middle,
  /// The secondary mouse button.
  Right,
  /// A nonnegative native button index greater than two.
  Other(i32),
}

/// How a newly received batch relates to earlier blocking batches.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum BatchStart {
  /// Start as soon as scheduling permits.
  #[default]
  Now,
  /// Wait until blocking work in earlier batches has completed.
  AfterEarlierBlockingWork,
  /// Wait for earlier batches that prepare assets, without waiting for unrelated operations.
  AfterEarlierAssetPreparation,
}

/// What a property-writing command does when another operation controls the property.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConflictPolicy {
  /// Cancel the older operation and start from the displayed value.
  #[default]
  Cancel,
  /// Wait for the older operation to finish before starting.
  Wait,
}

/// How an image texture is fitted into its requested world-space dimensions.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ImageFit {
  /// Fill both dimensions without preserving aspect ratio.
  #[default]
  Stretch,
  /// Preserve aspect ratio and leave transparent space as needed.
  Contain,
  /// Preserve aspect ratio and crop centered UVs as needed.
  Cover,
}

/// A camera's projection mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum CameraProjection {
  /// Perspective projection.
  #[default]
  Perspective,
  /// Orthographic projection.
  Orthographic,
}

/// Which buffers a camera clears before rendering.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum CameraClearMode {
  /// Draw the configured skybox.
  #[default]
  Skybox,
  /// Fill with the configured clear color.
  SolidColor,
  /// Clear only depth.
  Depth,
  /// Do not clear.
  Nothing,
}

/// A standard Unity light type.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum LightType {
  /// A light with a direction but no position or range.
  Directional,
  /// A point light.
  #[default]
  Point,
  /// A spot light.
  Spot,
}

/// A light's shadow rendering mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShadowMode {
  /// Do not render shadows.
  #[default]
  None,
  /// Render hard-edged shadows.
  Hard,
  /// Render soft-edged shadows.
  Soft,
}

/// Horizontal alignment for world-space text.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum HorizontalAlignment {
  /// Align to the left edge.
  Left,
  /// Center the text.
  #[default]
  Center,
  /// Align to the right edge.
  Right,
  /// Expand spacing to align both edges.
  Justified,
}

/// Vertical alignment for world-space text.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum VerticalAlignment {
  /// Align to the top edge.
  Top,
  /// Center vertically.
  #[default]
  Middle,
  /// Align to the bottom edge.
  Bottom,
}

/// How a repeated tween begins its next traversal.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum RepeatMode {
  /// Jump to the captured start value and move forward again.
  #[default]
  Restart,
  /// Reverse direction for each additional traversal.
  PingPong,
}

/// A built-in easing curve supported by Battlement.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Easing {
  /// Linear interpolation.
  Linear,
  /// Sine ease in.
  InSine,
  /// Sine ease out.
  OutSine,
  /// Sine ease in and out.
  #[default]
  InOutSine,
  /// Quadratic ease in.
  InQuad,
  /// Quadratic ease out.
  OutQuad,
  /// Quadratic ease in and out.
  InOutQuad,
  /// Cubic ease in.
  InCubic,
  /// Cubic ease out.
  OutCubic,
  /// Cubic ease in and out.
  InOutCubic,
  /// Quartic ease in.
  InQuart,
  /// Quartic ease out.
  OutQuart,
  /// Quartic ease in and out.
  InOutQuart,
  /// Quintic ease in.
  InQuint,
  /// Quintic ease out.
  OutQuint,
  /// Quintic ease in and out.
  InOutQuint,
  /// Exponential ease in.
  InExpo,
  /// Exponential ease out.
  OutExpo,
  /// Exponential ease in and out.
  InOutExpo,
  /// Circular ease in.
  InCirc,
  /// Circular ease out.
  OutCirc,
  /// Circular ease in and out.
  InOutCirc,
  /// Overshooting ease in.
  InBack,
  /// Overshooting ease out.
  OutBack,
  /// Overshooting ease in and out.
  InOutBack,
  /// Elastic ease in.
  InElastic,
  /// Elastic ease out.
  OutElastic,
  /// Elastic ease in and out.
  InOutElastic,
  /// Bounce ease in.
  InBounce,
  /// Bounce ease out.
  OutBounce,
  /// Bounce ease in and out.
  InOutBounce,
}

/// Timing and repetition shared by all tween commands.
///
/// A zero-duration tween cannot repeat, and a forever tween must be
/// nonblocking.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Tween {
  /// Initial delay in milliseconds. Applied only before the first traversal.
  pub delay_ms: u64,
  /// Duration of one traversal in milliseconds. Zero applies the final value immediately.
  pub duration_ms: u64,
  /// Easing curve used for each traversal.
  pub easing: Easing,
  /// Whether and how the tween repeats after its first traversal.
  pub repeat: TweenRepeat,
}

impl Tween {
  /// Creates tween settings with their defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }
}

/// Repetition behavior after a tween's first traversal.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum TweenRepeat {
  /// Stop after the first traversal.
  #[default]
  Once,
  /// Perform a bounded number of additional traversals.
  Count {
    /// Number of traversals after the first.
    additional_traversals: u32,
    /// How each additional traversal proceeds.
    mode: RepeatMode,
  },
  /// Repeat until explicitly canceled.
  Forever(RepeatMode),
}

/// A physical W3C `KeyboardEvent.code` supported by Battlement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PhysicalKey {
  /// Escape.
  Escape,
  /// Function key F1.
  F1,
  /// Function key F2.
  F2,
  /// Function key F3.
  F3,
  /// Function key F4.
  F4,
  /// Function key F5.
  F5,
  /// Function key F6.
  F6,
  /// Function key F7.
  F7,
  /// Function key F8.
  F8,
  /// Function key F9.
  F9,
  /// Function key F10.
  F10,
  /// Function key F11.
  F11,
  /// Function key F12.
  F12,
  /// Backquote.
  Backquote,
  /// Digit 0.
  Digit0,
  /// Digit 1.
  Digit1,
  /// Digit 2.
  Digit2,
  /// Digit 3.
  Digit3,
  /// Digit 4.
  Digit4,
  /// Digit 5.
  Digit5,
  /// Digit 6.
  Digit6,
  /// Digit 7.
  Digit7,
  /// Digit 8.
  Digit8,
  /// Digit 9.
  Digit9,
  /// Minus.
  Minus,
  /// Equal.
  Equal,
  /// Backspace.
  Backspace,
  /// Tab.
  Tab,
  /// Letter A.
  KeyA,
  /// Letter B.
  KeyB,
  /// Letter C.
  KeyC,
  /// Letter D.
  KeyD,
  /// Letter E.
  KeyE,
  /// Letter F.
  KeyF,
  /// Letter G.
  KeyG,
  /// Letter H.
  KeyH,
  /// Letter I.
  KeyI,
  /// Letter J.
  KeyJ,
  /// Letter K.
  KeyK,
  /// Letter L.
  KeyL,
  /// Letter M.
  KeyM,
  /// Letter N.
  KeyN,
  /// Letter O.
  KeyO,
  /// Letter P.
  KeyP,
  /// Letter Q.
  KeyQ,
  /// Letter R.
  KeyR,
  /// Letter S.
  KeyS,
  /// Letter T.
  KeyT,
  /// Letter U.
  KeyU,
  /// Letter V.
  KeyV,
  /// Letter W.
  KeyW,
  /// Letter X.
  KeyX,
  /// Letter Y.
  KeyY,
  /// Letter Z.
  KeyZ,
  /// Left bracket.
  BracketLeft,
  /// Right bracket.
  BracketRight,
  /// Backslash.
  Backslash,
  /// Caps Lock.
  CapsLock,
  /// Semicolon.
  Semicolon,
  /// Quote.
  Quote,
  /// Enter.
  Enter,
  /// Left Shift.
  ShiftLeft,
  /// Right Shift.
  ShiftRight,
  /// Left Control.
  ControlLeft,
  /// Right Control.
  ControlRight,
  /// Left Alt.
  AltLeft,
  /// Right Alt.
  AltRight,
  /// Left Meta/Command/Windows.
  MetaLeft,
  /// Right Meta/Command/Windows.
  MetaRight,
  /// Comma.
  Comma,
  /// Period.
  Period,
  /// Slash.
  Slash,
  /// Space.
  Space,
  /// Context menu.
  ContextMenu,
  /// Insert.
  Insert,
  /// Delete.
  Delete,
  /// Home.
  Home,
  /// End.
  End,
  /// Page Up.
  PageUp,
  /// Page Down.
  PageDown,
  /// Left arrow.
  ArrowLeft,
  /// Right arrow.
  ArrowRight,
  /// Up arrow.
  ArrowUp,
  /// Down arrow.
  ArrowDown,
  /// Print Screen.
  PrintScreen,
  /// Scroll Lock.
  ScrollLock,
  /// Pause.
  Pause,
  /// Num Lock.
  NumLock,
  /// Numpad digit 0.
  Numpad0,
  /// Numpad digit 1.
  Numpad1,
  /// Numpad digit 2.
  Numpad2,
  /// Numpad digit 3.
  Numpad3,
  /// Numpad digit 4.
  Numpad4,
  /// Numpad digit 5.
  Numpad5,
  /// Numpad digit 6.
  Numpad6,
  /// Numpad digit 7.
  Numpad7,
  /// Numpad digit 8.
  Numpad8,
  /// Numpad digit 9.
  Numpad9,
  /// Numpad decimal separator.
  NumpadDecimal,
  /// Numpad addition.
  NumpadAdd,
  /// Numpad subtraction.
  NumpadSubtract,
  /// Numpad multiplication.
  NumpadMultiply,
  /// Numpad division.
  NumpadDivide,
  /// Numpad Enter.
  NumpadEnter,
}

/// A named controller button independent of platform-specific glyphs.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ControllerButton {
  /// Bottom face button: A on Xbox-style controllers, Cross on PlayStation controllers.
  South,
  /// Right face button: B on Xbox-style controllers, Circle on PlayStation controllers.
  East,
  /// Left face button: X on Xbox-style controllers, Square on PlayStation controllers.
  West,
  /// Top face button: Y on Xbox-style controllers, Triangle on PlayStation controllers.
  North,
  /// Left shoulder button: LB or L1.
  LeftShoulder,
  /// Right shoulder button: RB or R1.
  RightShoulder,
  /// Left-stick press.
  LeftStickButton,
  /// Right-stick press.
  RightStickButton,
  /// Primary system-menu button: Menu, Options, or Plus.
  Start,
  /// Secondary system-menu button: View, Create, Share, or Minus.
  Select,
}

/// A cardinal controller-navigation direction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ControllerDirection {
  /// Move left.
  Left,
  /// Move right.
  Right,
  /// Move up.
  Up,
  /// Move down.
  Down,
}

/// The physical control that produced a controller-navigation action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ControllerNavigationSource {
  /// The controller directional pad.
  Dpad,
  /// The controller left analog stick.
  LeftStick,
}
