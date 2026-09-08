//! Shared audio-native pulse composed directly into control hosts.

use std::time::Duration;

use battlement_reactant::{hooks, motion_value::MotionValue, prelude::*};

use crate::background_music::{BackgroundMusicContext, BackgroundMusicStatus};

const HEARTBEAT_PERIOD: f64 = 60.0 / 56.0;
const HEARTBEAT_SECOND_HIT: f64 = 0.133_93;
const HEARTBEAT_PHASE: f64 = 1.04;

/// Shared native presentation values for the current song phase.
#[derive(Clone, PartialEq)]
pub struct Heartbeat {
  scale: MotionValue<f32>,
}

/// Optional inherited heartbeat for controls that also work outside a music provider.
pub struct ControlHeartbeat(Option<Heartbeat>);

impl ControlHeartbeat {
  /// Composes the beat after the control's own interaction target.
  pub fn apply(&self, target: impl Into<MotionTarget>) -> MotionTarget {
    let target = target.into();
    match &self.0 {
      Some(beat) => target.with_style(StyleTarget::new().scale_factor(beat.scale.clone())),
      None => target,
    }
  }
}

/// Reads the shared beat while honoring the control's motion policy.
pub fn use_control_heartbeat(reduced_motion: bool) -> ControlHeartbeat {
  let music = hooks::use_optional_context::<BackgroundMusicContext>();
  ControlHeartbeat(
    music
      .filter(|music| !reduced_motion && music.status == BackgroundMusicStatus::Playing)
      .filter(|music| !music.muted && music.effective_volume > 0.0)
      .map(|music| music.heartbeat),
  )
}

/// Derives one two-hit envelope from absolute native audio time.
pub fn use_heartbeat(audio_time: MotionValue<Duration>) -> Heartbeat {
  let phase = use_motion_value(HEARTBEAT_PHASE as f32);
  let second_hit = use_motion_value(HEARTBEAT_SECOND_HIT as f32);
  let shifted = use_motion_expression(MotionExpression::seconds(audio_time).subtract(phase));
  let first = use_motion_expression(MotionExpression::input(shifted).wrap(0.0, HEARTBEAT_PERIOD));
  let shifted_second =
    use_motion_expression(MotionExpression::input(first.clone()).subtract(second_hit));
  let second =
    use_motion_expression(MotionExpression::input(shifted_second).wrap(0.0, HEARTBEAT_PERIOD));
  let elapsed = use_motion_expression(MotionExpression::input(first).minimum(second));
  let decay =
    use_motion_expression(MotionExpression::input(elapsed.clone()).exponential_decay(1.0 / 0.045));
  let envelope = use_transform(
    elapsed,
    InputRange::new([0.0, 0.14]),
    OutputRange::new([1.0, 0.0]),
  );
  let strength = use_motion_expression(MotionExpression::input(decay).multiply(envelope));
  Heartbeat {
    scale: use_transform(
      strength,
      InputRange::new([0.0, 1.0]),
      OutputRange::new([1.0, 1.012]),
    ),
  }
}
