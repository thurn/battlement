//! Shared application text-size selection.

use crate::settings::{self, TextSize};

pub type FontScale = TextSize;

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
}
