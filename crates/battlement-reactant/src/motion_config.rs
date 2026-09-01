//! Hostless inherited Motion configuration.

use std::{any::TypeId, cell::RefCell};

use battlement::{MotionClockSource, ReducedMotionPolicy};

use crate::{
  motion::Transition,
  motion_value::MotionTimeSource,
  render::{Render, RenderSink},
  render_value::Sealed,
};

thread_local! {
  static CONFIGS: RefCell<Vec<MotionConfigState>> = const { RefCell::new(Vec::new()) };
}

/// Platform or explicit reduced-motion selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReducedMotion {
  /// Follow the native operating-system or browser preference.
  #[default]
  User,
  /// Always suppress spatial motion.
  Always,
  /// Never suppress spatial motion.
  Never,
}

/// Hostless inherited defaults for descendant Motion hosts.
pub struct MotionConfig<R> {
  child: R,
  transition: Option<Transition>,
  reduced_motion: Option<ReducedMotion>,
  time_source: Option<MotionTimeSource>,
}

#[derive(Clone, Debug)]
pub(crate) struct MotionConfigState {
  pub(crate) transition: Option<Transition>,
  pub(crate) reduced_motion: ReducedMotionPolicy,
  pub(crate) clock: MotionClockSource,
}

struct MotionConfigMarker;

struct ConfigGuard;

impl<R> MotionConfig<R> {
  /// Wraps content in a Motion configuration boundary.
  #[must_use]
  pub fn new(child: R) -> Self {
    Self {
      child,
      transition: None,
      reduced_motion: None,
      time_source: None,
    }
  }

  /// Sets the inherited transition for descendants without a local default.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.transition = Some(value);
    self
  }

  /// Selects inherited reduced-motion behavior.
  #[must_use]
  pub fn reduced_motion(mut self, value: ReducedMotion) -> Self {
    self.reduced_motion = Some(value);
    self
  }

  /// Selects the inherited native clock.
  #[must_use]
  pub fn time_source(mut self, value: MotionTimeSource) -> Self {
    self.time_source = Some(value);
    self
  }
}

impl<R: Render> Render for MotionConfig<R> {}

#[allow(private_interfaces)]
impl<R: Render> Sealed for MotionConfig<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<MotionConfigMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    with_config(self.state(), || self.child.render_into(sink));
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    let state = self.state();
    with_config(state, || self.child.render_owned(sink));
  }
}

/// Returns whether the current explicit configuration forces reduced motion.
///
/// Platform-owned `User` policy is resolved by the host before sampling; Rust
/// components that require a custom fallback should also expose an explicit
/// application override when deterministic rendering is required.
#[must_use]
pub fn use_reduced_motion() -> bool {
  current().reduced_motion == ReducedMotionPolicy::Always
}

pub(crate) fn current() -> MotionConfigState {
  CONFIGS
    .with(|configs| configs.borrow().last().cloned())
    .unwrap_or(MotionConfigState {
      transition: None,
      reduced_motion: ReducedMotionPolicy::Never,
      clock: MotionClockSource::Unscaled,
    })
}

impl<R> MotionConfig<R> {
  fn state(&self) -> MotionConfigState {
    let parent = current();
    MotionConfigState {
      transition: self.transition.clone().or(parent.transition),
      reduced_motion: self
        .reduced_motion
        .map_or(parent.reduced_motion, |value| match value {
          ReducedMotion::User => ReducedMotionPolicy::User,
          ReducedMotion::Always => ReducedMotionPolicy::Always,
          ReducedMotion::Never => ReducedMotionPolicy::Never,
        }),
      clock: self
        .time_source
        .clone()
        .map_or(parent.clock, MotionTimeSource::into_clock),
    }
  }
}

fn with_config<T>(state: MotionConfigState, operation: impl FnOnce() -> T) -> T {
  CONFIGS.with(|configs| configs.borrow_mut().push(state));
  let _guard = ConfigGuard;
  operation()
}

impl Drop for ConfigGuard {
  fn drop(&mut self) {
    CONFIGS.with(|configs| {
      configs.borrow_mut().pop();
    });
  }
}
