use serde::{Deserialize, Serialize};

use crate::MotionProperty;

/// Boundary placement for a stepped easing function.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StepPosition {
  /// Jumps at the beginning of each step.
  Start,
  /// Jumps at the end of each step.
  End,
}

/// Typed easing accepted by tween and CSS-style timelines.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionEasing {
  /// Constant normalized velocity.
  Linear,
  /// Motion's standard accelerating curve.
  EaseIn,
  /// Motion's standard decelerating curve.
  EaseOut,
  /// Motion's standard symmetric acceleration curve.
  EaseInOut,
  /// A cubic Bézier with x control points constrained to `0..=1`.
  CubicBezier([f32; 4]),
  /// A finite number of discrete easing steps.
  Steps {
    /// Number of steps; must be positive.
    count: u32,
    /// Whether each jump occurs at its step's start or end.
    position: StepPosition,
  },
}

impl MotionEasing {
  pub(crate) fn validate(self) -> Result<(), &'static str> {
    match self {
      Self::CubicBezier(values)
        if values.iter().any(|value| !value.is_finite())
          || !(0.0..=1.0).contains(&values[0])
          || !(0.0..=1.0).contains(&values[2]) =>
      {
        Err("cubic Bézier x coordinates must be finite and in 0..=1")
      }
      Self::Steps { count: 0, .. } => Err("step easing count must be positive"),
      _ => Ok(()),
    }
  }
}

/// Number of additional Motion iterations after the first.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionRepeat {
  /// Runs only the first iteration.
  None,
  /// Runs the supplied number of additional iterations.
  Count(u32),
  /// Repeats without a terminal completion.
  Forever,
}

/// How later Motion iterations derive their direction and endpoints.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionRepeatType {
  /// Starts each iteration at the authored origin.
  Loop,
  /// Alternates logical playback direction.
  Reverse,
  /// Swaps origin and target and negates physical initial velocity.
  Mirror,
}

/// Serializable inertia target modifier.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum InertiaTarget {
  /// Leaves the unconstrained target unchanged.
  Identity,
  /// Rounds to the nearest multiple.
  NearestMultiple(f64),
  /// Rounds down to a multiple.
  FloorMultiple(f64),
  /// Rounds up to a multiple.
  CeilingMultiple(f64),
  /// Clamps to inclusive bounds.
  Clamp {
    /// Inclusive lower target.
    min: f64,
    /// Inclusive upper target.
    max: f64,
  },
}

impl InertiaTarget {
  pub(crate) fn validate(self) -> Result<(), &'static str> {
    match self {
      Self::Identity => Ok(()),
      Self::NearestMultiple(value) | Self::FloorMultiple(value) | Self::CeilingMultiple(value)
        if value.is_finite() && value > 0.0 =>
      {
        Ok(())
      }
      Self::Clamp { min, max } if min.is_finite() && max.is_finite() && min <= max => Ok(()),
      _ => Err("inertia target modifier is invalid"),
    }
  }

  /// Applies the target operation to one finite scalar.
  #[must_use]
  pub fn apply(self, target: f64) -> f64 {
    match self {
      Self::Identity => target,
      Self::NearestMultiple(value) => (target / value).round() * value,
      Self::FloorMultiple(value) => (target / value).floor() * value,
      Self::CeilingMultiple(value) => (target / value).ceil() * value,
      Self::Clamp { min, max } => target.clamp(min, max),
    }
  }
}

/// Physical-parameter or duration-derived spring configuration.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum SpringConfiguration {
  /// Direct physical coefficients and completion thresholds.
  Physical {
    /// Hooke coefficient.
    stiffness: f64,
    /// Viscous damping coefficient.
    damping: f64,
    /// Animated mass.
    mass: f64,
    /// Optional authored initial velocity; omission adopts compatible incoming velocity.
    initial_velocity: Option<f64>,
    /// Terminal absolute velocity threshold.
    rest_speed: Option<f64>,
    /// Terminal target-distance threshold.
    rest_delta: Option<f64>,
  },
  /// Motion's duration and bounce solver.
  Duration {
    /// Requested duration in integer microseconds.
    duration_micros: u64,
    /// Bounce ratio before normative clamping.
    bounce: f64,
    /// Mass used to derive coefficients.
    mass: f64,
  },
  /// Motion's visual-duration angular-root solver.
  VisualDuration {
    /// Visual duration in integer microseconds.
    duration_micros: u64,
    /// Bounce ratio before normative clamping.
    bounce: f64,
    /// Mass used to derive coefficients.
    mass: f64,
  },
}

impl SpringConfiguration {
  pub(crate) fn validate(self) -> Result<(), &'static str> {
    match self {
      Self::Physical {
        stiffness,
        damping,
        mass,
        initial_velocity,
        rest_speed,
        rest_delta,
      } => {
        if !positive(stiffness) || !nonnegative(damping) || !positive(mass) {
          return Err("spring physical coefficients are invalid");
        }
        if initial_velocity.is_some_and(|value| !value.is_finite()) {
          return Err("spring initial velocity must be finite");
        }
        if rest_speed.is_some_and(|value| !nonnegative(value)) {
          return Err("spring rest speed must be nonnegative");
        }
        if rest_delta.is_some_and(|value| !nonnegative(value)) {
          return Err("spring rest delta must be nonnegative");
        }
        Ok(())
      }
      Self::Duration {
        duration_micros,
        bounce,
        mass,
      }
      | Self::VisualDuration {
        duration_micros,
        bounce,
        mass,
      } if duration_micros > 0 && bounce.is_finite() && positive(mass) => Ok(()),
      _ => Err("duration spring configuration is invalid"),
    }
  }
}

/// Fully normalized timing generator for one property track.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TransitionGenerator {
  /// Applies the target in the first eligible sample.
  Immediate,
  /// Samples keyframe segments over a finite duration.
  Tween {
    /// Active duration excluding delay and repeats.
    duration_micros: u64,
    /// One easing or a segment easing list.
    easings: Vec<MotionEasing>,
    /// Optional transition-level normalized keyframe times.
    times: Option<Vec<f64>>,
  },
  /// Closed-form physical or duration-derived spring.
  Spring(SpringConfiguration),
  /// Closed-form exponential inertia with an analytic boundary spring.
  Inertia {
    /// Initial velocity in canonical units per second.
    initial_velocity: f64,
    /// Target displacement power.
    power: f64,
    /// Exponential time constant in microseconds.
    time_constant_micros: u64,
    /// Optional lower boundary.
    minimum: Option<f64>,
    /// Optional upper boundary.
    maximum: Option<f64>,
    /// Terminal target-distance threshold.
    rest_delta: f64,
    /// Boundary spring stiffness.
    bounce_stiffness: f64,
    /// Boundary spring damping.
    bounce_damping: f64,
    /// Serializable target modifier.
    target: InertiaTarget,
  },
}

/// A generator plus delay and repetition semantics for one track.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransitionDefinition {
  /// Timing generator.
  pub generator: TransitionGenerator,
  /// Signed delay in microseconds; negative values seek into the timeline.
  pub delay_micros: i64,
  /// Additional iteration count.
  pub repeat: MotionRepeat,
  /// Inactive gap between iterations.
  pub repeat_delay_micros: u64,
  /// Endpoint/direction behavior for repeats.
  pub repeat_type: MotionRepeatType,
}

impl TransitionDefinition {
  /// Creates Motion's explicit tween defaults.
  #[must_use]
  pub fn tween() -> Self {
    Self {
      generator: TransitionGenerator::Tween {
        duration_micros: 300_000,
        easings: vec![MotionEasing::EaseInOut],
        times: None,
      },
      delay_micros: 0,
      repeat: MotionRepeat::None,
      repeat_delay_micros: 0,
      repeat_type: MotionRepeatType::Loop,
    }
  }

  /// Creates Motion's explicit physical spring defaults.
  #[must_use]
  pub const fn spring() -> Self {
    Self {
      generator: TransitionGenerator::Spring(SpringConfiguration::Physical {
        stiffness: 100.0,
        damping: 10.0,
        mass: 1.0,
        initial_velocity: None,
        rest_speed: None,
        rest_delta: None,
      }),
      delay_micros: 0,
      repeat: MotionRepeat::None,
      repeat_delay_micros: 0,
      repeat_type: MotionRepeatType::Loop,
    }
  }

  /// Creates Motion's explicit inertia defaults.
  #[must_use]
  pub const fn inertia(initial_velocity: f64) -> Self {
    Self {
      generator: TransitionGenerator::Inertia {
        initial_velocity,
        power: 0.8,
        time_constant_micros: 325_000,
        minimum: None,
        maximum: None,
        rest_delta: 0.5,
        bounce_stiffness: 500.0,
        bounce_damping: 10.0,
        target: InertiaTarget::Identity,
      },
      delay_micros: 0,
      repeat: MotionRepeat::None,
      repeat_delay_micros: 0,
      repeat_type: MotionRepeatType::Loop,
    }
  }

  pub(crate) fn validate(&self) -> Result<(), &'static str> {
    match &self.generator {
      TransitionGenerator::Immediate => {}
      TransitionGenerator::Tween {
        duration_micros,
        easings,
        times,
      } => {
        if *duration_micros == 0 && self.repeat != MotionRepeat::None {
          return Err("a repeating tween must have positive duration");
        }
        for easing in easings {
          easing.validate()?;
        }
        if let Some(times) = times {
          validate_times(times)?;
        }
      }
      TransitionGenerator::Spring(value) => value.validate()?,
      TransitionGenerator::Inertia {
        initial_velocity,
        power,
        time_constant_micros,
        minimum,
        maximum,
        rest_delta,
        bounce_stiffness,
        bounce_damping,
        target,
      } => {
        let bounds_valid = minimum.is_none_or(|value| value.is_finite())
          && maximum.is_none_or(|value| value.is_finite())
          && match (minimum, maximum) {
            (Some(minimum), Some(maximum)) => minimum <= maximum,
            _ => true,
          };
        if !initial_velocity.is_finite()
          || !power.is_finite()
          || *time_constant_micros == 0
          || !nonnegative(*rest_delta)
          || !positive(*bounce_stiffness)
          || !nonnegative(*bounce_damping)
          || !bounds_valid
        {
          return Err("inertia configuration is invalid");
        }
        target.validate()?;
      }
    }
    Ok(())
  }
}

/// One property-specific override on a transition.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PropertyTransition {
  /// Property receiving the override.
  pub property: MotionProperty,
  /// Complete replacement timing definition.
  pub transition: TransitionDefinition,
}

/// Default transition plus property-specific replacements.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionTransition {
  /// Default for every changed property without an override.
  pub default: TransitionDefinition,
  /// Unique per-property timing definitions.
  pub properties: Vec<PropertyTransition>,
}

fn validate_times(values: &[f64]) -> Result<(), &'static str> {
  if values.len() < 2 || values.first() != Some(&0.0) || values.last() != Some(&1.0) {
    return Err("transition times must begin at zero and end at one");
  }
  if values.iter().any(|value| !value.is_finite()) {
    return Err("transition times must be finite");
  }
  if values.windows(2).any(|values| values[0] > values[1]) {
    return Err("transition times must be nondecreasing");
  }
  Ok(())
}

fn positive(value: f64) -> bool {
  value.is_finite() && value > 0.0
}

fn nonnegative(value: f64) -> bool {
  value.is_finite() && value >= 0.0
}
