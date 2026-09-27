use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  time::Instant,
};

thread_local! {
  static DETAIL_ENABLED: Cell<bool> = const { Cell::new(false) };
  static COMPONENT_TOTALS: RefCell<BTreeMap<&'static str, ComponentTotals>> = const { RefCell::new(BTreeMap::new()) };
  static CHILD_DURATION_US: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct ComponentTotals {
  calls: u64,
  duration_us: u64,
  self_duration_us: u64,
  maximum_self_duration_us: u64,
}

impl ComponentTotals {
  fn add(&mut self, duration_us: u64, self_duration_us: u64) {
    self.calls += 1;
    self.duration_us += duration_us;
    self.self_duration_us += self_duration_us;
    self.maximum_self_duration_us = self.maximum_self_duration_us.max(self_duration_us);
  }
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
  component_named(std::any::type_name::<C>(), started);
}

pub(crate) fn component_named(component: &'static str, started: Option<Instant>) {
  let Some(started) = started else {
    return;
  };
  let duration = started.elapsed();
  let duration_us = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
  let (self_duration_us, depth) = pop_span(duration_us);
  COMPONENT_TOTALS.with_borrow_mut(|totals| {
    totals
      .entry(component)
      .or_default()
      .add(duration_us, self_duration_us);
  });
  if depth == 0 {
    COMPONENT_TOTALS.with_borrow_mut(|totals| {
      for (component, total) in &*totals {
        record_component(component, total);
      }
      totals.clear();
    });
  }
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

fn record_component(component: &'static str, total: &ComponentTotals) {
  tracing::event!(
    name: "reactant.component.render",
    tracing::Level::INFO,
    component,
    calls = total.calls,
    duration_us = total.duration_us,
    self_duration_us = total.self_duration_us,
    maximum_self_duration_us = total.maximum_self_duration_us,
    "Reactant component render totals."
  );
}

#[cfg(test)]
mod tests {
  use crate::performance::{self, CHILD_DURATION_US, ComponentTotals};

  #[test]
  fn repeated_components_preserve_hotspot_statistics_without_per_instance_logs() {
    let mut totals = ComponentTotals::default();
    for _ in 0..5000 {
      totals.add(20, 10);
    }
    totals.add(50, 30);
    assert_eq!(totals.calls, 5001);
    assert_eq!(totals.duration_us, 100050);
    assert_eq!(totals.self_duration_us, 50030);
    assert_eq!(totals.maximum_self_duration_us, 30);
  }

  #[test]
  fn parent_exclusion_can_cover_the_child_instrumentation_span() {
    CHILD_DURATION_US.with_borrow_mut(|stack| {
      stack.clear();
      stack.push(0);
      stack.push(0);
    });
    assert_eq!(performance::pop_span(10), (10, 1));
    performance::complete_instrumentation(15);
    assert_eq!(performance::pop_span(30), (15, 0));
    performance::complete_instrumentation(35);
    CHILD_DURATION_US.with_borrow(|stack| assert!(stack.is_empty()));
  }
}
