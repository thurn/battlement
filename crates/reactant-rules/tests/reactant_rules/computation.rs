use std::{
  future::Future,
  pin::Pin,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  },
  task::{Context, Poll, Wake, Waker},
  time::Duration,
};

use reactant_rules::{
  ComputationError, ComputationLane, ComputationStatus, GameReducer, ReducerContext, ReducerGame,
  ReducerOutput, RulesRun,
};

const TIMEOUT: Duration = Duration::from_secs(5);

struct DropProbe(Arc<AtomicUsize>);
impl Drop for DropProbe {
  fn drop(&mut self) {
    self.0.fetch_add(1, Ordering::SeqCst);
  }
}

#[derive(Default)]
struct Notifications(AtomicUsize);
impl Wake for Notifications {
  fn wake(self: Arc<Self>) {
    self.0.fetch_add(1, Ordering::SeqCst);
  }
}

struct Rules;
impl GameReducer for Rules {
  type State = usize;
  type Action = ();
  type Event = ();
  type Rejection = ();
  fn validate(_: &usize, _: &()) -> Result<(), ()> {
    Ok(())
  }
  fn reduce(&mut self, state: &mut usize, _: (), _: &mut ReducerOutput<Self>) {
    *state += 1;
  }
}

#[test]
fn replacing_work_invalidates_results_immediately_and_runs_only_the_latest_after_cleanup() {
  let lane = ComputationLane::default();
  let (started, entered) = mpsc::channel();
  let (release, held) = mpsc::channel();
  let drops = Arc::new(AtomicUsize::new(0));
  let cleanup = DropProbe(drops.clone());
  let mut old = lane.replace(move |cancel| {
    let _cleanup = cleanup;
    started.send(()).unwrap();
    held.recv_timeout(TIMEOUT).unwrap();
    cancel.checkpoint();
    1
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  let notifications = Arc::new(Notifications::default());
  let waker = Waker::from(notifications.clone());
  let mut context = Context::from_waker(&waker);
  assert!(Pin::new(&mut old).poll(&mut context).is_pending());
  let unused = DropProbe(drops.clone());
  let superseded = lane.replace(move |_| {
    drop(unused);
    panic!("superseded work started")
  });
  let latest = lane.replace(|_| 3);
  assert_eq!(old.status(), ComputationStatus::Cancelled);
  assert!(!old.observation().stopped);
  assert_eq!(
    Pin::new(&mut old).poll(&mut context),
    Poll::Ready(Err(ComputationError::Cancelled))
  );
  assert!(notifications.0.load(Ordering::SeqCst) > 0);
  assert_eq!(superseded.status(), ComputationStatus::Cancelled);
  assert!(superseded.wait_for_stopped(TIMEOUT));
  assert!(!superseded.observation().started);
  assert!(!latest.observation().started);
  assert_eq!(drops.load(Ordering::SeqCst), 1);
  release.send(()).unwrap();
  assert!(old.wait_for_stopped(TIMEOUT));
  assert!(old.observation().cancelled);
  assert_eq!(drops.load(Ordering::SeqCst), 2);
  assert!(old.take_result().is_none());
  assert!(latest.wait_for_stopped(TIMEOUT));
  assert_eq!(latest.take_result(), Some(3));
  assert_eq!(latest.take_result(), None);
}

#[test]
fn returning_after_cancellation_cannot_publish_a_late_result() {
  let lane = ComputationLane::default();
  let (started, entered) = mpsc::channel();
  let (release, held) = mpsc::channel();
  let old = lane.replace(move |cancel| {
    started.send(()).unwrap();
    held.recv_timeout(TIMEOUT).unwrap();
    assert!(cancel.is_cancelled());
    11
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  lane.cancel();
  assert_eq!(old.status(), ComputationStatus::Cancelled);
  release.send(()).unwrap();
  assert!(old.wait_for_stopped(TIMEOUT));
  assert!(old.take_result().is_none());
  let current = lane.replace(|_| 12);
  assert!(current.wait_for_stopped(TIMEOUT));
  assert_eq!(current.take_result(), Some(12));
  lane.cancel();
  assert_eq!(current.status(), ComputationStatus::Cancelled);
}

#[test]
fn owner_drop_is_nonjoining_and_observation_survives_until_cleanup() {
  let lane = ComputationLane::default();
  let (started, entered) = mpsc::channel();
  let (release, held) = mpsc::channel();
  let drops = Arc::new(AtomicUsize::new(0));
  let cleanup = DropProbe(drops.clone());
  let active = lane.replace(move |cancel| {
    let _cleanup = cleanup;
    started.send(()).unwrap();
    held.recv_timeout(TIMEOUT).unwrap();
    cancel.checkpoint();
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  let pending = lane.replace(|_| panic!("unmounted pending work started"));
  drop(lane);
  assert_eq!(active.status(), ComputationStatus::Cancelled);
  assert_eq!(pending.status(), ComputationStatus::Cancelled);
  assert!(!active.observation().stopped);
  assert!(pending.wait_for_stopped(TIMEOUT));
  release.send(()).unwrap();
  assert!(active.wait_for_stopped(TIMEOUT));
  assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn failure_and_success_wake_awaiters_and_allow_replacement() {
  let lane = ComputationLane::<usize>::default();
  let (release, held) = mpsc::channel();
  let (started, entered) = mpsc::channel();
  let mut failed = lane.replace(move |_| {
    started.send(()).unwrap();
    held.recv_timeout(TIMEOUT).unwrap();
    panic!("computation fault")
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  let notifications = Arc::new(Notifications::default());
  let waker = Waker::from(notifications.clone());
  let mut context = Context::from_waker(&waker);
  assert!(Pin::new(&mut failed).poll(&mut context).is_pending());
  release.send(()).unwrap();
  assert!(failed.wait_for_stopped(TIMEOUT));
  assert!(notifications.0.load(Ordering::SeqCst) > 0);
  assert_eq!(
    Pin::new(&mut failed).poll(&mut context),
    Poll::Ready(Err(ComputationError::Failed(
      "computation fault".to_owned()
    )))
  );
  let (release, held) = mpsc::channel();
  let (started, entered) = mpsc::channel();
  let mut succeeded = lane.replace(move |_| {
    started.send(()).unwrap();
    held.recv_timeout(TIMEOUT).unwrap();
    42
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  assert!(Pin::new(&mut succeeded).poll(&mut context).is_pending());
  let before = notifications.0.load(Ordering::SeqCst);
  release.send(()).unwrap();
  assert!(succeeded.wait_for_stopped(TIMEOUT));
  assert!(notifications.0.load(Ordering::SeqCst) > before);
  assert_eq!(
    Pin::new(&mut succeeded).poll(&mut context),
    Poll::Ready(Ok(42))
  );
}

#[test]
fn computation_does_not_occupy_the_rules_slot_and_checks_long_work_at_bounded_steps() {
  let lane = ComputationLane::default();
  let (started, entered) = mpsc::channel();
  let (release, held) = mpsc::channel();
  let steps = Arc::new(AtomicUsize::new(0));
  let progress = steps.clone();
  let search = lane.replace(move |cancel| {
    for assignment in 0..100_000 {
      cancel.checkpoint();
      for card in 0..52 {
        cancel.checkpoint();
        progress.fetch_add(1, Ordering::SeqCst);
        if assignment == 10 && card == 0 {
          started.send(()).unwrap();
          held.recv_timeout(TIMEOUT).unwrap();
        }
      }
    }
  });
  entered.recv_timeout(TIMEOUT).unwrap();
  let rules = RulesRun::<ReducerGame<Rules>>::start(&0, (), |connection| {
    ReducerContext::new(Rules, connection)
  });
  assert!(rules.wait_for_worker_stopped(TIMEOUT));
  assert_eq!(*rules.take_checkpoint().unwrap().state(), 1);
  assert!(!search.observation().stopped);
  lane.cancel();
  let before = steps.load(Ordering::SeqCst);
  release.send(()).unwrap();
  assert!(search.wait_for_stopped(TIMEOUT));
  assert_eq!(steps.load(Ordering::SeqCst), before);
}
