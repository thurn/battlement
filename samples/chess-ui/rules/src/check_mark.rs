use battlement::{
  Length, LengthUnits, MotionColor, MotionLength, PickingMode, Position, Scale, Style, Translate,
};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// The clipped check glyph used by settings checkboxes and selected options.
pub(crate) struct CheckMark {
  pub(crate) scale: f32,
}

impl Default for CheckMark {
  fn default() -> Self {
    Self { scale: 1.0 }
  }
}

impl Component for CheckMark {
  fn render(&self) -> impl Render {
    View::new()
      .name("check-mark")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(50.pct())
          .top(50.pct())
          .width(50)
          .height(44)
          .translate(Translate::two_dimensional(
            Length::Percent(-50.0),
            Length::Percent(-50.0),
          ))
          .scale(Scale::new(self.scale, self.scale)),
      )
      .paint(
        PaintStyle::new()
          .background(PaintFill::Color(MotionColor::new(
            97.0 / 255.0,
            241.0 / 255.0,
            1.0,
            1.0,
          )))
          .clip_polygon(
            [
              [0.0, 47.0],
              [14.0, 32.0],
              [35.0, 58.0],
              [85.0, 0.0],
              [100.0, 14.0],
              [35.0, 100.0],
            ]
            .map(|point| point.map(MotionLength::percent)),
          ),
      )
  }
}
