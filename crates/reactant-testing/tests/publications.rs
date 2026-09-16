#[path = "support/publication_game.rs"]
mod publication_game;

use std::sync::{Arc, atomic::Ordering};

use publication_game::{Action, HeldBuilder, Probe, State};

#[test]
fn held_consumer_releases_one_of_32_slots_before_more_builders_run() {
  let probe = Probe::new(HeldBuilder::None);
  let accepted = State {
    values: vec![0],
    probe: Arc::clone(&probe),
  };
  let display = publication_game::start(&accepted, 34);
  display.wait_for_worker_started();
  let held = display.take_checkpoint();
  assert_eq!(held.state().values, [1]);
  display.wait_for_publication(|o| o.waiting_for_capacity && o.published == 33);
  assert_eq!(probe.snapshots.load(Ordering::SeqCst), 34); // private state plus 33 snapshots
  assert_eq!(probe.animations.load(Ordering::SeqCst), 33);
  assert_eq!(display.publication_observation().builders_started, 33);

  let first = display.take_checkpoint();
  assert_eq!(first.state().values, [2]);
  display.wait_for_publication(|o| o.waiting_for_capacity && o.published == 34);
  assert_eq!(probe.snapshots.load(Ordering::SeqCst), 35);
  assert_eq!(probe.animations.load(Ordering::SeqCst), 34);
  // Final output uses the same full FIFO and cannot even clone yet.
  assert_eq!(display.publication_observation().builders_started, 34);
  for value in 3..=34 {
    let checkpoint = display.take_checkpoint();
    assert_eq!(checkpoint.state().values, [value]);
    assert_eq!(checkpoint.animation(), Some(&value));
    assert!(!checkpoint.is_final());
  }
  let final_output = display.take_checkpoint();
  assert_eq!(final_output.state().values, [999]);
  assert!(final_output.is_final());
  assert!(final_output.animation().is_none());
  display.wait_for_worker_stopped();
  assert!(display.worker_observation().completed);
  assert_eq!(display.publication_observation().published, 35);
  assert_eq!(display.publication_observation().consumed, 35);
  assert_eq!(accepted.values, [0]);
  assert_eq!(held.state().values, [1]);
  assert_eq!(first.state().values, [2]);
}

#[test]
fn cancellation_at_full_capacity_wakes_and_cleans_up_without_consumption() {
  let probe = Probe::new(HeldBuilder::None);
  let state = State {
    values: vec![0],
    probe: Arc::clone(&probe),
  };
  let display = publication_game::start(&state, 33);
  display.wait_for_publication(|o| o.waiting_for_capacity);
  display.stop();
  display.wait_for_worker_stopped();
  assert!(display.worker_observation().cancelled);
  assert!(display.try_take_checkpoint().is_none());
  assert_eq!(probe.snapshots.load(Ordering::SeqCst), 33);
  assert_eq!(probe.animations.load(Ordering::SeqCst), 32);
  assert_eq!(probe.context_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn final_publication_waits_for_capacity_and_cancels_before_cloning() {
  let probe = Probe::new(HeldBuilder::None);
  let state = State {
    values: vec![0],
    probe: Arc::clone(&probe),
  };
  let display = publication_game::start(&state, 32);
  display.wait_for_publication(|o| o.waiting_for_capacity);
  assert_eq!(probe.snapshots.load(Ordering::SeqCst), 33);
  display.stop();
  display.wait_for_worker_stopped();
  assert_eq!(probe.snapshots.load(Ordering::SeqCst), 33);
  assert!(display.worker_observation().cancelled);
}

#[test]
fn abandonment_during_each_builder_discards_late_output_after_cleanup() {
  for builder in [HeldBuilder::Snapshot, HeldBuilder::Animation] {
    let probe = Probe::new(builder);
    let state = State {
      values: vec![0],
      probe: Arc::clone(&probe),
    };
    let old = publication_game::start(&state, 1);
    probe.wait_for_builder();
    old.stop();
    assert!(old.publication_observation().abandoned);
    assert!(!old.worker_observation().stopped);
    assert!(old.try_take_checkpoint().is_none());
    probe.release();
    old.wait_for_worker_stopped();
    assert!(old.worker_observation().cancelled);
    assert_eq!(probe.context_drops.load(Ordering::SeqCst), 1);
    assert_eq!(old.publication_observation().published, 0);
    assert!(old.try_take_checkpoint().is_none());
    let replacement = publication_game::start(&state, 0);
    let output = replacement.take_checkpoint();
    assert!(output.is_final());
    assert_eq!(output.state().values, [999]);
    replacement.wait_for_worker_stopped();
  }
}

#[test]
fn failed_rules_discard_pending_output_before_reporting_worker_stopped() {
  let probe = Probe::new(HeldBuilder::None);
  let accepted = State {
    values: vec![0],
    probe: Arc::clone(&probe),
  };
  let display = publication_game::start_action(&accepted, Action::FailAfterPublication);
  display.wait_for_worker_stopped();
  assert_eq!(
    display.worker_observation().failure.as_deref(),
    Some("rules failed after publication")
  );
  assert!(display.publication_observation().abandoned);
  assert_eq!(display.publication_observation().published, 1);
  assert!(display.try_take_checkpoint().is_none());
  assert_eq!(probe.context_drops.load(Ordering::SeqCst), 1);
  assert_eq!(accepted.values, [0]);
}
