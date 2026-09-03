//! A decorative chevron whose orientation follows a selector’s open state.

use battlement::{Length, MotionLength, PickingMode, Position, Rotate, Style, Translate};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

use crate::frame_styles;

/// The decorative direction indicator on a select trigger.
pub struct Caret {
  is_open: bool,
}

impl Caret {
  /// Creates a closed selector’s decorative caret.
  pub fn new() -> Self {
    Self { is_open: false }
  }
  /// Points upward while the selector is open.
  pub fn open(mut self, value: bool) -> Self {
    self.is_open = value;
    self
  }
}

impl Component for Caret {
  fn render(&self) -> impl Render {
    View::new()
      .name("select-caret")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(Length::Percent(50.0))
          .right(45)
          .width(30)
          .height(18)
          .translate(Translate::two_dimensional(
            Length::Px(0.0),
            Length::Percent(-50.0),
          ))
          .rotate(Rotate::degrees(if self.is_open { 180.0 } else { 0.0 })),
      )
      .paint(
        PaintStyle::new()
          .background(PaintFill::Color(frame_styles::color(0xf4f5fa)))
          .clip_polygon([
            [MotionLength::percent(0.0), MotionLength::percent(0.0)],
            [MotionLength::percent(100.0), MotionLength::percent(0.0)],
            [MotionLength::percent(50.0), MotionLength::percent(100.0)],
          ]),
      )
  }
}

impl Default for Caret {
  fn default() -> Self {
    Self::new()
  }
}
