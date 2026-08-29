//! Shared immutable player-build cache.

use std::{
  fs::{self, File},
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::{
  build_cache_cleanup, build_cache_io,
  build_identity::{BuildIdentity, NoBuildDecision},
  fingerprint::SourceManifest,
};

const FAILURE_FILE: &str = "failure.json";
const JOURNAL_FILE: &str = "journal.jsonl";
const METADATA_FILE: &str = "metadata.json";

pub const DEFAULT_BUILD_CACHE_BYTES: u64 = 20 * 1024 * 1024 * 1024;
pub const BUILD_LOG_FILE: &str = "build.log";
pub const SOURCE_MANIFEST_FILE: &str = "source-manifest.json";

/// A shared build cache with a configurable global LRU limit.
#[derive(Clone, Debug)]
pub struct BuildCache {
  root: PathBuf,
  limit_bytes: u64,
}

/// Exact lookup result after serializing builders for one fingerprint.
#[derive(Debug)]
pub enum BuildAccess {
  Reused(BuildHandle),
  Build(PendingBuild),
}

/// A published build protected from eviction for this handle's lifetime.
#[derive(Debug)]
pub struct BuildHandle {
  path: PathBuf,
  metadata: BuildMetadata,
  active_lock: File,
}

/// Exclusive permission to populate one unpublished fingerprint.
#[derive(Debug)]
pub struct PendingBuild {
  cache: BuildCache,
  repository: String,
  suite: String,
  identity: BuildIdentity,
  staging: PathBuf,
  build_lock: File,
}

/// A new active build and the automatic LRU maintenance it triggered.
#[derive(Debug)]
pub struct PublishedBuild {
  pub build: BuildHandle,
  pub maintenance: CacheCleanup,
}

/// Complete immutable metadata stored beside a reusable player.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildMetadata {
  pub identity: BuildIdentity,
  pub repository: String,
  pub suite: String,
  pub player: String,
  pub created_unix_s: u64,
}

/// Retained diagnostics for an unsuccessful build attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildFailure {
  pub phase: String,
  pub error_ids: Vec<String>,
  pub message: String,
  pub failed_at_unix_s: u64,
}

/// Scope selected by an explicit build-cache cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupScope {
  Suite { repository: String, suite: String },
  Global,
}

/// One inactive immutable build removed by maintenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvictedBuild {
  pub fingerprint: String,
  pub repository: String,
  pub suite: String,
  pub bytes: u64,
}

/// One entry larger than the configured cache limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OversizeBuild {
  pub fingerprint: String,
  pub bytes: u64,
}

/// Result of automatic LRU enforcement or explicit cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheCleanup {
  pub evicted: Vec<EvictedBuild>,
  pub active: Vec<String>,
  pub oversize: Vec<OversizeBuild>,
  pub remaining_bytes: u64,
  pub limit_bytes: u64,
}

/// Inactive and leased builds found before an explicit cleanup mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPreview {
  pub inactive: Vec<EvictedBuild>,
  pub active: Vec<String>,
}

/// The closest reusable build and the inputs separating it from a requested build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NearestBuildMismatch {
  pub fingerprint: String,
  pub changed_inputs: Vec<String>,
  pub added_paths: Vec<String>,
  pub removed_paths: Vec<String>,
  pub changed_paths: Vec<String>,
}

/// Durable cache journal event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheEvent {
  Created,
  Reused,
  Failed,
  Evicted,
}

/// One append-only build-cache journal record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheJournalEntry {
  pub event: CacheEvent,
  pub fingerprint: String,
  pub suite: String,
  pub bytes: u64,
  pub at_unix_s: u64,
}

impl BuildCache {
  /// Opens or creates the machine-wide build cache.
  pub fn open(root: impl Into<PathBuf>, limit_bytes: u64) -> Result<Self> {
    ensure!(limit_bytes > 0, "build cache limit must be positive");
    let root = root.into();
    for directory in ["access", "entries", "failures", "locks", "staging"] {
      fs::create_dir_all(root.join(directory))?;
    }
    Ok(Self {
      root: root.canonicalize()?,
      limit_bytes,
    })
  }

  /// Waits for the exact fingerprint, then returns it or sole build permission.
  pub fn acquire(
    &self,
    repository: &str,
    suite: &str,
    identity: &BuildIdentity,
    now_unix_s: u64,
  ) -> Result<BuildAccess> {
    identity.validate()?;
    ensure!(!repository.is_empty(), "build repository is empty");
    ensure!(!suite.is_empty(), "build suite is empty");
    let fingerprint = &identity.fingerprint;
    let build_lock = build_cache_io::lock_exclusive(&self.build_lock_path(fingerprint))?;
    let active_lock = build_cache_io::lock_shared(&self.active_lock_path(fingerprint))?;
    let entry = self.entry_path(fingerprint);
    if entry.is_dir() {
      let metadata = self::load_metadata(&entry, identity)?;
      self.touch_access(fingerprint, now_unix_s)?;
      self.append_journal(CacheJournalEntry {
        event: CacheEvent::Reused,
        fingerprint: fingerprint.clone(),
        suite: metadata.suite.clone(),
        bytes: build_cache_io::directory_bytes(&entry)?,
        at_unix_s: now_unix_s,
      })?;
      drop(build_lock);
      return Ok(BuildAccess::Reused(BuildHandle {
        path: entry,
        metadata,
        active_lock,
      }));
    }
    ensure!(!entry.exists(), "build cache entry is not a directory");
    drop(active_lock);
    let staging = self.staging_path(fingerprint);
    if staging.exists() {
      fs::remove_dir_all(&staging).context("remove interrupted build staging")?;
    }
    fs::create_dir(&staging)?;
    Ok(BuildAccess::Build(PendingBuild {
      cache: self.clone(),
      repository: repository.to_owned(),
      suite: suite.to_owned(),
      identity: identity.clone(),
      staging,
      build_lock,
    }))
  }

  /// Evicts oldest inactive entries until the configured size is satisfied.
  pub fn enforce_limit(&self, now_unix_s: u64) -> Result<CacheCleanup> {
    build_cache_cleanup::enforce_limit(self, now_unix_s)
  }

  /// Removes every inactive build in one suite or across the shared cache.
  pub fn cleanup(&self, scope: &CleanupScope, now_unix_s: u64) -> Result<CacheCleanup> {
    build_cache_cleanup::cleanup(self, scope, now_unix_s)
  }

  /// Plans an explicit cleanup without deleting cache entries.
  pub fn cleanup_preview(&self, scope: &CleanupScope) -> Result<CleanupPreview> {
    build_cache_cleanup::preview(self, scope)
  }

  /// Applies only the inactive entries frozen by an earlier preview.
  pub fn cleanup_planned(&self, preview: &CleanupPreview, now_unix_s: u64) -> Result<CacheCleanup> {
    build_cache_cleanup::cleanup_planned(self, preview, now_unix_s)
  }

  /// Finds the same repository and suite build with the fewest changed inputs.
  pub fn nearest_build_mismatch(
    &self,
    repository: &str,
    suite: &str,
    expected: &BuildIdentity,
    current_source: &SourceManifest,
  ) -> Result<Option<NearestBuildMismatch>> {
    let mut nearest = None;
    for entry in fs::read_dir(self.entries_path())? {
      let entry = entry?;
      if !entry.file_type()?.is_dir() {
        continue;
      }
      let metadata: BuildMetadata = build_cache_io::read_json(&entry.path().join(METADATA_FILE))?;
      metadata.identity.validate()?;
      if metadata.repository != repository || metadata.suite != suite {
        continue;
      }
      let NoBuildDecision::Required { changed_inputs, .. } =
        expected.no_build_decision(Some(&metadata.identity))
      else {
        continue;
      };
      let retained = SourceManifest::read(&entry.path().join(SOURCE_MANIFEST_FILE))?;
      ensure!(
        retained.fingerprint == metadata.identity.source_fingerprint,
        "cached source manifest does not match build metadata"
      );
      let difference = current_source.difference(&retained);
      let score = (
        changed_inputs.len(),
        difference.added.len() + difference.removed.len() + difference.changed.len(),
        metadata.identity.fingerprint.clone(),
      );
      let mismatch = NearestBuildMismatch {
        fingerprint: metadata.identity.fingerprint,
        changed_inputs,
        added_paths: difference.added,
        removed_paths: difference.removed,
        changed_paths: difference.changed,
      };
      if nearest
        .as_ref()
        .is_none_or(|(nearest_score, _)| score < *nearest_score)
      {
        nearest = Some((score, mismatch));
      }
    }
    Ok(nearest.map(|(_, mismatch)| mismatch))
  }

  /// Reads the append-only creation, reuse, failure, and eviction journal.
  pub fn journal(&self) -> Result<Vec<CacheJournalEntry>> {
    build_cache_io::read_journal(&self.root.join(JOURNAL_FILE))
  }

  pub(super) fn entry_path(&self, fingerprint: &str) -> PathBuf {
    self.root.join("entries").join(fingerprint)
  }

  pub(super) fn entries_path(&self) -> PathBuf {
    self.root.join("entries")
  }

  pub(super) fn access_path(&self, fingerprint: &str) -> PathBuf {
    self.root.join("access").join(fingerprint)
  }

  pub(super) fn active_lock_path(&self, fingerprint: &str) -> PathBuf {
    self
      .root
      .join("locks")
      .join(format!("{fingerprint}.active"))
  }

  pub(super) fn limit_bytes(&self) -> u64 {
    self.limit_bytes
  }

  pub(super) fn append_journal(&self, event: CacheJournalEntry) -> Result<()> {
    build_cache_io::append_journal(&self.root.join(JOURNAL_FILE), &event)
  }

  fn build_lock_path(&self, fingerprint: &str) -> PathBuf {
    self.root.join("locks").join(format!("{fingerprint}.build"))
  }

  fn staging_path(&self, fingerprint: &str) -> PathBuf {
    self.root.join("staging").join(fingerprint)
  }

  fn touch_access(&self, fingerprint: &str, now_unix_s: u64) -> Result<()> {
    let path = self.access_path(fingerprint);
    let retained = if path.is_file() {
      build_cache_io::read_access(&path)?
    } else {
      0
    };
    build_cache_io::write_access(&path, retained.max(now_unix_s))
  }
}

impl BuildHandle {
  /// Returns the immutable cache-entry directory.
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Returns the retained immutable metadata.
  pub fn metadata(&self) -> &BuildMetadata {
    &self.metadata
  }

  /// Returns the selected player artifact inside the cache entry.
  pub fn player_path(&self) -> PathBuf {
    self.path.join(&self.metadata.player)
  }
}

impl Drop for BuildHandle {
  fn drop(&mut self) {
    let _ = FileExt::unlock(&self.active_lock);
  }
}

impl PendingBuild {
  /// Returns the private staging directory populated by the builder.
  pub fn path(&self) -> &Path {
    &self.staging
  }

  /// Returns the exact identity this exclusive build must publish.
  pub fn identity(&self) -> &BuildIdentity {
    &self.identity
  }

  /// Releases an unpublished selection without retaining a failed build.
  pub fn discard(self) -> Result<()> {
    if self.staging.is_dir() {
      fs::remove_dir_all(&self.staging)?;
    }
    Ok(())
  }

  /// Atomically publishes a complete player, metadata, manifest, and log.
  pub fn publish(self, player: &Path, created_unix_s: u64) -> Result<PublishedBuild> {
    let player = build_cache_io::relative_artifact(&self.staging, player)?;
    build_cache_io::relative_file(&self.staging, Path::new(BUILD_LOG_FILE))?;
    let source_path =
      build_cache_io::relative_file(&self.staging, Path::new(SOURCE_MANIFEST_FILE))?;
    let source = SourceManifest::read(&source_path)?;
    ensure!(
      source.fingerprint == self.identity.source_fingerprint,
      "source manifest does not match build identity"
    );
    let metadata = BuildMetadata {
      identity: self.identity.clone(),
      repository: self.repository.clone(),
      suite: self.suite.clone(),
      player: player
        .strip_prefix(&self.staging)?
        .to_string_lossy()
        .replace('\\', "/"),
      created_unix_s,
    };
    build_cache_io::write_json(&self.staging.join(METADATA_FILE), &metadata)?;
    build_cache_io::sync_tree(&self.staging)?;
    let active_lock =
      build_cache_io::lock_shared(&self.cache.active_lock_path(&self.identity.fingerprint))?;
    let entry = self.cache.entry_path(&self.identity.fingerprint);
    ensure!(!entry.exists(), "build cache entry already exists");
    fs::rename(&self.staging, &entry)?;
    build_cache_io::sync_directory(&self.cache.entries_path())?;
    build_cache_io::write_access(
      &self.cache.access_path(&self.identity.fingerprint),
      created_unix_s,
    )?;
    let bytes = build_cache_io::directory_bytes(&entry)?;
    self.cache.append_journal(CacheJournalEntry {
      event: CacheEvent::Created,
      fingerprint: self.identity.fingerprint.clone(),
      suite: self.suite,
      bytes,
      at_unix_s: created_unix_s,
    })?;
    drop(self.build_lock);
    let build = BuildHandle {
      path: entry,
      metadata,
      active_lock,
    };
    let maintenance = self.cache.enforce_limit(created_unix_s)?;
    Ok(PublishedBuild { build, maintenance })
  }

  /// Retains a failed attempt and its full log without publishing an entry.
  pub fn fail(self, failure: &BuildFailure) -> Result<PathBuf> {
    let log = build_cache_io::relative_file(&self.staging, Path::new(BUILD_LOG_FILE))?;
    ensure!(!failure.phase.is_empty(), "build failure phase is empty");
    ensure!(
      !failure.message.is_empty(),
      "build failure message is empty"
    );
    let parent = self
      .cache
      .root
      .join("failures")
      .join(&self.identity.fingerprint);
    fs::create_dir_all(&parent)?;
    let destination = build_cache_io::next_failure_path(&parent, failure.failed_at_unix_s);
    fs::create_dir(&destination)?;
    fs::copy(log, destination.join(BUILD_LOG_FILE))?;
    build_cache_io::write_json(&destination.join(FAILURE_FILE), failure)?;
    build_cache_io::sync_tree(&destination)?;
    build_cache_io::sync_directory(&parent)?;
    fs::remove_dir_all(&self.staging)?;
    let bytes = build_cache_io::directory_bytes(&destination)?;
    self.cache.append_journal(CacheJournalEntry {
      event: CacheEvent::Failed,
      fingerprint: self.identity.fingerprint,
      suite: self.suite,
      bytes,
      at_unix_s: failure.failed_at_unix_s,
    })?;
    drop(self.build_lock);
    Ok(destination)
  }
}

fn load_metadata(entry: &Path, expected: &BuildIdentity) -> Result<BuildMetadata> {
  build_cache_io::relative_file(entry, Path::new(BUILD_LOG_FILE))?;
  let source_path = build_cache_io::relative_file(entry, Path::new(SOURCE_MANIFEST_FILE))?;
  let source = SourceManifest::read(&source_path)?;
  let metadata: BuildMetadata = build_cache_io::read_json(&entry.join(METADATA_FILE))?;
  metadata.identity.validate()?;
  ensure!(
    metadata.identity == *expected,
    "cached build identity mismatch"
  );
  ensure!(
    source.fingerprint == expected.source_fingerprint,
    "cached source manifest mismatch"
  );
  build_cache_io::relative_artifact(entry, Path::new(&metadata.player))?;
  Ok(metadata)
}
