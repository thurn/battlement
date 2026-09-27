use std::{
  fs,
  sync::mpsc,
  thread,
  time::{Duration, Instant},
};

use battlement_tooling::unity_lease::{CompilerCapacityLease, UnityEditorLease};
use fs2::FileExt;
use tempfile::TempDir;

#[test]
fn rust_admission_respects_the_shared_publication_lock() {
  let temporary = TempDir::new().unwrap();
  let guard = fs::OpenOptions::new()
    .create(true)
    .truncate(false)
    .read(true)
    .write(true)
    .open(temporary.path().join(".machine-heavy.admission.lock"))
    .unwrap();
  guard.set_len(1).unwrap();
  guard.lock_exclusive().unwrap();
  let (started_sender, started_receiver) = mpsc::channel();
  let (entered_sender, entered_receiver) = mpsc::channel();
  let root = temporary.path().to_owned();
  let waiter = thread::spawn(move || {
    started_sender.send(()).unwrap();
    let _lease = CompilerCapacityLease::acquire(&root).unwrap();
    entered_sender.send(()).unwrap();
  });
  started_receiver
    .recv_timeout(Duration::from_secs(2))
    .unwrap();
  let entered_early = entered_receiver.recv_timeout(Duration::from_millis(200));
  drop(guard);
  if entered_early.is_err() {
    entered_receiver
      .recv_timeout(Duration::from_secs(2))
      .unwrap();
  }
  waiter.join().unwrap();
  assert!(
    entered_early.is_err(),
    "admission bypassed the publication lock"
  );
}

#[test]
fn blocking_rust_leases_enter_machine_capacity_in_fifo_order() {
  let temporary = TempDir::new().unwrap();
  let compiler = CompilerCapacityLease::acquire(temporary.path()).unwrap();
  let editor = UnityEditorLease::acquire(temporary.path()).unwrap();
  let (entered_sender, entered_receiver) = mpsc::channel();
  let (first_release_sender, first_release_receiver) = mpsc::channel();
  let first_root = temporary.path().to_owned();
  let first_sender = entered_sender.clone();
  let first = thread::spawn(move || {
    let _lease = CompilerCapacityLease::acquire(&first_root).unwrap();
    first_sender.send("first").unwrap();
    first_release_receiver.recv().unwrap();
  });
  wait_for_ticket_count(temporary.path(), 1);

  let (second_release_sender, second_release_receiver) = mpsc::channel();
  let second_root = temporary.path().to_owned();
  let second = thread::spawn(move || {
    let _lease = CompilerCapacityLease::acquire(&second_root).unwrap();
    entered_sender.send("second").unwrap();
    second_release_receiver.recv().unwrap();
  });
  wait_for_ticket_count(temporary.path(), 2);

  drop(editor);
  assert_eq!(
    entered_receiver
      .recv_timeout(Duration::from_secs(2))
      .unwrap(),
    "first"
  );
  assert!(
    entered_receiver
      .recv_timeout(Duration::from_millis(200))
      .is_err()
  );
  first_release_sender.send(()).unwrap();
  assert_eq!(
    entered_receiver
      .recv_timeout(Duration::from_secs(2))
      .unwrap(),
    "second"
  );
  second_release_sender.send(()).unwrap();
  first.join().unwrap();
  second.join().unwrap();
  drop(compiler);
}

fn wait_for_ticket_count(directory: &std::path::Path, expected: usize) {
  let deadline = Instant::now() + Duration::from_secs(2);
  while Instant::now() < deadline {
    let count = fs::read_dir(directory)
      .unwrap()
      .filter_map(|entry| entry.ok())
      .filter(|entry| {
        entry
          .file_name()
          .to_string_lossy()
          .starts_with(".machine-heavy.queue.")
      })
      .count();
    if count == expected {
      return;
    }
    thread::sleep(Duration::from_millis(10));
  }
  panic!("expected {expected} queued machine-capacity tickets");
}
