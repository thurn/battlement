//! Hostless inherited Motion configuration.

use std::{any::TypeId, cell::RefCell, rc::Rc};

use battlement::application::ReducedMotionPreference;
use battlement::{MotionClockSource, ReducedMotionPolicy};

use crate::{
  context::{ContextIdentity, ContextProvider, ProviderValue},
  hooks,
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
    let state = self.state();
    sink.push_provider::<MotionConfigMarker>(
      ProviderValue::new(
        ContextIdentity::of::<ReducedMotionPolicy>(),
        Rc::new(state.reduced_motion),
      ),
      |children| with_config(state, || self.child.render_into(children)),
    );
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    let state = self.state();
    sink.push_provider::<MotionConfigMarker>(
      ProviderValue::new(
        ContextIdentity::of::<ReducedMotionPolicy>(),
        Rc::new(state.reduced_motion),
      ),
      |children| with_config(state, || self.child.render_owned(children)),
    );
  }
}

/// Resolves the inherited motion policy against the latest host preference.
/// An unavailable host preference resolves to false under `User` policy.
#[must_use]
pub fn use_reduced_motion() -> bool {
  let preference = self::use_reduced_motion_preference();
  match hooks::use_context::<ReducedMotionPolicy>() {
    ReducedMotionPolicy::Always => true,
    ReducedMotionPolicy::Never => false,
    ReducedMotionPolicy::User => preference == ReducedMotionPreference::Reduce,
  }
}

/// Returns the host's preference independently of the inherited motion policy.
/// Host changes rerender applications; unsupported targets report `Unavailable`.
#[must_use]
pub fn use_reduced_motion_preference() -> ReducedMotionPreference {
  hooks::use_context::<ReducedMotionPreference>()
}

/// Provides host observations for a custom engine or an isolated preview.
/// This reports a preference; use `MotionConfig` to select application policy.
#[must_use]
pub fn preference_provider(
  value: ReducedMotionPreference,
) -> ContextProvider<ReducedMotionPreference> {
  ContextProvider::new().context(value)
}

pub(crate) fn current() -> MotionConfigState {
  CONFIGS
    .with(|configs| configs.borrow().last().cloned())
    .unwrap_or(MotionConfigState {
      transition: None,
      reduced_motion: ReducedMotionPolicy::User,
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
