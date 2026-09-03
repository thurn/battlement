//! Stable typed motion values, native expression graphs, clocks, and playback.

mod playback;

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  fmt,
  marker::PhantomData,
  rc::{Rc, Weak},
  time::Duration,
};

use battlement::{
  CommandBody, MotionClockSource, MotionExpressionOperation,
  MotionPlaybackOutcome as ProtocolPlaybackOutcome, MotionValueCommand, MotionValueDescriptor,
  MotionValueOperation, MotionValueSource, ObjectId, SpringConfiguration, TransitionGenerator,
};

use crate::{
  hook_storage::{HookKind, HookSlot},
  hooks,
  motion::Transition,
  motion_value_runtime::{self, MotionValueRuntime},
};

/// A stable typed handle whose authoritative presentation value lives in Unity.
pub struct MotionValue<T: MotionValueType> {
  inner: Rc<MotionValueInner>,
  marker: PhantomData<T>,
}

/// An ordered typed input range with at least two values.
#[derive(Clone, Debug, PartialEq)]
pub struct InputRange<T: MotionValueType>(Vec<T>);

/// An ordered typed output range aligned with an [`InputRange`].
#[derive(Clone, Debug, PartialEq)]
pub struct OutputRange<T: MotionValueType>(Vec<T>);

/// Physical parameters for a passive motion-value spring.
#[derive(Clone, Debug, PartialEq)]
pub struct SpringOptions {
  transition: Transition,
}

/// A closed serializable expression that Unity can evaluate without Rust traffic.
pub struct MotionExpression<T: MotionValueType> {
  operation: Option<MotionExpressionOperation>,
  inputs: Vec<ErasedMotionValue>,
  marker: PhantomData<T>,
}

/// A deterministic clock advanced only by addressed commands.
#[derive(Clone)]
pub struct ControlledMotionClock {
  clock_id: ObjectId,
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
}

/// Source selected for [`use_motion_time`].
#[derive(Clone)]
pub enum MotionTimeSource {
  /// Unity unscaled runtime time.
  Unscaled,
  /// Unity scaled game time.
  Scaled,
  /// An explicitly advanced deterministic clock.
  Controlled(ControlledMotionClock),
  /// A Battlement-owned audio operation.
  Audio(AudioPlayback),
}

/// Explicit Rust-side sample requested from a native motion value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionValueEvent {
  /// The latest presentation value after it changes.
  Change,
  /// The latest per-second velocity after it changes.
  Velocity,
  /// The presentation value at every rendered frame.
  AnimationFrame,
}

/// Stable identity of one Battlement-owned audio playback operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AudioPlayback {
  operation_id: ObjectId,
}

/// Options used when creating one stable audio playback operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioPlaybackOptions {
  volume: f64,
  pitch: f64,
  looping: bool,
  fade_in: Duration,
}

/// Stable identity and controls for one imperative animation start.
#[derive(Clone)]
pub struct AnimationPlayback {
  inner: Rc<PlaybackInner>,
}

#[derive(Clone)]
pub(crate) struct MotionValueRuntimeHandle {
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
}

/// Terminal playback outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackOutcome {
  /// The terminal target was applied.
  Completed,
  /// Playback froze at its presentation value.
  Stopped,
  /// Playback was removed and exposed its lower layer.
  Cancelled,
}

impl From<ProtocolPlaybackOutcome> for PlaybackOutcome {
  fn from(value: ProtocolPlaybackOutcome) -> Self {
    match value {
      ProtocolPlaybackOutcome::Completed => Self::Completed,
      ProtocolPlaybackOutcome::Stopped => Self::Stopped,
      ProtocolPlaybackOutcome::Cancelled => Self::Cancelled,
    }
  }
}

/// Direction selected for imperative playback.
pub type PlaybackDirection = battlement::MotionPlaybackDirection;

/// Types that have one sealed representation in the native motion graph.
pub trait MotionValueType: private::MotionValueTypeSealed + Clone + PartialEq + 'static {
  #[doc(hidden)]
  fn into_motion_value(self) -> battlement::MotionValue;

  #[doc(hidden)]
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self>;

  #[doc(hidden)]
  fn mix(from: &Self, to: &Self, progress: f64) -> Self;

  #[doc(hidden)]
  fn range_scalar(&self) -> Option<f64>;
}

/// Motion-value types accepted by passive springs.
pub trait SpringValue: MotionValueType + private::SpringValueSealed {}

#[derive(Clone)]
pub(crate) struct ErasedMotionValue {
  inner: Rc<MotionValueInner>,
}

struct MotionValueInner {
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
  descriptor: MotionValueDescriptor,
  dependencies: Vec<ErasedMotionValue>,
  latest: RefCell<battlement::MotionValue>,
  subscriptions: RefCell<Vec<battlement::MotionValueSubscription>>,
}

struct MotionValueSlot<T: MotionValueType> {
  value: MotionValue<T>,
}

type MotionValueCallback<T> = Rc<RefCell<Option<Box<dyn Fn(T)>>>>;

struct MotionValueEventSlot<T: MotionValueType> {
  subscription_id: ObjectId,
  value_id: ObjectId,
  event: MotionValueEvent,
  callback: MotionValueCallback<T>,
}

struct PlaybackInner {
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
  playback_id: ObjectId,
  generation: u32,
  terminal: RefCell<Option<PlaybackOutcome>>,
  reported: RefCell<Option<PlaybackOutcome>>,
  callbacks: RefCell<PlaybackCallbacks>,
}

#[derive(Default)]
struct PlaybackCallbacks {
  complete: Option<Box<dyn FnOnce()>>,
  stop: Option<Box<dyn FnOnce()>>,
  cancel: Option<Box<dyn FnOnce()>>,
}

impl PlaybackInner {
  fn finish(&self, outcome: PlaybackOutcome) -> bool {
    if self.reported.borrow().is_some() {
      return false;
    }
    *self.terminal.borrow_mut() = Some(outcome);
    *self.reported.borrow_mut() = Some(outcome);
    let callback = match outcome {
      PlaybackOutcome::Completed => self.callbacks.borrow_mut().complete.take(),
      PlaybackOutcome::Stopped => self.callbacks.borrow_mut().stop.take(),
      PlaybackOutcome::Cancelled => self.callbacks.borrow_mut().cancel.take(),
    };
    if let Some(callback) = callback {
      callback();
    }
    true
  }
}

/// Creates one stable mutable value in the current component hook slot.
pub fn use_motion_value<T: MotionValueType>(initial: T) -> MotionValue<T> {
  use_value(initial, MotionValueSource::Mutable, Vec::new())
}

/// Creates one passive spring following `source`.
pub fn use_spring<T: SpringValue>(
  source: MotionValue<T>,
  options: SpringOptions,
) -> MotionValue<T> {
  let initial = source.mount_value();
  use_value(
    initial,
    MotionValueSource::Spring {
      source: source.id(),
      configuration: options.configuration(),
    },
    vec![source.erase()],
  )
}

/// Maps a source through aligned typed ranges in Unity.
pub fn use_transform<I, O>(
  source: MotionValue<I>,
  input: InputRange<I>,
  output: OutputRange<O>,
) -> MotionValue<O>
where
  I: MotionValueType,
  O: MotionValueType,
{
  assert_eq!(input.0.len(), output.0.len(), "motion ranges must align");
  let initial = map_range(&source.mount_value(), &input.0, &output.0, true);
  use_value(
    initial,
    MotionValueSource::Range {
      source: source.id(),
      input: input
        .0
        .into_iter()
        .map(MotionValueType::into_motion_value)
        .collect(),
      output: output
        .0
        .into_iter()
        .map(MotionValueType::into_motion_value)
        .collect(),
      clamp: true,
    },
    vec![source.erase()],
  )
}

/// Creates the per-second velocity of `source`.
pub fn use_velocity(source: MotionValue<f32>) -> MotionValue<f32> {
  use_value(
    0.0,
    MotionValueSource::Velocity {
      source: source.id(),
    },
    vec![source.erase()],
  )
}

/// Reads Unity's unscaled runtime clock as a duration.
pub fn use_time() -> MotionValue<Duration> {
  use_motion_time(MotionTimeSource::Unscaled)
}

/// Reads one Unity-local clock without per-frame Rust traffic.
pub fn use_motion_time(source: MotionTimeSource) -> MotionValue<Duration> {
  use_value(
    Duration::ZERO,
    MotionValueSource::Time(source.into_clock()),
    Vec::new(),
  )
}

/// Lowers one closed expression to a stable native graph value.
pub fn use_motion_expression<T: MotionValueType>(
  expression: MotionExpression<T>,
) -> MotionValue<T> {
  let operation = expression
    .operation
    .expect("a motion expression requires an operation");
  let inputs = expression.inputs;
  let initial = evaluate_expression::<T>(operation, &inputs);
  use_value(
    initial,
    MotionValueSource::Expression {
      operation,
      inputs: inputs.iter().map(ErasedMotionValue::id).collect(),
    },
    inputs,
  )
}

/// Subscribes Rust to one explicitly requested coalesced native value sample.
pub fn use_motion_value_event<T: MotionValueType>(
  value: MotionValue<T>,
  event: MotionValueEvent,
  callback: impl Fn(T) + 'static,
) {
  let callback: MotionValueCallback<T> = Rc::new(RefCell::new(Some(Box::new(callback))));
  let initial_callback = Rc::clone(&callback);
  hooks::use_slot(
    HookKind::MotionValueEvent,
    TypeId::of::<T>(),
    |_| {
      let subscription_id = ObjectId::new_v4();
      value
        .inner
        .subscriptions
        .borrow_mut()
        .push(battlement::MotionValueSubscription {
          subscription_id,
          value_id: value.id(),
          event: event.into_protocol(),
        });
      let weak_value = Rc::downgrade(&value.inner);
      let weak_callback = Rc::downgrade(&initial_callback);
      let runtime = value
        .inner
        .runtime
        .upgrade()
        .expect("motion-value runtime was released during render");
      runtime
        .borrow_mut()
        .register_subscription(subscription_id, move |sample| {
          let (Some(value), Some(callback)) = (weak_value.upgrade(), weak_callback.upgrade())
          else {
            return false;
          };
          assert_eq!(
            sample.value_id, value.descriptor.value_id,
            "motion-value sample targeted the wrong identity"
          );
          *value.latest.borrow_mut() = sample.value.clone();
          let selected = match event {
            MotionValueEvent::Velocity => &sample.velocity,
            MotionValueEvent::Change | MotionValueEvent::AnimationFrame => &sample.value,
          };
          let selected = T::from_motion_value(selected)
            .expect("motion-value sample retained the wrong typed value");
          callback
            .borrow()
            .as_ref()
            .expect("motion-value callback is installed")(selected);
          true
        });
      MotionValueEventSlot {
        subscription_id,
        value_id: value.id(),
        event,
        callback: initial_callback,
      }
    },
    |slot| {
      assert_eq!(
        slot.value_id,
        value.id(),
        "motion-value event source changed in a stable hook slot"
      );
      assert_eq!(
        slot.event, event,
        "motion-value event kind changed in a stable hook slot"
      );
      if !Rc::ptr_eq(&slot.callback, &callback) {
        let replacement = callback.borrow_mut().take();
        *slot.callback.borrow_mut() = replacement;
      }
    },
  );
}

/// Creates one deterministic controlled clock in the current hook slot.
pub fn use_controlled_motion_clock() -> ControlledMotionClock {
  #[derive(Clone)]
  struct ClockSlot(ControlledMotionClock);

  impl HookSlot for ClockSlot {
    fn as_any_mut(&mut self) -> &mut dyn Any {
      self
    }
    fn clone_box(&self) -> Box<dyn HookSlot> {
      Box::new(self.clone())
    }
    fn commit(&mut self) {}
    fn discard_pending(&mut self) {}
    fn has_pending(&self) -> bool {
      false
    }
    fn has_pending_change(&self) -> bool {
      false
    }
    fn context_changed(&self) -> bool {
      false
    }
    fn kind(&self) -> HookKind {
      HookKind::MotionValue
    }
    fn value_type(&self) -> TypeId {
      TypeId::of::<ControlledMotionClock>()
    }
  }

  hooks::use_slot(
    HookKind::MotionValue,
    TypeId::of::<ControlledMotionClock>(),
    |_| {
      let (runtime_id, runtime) = motion_value_runtime::current_runtime();
      ClockSlot(ControlledMotionClock {
        clock_id: ObjectId::new_v4(),
        runtime_id,
        runtime,
      })
    },
    |slot| slot.0.clone(),
  )
}

impl<T: MotionValueType> MotionValue<T> {
  /// Retargets this mutable value while preserving its passive effect.
  pub fn set(&self, value: T) {
    self.command(MotionValueCommand::Set(value.into_motion_value()));
  }

  /// Writes this mutable value, zeros velocity, and detaches passive effects.
  pub fn jump(&self, value: T) {
    self.command(MotionValueCommand::Jump(value.into_motion_value()));
  }

  /// Freezes this value's current presentation and zeros velocity.
  pub fn stop(&self) {
    self.command(MotionValueCommand::Stop);
  }

  /// Starts one independently controlled transition to `value`.
  pub fn animate(&self, value: T, transition: Transition) -> AnimationPlayback {
    let playback = AnimationPlayback::new(self.inner.runtime_id, self.inner.runtime.clone());
    self.command(MotionValueCommand::Animate {
      playback_id: playback.inner.playback_id,
      generation: playback.inner.generation,
      target: Box::new(value.into_motion_value()),
      transition: transition.default,
    });
    playback
  }

  /// Returns the latest Rust-visible checkpoint for this identity.
  pub fn get(&self) -> T {
    assert!(
      !crate::context::rendering(),
      "MotionValue::get is forbidden during render"
    );
    self.mount_value()
  }

  /// Returns this value's stable protocol identity.
  #[must_use]
  pub fn id(&self) -> ObjectId {
    self.inner.descriptor.value_id
  }

  pub(crate) fn erase(&self) -> ErasedMotionValue {
    ErasedMotionValue {
      inner: Rc::clone(&self.inner),
    }
  }

  fn mount_value(&self) -> T {
    T::from_motion_value(&self.inner.latest.borrow())
      .expect("motion value retained the wrong typed value")
  }

  fn command(&self, command: MotionValueCommand) {
    match &command {
      MotionValueCommand::Set(value) | MotionValueCommand::Jump(value) => {
        *self.inner.latest.borrow_mut() = value.clone();
      }
      MotionValueCommand::Stop | MotionValueCommand::Animate { .. } => {}
    }
    motion_value_runtime::queue(
      self.inner.runtime_id,
      &self.inner.runtime,
      CommandBody::MotionValue(MotionValueOperation {
        value_id: self.id(),
        command,
      }),
    );
  }
}

impl<T: MotionValueType> Clone for MotionValue<T> {
  fn clone(&self) -> Self {
    Self {
      inner: Rc::clone(&self.inner),
      marker: PhantomData,
    }
  }
}

impl<T: MotionValueType> fmt::Debug for MotionValue<T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_tuple("MotionValue")
      .field(&self.inner.descriptor.value_id)
      .finish()
  }
}

impl<T: MotionValueType> PartialEq for MotionValue<T> {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

impl<T: MotionValueType> Eq for MotionValue<T> {}

impl<T: MotionValueType> InputRange<T> {
  /// Creates an ordered input range with at least two values.
  pub fn new(values: impl IntoIterator<Item = T>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    assert!(
      values.len() >= 2,
      "motion input ranges require at least two values"
    );
    for pair in values.windows(2) {
      let left = pair[0]
        .range_scalar()
        .expect("motion input range type is not orderable");
      let right = pair[1]
        .range_scalar()
        .expect("motion input range type is not orderable");
      assert!(
        left < right,
        "motion input ranges must be strictly increasing"
      );
    }
    Self(values)
  }
}

impl<T: MotionValueType> OutputRange<T> {
  /// Creates an ordered output range with at least two values.
  pub fn new(values: impl IntoIterator<Item = T>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    assert!(
      values.len() >= 2,
      "motion output ranges require at least two values"
    );
    Self(values)
  }
}

impl SpringOptions {
  /// Creates Motion's physical spring defaults.
  #[must_use]
  pub fn new() -> Self {
    Self {
      transition: Transition::spring(),
    }
  }

  /// Sets spring stiffness.
  #[must_use]
  pub fn stiffness(mut self, value: f64) -> Self {
    self.transition = self.transition.stiffness(value);
    self
  }

  /// Sets spring damping.
  #[must_use]
  pub fn damping(mut self, value: f64) -> Self {
    self.transition = self.transition.damping(value);
    self
  }

  /// Sets spring mass.
  #[must_use]
  pub fn mass(mut self, value: f64) -> Self {
    self.transition = self.transition.mass(value);
    self
  }

  fn configuration(&self) -> SpringConfiguration {
    let TransitionGenerator::Spring(configuration) = self.transition.default.generator else {
      unreachable!("SpringOptions always contains spring timing")
    };
    configuration
  }
}

impl Default for SpringOptions {
  fn default() -> Self {
    Self::new()
  }
}

impl MotionExpression<f32> {
  /// Begins a scalar expression with one motion value.
  pub fn input(value: MotionValue<f32>) -> Self {
    Self {
      operation: None,
      inputs: vec![value.erase()],
      marker: PhantomData,
    }
  }

  /// Raises the input to `power`.
  pub fn pow(mut self, power: f64) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.operation = Some(MotionExpressionOperation::Power(power));
    self
  }

  /// Takes the nonnegative square root.
  pub fn sqrt(mut self) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.operation = Some(MotionExpressionOperation::SquareRoot);
    self
  }

  /// Wraps the input into `[min, max)`.
  pub fn wrap(mut self, min: f64, max: f64) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.operation = Some(MotionExpressionOperation::Wrap { min, max });
    self
  }

  /// Adds another scalar graph value.
  #[allow(clippy::should_implement_trait)]
  pub fn add(mut self, value: MotionValue<f32>) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.inputs.push(value.erase());
    self.operation = Some(MotionExpressionOperation::Add);
    self
  }

  /// Subtracts another scalar graph value.
  pub fn subtract(mut self, value: MotionValue<f32>) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.inputs.push(value.erase());
    self.operation = Some(MotionExpressionOperation::Subtract);
    self
  }

  /// Multiplies by another scalar graph value.
  pub fn multiply(mut self, value: MotionValue<f32>) -> Self {
    assert!(
      self.operation.is_none(),
      "motion expressions compose through graph values"
    );
    self.inputs.push(value.erase());
    self.operation = Some(MotionExpressionOperation::Multiply);
    self
  }
}

impl ControlledMotionClock {
  /// Replaces the exact elapsed time.
  pub fn set(&self, elapsed: Duration) {
    self.queue(battlement::MotionControlledClockCommand::Set {
      elapsed_micros: duration_micros(elapsed),
    });
  }

  /// Advances by an exact duration.
  pub fn advance(&self, delta: Duration) {
    self.queue(battlement::MotionControlledClockCommand::Advance {
      delta_micros: duration_micros(delta),
    });
  }

  fn queue(&self, command: battlement::MotionControlledClockCommand) {
    motion_value_runtime::queue(
      self.runtime_id,
      &self.runtime,
      CommandBody::MotionControlledClock(battlement::MotionControlledClockOperation {
        clock_id: self.clock_id,
        command,
      }),
    );
  }
}

impl MotionTimeSource {
  pub(crate) fn into_clock(self) -> MotionClockSource {
    match self {
      Self::Unscaled => MotionClockSource::Unscaled,
      Self::Scaled => MotionClockSource::Scaled,
      Self::Controlled(value) => MotionClockSource::Controlled(value.clock_id),
      Self::Audio(value) => MotionClockSource::Audio(value.operation_id),
    }
  }
}

impl MotionValueEvent {
  fn into_protocol(self) -> battlement::MotionValueEventKind {
    match self {
      Self::Change => battlement::MotionValueEventKind::Change,
      Self::Velocity => battlement::MotionValueEventKind::Velocity,
      Self::AnimationFrame => battlement::MotionValueEventKind::AnimationFrame,
    }
  }
}

impl<T: MotionValueType> HookSlot for MotionValueSlot<T> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      value: self.value.clone(),
    })
  }
  fn commit(&mut self) {}
  fn discard_pending(&mut self) {}
  fn has_pending(&self) -> bool {
    false
  }
  fn has_pending_change(&self) -> bool {
    false
  }
  fn context_changed(&self) -> bool {
    false
  }
  fn kind(&self) -> HookKind {
    HookKind::MotionValue
  }
  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

impl<T: MotionValueType> HookSlot for MotionValueEventSlot<T> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      subscription_id: self.subscription_id,
      value_id: self.value_id,
      event: self.event,
      callback: Rc::clone(&self.callback),
    })
  }
  fn commit(&mut self) {}
  fn discard_pending(&mut self) {}
  fn has_pending(&self) -> bool {
    false
  }
  fn has_pending_change(&self) -> bool {
    false
  }
  fn context_changed(&self) -> bool {
    false
  }
  fn kind(&self) -> HookKind {
    HookKind::MotionValueEvent
  }
  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

fn use_value<T: MotionValueType>(
  initial: T,
  source: MotionValueSource,
  dependencies: Vec<ErasedMotionValue>,
) -> MotionValue<T> {
  hooks::use_slot(
    HookKind::MotionValue,
    TypeId::of::<T>(),
    |_| {
      let (runtime_id, runtime) = motion_value_runtime::current_runtime();
      let initial = initial.into_motion_value();
      MotionValueSlot {
        value: MotionValue {
          inner: Rc::new(MotionValueInner {
            runtime_id,
            runtime,
            descriptor: MotionValueDescriptor {
              value_id: ObjectId::new_v4(),
              initial: initial.clone(),
              source,
            },
            dependencies,
            latest: RefCell::new(initial),
            subscriptions: RefCell::new(Vec::new()),
          }),
          marker: PhantomData,
        },
      }
    },
    |slot| slot.value.clone(),
  )
}

fn map_range<I: MotionValueType, O: MotionValueType>(
  value: &I,
  input: &[I],
  output: &[O],
  clamp: bool,
) -> O {
  let scalar = value
    .range_scalar()
    .expect("motion range input is not orderable");
  let mut segment = input.len() - 2;
  for index in 0..input.len() - 1 {
    if scalar
      <= input[index + 1]
        .range_scalar()
        .expect("range input is orderable")
    {
      segment = index;
      break;
    }
  }
  let start = input[segment]
    .range_scalar()
    .expect("range input is orderable");
  let end = input[segment + 1]
    .range_scalar()
    .expect("range input is orderable");
  let mut progress = (scalar - start) / (end - start);
  if clamp {
    progress = progress.clamp(0.0, 1.0);
  }
  O::mix(&output[segment], &output[segment + 1], progress)
}

fn evaluate_expression<T: MotionValueType>(
  operation: MotionExpressionOperation,
  inputs: &[ErasedMotionValue],
) -> T {
  let scalar = |index: usize| match &*inputs[index].inner.latest.borrow() {
    battlement::MotionValue::Scalar(value) => f64::from(*value),
    _ => panic!("scalar motion expression received an incompatible value"),
  };
  let result = match operation {
    MotionExpressionOperation::Add => scalar(0) + scalar(1),
    MotionExpressionOperation::Subtract => scalar(0) - scalar(1),
    MotionExpressionOperation::Multiply => scalar(0) * scalar(1),
    MotionExpressionOperation::Divide => scalar(0) / scalar(1),
    MotionExpressionOperation::Power(power) => scalar(0).powf(power),
    MotionExpressionOperation::SquareRoot => scalar(0).sqrt(),
    MotionExpressionOperation::Absolute => scalar(0).abs(),
    MotionExpressionOperation::Minimum => scalar(0).min(scalar(1)),
    MotionExpressionOperation::Maximum => scalar(0).max(scalar(1)),
    MotionExpressionOperation::Clamp { min, max } => scalar(0).clamp(min, max),
    MotionExpressionOperation::Modulo(modulus) => scalar(0).rem_euclid(modulus),
    MotionExpressionOperation::Wrap { min, max } => min + (scalar(0) - min).rem_euclid(max - min),
    MotionExpressionOperation::ExponentialDecay { rate } => (-rate * scalar(0)).exp(),
    MotionExpressionOperation::Mix => {
      let from = &inputs[0].inner.latest.borrow();
      let to = &inputs[1].inner.latest.borrow();
      let from = T::from_motion_value(from).expect("motion mix input type mismatch");
      let to = T::from_motion_value(to).expect("motion mix input type mismatch");
      return T::mix(&from, &to, scalar(2));
    }
  };
  T::from_motion_value(&battlement::MotionValue::Scalar(result as f32))
    .expect("motion expression output type mismatch")
}

fn duration_micros(value: Duration) -> u64 {
  u64::try_from(value.as_micros()).expect("motion duration exceeds protocol range")
}

fn duration_millis(value: Duration) -> u64 {
  u64::try_from(value.as_millis()).expect("audio duration exceeds protocol range")
}

mod value_types;

mod private {
  pub trait MotionValueTypeSealed {}
  pub trait SpringValueSealed {}
}
