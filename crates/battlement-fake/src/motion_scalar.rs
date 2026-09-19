//! Scalar sampling shared by fake UI and world Motion targets.

use battlement::{
  InertiaTarget, MotionEasing, MotionRepeat, MotionRepeatType, SpringConfiguration, StepPosition,
  TransitionDefinition, TransitionGenerator,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
  pub(crate) value: f64,
  pub(crate) velocity: f64,
  pub(crate) done: bool,
  pub(crate) iteration: u32,
}

pub(crate) fn sample(
  origin: f64,
  target: f64,
  velocity: f64,
  transition: &TransitionDefinition,
  elapsed: u64,
) -> Sample {
  let active = elapsed as f64 - transition.delay_micros as f64;
  if active < 0.0 {
    return Sample {
      value: origin,
      velocity: 0.0,
      done: false,
      iteration: 0,
    };
  }
  match &transition.generator {
    TransitionGenerator::Immediate => terminal(target),
    TransitionGenerator::Tween {
      duration_micros,
      easings,
      ..
    } => {
      if *duration_micros == 0 {
        return terminal(target);
      }
      let (iteration, local, gap, done) = position(active, *duration_micros, transition);
      let reverse = reversed(iteration, transition.repeat_type);
      let progress = if gap {
        1.0
      } else {
        local / *duration_micros as f64
      };
      let progress = if reverse { 1.0 - progress } else { progress };
      let speed = if gap || done {
        0.0
      } else {
        (target - origin) / (*duration_micros as f64 / 1_000_000.0)
      };
      Sample {
        value: origin
          + (target - origin)
            * ease(
              easings.first().copied().unwrap_or(MotionEasing::Linear),
              progress,
            ),
        velocity: if reverse { -speed } else { speed },
        done,
        iteration,
      }
    }
    TransitionGenerator::Spring(configuration) => {
      let duration = spring_duration(origin, target, velocity, *configuration);
      let (iteration, local, gap, done) = position(active, duration, transition);
      let reverse = reversed(iteration, transition.repeat_type);
      let (from, to) = if reverse {
        (target, origin)
      } else {
        (origin, target)
      };
      let velocity = if reverse && transition.repeat_type == MotionRepeatType::Mirror {
        -velocity
      } else {
        velocity
      };
      let value = spring(from, to, velocity, *configuration, local as u64);
      Sample {
        velocity: if gap { 0.0 } else { value.velocity },
        done,
        iteration,
        ..value
      }
    }
    TransitionGenerator::Inertia { .. } => inertia(origin, &transition.generator, active),
  }
}

pub(crate) fn duration(
  origin: f64,
  target: f64,
  velocity: f64,
  transition: &TransitionDefinition,
) -> Option<u64> {
  let repeats = match transition.repeat {
    MotionRepeat::Forever => return None,
    MotionRepeat::Count(count) => u64::from(count),
    MotionRepeat::None => 0,
  };
  let duration = match transition.generator {
    TransitionGenerator::Immediate => 0,
    TransitionGenerator::Tween {
      duration_micros, ..
    } => duration_micros,
    TransitionGenerator::Spring(configuration) => {
      spring_duration(origin, target, velocity, configuration)
    }
    TransitionGenerator::Inertia { .. } => {
      return (0..=20_000_000)
        .step_by(50_000)
        .find(|elapsed| sample(origin, target, velocity, transition, *elapsed).done);
    }
  };
  let total = i128::from(transition.delay_micros)
    + i128::from(duration) * i128::from(repeats + 1)
    + i128::from(transition.repeat_delay_micros) * i128::from(repeats);
  Some(u64::try_from(total.max(0)).expect("Motion duration overflow"))
}

pub(crate) fn ease(easing: MotionEasing, progress: f64) -> f64 {
  let p = progress.clamp(0.0, 1.0);
  match easing {
    MotionEasing::Linear => p,
    MotionEasing::EaseIn => cubic([0.42, 0.0, 1.0, 1.0], p),
    MotionEasing::EaseOut => cubic([0.0, 0.0, 0.58, 1.0], p),
    MotionEasing::EaseInOut => cubic([0.42, 0.0, 0.58, 1.0], p),
    MotionEasing::InOutSine => -((std::f64::consts::PI * p).cos() - 1.0) / 2.0,
    MotionEasing::CubicBezier(points) => cubic(points.map(f64::from), p),
    MotionEasing::Steps {
      count,
      position: StepPosition::Start,
    } => ((p * f64::from(count)).ceil() / f64::from(count)).min(1.0),
    MotionEasing::Steps {
      count,
      position: StepPosition::End,
    } => (p * f64::from(count)).floor() / f64::from(count),
  }
}

pub(crate) fn spring_duration(
  origin: f64,
  target: f64,
  velocity: f64,
  config: SpringConfiguration,
) -> u64 {
  match config {
    SpringConfiguration::Duration {
      duration_micros, ..
    }
    | SpringConfiguration::VisualDuration {
      duration_micros, ..
    } => duration_micros,
    SpringConfiguration::Physical { .. } => (0..20_000_000)
      .step_by(50_000)
      .find(|elapsed| spring(origin, target, velocity, config, *elapsed).done)
      .unwrap_or(20_000_000),
  }
}

fn spring(
  origin: f64,
  target: f64,
  velocity: f64,
  config: SpringConfiguration,
  elapsed: u64,
) -> Sample {
  let (stiffness, damping, mass) = coefficients(config);
  let (initial, rest_speed, rest_delta, duration) = match config {
    SpringConfiguration::Physical {
      initial_velocity,
      rest_speed,
      rest_delta,
      ..
    } => (
      initial_velocity.unwrap_or(velocity),
      rest_speed.unwrap_or(if (target - origin).abs() < 5.0 {
        0.01
      } else {
        2.0
      }),
      rest_delta.unwrap_or(if (target - origin).abs() < 5.0 {
        0.005
      } else {
        0.5
      }),
      None,
    ),
    SpringConfiguration::Duration {
      duration_micros, ..
    }
    | SpringConfiguration::VisualDuration {
      duration_micros, ..
    } => (0.0, 0.0, 0.0, Some(duration_micros)),
  };
  let (value, velocity) = closed_spring(
    origin,
    target,
    initial,
    stiffness,
    damping,
    mass,
    elapsed as f64 / 1_000_000.0,
  );
  let done = duration.map_or(
    velocity.abs() <= rest_speed && (target - value).abs() <= rest_delta,
    |duration| elapsed >= duration,
  );
  if done {
    terminal(target)
  } else {
    Sample {
      value,
      velocity,
      done: false,
      iteration: 0,
    }
  }
}

fn coefficients(config: SpringConfiguration) -> (f64, f64, f64) {
  match config {
    SpringConfiguration::Physical {
      stiffness,
      damping,
      mass,
      ..
    } => (stiffness, damping, mass),
    SpringConfiguration::Duration {
      duration_micros,
      bounce,
      mass,
    } => {
      let duration = (duration_micros as f64 / 1_000_000.0).clamp(0.01, 10.0);
      let ratio = (1.0 - bounce).clamp(0.05, 1.0);
      let mut root = 5.0 / duration;
      for _ in 1..12 {
        let (envelope, derivative) = if ratio < 1.0 {
          let decay = root * ratio;
          let envelope =
            0.001 - (decay / (root * (1.0 - ratio * ratio).sqrt())) * (-decay * duration).exp();
          let e = ratio * ratio * root * root * duration;
          let factor = if -envelope + 0.001 > 0.0 { -1.0 } else { 1.0 };
          (
            envelope,
            factor * (-e * (-decay * duration).exp())
              / (root * root * (1.0 - ratio * ratio).sqrt()),
          )
        } else {
          (
            -0.001 + (-root * duration).exp() * (root * duration + 1.0),
            -(-root * duration).exp() * root * duration * duration,
          )
        };
        root -= envelope / derivative;
      }
      if !root.is_finite() {
        return (100.0, 10.0, mass);
      }
      let stiffness = root * root * mass;
      (stiffness, ratio * 2.0 * (mass * stiffness).sqrt(), mass)
    }
    SpringConfiguration::VisualDuration {
      duration_micros,
      bounce,
      mass,
    } => {
      let duration = (duration_micros as f64 / 1_000_000.0).clamp(0.01, 10.0);
      let root = 2.0 * std::f64::consts::PI / (1.2 * duration);
      let stiffness = root * root * mass;
      (
        stiffness,
        2.0 * (1.0 - bounce).clamp(0.05, 1.0) * (mass * stiffness).sqrt(),
        mass,
      )
    }
  }
}

fn closed_spring(
  origin: f64,
  target: f64,
  velocity: f64,
  stiffness: f64,
  damping: f64,
  mass: f64,
  t: f64,
) -> (f64, f64) {
  let delta = target - origin;
  let omega = (stiffness / mass).sqrt();
  let ratio = damping / (2.0 * (stiffness * mass).sqrt());
  if ratio < 1.0 {
    let frequency = omega * (1.0 - ratio * ratio).sqrt();
    let a = (ratio * omega * delta - velocity) / frequency;
    let envelope = (-ratio * omega * t).exp();
    let sin = (frequency * t).sin();
    let cos = (frequency * t).cos();
    (
      target - envelope * (a * sin + delta * cos),
      envelope
        * ((ratio * omega * a + delta * frequency) * sin
          + (ratio * omega * delta - a * frequency) * cos),
    )
  } else if ratio == 1.0 {
    let c = omega * delta - velocity;
    let envelope = (-omega * t).exp();
    (
      target - envelope * (delta + c * t),
      envelope * (omega * c * t - velocity),
    )
  } else {
    let damped = omega * (ratio * ratio - 1.0).sqrt();
    let p = (ratio * omega * delta - velocity) / damped;
    let bounded = (damped * t).min(300.0);
    let decay = (-ratio * omega * t).exp();
    (
      target - decay * (p * bounded.sinh() + delta * bounded.cosh()),
      decay
        * ((ratio * omega * p - delta * damped) * bounded.sinh()
          + (ratio * omega * delta - p * damped) * bounded.cosh()),
    )
  }
}

fn inertia(origin: f64, generator: &TransitionGenerator, active: f64) -> Sample {
  let TransitionGenerator::Inertia {
    initial_velocity,
    power,
    time_constant_micros,
    minimum,
    maximum,
    rest_delta,
    bounce_stiffness,
    bounce_damping,
    target,
  } = *generator
  else {
    unreachable!()
  };
  let raw = origin + power * initial_velocity;
  let target = match target {
    InertiaTarget::NearestMultiple(multiple) => (raw / multiple).round_ties_even() * multiple,
    modifier => modifier.apply(raw),
  };
  let boundary = if maximum.is_some_and(|max| target > max && origin <= max) {
    maximum
  } else if minimum.is_some_and(|min| target < min || origin < min) {
    minimum
  } else if maximum.is_some_and(|max| origin > max) {
    maximum
  } else {
    None
  };
  if let Some(boundary) = boundary {
    let ratio = (target - boundary) / (target - origin);
    let crossing = if origin == boundary || ratio <= 0.0 {
      0.0
    } else {
      -(time_constant_micros as f64) * ratio.ln()
    };
    if active >= crossing {
      let velocity = (target - boundary) * 1_000_000.0 / time_constant_micros as f64;
      return spring(
        boundary,
        boundary,
        velocity,
        SpringConfiguration::Physical {
          stiffness: bounce_stiffness,
          damping: bounce_damping,
          mass: 1.0,
          initial_velocity: Some(velocity),
          rest_speed: None,
          rest_delta: Some(rest_delta),
        },
        (active - crossing) as u64,
      );
    }
  }
  let decay = (-active / time_constant_micros as f64).exp();
  let value = target - (target - origin) * decay;
  if (target - value).abs() <= rest_delta {
    terminal(target)
  } else {
    Sample {
      value,
      velocity: (target - origin) * decay * 1_000_000.0 / time_constant_micros as f64,
      done: false,
      iteration: 0,
    }
  }
}

fn position(
  active: f64,
  duration: u64,
  transition: &TransitionDefinition,
) -> (u32, f64, bool, bool) {
  let cycle = duration as f64 + transition.repeat_delay_micros as f64;
  let iterations = match transition.repeat {
    MotionRepeat::Count(count) => u64::from(count) + 1,
    _ => 1,
  };
  let forever = transition.repeat == MotionRepeat::Forever;
  let total = if forever {
    f64::INFINITY
  } else {
    cycle * iterations as f64 - transition.repeat_delay_micros as f64
  };
  let done = active >= total;
  if cycle == 0.0 {
    return (0, 0.0, false, done);
  }
  let bounded = if done { total } else { active };
  let raw = (bounded / cycle).floor();
  let iteration = raw.min(if forever {
    f64::from(u32::MAX)
  } else {
    (iterations - 1) as f64
  }) as u32;
  let local = if done {
    duration as f64
  } else {
    bounded - f64::from(iteration) * cycle
  };
  (
    iteration,
    local.min(duration as f64),
    local > duration as f64,
    done,
  )
}

fn reversed(iteration: u32, mode: MotionRepeatType) -> bool {
  mode != MotionRepeatType::Loop && iteration % 2 == 1
}

fn terminal(value: f64) -> Sample {
  Sample {
    value,
    velocity: 0.0,
    done: true,
    iteration: 0,
  }
}

fn cubic([x1, y1, x2, y2]: [f64; 4], x: f64) -> f64 {
  let mut t = x;
  for _ in 0..8 {
    let error = bezier(t, x1, x2) - x;
    let slope =
      3.0 * (1.0 - t).powi(2) * x1 + 6.0 * (1.0 - t) * t * (x2 - x1) + 3.0 * t * t * (1.0 - x2);
    if slope.abs() < 1e-7 {
      break;
    }
    t = (t - error / slope).clamp(0.0, 1.0);
  }
  let (mut low, mut high) = (0.0, 1.0);
  for _ in 0..12 {
    if bezier(t, x1, x2) < x {
      low = t;
    } else {
      high = t;
    }
    t = (low + high) * 0.5;
  }
  bezier(t, y1, y2)
}

fn bezier(t: f64, a: f64, b: f64) -> f64 {
  3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t.powi(3)
}
