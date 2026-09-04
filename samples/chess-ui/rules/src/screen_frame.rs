//! An arcade frame and content canvas at the portrait design size.

use crate::{
  concept_frame::ConceptFrame,
  frame_styles,
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
};
use battlement::{Color, Length, Overflow, PickingMode, Position, Style, TransformOrigin};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};
use std::rc::Rc;

/// Fixed portrait frame surrounding application content.
#[builder]
pub struct ScreenFrame<R> {
  #[builder(required, into)]
  children: Rc<R>,
}

impl<R: Render> Component for ScreenFrame<R> {
  fn render(&self) -> impl Render {
    View::new()
      .name("screen-frame")
      .style(
        Style::new()
          .position(Position::Relative)
          .width(PORTRAIT_DESIGN_WIDTH)
          .height(PORTRAIT_DESIGN_HEIGHT)
          .overflow(Overflow::Hidden)
          .color(Color::rgb8(247, 248, 255))
          .background_color(Color::BLACK),
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
              .paint(
                PaintStyle::new()
                  .clip_polygon(frame_styles::clip())
                  .background(PaintFill::Gradient(frame_styles::interior())),
              ),
            ConceptFrame::new(),
          )),
        self.children.clone(),
      ))
  }
}
