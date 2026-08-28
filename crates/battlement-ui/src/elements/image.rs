use battlement_types::{
  Color, Rect, RenderTextureAddress, SpriteAddress, TextureAddress, VectorImageAddress,
};
use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UsageHint, VisualElement, VisualElementProperties,
};

/// One prepared graphical asset that a Unity UI Toolkit image can display.
///
/// The variants are exclusive native source properties. Applying one source
/// clears Unity's texture, sprite, vector-image, and render-texture alternatives
/// before assigning the selected prepared asset.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ImageSource {
  /// A raster `Texture2D`, sampled directly by the image element.
  Texture(TextureAddress),
  /// A sprite retaining its imported rectangle, border, and mesh behavior.
  Sprite(SpriteAddress),
  /// A resolution-independent UI Toolkit vector image.
  VectorImage(VectorImageAddress),
  /// A live render target displayed through Unity's texture source property.
  RenderTexture(RenderTextureAddress),
}

impl ImageSource {
  /// Returns the Addressables key held by this source.
  #[must_use]
  pub fn address(&self) -> &str {
    match self {
      Self::Texture(value) => value.as_str(),
      Self::Sprite(value) => value.as_str(),
      Self::VectorImage(value) => value.as_str(),
      Self::RenderTexture(value) => value.as_str(),
    }
  }
}

impl From<TextureAddress> for ImageSource {
  fn from(value: TextureAddress) -> Self {
    Self::Texture(value)
  }
}

impl From<SpriteAddress> for ImageSource {
  fn from(value: SpriteAddress) -> Self {
    Self::Sprite(value)
  }
}

impl From<VectorImageAddress> for ImageSource {
  fn from(value: VectorImageAddress) -> Self {
    Self::VectorImage(value)
  }
}

impl From<RenderTextureAddress> for ImageSource {
  fn from(value: RenderTextureAddress) -> Self {
    Self::RenderTexture(value)
  }
}

impl From<TextureAddress> for Prop<ImageSource> {
  fn from(value: TextureAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<SpriteAddress> for Prop<ImageSource> {
  fn from(value: SpriteAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<VectorImageAddress> for Prop<ImageSource> {
  fn from(value: VectorImageAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<RenderTextureAddress> for Prop<ImageSource> {
  fn from(value: RenderTextureAddress) -> Self {
    Self::Set(value.into())
  }
}

/// Controls how a source image fits its element's content rectangle.
///
/// This maps to Unity's `ScaleMode`. Aspect-preserving modes either leave
/// unused space or crop overflow; stretching fills both axes independently.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ImageScaleMode {
  /// Preserves aspect ratio and fits the complete source inside the element.
  ScaleToFit,
  /// Preserves aspect ratio, fills the element, and crops overflowing source areas.
  ScaleAndCrop,
  /// Stretches the source independently on each axis to fill the element.
  StretchToFill,
}

/// A Unity UI Toolkit image for raster, sprite, vector, or rendered content.
///
/// Use `Image` when the graphic is primary content whose dimensions may
/// participate in layout and whose fit, crop, tint, or sampled region needs
/// direct control. Use a background style when the graphic is decorative.
/// Battlement images are logical leaves even though native `Image` can host
/// overlay children.
///
/// The selected source owns a usage lease until it is replaced or the element
/// is destroyed. Omitted fields preserve Unity defaults during creation and
/// preserve live values during sparse updates.
///
/// See Unity's [Image manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Image.html)
/// and [Image scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.Image.html).
///
/// # Example
///
/// ```
/// use battlement_types::{ObjectId, TextureAddress};
/// use battlement_ui::{Image, ImageScaleMode, UiNode};
///
/// let portrait = UiNode::new(
///     ObjectId::new_v4(),
///     Image::new()
///         .source(TextureAddress::new("ui/portrait"))
///         .scale_mode(ImageScaleMode::ScaleAndCrop),
/// );
///
/// assert!(portrait.children.is_empty());
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Image {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Exclusive prepared graphical source displayed by the native image.
  ///
  /// Replacing this field stages a new usage lease before native mutation.
  /// Sprite sources cannot be combined with [`Self::source_rect`].
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub source: Prop<ImageSource>,
  /// Pixel rectangle sampled from a non-sprite source, relative to its upper-left corner.
  ///
  /// Width and height must be nonnegative. Omit this for the full source, and
  /// do not use it with a sprite because the sprite already defines its source rectangle.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub source_rect: Prop<Rect>,
  /// Linear RGBA color multiplied with sampled source pixels.
  ///
  /// White preserves source colors; alpha attenuates the rendered source.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub tint_color: Prop<Color>,
  /// Fit and crop behavior inside the element's content rectangle.
  ///
  /// Unity defaults to [`ImageScaleMode::ScaleToFit`].
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub scale_mode: Prop<ImageScaleMode>,
  /// Normalized texture-coordinate rectangle measured from the lower-left corner.
  ///
  /// `(0, 0, 1, 1)` samples the full base texture. Coordinates and extents
  /// must be finite; values outside `0..=1` are rejected.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub uv: Prop<Rect>,
}

impl Image {
  /// Creates a leaf image that initially uses Unity's source and display defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Selects one prepared graphical source and its native source property.
  #[must_use]
  pub fn source(mut self, value: impl Into<Prop<ImageSource>>) -> Self {
    self.source = value.into();
    self
  }

  /// Samples an upper-left-origin pixel rectangle from a non-sprite source.
  #[must_use]
  pub fn source_rect(mut self, value: impl Into<Prop<Rect>>) -> Self {
    self.source_rect = value.into();
    self
  }

  /// Multiplies source pixels by a linear RGBA tint.
  #[must_use]
  pub fn tint_color(mut self, value: impl Into<Prop<Color>>) -> Self {
    self.tint_color = value.into();
    self
  }

  /// Chooses aspect-preserving fit/crop or independent-axis stretching.
  #[must_use]
  pub fn scale_mode(mut self, value: impl Into<Prop<ImageScaleMode>>) -> Self {
    self.scale_mode = value.into();
    self
  }

  /// Selects a lower-left-origin normalized texture-coordinate rectangle.
  #[must_use]
  pub fn uv(mut self, value: impl Into<Prop<Rect>>) -> Self {
    self.uv = value.into();
    self
  }

  impl_common_visual_element_methods!();

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.source, Prop::Unset) {
      self.source.clone_from(&value.source);
    }
    if !matches!(value.source_rect, Prop::Unset) {
      self.source_rect = value.source_rect;
    }
    if !matches!(value.tint_color, Prop::Unset) {
      self.tint_color = value.tint_color;
    }
    if !matches!(value.scale_mode, Prop::Unset) {
      self.scale_mode = value.scale_mode;
    }
    if !matches!(value.uv, Prop::Unset) {
      self.uv = value.uv;
    }
  }
}

impl VisualElementProperties for Image {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
