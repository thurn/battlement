use battlement::{MotionColor, MotionGradient, MotionGradientStop, MotionLength, Position, Style};

pub(crate) const OUTER_INSET: f32 = 21.0;
pub(crate) const BORDER_THICKNESS: f32 = 8.0;
pub(crate) const OUTER_BOTTOM: f32 = 111.0;

pub(crate) fn cover() -> Style {
  Style::new()
    .position(Position::Absolute)
    .top(0)
    .right(0)
    .bottom(0)
    .left(0)
}

pub(crate) fn clip() -> Vec<[MotionLength; 2]> {
  [
    [4.5, 0.0],
    [14.7, 0.0],
    [17.0, 1.9],
    [83.0, 1.9],
    [85.3, 0.0],
    [95.5, 0.0],
    [100.0, 3.2],
    [100.0, 18.7],
    [98.1, 20.0],
    [98.1, 98.6],
    [96.5, 100.0],
    [3.5, 100.0],
    [1.9, 98.6],
    [1.9, 20.0],
    [0.0, 18.7],
    [0.0, 3.2],
  ]
  .map(|point| point.map(MotionLength::percent))
  .to_vec()
}

pub(crate) fn metal() -> MotionGradient {
  MotionGradient::Linear {
    angle: 110.0,
    stops: [
      (0.0, 0xf4ffff),
      (0.04, 0x53dcff),
      (0.12, 0x0874ef),
      (0.18, 0x09234c),
      (0.32, 0x19ddff),
      (0.5, 0xe9fbff),
      (0.64, 0x806cff),
      (0.83, 0xff39c9),
      (0.96, 0xffd4f4),
      (1.0, 0xff5ec2),
    ]
    .map(|(position, color)| self::stop(position, color))
    .to_vec(),
  }
}

pub(crate) fn interior() -> MotionGradient {
  MotionGradient::Radial {
    center: [0.5, 0.43],
    radius: [0.959, 0.667],
    stops: [
      (0.0, 0x06152c),
      (0.42, 0x020817),
      (0.7, 0x01030b),
      (1.0, 0x000107),
    ]
    .map(|(position, color)| self::stop(position, color))
    .to_vec(),
  }
}

pub(crate) fn color(value: u32) -> MotionColor {
  MotionColor::new(
    ((value >> 16) & 255) as f32 / 255.0,
    ((value >> 8) & 255) as f32 / 255.0,
    (value & 255) as f32 / 255.0,
    1.0,
  )
}

fn stop(position: f32, color: u32) -> MotionGradientStop {
  MotionGradientStop {
    color: self::color(color),
    position,
  }
}
