use battlement::{MotionColor, MotionGradient, MotionGradientStop, MotionLength};
use battlement_reactant::paint::PaintFill;

use crate::frame_styles;

pub(crate) const INTERIOR: MotionColor =
  MotionColor::new(2.0 / 255.0, 6.0 / 255.0, 19.0 / 255.0, 1.0);

pub(crate) fn clip(x: f32, y: f32) -> Vec<[MotionLength; 2]> {
  let left = MotionLength::px(x);
  let right = MotionLength::calc(-x, 100.0);
  let top = MotionLength::px(y);
  let bottom = MotionLength::calc(-y, 100.0);
  let zero = MotionLength::px(0.0);
  let full = MotionLength::percent(100.0);
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

pub(crate) fn border() -> PaintFill {
  PaintFill::Gradient(MotionGradient::Linear {
    angle: 110.0,
    stops: [
      (0.0, 0xb9fbff),
      (0.22, 0x3bb9ff),
      (0.56, 0xa49cff),
      (0.9, 0xff4bd1),
    ]
    .map(|(position, color)| MotionGradientStop {
      position,
      color: frame_styles::color(color),
    })
    .to_vec(),
  })
}
