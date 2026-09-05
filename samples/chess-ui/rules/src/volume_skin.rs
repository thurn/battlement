//! Decorative slider paint; input behavior belongs to VolumeControl.

use battlement::{
  Color, Gradient, GradientStop, Length, LengthUnits, MotionProperty, PickingMode, Position,
  Shadow, Style, Translate,
};
use battlement_reactant::{
  component::Component,
  host::View,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  paint::{PaintFill, PaintLayer, PaintStyle},
  prelude::{PaintDropShadow, PaintFilterList, builder},
  render::Render,
};

use crate::use_interaction::InteractionState;

/// The filled portion of a zero-to-one-hundred slider track.
#[builder]
pub struct VolumeTrack {
  #[builder(required)]
  value: u32,
  #[builder(required)]
  interaction: InteractionState,
}

/// Evenly spaced decorative scale marks.
#[builder]
#[derive(Default)]
pub struct VolumeTicks;

/// The decorative slider thumb positioned at an integer percentage.
#[builder]
pub struct VolumeThumb {
  #[builder(required)]
  value: u32,
  #[builder(required)]
  interaction: InteractionState,
}

impl Component for VolumeTrack {
  fn render(&self) -> impl Render {
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
      .initial(false)
      .animate(self::track_target(self.interaction))
      .paint(
        self::gradient(0.0, self::track_colors(self.interaction.hovered))
          .box_shadow(self::track_shadows(self.interaction.hovered)),
      )
      .child(
        View::new()
          .name("volume-track-interior")
          .style(Style::new().width(100.pct()).height(20).border_radius(5))
          .paint(
            PaintStyle::new()
              .background(PaintFill::Color(Color::hex(if self.interaction.hovered {
                0x071830
              } else {
                0x061125
              })))
              .box_shadow([self::shadow(0.0, 0.0, 8.0, 0.0, 0x000000, 0.69, true)]),
          )
          .child(
            View::new()
              .name("volume-fill")
              .style(
                Style::new()
                  .width((self.value as f32).pct())
                  .height(20)
                  .border_radius(4),
              )
              .paint(
                self::gradient(
                  0.0,
                  &[
                    (0.0, 0x17e9ff),
                    (0.35, 0x286fff),
                    (0.62, 0x8f5dff),
                    (0.86, 0xff3abe),
                    (1.0, 0xff326d),
                  ],
                )
                .box_shadow([self::shadow(0.0, 0.0, 8.0, 0.0, 0x2d84ff, 0.8, false)]),
              ),
          ),
      )
  }
}

impl Component for VolumeTicks {
  fn render(&self) -> impl Render {
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
              .paint(PaintStyle::new().background(PaintFill::Color(Color::hex(0x465ccb))))
          })
          .collect::<Vec<_>>(),
      )
  }
}

impl Component for VolumeThumb {
  fn render(&self) -> impl Render {
    View::decorative()
      .name("volume-thumb")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left((self.value as f32).pct())
          .top(0)
          .width(43)
          .height(64)
          .translate(Translate::two_dimensional(
            Length::Px(-21.0),
            Length::Px(0.0),
          )),
      )
      .initial(false)
      .animate(self::thumb_target(self.interaction))
      .paint(
        self::gradient(45.0, self::thumb_colors(self.interaction.hovered))
          .clip_polygon(self::clip())
          .paint_filter(PaintFilterList::default().drop_shadow(PaintDropShadow::new(
            0.0,
            0.0,
            7.0,
            0.0,
            Color::hex(0x1479ff),
          )))
          .layer(
            PaintLayer::new(
              Gradient::linear(90.0)
                .stop(0.0, Color::hex(0x07142b))
                .stop(1.0, Color::hex(0x02091b)),
            )
            .bounds_inset(4.0)
            .clip_polygon(self::clip())
            .box_shadow([self::shadow(0.0, 0.0, 12.0, 0.0, 0x000000, 0.69, true)]),
          ),
      )
  }
}

fn track_target(state: InteractionState) -> MotionTarget {
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(self::gradient_value(0.0, self::track_colors(state.hovered)))
      .box_shadow(self::track_shadows(state.hovered))
      .paint_filter(PaintFilterList::default().brightness(if state.pressed { 0.74 } else { 1.0 }))
      .scale_y(if state.pressed && !state.reduced_motion {
        0.82
      } else {
        1.0
      }),
  )
  .transition(self::feedback_transition())
}

fn thumb_target(state: InteractionState) -> MotionTarget {
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(self::gradient_value(
        45.0,
        self::thumb_colors(state.hovered),
      ))
      .paint_filter(
        PaintFilterList::default()
          .brightness(if state.hovered { 1.16 } else { 1.0 })
          .drop_shadow(PaintDropShadow::new(
            0.0,
            0.0,
            if state.hovered { 10.0 } else { 7.0 },
            0.0,
            Color::hex(if state.hovered { 0x2bc8ff } else { 0x1479ff }),
          )),
      )
      .scale(if state.pressed && !state.reduced_motion {
        0.88
      } else {
        1.0
      }),
  )
  .transition(self::feedback_transition())
}

fn feedback_transition() -> Transition {
  Transition::tween()
    .duration_secs(0.14)
    .ease(Easing::Ease)
    .property(
      MotionProperty::Scale,
      Transition::tween()
        .duration_secs(0.09)
        .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
    )
    .property(
      MotionProperty::ScaleY,
      Transition::tween()
        .duration_secs(0.09)
        .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
    )
    .property(
      MotionProperty::PaintFilter,
      Transition::tween().duration_secs(0.09).ease(Easing::Ease),
    )
}

fn track_colors(hovered: bool) -> &'static [(f32, u32)] {
  if hovered {
    &[
      (0.0, 0x9dffff),
      (0.47, 0x7c8dff),
      (0.76, 0xff74d7),
      (1.0, 0xff668e),
    ]
  } else {
    &[
      (0.0, 0x13e7ff),
      (0.47, 0x735cff),
      (0.76, 0xff43c7),
      (1.0, 0xff326e),
    ]
  }
}

fn thumb_colors(hovered: bool) -> &'static [(f32, u32)] {
  if hovered {
    &[(0.0, 0xffffff), (0.55, 0x7edfff), (1.0, 0xb58cff)]
  } else {
    &[(0.0, 0xc8ffff), (0.55, 0x599cff), (1.0, 0x875fff)]
  }
}

fn track_shadows(hovered: bool) -> Vec<Shadow> {
  if hovered {
    vec![
      self::shadow(0.0, 0.0, 15.0, 0.0, 0x31bdff, 0.88, false),
      self::shadow(0.0, 0.0, 7.0, 0.0, 0x000000, 1.0, true),
    ]
  } else {
    vec![
      self::shadow(0.0, 0.0, 9.0, 0.0, 0x1868ff, 0.72, false),
      self::shadow(0.0, 0.0, 8.0, 0.0, 0x000000, 1.0, true),
    ]
  }
}

fn gradient_value(angle: f32, colors: &[(f32, u32)]) -> Gradient {
  Gradient::linear(angle).stops(
    colors
      .iter()
      .map(|&(position, color)| GradientStop::new(position, Color::hex(color))),
  )
}

fn gradient(angle: f32, colors: &[(f32, u32)]) -> PaintStyle {
  PaintStyle::new().background(PaintFill::Gradient(self::gradient_value(angle, colors)))
}

fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32, alpha: f64, inset: bool) -> Shadow {
  Shadow {
    x,
    y,
    blur,
    spread,
    color: Color::hex(color).with_alpha(alpha),
    inset,
  }
}

fn clip() -> Vec<[Length; 2]> {
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
  .map(|point| point.map(Length::percent))
  .to_vec()
}
