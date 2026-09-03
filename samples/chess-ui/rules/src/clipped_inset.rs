//! Shared inset paint for clipped control interiors.

use battlement::{MotionLength, MotionShadow, PickingMode, Position, Style};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// A non-interactive clipped background inset from its containing control.
pub struct ClippedInset {
  background: PaintFill,
  box_shadow: Option<Vec<MotionShadow>>,
  clip_path: Vec<[MotionLength; 2]>,
  inset: f32,
}

impl ClippedInset {
  /// Creates a clipped interior using the supplied paint.
  pub fn new(background: PaintFill) -> Self {
    Self {
      background,
      box_shadow: None,
      clip_path: Vec::new(),
      inset: 0.0,
    }
  }
  /// Moves each edge inward by this many design pixels.
  pub fn inset(mut self, inset: f32) -> Self {
    self.inset = inset;
    self
  }
  /// Sets the polygon used to clip the painted interior.
  pub fn clip_path(mut self, points: impl IntoIterator<Item = [MotionLength; 2]>) -> Self {
    self.clip_path = points.into_iter().collect();
    self
  }
  /// Adds painted shadows inside the clipped surface.
  pub fn box_shadow(mut self, shadows: Vec<MotionShadow>) -> Self {
    self.box_shadow = Some(shadows);
    self
  }
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
