//! Paint recipes for the beveled outline and dark interior of arcade actions.

use battlement::{Color, Gradient, Length};
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
  PaintFill::Gradient(
    Gradient::linear(110.0)
      .stop(0.0, frame_styles::color(0xb9fbff))
      .stop(0.22, frame_styles::color(0x3bb9ff))
      .stop(0.56, frame_styles::color(0xa49cff))
      .stop(0.9, frame_styles::color(0xff4bd1)),
  )
}
