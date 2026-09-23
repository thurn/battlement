//! Optional full-scenario latency measurement; unsuitable for shared-load CI gates.
use std::time::{Duration, Instant};

/// Measures fresh scenarios including setup, assertions, and destruction.
/// Warm-up touches code and immutable assets only; each invocation owns its state.
#[allow(clippy::assertions_on_constants)]
pub fn assert_latency(scenarios: &[(&str, fn())], limit: Duration) {
  // Compile in debug CI, but reject an accidental debug benchmark invocation.
  assert!(!cfg!(debug_assertions), "measure optimized code");
  let mut missed = Vec::new();
  for (name, scenario) in scenarios {
    for _ in 0..20 {
      scenario();
    }
    let mut samples = (0..1_000)
      .map(|_| {
        let start = Instant::now();
        scenario();
        start.elapsed()
      })
      .collect::<Vec<_>>();
    samples.sort_unstable();
    eprintln!(
      "{name}: median={:?} p95={:?} max={:?}",
      samples[500], samples[950], samples[999]
    );
    if samples[950] >= limit {
      missed.push(*name);
    }
  }
  assert!(
    missed.is_empty(),
    "latency target {limit:?} missed: {missed:?}"
  );
}
