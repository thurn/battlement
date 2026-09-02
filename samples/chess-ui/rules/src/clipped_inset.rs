use battlement::{MotionLength, MotionShadow, PickingMode, Position, Style};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// A non-interactive clipped background inset from its containing control.
pub struct ClippedInset {
  pub background: PaintFill,
  pub box_shadow: Option<Vec<MotionShadow>>,
  pub clip_path: Vec<[MotionLength; 2]>,
  pub inset: f32,
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
