use battlement_types::ScreenSize;
use serde::{Deserialize, Serialize};

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

/// Checked design and fallback densities for physical-size scaling.
///
/// Construct this through [`PanelScaleMode::constant_physical_size`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelPhysicalScale {
  reference_dpi: f32,
  fallback_dpi: f32,
}

/// Checked reference resolution and matching strategy for screen-size scaling.
///
/// Construct this through [`PanelScaleMode::scale_with_screen_size`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelScreenScale {
  reference_resolution: ScreenSize,
  screen_match_mode: PanelScreenMatchMode,
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

  pub(crate) fn wire(self) -> PanelScaleWire {
    match self {
      Self::ConstantPixelSize(PanelPixelScale { value }) => PanelScaleWire {
        scale_mode: PanelScaleModeWire::Pixel,
        scale: value,
        reference_dpi: default_dpi(),
        fallback_dpi: default_dpi(),
        reference_resolution: default_reference_resolution(),
        screen_match_mode: PanelScreenMatchModeWire::default(),
        match_factor: 0.0,
      },
      Self::ConstantLogicalPixelSize => PanelScaleWire {
        scale_mode: PanelScaleModeWire::LogicalPixel,
        scale: default_one(),
        reference_dpi: default_dpi(),
        fallback_dpi: default_dpi(),
        reference_resolution: default_reference_resolution(),
        screen_match_mode: PanelScreenMatchModeWire::default(),
        match_factor: 0.0,
      },
      Self::ConstantPhysicalSize(PanelPhysicalScale {
        reference_dpi,
        fallback_dpi,
      }) => PanelScaleWire {
        scale_mode: PanelScaleModeWire::Physical,
        scale: default_one(),
        reference_dpi,
        fallback_dpi,
        reference_resolution: default_reference_resolution(),
        screen_match_mode: PanelScreenMatchModeWire::default(),
        match_factor: 0.0,
      },
      Self::ScaleWithScreenSize(PanelScreenScale {
        reference_resolution,
        screen_match_mode,
      }) => {
        let (screen_match_mode, match_factor) = screen_match_mode.wire();
        PanelScaleWire {
          scale_mode: PanelScaleModeWire::ScreenSize,
          scale: default_one(),
          reference_dpi: default_dpi(),
          fallback_dpi: default_dpi(),
          reference_resolution,
          screen_match_mode,
          match_factor,
        }
      }
    }
  }

  pub(crate) fn from_wire(value: PanelScaleWire) -> Result<Self, String> {
    match value.scale_mode {
      PanelScaleModeWire::Pixel => {
        require_default(value.reference_dpi, default_dpi(), "reference_dpi")?;
        require_default(value.fallback_dpi, default_dpi(), "fallback_dpi")?;
        require_default(
          value.reference_resolution,
          default_reference_resolution(),
          "reference_resolution",
        )?;
        require_default(
          value.screen_match_mode,
          PanelScreenMatchModeWire::default(),
          "screen_match_mode",
        )?;
        require_default(value.match_factor, 0.0, "match_factor")?;
        Ok(Self::ConstantPixelSize(PanelPixelScale {
          value: serialized_positive(value.scale, "panel pixel scale")?,
        }))
      }
      PanelScaleModeWire::LogicalPixel => {
        require_default(value.scale, default_one(), "scale")?;
        require_default(value.reference_dpi, default_dpi(), "reference_dpi")?;
        require_default(value.fallback_dpi, default_dpi(), "fallback_dpi")?;
        require_default(
          value.reference_resolution,
          default_reference_resolution(),
          "reference_resolution",
        )?;
        require_default(
          value.screen_match_mode,
          PanelScreenMatchModeWire::default(),
          "screen_match_mode",
        )?;
        require_default(value.match_factor, 0.0, "match_factor")?;
        Ok(Self::constant_logical_pixel_size())
      }
      PanelScaleModeWire::Physical => {
        require_default(value.scale, default_one(), "scale")?;
        require_default(
          value.reference_resolution,
          default_reference_resolution(),
          "reference_resolution",
        )?;
        require_default(
          value.screen_match_mode,
          PanelScreenMatchModeWire::default(),
          "screen_match_mode",
        )?;
        require_default(value.match_factor, 0.0, "match_factor")?;
        Ok(Self::ConstantPhysicalSize(PanelPhysicalScale {
          reference_dpi: serialized_positive(value.reference_dpi, "panel reference DPI")?,
          fallback_dpi: serialized_positive(value.fallback_dpi, "panel fallback DPI")?,
        }))
      }
      PanelScaleModeWire::ScreenSize => {
        require_default(value.scale, default_one(), "scale")?;
        require_default(value.reference_dpi, default_dpi(), "reference_dpi")?;
        require_default(value.fallback_dpi, default_dpi(), "fallback_dpi")?;
        let reference_resolution =
          if value.reference_resolution.width > 0 && value.reference_resolution.height > 0 {
            value.reference_resolution
          } else {
            return Err("panel reference resolution must be positive".to_owned());
          };
        let screen_match_mode = match value.screen_match_mode {
          PanelScreenMatchModeWire::MatchWidthOrHeight => {
            PanelScreenMatchMode::MatchWidthOrHeight(PanelMatchFactor {
              value: serialized_match_factor(value.match_factor)?,
            })
          }
          PanelScreenMatchModeWire::Shrink => {
            require_default(value.match_factor, 0.0, "match_factor")?;
            PanelScreenMatchMode::Shrink
          }
          PanelScreenMatchModeWire::Expand => {
            require_default(value.match_factor, 0.0, "match_factor")?;
            PanelScreenMatchMode::Expand
          }
        };
        Ok(Self::ScaleWithScreenSize(PanelScreenScale {
          reference_resolution,
          screen_match_mode,
        }))
      }
    }
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

  fn wire(self) -> (PanelScreenMatchModeWire, f32) {
    match self {
      Self::MatchWidthOrHeight(PanelMatchFactor { value }) => {
        (PanelScreenMatchModeWire::MatchWidthOrHeight, value)
      }
      Self::Shrink => (PanelScreenMatchModeWire::Shrink, 0.0),
      Self::Expand => (PanelScreenMatchModeWire::Expand, 0.0),
    }
  }
}

impl Default for PanelScreenMatchMode {
  fn default() -> Self {
    Self::match_width_or_height(0.0)
  }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum PanelScaleModeWire {
  #[serde(rename = "ConstantPixelSize")]
  Pixel,
  #[serde(rename = "ConstantLogicalPixelSize")]
  LogicalPixel,
  #[default]
  #[serde(rename = "ConstantPhysicalSize")]
  Physical,
  #[serde(rename = "ScaleWithScreenSize")]
  ScreenSize,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum PanelScreenMatchModeWire {
  #[default]
  MatchWidthOrHeight,
  Shrink,
  Expand,
}

#[derive(Clone, Copy)]
pub(crate) struct PanelScaleWire {
  pub(crate) scale_mode: PanelScaleModeWire,
  pub(crate) scale: f32,
  pub(crate) reference_dpi: f32,
  pub(crate) fallback_dpi: f32,
  pub(crate) reference_resolution: ScreenSize,
  pub(crate) screen_match_mode: PanelScreenMatchModeWire,
  pub(crate) match_factor: f32,
}

pub(crate) fn default_reference_resolution() -> ScreenSize {
  ScreenSize::new(1200, 800)
}
pub(crate) fn is_default_reference_resolution(value: &ScreenSize) -> bool {
  *value == default_reference_resolution()
}
pub(crate) fn default_one() -> f32 {
  1.0
}
pub(crate) fn is_one(value: &f32) -> bool {
  *value == default_one()
}
pub(crate) fn default_dpi() -> f32 {
  96.0
}
pub(crate) fn is_dpi(value: &f32) -> bool {
  *value == default_dpi()
}
pub(crate) fn is_zero(value: &f32) -> bool {
  *value == 0.0
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
fn serialized_positive(value: f32, name: &str) -> Result<f32, String> {
  if value.is_finite() && value > 0.0 {
    Ok(value)
  } else {
    Err(format!("{name} must be finite and positive"))
  }
}
fn serialized_match_factor(value: f32) -> Result<f32, String> {
  if value.is_finite() && (0.0..=1.0).contains(&value) {
    Ok(value)
  } else {
    Err("panel match factor must be finite and in 0..=1".to_owned())
  }
}
fn require_default<T>(value: T, default: T, name: &str) -> Result<(), String>
where
  T: PartialEq,
{
  if value == default {
    Ok(())
  } else {
    Err(format!(
      "{name} must remain at its default for this panel scale mode"
    ))
  }
}
