//! Component-scoped monotonic timers.

use std::{rc::Rc, time::Duration};

use reactant_core::{app_runtime::ApplicationContext, hooks};

use crate::game_app::ServicesContext;

/// Runs a callback once after a monotonic duration and cancels it on unmount.
pub fn use_timeout(duration: Duration, callback: impl Fn() + 'static) {
  self::use_timer(duration, None, callback);
}

/// Runs a callback on a monotonic interval and cancels it on unmount.
/// Missed periods produce at most one callback per application poll.
pub fn use_interval(duration: Duration, callback: impl Fn() + 'static) {
  self::use_timer(duration, Some(duration), callback);
}

fn use_timer(duration: Duration, interval: Option<Duration>, callback: impl Fn() + 'static) {
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
  hooks::use_effect(
    move || {
      let current = current.clone();
      coordinator.register_timer(
        identity.clone(),
        duration,
        interval,
        Rc::new(move || {
          current.with(|callback| callback());
        }),
      );
      move || coordinator.unregister_timer(&identity)
    },
    (duration, interval),
  );
}
