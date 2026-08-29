//! Debounced watch-state classification and coalescing.

use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
  time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, Result, ensure};

const IGNORED_DIRECTORIES: &[&str] = &[
  ".git",
  ".worktrees",
  "Library",
  "Logs",
  "Temp",
  "obj",
  "target",
];

/// File changes merged into one immutable watch-cycle input.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChangeSet {
  pub scenario: bool,
  pub lock: bool,
  pub source: bool,
  pub retry_broken_build: bool,
}

/// The cheapest correct path for one coalesced watch cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclePath {
  Execution,
  ComparisonOnly,
  ReplacementBuild,
}

/// A single-active-cycle coalescer that retains at most one pending state.
#[derive(Debug, Default)]
pub struct PendingState {
  active: bool,
  pending: ChangeSet,
  broken_source: bool,
}

/// Polling file observer with a quiet-period debounce.
#[derive(Debug)]
pub struct FileObserver {
  repository: PathBuf,
  scenario_files: Vec<PathBuf>,
  lock_file: PathBuf,
  debounce: Duration,
  snapshot: Snapshot,
  pending: ChangeSet,
  last_change: Option<Instant>,
}

type Snapshot = BTreeMap<PathBuf, FileStamp>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileStamp {
  length: u64,
  modified: Option<SystemTime>,
}

impl ChangeSet {
  /// Returns whether at least one observable input changed.
  pub fn is_empty(&self) -> bool {
    if self.scenario || self.lock {
      return false;
    }
    !self.source && !self.retry_broken_build
  }

  /// Returns the required cycle path, with source changes taking precedence.
  pub fn path(&self) -> Option<CyclePath> {
    if self.source || self.retry_broken_build {
      Some(CyclePath::ReplacementBuild)
    } else if self.scenario {
      Some(CyclePath::Execution)
    } else if self.lock {
      Some(CyclePath::ComparisonOnly)
    } else {
      None
    }
  }

  /// Unions another observed state into this pending state.
  pub fn merge(&mut self, other: Self) {
    self.scenario |= other.scenario;
    self.lock |= other.lock;
    self.source |= other.source;
    self.retry_broken_build |= other.retry_broken_build;
  }
}

impl PendingState {
  /// Begins the initial or next immutable cycle.
  pub fn begin(&mut self, changes: ChangeSet) -> Result<CyclePath> {
    ensure!(!self.active, "a watch cycle is already active");
    let path = changes.path().context("watch cycle has no changes")?;
    if self.broken_source && path == CyclePath::Execution {
      anyhow::bail!("scenario changes wait for a new or explicitly retried source fingerprint");
    }
    self.active = true;
    Ok(path)
  }

  /// Merges edits that arrive while another immutable cycle is active.
  pub fn enqueue(&mut self, changes: ChangeSet) {
    self.pending.merge(changes);
  }

  /// Records a replacement-build outcome and returns the one pending state.
  pub fn finish(&mut self, replacement_build_succeeded: Option<bool>) -> Option<ChangeSet> {
    assert!(self.active, "no watch cycle is active");
    if let Some(succeeded) = replacement_build_succeeded {
      self.broken_source = !succeeded;
    }
    self.active = false;
    (!self.pending.is_empty()).then(|| std::mem::take(&mut self.pending))
  }

  /// Creates the explicit terminal retry input for a broken fingerprint.
  pub fn retry(&self) -> Option<ChangeSet> {
    self.broken_source.then_some(ChangeSet {
      retry_broken_build: true,
      ..ChangeSet::default()
    })
  }

  /// Reports whether the warm player must remain idle for broken source.
  pub fn source_is_broken(&self) -> bool {
    self.broken_source
  }
}

impl FileObserver {
  /// Captures an initial repository snapshot without emitting a change.
  pub fn new(
    repository: impl Into<PathBuf>,
    scenario_files: impl IntoIterator<Item = PathBuf>,
    lock_file: impl Into<PathBuf>,
    debounce: Duration,
  ) -> Result<Self> {
    ensure!(!debounce.is_zero(), "watch debounce must be positive");
    let repository = repository.into();
    ensure!(repository.is_dir(), "watch repository is not a directory");
    let scenario_files: Vec<_> = scenario_files
      .into_iter()
      .map(|path| absolute(&repository, path))
      .collect();
    let lock_file = absolute(&repository, lock_file.into());
    let mut snapshot = snapshot(&repository)?;
    observe_explicit(&mut snapshot, scenario_files.iter().chain([&lock_file]))?;
    Ok(Self {
      repository,
      scenario_files,
      lock_file,
      debounce,
      snapshot,
      pending: ChangeSet::default(),
      last_change: None,
    })
  }

  /// Observes current files and emits only after the debounce period is quiet.
  pub fn poll(&mut self, now: Instant) -> Result<Option<ChangeSet>> {
    let mut current = snapshot(&self.repository)?;
    observe_explicit(
      &mut current,
      self.scenario_files.iter().chain([&self.lock_file]),
    )?;
    let changed = changed_paths(&self.snapshot, &current);
    if !changed.is_empty() {
      self.pending.merge(self.classify(&changed));
      self.last_change = Some(now);
      self.snapshot = current;
      return Ok(None);
    }
    let ready = self
      .last_change
      .is_some_and(|changed_at| now.saturating_duration_since(changed_at) >= self.debounce);
    if !ready {
      return Ok(None);
    }
    self.last_change = None;
    Ok(Some(std::mem::take(&mut self.pending)))
  }

  fn classify(&self, paths: &[PathBuf]) -> ChangeSet {
    let mut changes = ChangeSet::default();
    for path in paths {
      if path == &self.lock_file {
        changes.lock = true;
      } else if self.scenario_files.contains(path) {
        changes.scenario = true;
      } else {
        changes.source = true;
      }
    }
    changes
  }
}

fn snapshot(repository: &Path) -> Result<Snapshot> {
  let mut files = BTreeMap::new();
  visit(repository, &mut files)?;
  Ok(files)
}

fn visit(directory: &Path, files: &mut Snapshot) -> Result<()> {
  for entry in fs::read_dir(directory)
    .with_context(|| format!("observe watch directory {}", directory.display()))?
  {
    let entry = entry?;
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() {
      continue;
    }
    if metadata.is_dir() {
      let name = entry.file_name();
      if !IGNORED_DIRECTORIES.iter().any(|ignored| name == *ignored) {
        visit(&path, files)?;
      }
      continue;
    }
    if metadata.is_file() {
      files.insert(
        path,
        FileStamp {
          length: metadata.len(),
          modified: metadata.modified().ok(),
        },
      );
    }
  }
  Ok(())
}

fn observe_explicit<'a>(
  files: &mut Snapshot,
  paths: impl IntoIterator<Item = &'a PathBuf>,
) -> Result<()> {
  for path in paths {
    match fs::metadata(path) {
      Ok(metadata) if metadata.is_file() => {
        files.insert(
          path.clone(),
          FileStamp {
            length: metadata.len(),
            modified: metadata.modified().ok(),
          },
        );
      }
      Ok(_) => anyhow::bail!("watched path {} is not a file", path.display()),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
      Err(error) => return Err(error.into()),
    }
  }
  Ok(())
}

fn absolute(repository: &Path, path: PathBuf) -> PathBuf {
  if path.is_absolute() {
    path
  } else {
    repository.join(path)
  }
}

fn changed_paths(before: &Snapshot, after: &Snapshot) -> Vec<PathBuf> {
  before
    .keys()
    .chain(after.keys())
    .filter(|path| before.get(*path) != after.get(*path))
    .cloned()
    .collect::<std::collections::BTreeSet<_>>()
    .into_iter()
    .collect()
}
