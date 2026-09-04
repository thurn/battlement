//! The arcade design canvas and its parent-relative fitting policy.

use trox::tx;

use battlement::{Align, Color, Justify, LengthUnits, Overflow};
use battlement_reactant::prelude::{Child, builder};
use battlement_reactant::semantics::{SemanticName, SemanticProps};
use battlement_reactant::{component::Component, render::Render, scale_to_fit::ScaleToFit};

/// Logical width of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_WIDTH: f32 = 1024.0;

/// Logical height of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_HEIGHT: f32 = 1536.0;

/// Fits the arcade canvas into its parent while preserving design coordinates.
/// The shared fit component owns measurement, scaling, and centering.
#[builder]
pub struct PortraitViewport {
  /// Sets content authored at the portrait design size.
  #[builder(required, into)]
  child: Child,
}

impl Component for PortraitViewport {
  fn render(&self) -> impl Render {
    ScaleToFit::new(PORTRAIT_DESIGN_WIDTH, PORTRAIT_DESIGN_HEIGHT)
      .bounds_name("portrait-bounds")
      .viewport(|view| {
        view.name("portrait-viewport").semantic(
          SemanticProps::new(battlement::SemanticRole::Region).name(SemanticName::Text(tx(
            "Main content",
            "Portrait viewport accessibility label.",
          ))),
        )
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
      .child(self.child.render())
  }
}
