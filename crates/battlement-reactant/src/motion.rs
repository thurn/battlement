//! Typed Motion authoring for Reactant hosts and forwarding components.

use std::any::TypeId;

use battlement::{
  MotionCallbackSubscriptions, MotionClockSource, MotionColor, MotionDescriptor, MotionEasing,
  MotionGeneration, MotionLayer, MotionLength, MotionProperty, MotionPropertyTrack,
  MotionPropertyValue, MotionRepeat, MotionRepeatType, MotionSlotDescriptor, MotionSlotId,
  MotionTargetDescriptor, MotionValue, ReducedMotionPolicy, SpringConfiguration, StepPosition,
  TransitionDefinition, TransitionGenerator, Visibility,
};

use crate::{
  component::Component,
  render::{Render, RenderSink},
  render_value::Sealed,
};

/// A property-local sequence of typed Motion keyframes.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframes<T> {
  values: Vec<T>,
  times: Option<Vec<f64>>,
}

/// A collection of typed property targets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionStyle {
  entries: Vec<MotionStyleEntry>,
}

/// A style target with optional timing and terminal assignments.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionTarget {
  style: MotionStyle,
  transition: Option<Transition>,
  transition_end: MotionStyle,
}

/// Complete Motion props forwarded to one host façade.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionProps {
  initial: Option<InitialTarget>,
  animate: Option<MotionTarget>,
  exit: Option<MotionTarget>,
  transition: Option<Transition>,
}

/// Rust-only adapter that forwards one complete Motion value without a host wrapper.
#[doc(hidden)]
pub struct ForwardedMotion<C> {
  component: C,
  motion: MotionProps,
}

/// A concrete or disabled mount origin.
#[derive(Clone, Debug, PartialEq)]
pub enum InitialTarget {
  /// A concrete mount origin.
  Target(Box<MotionTarget>),
  /// Suppresses mount animation.
  Disabled,
}

/// Sealed input accepted by [`MotionProps::initial`].
pub trait InitialValue: private::InitialValueSealed {
  #[doc(hidden)]
  fn into_initial(self) -> InitialTarget;
}

/// Tween easing functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
  /// Constant normalized velocity.
  Linear,
  /// Motion's accelerating curve.
  EaseIn,
  /// Motion's decelerating curve.
  EaseOut,
  /// Motion's symmetric curve.
  EaseInOut,
  /// A cubic Bézier curve.
  CubicBezier([f32; 4]),
  /// A finite stepped curve.
  Steps {
    /// Number of steps.
    count: u32,
    /// Placement of each jump.
    position: StepPosition,
  },
}

/// Number of additional iterations after the first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Repeat {
  /// A finite number of additional iterations.
  Count(u32),
  /// No terminal iteration.
  Forever,
}

/// How repeated iterations derive their endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepeatType {
  /// Restart from the origin.
  Loop,
  /// Alternate playback direction.
  Reverse,
  /// Swap origin and target before sampling.
  Mirror,
}

/// Timing shared by a target with optional property replacements.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
  default: TransitionDefinition,
  properties: Vec<(MotionProperty, TransitionDefinition)>,
}

/// Components that forward one complete Motion value to a stable host.
pub trait MotionComponent: Component + Sized {
  /// Applies the complete forwarded Motion value.
  fn with_motion(self, motion: MotionProps) -> Self;
}

/// Motion builders available on forwarding components.
pub trait MotionComponentExt: MotionComponent + Clone {
  /// Applies a complete Motion value.
  #[must_use]
  fn motion(self, value: MotionProps) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, value)
  }

  /// Selects the mount origin.
  #[must_use]
  fn initial(self, value: impl InitialValue) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().initial(value))
  }

  /// Selects the base animation target.
  #[must_use]
  fn animate(self, value: impl Into<MotionTarget>) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().animate(value))
  }

  /// Selects the presence-exit target.
  #[must_use]
  fn exit(self, value: impl Into<MotionTarget>) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().exit(value))
  }

  /// Replaces the default transition.
  #[must_use]
  fn transition(self, value: Transition) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().transition(value))
  }
}

impl<T: MotionComponent + Clone> MotionComponentExt for T {}

impl<C> ForwardedMotion<C> {
  fn new(component: C, motion: MotionProps) -> Self {
    Self { component, motion }
  }

  /// Selects the mount origin.
  #[must_use]
  pub fn initial(mut self, value: impl InitialValue) -> Self {
    self.motion = self.motion.merge(MotionProps::new().initial(value));
    self
  }

  /// Selects the base animation target.
  #[must_use]
  pub fn animate(mut self, value: impl Into<MotionTarget>) -> Self {
    self.motion = self.motion.merge(MotionProps::new().animate(value));
    self
  }

  /// Selects the presence-exit target.
  #[must_use]
  pub fn exit(mut self, value: impl Into<MotionTarget>) -> Self {
    self.motion = self.motion.merge(MotionProps::new().exit(value));
    self
  }

  /// Replaces inherited target timing.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.motion = self.motion.merge(MotionProps::new().transition(value));
    self
  }
}

impl<C> Render for ForwardedMotion<C> where C: MotionComponent + Clone {}

#[allow(private_interfaces)]
impl<C> Sealed for ForwardedMotion<C>
where
  C: MotionComponent + Clone,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_motion_component::<Self, C>(self.component.clone(), self.motion.clone());
  }
}

#[derive(Clone, Debug, PartialEq)]
struct MotionStyleEntry {
  property: MotionProperty,
  values: Vec<MotionValue>,
  times: Option<Vec<f64>>,
}

impl<T> Keyframes<T> {
  /// Creates a sequence with evenly distributed times.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = T>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    assert!(!values.is_empty(), "motion keyframes cannot be empty");
    Self {
      values,
      times: None,
    }
  }

  /// Assigns normalized property-local times.
  #[must_use]
  pub fn times(mut self, values: impl IntoIterator<Item = f64>) -> Self {
    let times = values.into_iter().collect::<Vec<_>>();
    assert_eq!(
      times.len(),
      self.values.len(),
      "motion keyframe times must match values"
    );
    validate_times(&times);
    self.times = Some(times);
    self
  }
}

impl MotionStyle {
  /// Creates an empty target style.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      entries: Vec::new(),
    }
  }

  /// Sets opacity.
  #[must_use]
  pub fn opacity(self, value: f32) -> Self {
    self.scalar(MotionProperty::Opacity, value)
  }

  /// Sets opacity keyframes.
  #[must_use]
  pub fn opacity_keyframes(self, value: Keyframes<f32>) -> Self {
    self.scalar_keyframes(MotionProperty::Opacity, value)
  }

  /// Sets horizontal translation in panel pixels.
  #[must_use]
  pub fn x(self, value: f32) -> Self {
    self.length(MotionProperty::X, value.into())
  }

  /// Sets horizontal translation keyframes in panel pixels.
  #[must_use]
  pub fn x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.length_keyframes(MotionProperty::X, map_keyframes(value, MotionLength::px))
  }

  /// Sets vertical translation in panel pixels.
  #[must_use]
  pub fn y(self, value: f32) -> Self {
    self.length(MotionProperty::Y, value.into())
  }

  /// Sets vertical translation keyframes in panel pixels.
  #[must_use]
  pub fn y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.length_keyframes(MotionProperty::Y, map_keyframes(value, MotionLength::px))
  }

  /// Sets two-axis scale.
  #[must_use]
  pub fn scale(self, value: f32) -> Self {
    self.vector2(MotionProperty::Scale, [value, value])
  }

  /// Sets two-axis scale keyframes.
  #[must_use]
  pub fn scale_keyframes(self, value: Keyframes<f32>) -> Self {
    self.vector2_keyframes(
      MotionProperty::Scale,
      map_keyframes(value, |value| [value, value]),
    )
  }

  /// Sets structured two-axis background size.
  #[must_use]
  pub fn background_size(self, value: [f32; 2]) -> Self {
    self.vector2(MotionProperty::BackgroundSize, value)
  }

  /// Sets the background color.
  #[must_use]
  pub fn background_color(self, value: MotionColor) -> Self {
    self.color_property(MotionProperty::BackgroundColor, value)
  }

  /// Sets background-color keyframes.
  #[must_use]
  pub fn background_color_keyframes(self, value: Keyframes<MotionColor>) -> Self {
    self.color_keyframes(MotionProperty::BackgroundColor, value)
  }

  /// Sets text color.
  #[must_use]
  pub fn color(self, value: MotionColor) -> Self {
    self.color_property(MotionProperty::Color, value)
  }

  /// Sets discrete visibility.
  #[must_use]
  pub fn visibility(self, value: Visibility) -> Self {
    self.set(
      MotionProperty::Visibility,
      vec![MotionValue::Discrete(visibility_value(value).into())],
      None,
    )
  }

  /// Sets discrete visibility keyframes.
  #[must_use]
  pub fn visibility_keyframes(self, value: Keyframes<Visibility>) -> Self {
    self.set(
      MotionProperty::Visibility,
      value
        .values
        .into_iter()
        .map(|value| MotionValue::Discrete(visibility_value(value).into()))
        .collect(),
      value.times,
    )
  }

  fn scalar(self, property: MotionProperty, value: f32) -> Self {
    self.set(property, vec![MotionValue::Scalar(value)], None)
  }

  fn scalar_keyframes(self, property: MotionProperty, value: Keyframes<f32>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  fn length(self, property: MotionProperty, value: MotionLength) -> Self {
    self.set(property, vec![MotionValue::Length(value)], None)
  }

  fn length_keyframes(self, property: MotionProperty, value: Keyframes<MotionLength>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Length).collect(),
      value.times,
    )
  }

  fn vector2(self, property: MotionProperty, value: [f32; 2]) -> Self {
    self.set(property, vec![MotionValue::Vector2(value)], None)
  }

  fn vector2_keyframes(self, property: MotionProperty, value: Keyframes<[f32; 2]>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Vector2).collect(),
      value.times,
    )
  }

  fn color_property(self, property: MotionProperty, value: MotionColor) -> Self {
    self.set(property, vec![MotionValue::Color(value)], None)
  }

  fn color_keyframes(self, property: MotionProperty, value: Keyframes<MotionColor>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Color).collect(),
      value.times,
    )
  }

  fn set(
    mut self,
    property: MotionProperty,
    values: Vec<MotionValue>,
    times: Option<Vec<f64>>,
  ) -> Self {
    let entry = MotionStyleEntry {
      property,
      values,
      times,
    };
    if let Some(index) = self
      .entries
      .iter()
      .position(|value| value.property == property)
    {
      self.entries[index] = entry;
    } else {
      self.entries.push(entry);
    }
    self
  }

  fn target(&self, transition: Option<&Transition>) -> MotionTargetDescriptor {
    MotionTargetDescriptor {
      tracks: self
        .entries
        .iter()
        .map(|entry| MotionPropertyTrack {
          property: entry.property,
          values: entry.values.clone(),
          times: entry.times.clone(),
          transition: transition.map_or_else(
            || implicit_transition(entry),
            |transition| transition.for_property(entry.property),
          ),
        })
        .collect(),
      transition_end: Vec::new(),
    }
  }

  fn values(&self) -> Vec<MotionPropertyValue> {
    self
      .entries
      .iter()
      .map(|entry| MotionPropertyValue {
        property: entry.property,
        value: entry
          .values
          .last()
          .expect("motion style entry has a value")
          .clone(),
      })
      .collect()
  }
}

impl MotionTarget {
  /// Creates a target from typed style values.
  #[must_use]
  pub fn new(style: MotionStyle) -> Self {
    Self {
      style,
      transition: None,
      transition_end: MotionStyle::new(),
    }
  }

  /// Replaces timing for this target.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.transition = Some(value);
    self
  }

  /// Assigns values atomically after successful finite completion.
  #[must_use]
  pub fn transition_end(mut self, value: MotionStyle) -> Self {
    self.transition_end = value;
    self
  }

  fn descriptor(&self, inherited: Option<&Transition>) -> MotionTargetDescriptor {
    let transition = self.transition.as_ref().or(inherited);
    let mut target = self.style.target(transition);
    target.transition_end = self.transition_end.values();
    target
  }
}

impl From<MotionStyle> for MotionTarget {
  fn from(value: MotionStyle) -> Self {
    Self::new(value)
  }
}

impl MotionProps {
  /// Creates empty Motion props.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      initial: None,
      animate: None,
      exit: None,
      transition: None,
    }
  }

  /// Selects the mount origin.
  #[must_use]
  pub fn initial(mut self, value: impl InitialValue) -> Self {
    self.initial = Some(value.into_initial());
    self
  }

  /// Selects the base animation target.
  #[must_use]
  pub fn animate(mut self, value: impl Into<MotionTarget>) -> Self {
    self.animate = Some(value.into());
    self
  }

  /// Selects the stored presence-exit target.
  #[must_use]
  pub fn exit(mut self, value: impl Into<MotionTarget>) -> Self {
    self.exit = Some(value.into());
    self
  }

  /// Replaces target timing inherited by properties without local overrides.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.transition = Some(value);
    self
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    if value.initial.is_some() {
      self.initial = value.initial;
    }
    if value.animate.is_some() {
      self.animate = value.animate;
    }
    if value.exit.is_some() {
      self.exit = value.exit;
    }
    if value.transition.is_some() {
      self.transition = value.transition;
    }
    self
  }

  pub(crate) fn descriptor(
    &self,
    host_id: battlement::ObjectId,
    generation: MotionGeneration,
  ) -> MotionDescriptor {
    let transition = self.transition.as_ref();
    let (initial, initial_disabled) = match &self.initial {
      Some(InitialTarget::Target(value)) => (
        Some(value.descriptor(Some(&Transition::immediate()))),
        false,
      ),
      Some(InitialTarget::Disabled) => (None, true),
      None => (None, false),
    };
    let slots = self
      .animate
      .iter()
      .map(|target| MotionSlotDescriptor {
        slot: MotionSlotId(1),
        generation,
        layer: MotionLayer::Animate,
        target: target.descriptor(transition),
        callbacks: MotionCallbackSubscriptions::default(),
      })
      .collect();
    MotionDescriptor {
      descriptor_id: host_id,
      host_id,
      generation,
      static_baseline: Vec::new(),
      initial,
      initial_disabled,
      slots,
      clock: MotionClockSource::Unscaled,
      reduced_motion: ReducedMotionPolicy::Never,
    }
  }
}

impl InitialValue for bool {
  fn into_initial(self) -> InitialTarget {
    assert!(!self, "initial(true) has no Motion meaning");
    InitialTarget::Disabled
  }
}

impl InitialValue for MotionStyle {
  fn into_initial(self) -> InitialTarget {
    InitialTarget::Target(Box::new(self.into()))
  }
}

impl InitialValue for MotionTarget {
  fn into_initial(self) -> InitialTarget {
    InitialTarget::Target(Box::new(self))
  }
}

impl Transition {
  /// Creates Motion's explicit tween defaults.
  #[must_use]
  pub fn tween() -> Self {
    Self {
      default: TransitionDefinition::tween(),
      properties: Vec::new(),
    }
  }

  /// Creates an immediate transition.
  #[must_use]
  pub fn immediate() -> Self {
    Self {
      default: TransitionDefinition {
        generator: TransitionGenerator::Immediate,
        delay_micros: 0,
        repeat: MotionRepeat::None,
        repeat_delay_micros: 0,
        repeat_type: MotionRepeatType::Loop,
      },
      properties: Vec::new(),
    }
  }

  /// Sets tween duration in seconds.
  #[must_use]
  pub fn duration_secs(mut self, value: f64) -> Self {
    let TransitionGenerator::Tween {
      duration_micros, ..
    } = &mut self.default.generator
    else {
      panic!("duration_secs requires a tween transition");
    };
    *duration_micros = micros(value, false);
    self
  }

  /// Sets signed delay in seconds.
  #[must_use]
  pub fn delay_secs(mut self, value: f64) -> Self {
    assert!(value.is_finite(), "motion delay must be finite");
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

  fn for_property(&self, property: MotionProperty) -> TransitionDefinition {
    self
      .properties
      .iter()
      .find(|(key, _)| *key == property)
      .map_or_else(|| self.default.clone(), |(_, value)| value.clone())
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

fn map_keyframes<T, U>(value: Keyframes<T>, map: impl Fn(T) -> U) -> Keyframes<U> {
  Keyframes {
    values: value.values.into_iter().map(map).collect(),
    times: value.times,
  }
}

fn implicit_transition(entry: &MotionStyleEntry) -> TransitionDefinition {
  if entry.values.len() > 2 {
    return Transition::tween().duration_secs(0.8).default;
  }
  match entry.property {
    MotionProperty::X
    | MotionProperty::Y
    | MotionProperty::Z
    | MotionProperty::Translate
    | MotionProperty::Rotate
    | MotionProperty::RotateX
    | MotionProperty::RotateY => physical_default(500.0, 25.0),
    MotionProperty::Scale | MotionProperty::ScaleX | MotionProperty::ScaleY => {
      let damping = if targets_zero(entry) {
        2.0 * 550.0_f64.sqrt()
      } else {
        30.0
      };
      physical_default(550.0, damping)
    }
    _ => {
      Transition::tween()
        .ease(Easing::CubicBezier([0.25, 0.1, 0.35, 1.0]))
        .default
    }
  }
}

fn physical_default(stiffness: f64, damping: f64) -> TransitionDefinition {
  TransitionDefinition {
    generator: TransitionGenerator::Spring(SpringConfiguration::Physical {
      stiffness,
      damping,
      mass: 1.0,
      initial_velocity: None,
      rest_speed: Some(10.0),
      rest_delta: None,
    }),
    delay_micros: 0,
    repeat: MotionRepeat::None,
    repeat_delay_micros: 0,
    repeat_type: MotionRepeatType::Loop,
  }
}

fn targets_zero(entry: &MotionStyleEntry) -> bool {
  match entry.values.last() {
    Some(MotionValue::Scalar(value)) => *value == 0.0,
    Some(MotionValue::Vector2(value)) => value.iter().all(|value| *value == 0.0),
    _ => false,
  }
}

fn visibility_value(value: Visibility) -> &'static str {
  match value {
    Visibility::Visible => "visible",
    Visibility::Hidden => "hidden",
  }
}

fn micros(value: f64, allow_zero: bool) -> u64 {
  assert!(
    value.is_finite() && value >= 0.0,
    "motion duration must be finite and nonnegative"
  );
  let value = (value * 1_000_000.0).round() as u64;
  assert!(allow_zero || value > 0, "motion duration must be positive");
  value
}

fn validate_times(values: &[f64]) {
  assert!(
    values.len() >= 2,
    "motion times require at least two entries"
  );
  assert_eq!(
    values.first(),
    Some(&0.0),
    "motion times must begin at zero"
  );
  assert_eq!(values.last(), Some(&1.0), "motion times must end at one");
  assert!(
    values.iter().all(|value| value.is_finite()),
    "motion times must be finite"
  );
  assert!(
    values.windows(2).all(|pair| pair[0] <= pair[1]),
    "motion times must be nondecreasing"
  );
}

mod private {
  pub trait InitialValueSealed {}

  impl InitialValueSealed for bool {}
  impl InitialValueSealed for super::MotionStyle {}
  impl InitialValueSealed for super::MotionTarget {}
}
