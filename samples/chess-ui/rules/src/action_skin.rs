//! Paint recipes for the beveled outline and dark interior of arcade actions.

use battlement::{Color, Gradient, GradientStop, Length};
use battlement_reactant::paint::PaintFill;

use crate::frame_styles;

pub const INTERIOR: Color = Color::rgba(2.0 / 255.0, 6.0 / 255.0, 19.0 / 255.0, 1.0);

/// Builds the beveled action outline from horizontal and vertical corner cuts.
pub fn clip(x: f32, y: f32) -> Vec<[Length; 2]> {
  let left = Length::px(x);
  let right = Length::calc(-x, 100.0);
  let top = Length::px(y);
  let bottom = Length::calc(-y, 100.0);
  let zero = Length::px(0.0);
  let full = Length::percent(100.0);
  vec![
    [left, zero],
    [right, zero],
    [full, top],
    [full, bottom],
    [right, full],
    [left, full],
    [zero, bottom],
    [zero, top],
  ]
}

/// Returns the bright metallic border paint.
pub fn border() -> PaintFill {
  PaintFill::Gradient(Gradient::Linear {
    angle: 110.0,
    stops: [
      (0.0, 0xb9fbff),
      (0.22, 0x3bb9ff),
      (0.56, 0xa49cff),
      (0.9, 0xff4bd1),
    ]
    .map(|(position, color)| GradientStop {
      position,
      color: frame_styles::color(color),
    })
    .to_vec(),
  })
}
