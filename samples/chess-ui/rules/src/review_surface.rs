use battlement::{FlexDirection, LengthUnits, Overflow, Style, UiDocument};
use battlement_reactant::{component::Component, host::View, render::Render};

use crate::review_theme;

/// A full-window review surface with navigation beside its content.
pub struct ReviewSurface {
  pub view: View,
}

impl ReviewSurface {
  /// Applies the surface's sizing and theme to its native document root.
  pub fn document(document: UiDocument) -> UiDocument {
    document.style(Self::style())
  }

  fn style() -> Style {
    Style::new()
      .width(100.pct())
      .height(100.pct())
      .overflow(Overflow::Hidden)
      .background_color(review_theme::BACKGROUND)
      .color(review_theme::TEXT)
  }
}

impl Component for ReviewSurface {
  fn render(&self) -> impl Render {
    self
      .view
      .clone()
      .style(Self::style().flex_direction(FlexDirection::Row).padding(24))
  }
}
