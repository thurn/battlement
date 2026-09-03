//! Shared geometry and material recipes for the arcade bezel.

use battlement::{Color, Gradient, Length, Position, Style};

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
  Gradient::linear(110.0)
    .stop(0.0, self::color(0xf4ffff))
    .stop(0.04, self::color(0x53dcff))
    .stop(0.12, self::color(0x0874ef))
    .stop(0.18, self::color(0x09234c))
    .stop(0.32, self::color(0x19ddff))
    .stop(0.5, self::color(0xe9fbff))
    .stop(0.64, self::color(0x806cff))
    .stop(0.83, self::color(0xff39c9))
    .stop(0.96, self::color(0xffd4f4))
    .stop(1.0, self::color(0xff5ec2))
}

/// Returns the dark interior gradient.
pub fn interior() -> Gradient {
  Gradient::radial([0.5, 0.43], [0.959, 0.667])
    .stop(0.0, self::color(0x06152c))
    .stop(0.42, self::color(0x020817))
    .stop(0.7, self::color(0x01030b))
    .stop(1.0, self::color(0x000107))
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
