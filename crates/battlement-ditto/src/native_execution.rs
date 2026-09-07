//! Machine-wide ownership for native Ditto players.

use std::{
  fs::{File, OpenOptions},
  io::{Read, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use anyhow::{Context, Result, bail};
use battlement_tooling::{discovery::machine_resource_slots, host::SystemHost};
use fs2::FileExt;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct Owner<'a> {
  schema: u8,
  lease_id: &'a str,
  pid: u32,
}

/// Exclusive process lease held for the complete lifetime of a native execution.
#[derive(Debug)]
pub struct NativeExecutionLease {
  file: File,
  id: String,
  path: PathBuf,
  capture_active: AtomicBool,
}

impl NativeExecutionLease {
  /// Acquires the one host-global native Ditto execution slot without waiting.
  pub fn acquire() -> Result<Self> {
    Self::acquire_at(&machine_resource_slots(&SystemHost).join("ditto-native-player-0.lock"))
  }

  fn acquire_at(path: &Path) -> Result<Self> {
    let parent = path
      .parent()
      .context("native execution lease has no parent")?;
    std::fs::create_dir_all(parent).with_context(|| {
      format!(
        "create native execution lease directory {}",
        parent.display()
      )
    })?;
    let mut file = OpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .open(path)
      .with_context(|| format!("open native execution lease {}", path.display()))?;
    if file.try_lock_exclusive().is_err() {
      let mut owner = String::new();
      let _ = file.read_to_string(&mut owner);
      let owner = owner.trim();
      bail!(
        "native execution is already owned{}; Ditto refused to launch a competing player",
        if owner.is_empty() {
          String::new()
        } else {
          format!(" by {owner}")
        }
      );
    }

    let id = Uuid::new_v4().to_string();
    let encoded = serde_json::to_vec(&Owner {
      schema: 1,
      lease_id: &id,
      pid: std::process::id(),
    })?;
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(Self {
      file,
      id,
      path: path.to_path_buf(),
      capture_active: AtomicBool::new(false),
    })
  }

  /// Returns the opaque identity passed directly to the owned player process.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// Returns the host-global lock path used by this lease.
  pub fn path(&self) -> &Path {
    &self.path
  }

  pub(crate) fn claim_capture(self: &Arc<Self>) -> Result<NativeExecutionClaim> {
    if self
      .capture_active
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_err()
    {
      bail!("native execution lease is already launching or supervising a player");
    }
    Ok(NativeExecutionClaim(self.clone()))
  }
}

pub(crate) struct NativeExecutionClaim(Arc<NativeExecutionLease>);

impl Drop for NativeExecutionClaim {
  fn drop(&mut self) {
    self.0.capture_active.store(false, Ordering::Release);
  }
}

impl Drop for NativeExecutionLease {
  fn drop(&mut self) {
    let _ = self.file.unlock();
  }
}

#[cfg(test)]
mod tests {
  use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
  };

  use super::NativeExecutionLease;

  const CHILD_PATH: &str = "DITTO_NATIVE_LEASE_CHILD_PATH";
  const CHILD_READY: &str = "DITTO_NATIVE_LEASE_CHILD_READY";
  const CHILD_RELEASE: &str = "DITTO_NATIVE_LEASE_CHILD_RELEASE";

  #[test]
  fn native_execution_child() {
    let Some(path) = std::env::var_os(CHILD_PATH) else {
      return;
    };
    let ready = std::path::PathBuf::from(std::env::var_os(CHILD_READY).unwrap());
    let release = std::path::PathBuf::from(std::env::var_os(CHILD_RELEASE).unwrap());
    let _lease = NativeExecutionLease::acquire_at(path.as_ref()).unwrap();
    fs::write(&ready, b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !release.exists() && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(10));
    }
    assert!(release.exists(), "parent did not release child");
  }

  #[test]
  fn independent_process_is_rejected_before_player_launch() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("native.lock");
    let ready = temporary.path().join("ready");
    let release = temporary.path().join("release");
    let mut child = Command::new(std::env::current_exe().unwrap())
      .args([
        "--exact",
        "native_execution::tests::native_execution_child",
        "--nocapture",
      ])
      .env(CHILD_PATH, &path)
      .env(CHILD_READY, &ready)
      .env(CHILD_RELEASE, &release)
      .spawn()
      .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child did not acquire native lease");

    let failure = NativeExecutionLease::acquire_at(&path).unwrap_err();
    assert!(failure.to_string().contains("refused to launch"));

    fs::write(release, b"release").unwrap();
    assert!(child.wait().unwrap().success());
  }

  #[test]
  fn competing_native_execution_is_rejected_before_launch() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("native.lock");
    let owner = NativeExecutionLease::acquire_at(&path).unwrap();
    let failure = NativeExecutionLease::acquire_at(&path).unwrap_err();

    assert!(failure.to_string().contains(owner.id()));
    assert!(failure.to_string().contains("refused to launch"));
  }

  #[test]
  fn released_native_execution_can_be_reacquired() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("native.lock");
    let first = NativeExecutionLease::acquire_at(&path).unwrap();
    let first_id = first.id().to_owned();
    drop(first);

    let second = NativeExecutionLease::acquire_at(&path).unwrap();
    assert_ne!(second.id(), first_id);
  }
}
