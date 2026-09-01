use std::{
  fs::{self, File, OpenOptions},
  io::{BufRead, BufReader, Write},
  path::{Component, Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result, ensure};
use fs2::{FileExt, lock_contended_error};
use serde::{Serialize, de::DeserializeOwned};

use crate::build_cache::CacheJournalEntry;

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn open_lock(path: &Path) -> Result<File> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let file = OpenOptions::new()
    .create(true)
    .read(true)
    .write(true)
    .truncate(false)
    .open(path)
    .with_context(|| format!("open cache lock {}", path.display()))?;
  if file.metadata()?.len() == 0 {
    file.set_len(1)?;
  }
  Ok(file)
}

pub(super) fn lock_exclusive(path: &Path) -> Result<File> {
  let file = self::open_lock(path)?;
  file.lock_exclusive()?;
  Ok(file)
}

pub(super) fn lock_shared(path: &Path) -> Result<File> {
  let file = self::open_lock(path)?;
  FileExt::lock_shared(&file)?;
  Ok(file)
}

pub(super) fn try_lock_exclusive(path: &Path) -> Result<Option<File>> {
  let file = self::open_lock(path)?;
  match file.try_lock_exclusive() {
    Ok(()) => Ok(Some(file)),
    Err(error) if error.raw_os_error() == lock_contended_error().raw_os_error() => Ok(None),
    Err(error) => Err(error.into()),
  }
}

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  self::write_atomic(path, &bytes)
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
  serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
    .with_context(|| format!("parse {}", path.display()))
}

pub(super) fn write_access(path: &Path, now_unix_s: u64) -> Result<()> {
  self::write_atomic(path, format!("{now_unix_s}\n").as_bytes())
}

pub(super) fn read_access(path: &Path) -> Result<u64> {
  fs::read_to_string(path)?
    .trim()
    .parse()
    .with_context(|| format!("parse cache access time {}", path.display()))
}

pub(super) fn append_journal(path: &Path, event: &CacheJournalEntry) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .read(true)
    .open(path)?;
  file.lock_exclusive()?;
  let mut bytes = serde_json::to_vec(event)?;
  bytes.push(b'\n');
  file.write_all(&bytes)?;
  file.sync_all()?;
  FileExt::unlock(&file)?;
  Ok(())
}

pub(super) fn read_journal(path: &Path) -> Result<Vec<CacheJournalEntry>> {
  if !path.exists() {
    return Ok(Vec::new());
  }
  let file = File::open(path)?;
  FileExt::lock_shared(&file)?;
  let entries = BufReader::new(&file)
    .lines()
    .map(|line| Ok(serde_json::from_str(&line?)?))
    .collect();
  FileExt::unlock(&file)?;
  entries
}

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  let parent = path
    .parent()
    .ok_or_else(|| anyhow::anyhow!("atomic cache path has no parent"))?;
  fs::create_dir_all(parent)?;
  let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
  let temporary = parent.join(format!(
    ".{}.{}.{sequence}.tmp",
    path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("atomic cache path has no name"))?
      .to_string_lossy(),
    std::process::id(),
  ));
  let mut file = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&temporary)?;
  let result = (|| {
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    self::sync_directory(parent)
  })();
  if result.is_err() && temporary.is_file() {
    let _ = fs::remove_file(temporary);
  }
  result
}

pub(super) fn sync_tree(directory: &Path) -> Result<()> {
  self::sync_tree_at(directory, directory)
}

fn sync_tree_at(root: &Path, directory: &Path) -> Result<()> {
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      self::sync_tree_at(root, &entry.path())?;
    } else if file_type.is_symlink() {
      ensure!(
        entry.path().canonicalize()?.starts_with(root),
        "build artifact symlink escapes staging"
      );
    } else {
      ensure!(file_type.is_file(), "build cache contains a special file");
      OpenOptions::new()
        .write(true)
        .open(entry.path())?
        .sync_all()?;
    }
  }
  self::sync_directory(directory)
}

#[cfg(not(windows))]
pub(super) fn sync_directory(path: &Path) -> Result<()> {
  File::open(path)?
    .sync_all()
    .with_context(|| format!("sync cache directory {}", path.display()))
}

#[cfg(windows)]
pub(super) fn sync_directory(_path: &Path) -> Result<()> {
  Ok(())
}

pub(super) fn directory_bytes(directory: &Path) -> Result<u64> {
  let mut total = 0_u64;
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      total = total.saturating_add(self::directory_bytes(&entry.path())?);
    } else {
      total = total.saturating_add(fs::symlink_metadata(entry.path())?.len());
    }
  }
  Ok(total)
}

pub(super) fn relative_file(root: &Path, relative: &Path) -> Result<PathBuf> {
  let path = self::relative_artifact(root, relative)?;
  ensure!(
    fs::symlink_metadata(&path)?.file_type().is_file(),
    "cache artifact is not a regular file"
  );
  Ok(path)
}

pub(super) fn relative_artifact(root: &Path, relative: &Path) -> Result<PathBuf> {
  ensure!(!relative.is_absolute(), "cache artifact path is absolute");
  ensure!(
    !relative.as_os_str().is_empty(),
    "cache artifact path is empty"
  );
  ensure!(
    relative
      .components()
      .all(|component| matches!(component, Component::Normal(_))),
    "cache artifact path is not normalized"
  );
  let path = root.join(relative);
  let file_type = fs::symlink_metadata(&path)
    .with_context(|| format!("inspect cache artifact {}", path.display()))?
    .file_type();
  ensure!(
    file_type.is_file() || file_type.is_dir(),
    "cache artifact is not a regular file or directory"
  );
  Ok(path)
}

pub(super) fn next_failure_path(parent: &Path, failed_at_unix_s: u64) -> PathBuf {
  for sequence in 0_u64.. {
    let path = parent.join(format!("{failed_at_unix_s}-{sequence}"));
    if !path.exists() {
      return path;
    }
  }
  unreachable!("failure sequence is unbounded")
}
