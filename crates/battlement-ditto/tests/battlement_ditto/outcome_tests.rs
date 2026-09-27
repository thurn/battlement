use battlement_ditto::wire::{
  common::{DeadlineKind, ErrorCode, ErrorSource},
  outcome::{ErrorContext, ErrorDraft, FailureImpact, RunOutcome},
  player_errors::{PlayerErrorMapper, PlayerErrorObservation},
  result::{PhaseName, PhaseResult, PhaseStatus},
};
use serde_json::json;

#[test]
fn stable_terminal_excerpts_cover_every_status_and_precedence() {
  let mut outcome = RunOutcome::default();
  assert_eq!(
    excerpt(&outcome),
    r#"{"errors":[],"exit_code":0,"primary_error_id":null,"status":"passed"}"#
  );

  let functional = outcome
    .record_host("image", image_mismatch(), FailureImpact::Functional)
    .unwrap();
  assert_eq!(functional, "E0001");
  assert_eq!(
    excerpt(&outcome),
    r#"{"errors":["E0001"],"exit_code":1,"primary_error_id":"E0001","status":"failed"}"#
  );

  let durability = outcome
    .record_host(
      "result-commit",
      durability_failure(),
      FailureImpact::SecondaryInfrastructure,
    )
    .unwrap();
  assert_eq!(durability, "E0002");
  assert_eq!(
    excerpt(&outcome),
    r#"{"errors":["E0001","E0002"],"exit_code":2,"primary_error_id":"E0001","status":"infrastructure-error"}"#
  );

  outcome.mark_interrupted();
  assert_eq!(
    excerpt(&outcome),
    r#"{"errors":["E0001","E0002"],"exit_code":130,"primary_error_id":"E0001","status":"interrupted"}"#
  );
}

#[test]
fn direct_infrastructure_failure_is_primary() {
  let mut outcome = RunOutcome::default();
  let error_id = outcome
    .record_host("launch", launch_failure(), FailureImpact::Infrastructure)
    .unwrap();
  assert_eq!(error_id, "E0001");
  assert_eq!(outcome.primary_error_id(), Some("E0001"));
  assert_eq!(outcome.exit_code(), 2);
}

#[test]
fn player_completion_replay_reuses_one_host_occurrence() {
  let mut player = PlayerErrorMapper::default();
  let observation = unity_exception();
  let player_ref = player.observe("unity-entry", observation.clone()).unwrap();
  assert_eq!(player_ref, "P0001");
  assert_eq!(
    player.observe("same-entry", observation.clone()).unwrap(),
    "P0001"
  );

  let mut outcome = RunOutcome::default();
  let first = outcome
    .record_player(
      SCENARIO_ID,
      &player_ref,
      &observation,
      player_context(),
      FailureImpact::Functional,
    )
    .unwrap();
  let replay = outcome
    .record_player(
      SCENARIO_ID,
      &player_ref,
      &observation,
      player_context(),
      FailureImpact::Functional,
    )
    .unwrap();
  assert_eq!(first, "E0001");
  assert_eq!(replay, first);
  assert_eq!(outcome.errors().len(), 1);
  assert_eq!(outcome.errors()[0].log_sequence, Some(7));
}

#[test]
fn player_and_host_namespaces_share_only_the_host_allocator() {
  let mut player = PlayerErrorMapper::default();
  let observation = unity_exception();
  let player_ref = player.observe("unity-entry", observation.clone()).unwrap();

  let mut outcome = RunOutcome::default();
  assert_eq!(
    outcome
      .record_host("build", launch_failure(), FailureImpact::Infrastructure)
      .unwrap(),
    "E0001"
  );
  assert_eq!(
    outcome
      .record_player(
        SCENARIO_ID,
        &player_ref,
        &observation,
        player_context(),
        FailureImpact::Functional,
      )
      .unwrap(),
    "E0002"
  );
  assert_eq!(player_ref, "P0001");
}

#[test]
fn scenario_local_player_references_do_not_alias_between_scenarios() {
  let observation = unity_exception();
  let mut outcome = RunOutcome::default();
  let first = outcome
    .record_player(
      SCENARIO_ID,
      "P0001",
      &observation,
      player_context(),
      FailureImpact::Functional,
    )
    .unwrap();
  let second = outcome
    .record_player(
      OTHER_SCENARIO_ID,
      "P0001",
      &observation,
      ErrorContext {
        scenario_id: Some(OTHER_SCENARIO_ID.to_owned()),
        ..player_context()
      },
      FailureImpact::Functional,
    )
    .unwrap();
  assert_eq!(first, "E0001");
  assert_eq!(second, "E0002");
}

#[test]
fn caught_failure_envelopes_never_allocate_another_player_reference() {
  let mut player = PlayerErrorMapper::default();
  let mut original = unity_exception();
  original.battlement_error_id = Some("battlement-error-1".to_owned());
  assert_eq!(player.observe("native", original).unwrap(), "P0001");
  player
    .suppress_caught_failure("battlement-error-1")
    .unwrap();
  assert!(player.suppress_caught_failure("unknown").is_err());

  let without_record = PlayerErrorObservation {
    code: ErrorCode::RuntimeFatal,
    source: ErrorSource::Rust,
    message: "fatal boundary".to_owned(),
    record_sequence: None,
    battlement_error_id: None,
  };
  assert_eq!(
    player.observe("boundary", without_record.clone()).unwrap(),
    "P0002"
  );
  assert_eq!(player.observe("boundary", without_record).unwrap(), "P0002");
}

#[test]
fn idempotency_conflicts_are_rejected_without_allocating() {
  let mut outcome = RunOutcome::default();
  outcome
    .record_host("startup", launch_failure(), FailureImpact::Infrastructure)
    .unwrap();
  assert!(
    outcome
      .record_host(
        "startup",
        durability_failure(),
        FailureImpact::Infrastructure
      )
      .is_err()
  );
  assert_eq!(outcome.errors().len(), 1);

  let mut player = PlayerErrorMapper::default();
  player.observe("entry", unity_exception()).unwrap();
  let mut changed = unity_exception();
  changed.message = "changed replay".to_owned();
  assert!(player.observe("entry", changed).is_err());
  assert!(player.observation("P0002").is_none());
}

#[test]
fn phases_require_existing_errors_and_matching_deadlines() {
  let mut outcome = RunOutcome::default();
  let error_id = outcome
    .record_host("build", launch_failure(), FailureImpact::Infrastructure)
    .unwrap();
  outcome
    .record_phase(PhaseResult {
      name: PhaseName::Build,
      status: PhaseStatus::Failed,
      duration_ms: 50,
      expired_deadline: Some(DeadlineKind::Build),
      log_path: Some("logs/build.log".to_owned()),
      error_ids: vec![error_id],
    })
    .unwrap();
  assert_eq!(outcome.phases().len(), 1);

  assert!(
    outcome
      .record_phase(PhaseResult {
        name: PhaseName::Build,
        status: PhaseStatus::Failed,
        duration_ms: 1,
        expired_deadline: Some(DeadlineKind::Launch),
        log_path: None,
        error_ids: vec!["E0001".to_owned()],
      })
      .is_err()
  );
  assert!(
    outcome
      .record_phase(PhaseResult {
        name: PhaseName::Cleanup,
        status: PhaseStatus::Passed,
        duration_ms: 1,
        expired_deadline: None,
        log_path: None,
        error_ids: vec!["E0001".to_owned()],
      })
      .is_err()
  );
  assert_eq!(outcome.phases().len(), 1);
}

fn excerpt(outcome: &RunOutcome) -> String {
  serde_json::to_string(&json!({
    "status": outcome.status(),
    "exit_code": outcome.exit_code(),
    "primary_error_id": outcome.primary_error_id(),
    "errors": outcome.errors().iter().map(|error| &error.id).collect::<Vec<_>>(),
  }))
  .unwrap()
}

fn image_mismatch() -> ErrorDraft {
  ErrorDraft {
    code: ErrorCode::ImageMismatch,
    source: ErrorSource::ODiff,
    message: "image mismatch".to_owned(),
    context: ErrorContext {
      scenario_id: Some(SCENARIO_ID.to_owned()),
      step_index: Some(1),
      ..ErrorContext::default()
    },
  }
}

fn launch_failure() -> ErrorDraft {
  ErrorDraft {
    code: ErrorCode::LaunchFailed,
    source: ErrorSource::Ditto,
    message: "launch failed".to_owned(),
    context: ErrorContext::default(),
  }
}

fn durability_failure() -> ErrorDraft {
  ErrorDraft {
    code: ErrorCode::DurabilityResultCommitFailed,
    source: ErrorSource::Filesystem,
    message: "result commit failed".to_owned(),
    context: ErrorContext::default(),
  }
}

fn unity_exception() -> PlayerErrorObservation {
  PlayerErrorObservation {
    code: ErrorCode::RuntimeUnityException,
    source: ErrorSource::Unity,
    message: "ordinary Unity exception".to_owned(),
    record_sequence: Some(7),
    battlement_error_id: None,
  }
}

fn player_context() -> ErrorContext {
  ErrorContext {
    job_id: Some(JOB_ID.to_owned()),
    player_session_id: Some(PLAYER_SESSION_ID.to_owned()),
    scenario_id: Some(SCENARIO_ID.to_owned()),
    step_index: Some(0),
    log_sequence: Some(7),
  }
}

const JOB_ID: &str = "0197b35f-6c59-7b98-b1f0-a39f5ee54db8";
const PLAYER_SESSION_ID: &str = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";
const SCENARIO_ID: &str = "0197b35f-6e24-75d8-9482-aa6c22a15133";
const OTHER_SCENARIO_ID: &str = "0197b35f-6f36-76e9-a593-bb7d33b26244";
