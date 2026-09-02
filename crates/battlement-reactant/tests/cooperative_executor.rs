use std::{
  future,
  sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
  },
  task::Poll,
  thread,
};

use battlement_reactant::{cooperative_executor::CooperativeExecutor, executor::Spawner};

#[test]
fn self_waking_work_is_bounded_and_cancellation_prevents_further_polls() {
  let executor = CooperativeExecutor::default();
  let polls = Arc::new(AtomicUsize::new(0));
  let count = Arc::clone(&polls);
  let task = executor.spawn(Box::pin(future::poll_fn(move |context| {
    count.fetch_add(1, Ordering::Relaxed);
    context.waker().wake_by_ref();
    Poll::Pending
  })));
  executor.tick();
  assert_eq!(polls.load(Ordering::Relaxed), 1);
  executor.tick();
  assert_eq!(polls.load(Ordering::Relaxed), 2);
  task.cancel();
  executor.tick();
  assert_eq!(polls.load(Ordering::Relaxed), 2);
}

#[test]
fn concurrent_servicing_cannot_consume_a_wake_before_the_future_is_restored() {
  let executor = CooperativeExecutor::default();
  let barrier = Arc::new(Barrier::new(2));
  let worker_barrier = Arc::clone(&barrier);
  let polls = Arc::new(AtomicUsize::new(0));
  let count = Arc::clone(&polls);
  let _task = executor.spawn(Box::pin(future::poll_fn(move |context| {
    if count.fetch_add(1, Ordering::Relaxed) == 0 {
      context.waker().wake_by_ref();
      worker_barrier.wait();
      worker_barrier.wait();
      Poll::Pending
    } else {
      Poll::Ready(())
    }
  })));
  let worker = executor.clone();
  let thread = thread::spawn(move || worker.tick());
  barrier.wait();
  executor.tick();
  barrier.wait();
  thread.join().unwrap();
  executor.tick();
  assert_eq!(polls.load(Ordering::Relaxed), 2);
}
