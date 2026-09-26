use crate::persistence_file::Boundary;
use crate::{FilePersistenceBackend, PersistenceBackend, persistence_file};
use std::{
  cell::Cell,
  env,
  fs::{self, File},
  io::{self, BufRead, BufReader, Write},
  path::Path,
  process::{Command, Stdio},
  sync::mpsc,
  thread,
  time::Duration,
};
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

#[test]
fn faults_at_write_sync_and_replace_boundaries_preserve_previous_bytes() {
  for boundary in [
    Boundary::Write,
    Boundary::FileSync,
    Boundary::Replace,
    Boundary::DirectorySync,
  ] {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("value.json");
    FilePersistenceBackend.store(&path, b"previous").unwrap();
    let result = persistence_file::replace_checked(&path, Some(b"new"), &File::sync_all, &|at| {
      if at == boundary {
        Err(io::Error::other(format!("injected {at:?}")))
      } else {
        Ok(())
      }
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), b"previous");
  }
}

#[test]
fn abrupt_termination_before_or_after_commit_leaves_a_complete_record() {
  for (boundary, expected) in [("Replace", &b"previous"[..]), ("Committed", &b"new"[..])] {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("value.json");
    FilePersistenceBackend.store(&path, b"previous").unwrap();
    let mut child = Command::new(env::current_exe().unwrap())
      .args([
        "--exact",
        "persistence_file::tests::storage_crash_child",
        "--nocapture",
      ])
      .env("BATTLEMENT_STORAGE_CRASH_PATH", &path)
      .env("BATTLEMENT_STORAGE_CRASH_BOUNDARY", boundary)
      .stdout(Stdio::piped())
      .spawn()
      .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let (ready, received) = mpsc::channel();
    thread::spawn(move || {
      let mut line = String::new();
      loop {
        if output.read_line(&mut line).unwrap() == 0 {
          let _ = ready.send(false);
          return;
        }
        if line.trim() == "storage-boundary" {
          let _ = ready.send(true);
          return;
        }
        line.clear();
      }
    });
    let reached = received.recv_timeout(Duration::from_secs(10));
    let _ = child.kill();
    child.wait().unwrap();
    assert_eq!(reached, Ok(true), "child did not reach {boundary}");
    assert_eq!(fs::read(path).unwrap(), expected);
  }
}

#[test]
fn storage_crash_child() {
  let Some(path) = env::var_os("BATTLEMENT_STORAGE_CRASH_PATH") else {
    return;
  };
  let boundary = env::var("BATTLEMENT_STORAGE_CRASH_BOUNDARY").unwrap();
  persistence_file::replace_checked(Path::new(&path), Some(b"new"), &File::sync_all, &|at| {
    if format!("{at:?}") == boundary {
      println!("storage-boundary");
      io::stdout().flush().unwrap();
      loop {
        thread::park();
      }
    }
    Ok(())
  })
  .unwrap();
}
