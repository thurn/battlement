//! Audio-clocked two-hit pulse for completed arcade controls.

use crate::background_music::{
  BackgroundMusicContext, BackgroundMusicStatus, use_background_music,
};
use battlement::{Color, LengthUnits, Position, Scale, Style};
use battlement_reactant::{paint::PaintStyle, prelude::*};

const HEARTBEAT_PERIOD: f64 = 60.0 / 56.0;
const HEARTBEAT_SECOND_HIT: f64 = 0.133_93;
const HEARTBEAT_PHASE: f64 = 1.04;
const HEARTBEAT_SAMPLES: u32 = 96;

/// Applies the source two-hit heartbeat using the shared native audio clock.
#[builder]
pub struct MusicHeartbeat {
  #[builder(required, into)]
  children: Children,
  reduced_motion: bool,
}

impl Component for MusicHeartbeat {
  fn render(&self) -> impl Render {
    let music = use_background_music();
    MotionConfig::new(self::surface(self, &music)).time_source(music.motion_time_source())
  }
}

/// Returns the source heartbeat strength for an audio-ledger position.
pub fn heartbeat_strength(current_time: f64) -> f32 {
  let cycle_position = (current_time - HEARTBEAT_PHASE).rem_euclid(HEARTBEAT_PERIOD);
  let second_offset = cycle_position - HEARTBEAT_SECOND_HIT;
  let time_since_second_hit = if second_offset.abs() < 1e-9 {
    0.0
  } else {
    second_offset.rem_euclid(HEARTBEAT_PERIOD)
  };
  let time_since_hit = cycle_position.min(time_since_second_hit);
  if time_since_hit > 0.14 {
    0.0
  } else {
    (f64::exp(-time_since_hit / 0.045) * (1.0 - time_since_hit / 0.14)) as f32
  }
}

fn surface(component: &MusicHeartbeat, music: &BackgroundMusicContext) -> View {
  let surface = View::new()
    .name("music-heartbeat")
    .style(
      Style::new()
        .position(Position::Relative)
        .width(100.pct())
        .height(100.pct())
        .scale(Scale::uniform(1.0)),
    )
    .paint(
      PaintStyle::new()
        .background(Color::TRANSPARENT)
        .paint_filter(self::filter(0.0)),
    )
    .child(MotionConfig::new(component.children.render()).time_source(MotionTimeSource::Unscaled));
  if component.reduced_motion
    || music.status != BackgroundMusicStatus::Playing
    || music.muted
    || music.master_volume == 0
    || music.music_volume == 0
  {
    surface
  } else {
    surface.animation(self::heartbeat_animation())
  }
}

fn heartbeat_animation() -> Animation {
  let times = (0..=HEARTBEAT_SAMPLES)
    .map(|index| f64::from(index) / f64::from(HEARTBEAT_SAMPLES))
    .collect::<Vec<_>>();
  let frames = times
    .iter()
    .map(|time| self::target(heartbeat_strength(*time * HEARTBEAT_PERIOD)))
    .collect::<Vec<_>>();
  Animation::new(Keyframes::new(frames).times(times))
    .duration_secs(HEARTBEAT_PERIOD)
    .ease(Easing::Linear)
    .iterations(AnimationIterations::Forever)
    .diagnostic_name("music-control-heartbeat")
}

fn target(strength: f32) -> StyleTarget {
  StyleTarget::new()
    .scale(1.0 + strength * 0.012)
    .paint_filter(self::filter(strength))
}

fn filter(strength: f32) -> PaintFilterList {
  PaintFilterList::default()
    .brightness(1.0 + strength * 0.075)
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      strength * 7.0,
      0.0,
      Color::rgb8(91, 224, 255).with_alpha(f64::from(strength) * 0.34),
    ))
}
