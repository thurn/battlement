use std::{
  fs,
  path::Path,
  sync::{Arc, Barrier},
  thread,
};

use battlement_ditto::wire::{
  result::{ResultCommand, RunResult, RunStatus},
  run_storage::{RunIndexEntry, RunStore},
};
use tempfile::TempDir;

#[test]
fn large_terminal_history_is_not_deserialized_by_startup_or_recovery() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path();
  let entries = history(root, 1024, false);
  fs::write(
    root.join("index.json"),
    serde_json::to_vec(&serde_json::json!({"entries": entries})).unwrap(),
  )
  .unwrap();
  let leased = root.join(&entries[0].run_id);
  let lease = b"{\"owner\":\"active\",\"expires_unix_s\":1000}";
  fs::write(leased.join(".lease.json"), lease).unwrap();
  let mut store = RunStore::open(root).unwrap();
  assert!(store.maintain(100, u64::MAX).unwrap().recovered.is_empty());
  assert_eq!(store.entries().len(), 1024);
  assert_eq!(fs::read(leased.join(".lease.json")).unwrap(), lease);
  assert!(leased.join("partial-result.json").exists());
  assert!(
    !root
      .join(&entries[1].run_id)
      .join("partial-result.json")
      .exists()
  );
  let error = store.load_result(&entries[1].run_id, 101).unwrap_err();
  assert!(
    format!("{error:#}").contains(
      &root
        .join(&entries[1].run_id)
        .join("result.json")
        .display()
        .to_string()
    )
  );
  let id = "20000000-0000-4000-8000-000000000001";
  let mut active = store
    .begin(result(id, ResultCommand::Run), &mut Vec::new(), 101)
    .unwrap();
  store
    .finalize(&mut active, result(id, ResultCommand::Run), 102)
    .unwrap();
  assert_eq!(
    store.load_result(id, 103).unwrap().status,
    RunStatus::Passed
  );
}

#[test]
fn startup_and_retention_tolerate_concurrently_removed_history() {
  for _ in 0..8 {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path();
    let entries = history(root, 256, true);
    let barrier = Arc::new(Barrier::new(2));
    let remover_barrier = barrier.clone();
    thread::scope(|scope| {
      let remover = scope.spawn(|| {
        remover_barrier.wait();
        for entry in &entries {
          fs::remove_dir_all(root.join(&entry.run_id)).unwrap();
          thread::yield_now();
        }
      });
      barrier.wait();
      for _ in 0..8 {
        let mut store = RunStore::open(root).unwrap();
        store.cleanup(100, u64::MAX).unwrap();
      }
      remover.join().unwrap();
    });
    let mut store = RunStore::open(root).unwrap();
    store.cleanup(100, u64::MAX).unwrap();
    assert!(store.entries().iter().all(|entry| entry.artifacts_evicted));
  }
}

#[test]
fn malformed_unfinished_result_identifies_its_exact_path() {
  let temporary = TempDir::new().unwrap();
  let id = "20000000-0000-4000-8000-000000000001";
  let mut store = RunStore::open(temporary.path()).unwrap();
  let active = store
    .begin(result(id, ResultCommand::Run), &mut Vec::new(), 10)
    .unwrap();
  let partial = active.path().join("partial-result.json");
  fs::write(&partial, b"invalid").unwrap();
  let recovery = store.recover_abandoned(100).unwrap_err();
  let startup = RunStore::open(temporary.path()).unwrap_err();
  for error in [recovery, startup] {
    assert!(format!("{error:#}").contains(&partial.display().to_string()));
  }
}

fn history(root: &Path, count: usize, valid: bool) -> Vec<RunIndexEntry> {
  (0..count)
    .map(|number| {
      let run_id = format!("10000000-0000-4000-8000-{number:012}");
      let directory = root.join(&run_id);
      fs::create_dir(&directory).unwrap();
      let bytes = if valid {
        serde_json::to_vec(&result(&run_id, ResultCommand::Run)).unwrap()
      } else {
        b"{\"legacy\":true}".to_vec()
      };
      fs::write(directory.join("result.json"), &bytes).unwrap();
      fs::write(
        directory.join("partial-result.json"),
        b"invalid stale partial",
      )
      .unwrap();
      RunIndexEntry {
        run_id,
        repository: None,
        suite: None,
        last_accessed_unix_s: 10,
        terminal_status: Some(RunStatus::Passed),
        artifact_bytes: 0,
        artifacts_evicted: false,
      }
    })
    .collect()
}

fn result(run_id: &str, command: ResultCommand) -> RunResult {
  RunResult {
    run_id: run_id.to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command,
    source_command: None,
    cycle: 1,
    suite: None,
    profile: None,
    started_at: "2026-08-28T20:00:00Z".to_owned(),
    duration_ms: 0,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: vec![],
    player_sessions: vec![],
    jobs: vec![],
    scenarios: vec![],
    warnings: vec![],
    errors: vec![],
    baseline_writes: vec![],
    artifacts: vec![],
    performance: None,
  }
}
