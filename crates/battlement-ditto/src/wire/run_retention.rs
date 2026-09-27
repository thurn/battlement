use std::{fs, io, path::Path};

use anyhow::{Context, Result, ensure};
use uuid::Uuid;

use crate::wire::{
  run_index, run_storage,
  run_storage::{
    ActiveRun, EvictedRun, RETENTION_SECONDS, RecoveredRun, RunCleanupPreview, RunCleanupScope,
    RunMaintenance, RunStore,
  },
  run_storage_io,
};

impl RunStore {
  /// Recovers expired runs, then applies age and LRU retention.
  pub fn maintain(&mut self, now_unix_s: u64, maximum_bytes: u64) -> Result<RunMaintenance> {
    let recovered = self.recover_abandoned(now_unix_s)?;
    let evicted = self.cleanup(now_unix_s, maximum_bytes)?;
    Ok(RunMaintenance { recovered, evicted })
  }

  /// Converts every expired partial run into an interrupted or durability result.
  pub fn recover_abandoned(&mut self, now_unix_s: u64) -> Result<Vec<RecoveredRun>> {
    self.sync_index()?;
    let runs: Vec<_> = self
      .index
      .entries
      .iter()
      .map(|entry| (entry.run_id.clone(), entry.terminal_status.is_some()))
      .collect();
    let mut recovered = Vec::new();
    for (run_id, terminal) in runs {
      let directory = self.run_directory(&run_id)?;
      if !directory.exists() || run_storage_io::lease_active(&directory, now_unix_s)? {
        continue;
      }
      let terminal_pending = directory.join(run_storage_io::PENDING_FILE).is_file();
      if terminal && !terminal_pending {
        remove_internal_recovery_files(&directory)?;
        continue;
      }
      if directory.join(run_storage_io::RESULT_FILE).is_file() && !terminal_pending {
        let Some(result) =
          run_storage_io::read_result_if_present(&directory.join(run_storage_io::RESULT_FILE))?
        else {
          continue;
        };
        ensure!(
          result.run_id == run_id,
          "run directory {} and result ID disagree",
          directory.display()
        );
        let Some(bytes) = run_storage_io::retained_directory_bytes(&directory)? else {
          continue;
        };
        let entry = self.entry_mut(&run_id)?;
        entry.terminal_status = Some(result.status);
        entry.artifact_bytes = bytes;
        remove_internal_recovery_files(&directory)?;
        continue;
      }
      let partial_path = directory.join(run_storage_io::PARTIAL_FILE);
      let Some(result) = run_storage_io::read_result_if_present(&partial_path)? else {
        continue;
      };
      ensure!(
        result.run_id == run_id,
        "run directory {} and result ID disagree",
        directory.display()
      );
      let mut result = run_storage::recover_result(result, terminal_pending)
        .with_context(|| format!("recover run {}", directory.display()))?;
      result.artifacts = run_storage_io::scan_artifacts(&directory)?;
      let owner = Uuid::new_v4().to_string();
      run_storage_io::write_lease(&directory, &owner, now_unix_s)?;
      let mut active = ActiveRun::recovered(run_id.clone(), directory.clone(), owner);
      let path = self.finalize(&mut active, result.clone(), now_unix_s)?;
      recovered.push(RecoveredRun {
        run_id,
        result_path: path,
        status: result.status,
      });
    }
    self.sync_index()?;
    Ok(recovered)
  }

  /// Evicts expired and least-recently-used inactive terminal run artifacts.
  pub fn cleanup(&mut self, now_unix_s: u64, maximum_bytes: u64) -> Result<Vec<EvictedRun>> {
    self.sync_index()?;
    let mut candidates = Vec::new();
    let mut retained_bytes = 0_u64;
    for entry in &mut self.index.entries {
      if entry.artifacts_evicted {
        continue;
      }
      let directory = self.root.join(&entry.run_id);
      if !directory.exists() {
        entry.artifacts_evicted = true;
        entry.artifact_bytes = 0;
        continue;
      }
      let Some(bytes) = run_storage_io::retained_directory_bytes(&directory)? else {
        retained_bytes = retained_bytes.saturating_add(entry.artifact_bytes);
        continue;
      };
      entry.artifact_bytes = bytes;
      if run_storage_io::lease_active(&directory, now_unix_s)? {
        retained_bytes = retained_bytes.saturating_add(entry.artifact_bytes);
        continue;
      }
      if entry.terminal_status.is_some() {
        retained_bytes = retained_bytes.saturating_add(entry.artifact_bytes);
        candidates.push((entry.last_accessed_unix_s, entry.run_id.clone()));
      }
    }
    candidates.sort();
    let mut evicted = Vec::new();
    for (accessed, run_id) in candidates {
      let expired = now_unix_s.saturating_sub(accessed) >= RETENTION_SECONDS;
      if !expired && retained_bytes <= maximum_bytes {
        continue;
      }
      let directory = self.run_directory(&run_id)?;
      ensure!(
        !run_storage_io::lease_active(&directory, now_unix_s)?,
        "active run selected for eviction"
      );
      let bytes = self.entry_mut(&run_id)?.artifact_bytes;
      match fs::remove_dir_all(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
          return Err(error)
            .with_context(|| format!("evict run directory {}", directory.display()));
        }
      }
      let entry = self.entry_mut(&run_id)?;
      entry.artifact_bytes = 0;
      entry.artifacts_evicted = true;
      retained_bytes = retained_bytes.saturating_sub(bytes);
      evicted.push(EvictedRun {
        run_id,
        artifact_bytes: bytes,
      });
    }
    self.sync_index()?;
    Ok(evicted)
  }

  /// Plans removal of every inactive terminal run in an explicit scope.
  pub fn cleanup_preview(
    &self,
    scope: &RunCleanupScope,
    now_unix_s: u64,
  ) -> Result<RunCleanupPreview> {
    let mut preview = RunCleanupPreview::default();
    for entry in run_index::read(&self.root)?
      .entries
      .iter()
      .filter(|entry| in_scope(entry, scope))
    {
      let directory = self.run_directory(&entry.run_id)?;
      if !directory.is_dir() || entry.artifacts_evicted {
        continue;
      }
      if run_storage_io::lease_active(&directory, now_unix_s)? {
        preview.active.push(entry.run_id.clone());
      } else if entry.terminal_status.is_some() {
        preview.inactive.push(EvictedRun {
          run_id: entry.run_id.clone(),
          artifact_bytes: run_storage_io::directory_bytes(&directory)?,
        });
      }
    }
    preview
      .inactive
      .sort_by(|left, right| left.run_id.cmp(&right.run_id));
    preview.active.sort();
    Ok(preview)
  }

  /// Removes the currently inactive terminal runs in an explicit scope.
  pub fn cleanup_scope(
    &mut self,
    scope: &RunCleanupScope,
    now_unix_s: u64,
  ) -> Result<Vec<EvictedRun>> {
    let preview = self.cleanup_preview(scope, now_unix_s)?;
    self.cleanup_planned(&preview, now_unix_s)
  }

  /// Removes only the inactive terminal runs frozen by an earlier preview.
  pub fn cleanup_planned(
    &mut self,
    preview: &RunCleanupPreview,
    now_unix_s: u64,
  ) -> Result<Vec<EvictedRun>> {
    self.sync_index()?;
    let mut evicted = Vec::new();
    for planned in &preview.inactive {
      let directory = self.run_directory(&planned.run_id)?;
      let entry = self
        .index
        .entries
        .iter()
        .find(|entry| entry.run_id == planned.run_id)
        .context("planned run cleanup entry disappeared")?;
      ensure!(
        entry.terminal_status.is_some()
          && !entry.artifacts_evicted
          && run_storage_io::directory_bytes(&directory)? == planned.artifact_bytes,
        "planned run cleanup entry changed"
      );
      if run_storage_io::lease_active(&directory, now_unix_s)? {
        continue;
      }
      fs::remove_dir_all(&directory)
        .with_context(|| format!("clean run directory {}", directory.display()))?;
      let entry = self.entry_mut(&planned.run_id)?;
      entry.artifact_bytes = 0;
      entry.artifacts_evicted = true;
      evicted.push(planned.clone());
    }
    self.sync_index()?;
    Ok(evicted)
  }
}

fn in_scope(entry: &run_storage::RunIndexEntry, scope: &RunCleanupScope) -> bool {
  match scope {
    RunCleanupScope::Suite { repository, suite } => {
      entry.repository.as_ref() == Some(repository) && entry.suite.as_ref() == Some(suite)
    }
    RunCleanupScope::Global => true,
  }
}

fn remove_internal_recovery_files(directory: &Path) -> Result<()> {
  run_storage_io::remove_if_file(&directory.join(run_storage_io::PARTIAL_FILE))?;
  run_storage_io::remove_if_file(&directory.join(run_storage_io::PENDING_FILE))?;
  run_storage_io::remove_if_file(&directory.join(run_storage_io::LEASE_FILE))
}
