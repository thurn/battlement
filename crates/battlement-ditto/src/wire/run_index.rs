use std::{
  collections::{BTreeMap, BTreeSet},
  fs::{self, File, OpenOptions},
  io,
  path::Path,
};

use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::wire::{result_format, run_storage::RunIndexEntry, run_storage_io, validation};

const INDEX_FILE: &str = "index.json";
const LOCK_FILE: &str = ".index.lock";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunIndex {
  pub(super) entries: Vec<RunIndexEntry>,
}

pub(super) fn read(root: &Path) -> Result<RunIndex> {
  let path = root.join(INDEX_FILE);
  let bytes = match fs::read(&path) {
    Ok(bytes) => bytes,
    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(RunIndex::default()),
    Err(error) => return Err(error).with_context(|| format!("read run index {}", path.display())),
  };
  let index: RunIndex = serde_json::from_slice(&bytes)
    .with_context(|| format!("parse run index {}", path.display()))?;
  let mut ids = BTreeSet::new();
  for entry in &index.entries {
    validation::identifier("run index ID", &entry.run_id)?;
    ensure!(
      ids.insert(&entry.run_id),
      "run index IDs must be unique: {}",
      path.display()
    );
  }
  Ok(index)
}

pub(super) fn lock(root: &Path) -> Result<File> {
  let lock = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open(root.join(LOCK_FILE))
    .context("open run index lock")?;
  lock.lock_exclusive().context("lock run index")?;
  Ok(lock)
}

/// Requires the caller to hold the catalog lock.
pub(super) fn write(root: &Path, index: &RunIndex) -> Result<()> {
  run_storage_io::write_atomic(
    &root.join(INDEX_FILE),
    &result_format::canonical_pretty_json(index)?,
  )
}

/// Applies local changes without replacing fields written by other owners.
pub(super) fn merge(root: &Path, baseline: &RunIndex, changed: &RunIndex) -> Result<RunIndex> {
  let _lock = lock(root)?;
  let mut latest = read(root)?;
  let original: BTreeMap<_, _> = baseline
    .entries
    .iter()
    .map(|entry| (entry.run_id.as_str(), entry))
    .collect();
  let positions: BTreeMap<_, _> = latest
    .entries
    .iter()
    .enumerate()
    .map(|(position, entry)| (entry.run_id.clone(), position))
    .collect();
  let mut dirty = false;
  for entry in &changed.entries {
    let previous = original.get(entry.run_id.as_str()).copied();
    if previous == Some(entry) {
      continue;
    }
    if let Some(position) = positions.get(&entry.run_id) {
      merge_entry(&mut latest.entries[*position], previous, entry);
    } else {
      latest.entries.push(entry.clone());
    }
    dirty = true;
  }
  if dirty {
    write(root, &latest)?;
  }
  Ok(latest)
}

fn merge_entry(
  latest: &mut RunIndexEntry,
  previous: Option<&RunIndexEntry>,
  changed: &RunIndexEntry,
) {
  if changed.repository.is_some()
    && previous.map(|entry| &entry.repository) != Some(&changed.repository)
  {
    latest.repository.clone_from(&changed.repository);
  }
  if changed.suite.is_some() && previous.map(|entry| &entry.suite) != Some(&changed.suite) {
    latest.suite.clone_from(&changed.suite);
  }
  if changed.terminal_status.is_some()
    && previous.map(|entry| entry.terminal_status) != Some(changed.terminal_status)
  {
    latest.terminal_status = changed.terminal_status;
  }
  latest.last_accessed_unix_s = latest
    .last_accessed_unix_s
    .max(changed.last_accessed_unix_s);
  latest.artifacts_evicted |= changed.artifacts_evicted;
  if latest.artifacts_evicted {
    latest.artifact_bytes = 0;
  } else if previous.map(|entry| entry.artifact_bytes) != Some(changed.artifact_bytes) {
    latest.artifact_bytes = changed.artifact_bytes;
  }
}
