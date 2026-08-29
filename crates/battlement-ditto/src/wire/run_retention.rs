use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use uuid::Uuid;

use crate::wire::{
  result::RunResult,
  run_storage,
  run_storage::{ActiveRun, EvictedRun, RETENTION_SECONDS, RecoveredRun, RunMaintenance, RunStore},
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
    let run_ids: Vec<String> = self
      .index
      .entries
      .iter()
      .map(|entry| entry.run_id.clone())
      .collect();
    let mut recovered = Vec::new();
    for run_id in run_ids {
      let directory = self.run_directory(&run_id)?;
      if !directory.exists() || run_storage_io::lease_active(&directory, now_unix_s)? {
        continue;
      }
      let terminal_pending = directory.join(".terminal-pending").is_file();
      if directory.join(run_storage_io::RESULT_FILE).is_file() && !terminal_pending {
        let result: RunResult =
          serde_json::from_slice(&fs::read(directory.join(run_storage_io::RESULT_FILE))?)?;
        result.validate()?;
        let entry = self.entry_mut(&run_id)?;
        entry.terminal_status = Some(result.status);
        entry.artifact_bytes = run_storage_io::directory_bytes(&directory)?;
        remove_internal_recovery_files(&directory)?;
        continue;
      }
      let partial_path = directory.join("partial-result.json");
      if !partial_path.is_file() {
        continue;
      }
      let durability_failure = terminal_pending;
      let mut result: RunResult = serde_json::from_slice(&fs::read(&partial_path)?)?;
      result = run_storage::recover_result(result, durability_failure)?;
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
    self.persist_index()?;
    Ok(recovered)
  }

  /// Evicts expired and least-recently-used inactive terminal run artifacts.
  pub fn cleanup(&mut self, now_unix_s: u64, maximum_bytes: u64) -> Result<Vec<EvictedRun>> {
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
      entry.artifact_bytes = run_storage_io::directory_bytes(&directory)?;
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
      fs::remove_dir_all(&directory)
        .with_context(|| format!("evict run directory {}", directory.display()))?;
      let entry = self.entry_mut(&run_id)?;
      entry.artifact_bytes = 0;
      entry.artifacts_evicted = true;
      retained_bytes = retained_bytes.saturating_sub(bytes);
      evicted.push(EvictedRun {
        run_id,
        artifact_bytes: bytes,
      });
    }
    self.persist_index()?;
    Ok(evicted)
  }
}

fn remove_internal_recovery_files(directory: &Path) -> Result<()> {
  run_storage_io::remove_if_file(&directory.join(run_storage_io::PARTIAL_FILE))?;
  run_storage_io::remove_if_file(&directory.join(run_storage_io::PENDING_FILE))?;
  run_storage_io::remove_if_file(&directory.join(run_storage_io::LEASE_FILE))
}
