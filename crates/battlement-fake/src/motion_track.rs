//! Host-neutral track sampling on an explicitly supplied clock.

use battlement::{
  MotionEasing, MotionPlaybackDirection, MotionPropertyTrack, MotionRepeatType, MotionValue,
  TransitionGenerator,
};

use crate::{motion_mix, motion_scalar};

#[derive(Clone)]
pub(crate) struct Track {
  pub(crate) definition: MotionPropertyTrack,
  pub(crate) origin: MotionValue,
  pub(crate) velocity: f64,
  pub(crate) done: bool,
  pub(crate) iteration: u32,
  incoming: f64,
  anchor_elapsed: u64,
}

impl Track {
  pub(crate) fn new(definition: MotionPropertyTrack, origin: MotionValue, velocity: f64) -> Self {
    Self {
      definition,
      origin,
      velocity,
      incoming: velocity,
      done: false,
      iteration: 0,
      anchor_elapsed: 0,
    }
  }

  pub(crate) fn reset(&mut self) {
    self.velocity = self.incoming;
    self.done = false;
    self.iteration = 0;
    self.anchor_elapsed = 0;
  }

  pub(crate) fn retarget(&mut self, origin: MotionValue) {
    self.origin = origin;
    self.incoming = self.velocity;
    self.done = false;
    self.iteration = 0;
    self.anchor_elapsed = 0;
  }

  pub(crate) fn retarget_destination(
    &mut self,
    origin: MotionValue,
    destination: MotionValue,
    elapsed: u64,
  ) {
    if self.target() == &destination {
      return;
    }
    self.origin = origin;
    self.incoming = self.velocity;
    self.definition.values = vec![destination];
    self.velocity = self.incoming;
    self.done = false;
    self.iteration = 0;
    self.anchor_elapsed = elapsed;
  }

  pub(crate) fn duration(&self) -> Option<u64> {
    let (origin, target) = match (&self.origin, self.target()) {
      (MotionValue::Scalar(origin), MotionValue::Scalar(target)) => {
        (f64::from(*origin), f64::from(*target))
      }
      _ => (0.0, 1.0),
    };
    motion_scalar::duration(origin, target, self.incoming, &self.definition.transition)
  }

  pub(crate) fn target(&self) -> &MotionValue {
    self.definition.values.last().unwrap_or(&self.origin)
  }

  pub(crate) fn sample(&mut self, elapsed: u64, direction: MotionPlaybackDirection) -> MotionValue {
    let elapsed = elapsed.saturating_sub(self.anchor_elapsed);
    let reverse = matches!(
      direction,
      MotionPlaybackDirection::Reverse | MotionPlaybackDirection::AlternateReverse
    );
    let mut transition = self.definition.transition.clone();
    if matches!(
      direction,
      MotionPlaybackDirection::Alternate | MotionPlaybackDirection::AlternateReverse
    ) {
      transition.repeat_type = MotionRepeatType::Reverse;
    }
    let authored = &self.definition.values;
    let mut values = if authored.len() < 2 {
      vec![self.origin.clone(), self.target().clone()]
    } else {
      authored.clone()
    };
    if reverse {
      values.reverse();
    }
    let incoming = if reverse {
      -self.incoming
    } else {
      self.incoming
    };
    let tween = match &transition.generator {
      TransitionGenerator::Tween {
        duration_micros,
        easings,
        times,
      } if values.len() > 2 => Some((*duration_micros, easings.clone(), times.clone())),
      _ => None,
    };
    if let Some((duration_micros, easings, times)) = tween {
      transition.generator = TransitionGenerator::Tween {
        duration_micros,
        easings: vec![MotionEasing::Linear],
        times: None,
      };
      let timeline = motion_scalar::sample(0.0, 1.0, 0.0, &transition, elapsed);
      let mut times = self
        .definition
        .times
        .as_ref()
        .or(times.as_ref())
        .cloned()
        .unwrap_or_else(|| {
          (0..values.len())
            .map(|i| i as f64 / (values.len() - 1) as f64)
            .collect()
        });
      if reverse {
        times = times.into_iter().rev().map(|v| 1.0 - v).collect();
      }
      let segment = (1..times.len())
        .find(|i| timeline.value <= times[*i])
        .unwrap_or(times.len() - 1)
        - 1;
      let span = times[segment + 1] - times[segment];
      let local = if span == 0.0 {
        1.0
      } else {
        (timeline.value - times[segment]) / span
      };
      let progress = motion_scalar::ease(
        easings
          .get(segment)
          .copied()
          .unwrap_or(MotionEasing::Linear),
        local,
      );
      self.velocity = 0.0;
      self.done = timeline.done;
      self.iteration = timeline.iteration;
      return motion_mix::mix(&values[segment], &values[segment + 1], progress);
    }
    let first = &values[0];
    let last = &values[values.len() - 1];
    let (sample, result) = if let (MotionValue::Scalar(a), MotionValue::Scalar(b)) = (first, last) {
      let sample =
        motion_scalar::sample(f64::from(*a), f64::from(*b), incoming, &transition, elapsed);
      (sample, MotionValue::Scalar(sample.value as f32))
    } else {
      let sample = motion_scalar::sample(0.0, 1.0, 0.0, &transition, elapsed);
      (sample, motion_mix::mix(first, last, sample.value))
    };
    self.velocity = if matches!(result, MotionValue::Scalar(_)) {
      sample.velocity
    } else {
      0.0
    };
    self.done = sample.done;
    self.iteration = sample.iteration;
    result
  }
}
