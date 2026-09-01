use battlement::{
  InertiaTarget as MotionInertiaTarget, MotionEasing, MotionProperty, MotionRepeat,
  MotionRepeatType, SpringConfiguration, StaggerDirection, TransitionDefinition,
  TransitionGenerator, VariantWhen,
};

use crate::{
  motion::{
    Easing, InertiaTarget, Repeat, RepeatType, SpringAuthoring, Transition, micros, validate_times,
  },
  motion_variants::VariantOrchestration,
};

impl InertiaTarget {
  /// Rounds the projected target to the nearest positive multiple.
  #[must_use]
  pub fn nearest_multiple(value: f64) -> Self {
    positive(value, "inertia target multiple");
    Self::NearestMultiple(value)
  }

  /// Rounds the projected target down to a positive multiple.
  #[must_use]
  pub fn floor_multiple(value: f64) -> Self {
    positive(value, "inertia target multiple");
    Self::FloorMultiple(value)
  }

  /// Rounds the projected target up to a positive multiple.
  #[must_use]
  pub fn ceiling_multiple(value: f64) -> Self {
    positive(value, "inertia target multiple");
    Self::CeilingMultiple(value)
  }

  /// Clamps the projected target to inclusive finite bounds.
  #[must_use]
  pub fn clamp(min: f64, max: f64) -> Self {
    assert!(
      min.is_finite() && max.is_finite(),
      "inertia target bounds must be finite"
    );
    assert!(min <= max, "inertia target minimum must not exceed maximum");
    Self::Clamp { min, max }
  }

  fn into_motion(self) -> MotionInertiaTarget {
    match self {
      Self::Identity => MotionInertiaTarget::Identity,
      Self::NearestMultiple(value) => MotionInertiaTarget::NearestMultiple(value),
      Self::FloorMultiple(value) => MotionInertiaTarget::FloorMultiple(value),
      Self::CeilingMultiple(value) => MotionInertiaTarget::CeilingMultiple(value),
      Self::Clamp { min, max } => MotionInertiaTarget::Clamp { min, max },
    }
  }
}

impl Transition {
  /// Creates Motion's explicit tween defaults.
  #[must_use]
  pub fn tween() -> Self {
    Self::new(TransitionDefinition::tween(), SpringAuthoring::NotSpring)
  }

  /// Creates Motion's explicit physical spring defaults.
  #[must_use]
  pub fn spring() -> Self {
    Self::new(
      TransitionDefinition::spring(),
      SpringAuthoring::Unconfigured,
    )
  }

  /// Creates Motion's explicit inertia defaults.
  #[must_use]
  pub fn inertia() -> Self {
    Self::new(
      TransitionDefinition::inertia(0.0),
      SpringAuthoring::NotSpring,
    )
  }

  /// Creates an immediate transition.
  #[must_use]
  pub fn immediate() -> Self {
    Self::new(
      TransitionDefinition {
        generator: TransitionGenerator::Immediate,
        delay_micros: 0,
        repeat: MotionRepeat::None,
        repeat_delay_micros: 0,
        repeat_type: MotionRepeatType::Loop,
      },
      SpringAuthoring::NotSpring,
    )
  }

  /// Selects concurrent, parent-first, or children-first variant playback.
  #[must_use]
  pub fn when(mut self, value: VariantWhen) -> Self {
    self.orchestration = self.orchestration.when(value);
    self
  }

  /// Delays propagated variant children from the orchestration origin.
  #[must_use]
  pub fn delay_children_secs(mut self, value: f64) -> Self {
    self.orchestration = self.orchestration.delay_children_secs(value);
    self
  }

  /// Assigns a delay step between direct propagated variant children.
  #[must_use]
  pub fn stagger_children_secs(mut self, value: f64) -> Self {
    self.orchestration = self.orchestration.stagger_secs(value);
    self
  }

  /// Selects forward or reverse propagated child order.
  #[must_use]
  pub fn stagger_direction(mut self, value: StaggerDirection) -> Self {
    self.orchestration = self.orchestration.stagger_direction(value);
    self
  }

  /// Supplies the participating child count needed by reverse staggering.
  #[must_use]
  pub fn stagger_child_count(mut self, value: u32) -> Self {
    self.orchestration = self.orchestration.child_count(value);
    self
  }

  /// Sets tween or duration-spring duration in seconds.
  #[must_use]
  pub fn duration_secs(mut self, value: f64) -> Self {
    let duration_micros = micros(value, false);
    match &mut self.default.generator {
      TransitionGenerator::Tween {
        duration_micros: current,
        ..
      } => *current = duration_micros,
      TransitionGenerator::Spring(_) => {
        self.require_spring_form(SpringAuthoring::Duration);
        let (bounce, mass) = self.duration_spring_options();
        self.default.generator = TransitionGenerator::Spring(SpringConfiguration::Duration {
          duration_micros,
          bounce,
          mass,
        });
      }
      _ => panic!("duration_secs requires a tween or spring transition"),
    }
    self
  }

  /// Sets a visual-duration spring in seconds.
  #[must_use]
  pub fn visual_duration_secs(mut self, value: f64) -> Self {
    let duration_micros = micros(value, false);
    self.require_spring_form(SpringAuthoring::VisualDuration);
    let (bounce, mass) = self.duration_spring_options();
    self.default.generator = TransitionGenerator::Spring(SpringConfiguration::VisualDuration {
      duration_micros,
      bounce,
      mass,
    });
    self
  }

  /// Sets the duration-spring bounce ratio.
  #[must_use]
  pub fn bounce(mut self, value: f64) -> Self {
    finite(value, "spring bounce");
    let requested = match self.spring_authoring {
      SpringAuthoring::VisualDuration => SpringAuthoring::VisualDuration,
      _ => SpringAuthoring::Duration,
    };
    self.require_spring_form(requested);
    let (duration_micros, mass) = self.duration_spring_duration_and_mass(requested);
    self.default.generator = match requested {
      SpringAuthoring::VisualDuration => {
        TransitionGenerator::Spring(SpringConfiguration::VisualDuration {
          duration_micros,
          bounce: value,
          mass,
        })
      }
      _ => TransitionGenerator::Spring(SpringConfiguration::Duration {
        duration_micros,
        bounce: value,
        mass,
      }),
    };
    self
  }

  /// Sets physical spring stiffness.
  #[must_use]
  pub fn stiffness(mut self, value: f64) -> Self {
    positive(value, "spring stiffness");
    *self.physical_spring().0 = value;
    self
  }

  /// Sets physical spring damping.
  #[must_use]
  pub fn damping(mut self, value: f64) -> Self {
    nonnegative(value, "spring damping");
    *self.physical_spring().1 = value;
    self
  }

  /// Sets spring mass.
  #[must_use]
  pub fn mass(mut self, value: f64) -> Self {
    positive(value, "spring mass");
    match &mut self.default.generator {
      TransitionGenerator::Spring(SpringConfiguration::Physical { mass, .. }) => *mass = value,
      TransitionGenerator::Spring(SpringConfiguration::Duration { mass, .. })
      | TransitionGenerator::Spring(SpringConfiguration::VisualDuration { mass, .. }) => {
        *mass = value
      }
      _ => panic!("mass requires a spring transition"),
    }
    self
  }

  /// Sets authored spring or inertia initial velocity.
  #[must_use]
  pub fn initial_velocity(mut self, value: f64) -> Self {
    finite(value, "initial velocity");
    if matches!(
      self.default.generator,
      TransitionGenerator::Spring(SpringConfiguration::Physical { .. })
    ) {
      self.require_spring_form(SpringAuthoring::Physical);
    }
    match &mut self.default.generator {
      TransitionGenerator::Spring(SpringConfiguration::Physical {
        initial_velocity, ..
      }) => {
        *initial_velocity = Some(value);
      }
      TransitionGenerator::Spring(_) => {
        panic!("initial_velocity cannot be mixed with duration spring options")
      }
      TransitionGenerator::Inertia {
        initial_velocity, ..
      } => *initial_velocity = value,
      _ => panic!("initial_velocity requires a spring or inertia transition"),
    }
    self
  }

  /// Sets physical spring rest-speed threshold.
  #[must_use]
  pub fn rest_speed(mut self, value: f64) -> Self {
    nonnegative(value, "spring rest speed");
    *self.physical_spring().4 = Some(value);
    self
  }

  /// Sets the spring or inertia terminal target-distance threshold.
  #[must_use]
  pub fn rest_delta(mut self, value: f64) -> Self {
    nonnegative(value, "rest delta");
    if matches!(
      self.default.generator,
      TransitionGenerator::Spring(SpringConfiguration::Physical { .. })
    ) {
      self.require_spring_form(SpringAuthoring::Physical);
    }
    match &mut self.default.generator {
      TransitionGenerator::Spring(SpringConfiguration::Physical { rest_delta, .. }) => {
        *rest_delta = Some(value);
      }
      TransitionGenerator::Spring(_) => {
        panic!("rest_delta cannot be mixed with duration spring options")
      }
      TransitionGenerator::Inertia { rest_delta, .. } => *rest_delta = value,
      _ => panic!("rest_delta requires a spring or inertia transition"),
    }
    self
  }

  /// Sets inertia target-displacement power.
  #[must_use]
  pub fn power(mut self, value: f64) -> Self {
    finite(value, "inertia power");
    *self.inertia_fields().0 = value;
    self
  }

  /// Sets inertia exponential time constant in seconds.
  #[must_use]
  pub fn time_constant_secs(mut self, value: f64) -> Self {
    *self.inertia_fields().1 = micros(value, false);
    self
  }

  /// Sets an inclusive inertia minimum boundary.
  #[must_use]
  pub fn minimum(mut self, value: f64) -> Self {
    finite(value, "inertia minimum");
    *self.inertia_fields().2 = Some(value);
    self
  }

  /// Sets an inclusive inertia maximum boundary.
  #[must_use]
  pub fn maximum(mut self, value: f64) -> Self {
    finite(value, "inertia maximum");
    *self.inertia_fields().3 = Some(value);
    self
  }

  /// Sets inertia boundary-spring stiffness.
  #[must_use]
  pub fn bounce_stiffness(mut self, value: f64) -> Self {
    positive(value, "inertia bounce stiffness");
    *self.inertia_fields().5 = value;
    self
  }

  /// Sets inertia boundary-spring damping.
  #[must_use]
  pub fn bounce_damping(mut self, value: f64) -> Self {
    nonnegative(value, "inertia bounce damping");
    *self.inertia_fields().6 = value;
    self
  }

  /// Sets the serializable inertia target modifier.
  #[must_use]
  pub fn target(mut self, value: InertiaTarget) -> Self {
    *self.inertia_fields().7 = value.into_motion();
    self
  }

  /// Sets signed delay in seconds.
  #[must_use]
  pub fn delay_secs(mut self, value: f64) -> Self {
    finite(value, "motion delay");
    self.default.delay_micros = (value * 1_000_000.0).round() as i64;
    self
  }

  /// Sets one tween easing for every segment.
  #[must_use]
  pub fn ease(mut self, value: Easing) -> Self {
    let TransitionGenerator::Tween { easings, .. } = &mut self.default.generator else {
      panic!("ease requires a tween transition");
    };
    *easings = vec![value.into_motion()];
    self
  }

  /// Sets one easing per tween segment.
  #[must_use]
  pub fn easings(mut self, values: impl IntoIterator<Item = Easing>) -> Self {
    let TransitionGenerator::Tween { easings, .. } = &mut self.default.generator else {
      panic!("easings requires a tween transition");
    };
    *easings = values.into_iter().map(Easing::into_motion).collect();
    self
  }

  /// Sets normalized transition-level keyframe times.
  #[must_use]
  pub fn times(mut self, values: impl IntoIterator<Item = f64>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    validate_times(&values);
    let TransitionGenerator::Tween { times, .. } = &mut self.default.generator else {
      panic!("times requires a tween transition");
    };
    *times = Some(values);
    self
  }

  /// Sets the additional iteration count.
  #[must_use]
  pub fn repeat(mut self, value: Repeat) -> Self {
    self.default.repeat = match value {
      Repeat::Count(value) => MotionRepeat::Count(value),
      Repeat::Forever => MotionRepeat::Forever,
    };
    self
  }

  /// Sets the inactive gap between iterations in seconds.
  #[must_use]
  pub fn repeat_delay_secs(mut self, value: f64) -> Self {
    self.default.repeat_delay_micros = micros(value, false);
    self
  }

  /// Sets repeat direction semantics.
  #[must_use]
  pub fn repeat_type(mut self, value: RepeatType) -> Self {
    self.default.repeat_type = match value {
      RepeatType::Loop => MotionRepeatType::Loop,
      RepeatType::Reverse => MotionRepeatType::Reverse,
      RepeatType::Mirror => MotionRepeatType::Mirror,
    };
    self
  }

  /// Replaces timing for one property.
  #[must_use]
  pub fn property(mut self, property: MotionProperty, value: Self) -> Self {
    if let Some(existing) = self.properties.iter_mut().find(|(key, _)| *key == property) {
      existing.1 = value.default;
    } else {
      self.properties.push((property, value.default));
    }
    self
  }

  pub(crate) fn for_property(&self, property: MotionProperty) -> TransitionDefinition {
    self
      .properties
      .iter()
      .find(|(key, _)| *key == property)
      .map_or_else(|| self.default.clone(), |(_, value)| value.clone())
  }

  pub(crate) fn merge_inherited(inherited: &Self, local: &Self) -> Self {
    let mut merged = local.clone();
    for (property, transition) in &inherited.properties {
      if !merged
        .properties
        .iter()
        .any(|(candidate, _)| candidate == property)
      {
        merged.properties.push((*property, transition.clone()));
      }
    }
    merged
  }

  fn new(default: TransitionDefinition, spring_authoring: SpringAuthoring) -> Self {
    Self {
      default,
      properties: Vec::new(),
      spring_authoring,
      orchestration: VariantOrchestration::new(),
    }
  }

  fn require_spring_form(&mut self, requested: SpringAuthoring) {
    match self.spring_authoring {
      SpringAuthoring::Unconfigured => self.spring_authoring = requested,
      current if current == requested => {}
      SpringAuthoring::Physical | SpringAuthoring::Duration | SpringAuthoring::VisualDuration => {
        panic!("physical and duration spring options cannot be mixed")
      }
      SpringAuthoring::NotSpring => panic!("spring option requires a spring transition"),
    }
  }

  fn physical_spring(
    &mut self,
  ) -> (
    &mut f64,
    &mut f64,
    &mut f64,
    &mut Option<f64>,
    &mut Option<f64>,
    &mut Option<f64>,
  ) {
    self.require_spring_form(SpringAuthoring::Physical);
    let TransitionGenerator::Spring(SpringConfiguration::Physical {
      stiffness,
      damping,
      mass,
      initial_velocity,
      rest_speed,
      rest_delta,
    }) = &mut self.default.generator
    else {
      panic!("physical spring configuration is unavailable");
    };
    (
      stiffness,
      damping,
      mass,
      initial_velocity,
      rest_speed,
      rest_delta,
    )
  }

  fn duration_spring_options(&self) -> (f64, f64) {
    match self.default.generator {
      TransitionGenerator::Spring(SpringConfiguration::Duration { bounce, mass, .. })
      | TransitionGenerator::Spring(SpringConfiguration::VisualDuration { bounce, mass, .. }) => {
        (bounce, mass)
      }
      TransitionGenerator::Spring(SpringConfiguration::Physical { mass, .. }) => (0.3, mass),
      _ => panic!("duration option requires a spring transition"),
    }
  }

  fn duration_spring_duration_and_mass(&self, requested: SpringAuthoring) -> (u64, f64) {
    match self.default.generator {
      TransitionGenerator::Spring(SpringConfiguration::Duration {
        duration_micros,
        mass,
        ..
      }) if requested == SpringAuthoring::Duration => (duration_micros, mass),
      TransitionGenerator::Spring(SpringConfiguration::VisualDuration {
        duration_micros,
        mass,
        ..
      }) if requested == SpringAuthoring::VisualDuration => (duration_micros, mass),
      TransitionGenerator::Spring(SpringConfiguration::Physical { mass, .. }) => (800_000, mass),
      _ => panic!("duration spring configuration is unavailable"),
    }
  }

  #[allow(clippy::type_complexity)]
  fn inertia_fields(
    &mut self,
  ) -> (
    &mut f64,
    &mut u64,
    &mut Option<f64>,
    &mut Option<f64>,
    &mut f64,
    &mut f64,
    &mut f64,
    &mut MotionInertiaTarget,
  ) {
    let TransitionGenerator::Inertia {
      power,
      time_constant_micros,
      minimum,
      maximum,
      rest_delta,
      bounce_stiffness,
      bounce_damping,
      target,
      ..
    } = &mut self.default.generator
    else {
      panic!("inertia option requires an inertia transition");
    };
    (
      power,
      time_constant_micros,
      minimum,
      maximum,
      rest_delta,
      bounce_stiffness,
      bounce_damping,
      target,
    )
  }
}

impl Easing {
  fn into_motion(self) -> MotionEasing {
    match self {
      Self::Linear => MotionEasing::Linear,
      Self::EaseIn => MotionEasing::EaseIn,
      Self::EaseOut => MotionEasing::EaseOut,
      Self::EaseInOut => MotionEasing::EaseInOut,
      Self::CubicBezier(value) => MotionEasing::CubicBezier(value),
      Self::Steps { count, position } => MotionEasing::Steps { count, position },
    }
  }
}

fn finite(value: f64, name: &str) {
  assert!(value.is_finite(), "{name} must be finite");
}

fn positive(value: f64, name: &str) {
  assert!(value.is_finite() && value > 0.0, "{name} must be positive");
}

fn nonnegative(value: f64, name: &str) {
  assert!(
    value.is_finite() && value >= 0.0,
    "{name} must be nonnegative"
  );
}
