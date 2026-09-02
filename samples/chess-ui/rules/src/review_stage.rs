use battlement::{
  Align, FlexDirection, Justify, Length, LengthUnits, Overflow, Position, Scale, Style,
  TransformOrigin,
};
use battlement_reactant::{
  component::Component,
  host::{Flex, View},
  render::{Node, Render},
};

use crate::{
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
  review_theme,
};

/// Centers a scaled portrait canvas in the remaining review surface.
pub struct ReviewStage {
  pub scale: f32,
  pub children: Node,
}

impl Component for ReviewStage {
  fn render(&self) -> impl Render {
    Flex::new()
      .direction(FlexDirection::Column)
      .style(
        Style::new()
          .flex_grow(1)
          .min_width(0)
          .height(100.pct())
          .align_items(Align::Center)
          .justify_content(Justify::Center),
      )
      .child(
        View::new()
          .name("design-stage-bounds")
          .style(
            Style::new()
              .width(PORTRAIT_DESIGN_WIDTH * self.scale)
              .height(PORTRAIT_DESIGN_HEIGHT * self.scale)
              .flex_shrink(0),
          )
          .child(
            View::new()
              .name("design-stage")
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(0)
                  .top(0)
                  .width(PORTRAIT_DESIGN_WIDTH)
                  .height(PORTRAIT_DESIGN_HEIGHT)
                  .scale(Scale::uniform(self.scale))
                  .transform_origin(TransformOrigin::two_dimensional(
                    Length::Px(0.0),
                    Length::Px(0.0),
                  ))
                  .overflow(Overflow::Hidden)
                  .background_color(review_theme::SURFACE),
              )
              .child(self.children.clone()),
          ),
      )
  }
}
