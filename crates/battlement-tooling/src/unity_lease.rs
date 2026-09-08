use std::{
  fs::{self, File, OpenOptions},
  path::{Path, PathBuf},
  thread,
  time::Duration,
};

use anyhow::{Context, Result};
use fs2::FileExt;

const UNITY_EDITOR_SLOTS: usize = 2;
const MACHINE_CAPACITY_SLOTS: usize = 6;
const COMPILER_CAPACITY_UNITS: usize = 3;
const UNITY_EDITOR_CAPACITY_UNITS: usize = 3;

#[derive(Debug)]
struct SlotSet {
  files: Vec<(File, PathBuf, usize)>,
}

/// Machine capacity held while one bounded Cargo writer is running.
#[derive(Debug)]
pub struct CompilerCapacityLease {
  _capacity: SlotSet,
}

/// One machine-wide Unity Editor capacity slot shared with legacy Python CI.
#[derive(Debug)]
pub struct UnityEditorLease {
  _capacity: SlotSet,
  editor: SlotSet,
}

impl CompilerCapacityLease {
  /// Waits for the capacity assigned to one three-job Cargo writer.
  pub fn acquire(directory: &Path) -> Result<Self> {
    Ok(Self {
      _capacity: SlotSet::acquire(
        directory,
        "machine-heavy",
        MACHINE_CAPACITY_SLOTS,
        COMPILER_CAPACITY_UNITS,
      )?,
    })
  }
}

impl UnityEditorLease {
  /// Tries both shared slots without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    let Some(capacity) = SlotSet::try_acquire(
      directory,
      "machine-heavy",
      MACHINE_CAPACITY_SLOTS,
      UNITY_EDITOR_CAPACITY_UNITS,
    )?
    else {
      return Ok(None);
    };
    Ok(
      SlotSet::try_acquire(directory, "unity-editor", UNITY_EDITOR_SLOTS, 1)?.map(|editor| Self {
        _capacity: capacity,
        editor,
      }),
    )
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
    self.editor.files[0].2
  }

  /// Returns the exact legacy-compatible lock path.
  pub fn path(&self) -> &Path {
    &self.editor.files[0].1
  }
}

impl SlotSet {
  fn try_acquire(directory: &Path, name: &str, count: usize, units: usize) -> Result<Option<Self>> {
    fs::create_dir_all(directory)
      .with_context(|| format!("create resource slot directory {}", directory.display()))?;
    let mut files = Vec::with_capacity(units);
    for slot in 0..count {
      let path = directory.join(format!("{name}-{slot}.lock"));
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
        files.push((file, path, slot));
        if files.len() == units {
          return Ok(Some(Self { files }));
        }
      }
    }
    Ok(None)
  }

  fn acquire(directory: &Path, name: &str, count: usize, units: usize) -> Result<Self> {
    loop {
      if let Some(lease) = Self::try_acquire(directory, name, count, units)? {
        return Ok(lease);
      }
      thread::sleep(Duration::from_millis(100));
    }
  }
}

impl Drop for SlotSet {
  fn drop(&mut self) {
    for (file, _, _) in &self.files {
      let _ = FileExt::unlock(file);
    }
  }
}
