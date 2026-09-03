//! Window-level layout and theme for the review application.

use crate::review_theme;
use battlement::{FlexDirection, LengthUnits, Overflow, Style, UiDocument};
use battlement_reactant::prelude::builder;
use battlement_reactant::{component::Component, host::View, render::Render};

/// A full-window review surface with navigation beside its content.
#[builder]
pub struct ReviewSurface {
  #[builder(default = View::new().name("gallery"))]
  view: View,
}

impl ReviewSurface {
  /// Adds navigation and content to the full-window surface.
  pub fn child(mut self, content: impl Render) -> Self {
    self.view = self.view.child(content);
    self
  }
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

impl Default for ReviewSurface {
  fn default() -> Self {
    Self::new()
  }
}
