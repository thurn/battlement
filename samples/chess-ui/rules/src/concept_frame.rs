use battlement::{Length, PickingMode, Position, Style};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

use crate::frame_styles;

/// Decorative arcade bezel with its clipped inner surface.
pub struct ConceptFrame;

struct FrameLayer {
  inset: f32,
  thickness: f32,
  opacity: f32,
  bottom: Option<f32>,
}

impl Component for ConceptFrame {
  fn render(&self) -> impl Render {
    View::new()
      .name("concept-frame")
      .picking_mode(PickingMode::Ignore)
      .style(frame_styles::cover())
      .child(FrameLayer {
        inset: frame_styles::OUTER_INSET,
        thickness: frame_styles::BORDER_THICKNESS,
        opacity: 1.0,
        bottom: None,
      })
  }
}

impl Component for FrameLayer {
  fn render(&self) -> impl Render {
    View::new()
      .name("frame-layer")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(self.inset)
          .left(self.inset)
          .right(self.inset)
          .bottom(
            self
              .bottom
              .unwrap_or(frame_styles::OUTER_BOTTOM + self.inset - frame_styles::OUTER_INSET),
          )
          .opacity(self.opacity)
          .padding(self.thickness),
      )
      .paint(
        PaintStyle::new()
          .clip_polygon(frame_styles::clip())
          .background(PaintFill::Gradient(frame_styles::metal())),
      )
      .child(
        View::new()
          .name("frame-layer-inner")
          .picking_mode(PickingMode::Ignore)
          .style(
            Style::new()
              .width(Length::Percent(100.0))
              .height(Length::Percent(100.0)),
          )
          .paint(
            PaintStyle::new()
              .clip_polygon(frame_styles::clip())
              .background(PaintFill::Color(frame_styles::color(0x020713))),
          ),
      )
  }
}
