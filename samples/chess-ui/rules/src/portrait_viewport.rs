//! The arcade design canvas and its parent-relative fitting policy.

use battlement::{Align, Color, Justify, LengthUnits, Overflow};
use battlement_reactant::prelude::builder;
use battlement_reactant::{accessibility_collections, semantics};
use battlement_reactant::{component::Component, render::Render, scale_to_fit::ScaleToFit};
use std::rc::Rc;

/// Logical width of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_WIDTH: f32 = 1024.0;

/// Logical height of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_HEIGHT: f32 = 1536.0;

/// Fits the arcade canvas into its parent while preserving design coordinates.
/// The shared fit component owns measurement, scaling, and centering.
#[builder]
pub struct PortraitViewport<R> {
  /// Sets content authored at the portrait design size.
  #[builder(required, into)]
  child: Rc<R>,
}

impl<R: Render> Component for PortraitViewport<R> {
  fn render(&self) -> impl Render {
    ScaleToFit::new(PORTRAIT_DESIGN_WIDTH, PORTRAIT_DESIGN_HEIGHT)
      .bounds_name("portrait-bounds")
      .viewport(|view| {
        view
          .name("portrait-viewport")
          .semantic(accessibility_collections::use_region(semantics::text(
            "Main content",
          )))
      })
      .viewport_style(|style| style.width(100.pct()).background_color(Color::BLACK))
      .roomy_scale(1024.0, 0.75)
      .canvas(|view| view.name("portrait-canvas"))
      .canvas_style(|style| {
        style
          .overflow(Overflow::Hidden)
          .align_items(Align::Center)
          .justify_content(Justify::Center)
          .background_color(Color::BLACK)
      })
      .child(self.child.clone())
  }
}
