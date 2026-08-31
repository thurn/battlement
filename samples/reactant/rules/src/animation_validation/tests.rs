use crate::animation_validation::model::FixtureCheckpoint;
use crate::animation_validation::runner::{fixture_metadata, fixture_observation};
use crate::animation_validation::{
  CaseId, CheckpointId, FixtureAction, FixtureSession, ReducedMotionOverride, ScreenId, Tolerance,
  fixture_registry, run_fixture_case,
};

#[test]
fn registry_and_fast_selector_share_valid_case_identity() {
  let registry = fixture_registry();
  registry.validate().unwrap();
  let case = registry
    .select(
      ScreenId("validation-infrastructure"),
      CaseId("static-presentation"),
    )
    .unwrap();
  assert_eq!(case.checkpoints[0].id, CheckpointId("captured"));
}

#[test]
fn passing_probe_produces_machine_and_human_evidence() {
  let registry = fixture_registry();
  let case = registry
    .select(
      ScreenId("validation-infrastructure"),
      CaseId("static-presentation"),
    )
    .unwrap();
  let report = run_fixture_case(case, fixture_metadata(), fixture_observation);
  assert!(report.passed());
  assert!(report.concise().contains("PASS"));
  let json = report.json();
  assert!(json.contains("\"clock_quantum_micros\": 1000"));
  assert!(json.contains("\"renderer\": \"static-probe\""));
}

#[test]
fn permanent_negative_self_test_explains_the_wrong_value() {
  let registry = fixture_registry();
  let case = registry
    .select(
      ScreenId("validation-infrastructure"),
      CaseId("wrong-expectation"),
    )
    .unwrap();
  let report = run_fixture_case(case, fixture_metadata(), fixture_observation);
  assert!(!report.passed());
  assert_eq!(report.checkpoints[0].failures.len(), 1);
  assert_eq!(
    report.checkpoints[0].failures[0],
    "scalar expected 99.000000 ± 0.000000, observed 42.000000"
  );
}

#[test]
fn malformed_case_missing_checkpoint_and_unexplained_tolerance_are_rejected() {
  let mut registry = fixture_registry();
  registry.cases[0].id = CaseId("Not Valid");
  assert!(
    registry
      .validate()
      .unwrap_err()
      .contains("invalid case identity")
  );

  let mut registry = fixture_registry();
  registry.cases[0].checkpoints.clear();
  assert!(
    registry
      .validate()
      .unwrap_err()
      .contains("has no checkpoints")
  );

  let mut registry = fixture_registry();
  registry.cases[0].checkpoints[0].expected.scalar_tolerance = Tolerance::new(0.1, "");
  assert!(
    registry
      .validate()
      .unwrap_err()
      .contains("unexplained tolerance")
  );

  let duplicate = registry.cases[1].checkpoints[0].clone();
  let mut registry = fixture_registry();
  registry.cases[0].checkpoints.push(duplicate);
  assert!(
    registry
      .validate()
      .unwrap_err()
      .contains("duplicate checkpoint")
  );
}

#[test]
fn shared_control_path_dispatches_every_fixture_action() {
  let mut session = FixtureSession::default();
  for action in [
    FixtureAction::Trigger,
    FixtureAction::Play,
    FixtureAction::Pause,
    FixtureAction::Replay,
    FixtureAction::Speed(2.0),
    FixtureAction::ReducedMotion(ReducedMotionOverride::Always),
    FixtureAction::Reconnect,
  ] {
    session.dispatch(action);
  }
  session.seek(250_000);
  assert_eq!(session.actions().len(), 7);
  assert_eq!(session.elapsed_micros(), 250_000);
  assert!(session.playing());
  assert_eq!(session.speed(), 2.0);
  assert_eq!(session.reduced_motion(), ReducedMotionOverride::Always);
  assert_eq!(session.generation(), 2);
  assert_eq!(session.reconnects(), 1);
}

#[test]
fn missing_observation_is_reported_at_the_named_checkpoint() {
  let registry = fixture_registry();
  let case = registry
    .select(
      ScreenId("validation-infrastructure"),
      CaseId("static-presentation"),
    )
    .unwrap();
  let report = run_fixture_case(case, fixture_metadata(), |_: &FixtureCheckpoint| {
    let mut observation = fixture_observation(&case.checkpoints[0]);
    observation.geometry = None;
    observation
  });
  assert_eq!(report.checkpoints[0].checkpoint, CheckpointId("captured"));
  assert!(
    report.checkpoints[0]
      .failures
      .contains(&"geometry observation is missing".to_owned())
  );
}
