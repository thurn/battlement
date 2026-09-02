use battlement::{
  Align, Color, Justify, Length, LengthUnits, Overflow, Position, Scale, Style, TransformOrigin,
};
use battlement_reactant::{
  accessibility_collections as collections,
  component::Component,
  element_ref, geometry,
  host::View,
  render::{Node, Render},
  semantics,
};

/// Logical width of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_WIDTH: f32 = 1024.0;
/// Logical height of the portrait canvas before viewport scaling.
pub const PORTRAIT_DESIGN_HEIGHT: f32 = 1536.0;

/// Centers a fixed portrait canvas within its available viewport.
pub struct PortraitViewport {
  pub children: Node,
}

impl Component for PortraitViewport {
  fn render(&self) -> impl Render {
    let viewport = element_ref::use_element_ref();
    let scale = geometry::use_geometry(viewport.clone())
      .measurements
      .latest
      .map_or(1.0, |geometry| {
        let width = geometry.layout.width as f32;
        let height = geometry.layout.height as f32;
        let fit = (width / PORTRAIT_DESIGN_WIDTH)
          .min(height / PORTRAIT_DESIGN_HEIGHT)
          .clamp(0.0, 1.0);
        fit * if width >= 1024.0 { 0.75 } else { 1.0 }
      });
    View::new()
      .name("portrait-viewport")
      .element_ref(viewport)
      .semantic(collections::use_region(semantics::text("Main content")))
      .style(
        Style::new()
          .width(100.pct())
          .height(100.pct())
          .overflow(Overflow::Hidden)
          .align_items(Align::Center)
          .justify_content(Justify::Center)
          .background_color(Color::rgb(0.0, 0.0, 0.0)),
      )
      .child(
        View::new()
          .name("portrait-bounds")
          .style(
            Style::new()
              .position(Position::Relative)
              .width(PORTRAIT_DESIGN_WIDTH * scale)
              .height(PORTRAIT_DESIGN_HEIGHT * scale)
              .flex_shrink(0),
          )
          .child(
            View::new()
              .name("portrait-canvas")
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(0)
                  .top(0)
                  .width(PORTRAIT_DESIGN_WIDTH)
                  .height(PORTRAIT_DESIGN_HEIGHT)
                  .overflow(Overflow::Hidden)
                  .align_items(Align::Center)
                  .justify_content(Justify::Center)
                  .scale(Scale::uniform(scale))
                  .transform_origin(TransformOrigin::two_dimensional(
                    Length::Px(0.0),
                    Length::Px(0.0),
                  ))
                  .background_color(Color::rgb(0.0, 0.0, 0.0)),
              )
              .child(self.children.clone()),
          ),
      )
  }
}
