use battlement::{Color, Length, Overflow, PickingMode, Position, Style, TransformOrigin};
use battlement_reactant::{
  component::Component,
  host::View,
  motion::MotionStyle,
  render::{Node, Render},
};

use crate::{
  concept_frame::ConceptFrame,
  frame_styles,
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
};

/// Fixed portrait frame surrounding application content.
pub struct ScreenFrame {
  pub children: Node,
}

impl Component for ScreenFrame {
  fn render(&self) -> impl Render {
    View::new()
      .name("screen-frame")
      .style(
        Style::new()
          .position(Position::Relative)
          .width(PORTRAIT_DESIGN_WIDTH)
          .height(PORTRAIT_DESIGN_HEIGHT)
          .overflow(Overflow::Hidden)
          .color(Color::rgb(247.0 / 255.0, 248.0 / 255.0, 1.0))
          .background_color(Color::rgb(0.0, 0.0, 0.0)),
      )
      .child((
        View::new()
          .name("exit-frame-surface")
          .picking_mode(PickingMode::Ignore)
          .style(
            frame_styles::cover().transform_origin(TransformOrigin::two_dimensional(
              Length::Percent(50.0),
              Length::Percent(47.07),
            )),
          )
          .child((
            View::new()
              .name("frame-interior")
              .picking_mode(PickingMode::Ignore)
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
                  .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
                  .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
                  .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS),
              )
              .initial(false)
              .animate(
                MotionStyle::new()
                  .clip_polygon(frame_styles::clip())
                  .background_gradient(frame_styles::interior()),
              ),
            ConceptFrame,
          )),
        self.children.clone(),
      ))
  }
}
