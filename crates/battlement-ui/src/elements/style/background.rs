use battlement_types::TextureAddress;
use serde::{Deserialize, Serialize};

use super::{Length, LengthOrAuto};

/// How the center and edges of a nine-sliced background are painted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SliceType {
  /// Stretches the center and edge regions between fixed corners.
  Sliced,
  /// Repeats the center and edge regions between fixed corners.
  Tiled,
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
