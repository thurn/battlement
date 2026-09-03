//! Shared inset paint for clipped control interiors.

use battlement::{Length, PickingMode, Position, Shadow, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// A non-interactive clipped background inset from its containing control.
#[builder]
pub struct ClippedInset {
  #[builder(required)]
  background: PaintFill,
  /// Adds painted shadows inside the clipped surface.
  box_shadow: Option<Vec<Shadow>>,
  /// Sets the polygon used to clip the painted interior.
  clip_path: Vec<[Length; 2]>,
  /// Moves each edge inward by this many design pixels.
  inset: f32,
}

impl Component for ClippedInset {
  fn render(&self) -> impl Render {
    View::new()
      .name("clipped-inset")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(self.inset)
          .right(self.inset)
          .bottom(self.inset)
          .left(self.inset),
      )
      .paint(
        PaintStyle::new()
          .background(self.background.clone())
          .clip_polygon(self.clip_path.clone())
          .box_shadow(self.box_shadow.clone().unwrap_or_default()),
      )
  }
}
