//! Shared text-size state and source growth formulas.

use crate::settings::{self, TextSize};

pub type FontScale = TextSize;

/// Text roles with distinct source growth rates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontScaleRole {
  Control,
  Navigation,
  Heading,
}

/// Reads the complete application's current text size.
pub fn use_font_scale() -> FontScale {
  settings::use_settings().desired.text_size
}

impl FontScale {
  /// All supported sizes in selector order.
  pub const ALL: [Self; 3] = [Self::Percent100, Self::Percent150, Self::Percent200];

  /// Returns the numeric scaling factor.
  pub const fn factor(self) -> f32 {
    match self {
      Self::Percent100 => 1.0,
      Self::Percent150 => 1.5,
      Self::Percent200 => 2.0,
    }
  }

  /// Returns the visible percentage label.
  pub const fn label(self) -> &'static str {
    match self {
      Self::Percent100 => "100%",
      Self::Percent150 => "150%",
      Self::Percent200 => "200%",
    }
  }

  /// Applies the source growth curve for a text role.
  pub fn dynamic(self, role: FontScaleRole) -> f32 {
    let growth = self.factor() - 1.0;
    1.0
      + growth
        * match role {
          FontScaleRole::Control => 0.65,
          FontScaleRole::Navigation => 0.45,
          FontScaleRole::Heading => 0.2,
        }
  }
}
