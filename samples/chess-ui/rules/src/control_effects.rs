//! Animated shine and release effects shared by arcade controls.

use battlement::{
  Color, Gradient, Length, LengthUnits, Overflow, Position, Rotate, Style, TransformOrigin,
};
use battlement_reactant::prelude::{
  Animation, AnimationFill, AnimationPlayState, Decoration, DecorationOverflow, Easing, Keyframes,
  StyleTarget,
};
use battlement_reactant::{hooks, prelude::EventCallback};

const BUTTON_PARTICLES: [(f32, f32, f32, f32, f32, f32); 10] = [
  (0.10, 0.20, -34.0, -18.0, -22.0, 16.0),
  (0.28, 0.05, -12.0, -28.0, 72.0, 12.0),
  (0.52, 0.03, 5.0, -31.0, 94.0, 18.0),
  (0.78, 0.08, 19.0, -27.0, 112.0, 13.0),
  (0.93, 0.30, 34.0, -12.0, 18.0, 17.0),
  (0.95, 0.72, 36.0, 15.0, -20.0, 12.0),
  (0.72, 0.95, 16.0, 28.0, 68.0, 16.0),
  (0.45, 0.97, -4.0, 31.0, 92.0, 13.0),
  (0.17, 0.91, -24.0, 24.0, 118.0, 17.0),
  (0.03, 0.61, -36.0, 10.0, 14.0, 12.0),
];

const CHECKBOX_SPARKS: [(f32, f32, f32, f32); 6] = [
  (-38.0, -29.0, -38.0, 15.0),
  (0.0, -43.0, 90.0, 12.0),
  (39.0, -25.0, 36.0, 17.0),
  (44.0, 17.0, -24.0, 11.0),
  (5.0, 43.0, 84.0, 15.0),
  (-42.0, 22.0, 28.0, 13.0),
];

const SLIDER_PARTICLES: [(f32, f32, f32, f32); 8] = [
  (-35.0, -27.0, -34.0, 14.0),
  (-9.0, -39.0, 76.0, 11.0),
  (25.0, -34.0, 43.0, 16.0),
  (38.0, -5.0, 8.0, 12.0),
  (31.0, 29.0, -39.0, 15.0),
  (1.0, 42.0, 88.0, 12.0),
  (-31.0, 30.0, 37.0, 16.0),
  (-41.0, 2.0, -8.0, 11.0),
];

/// Playback controls used by deterministic review specimens.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectPlayback {
  /// Samples the effect this many seconds after its start.
  pub elapsed: f64,
  /// Holds the sampled frame instead of advancing the native clock.
  pub paused: bool,
  /// Uses the source's short reduced-motion duration.
  pub reduced_motion: bool,
}

/// Returns a generation and callback that restart an effect before forwarding a proposal.
pub fn use_burst_callback<T: Clone + 'static>(
  callback: EventCallback<T>,
) -> (u32, EventCallback<T>) {
  let (generation, set_generation) = hooks::use_state(0_u32);
  (
    generation,
    EventCallback::new(move |_value: T| {
      set_generation.update(|current| current.wrapping_add(1));
    })
    .then(callback),
  )
}

/// Release bookkeeping for a slider that changes continuously while captured.
pub struct SliderBurstInteraction {
  /// Current effect generation.
  pub generation: u32,
  /// Forwards value changes and bursts immediately outside a pointer drag.
  pub on_change: EventCallback<u32>,
  /// Marks the beginning of pointer capture.
  pub on_pointer_begin: EventCallback<()>,
  /// Ends pointer capture and emits exactly one release burst.
  pub on_pointer_release: EventCallback<()>,
  /// Ends pointer capture without emitting a burst.
  pub on_pointer_cancel: EventCallback<()>,
}

/// Tracks pointer and non-pointer slider releases with source-equivalent semantics.
pub fn use_slider_burst(callback: EventCallback<u32>) -> SliderBurstInteraction {
  let (generation, set_generation) = hooks::use_state(0_u32);
  let pointer_active = hooks::use_ref(false);
  SliderBurstInteraction {
    generation,
    on_change: EventCallback::new({
      let pointer_active = pointer_active.clone();
      let set_generation = set_generation.clone();
      move |_value: u32| {
        if !pointer_active.with(|active| *active) {
          set_generation.update(|current| current.wrapping_add(1));
        }
      }
    })
    .then(callback),
    on_pointer_begin: EventCallback::new({
      let pointer_active = pointer_active.clone();
      move |()| pointer_active.with_mut(|active| *active = true)
    }),
    on_pointer_release: EventCallback::new({
      let pointer_active = pointer_active.clone();
      move |()| {
        pointer_active.with_mut(|active| *active = false);
        set_generation.update(|current| current.wrapping_add(1));
      }
    }),
    on_pointer_cancel: EventCallback::new(move |()| {
      pointer_active.with_mut(|active| *active = false);
    }),
  }
}

/// Moving highlight used by source action buttons.
pub fn shine(active: bool, reduced_motion: bool, width: f32, inset: f32) -> Vec<Decoration> {
  if !active || reduced_motion {
    return Vec::new();
  }
  let available_width = width - inset * 2.0;
  let gradient = Gradient::linear(90.0)
    .stop(0.0, Color::TRANSPARENT)
    .stop(0.48, Color::WHITE.with_alpha(0.82))
    .stop(0.62, Color::hex(0xaef5ff).with_alpha(0.46))
    .stop(1.0, Color::TRANSPARENT);
  vec![
    Decoration::new()
      .key("control-shine")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(inset)
          .top(inset)
          .bottom(inset)
          .width(32.pct())
          .opacity(0.0),
      )
      .animation(
        Animation::new(
          Keyframes::new([
            StyleTarget::new()
              .x(0.0)
              .opacity(0.0)
              .background_gradient(gradient.clone()),
            StyleTarget::new()
              .x(0.12 * available_width)
              .opacity(0.78)
              .background_gradient(gradient.clone()),
            StyleTarget::new()
              .x(0.68 * available_width)
              .opacity(0.0)
              .background_gradient(gradient),
          ])
          .times([0.0, 0.18, 1.0]),
        )
        .duration_secs(0.72)
        .ease(Easing::EaseOut)
        .fill(AnimationFill::Both)
        .animation_key("control-shine-sweep")
        .diagnostic_name("control-shine-sweep"),
      ),
  ]
}

/// Full or compact release burst used by arcade buttons.
pub fn button_burst(generation: u32, compact: bool, playback: EffectPlayback) -> Vec<Decoration> {
  if generation == 0 {
    return Vec::new();
  }
  let duration = if playback.reduced_motion {
    0.01
  } else if compact {
    0.48
  } else {
    0.62
  };
  let beam_gradient = Gradient::linear(90.0)
    .stop(0.0, Color::TRANSPARENT)
    .stop(0.22, Color::hex(0x66f7ff))
    .stop(0.48, Color::WHITE)
    .stop(0.76, Color::hex(0xff5bd8))
    .stop(1.0, Color::TRANSPARENT);
  let mut decorations = vec![
    animated(
      Decoration::new().key((generation, "button-ring")).style(
        Style::new()
          .background_color(Color::TRANSPARENT)
          .margin(if compact { -3 } else { -5 })
          .border_width(if compact { 2 } else { 3 })
          .border_color(Color::hex(0x6bf6ff))
          .border_radius(if compact { 9 } else { 13 }),
      ),
      Keyframes::new([
        StyleTarget::new()
          .opacity(0.92)
          .scale(0.94)
          .background_gradient(self::transparent_gradient()),
        StyleTarget::new()
          .opacity(0.70)
          .scale(1.02)
          .background_gradient(self::transparent_gradient()),
        StyleTarget::new()
          .opacity(0.0)
          .scale(1.12)
          .background_gradient(self::transparent_gradient()),
      ]),
      duration,
      0.0,
      Easing::CubicBezier([0.16, 0.78, 0.30, 1.0]),
      playback,
      (generation, "button-ring"),
    ),
    animated(
      Decoration::new().key((generation, "button-beam")).style(
        Style::new()
          .background_color(Color::TRANSPARENT)
          .position(Position::Absolute)
          .left(7.pct())
          .right(7.pct())
          .top(-2)
          .height(if compact { 3 } else { 4 }),
      ),
      Keyframes::new([
        StyleTarget::new()
          .opacity(0.9)
          .scale_x(0.12)
          .background_gradient(beam_gradient.clone()),
        StyleTarget::new()
          .opacity(0.75)
          .scale_x(1.08)
          .background_gradient(beam_gradient.clone()),
        StyleTarget::new()
          .opacity(0.0)
          .scale_x(0.45)
          .background_gradient(beam_gradient),
      ]),
      duration * 0.7,
      0.0,
      Easing::EaseOut,
      playback,
      (generation, "button-beam"),
    ),
  ];
  decorations.extend(BUTTON_PARTICLES.into_iter().enumerate().map(
    |(index, (left, top, x, y, rotate, width))| {
      let distance = if compact { 0.7 } else { 1.0 };
      animated(
        Decoration::new()
          .key((generation, "button-particle", index))
          .style(particle_style(
            Length::Percent(left * 100.0),
            Length::Percent(top * 100.0),
            width * if compact { 0.8 } else { 1.25 },
            if compact { 3.0 } else { 4.0 },
            index,
          )),
        particle_frames(x, y, rotate, 0.15, distance, 0.3),
        duration * (0.72 + index as f64 * 0.015),
        if playback.reduced_motion {
          0.0
        } else {
          index as f64 * 0.008
        },
        Easing::CubicBezier([0.2, 0.82, 0.32, 1.0]),
        playback,
        (generation, "button-particle", index),
      )
    },
  ));
  decorations
}

/// Toggle-state burst keyed by the accepted state change.
pub fn checkbox_burst(generation: u32, checked: bool, playback: EffectPlayback) -> Vec<Decoration> {
  if generation == 0 {
    return Vec::new();
  }
  let color = if checked {
    Color::hex(0x5ff6ff)
  } else {
    Color::hex(0xff55c8)
  };
  let secondary = if checked {
    Color::hex(0x5f7dff)
  } else {
    Color::hex(0x775dff)
  };
  let duration = if playback.reduced_motion { 0.01 } else { 0.78 };
  let mut decorations = vec![
    animated(
      Decoration::new().key((generation, "checkbox-ring")).style(
        Style::new()
          .background_color(Color::TRANSPARENT)
          .margin(-5)
          .border_width(3)
          .border_color(color)
          .border_radius(15),
      ),
      Keyframes::new([
        StyleTarget::new()
          .opacity(0.9)
          .scale(0.78)
          .background_gradient(self::transparent_gradient()),
        StyleTarget::new()
          .opacity(0.0)
          .scale(1.72)
          .background_gradient(self::transparent_gradient()),
      ]),
      duration,
      0.0,
      Easing::CubicBezier([0.16, 0.8, 0.35, 1.0]),
      playback,
      (generation, "checkbox-ring"),
    ),
    animated(
      Decoration::new().key((generation, "checkbox-flash")).style(
        Style::new()
          .margin(-11)
          .border_top_width(3)
          .border_right_width(3)
          .border_bottom_width(3)
          .border_left_width(3)
          .border_top_color(color)
          .border_right_color(secondary)
          .border_bottom_color(Color::TRANSPARENT)
          .border_left_color(Color::TRANSPARENT)
          .border_radius(20),
      ),
      Keyframes::new([
        StyleTarget::new()
          .opacity(0.95)
          .scale(0.7)
          .rotate(if checked { -18.0 } else { 18.0 })
          .background_gradient(self::transparent_gradient()),
        StyleTarget::new()
          .opacity(0.7)
          .scale(1.1)
          .rotate(0.0)
          .background_gradient(self::transparent_gradient()),
        StyleTarget::new()
          .opacity(0.0)
          .scale(1.42)
          .rotate(0.0)
          .background_gradient(self::transparent_gradient()),
      ]),
      duration * 0.9,
      0.0,
      Easing::EaseOut,
      playback,
      (generation, "checkbox-flash"),
    ),
    animated(
      Decoration::new()
        .key((generation, "checkbox-beam"))
        .style(self::checkbox_beam_style(checked)),
      Keyframes::new([
        StyleTarget::new()
          .opacity(0.85)
          .scale_x(0.08)
          .background_gradient(self::checkbox_beam_gradient(color)),
        StyleTarget::new()
          .opacity(0.7)
          .scale_x(1.35)
          .background_gradient(self::checkbox_beam_gradient(color)),
        StyleTarget::new()
          .opacity(0.0)
          .scale_x(0.35)
          .background_gradient(self::checkbox_beam_gradient(color)),
      ]),
      duration * 0.65,
      0.0,
      Easing::EaseOut,
      playback,
      (generation, "checkbox-beam"),
    ),
  ];
  decorations.extend(CHECKBOX_SPARKS.into_iter().enumerate().map(
    |(index, (x, y, rotate, width))| {
      animated(
        Decoration::new()
          .key((generation, "checkbox-spark", index))
          .style(centered_particle_style(width, color, index)),
        particle_frames(x, y, rotate, 0.55, 1.0, 0.4),
        duration * (0.7 + index as f64 * 0.025),
        if playback.reduced_motion {
          0.0
        } else {
          index as f64 * 0.012
        },
        Easing::CubicBezier([0.2, 0.85, 0.35, 1.0]),
        playback,
        (generation, "checkbox-spark", index),
      )
    },
  ));
  decorations
}

/// Slider-thumb burst keyed by one completed release or keyboard change.
pub fn slider_burst(generation: u32, playback: EffectPlayback) -> Vec<Decoration> {
  if generation == 0 {
    return Vec::new();
  }
  let duration = if playback.reduced_motion { 0.01 } else { 0.66 };
  let mut decorations = vec![animated(
    Decoration::new().key((generation, "slider-ring")).style(
      Style::new()
        .background_color(Color::TRANSPARENT)
        .margin(-5)
        .border_width(3)
        .border_color(Color::hex(0x62f6ff))
        .border_radius(15),
    ),
    Keyframes::new([
      StyleTarget::new()
        .opacity(0.9)
        .scale(0.72)
        .rotate(-16.0)
        .background_gradient(self::transparent_gradient()),
      StyleTarget::new()
        .opacity(0.65)
        .scale(1.3)
        .rotate(-2.0)
        .background_gradient(self::transparent_gradient()),
      StyleTarget::new()
        .opacity(0.0)
        .scale(1.6)
        .rotate(12.0)
        .background_gradient(self::transparent_gradient()),
    ]),
    duration,
    0.0,
    Easing::CubicBezier([0.16, 0.8, 0.35, 1.0]),
    playback,
    (generation, "slider-ring"),
  )];
  decorations.extend(SLIDER_PARTICLES.into_iter().enumerate().map(
    |(index, (x, y, rotate, width))| {
      animated(
        Decoration::new()
          .key((generation, "slider-particle", index))
          .style(centered_particle_style(
            width,
            if index % 2 == 0 {
              Color::hex(0x68f7ff)
            } else {
              Color::hex(0xff5cda)
            },
            index,
          )),
        particle_frames(x, y, rotate, 0.45, 1.0, 0.35),
        duration * (0.72 + index as f64 * 0.025),
        if playback.reduced_motion {
          0.0
        } else {
          index as f64 * 0.01
        },
        Easing::CubicBezier([0.2, 0.85, 0.35, 1.0]),
        playback,
        (generation, "slider-particle", index),
      )
    },
  ));
  decorations
}

fn animated(
  decoration: Decoration,
  frames: Keyframes<StyleTarget>,
  duration: f64,
  delay: f64,
  easing: Easing,
  playback: EffectPlayback,
  key: impl std::hash::Hash,
) -> Decoration {
  decoration.overflow(DecorationOverflow::Visible).animation(
    Animation::new(frames)
      .duration_secs(duration)
      .delay_secs(delay - playback.elapsed)
      .ease(easing)
      .fill(AnimationFill::Both)
      .play_state(if playback.paused {
        AnimationPlayState::Paused
      } else {
        AnimationPlayState::Running
      })
      .animation_key(key),
  )
}

fn particle_frames(
  x: f32,
  y: f32,
  rotate: f32,
  start_distance: f32,
  end_distance: f32,
  start_scale: f32,
) -> Keyframes<StyleTarget> {
  Keyframes::new([
    StyleTarget::new()
      .opacity(0.95)
      .x(x * start_distance)
      .y(y * start_distance)
      .rotate(rotate)
      .scale_x(start_scale),
    StyleTarget::new()
      .opacity(0.9)
      .x(x * end_distance)
      .y(y * end_distance)
      .rotate(rotate)
      .scale_x(1.0),
    StyleTarget::new()
      .opacity(0.0)
      .x(x * end_distance)
      .y(y * end_distance)
      .rotate(rotate)
      .scale_x(0.2),
  ])
}

fn checkbox_beam_style(checked: bool) -> Style {
  let style = Style::new()
    .position(Position::Absolute)
    .top(50.pct())
    .width(28)
    .height(2);
  if checked {
    style.left(-34)
  } else {
    style.right(-34)
  }
}

fn checkbox_beam_gradient(color: Color) -> Gradient {
  Gradient::linear(90.0)
    .stop(0.0, Color::TRANSPARENT)
    .stop(0.24, color)
    .stop(0.5, Color::WHITE)
    .stop(0.76, color)
    .stop(1.0, Color::TRANSPARENT)
}

fn transparent_gradient() -> Gradient {
  Gradient::linear(0.0)
    .stop(0.0, Color::TRANSPARENT)
    .stop(1.0, Color::TRANSPARENT)
}

fn particle_style(left: Length, top: Length, width: f32, height: f32, index: usize) -> Style {
  Style::new()
    .position(Position::Absolute)
    .left(left)
    .top(top)
    .width(width)
    .height(height)
    .border_radius(2)
    .background_color(if index.is_multiple_of(2) {
      Color::hex(0x68f7ff)
    } else {
      Color::hex(0xff5cda)
    })
    .overflow(Overflow::Visible)
    .transform_origin(TransformOrigin::two_dimensional(
      Length::Percent(50.0),
      Length::Percent(50.0),
    ))
    .rotate(Rotate::degrees(0.0))
}

fn centered_particle_style(width: f32, color: Color, index: usize) -> Style {
  self::particle_style(
    Length::Percent(50.0),
    Length::Percent(50.0),
    width,
    3.0,
    index,
  )
  .margin_left(-width / 2.0)
  .margin_top(-1)
  .background_color(color)
}
