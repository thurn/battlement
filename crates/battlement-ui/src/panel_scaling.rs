use battlement_types::ScreenSize;

/// Strategy for converting authored UI dimensions to display pixels.
///
/// Each variant owns only the values that affect that scaling strategy. The
/// checked wrapper values can only be obtained through the constructors on
/// this enum and [`PanelScreenMatchMode`].
///
/// ```compile_fail
/// use battlement_ui::{PanelPixelScale, PanelScaleMode};
///
/// let _ = PanelScaleMode::ConstantPixelSize(PanelPixelScale { value: f32::NAN });
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelScaleMode {
  /// Multiplies authored pixel sizes uniformly by a positive scale.
  ConstantPixelSize(PanelPixelScale),
  /// Maps authored pixel sizes to CSS pixels or native logical screen pixels.
  ConstantLogicalPixelSize,
  /// Preserves physical size using the display DPI or fallback DPI.
  ConstantPhysicalSize(PanelPhysicalScale),
  /// Scales relative to a nonzero reference resolution.
  ScaleWithScreenSize(PanelScreenScale),
}

/// A checked positive multiplier for constant-pixel-size scaling.
///
/// Construct this through [`PanelScaleMode::constant_pixel_size`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelPixelScale {
  value: f32,
}

impl PanelPixelScale {
  /// Returns the checked positive pixel multiplier.
  #[must_use]
  pub const fn value(self) -> f32 {
    self.value
  }
}

/// Checked design and fallback densities for physical-size scaling.
///
/// Construct this through [`PanelScaleMode::constant_physical_size`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelPhysicalScale {
  reference_dpi: f32,
  fallback_dpi: f32,
}

impl PanelPhysicalScale {
  /// Returns the checked design density.
  #[must_use]
  pub const fn reference_dpi(self) -> f32 {
    self.reference_dpi
  }

  /// Returns the checked fallback density.
  #[must_use]
  pub const fn fallback_dpi(self) -> f32 {
    self.fallback_dpi
  }
}

/// Checked reference resolution and matching strategy for screen-size scaling.
///
/// Construct this through [`PanelScaleMode::scale_with_screen_size`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelScreenScale {
  reference_resolution: ScreenSize,
  screen_match_mode: PanelScreenMatchMode,
}

impl PanelScreenScale {
  /// Returns the checked reference resolution.
  #[must_use]
  pub const fn reference_resolution(self) -> ScreenSize {
    self.reference_resolution
  }

  /// Returns the checked aspect-ratio matching strategy.
  #[must_use]
  pub const fn screen_match_mode(self) -> PanelScreenMatchMode {
    self.screen_match_mode
  }
}

impl PanelScaleMode {
  /// Creates constant-pixel scaling with a positive finite multiplier.
  #[must_use]
  pub fn constant_pixel_size(scale: f32) -> Self {
    Self::ConstantPixelSize(PanelPixelScale {
      value: checked_positive(scale, "panel pixel scale"),
    })
  }

  /// Creates logical-pixel scaling using the host display's logical-pixel factor.
  #[must_use]
  pub const fn constant_logical_pixel_size() -> Self {
    Self::ConstantLogicalPixelSize
  }

  /// Creates physical-size scaling with positive finite design and fallback densities.
  #[must_use]
  pub fn constant_physical_size(reference_dpi: f32, fallback_dpi: f32) -> Self {
    Self::ConstantPhysicalSize(PanelPhysicalScale {
      reference_dpi: checked_positive(reference_dpi, "panel reference DPI"),
      fallback_dpi: checked_positive(fallback_dpi, "panel fallback DPI"),
    })
  }

  /// Creates screen-size scaling with a nonzero reference resolution and match strategy.
  #[must_use]
  pub fn scale_with_screen_size(
    reference_resolution: ScreenSize,
    screen_match_mode: PanelScreenMatchMode,
  ) -> Self {
    assert!(
      reference_resolution.width > 0 && reference_resolution.height > 0,
      "panel reference resolution must be positive"
    );
    Self::ScaleWithScreenSize(PanelScreenScale {
      reference_resolution,
      screen_match_mode,
    })
  }
}

impl Default for PanelScaleMode {
  fn default() -> Self {
    Self::constant_physical_size(default_dpi(), default_dpi())
  }
}

/// Strategy for reconciling target and reference aspect ratios.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelScreenMatchMode {
  /// Blends width- and height-based ratios using a factor in `0..=1`.
  MatchWidthOrHeight(PanelMatchFactor),
  /// Uses the smaller ratio so the reference resolution fits within the target.
  Shrink,
  /// Uses the larger ratio so the reference resolution covers the target.
  Expand,
}

/// A checked width/height interpolation factor in `0..=1`.
///
/// Construct this through [`PanelScreenMatchMode::match_width_or_height`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelMatchFactor {
  value: f32,
}

impl PanelMatchFactor {
  /// Returns the checked width/height interpolation factor.
  #[must_use]
  pub const fn value(self) -> f32 {
    self.value
  }
}

impl PanelScreenMatchMode {
  /// Creates width/height matching with a finite factor in `0..=1`.
  #[must_use]
  pub fn match_width_or_height(match_factor: f32) -> Self {
    Self::MatchWidthOrHeight(PanelMatchFactor {
      value: checked_match_factor(match_factor),
    })
  }

  /// Uses the smaller width or height scale ratio.
  #[must_use]
  pub const fn shrink() -> Self {
    Self::Shrink
  }

  /// Uses the larger width or height scale ratio.
  #[must_use]
  pub const fn expand() -> Self {
    Self::Expand
  }
}

impl Default for PanelScreenMatchMode {
  fn default() -> Self {
    Self::match_width_or_height(0.0)
  }
}

pub(crate) fn default_dpi() -> f32 {
  96.0
}
fn checked_positive(value: f32, name: &str) -> f32 {
  assert!(
    value.is_finite() && value > 0.0,
    "{name} must be finite and positive"
  );
  value
}
fn checked_match_factor(value: f32) -> f32 {
  assert!(
    value.is_finite() && (0.0..=1.0).contains(&value),
    "panel match factor must be finite and in 0..=1"
  );
  value
}
