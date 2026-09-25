use crate::{FilePersistenceBackend, PersistenceBackend, persistence_file};
use std::{cell::Cell, fs, io};
use tempfile::TempDir;

#[test]
fn sync_failure_after_replace_restores_previous_complete_bytes() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join("value.json");
  fs::write(&path, b"previous").unwrap();
  let result = persistence_file::replace(&path, Some(b"new"), &|_| {
    Err(io::Error::other("injected sync failure"))
  });
  assert!(result.is_err());
  assert_eq!(fs::read(&path).unwrap(), b"previous");
}

#[cfg(unix)]
#[test]
fn directory_sync_failure_rolls_back_replace_and_delete() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join("value.json");
  let backend = FilePersistenceBackend;
  backend.store(&path, b"previous").unwrap();
  let calls = Cell::new(0);
  let result = persistence_file::replace(&path, Some(b"new"), &|file| {
    calls.set(calls.get() + 1);
    if calls.get() == 2 {
      Err(io::Error::other("injected directory sync failure"))
    } else {
      file.sync_all()
    }
  });
  assert!(result.is_err());
  assert_eq!(backend.load(&path).unwrap(), Some(b"previous".to_vec()));
  let result = persistence_file::replace(&path, None, &|_| {
    Err(io::Error::other("injected deletion sync failure"))
  });
  assert!(result.is_err());
  assert_eq!(backend.load(&path).unwrap(), Some(b"previous".to_vec()));
}

#[test]
fn failed_initial_write_leaves_no_new_file() {
  let directory = TempDir::new().unwrap();
  let path = directory.path().join("value.json");
  let result = persistence_file::replace(&path, Some(b"new"), &|_| {
    Err(io::Error::other("injected sync failure"))
  });
  assert!(result.is_err());
  assert!(!path.exists());
}
