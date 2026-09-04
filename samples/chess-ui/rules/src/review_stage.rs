//! The gallery’s fixed design canvas, fitted into the available content column.

use crate::{
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
  review_theme,
};
use battlement::Overflow;
use battlement_reactant::prelude::{Child, builder};
use battlement_reactant::{component::Component, render::Render, scale_to_fit::ScaleToFit};

/// Centers a portrait review page in the space beside navigation.
/// The shared fit component owns measurement, scaling, and centering.
#[builder]
pub struct ReviewStage {
  /// Sets content authored at the portrait design size.
  #[builder(required, into)]
  child: Child,
}

impl Component for ReviewStage {
  fn render(&self) -> impl Render {
    ScaleToFit::new(PORTRAIT_DESIGN_WIDTH, PORTRAIT_DESIGN_HEIGHT)
      .bounds_name("design-stage-bounds")
      .canvas(|view| view.name("design-stage"))
      .canvas_style(|style| {
        style
          .overflow(Overflow::Hidden)
          .background_color(review_theme::SURFACE)
      })
      .child(self.child.render())
  }
}
