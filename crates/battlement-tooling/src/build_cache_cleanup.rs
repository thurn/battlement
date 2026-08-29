use std::{fs, path::PathBuf};

use anyhow::{Result, ensure};

use crate::{
  build_cache::{
    BuildCache, BuildMetadata, CacheCleanup, CacheEvent, CacheJournalEntry, CleanupPreview,
    CleanupScope, EvictedBuild, OversizeBuild,
  },
  build_cache_io,
};

#[derive(Clone, Debug)]
struct CacheEntry {
  fingerprint: String,
  repository: String,
  suite: String,
  path: PathBuf,
  bytes: u64,
  last_accessed_unix_s: u64,
}

pub(super) fn enforce_limit(cache: &BuildCache, now_unix_s: u64) -> Result<CacheCleanup> {
  let mut entries = self::scan(cache)?;
  entries.sort_by(|left, right| {
    (left.last_accessed_unix_s, &left.fingerprint)
      .cmp(&(right.last_accessed_unix_s, &right.fingerprint))
  });
  let mut remaining = entries.iter().map(|entry| entry.bytes).sum::<u64>();
  let mut evicted = Vec::new();
  let mut active = Vec::new();
  for entry in entries {
    if remaining <= cache.limit_bytes() {
      break;
    }
    match self::evict(cache, &entry, now_unix_s)? {
      Some(eviction) => {
        remaining = remaining.saturating_sub(eviction.bytes);
        evicted.push(eviction);
      }
      None => active.push(entry.fingerprint),
    }
  }
  self::report(cache, evicted, active)
}

pub(super) fn cleanup(
  cache: &BuildCache,
  scope: &CleanupScope,
  now_unix_s: u64,
) -> Result<CacheCleanup> {
  let mut entries = self::scan(cache)?;
  entries.sort_by(|left, right| {
    (left.last_accessed_unix_s, &left.fingerprint)
      .cmp(&(right.last_accessed_unix_s, &right.fingerprint))
  });
  let mut evicted = Vec::new();
  let mut active = Vec::new();
  for entry in entries
    .into_iter()
    .filter(|entry| self::in_scope(scope, &entry.repository, &entry.suite))
  {
    match self::evict(cache, &entry, now_unix_s)? {
      Some(eviction) => evicted.push(eviction),
      None => active.push(entry.fingerprint),
    }
  }
  self::report(cache, evicted, active)
}

pub(super) fn preview(cache: &BuildCache, scope: &CleanupScope) -> Result<CleanupPreview> {
  let mut inactive = Vec::new();
  let mut active = Vec::new();
  for entry in self::scan(cache)?
    .into_iter()
    .filter(|entry| self::in_scope(scope, &entry.repository, &entry.suite))
  {
    if build_cache_io::try_lock_exclusive(&cache.active_lock_path(&entry.fingerprint))?.is_some() {
      inactive.push(EvictedBuild {
        fingerprint: entry.fingerprint,
        repository: entry.repository,
        suite: entry.suite,
        bytes: entry.bytes,
      });
    } else {
      active.push(entry.fingerprint);
    }
  }
  inactive.sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
  active.sort();
  Ok(CleanupPreview { inactive, active })
}

pub(super) fn cleanup_planned(
  cache: &BuildCache,
  preview: &CleanupPreview,
  now_unix_s: u64,
) -> Result<CacheCleanup> {
  let entries = self::scan(cache)?;
  let mut evicted = Vec::new();
  let mut active = Vec::new();
  for planned in &preview.inactive {
    let Some(entry) = entries
      .iter()
      .find(|entry| entry.fingerprint == planned.fingerprint)
    else {
      continue;
    };
    ensure!(
      entry.repository == planned.repository
        && entry.suite == planned.suite
        && entry.bytes == planned.bytes,
      "planned build cleanup entry changed"
    );
    match self::evict(cache, entry, now_unix_s)? {
      Some(eviction) => evicted.push(eviction),
      None => active.push(entry.fingerprint.clone()),
    }
  }
  self::report(cache, evicted, active)
}

fn evict(cache: &BuildCache, entry: &CacheEntry, now_unix_s: u64) -> Result<Option<EvictedBuild>> {
  let Some(_lock) =
    build_cache_io::try_lock_exclusive(&cache.active_lock_path(&entry.fingerprint))?
  else {
    return Ok(None);
  };
  if !entry.path.exists() {
    return Ok(Some(EvictedBuild {
      fingerprint: entry.fingerprint.clone(),
      repository: entry.repository.clone(),
      suite: entry.suite.clone(),
      bytes: 0,
    }));
  }
  fs::remove_dir_all(&entry.path)?;
  let access = cache.access_path(&entry.fingerprint);
  if access.is_file() {
    fs::remove_file(access)?;
  }
  build_cache_io::sync_directory(&cache.entries_path())?;
  if let Some(access_directory) = cache.access_path(&entry.fingerprint).parent() {
    build_cache_io::sync_directory(access_directory)?;
  }
  cache.append_journal(CacheJournalEntry {
    event: CacheEvent::Evicted,
    fingerprint: entry.fingerprint.clone(),
    suite: entry.suite.clone(),
    bytes: entry.bytes,
    at_unix_s: now_unix_s,
  })?;
  Ok(Some(EvictedBuild {
    fingerprint: entry.fingerprint.clone(),
    repository: entry.repository.clone(),
    suite: entry.suite.clone(),
    bytes: entry.bytes,
  }))
}

fn report(
  cache: &BuildCache,
  evicted: Vec<EvictedBuild>,
  mut active: Vec<String>,
) -> Result<CacheCleanup> {
  let remaining = self::scan(cache)?;
  active.sort();
  active.dedup();
  Ok(CacheCleanup {
    evicted,
    active,
    oversize: remaining
      .iter()
      .filter(|entry| entry.bytes > cache.limit_bytes())
      .map(|entry| OversizeBuild {
        fingerprint: entry.fingerprint.clone(),
        bytes: entry.bytes,
      })
      .collect(),
    remaining_bytes: remaining.iter().map(|entry| entry.bytes).sum(),
    limit_bytes: cache.limit_bytes(),
  })
}

fn scan(cache: &BuildCache) -> Result<Vec<CacheEntry>> {
  let mut entries = Vec::new();
  for entry in fs::read_dir(cache.entries_path())? {
    let entry = entry?;
    if !entry.file_type()?.is_dir() {
      continue;
    }
    let fingerprint = entry.file_name().to_string_lossy().into_owned();
    let metadata: BuildMetadata = build_cache_io::read_json(&entry.path().join("metadata.json"))?;
    metadata.identity.validate()?;
    ensure!(
      metadata.identity.fingerprint == fingerprint,
      "cache entry directory does not match fingerprint"
    );
    let access = cache.access_path(&fingerprint);
    entries.push(CacheEntry {
      bytes: build_cache_io::directory_bytes(&entry.path())?,
      last_accessed_unix_s: if access.is_file() {
        build_cache_io::read_access(&access)?
      } else {
        metadata.created_unix_s
      },
      fingerprint,
      repository: metadata.repository,
      suite: metadata.suite,
      path: entry.path(),
    });
  }
  Ok(entries)
}

fn in_scope(scope: &CleanupScope, repository: &str, suite: &str) -> bool {
  match scope {
    CleanupScope::Suite {
      repository: selected_repository,
      suite: selected_suite,
    } => selected_repository == repository && selected_suite == suite,
    CleanupScope::Global => true,
  }
}
