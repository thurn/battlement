use std::{
  cell::{Cell, RefCell},
  time::{Duration, Instant},
};

thread_local! {
  static DETAIL_ENABLED: Cell<bool> = const { Cell::new(false) };
  static CHILD_DURATION_US: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn initialize() {
  DETAIL_ENABLED
    .set(std::env::var("BATTLEMENT_REACTANT_PROFILE").is_ok_and(|value| value == "detail"));
}

pub(crate) fn start() -> Option<Instant> {
  DETAIL_ENABLED.get().then(|| {
    CHILD_DURATION_US.with_borrow_mut(|stack| stack.push(0));
    Instant::now()
  })
}

pub(crate) fn component<C: 'static>(started: Option<Instant>) {
  let Some(started) = started else {
    return;
  };
  let duration = started.elapsed();
  let duration_us = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
  let (self_duration_us, depth) = pop_span(duration_us);
  record_component(
    std::any::type_name::<C>(),
    duration,
    self_duration_us,
    depth,
  );
  let full_duration_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
  complete_instrumentation(full_duration_us);
}

fn pop_span(duration_us: u64) -> (u64, usize) {
  CHILD_DURATION_US.with_borrow_mut(|stack| {
    let children = stack.pop().expect("Reactant performance stack underflow");
    (duration_us.saturating_sub(children), stack.len())
  })
}

fn complete_instrumentation(full_duration_us: u64) {
  CHILD_DURATION_US.with_borrow_mut(|stack| {
    if let Some(parent) = stack.last_mut() {
      *parent = parent.saturating_add(full_duration_us);
    }
  });
}

fn record_component(
  component: &'static str,
  duration: Duration,
  self_duration_us: u64,
  depth: usize,
) {
  tracing::event!(
    name: "reactant.component.render",
    tracing::Level::INFO,
    component,
    duration_us = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX),
    self_duration_us,
    depth,
    "Reactant component rendered."
  );
}

#[cfg(test)]
mod tests {
  use super::{CHILD_DURATION_US, complete_instrumentation, pop_span};

  #[test]
  fn parent_exclusion_can_cover_the_child_instrumentation_span() {
    CHILD_DURATION_US.with_borrow_mut(|stack| {
      stack.clear();
      stack.push(0);
      stack.push(0);
    });
    assert_eq!(pop_span(10), (10, 1));
    complete_instrumentation(15);
    assert_eq!(pop_span(30), (15, 0));
    complete_instrumentation(35);
    CHILD_DURATION_US.with_borrow(|stack| assert!(stack.is_empty()));
  }
}
