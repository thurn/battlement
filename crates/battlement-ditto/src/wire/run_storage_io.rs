use std::{
  fs::{self, File, OpenOptions},
  io::Write,
  path::Path,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::wire::result_format;

pub(super) const LEASE_FILE: &str = ".lease.json";
pub(super) const PENDING_FILE: &str = ".terminal-pending";
pub(super) const PARTIAL_FILE: &str = "partial-result.json";
pub(super) const RESULT_FILE: &str = "result.json";
const LEASE_SECONDS: u64 = 60;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunLease {
  owner: String,
  expires_unix_s: u64,
}

pub(super) fn scan_artifacts(directory: &Path) -> Result<Vec<String>> {
  let mut artifacts = Vec::new();
  collect_artifacts(directory, directory, &mut artifacts)?;
  artifacts.sort();
  Ok(artifacts)
}

pub(super) fn directory_bytes(directory: &Path) -> Result<u64> {
  let mut total = 0_u64;
  for entry in fs::read_dir(directory).context("read run directory")? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      total = total.saturating_add(directory_bytes(&entry.path())?);
    } else if file_type.is_file() {
      total = total.saturating_add(entry.metadata()?.len());
    }
  }
  Ok(total)
}

pub(super) fn lease_active(directory: &Path, now_unix_s: u64) -> Result<bool> {
  Ok(read_lease(directory)?.is_some_and(|lease| lease.expires_unix_s > now_unix_s))
}

pub(super) fn lease_owner(directory: &Path) -> Result<Option<String>> {
  Ok(read_lease(directory)?.map(|lease| lease.owner))
}

pub(super) fn write_lease(directory: &Path, owner: &str, now_unix_s: u64) -> Result<()> {
  write_atomic(
    &directory.join(LEASE_FILE),
    &result_format::canonical_pretty_json(&RunLease {
      owner: owner.to_owned(),
      expires_unix_s: now_unix_s.saturating_add(LEASE_SECONDS),
    })?,
  )
}

pub(super) fn remove_if_file(path: &Path) -> Result<()> {
  if path.is_file() {
    fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
  }
  Ok(())
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  let parent = path
    .parent()
    .ok_or_else(|| anyhow::anyhow!("atomic path has no parent"))?;
  let file_name = path
    .file_name()
    .ok_or_else(|| anyhow::anyhow!("atomic path has no file name"))?;
  let temporary = parent.join(format!(
    ".{}.{}.tmp",
    file_name.to_string_lossy(),
    Uuid::new_v4()
  ));
  let mut file = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&temporary)?;
  let write = (|| {
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path).with_context(|| format!("commit {}", path.display()))?;
    sync_directory(parent)
  })();
  if write.is_err() && temporary.is_file() {
    let _ = fs::remove_file(&temporary);
  }
  write
}

pub(super) fn sync_directory(path: &Path) -> Result<()> {
  File::open(path)?
    .sync_all()
    .with_context(|| format!("sync directory {}", path.display()))
}

pub(super) fn materialize_paths(source: &Path, destination: &Path, paths: &[String]) -> Result<()> {
  for relative in paths {
    result_format::artifact_path("derived artifact", relative)?;
    let source_path = source.join(relative);
    let source_type = fs::symlink_metadata(&source_path)
      .with_context(|| format!("inspect source artifact {relative}"))?
      .file_type();
    ensure!(
      source_type.is_file(),
      "source artifact is not a regular file"
    );
    let destination_path = destination.join(relative);
    if let Some(parent) = destination_path.parent() {
      create_safe_directories(destination, parent)?;
    }
    if fs::hard_link(&source_path, &destination_path).is_err() {
      fs::copy(&source_path, &destination_path)?;
    }
  }
  sync_directory(destination)
}

fn read_lease(directory: &Path) -> Result<Option<RunLease>> {
  let path = directory.join(LEASE_FILE);
  if !path.exists() {
    return Ok(None);
  }
  Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

fn collect_artifacts(root: &Path, directory: &Path, artifacts: &mut Vec<String>) -> Result<()> {
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    let path = entry.path();
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      collect_artifacts(root, &path, artifacts)?;
    } else if file_type.is_file() && !internal_file(root, &path) {
      artifacts.push(
        path
          .strip_prefix(root)?
          .to_string_lossy()
          .replace('\\', "/"),
      );
    }
  }
  Ok(())
}

fn internal_file(root: &Path, path: &Path) -> bool {
  let relative = path
    .strip_prefix(root)
    .expect("artifact path belongs to run");
  if relative == Path::new(RESULT_FILE) || relative == Path::new(PARTIAL_FILE) {
    return true;
  }
  relative
    .components()
    .next()
    .is_some_and(|part| part.as_os_str().to_string_lossy().starts_with('.'))
}

fn create_safe_directories(root: &Path, directory: &Path) -> Result<()> {
  let relative = directory.strip_prefix(root)?;
  let mut current = root.to_owned();
  for component in relative.components() {
    current.push(component);
    if current.exists() {
      ensure!(
        fs::symlink_metadata(&current)?.file_type().is_dir(),
        "derived artifact parent is not a directory"
      );
    } else {
      fs::create_dir(&current)?;
    }
  }
  Ok(())
}
