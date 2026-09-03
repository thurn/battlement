//! The gallery’s fixed design canvas, fitted into the available content column.

use battlement::Overflow;
use battlement_reactant::{component::Component, render::Render, scale_to_fit::ScaleToFit};

use crate::{
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
  review_theme,
};

/// Centers a portrait review page in the space beside navigation.
/// The shared fit component owns measurement, scaling, and centering.
pub struct ReviewStage<R = ()> {
  fit: ScaleToFit<R>,
}

impl ReviewStage {
  /// Creates a portrait stage fitted into its parent layout.
  pub fn new() -> Self {
    Self {
      fit: ScaleToFit::new(PORTRAIT_DESIGN_WIDTH, PORTRAIT_DESIGN_HEIGHT)
        .bounds_name("design-stage-bounds")
        .canvas(|view| view.name("design-stage"))
        .canvas_style(|style| {
          style
            .overflow(Overflow::Hidden)
            .background_color(review_theme::SURFACE)
        }),
    }
  }
}

impl Default for ReviewStage {
  fn default() -> Self {
    Self::new()
  }
}

impl<R: Render> ReviewStage<R> {
  /// Sets content authored at the portrait design size.
  pub fn child<C: Render>(self, child: C) -> ReviewStage<C> {
    ReviewStage {
      fit: self.fit.child(child),
    }
  }
}

impl<R: Render> Component for ReviewStage<R> {
  fn render(&self) -> impl Render {
    self.fit.render()
  }
}
