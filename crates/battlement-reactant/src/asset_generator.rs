//! Static generated-asset handles and their linked runtime catalog.
//!
//! Declarations compile into copyable handles without reading the Unity
//! project or launching the renderer.
//!
//! ```
//! use battlement_reactant::asset_generator;
//!
//! asset_generator::generate! {
//!   @background PANEL {
//!     @canvas 20px 10px;
//!     background: linear-gradient(red, blue);
//!   }
//! }
//!
//! assert_eq!(PANEL.canvas_size(), asset_generator::LogicalSize::new(20.0, 10.0));
//! let _image = PANEL.image();
//! let _style = PANEL.background_style();
//! ```

use battlement::{
  BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat, BackgroundRepeatMode,
  BackgroundSize, BackgroundSource, ImageSource, Length, PreparedAsset, Snapshot, Style,
  TextureAddress,
};

pub use battlement_reactant_asset_macros::{generate, generate_family};

#[doc(hidden)]
pub mod __private {
  pub use inventory::submit;
}

/// Logical generated texture dimensions in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalSize {
  /// Canvas width.
  pub width: f32,
  /// Canvas height.
  pub height: f32,
}

impl LogicalSize {
  /// Creates logical canvas dimensions.
  #[must_use]
  pub const fn new(width: f32, height: f32) -> Self {
    Self { width, height }
  }
}

/// Logical subject bounds within a generated canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalRect {
  /// Horizontal origin from the canvas left edge.
  pub x: f32,
  /// Vertical origin from the canvas top edge.
  pub y: f32,
  /// Subject width.
  pub width: f32,
  /// Subject height.
  pub height: f32,
}

impl LogicalRect {
  /// Creates logical subject bounds.
  #[must_use]
  pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
    Self {
      x,
      y,
      width,
      height,
    }
  }
}

/// Logical edge insets for a resizable generated texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalInsets {
  /// Top inset.
  pub top: f32,
  /// Right inset.
  pub right: f32,
  /// Bottom inset.
  pub bottom: f32,
  /// Left inset.
  pub left: f32,
}

impl LogicalInsets {
  /// Creates logical edge insets in CSS order.
  #[must_use]
  pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
    Self {
      top,
      right,
      bottom,
      left,
    }
  }
}

macro_rules! asset_methods {
  () => {
    /// Returns the stable generated `Texture2D` address.
    #[must_use]
    pub const fn texture_address(self) -> TextureAddress {
      TextureAddress::from_static(self.address)
    }

    /// Returns the logical generated canvas dimensions.
    #[must_use]
    pub const fn canvas_size(self) -> LogicalSize {
      self.canvas
    }

    /// Returns logical subject bounds within the canvas.
    #[must_use]
    pub const fn subject_bounds(self) -> LogicalRect {
      self.subject
    }

    /// Creates the shared image-source value for this texture.
    #[must_use]
    pub fn image_source(self) -> ImageSource {
      ImageSource::Texture(self.texture_address())
    }

    /// Creates one Reactant image façade using this texture.
    #[must_use]
    pub fn image(self) -> crate::host::ImageHost {
      crate::host::ImageHost::new().source(self.image_source())
    }
  };
}

/// Fixed generated background texture metadata.
///
/// ```
/// use battlement_reactant::asset_generator;
///
/// asset_generator::generate! {
///   @background EMBLEM {
///     @canvas 32px 16px;
///     box-shadow: 1px 2px black;
///   }
/// }
///
/// let _source = EMBLEM.image_source();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundAsset {
  address: &'static str,
  canvas: LogicalSize,
  subject: LogicalRect,
}

impl BackgroundAsset {
  /// Creates macro-generated background metadata.
  #[doc(hidden)]
  #[must_use]
  pub const fn __new(address: &'static str, canvas: LogicalSize, subject: LogicalRect) -> Self {
    Self {
      address,
      canvas,
      subject,
    }
  }

  asset_methods!();

  /// Creates paint-only background style for this texture.
  #[must_use]
  pub fn background_style(self) -> Style {
    self::background_style(self.address, self.canvas)
  }
}

/// Resizable generated background texture metadata.
///
/// ```
/// use battlement_reactant::asset_generator;
///
/// asset_generator::generate! {
///   @nine-slice FRAME {
///     @canvas 40px 20px;
///     @slices 2px 3px 2px 3px;
///     border: 1px dashed red;
///   }
/// }
///
/// assert_eq!(FRAME.slice_insets().left, 3.0);
/// let _style = FRAME.background_style();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NineSliceAsset {
  address: &'static str,
  canvas: LogicalSize,
  subject: LogicalRect,
  slices: LogicalInsets,
  raster_scale: u8,
}

impl NineSliceAsset {
  /// Creates macro-generated nine-slice metadata.
  #[doc(hidden)]
  #[must_use]
  pub const fn __new(
    address: &'static str,
    canvas: LogicalSize,
    subject: LogicalRect,
    slices: LogicalInsets,
    raster_scale: u8,
  ) -> Self {
    Self {
      address,
      canvas,
      subject,
      slices,
      raster_scale,
    }
  }

  asset_methods!();

  /// Returns logical nine-slice insets in CSS order.
  #[must_use]
  pub const fn slice_insets(self) -> LogicalInsets {
    self.slices
  }

  /// Creates paint-only nine-slice style for this texture.
  #[must_use]
  pub fn background_style(self) -> Style {
    Style::new()
      .background_image(BackgroundSource::Texture(self.texture_address()))
      .unity_slice_top(self::source_pixels(self.slices.top, self.raster_scale))
      .unity_slice_right(self::source_pixels(self.slices.right, self.raster_scale))
      .unity_slice_bottom(self::source_pixels(self.slices.bottom, self.raster_scale))
      .unity_slice_left(self::source_pixels(self.slices.left, self.raster_scale))
      .unity_slice_scale(1.0 / f32::from(self.raster_scale))
  }
}

/// Fixed generated text texture metadata.
///
/// ```
/// use battlement_reactant::asset_generator;
///
/// asset_generator::generate! {
///   @text-image TITLE {
///     @canvas 80px 24px;
///     @font-file unity("Assets/title.ttf");
///     content: "Ready";
///     font-size: 16px;
///     color: transparent;
///     background: linear-gradient(red, blue);
///     background-clip: text;
///   }
/// }
///
/// let _image = TITLE.image();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextImageAsset {
  address: &'static str,
  canvas: LogicalSize,
  subject: LogicalRect,
}

impl TextImageAsset {
  /// Creates macro-generated text-image metadata.
  #[doc(hidden)]
  #[must_use]
  pub const fn __new(address: &'static str, canvas: LogicalSize, subject: LogicalRect) -> Self {
    Self {
      address,
      canvas,
      subject,
    }
  }

  asset_methods!();

  /// Creates paint-only background style for this texture.
  #[must_use]
  pub fn background_style(self) -> Style {
    self::background_style(self.address, self.canvas)
  }
}

/// One linker-collected generated texture registration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetRegistration {
  /// Public generated texture address.
  pub address: &'static str,
  /// Logical canvas dimensions.
  pub canvas: LogicalSize,
  /// Logical subject bounds.
  pub subject: LogicalRect,
  /// Logical nine-slice insets, when applicable.
  pub slices: Option<LogicalInsets>,
  /// Source symbol retained for developer diagnostics.
  pub source_symbol: &'static str,
}

impl AssetRegistration {
  /// Creates one macro-generated linked registration.
  #[doc(hidden)]
  #[must_use]
  pub const fn __new(
    address: &'static str,
    canvas: LogicalSize,
    subject: LogicalRect,
    slices: Option<LogicalInsets>,
    source_symbol: &'static str,
  ) -> Self {
    Self {
      address,
      canvas,
      subject,
      slices,
      source_symbol,
    }
  }
}

inventory::collect!(AssetRegistration);

/// Enumerates linked generated-asset registrations in linker order.
pub fn registrations() -> impl Iterator<Item = &'static AssetRegistration> {
  inventory::iter::<AssetRegistration>.into_iter()
}

pub(crate) fn merge_into_snapshot(snapshot: &mut Snapshot) {
  let registrations = self::canonical_registrations();
  for asset in &snapshot.prepared_assets {
    let (case, address) = self::prepared_asset(asset);
    if !address.starts_with("battlement-reactant/generated/") {
      continue;
    }
    let source = registrations
      .iter()
      .find(|registration| registration.address == address)
      .map_or("<no linked source symbol>", |registration| {
        registration.source_symbol
      });
    panic!(
      "caller-authored PreparedAsset::{case} at reserved generated address {address} conflicts with linked source symbol {source}; the linked generated-asset registry exclusively owns battlement-reactant/generated/"
    );
  }
  snapshot.prepared_assets.extend(
    registrations.into_iter().map(|registration| {
      PreparedAsset::Texture(TextureAddress::from_static(registration.address))
    }),
  );
}

pub(crate) fn validate_discovered_asset(asset: &PreparedAsset) {
  let (_, address) = self::prepared_asset(asset);
  if address.starts_with("battlement-reactant/generated/") {
    assert!(
      matches!(asset, PreparedAsset::Texture(_))
        && self::registrations().any(|registration| registration.address == address),
      "generated asset reference is not owned by the linked registry: {address}"
    );
  }
}

fn canonical_registrations() -> Vec<&'static AssetRegistration> {
  let mut registrations = self::registrations().collect::<Vec<_>>();
  registrations.sort_by(|left, right| {
    left
      .address
      .cmp(right.address)
      .then_with(|| left.source_symbol.cmp(right.source_symbol))
  });
  let mut unique: Vec<&AssetRegistration> = Vec::new();
  for registration in registrations {
    let Some(previous) = unique.last() else {
      unique.push(registration);
      continue;
    };
    if previous.address != registration.address {
      unique.push(registration);
      continue;
    }
    if self::same_metadata(previous, registration) {
      continue;
    }
    panic!(
      "conflicting linked generated asset registrations at {}: {} metadata canvas={:?}, subject={:?}, slices={:?}; {} metadata canvas={:?}, subject={:?}, slices={:?}",
      registration.address,
      previous.source_symbol,
      previous.canvas,
      previous.subject,
      previous.slices,
      registration.source_symbol,
      registration.canvas,
      registration.subject,
      registration.slices,
    );
  }
  unique
}

fn same_metadata(left: &AssetRegistration, right: &AssetRegistration) -> bool {
  left.canvas == right.canvas && left.subject == right.subject && left.slices == right.slices
}

fn prepared_asset(asset: &PreparedAsset) -> (&'static str, &str) {
  match asset {
    PreparedAsset::Scene(value) => ("Scene", value.as_str()),
    PreparedAsset::Prefab(value) => ("Prefab", value.as_str()),
    PreparedAsset::ParticleEffect(value) => ("ParticleEffect", value.as_str()),
    PreparedAsset::Material(value) => ("Material", value.as_str()),
    PreparedAsset::Texture(value) => ("Texture", value.as_str()),
    PreparedAsset::Sprite(value) => ("Sprite", value.as_str()),
    PreparedAsset::VectorImage(value) => ("VectorImage", value.as_str()),
    PreparedAsset::RenderTexture(value) => ("RenderTexture", value.as_str()),
    PreparedAsset::AudioClip(value) => ("AudioClip", value.as_str()),
    PreparedAsset::TextMeshProFont(value) => ("TextMeshProFont", value.as_str()),
    PreparedAsset::UiFont(value) => ("UiFont", value.as_str()),
  }
}

fn background_style(address: &'static str, canvas: LogicalSize) -> Style {
  Style::new()
    .background_image(BackgroundSource::Texture(TextureAddress::from_static(
      address,
    )))
    .background_position_x(BackgroundPosition::new(
      BackgroundPositionKeyword::Left,
      Length::Px(0.0),
    ))
    .background_position_y(BackgroundPosition::new(
      BackgroundPositionKeyword::Top,
      Length::Px(0.0),
    ))
    .background_repeat(BackgroundRepeat::new(
      BackgroundRepeatMode::NoRepeat,
      BackgroundRepeatMode::NoRepeat,
    ))
    .background_size(BackgroundSize::axes(canvas.width, canvas.height))
}

fn source_pixels(logical: f32, raster_scale: u8) -> i32 {
  (logical * f32::from(raster_scale)).round() as i32
}
