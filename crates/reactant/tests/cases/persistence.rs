use std::{cell::RefCell, collections::VecDeque, path::Path, rc::Rc};

use reactant::{
  FilePersistenceBackend, PersistenceBackend, PersistenceCompletion, PersistenceOperation,
  PersistenceRequest, PersistenceStore,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[derive(Default)]
struct DelayedStorage {
  operations: RefCell<VecDeque<(PersistenceRequest, PersistenceCompletion)>>,
}

impl PersistenceBackend for DelayedStorage {
  fn load(&self, _: &Path) -> Result<Option<Vec<u8>>, String> {
    panic!("async backend")
  }
  fn store(&self, _: &Path, _: &[u8]) -> Result<(), String> {
    panic!("async backend")
  }
  fn remove(&self, _: &Path) -> Result<(), String> {
    panic!("async backend")
  }
  fn start(&self, request: PersistenceRequest, complete: PersistenceCompletion) {
    self.operations.borrow_mut().push_back((request, complete));
  }
}

impl DelayedStorage {
  fn next(&self) -> (PersistenceRequest, PersistenceCompletion) {
    self.operations.borrow_mut().pop_front().expect("request")
  }
}

#[test]
fn hydration_gates_writes_and_newest_intent_follows_the_active_write() {
  let backend = Rc::new(DelayedStorage::default());
  let store = PersistenceStore::<u32>::new(Some("state.json".into()), backend.clone());
  let (read, complete_read) = backend.next();
  store.update(1);
  store.update(2);
  assert!(backend.operations.borrow().is_empty());
  assert!(!store.snapshot().hydrated);
  complete_read(read.id, Ok(Some(b"0".to_vec())));
  let (write, complete_write) = backend.next();
  assert_eq!(write.operation, PersistenceOperation::Store(b"2".to_vec()));
  assert_eq!(store.snapshot().durable, Some(0));
  store.update(3);
  store.update(4);
  complete_write(write.id, Ok(None));
  let (latest, complete_latest) = backend.next();
  assert_eq!(latest.operation, PersistenceOperation::Store(b"4".to_vec()));
  assert_eq!(store.snapshot().durable, Some(2));
  complete_read(read.id, Ok(Some(b"88".to_vec())));
  complete_write(write.id, Err("late duplicate".into()));
  assert_eq!(store.snapshot().desired, Some(4));
  assert_eq!(store.snapshot().pending, Some(latest.id));
  assert_eq!(store.snapshot().error, None);
  complete_latest(latest.id, Ok(None));
  assert_eq!(store.snapshot().durable, Some(4));
  assert_eq!(store.snapshot().pending, None);
}

#[test]
fn deletion_orders_after_write_and_failed_delete_retries_latest_intent() {
  let backend = Rc::new(DelayedStorage::default());
  let store = PersistenceStore::<u32>::new(Some("state.json".into()), backend.clone());
  let (read, done) = backend.next();
  done(read.id, Ok(Some(b"1".to_vec())));
  store.update(2);
  let (write, done_write) = backend.next();
  store.update(3);
  store.clear();
  done_write(write.id, Ok(None));
  let (delete, done_delete) = backend.next();
  assert_eq!(delete.operation, PersistenceOperation::Remove);
  done_delete(delete.id, Err("denied".into()));
  assert_eq!(store.snapshot().durable, Some(2));
  assert_eq!(store.snapshot().desired, None);
  store.update(4);
  let (new, done_new) = backend.next();
  done_new(new.id, Err("quota".into()));
  done_delete(delete.id, Ok(None));
  assert_eq!(store.snapshot().error.as_deref(), Some("quota"));
  assert_eq!(store.snapshot().durable, Some(2));
  store.retry();
  let (retry, done_retry) = backend.next();
  assert_eq!(retry.operation, PersistenceOperation::Store(b"4".to_vec()));
  done_retry(retry.id, Ok(None));
  assert_eq!(store.snapshot().durable, Some(4));
  assert_eq!(store.snapshot().error, None);
}

#[test]
fn failed_hydration_never_overwrites_unread_data_and_corruption_is_visible() {
  let backend = Rc::new(DelayedStorage::default());
  let store = PersistenceStore::<u32>::new(Some("state.json".into()), backend.clone());
  let (read, done) = backend.next();
  store.update(7);
  done(read.id, Err("not yet mounted".into()));
  assert!(backend.operations.borrow().is_empty());
  assert!(!store.snapshot().hydrated);
  store.retry();
  let (retry, done_retry) = backend.next();
  assert_eq!(retry.operation, PersistenceOperation::Load);
  done_retry(retry.id, Ok(Some(b"broken json".to_vec())));
  assert!(store.snapshot().hydrated);
  assert!(store.snapshot().error.is_some());
  let (write, done_write) = backend.next();
  assert_eq!(write.operation, PersistenceOperation::Store(b"7".to_vec()));
  done_write(write.id, Ok(None));
  assert_eq!(store.snapshot().durable, Some(7));
}

#[test]
fn unavailable_storage_keeps_session_intent_without_claiming_durability() {
  let store = PersistenceStore::<u32>::new(None, Rc::new(FilePersistenceBackend));
  store.update(9);
  assert_eq!(store.snapshot().desired, Some(9));
  assert_eq!(store.snapshot().durable, None);
  assert!(store.snapshot().hydrated);
  assert!(store.snapshot().error.is_some());
}

#[test]
fn native_replacement_survives_reopen_and_missing_delete_succeeds() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join("value.json");
  let backend = Rc::new(FilePersistenceBackend);
  let first = PersistenceStore::<u32>::new(Some(path.clone()), backend.clone());
  first.update(17);
  assert_eq!(first.snapshot().durable, Some(17));
  first.update(29);
  drop(first);
  let reopened = PersistenceStore::<u32>::new(Some(path), backend);
  assert_eq!(reopened.snapshot().durable, Some(29));
  reopened.clear();
  reopened.clear();
  assert_eq!(reopened.snapshot().durable, None);
  assert_eq!(reopened.snapshot().error, None);
}

#[cfg(unix)]
#[test]
fn denied_replacement_preserves_the_previous_complete_file() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join("value.json");
  let backend = FilePersistenceBackend;
  backend.store(&path, b"previous complete bytes").unwrap();
  std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
  let outcome = backend.store(&path, b"replacement");
  std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
  assert!(outcome.is_err());
  assert_eq!(
    backend.load(&path).unwrap(),
    Some(b"previous complete bytes".to_vec())
  );
}
