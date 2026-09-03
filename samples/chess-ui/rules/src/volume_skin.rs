//! Decorative slider paint; input behavior belongs to VolumeControl.

use battlement::{
  Length, LengthUnits, MotionGradient, MotionGradientStop, MotionLength, PickingMode, Position,
  Style, Translate,
};
use battlement_reactant::{
  host::View,
  paint::{PaintFill, PaintStyle},
};

use crate::{clipped_inset::ClippedInset, frame_styles};

/// Paints the filled portion of a zero-to-one-hundred slider track.
pub fn track(value: u32) -> View {
  View::new()
    .name("volume-track")
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(0)
        .top(20)
        .width(284)
        .height(26)
        .padding(3)
        .border_radius(8),
    )
    .paint(self::gradient(
      0.0,
      &[
        (0.0, 0x13e7ff),
        (0.47, 0x735cff),
        (0.76, 0xff43c7),
        (1.0, 0xff326e),
      ],
    ))
    .child(
      View::new()
        .name("volume-track-interior")
        .style(Style::new().width(100.pct()).height(20).border_radius(5))
        .paint(PaintStyle::new().background(PaintFill::Color(frame_styles::color(0x061125))))
        .child(
          View::new()
            .name("volume-fill")
            .style(
              Style::new()
                .width((value as f32).pct())
                .height(20)
                .border_radius(4),
            )
            .paint(self::gradient(
              0.0,
              &[
                (0.0, 0x17e9ff),
                (0.35, 0x286fff),
                (0.62, 0x8f5dff),
                (0.86, 0xff3abe),
                (1.0, 0xff326d),
              ],
            )),
        ),
    )
}

/// Paints evenly spaced decorative scale marks.
pub fn ticks() -> View {
  View::new()
    .name("volume-ticks")
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(0)
        .top(49)
        .width(284)
        .height(10),
    )
    .child(
      (0..4)
        .map(|index| {
          View::new()
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(62 + index * 64)
                .width(2)
                .height(10),
            )
            .paint(PaintStyle::new().background(PaintFill::Color(frame_styles::color(0x465ccb))))
        })
        .collect::<Vec<_>>(),
    )
}

/// Positions the decorative thumb at an integer percentage.
pub fn thumb(value: u32) -> View {
  View::new()
    .name("volume-thumb")
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left((value as f32).pct())
        .top(0)
        .width(43)
        .height(64)
        .translate(Translate::two_dimensional(
          Length::Px(-21.0),
          Length::Px(0.0),
        )),
    )
    .paint(
      self::gradient(45.0, &[(0.0, 0xc8ffff), (0.55, 0x599cff), (1.0, 0x875fff)])
        .clip_polygon(self::clip()),
    )
    .child(
      ClippedInset::new(PaintFill::Gradient(MotionGradient::Linear {
        angle: 90.0,
        stops: vec![self::stop(0.0, 0x07142b), self::stop(1.0, 0x02091b)],
      }))
      .inset(4.0)
      .clip_path(self::clip()),
    )
}

fn gradient(angle: f32, colors: &[(f32, u32)]) -> PaintStyle {
  PaintStyle::new().background(PaintFill::Gradient(MotionGradient::Linear {
    angle,
    stops: colors
      .iter()
      .map(|&(position, color)| self::stop(position, color))
      .collect(),
  }))
}

fn stop(position: f32, color: u32) -> MotionGradientStop {
  MotionGradientStop {
    position,
    color: frame_styles::color(color),
  }
}

fn clip() -> Vec<[MotionLength; 2]> {
  [
    [23.0, 0.0],
    [77.0, 0.0],
    [100.0, 17.0],
    [100.0, 83.0],
    [77.0, 100.0],
    [23.0, 100.0],
    [0.0, 83.0],
    [0.0, 17.0],
  ]
  .map(|point| point.map(MotionLength::percent))
  .to_vec()
}
