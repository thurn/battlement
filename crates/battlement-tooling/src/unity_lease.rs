use std::{
  fs::{self, File, OpenOptions},
  path::{Path, PathBuf},
  thread,
  time::Duration,
};

use anyhow::{Context, Result};
use fs2::FileExt;

const UNITY_EDITOR_SLOTS: usize = 2;

/// One machine-wide Unity Editor capacity slot shared with legacy Python CI.
#[derive(Debug)]
pub struct UnityEditorLease {
  file: File,
  path: PathBuf,
  slot: usize,
}

impl UnityEditorLease {
  /// Tries both shared slots without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    fs::create_dir_all(directory)
      .with_context(|| format!("create resource slot directory {}", directory.display()))?;
    for slot in 0..UNITY_EDITOR_SLOTS {
      let path = directory.join(format!("unity-editor-{slot}.lock"));
      let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)?;
      if file.metadata()?.len() == 0 {
        file.set_len(1)?;
      }
      if file.try_lock_exclusive().is_ok() {
        return Ok(Some(Self { file, path, slot }));
      }
    }
    Ok(None)
  }

  /// Waits until one shared slot can be acquired.
  pub fn acquire(directory: &Path) -> Result<Self> {
    loop {
      if let Some(lease) = Self::try_acquire(directory)? {
        return Ok(lease);
      }
      thread::sleep(Duration::from_millis(100));
    }
  }

  /// Returns the stable zero-based slot number.
  pub fn slot(&self) -> usize {
    self.slot
  }

  /// Returns the exact legacy-compatible lock path.
  pub fn path(&self) -> &Path {
    &self.path
  }
}

impl Drop for UnityEditorLease {
  fn drop(&mut self) {
    let _ = FileExt::unlock(&self.file);
  }
}
