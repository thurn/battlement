use std::fs;

use battlement_ditto::{
  config::{
    self,
    model::{Baseline, Suite},
  },
  review_acceptance::ReviewAcceptanceService,
  wire::{
    common::{ErrorCode, StepName, StepStatus},
    job::Motion,
    lifecycle::HttpError,
    result::{
      BaselineOutcome, BaselineWriteStatus, ImageFile, Recovery, ResultCommand, RunResult,
      RunStatus, ScenarioResult, ScenarioStatus, ScenarioTimings, ScreenshotResult, StepResult,
    },
    review::{ReviewAcceptance, ReviewAcceptanceResult, ReviewSelection},
    run_storage::RunStore,
  },
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SOURCE_RUN: &str = "519a7d47-2aa4-43c2-b493-b1b8fd284b58";

#[test]
fn selective_acceptance_is_atomic_idempotent_and_retains_stale_attempts() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let runs = temporary.path().join("runs");
  fs::create_dir(&repository).unwrap();
  let suite = suite(&repository, temporary.path().join("baselines"));
  let reviewed = store_reviewed_run(&runs, &repository, &suite.name);
  let directory = runs.join(SOURCE_RUN);
  let mut service =
    ReviewAcceptanceService::open(suite, &runs, reviewed.clone(), directory).unwrap();
  let acceptance_request = request(&reviewed, None);
  let bytes = serde_json::to_vec(&acceptance_request).unwrap();

  let accepted = service.accept(&bytes);
  assert_eq!(accepted.status, 200);
  let response: ReviewAcceptanceResult = serde_json::from_slice(&accepted.body).unwrap();
  let lock_before_stale = fs::read(repository.join("ditto.lock")).unwrap();
  let mut store = RunStore::open(&runs).unwrap();
  let derived = store.load_result(&response.comparison_run_id, 30).unwrap();
  assert_eq!(derived.source_run_id.as_deref(), Some(SOURCE_RUN));
  assert_eq!(derived.cycle, reviewed.cycle + 1);
  assert_eq!(derived.status, RunStatus::Passed);
  assert_eq!(derived.baseline_writes.len(), 2);
  assert!(
    derived
      .baseline_writes
      .iter()
      .all(|write| write.status == BaselineWriteStatus::Published)
  );
  assert_eq!(store.load_result(SOURCE_RUN, 31).unwrap(), reviewed);

  let replay = service.accept(&bytes);
  assert_eq!(replay.status, 200);
  assert_eq!(replay.body, accepted.body);
  assert_eq!(RunStore::open(&runs).unwrap().entries().len(), 2);

  let mut conflicting = acceptance_request.clone();
  conflicting.selections.reverse();
  let conflict = service.accept(&serde_json::to_vec(&conflicting).unwrap());
  assert_eq!(conflict.status, 409);
  assert_eq!(
    serde_json::from_slice::<HttpError>(&conflict.body)
      .unwrap()
      .code,
    ErrorCode::BaselineStoreConflict
  );

  let mut stale = request(&derived, None);
  stale.request_id = Uuid::new_v4().to_string();
  let stale_reply = service.accept(&serde_json::to_vec(&stale).unwrap());
  assert_eq!(stale_reply.status, 409);
  let stale_error: HttpError = serde_json::from_slice(&stale_reply.body).unwrap();
  assert_eq!(stale_error.code, ErrorCode::BaselineLockStale);
  assert_eq!(
    fs::read(repository.join("ditto.lock")).unwrap(),
    lock_before_stale
  );
  let mut reopened = RunStore::open(&runs).unwrap();
  let attempt = reopened
    .load_result(stale_error.related_run_id.as_deref().unwrap(), 32)
    .unwrap();
  assert_eq!(attempt.status, RunStatus::InfrastructureError);
}

#[test]
fn replaying_an_older_success_does_not_switch_back_from_a_newer_run() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let runs = temporary.path().join("runs");
  fs::create_dir(&repository).unwrap();
  let suite = suite(&repository, temporary.path().join("baselines"));
  let reviewed = store_reviewed_run(&runs, &repository, &suite.name);
  let mut service =
    ReviewAcceptanceService::open(suite, &runs, reviewed.clone(), runs.join(SOURCE_RUN)).unwrap();
  let mut first = request(&reviewed, None);
  first.selections.truncate(1);
  let first_bytes = serde_json::to_vec(&first).unwrap();
  let first_reply = service.accept(&first_bytes);
  let first_response: ReviewAcceptanceResult = serde_json::from_slice(&first_reply.body).unwrap();
  let mut store = RunStore::open(&runs).unwrap();
  let first_run = store
    .load_result(&first_response.comparison_run_id, 60)
    .unwrap();

  let mut second = request(&first_run, first_run.lock_sha256.clone());
  second.selections.remove(0);
  let second_reply = service.accept(&serde_json::to_vec(&second).unwrap());
  assert_eq!(second_reply.status, 200);
  let second_response: ReviewAcceptanceResult = serde_json::from_slice(&second_reply.body).unwrap();
  assert_ne!(
    second_response.comparison_run_id,
    first_response.comparison_run_id
  );
  assert_eq!(
    RunStore::open(&runs)
      .unwrap()
      .load_result(&second_response.comparison_run_id, 61)
      .unwrap()
      .cycle,
    first_run.cycle + 1
  );

  let replay = service.accept(&first_bytes);
  assert_eq!(replay.body, first_reply.body);
  assert!(replay.replacement.is_none());
}

#[test]
fn changed_actual_and_invalid_fragment_checkpoint_leave_attempts_without_a_lock() {
  for failure in ["changed-actual", "fragment-checkpoint"] {
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("repository");
    let runs = temporary.path().join("runs");
    fs::create_dir(&repository).unwrap();
    let mut suite = suite(&repository, temporary.path().join("baselines"));
    let reviewed = store_reviewed_run(&runs, &repository, &suite.name);
    let mut acceptance = request(&reviewed, None);
    acceptance.selections.truncate(1);
    if failure == "changed-actual" {
      fs::write(runs.join(SOURCE_RUN).join("actuals/0.png"), png(0x44)).unwrap();
    } else {
      suite.scenarios[0].steps.remove(0);
    }
    let mut service =
      ReviewAcceptanceService::open(suite, &runs, reviewed, runs.join(SOURCE_RUN)).unwrap();

    let reply = service.accept(&serde_json::to_vec(&acceptance).unwrap());
    assert_eq!(reply.status, 422, "{failure}");
    let error: HttpError = serde_json::from_slice(&reply.body).unwrap();
    assert!(error.related_run_id.is_some(), "{failure}");
    assert!(!repository.join("ditto.lock").exists(), "{failure}");
    let mut store = RunStore::open(&runs).unwrap();
    assert_eq!(
      store
        .load_result(error.related_run_id.as_deref().unwrap(), 40)
        .unwrap()
        .status,
      RunStatus::InfrastructureError,
      "{failure}"
    );
  }
}

#[test]
fn failed_upload_leaves_the_lock_absent_and_retains_proposals() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let runs = temporary.path().join("runs");
  fs::create_dir(&repository).unwrap();
  let mut suite = suite(&repository, temporary.path().join("baselines"));
  let blocked = temporary.path().join("blocked-store");
  fs::write(&blocked, "not a directory").unwrap();
  suite.baseline = Some(Baseline::Filesystem {
    namespace: "review".to_owned(),
    root: blocked,
  });
  let reviewed = store_reviewed_run(&runs, &repository, &suite.name);
  let acceptance = request(&reviewed, None);
  let mut service =
    ReviewAcceptanceService::open(suite, &runs, reviewed, runs.join(SOURCE_RUN)).unwrap();

  let reply = service.accept(&serde_json::to_vec(&acceptance).unwrap());
  assert_eq!(reply.status, 500);
  let error: HttpError = serde_json::from_slice(&reply.body).unwrap();
  assert_eq!(error.code, ErrorCode::BaselinePublishFailed);
  assert!(!repository.join("ditto.lock").exists());
  let mut store = RunStore::open(&runs).unwrap();
  let attempt = store
    .load_result(error.related_run_id.as_deref().unwrap(), 50)
    .unwrap();
  assert_eq!(attempt.status, RunStatus::InfrastructureError);
  assert_eq!(attempt.baseline_writes.len(), 2);
  assert!(
    attempt
      .baseline_writes
      .iter()
      .all(|write| write.status == BaselineWriteStatus::Proposed)
  );
}

fn store_reviewed_run(
  runs: &std::path::Path,
  repository: &std::path::Path,
  suite: &str,
) -> RunResult {
  let mut store = RunStore::open(runs).unwrap();
  let mut progress = Vec::new();
  let mut active = store.begin(empty_result(), &mut progress, 10).unwrap();
  store
    .index_identity(&active, repository, suite, 10)
    .unwrap();
  fs::create_dir_all(active.path().join("actuals")).unwrap();
  let images = [png(0x22), png(0x99)];
  for (index, image) in images.iter().enumerate() {
    fs::write(active.path().join(format!("actuals/{index}.png")), image).unwrap();
  }
  let result = reviewed_result(&images);
  store.finalize(&mut active, result.clone(), 11).unwrap();
  store.load_result(SOURCE_RUN, 12).unwrap()
}

fn empty_result() -> RunResult {
  RunResult {
    run_id: SOURCE_RUN.to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command: ResultCommand::Run,
    source_command: None,
    cycle: 1,
    suite: Some("review suite".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-29T10:00:00Z".to_owned(),
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

fn reviewed_result(images: &[Vec<u8>; 2]) -> RunResult {
  let steps = images
    .iter()
    .enumerate()
    .map(|(index, image)| StepResult {
      index: index as u32,
      name: None,
      kind: StepName::Screenshot,
      status: StepStatus::Failed,
      status_reason: None,
      duration_ms: 1,
      expired_deadline: None,
      error_ids: vec![],
      assertion: None,
      screenshot: Some(ScreenshotResult::Captured {
        checkpoint: format!("checkpoint-{index}"),
        actual: ImageFile {
          path: format!("actuals/{index}.png"),
          sha256: format!("{:x}", Sha256::digest(image)),
          width: 1,
          height: 1,
        },
        baseline: BaselineOutcome::Missing,
        comparison: None,
        matched_before_update: None,
        updated: None,
      }),
      video: None,
    })
    .collect();
  RunResult {
    duration_ms: 2,
    status: RunStatus::Failed,
    exit_code: 1,
    scenarios: vec![ScenarioResult {
      id: Uuid::new_v4().to_string(),
      name: "scenario".to_owned(),
      status: ScenarioStatus::Failed,
      status_reason: None,
      motion: Motion::Instant,
      duration_ms: 2,
      expired_deadline: None,
      timings: ScenarioTimings::default(),
      steps,
      logs: None,
      failure_frame: None,
      recovery: Recovery::None,
    }],
    artifacts: vec!["actuals/0.png".to_owned(), "actuals/1.png".to_owned()],
    ..empty_result()
  }
}

fn request(result: &RunResult, lock_sha256: Option<String>) -> ReviewAcceptance {
  ReviewAcceptance {
    request_id: Uuid::new_v4().to_string(),
    run_id: result.run_id.clone(),
    lock_sha256,
    selections: result.scenarios[0]
      .steps
      .iter()
      .map(|step| {
        let ScreenshotResult::Captured {
          checkpoint, actual, ..
        } = step.screenshot.as_ref().unwrap()
        else {
          unreachable!()
        };
        ReviewSelection {
          profile: "macos-local".to_owned(),
          scenario: "scenario".to_owned(),
          checkpoint: checkpoint.clone(),
          width: actual.width,
          height: actual.height,
          actual_sha256: actual.sha256.clone(),
        }
      })
      .collect(),
  }
}

fn suite(repository: &std::path::Path, baseline_root: std::path::PathBuf) -> Suite {
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("rules")).unwrap();
  fs::write(repository.join("Assets/Scenes/Game.unity"), "").unwrap();
  fs::write(
    repository.join("rules/Cargo.toml"),
    "[package]\nname='rules'\n",
  )
  .unwrap();
  assert!(
    std::process::Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(repository)
      .status()
      .unwrap()
      .success()
  );
  let source = format!(
    r#"name = "review suite"
default_profile = "macos-local"

[defaults]
step_timeout = "1s"
scenario_timeout = "2s"

[defaults.comparison]
threshold = 0.1
max_changed_percent = 0

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[baseline]
kind = "filesystem"
namespace = "review"
root = {baseline_root:?}

[profiles.macos-local]
target = "macos"
display = {{ width = 1, height = 1, scale = 1.0 }}

[[scenarios]]
name = "scenario"

[[scenarios.steps]]
screenshot = {{ name = "checkpoint-0" }}

[[scenarios.steps]]
screenshot = {{ name = "checkpoint-1" }}
"#,
  );
  let path = repository.join("ditto.toml");
  fs::write(&path, source).unwrap();
  config::load(Some(&path)).unwrap()
}

fn png(value: u8) -> Vec<u8> {
  let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01".to_vec();
  bytes.push(value);
  bytes
}
