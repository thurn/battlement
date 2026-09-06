//! Seeded ambient grid and particles behind the arcade menu.

use crate::frame_styles;
use battlement::{
  Color, Gradient, Length, LengthUnits, Overflow, Position, Rotate, Scale, Shadow, Style,
  TransformOrigin,
};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintLayer, PaintStyle},
  prelude::{
    Easing, Keyframes, MotionTarget, Repeat, RepeatType, StyleTarget, Transition, builder,
  },
  render::Render,
};

const PARTICLE_COUNT: usize = 48;
const PARTICLE_COLORS: [Color; 5] = [
  Color::hex(0xbff8ff),
  Color::hex(0x59cfff),
  Color::WHITE,
  Color::hex(0xcf9cff),
  Color::hex(0xff69d7),
];
const GRID_RAYS: [f32; 9] = [-22.0, -16.5, -11.0, -5.5, 0.0, 5.5, 11.0, 16.5, 22.0];
const GRID_HORIZONS: [(f32, f32); 7] = [
  (23.0, 10.0),
  (28.0, 18.0),
  (34.0, 29.0),
  (42.0, 42.0),
  (52.0, 57.0),
  (65.0, 75.0),
  (82.0, 96.0),
];

#[derive(Clone, Copy)]
struct Particle {
  accent: bool,
  color: Color,
  drift_x: f32,
  drift_y: f32,
  duration: f64,
  phase: f64,
  size: f32,
  x: f32,
  y: f32,
}

/// Source-shaped attract background with deterministic particle generation.
#[builder]
pub struct ArcadeAttractMode {
  #[builder(required)]
  reduce_motion: bool,
}

impl Component for ArcadeAttractMode {
  fn render(&self) -> impl Render {
    View::decorative()
      .name("arcade-attract-mode")
      .style(Style::new().absolute_fill())
      .child(
        View::decorative()
          .name("arcade-attract-interior")
          .style(
            Style::new()
              .position(Position::Absolute)
              .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
              .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
              .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS)
              .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
              .overflow(Overflow::Hidden),
          )
          .paint(self::background_paint())
          .child(self::perspective_grid(self.reduce_motion))
          .children(
            self::particles()
              .into_iter()
              .enumerate()
              .map(|(index, particle)| self::particle(index, particle, self.reduce_motion)),
          )
          .child(
            View::decorative()
              .name("arcade-attract-vignette")
              .style(Style::new().absolute_fill())
              .paint(self::vignette_paint()),
          ),
      )
  }
}

fn perspective_grid(reduce_motion: bool) -> View {
  let grid = View::decorative()
    .name("arcade-attract-grid")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(20.pct())
        .right(4.pct())
        .bottom(5.pct())
        .left(4.pct())
        .overflow(Overflow::Hidden)
        .opacity(0.26)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::Percent(50.0),
          Length::Percent(0.0),
        )),
    )
    .children(
      GRID_RAYS
        .into_iter()
        .enumerate()
        .map(|(index, angle)| self::grid_ray(index, angle)),
    )
    .children(
      GRID_HORIZONS
        .into_iter()
        .enumerate()
        .map(|(index, (top, width))| self::grid_horizon(index, top, width)),
    );
  if reduce_motion {
    grid
  } else {
    grid
      .initial(StyleTarget::new().y(-6.0).scale_y(0.96).opacity(0.58))
      .animate(
        MotionTarget::new(StyleTarget::new().y(12.0).scale_y(1.02).opacity(1.0)).transition(
          Transition::tween()
            .duration_secs(5.2)
            .ease(Easing::EaseInOut)
            .repeat(Repeat::Forever)
            .repeat_type(RepeatType::Reverse),
        ),
      )
  }
}

fn grid_ray(index: usize, angle: f32) -> View {
  View::decorative()
    .name(format!("arcade-attract-grid-ray-{index}"))
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(21.pct())
        .left(50.pct())
        .width(2)
        .height(76.pct())
        .rotate(Rotate::degrees(angle))
        .transform_origin(TransformOrigin::two_dimensional(
          Length::Percent(50.0),
          Length::Percent(0.0),
        )),
    )
    .paint(
      PaintStyle::new()
        .background(
          Gradient::linear(180.0)
            .stop(0.0, Color::rgba8(94, 212, 255, 20))
            .stop(1.0, Color::rgba8(94, 212, 255, 199)),
        )
        .box_shadow([Shadow::outer(
          0.0,
          0.0,
          7.0,
          0.0,
          Color::rgba8(64, 186, 255, 87),
        )]),
    )
}

fn grid_horizon(index: usize, top: f32, width: f32) -> View {
  View::decorative()
    .name(format!("arcade-attract-grid-horizon-{index}"))
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(top.pct())
        .left(50.pct())
        .width(width.pct())
        .height(2)
        .margin_left(Length::Percent(-width / 2.0)),
    )
    .paint(
      PaintStyle::new()
        .background(
          Gradient::linear(90.0)
            .stop(0.0, Color::TRANSPARENT)
            .stop(0.12, Color::rgba8(94, 212, 255, 173))
            .stop(0.88, Color::rgba8(210, 116, 255, 158))
            .stop(1.0, Color::TRANSPARENT),
        )
        .box_shadow([Shadow::outer(
          0.0,
          0.0,
          7.0,
          0.0,
          Color::rgba8(152, 101, 255, 71),
        )]),
    )
}

fn particle(index: usize, particle: Particle, reduce_motion: bool) -> View {
  let view = View::decorative()
    .name(format!("arcade-attract-particle-{index}"))
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(particle.y.pct())
        .left(particle.x.pct())
        .width(particle.size)
        .height(particle.size)
        .border_radius(particle.size / 2.0)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::Percent(50.0),
          Length::Percent(50.0),
        )),
    )
    .paint(self::particle_paint(particle));
  if reduce_motion {
    view.style(Style::new().opacity(0.62).scale(Scale::uniform(0.85)))
  } else {
    view
      .initial(StyleTarget::new().x(0.0).y(110.0).scale(0.55).opacity(0.0))
      .animate(
        MotionTarget::new(
          StyleTarget::new()
            .x(particle.drift_x)
            .y(particle.drift_y)
            .scale(1.22)
            .opacity_keyframes(
              Keyframes::new([0.0, 0.52, 0.96, 0.74, 0.0]).times([0.0, 0.12, 0.38, 0.76, 1.0]),
            ),
        )
        .transition(
          Transition::tween()
            .duration_secs(particle.duration)
            .delay_secs(-particle.phase * particle.duration)
            .ease(Easing::Linear)
            .repeat(Repeat::Forever),
        ),
      )
  }
}

fn particles() -> Vec<Particle> {
  let mut seed = 0x00a7_7ac7_u32;
  let mut random = || {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    f64::from(seed) / 4_294_967_296.0
  };
  (0..PARTICLE_COUNT)
    .map(|index| Particle {
      accent: index % 9 == 0,
      color: PARTICLE_COLORS[index % PARTICLE_COLORS.len()],
      drift_x: (-58.0 + random() * 116.0) as f32,
      drift_y: (-165.0 - random() * 210.0) as f32,
      duration: 8.0 + random() * 7.0,
      phase: random(),
      size: (3.0 + random() * 4.5) as f32,
      x: (3.0 + random() * 94.0) as f32,
      y: (12.0 + random() * 86.0) as f32,
    })
    .collect()
}

fn particle_paint(particle: Particle) -> PaintStyle {
  let glow = if particle.accent {
    vec![
      Shadow::outer(0.0, 0.0, 3.0, 0.0, Color::WHITE),
      Shadow::outer(0.0, 0.0, particle.size * 3.0, 0.0, particle.color),
      Shadow::outer(0.0, 0.0, particle.size * 6.0, 0.0, particle.color),
    ]
  } else {
    vec![
      Shadow::outer(0.0, 0.0, 2.0, 0.0, Color::WHITE),
      Shadow::outer(0.0, 0.0, particle.size * 3.5, 0.0, particle.color),
    ]
  };
  PaintStyle::new()
    .background(particle.color)
    .box_shadow(glow)
}

fn background_paint() -> PaintStyle {
  PaintStyle::new()
    .clip_polygon(frame_styles::clip())
    .background(
      Gradient::linear(180.0)
        .stop(0.0, Color::rgba8(3, 9, 26, 5))
        .stop(1.0, Color::rgba8(1, 5, 18, 61)),
    )
    .layer(PaintLayer::new(
      Gradient::radial([0.5, 0.68], [0.62, 0.34])
        .stop(0.0, Color::rgba8(18, 76, 144, 46))
        .stop(0.49, Color::TRANSPARENT),
    ))
}

fn vignette_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(90.0)
        .stop(0.0, Color::rgba8(1, 3, 12, 56))
        .stop(0.17, Color::TRANSPARENT)
        .stop(0.83, Color::TRANSPARENT)
        .stop(1.0, Color::rgba8(1, 3, 12, 56)),
    )
    .layer(PaintLayer::new(
      Gradient::radial([0.5, 0.44], [0.65, 0.45])
        .stop(0.0, Color::rgba8(1, 4, 16, 179))
        .stop(0.26, Color::rgba8(1, 4, 16, 179))
        .stop(0.60, Color::TRANSPARENT),
    ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn source_seed_is_reproducible() {
    let first = particles();
    let second = particles();
    assert_eq!(first.len(), PARTICLE_COUNT);
    assert_eq!(first[0].drift_x.to_bits(), second[0].drift_x.to_bits());
    assert_eq!(first[47].y.to_bits(), second[47].y.to_bits());
    assert!(first.iter().enumerate().all(|(index, value)| {
      value.accent == index.is_multiple_of(9)
        && (8.0..15.0).contains(&value.duration)
        && (3.0..7.5).contains(&value.size)
        && (3.0..97.0).contains(&value.x)
        && (12.0..98.0).contains(&value.y)
    }));
  }
}
