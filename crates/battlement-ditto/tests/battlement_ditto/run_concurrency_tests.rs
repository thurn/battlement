use battlement_ditto::wire::{
  result::{ResultCommand, RunResult, RunStatus},
  run_storage::{RETENTION_SECONDS, RunCleanupScope, RunStore},
};
use std::{
  env, fs, io,
  path::Path,
  process::{Child, Command, Stdio},
  thread,
  time::{Duration, Instant},
};
use tempfile::TempDir;
struct Worker(Child);

impl Drop for Worker {
  fn drop(&mut self) {
    if self.0.try_wait().ok().flatten().is_none() {
      let _ = self.0.kill();
      let _ = self.0.wait();
    }
  }
}

#[test]
fn interleaved_owners_preserve_each_run_identity() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let mut first = RunStore::open(&root).unwrap();
  let mut second = RunStore::open(&root).unwrap();
  let a = "10000000-0000-4000-8000-000000000001";
  let b = "10000000-0000-4000-8000-000000000002";
  let mut active_a = first
    .begin(result(a, ResultCommand::Run), &mut Vec::new(), 10)
    .unwrap();
  let active_b = second
    .begin(result(b, ResultCommand::Run), &mut Vec::new(), 10)
    .unwrap();
  second
    .index_identity(&active_b, temporary.path(), "selected-suite", 10)
    .unwrap();
  first
    .checkpoint(&mut active_a, result(a, ResultCommand::Run), 11)
    .unwrap();
  let raw = fs::read_to_string(root.join("index.json")).unwrap();
  assert!(raw.contains(b));
  let reopened = RunStore::open(&root).unwrap();
  let lost = reopened
    .entries()
    .iter()
    .find(|entry| entry.run_id == b)
    .unwrap();
  assert_eq!(
    lost.repository.as_deref(),
    Some(temporary.path().canonicalize().unwrap().to_str().unwrap())
  );
  assert_eq!(lost.suite.as_deref(), Some("selected-suite"));
}
#[test]
fn stale_owners_preserve_newer_access_and_eviction_metadata() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join("runs");
  let a = "10000000-0000-4000-8000-000000000001";
  let b = "10000000-0000-4000-8000-000000000002";
  let mut first = RunStore::open(&root).unwrap();
  let mut active = first
    .begin(result(a, ResultCommand::Run), &mut Vec::new(), 10)
    .unwrap();
  first
    .finalize(&mut active, result(a, ResultCommand::Run), 11)
    .unwrap();
  let mut stale = RunStore::open(&root).unwrap();
  first.load_result(a, RETENTION_SECONDS + 15).unwrap();
  assert!(
    stale
      .cleanup(RETENTION_SECONDS + 20, u64::MAX)
      .unwrap()
      .is_empty()
  );
  stale.load_result(a, 20).unwrap();
  assert_eq!(
    RunStore::open(&root).unwrap().entries()[0].last_accessed_unix_s,
    RETENTION_SECONDS + 15
  );
  first
    .cleanup_scope(&RunCleanupScope::Global, RETENTION_SECONDS + 20)
    .unwrap();
  let mut other = stale
    .begin(
      result(b, ResultCommand::Run),
      &mut Vec::new(),
      RETENTION_SECONDS + 21,
    )
    .unwrap();
  stale
    .finalize(
      &mut other,
      result(b, ResultCommand::Run),
      RETENTION_SECONDS + 22,
    )
    .unwrap();
  let reopened = RunStore::open(&root).unwrap();
  let evicted = reopened
    .entries()
    .iter()
    .find(|entry| entry.run_id == a)
    .unwrap();
  assert!(evicted.artifacts_evicted);
  assert_eq!(evicted.artifact_bytes, 0);
  assert!(!root.join(a).exists());
  assert_eq!(
    reopened
      .entries()
      .iter()
      .find(|entry| entry.run_id == b)
      .unwrap()
      .terminal_status,
    Some(RunStatus::Passed)
  );
}

#[test]
fn stale_owner_cannot_reallocate_an_evicted_run_id() {
  let temporary = TempDir::new().unwrap();
  let mut stale = RunStore::open(temporary.path()).unwrap();
  let mut writer = RunStore::open(temporary.path()).unwrap();
  let id = "10000000-0000-4000-8000-000000000001";
  let mut active = writer
    .begin(result(id, ResultCommand::Run), &mut Vec::new(), 10)
    .unwrap();
  writer
    .finalize(&mut active, result(id, ResultCommand::Run), 11)
    .unwrap();
  writer.cleanup_scope(&RunCleanupScope::Global, 12).unwrap();
  assert!(
    stale
      .begin(result(id, ResultCommand::Run), &mut Vec::new(), 13)
      .is_err()
  );
  assert!(!temporary.path().join(id).exists());
}

#[test]
fn four_processes_preserve_all_runs_and_allocate_each_id_once() {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path();
  let mut workers = Vec::new();
  for number in 0..4 {
    let log = fs::File::create(root.join(format!("worker-{number}.log"))).unwrap();
    workers.push(Worker(
      Command::new(env::current_exe().unwrap())
        .args([
          "--exact",
          "run_concurrency_tests::catalog_worker",
          "--nocapture",
        ])
        .env("BATTLEMENT_CATALOG_TEST_ROOT", root)
        .env("BATTLEMENT_CATALOG_TEST_WORKER", number.to_string())
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap(),
    ));
  }
  wait_for(|| (0..4).all(|number| root.join(format!("ready-{number}")).exists()));
  fs::write(root.join("go"), b"go").unwrap();
  for (number, worker) in workers.iter_mut().enumerate() {
    wait_for(|| worker.0.try_wait().unwrap().is_some());
    assert!(
      worker.0.wait().unwrap().success(),
      "{}",
      fs::read_to_string(root.join(format!("worker-{number}.log"))).unwrap()
    );
  }
  let index: serde_json::Value =
    serde_json::from_slice(&fs::read(root.join("runs/index.json")).unwrap()).unwrap();
  assert_eq!(index["entries"].as_array().unwrap().len(), 17);
  let store = RunStore::open(root.join("runs")).unwrap();
  for entry in store.entries() {
    assert_eq!(
      entry.repository.as_deref(),
      Some(root.canonicalize().unwrap().to_str().unwrap())
    );
    assert!(entry.suite.is_some());
    assert_eq!(entry.terminal_status, Some(RunStatus::Passed));
    assert!(!entry.artifacts_evicted);
    assert!(entry.artifact_bytes > 0);
  }
}

#[test]
fn catalog_worker() {
  let Some(root) = env::var_os("BATTLEMENT_CATALOG_TEST_ROOT") else {
    return;
  };
  let root = Path::new(&root);
  let worker: usize = env::var("BATTLEMENT_CATALOG_TEST_WORKER")
    .unwrap()
    .parse()
    .unwrap();
  let mut store = RunStore::open(root.join("runs")).unwrap();
  fs::write(root.join(format!("ready-{worker}")), b"ready").unwrap();
  wait_for(|| root.join("go").exists());
  let common = "30000000-0000-4000-8000-000000000000";
  match store.begin(result(common, ResultCommand::Run), &mut Vec::new(), 10) {
    Ok(mut active) => {
      store.index_identity(&active, root, "common", 10).unwrap();
      store
        .finalize(&mut active, result(common, ResultCommand::Run), 11)
        .unwrap();
    }
    Err(error) => assert!(
      error.to_string() == "run ID is already indexed"
        || error
          .downcast_ref::<io::Error>()
          .is_some_and(|error| error.kind() == io::ErrorKind::AlreadyExists)
    ),
  }
  for number in 0..4 {
    let id = format!("40000000-0000-4000-8000-{:012}", worker * 4 + number);
    let mut active = store
      .begin(result(&id, ResultCommand::Run), &mut Vec::new(), 20)
      .unwrap();
    store
      .index_identity(&active, root, &format!("worker-{worker}"), 21)
      .unwrap();
    store
      .checkpoint(&mut active, result(&id, ResultCommand::Run), 22)
      .unwrap();
    store
      .finalize(&mut active, result(&id, ResultCommand::Run), 23)
      .unwrap();
  }
}

fn wait_for(mut ready: impl FnMut() -> bool) {
  let deadline = Instant::now() + Duration::from_secs(30);
  while !ready() {
    assert!(Instant::now() < deadline, "catalog worker did not progress");
    thread::sleep(Duration::from_millis(5));
  }
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
