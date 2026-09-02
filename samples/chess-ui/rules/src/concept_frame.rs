use battlement::{PickingMode, Position, Style};
use battlement_reactant::{component::Component, host::View, motion::MotionStyle, render::Render};

use crate::{clipped_inset::ClippedInset, frame_styles};

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
          .opacity(self.opacity),
      )
      .initial(false)
      .animate(
        MotionStyle::new()
          .clip_polygon(frame_styles::clip())
          .background_gradient(frame_styles::metal()),
      )
      .child(ClippedInset {
        inset: self.thickness,
        clip_path: frame_styles::clip(),
        background: frame_styles::solid(0x020713),
        box_shadow: None,
      })
  }
}
