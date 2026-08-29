//! Durable local storage for immutable Ditto runs.

use std::{
  fs::{self, File},
  io::Write,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::wire::{
  common::{ErrorCode, ErrorSource},
  result::{ErrorOccurrence, RunResult, RunStatus},
  result_format, run_storage_io, validation,
};

pub const DEFAULT_RETENTION_BYTES: u64 = 1024 * 1024 * 1024;
pub const RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;
const INDEX_FILE: &str = "index.json";

/// One active, exclusively owned run directory.
#[derive(Debug)]
pub struct ActiveRun {
  run_id: String,
  path: PathBuf,
  owner: String,
  finalized: bool,
}

/// Lightweight retained metadata for one allocated run.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunIndexEntry {
  pub run_id: String,
  pub repository: Option<String>,
  pub suite: Option<String>,
  pub last_accessed_unix_s: u64,
  pub terminal_status: Option<RunStatus>,
  pub artifact_bytes: u64,
  pub artifacts_evicted: bool,
}

/// One abandoned run converted into an authoritative terminal result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveredRun {
  pub run_id: String,
  pub result_path: PathBuf,
  pub status: RunStatus,
}

/// A cleanup decision retained after run artifacts are evicted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvictedRun {
  pub run_id: String,
  pub artifact_bytes: u64,
}

/// Results of one startup recovery and retention pass.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct RunMaintenance {
  pub recovered: Vec<RecoveredRun>,
  pub evicted: Vec<EvictedRun>,
}

/// Repository-wide local run storage rooted in a user cache directory.
#[derive(Debug)]
pub struct RunStore {
  pub(super) root: PathBuf,
  pub(super) index: RunIndex,
}

impl RunStore {
  /// Opens or creates a run store and validates its retained index.
  pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
    let root = root.into();
    fs::create_dir_all(&root).with_context(|| format!("create run store {}", root.display()))?;
    let index_path = root.join(INDEX_FILE);
    let mut index = if index_path.exists() {
      let bytes = fs::read(&index_path).context("read run index")?;
      serde_json::from_slice(&bytes).context("parse run index")?
    } else {
      RunIndex::default()
    };
    validate_index(&index)?;
    let changed = reconcile_index(&root, &mut index)?;
    if changed {
      run_storage_io::write_atomic(&index_path, &result_format::canonical_pretty_json(&index)?)?;
    }
    Ok(Self { root, index })
  }

  /// Allocates a run, its event stream, active lease, and first partial result.
  pub fn begin(
    &mut self,
    mut initial: RunResult,
    stderr: &mut impl Write,
    now_unix_s: u64,
  ) -> Result<ActiveRun> {
    validation::identifier("run_id", &initial.run_id)?;
    ensure!(
      !self
        .index
        .entries
        .iter()
        .any(|entry| entry.run_id == initial.run_id),
      "run ID is already indexed"
    );
    let path = self.run_directory(&initial.run_id)?;
    ensure!(!path.exists(), "run directory already exists");
    fs::create_dir_all(path.join("logs")).context("create run logs directory")?;
    File::create(path.join("logs/events.jsonl")).context("create run event stream")?;
    initial.artifacts = run_storage_io::scan_artifacts(&path)?;
    initial.validate()?;
    let owner = Uuid::new_v4().to_string();
    run_storage_io::write_lease(&path, &owner, now_unix_s)?;
    run_storage_io::write_atomic(
      &path.join(run_storage_io::PARTIAL_FILE),
      &initial.to_canonical_json()?,
    )?;
    self.index.entries.push(RunIndexEntry {
      run_id: initial.run_id.clone(),
      repository: None,
      suite: None,
      last_accessed_unix_s: now_unix_s,
      terminal_status: None,
      artifact_bytes: run_storage_io::directory_bytes(&path)?,
      artifacts_evicted: false,
    });
    self.persist_index()?;
    writeln!(stderr, "DITTO_RUN_DIR={}", path.display()).context("write run directory progress")?;
    Ok(ActiveRun {
      run_id: initial.run_id,
      path,
      owner,
      finalized: false,
    })
  }

  /// Adds the discovered repository and suite identities to the run index.
  pub fn index_identity(
    &mut self,
    active: &ActiveRun,
    repository: &Path,
    suite: &str,
    now_unix_s: u64,
  ) -> Result<()> {
    active.ensure_owned()?;
    validation::name("suite", suite)?;
    let repository = repository
      .canonicalize()
      .with_context(|| format!("canonicalize repository {}", repository.display()))?;
    let entry = self.entry_mut(&active.run_id)?;
    entry.repository = Some(repository.to_string_lossy().into_owned());
    entry.suite = Some(suite.to_owned());
    entry.last_accessed_unix_s = now_unix_s;
    self.persist_index()
  }

  /// Atomically replaces the recoverable partial result.
  pub fn checkpoint(
    &mut self,
    active: &mut ActiveRun,
    mut result: RunResult,
    now_unix_s: u64,
  ) -> Result<()> {
    self.prepare_result(active, &mut result, now_unix_s)?;
    run_storage_io::write_atomic(
      &active.path.join(run_storage_io::PARTIAL_FILE),
      &result.to_canonical_json()?,
    )?;
    self.update_active_size(active, now_unix_s)
  }

  /// Commits the sole authoritative terminal result and releases the run lease.
  pub fn finalize(
    &mut self,
    active: &mut ActiveRun,
    mut result: RunResult,
    now_unix_s: u64,
  ) -> Result<PathBuf> {
    self.prepare_result(active, &mut result, now_unix_s)?;
    let bytes = result.to_canonical_json()?;
    run_storage_io::write_atomic(&active.path.join(run_storage_io::PARTIAL_FILE), &bytes)?;
    run_storage_io::write_atomic(
      &active.path.join(run_storage_io::PENDING_FILE),
      b"terminal\n",
    )?;
    run_storage_io::write_atomic(&active.path.join(run_storage_io::RESULT_FILE), &bytes)?;
    run_storage_io::remove_if_file(&active.path.join(run_storage_io::PARTIAL_FILE))?;
    run_storage_io::remove_if_file(&active.path.join(run_storage_io::PENDING_FILE))?;
    run_storage_io::remove_if_file(&active.path.join(run_storage_io::LEASE_FILE))?;
    run_storage_io::sync_directory(&active.path)?;
    active.finalized = true;
    let bytes = run_storage_io::directory_bytes(&active.path)?;
    let entry = self.entry_mut(&active.run_id)?;
    entry.last_accessed_unix_s = now_unix_s;
    entry.terminal_status = Some(result.status);
    entry.artifact_bytes = bytes;
    self.persist_index()?;
    Ok(active.path.join(run_storage_io::RESULT_FILE))
  }

  /// Refreshes an active run lease for another fixed lease interval.
  pub fn refresh_lease(&self, active: &ActiveRun, now_unix_s: u64) -> Result<()> {
    active.ensure_owned()?;
    run_storage_io::write_lease(&active.path, &active.owner, now_unix_s)
  }

  /// Copies or hard-links immutable artifacts into a comparison-only run.
  pub fn materialize_derived(
    &mut self,
    active: &ActiveRun,
    source_run_id: &str,
    paths: &[String],
    now_unix_s: u64,
  ) -> Result<()> {
    active.ensure_owned()?;
    self.refresh_lease(active, now_unix_s)?;
    ensure!(
      active.run_id != source_run_id,
      "derived run cannot source itself"
    );
    let source = self.run_directory(source_run_id)?;
    ensure!(
      source.join(run_storage_io::RESULT_FILE).is_file(),
      "source run is not terminal"
    );
    ensure!(
      !source.join(run_storage_io::PENDING_FILE).exists(),
      "source run terminal commit is uncertain"
    );
    ensure!(
      !run_storage_io::lease_active(&source, now_unix_s)?,
      "source run is active"
    );
    let owner = Uuid::new_v4().to_string();
    run_storage_io::write_lease(&source, &owner, now_unix_s)?;
    let materialized = run_storage_io::materialize_paths(&source, &active.path, paths);
    let release = run_storage_io::remove_if_file(&source.join(run_storage_io::LEASE_FILE));
    if let Err(error) = materialized {
      release?;
      return Err(error);
    }
    release?;
    self.entry_mut(source_run_id)?.last_accessed_unix_s = now_unix_s;
    self.persist_index()
  }

  /// Loads an authoritative result and updates its LRU access time.
  pub fn load_result(&mut self, run_id: &str, now_unix_s: u64) -> Result<RunResult> {
    let directory = self.run_directory(run_id)?;
    ensure!(
      !directory.join(run_storage_io::PENDING_FILE).exists(),
      "run terminal commit is uncertain"
    );
    let path = directory.join(run_storage_io::RESULT_FILE);
    let result: RunResult = serde_json::from_slice(&fs::read(&path).context("read run result")?)
      .context("parse run result")?;
    result.validate()?;
    self.entry_mut(run_id)?.last_accessed_unix_s = now_unix_s;
    self.persist_index()?;
    Ok(result)
  }

  /// Returns the immutable lightweight index in allocation order.
  pub fn entries(&self) -> &[RunIndexEntry] {
    &self.index.entries
  }

  pub(super) fn run_directory(&self, run_id: &str) -> Result<PathBuf> {
    validation::identifier("run_id", run_id)?;
    Ok(self.root.join(run_id))
  }

  pub(super) fn entry_mut(&mut self, run_id: &str) -> Result<&mut RunIndexEntry> {
    self
      .index
      .entries
      .iter_mut()
      .find(|entry| entry.run_id == run_id)
      .ok_or_else(|| anyhow::anyhow!("run is not indexed"))
  }

  pub(super) fn persist_index(&self) -> Result<()> {
    run_storage_io::write_atomic(
      &self.root.join(INDEX_FILE),
      &result_format::canonical_pretty_json(&self.index)?,
    )
  }

  fn prepare_result(
    &mut self,
    active: &ActiveRun,
    result: &mut RunResult,
    now_unix_s: u64,
  ) -> Result<()> {
    active.ensure_owned()?;
    ensure!(
      result.run_id == active.run_id,
      "result belongs to another run"
    );
    result.artifacts = run_storage_io::scan_artifacts(&active.path)?;
    result.validate()?;
    self.refresh_lease(active, now_unix_s)
  }

  fn update_active_size(&mut self, active: &ActiveRun, now_unix_s: u64) -> Result<()> {
    let bytes = run_storage_io::directory_bytes(&active.path)?;
    let entry = self.entry_mut(&active.run_id)?;
    entry.last_accessed_unix_s = now_unix_s;
    entry.artifact_bytes = bytes;
    self.persist_index()
  }
}

impl ActiveRun {
  pub fn run_id(&self) -> &str {
    &self.run_id
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub(super) fn recovered(run_id: String, path: PathBuf, owner: String) -> Self {
    Self {
      run_id,
      path,
      owner,
      finalized: false,
    }
  }

  fn ensure_owned(&self) -> Result<()> {
    ensure!(!self.finalized, "run is already finalized");
    let owner = run_storage_io::lease_owner(&self.path)?
      .ok_or_else(|| anyhow::anyhow!("run lease is missing"))?;
    ensure!(owner == self.owner, "run lease ownership was lost");
    Ok(())
  }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunIndex {
  pub(super) entries: Vec<RunIndexEntry>,
}

pub(super) fn recover_result(mut result: RunResult, durability_failure: bool) -> Result<RunResult> {
  if durability_failure {
    let next = result.errors.len() + 1;
    ensure!(next <= 9999, "run error occurrence limit exceeded");
    result.errors.push(ErrorOccurrence {
      id: format!("E{next:04}"),
      code: ErrorCode::DurabilityResultCommitFailed,
      source: ErrorSource::Filesystem,
      message: "terminal result commit failed before recovery".to_owned(),
      job_id: None,
      player_session_id: None,
      scenario_id: None,
      step_index: None,
      log_sequence: None,
    });
    if result.status != RunStatus::Interrupted {
      result.status = RunStatus::InfrastructureError;
      result.exit_code = 2;
    }
  } else {
    result.status = RunStatus::Interrupted;
    result.exit_code = 130;
  }
  Ok(result)
}

fn validate_index(index: &RunIndex) -> Result<()> {
  let mut ids = std::collections::BTreeSet::new();
  for entry in &index.entries {
    validation::identifier("run index ID", &entry.run_id)?;
    ensure!(ids.insert(&entry.run_id), "run index IDs must be unique");
  }
  Ok(())
}

fn reconcile_index(root: &Path, index: &mut RunIndex) -> Result<bool> {
  let mut changed = false;
  let mut discovered = Vec::new();
  for entry in fs::read_dir(root).context("scan run store")? {
    let entry = entry?;
    if !entry.file_type()?.is_dir() {
      continue;
    }
    let run_id = entry.file_name().to_string_lossy().into_owned();
    if validation::identifier("run directory ID", &run_id).is_err() {
      continue;
    }
    let directory = entry.path();
    let pending = directory.join(run_storage_io::PENDING_FILE).is_file();
    let authoritative = directory.join(run_storage_io::RESULT_FILE).is_file() && !pending;
    let candidate = if authoritative {
      directory.join(run_storage_io::RESULT_FILE)
    } else {
      directory.join(run_storage_io::PARTIAL_FILE)
    };
    if !candidate.is_file() {
      continue;
    }
    let result: RunResult = serde_json::from_slice(&fs::read(candidate)?)?;
    result.validate()?;
    ensure!(
      result.run_id == run_id,
      "run directory and result ID disagree"
    );
    discovered.push((run_id, directory, result, authoritative));
  }
  discovered.sort_by(|left, right| left.0.cmp(&right.0));
  for (run_id, directory, result, authoritative) in discovered {
    if let Some(existing) = index
      .entries
      .iter_mut()
      .find(|entry| entry.run_id == run_id)
    {
      if authoritative && existing.terminal_status != Some(result.status) {
        existing.terminal_status = Some(result.status);
        changed = true;
      }
      continue;
    }
    index.entries.push(RunIndexEntry {
      run_id,
      repository: None,
      suite: result.suite,
      last_accessed_unix_s: modified_unix_s(&directory),
      terminal_status: authoritative.then_some(result.status),
      artifact_bytes: run_storage_io::directory_bytes(&directory)?,
      artifacts_evicted: false,
    });
    changed = true;
  }
  Ok(changed)
}

fn modified_unix_s(path: &Path) -> u64 {
  path
    .metadata()
    .and_then(|metadata| metadata.modified())
    .ok()
    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
    .map_or(0, |duration| duration.as_secs())
}
