//! Component-scoped monotonic timers.

use std::{rc::Rc, time::Duration};

use reactant_core::{app_runtime::ApplicationContext, hooks};

use crate::game_app::ServicesContext;

/// Runs a callback once after a monotonic duration and cancels it on unmount.
pub fn use_timeout(duration: Duration, callback: impl Fn() + 'static) {
  self::use_timer(duration, None, false, callback);
}

/// Runs a callback on a monotonic interval and cancels it on unmount.
/// Missed periods produce at most one callback per application poll.
pub fn use_interval(duration: Duration, callback: impl Fn() + 'static) {
  self::use_timer(duration, Some(duration), false, callback);
}

/// Runs once after unpaused application time, preserving remaining time while paused.
/// Changing the duration restarts the timer; resuming a fired timer does not rearm it.
/// Unmounting cancels the timer. The callback always uses the latest committed render.
pub fn use_pausable_timeout(duration: Duration, paused: bool, callback: impl Fn() + 'static) {
  self::use_timer(duration, None, paused, callback);
}

fn use_timer(
  duration: Duration,
  interval: Option<Duration>,
  paused: bool,
  callback: impl Fn() + 'static,
) {
  assert!(
    !duration.is_zero(),
    "Reactant timers require a positive duration"
  );
  let services = hooks::use_required_context::<ApplicationContext>()
    .value::<ServicesContext>()
    .expect("timers require a Reactant Application root");
  let coordinator = services
    .coordinator
    .upgrade()
    .expect("application runtime ended while rendering");
  let identity = hooks::use_id();
  let callback: Rc<dyn Fn()> = Rc::new(callback);
  let current = hooks::use_ref(callback.clone());
  let next = callback.clone();
  let committed = current.clone();
  hooks::use_effect_always(move || {
    committed.replace(next);
  });
  let pause_owner = coordinator.clone();
  let pause_identity = identity.clone();
  hooks::use_effect(
    move || {
      let current = current.clone();
      coordinator.register_timer(
        identity.clone(),
        duration,
        interval,
        paused,
        Rc::new(move || {
          current.with(|callback| callback());
        }),
      );
      move || coordinator.unregister_timer(&identity)
    },
    (duration, interval),
  );
  hooks::use_effect(
    move || pause_owner.set_timer_paused(&pause_identity, paused),
    (duration, interval, paused),
  );
}
