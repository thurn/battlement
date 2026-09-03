//! Shared geometry and material recipes for the arcade bezel.

use battlement::{Color, Gradient, GradientStop, Length, Position, Style};

pub const OUTER_INSET: f32 = 21.0;
pub const BORDER_THICKNESS: f32 = 8.0;
pub const OUTER_BOTTOM: f32 = 111.0;

/// Pins a decorative layer to every edge of its parent.
pub fn cover() -> Style {
  Style::new()
    .position(Position::Absolute)
    .top(0)
    .right(0)
    .bottom(0)
    .left(0)
}

/// Builds the frame polygon, including the bottom Return cutout.
pub fn clip() -> Vec<[Length; 2]> {
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
  .map(|point| point.map(Length::percent))
  .to_vec()
}

/// Returns the bright metal gradient around the bezel.
pub fn metal() -> Gradient {
  Gradient::Linear {
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

/// Returns the dark interior gradient.
pub fn interior() -> Gradient {
  Gradient::Radial {
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

/// Converts an RGB hexadecimal color to opaque motion paint.
pub fn color(value: u32) -> Color {
  Color::rgba(
    f64::from((value >> 16) & 255) / 255.0,
    f64::from((value >> 8) & 255) / 255.0,
    f64::from(value & 255) / 255.0,
    1.0,
  )
}

fn stop(position: f32, color: u32) -> GradientStop {
  GradientStop {
    color: self::color(color),
    position,
  }
}
