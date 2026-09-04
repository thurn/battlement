//! Shared geometry and material recipes for the arcade bezel.

use battlement::{Color, Gradient, Length};

pub const OUTER_INSET: f32 = 21.0;
pub const BORDER_THICKNESS: f32 = 8.0;
pub const OUTER_BOTTOM: f32 = 111.0;

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
    .stop(0.0, Color::hex(0xf4ffff))
    .stop(0.04, Color::hex(0x53dcff))
    .stop(0.12, Color::hex(0x0874ef))
    .stop(0.18, Color::hex(0x09234c))
    .stop(0.32, Color::hex(0x19ddff))
    .stop(0.5, Color::hex(0xe9fbff))
    .stop(0.64, Color::hex(0x806cff))
    .stop(0.83, Color::hex(0xff39c9))
    .stop(0.96, Color::hex(0xffd4f4))
    .stop(1.0, Color::hex(0xff5ec2))
}

/// Returns the dark interior gradient.
pub fn interior() -> Gradient {
  Gradient::radial([0.5, 0.43], [0.959, 0.667])
    .stop(0.0, Color::hex(0x06152c))
    .stop(0.42, Color::hex(0x020817))
    .stop(0.7, Color::hex(0x01030b))
    .stop(1.0, Color::hex(0x000107))
}
