use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
};

use battlement_ditto::wire::{
  common::{ErrorCode, ErrorSource},
  result::{ResultCommand, RunResult, RunStatus},
  run_storage::{RETENTION_SECONDS, RunCleanupScope, RunStore},
};
use tempfile::TempDir;

const RUN_A: &str = "10000000-0000-4000-8000-000000000001";
const RUN_B: &str = "10000000-0000-4000-8000-000000000002";
const RUN_C: &str = "10000000-0000-4000-8000-000000000003";
const RUN_D: &str = "10000000-0000-4000-8000-000000000004";

#[test]
fn run_is_announced_checkpointed_and_committed_durably() {
  let temporary = TempDir::new().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir(&repository).unwrap();
  let root = temporary.path().join("runs");
  let mut store = RunStore::open(&root).unwrap();
  let mut stderr = Vec::new();
  let mut active = store
    .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
    .unwrap();

  assert_eq!(
    String::from_utf8(stderr).unwrap(),
    format!("DITTO_RUN_DIR={}\n", active.path().display())
  );
  assert!(active.path().join("logs/events.jsonl").is_file());
  assert!(active.path().join("partial-result.json").is_file());
  store
    .index_identity(&active, &repository, "suite", 11)
    .unwrap();

  fs::create_dir(active.path().join("diagnostics")).unwrap();
  fs::write(active.path().join("diagnostics/player.log"), b"diagnostic").unwrap();
  let mut checkpoint = result(RUN_A, ResultCommand::Run);
  checkpoint.duration_ms = 20;
  store.checkpoint(&mut active, checkpoint, 12).unwrap();
  let partial: RunResult =
    serde_json::from_slice(&fs::read(active.path().join("partial-result.json")).unwrap()).unwrap();
  assert_eq!(partial.duration_ms, 20);
  assert_eq!(
    partial.artifacts,
    vec![
      "diagnostics/player.log".to_owned(),
      "logs/events.jsonl".to_owned()
    ]
  );

  let result_path = store
    .finalize(&mut active, result(RUN_A, ResultCommand::Run), 13)
    .unwrap();
  assert!(result_path.is_file());
  assert!(!active.path().join("partial-result.json").exists());
  assert!(!active.path().join(".terminal-pending").exists());
  assert!(!active.path().join(".lease.json").exists());
  let committed = store.load_result(RUN_A, 14).unwrap();
  assert_eq!(
    committed.artifacts,
    vec![
      "diagnostics/player.log".to_owned(),
      "logs/events.jsonl".to_owned()
    ]
  );
  let entry = &store.entries()[0];
  assert_eq!(
    entry.repository,
    Some(repository.canonicalize().unwrap().display().to_string())
  );
  assert_eq!(entry.suite.as_deref(), Some("suite"));
  assert_eq!(entry.terminal_status, Some(RunStatus::Passed));
}

#[test]
fn explicit_cleanup_preview_is_scoped_and_preserves_active_runs() {
  let temporary = TempDir::new().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir(&repository).unwrap();
  let root = temporary.path().join("runs");
  let mut store = RunStore::open(&root).unwrap();
  let mut stderr = Vec::new();
  let mut terminal = store
    .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
    .unwrap();
  store
    .index_identity(&terminal, &repository, "suite-a", 10)
    .unwrap();
  store
    .finalize(&mut terminal, result(RUN_A, ResultCommand::Run), 10)
    .unwrap();
  let active = store
    .begin(result(RUN_B, ResultCommand::Run), &mut stderr, 10)
    .unwrap();
  store
    .index_identity(&active, &repository, "suite-a", 10)
    .unwrap();
  let scope = RunCleanupScope::Suite {
    repository: repository.canonicalize().unwrap().display().to_string(),
    suite: "suite-a".to_owned(),
  };

  let preview = store.cleanup_preview(&scope, 11).unwrap();
  assert_eq!(preview.inactive.len(), 1);
  assert_eq!(preview.inactive[0].run_id, RUN_A);
  assert_eq!(preview.active, [RUN_B]);
  let mut later = store
    .begin(result(RUN_C, ResultCommand::Run), &mut stderr, 11)
    .unwrap();
  store
    .index_identity(&later, &repository, "suite-a", 11)
    .unwrap();
  store
    .finalize(&mut later, result(RUN_C, ResultCommand::Run), 11)
    .unwrap();
  let cleaned = store.cleanup_planned(&preview, 11).unwrap();
  assert_eq!(cleaned.len(), 1);
  assert!(!root.join(RUN_A).exists());
  assert!(root.join(RUN_B).is_dir());
  assert!(root.join(RUN_C).is_dir());
}

#[test]
fn expired_partial_is_recovered_as_interrupted_with_late_artifacts() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let path = {
    let mut store = RunStore::open(&root).unwrap();
    let mut stderr = Vec::new();
    let active = store
      .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
      .unwrap();
    fs::write(active.path().join("late-diagnostic.txt"), b"late").unwrap();
    active.path().to_owned()
  };

  let mut reopened = RunStore::open(&root).unwrap();
  let maintenance = reopened.maintain(100, u64::MAX).unwrap();
  assert_eq!(maintenance.recovered.len(), 1);
  assert_eq!(maintenance.recovered[0].status, RunStatus::Interrupted);
  let recovered = reopened.load_result(RUN_A, 101).unwrap();
  assert_eq!(recovered.exit_code, 130);
  assert!(recovered.errors.is_empty());
  assert!(
    recovered
      .artifacts
      .contains(&"late-diagnostic.txt".to_owned())
  );
  assert!(path.join("result.json").is_file());
  assert!(!path.join("partial-result.json").exists());
}

#[test]
fn startup_reindexes_a_durable_partial_after_an_index_write_is_lost() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  {
    let mut store = RunStore::open(&root).unwrap();
    let mut stderr = Vec::new();
    store
      .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
      .unwrap();
  }
  fs::write(root.join("index.json"), b"{\"entries\":[]}\n").unwrap();

  let mut reopened = RunStore::open(&root).unwrap();
  assert_eq!(reopened.entries().len(), 1);
  assert_eq!(reopened.entries()[0].run_id, RUN_A);
  assert_eq!(reopened.maintain(100, u64::MAX).unwrap().recovered.len(), 1);
  assert_eq!(
    reopened.load_result(RUN_A, 101).unwrap().status,
    RunStatus::Interrupted
  );
}

#[test]
fn failed_terminal_commit_is_retried_as_a_durability_failure() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let path = {
    let mut store = RunStore::open(&root).unwrap();
    let mut stderr = Vec::new();
    let mut active = store
      .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
      .unwrap();
    fs::create_dir(active.path().join("result.json")).unwrap();
    assert!(
      store
        .finalize(&mut active, result(RUN_A, ResultCommand::Run), 11)
        .is_err()
    );
    assert!(active.path().join("partial-result.json").is_file());
    assert!(active.path().join(".terminal-pending").is_file());
    assert!(temporary_files(active.path()).is_empty());
    fs::remove_dir(active.path().join("result.json")).unwrap();
    active.path().to_owned()
  };

  let mut reopened = RunStore::open(&root).unwrap();
  assert!(reopened.load_result(RUN_A, 99).is_err());
  let maintenance = reopened.maintain(100, u64::MAX).unwrap();
  assert_eq!(
    maintenance.recovered[0].status,
    RunStatus::InfrastructureError
  );
  let recovered = reopened.load_result(RUN_A, 101).unwrap();
  assert_eq!(recovered.exit_code, 2);
  assert_eq!(recovered.errors.len(), 1);
  assert_eq!(
    recovered.errors[0].code,
    ErrorCode::DurabilityResultCommitFailed
  );
  assert_eq!(recovered.errors[0].source, ErrorSource::Filesystem);
  assert!(path.join("result.json").is_file());
  assert!(!path.join(".terminal-pending").exists());
}

#[test]
fn authoritative_result_ignores_stale_recovery_files() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let path = terminal_run(&root, RUN_A, 10);
  let original = fs::read(path.join("result.json")).unwrap();
  fs::write(path.join("partial-result.json"), b"not json").unwrap();
  fs::write(
    path.join(".lease.json"),
    b"{\"owner\":\"stale\",\"expires_unix_s\":0}",
  )
  .unwrap();

  let mut reopened = RunStore::open(&root).unwrap();
  let maintenance = reopened.maintain(100, u64::MAX).unwrap();
  assert!(maintenance.recovered.is_empty());
  assert_eq!(fs::read(path.join("result.json")).unwrap(), original);
  assert!(!path.join("partial-result.json").exists());
  assert!(!path.join(".lease.json").exists());
}

#[test]
fn pending_marker_makes_even_a_present_result_commit_uncertain() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let path = terminal_run(&root, RUN_A, 10);
  fs::copy(path.join("result.json"), path.join("partial-result.json")).unwrap();
  fs::write(path.join(".terminal-pending"), b"terminal\n").unwrap();
  fs::write(
    path.join(".lease.json"),
    b"{\"owner\":\"stale\",\"expires_unix_s\":0}",
  )
  .unwrap();

  let mut reopened = RunStore::open(&root).unwrap();
  let maintenance = reopened.maintain(100, u64::MAX).unwrap();
  assert_eq!(maintenance.recovered.len(), 1);
  let recovered = reopened.load_result(RUN_A, 101).unwrap();
  assert_eq!(recovered.status, RunStatus::InfrastructureError);
  assert_eq!(
    recovered.errors[0].code,
    ErrorCode::DurabilityResultCommitFailed
  );
  assert!(!path.join("partial-result.json").exists());
  assert!(!path.join(".terminal-pending").exists());
}

#[test]
fn comparison_run_materializes_paths_without_mutating_its_source() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let mut store = RunStore::open(&root).unwrap();
  let mut stderr = Vec::new();
  let mut source = store
    .begin(result(RUN_A, ResultCommand::Run), &mut stderr, 10)
    .unwrap();
  fs::create_dir(source.path().join("actual")).unwrap();
  fs::write(source.path().join("actual/frame.png"), b"pixels").unwrap();
  fs::write(source.path().join("logs/source.jsonl"), b"event\n").unwrap();
  store
    .finalize(&mut source, result(RUN_A, ResultCommand::Run), 11)
    .unwrap();
  let before = file_contents(source.path());

  let mut derived_result = result(RUN_B, ResultCommand::ComparisonOnly);
  derived_result.source_run_id = Some(RUN_A.to_owned());
  derived_result.source_command = Some(ResultCommand::Run);
  let mut derived = store
    .begin(derived_result.clone(), &mut stderr, 20)
    .unwrap();
  store
    .materialize_derived(
      &derived,
      RUN_A,
      &[
        "actual/frame.png".to_owned(),
        "logs/source.jsonl".to_owned(),
      ],
      21,
    )
    .unwrap();
  assert_eq!(file_contents(source.path()), before);
  assert!(!source.path().join(".lease.json").exists());
  store.finalize(&mut derived, derived_result, 22).unwrap();
  let committed = store.load_result(RUN_B, 23).unwrap();
  for relative in &committed.artifacts {
    assert!(derived.path().join(relative).is_file());
  }
  assert_eq!(
    fs::read(derived.path().join("actual/frame.png")).unwrap(),
    b"pixels"
  );
}

#[test]
fn retention_expires_old_runs_then_evicts_lru_but_never_active_runs() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let old = terminal_run(&root, RUN_A, 1);
  let recent_lru = terminal_run(&root, RUN_B, RETENTION_SECONDS + 10);
  let recent = terminal_run(&root, RUN_C, RETENTION_SECONDS + 20);
  let mut store = RunStore::open(&root).unwrap();
  let mut stderr = Vec::new();
  let mut active = store
    .begin(
      result(RUN_D, ResultCommand::Run),
      &mut stderr,
      RETENTION_SECONDS + 30,
    )
    .unwrap();
  fs::write(active.path().join("oversize.bin"), vec![0_u8; 4096]).unwrap();
  store
    .checkpoint(
      &mut active,
      result(RUN_D, ResultCommand::Run),
      RETENTION_SECONDS + 30,
    )
    .unwrap();

  let retained_bytes: u64 = store
    .entries()
    .iter()
    .filter(|entry| entry.run_id == RUN_C || entry.run_id == RUN_D)
    .map(|entry| entry.artifact_bytes)
    .sum();
  let maintenance = store
    .maintain(RETENTION_SECONDS + 30, retained_bytes)
    .unwrap();
  assert_eq!(
    maintenance
      .evicted
      .iter()
      .map(|entry| entry.run_id.as_str())
      .collect::<Vec<_>>(),
    vec![RUN_A, RUN_B]
  );
  assert!(!old.exists());
  assert!(!recent_lru.exists());
  assert!(recent.exists());
  assert!(active.path().exists());
  assert!(active.path().join("oversize.bin").is_file());
  assert!(
    store
      .entries()
      .iter()
      .filter(|entry| entry.run_id == RUN_A || entry.run_id == RUN_B)
      .all(|entry| entry.artifacts_evicted && entry.artifact_bytes == 0)
  );
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
  }
}

fn terminal_run(root: &Path, run_id: &str, now_unix_s: u64) -> PathBuf {
  let mut store = RunStore::open(root).unwrap();
  let mut stderr = Vec::new();
  let mut active = store
    .begin(result(run_id, ResultCommand::Run), &mut stderr, now_unix_s)
    .unwrap();
  store
    .finalize(&mut active, result(run_id, ResultCommand::Run), now_unix_s)
    .unwrap();
  active.path().to_owned()
}

fn temporary_files(path: &Path) -> Vec<PathBuf> {
  fs::read_dir(path)
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .filter(|path| path.extension().is_some_and(|extension| extension == "tmp"))
    .collect()
}

fn file_contents(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
  let mut contents = BTreeMap::new();
  collect_file_contents(path, path, &mut contents);
  contents
}

fn collect_file_contents(root: &Path, path: &Path, contents: &mut BTreeMap<PathBuf, Vec<u8>>) {
  for entry in fs::read_dir(path).unwrap() {
    let path = entry.unwrap().path();
    if path.is_dir() {
      collect_file_contents(root, &path, contents);
    } else {
      contents.insert(
        path.strip_prefix(root).unwrap().to_owned(),
        fs::read(path).unwrap(),
      );
    }
  }
}
