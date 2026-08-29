use std::fmt::Debug;

use battlement_ditto::wire::{
  common::{DeadlineKind, ErrorCode, ErrorSource, StepName, StepStatus},
  job::{Capability, Job},
  lifecycle::{
    AcceptedPlayerSessionIdentity, ArtifactAck, ArtifactKind, BoundaryStage, DittoContext,
    DittoEventRecord, DittoLogSeverity, DittoLogSource, ExecutionStatus, HttpError, JobComplete,
    JobCompleteAck, JobFailed, JobFailedAck, LogBatchAck, NextAction, PlayerFailureFrame,
    PlayerInfrastructureFailure, ScenarioBoundaryOutcome, ScenarioComplete, ScenarioDecision,
    Started, StartupIdentity, TerminalReason, decode_ndjson,
  },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

const SHARED_LIFECYCLE_FIXTURE: &str = include_str!(
  "../../../Packages/com.battlement.client/Tests/Fixtures/Ditto/lifecycle-contract.json"
);

#[test]
fn shared_csharp_lifecycle_fixture_has_matching_acceptance() {
  let fixture: Value = serde_json::from_str(SHARED_LIFECYCLE_FIXTURE).unwrap();
  let job: Job = serde_json::from_value(fixture["job"].clone()).unwrap();
  job.validate().unwrap();

  let started: Started = serde_json::from_value(fixture["started"].clone()).unwrap();
  started.validate(&job, PLAYER_SESSION_ID, None).unwrap();
  let completion: ScenarioComplete =
    serde_json::from_value(fixture["scenario_complete"].clone()).unwrap();
  completion.validate(&job, &["P0001".to_owned()]).unwrap();
  let ndjson = shared_ndjson(&fixture["events"]);
  decode_ndjson(&ndjson, &job, PLAYER_SESSION_ID, 81).unwrap();

  for case in fixture["invalid"].as_array().unwrap() {
    let target = case["target"].as_str().unwrap();
    let mut changed = fixture[target].clone();
    mutate(
      &mut changed,
      case["pointer"].as_str().unwrap(),
      case["value"].clone(),
    );
    let accepted = match target {
      "started" => serde_json::from_value::<Started>(changed)
        .is_ok_and(|value| value.validate(&job, PLAYER_SESSION_ID, None).is_ok()),
      "scenario_complete" => serde_json::from_value::<ScenarioComplete>(changed)
        .is_ok_and(|value| value.validate(&job, &["P0001".to_owned()]).is_ok()),
      "events" => decode_ndjson(&shared_ndjson(&changed), &job, PLAYER_SESSION_ID, 81).is_ok(),
      _ => unreachable!(),
    };
    assert!(!accepted, "{} unexpectedly validated", case["name"]);
  }

  for context in fixture["contexts"].as_array().unwrap() {
    value_round_trip::<DittoContext>(context.clone());
  }
}

#[test]
fn complete_cold_lifecycle_exchange_validates() {
  let job = job();
  let started: Started = serde_json::from_str(STARTED).unwrap();
  started.validate(&job, PLAYER_SESSION_ID, None).unwrap();

  let log_ack = LogBatchAck {
    player_session_id: PLAYER_SESSION_ID.to_owned(),
    next_sequence: 83,
  };
  log_ack.validate(PLAYER_SESSION_ID, 83).unwrap();
  let artifact_ack = ArtifactAck {
    artifact_id: SCREENSHOT_ID.to_owned(),
    sha256: HASH_A.to_owned(),
  };
  artifact_ack.validate(SCREENSHOT_ID, HASH_A).unwrap();

  let completion = completion();
  completion.validate(&job, &["P0001".to_owned()]).unwrap();
  ScenarioDecision {
    action: NextAction::Continue,
    completed_failures: 1,
    error_id: None,
    error_code: None,
    message: None,
  }
  .validate()
  .unwrap();

  let complete: JobComplete = serde_json::from_value(json!({
    "job_id": JOB_ID,
    "last_log_sequence": 90,
    "executed_scenario_ids": [SCENARIO_ID],
    "unstarted_scenarios": [],
    "reason": "completed",
    "execution_duration_ms": 12
  }))
  .unwrap();
  complete.validate(&job).unwrap();
  JobCompleteAck {
    job_id: JOB_ID.to_owned(),
  }
  .validate(JOB_ID)
  .unwrap();
}

#[test]
fn warm_start_and_terminal_failure_validate() {
  let job = job();
  let mut started: Started = serde_json::from_str(STARTED).unwrap();
  started.first_log_sequence = None;
  started.startup_log_failure = Some(PlayerInfrastructureFailure {
    code: ErrorCode::TransportLogBufferOverflow,
    message: "log queue overflow".to_owned(),
  });
  started.identity = StartupIdentity::Accepted(AcceptedPlayerSessionIdentity {
    accepted_player_session_id: PLAYER_SESSION_ID.to_owned(),
  });
  started
    .validate(&job, PLAYER_SESSION_ID, Some(PLAYER_SESSION_ID))
    .unwrap();

  let failed: JobFailed = serde_json::from_value(json!({
    "job_id": JOB_ID,
    "failure": {"code":"runtime.process-exit","message":"player exited"},
    "last_log_sequence": null,
    "executed_scenario_ids": [],
    "unstarted_scenarios": [{"scenario_id":SCENARIO_ID,"reason":"run-infrastructure-error"}]
  }))
  .unwrap();
  failed.validate(&job).unwrap();
  JobFailedAck {
    job_id: JOB_ID.to_owned(),
    error_id: "E0001".to_owned(),
  }
  .validate(JOB_ID)
  .unwrap();
}

#[test]
fn exact_mixed_ndjson_body_preserves_contiguous_owned_records() {
  let records = decode_ndjson(NDJSON.as_bytes(), &job(), PLAYER_SESSION_ID, 81).unwrap();
  assert_eq!(records.len(), 2);
  assert!(matches!(records[0], DittoEventRecord::Log(_)));
  assert!(matches!(records[1], DittoEventRecord::Context(_)));
  assert_eq!(NDJSON.as_bytes().last(), Some(&b'\n'));
  let exact_lines: Vec<&str> = NDJSON.strip_suffix('\n').unwrap().split('\n').collect();
  assert_eq!(serde_json::to_string(&records[0]).unwrap(), exact_lines[0]);
  assert_eq!(serde_json::to_string(&records[1]).unwrap(), exact_lines[1]);
}

#[test]
fn every_lifecycle_and_shared_enum_variant_round_trips() {
  round_trip(&[
    ExecutionStatus::Passed,
    ExecutionStatus::Failed,
    ExecutionStatus::Interrupted,
  ]);
  round_trip(&[BoundaryStage::Destroy, BoundaryStage::Reset]);
  round_trip(&[NextAction::Continue, NextAction::Stop, NextAction::Relaunch]);
  round_trip(&[
    TerminalReason::Completed,
    TerminalReason::Bail,
    TerminalReason::InfrastructureError,
    TerminalReason::Interrupted,
  ]);
  round_trip(&[
    DittoLogSource::Battlement,
    DittoLogSource::Rust,
    DittoLogSource::Unity,
    DittoLogSource::DittoPlayer,
  ]);
  round_trip(&[
    DittoLogSeverity::Trace,
    DittoLogSeverity::Debug,
    DittoLogSeverity::Information,
    DittoLogSeverity::Warning,
    DittoLogSeverity::Error,
  ]);
  for values in [
    &[
      "click",
      "hover",
      "drag",
      "key",
      "wait",
      "assert",
      "screenshot",
      "video",
    ][..],
    &[
      "passed",
      "failed",
      "not-run",
      "infrastructure-error",
      "interrupted",
    ],
    &[
      "step",
      "scenario",
      "run",
      "reset",
      "baseline-download",
      "build",
      "launch",
      "startup",
      "simulator-boot",
      "comparison",
      "media",
      "durability",
    ],
  ] {
    for value in values {
      assert!(
        serde_json::from_value::<StepName>(json!(value)).is_ok()
          || serde_json::from_value::<StepStatus>(json!(value)).is_ok()
          || serde_json::from_value::<DeadlineKind>(json!(value)).is_ok()
      );
    }
  }
  for value in ERROR_CODES {
    let parsed: ErrorCode = serde_json::from_value(json!(value)).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), json!(value));
  }
  for value in [
    "ditto",
    "ditto-player",
    "unity",
    "rust",
    "odiff",
    "ffmpeg",
    "filesystem",
    "r2",
  ] {
    let parsed: ErrorSource = serde_json::from_value(json!(value)).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), json!(value));
  }

  let artifacts = [
    json!({"kind":"screenshot","checkpoint":"snap"}),
    json!({"kind":"failure-frame"}),
  ];
  for value in artifacts {
    value_round_trip::<ArtifactKind>(value);
  }
  let frames = [
    json!({"status":"captured","artifact_id":FAILURE_ID}),
    json!({"status":"unavailable","reason":"capture failed","error_ref":"P0001"}),
  ];
  for value in frames {
    value_round_trip::<PlayerFailureFrame>(value);
  }
  for value in [
    json!({"status":"passed","duration_ms":2}),
    json!({"status":"failed","duration_ms":2,"stage":"reset","error_ref":"P0001"}),
  ] {
    value_round_trip::<ScenarioBoundaryOutcome>(value);
  }
  for body in context_bodies() {
    value_round_trip::<DittoContext>(body);
  }
}

#[test]
fn serde_rejects_unknown_fields_and_malformed_closed_unions() {
  reject_unknown::<Started>(serde_json::from_str(STARTED).unwrap(), "/unexpected");
  reject_unknown::<ScenarioComplete>(completion(), "/steps/0/unexpected");
  reject_unknown::<ScenarioComplete>(completion(), "/artifacts/0/kind/unexpected");
  reject_unknown::<ScenarioComplete>(completion(), "/failure_frame/unexpected");
  reject_unknown::<ScenarioComplete>(completion(), "/video_inputs/0/unexpected");
  reject_unknown::<ScenarioComplete>(completion(), "/boundary/unexpected");

  let mut both: Value = serde_json::from_str(STARTED).unwrap();
  both["identity"] = json!({
    "startup_report": serde_json::from_str::<Value>(STARTED).unwrap()["identity"]["startup_report"],
    "accepted_player_session_id": PLAYER_SESSION_ID
  });
  assert!(serde_json::from_value::<Started>(both).is_err());
  let mut neither: Value = serde_json::from_str(STARTED).unwrap();
  neither["identity"] = json!({});
  assert!(serde_json::from_value::<Started>(neither).is_err());
  assert!(
    serde_json::from_value::<ArtifactKind>(json!({
      "kind":"failure-frame", "checkpoint":"extra"
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<PlayerFailureFrame>(json!({
      "status":"captured", "artifact_id":FAILURE_ID, "reason":"extra"
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<ScenarioBoundaryOutcome>(json!({
      "status":"passed", "duration_ms":1, "stage":"reset"
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<DittoContext>(json!({
      "context":"job-ended", "reason":"completed", "run_id":RUN_ID
    }))
    .is_err()
  );
}

#[test]
fn startup_identity_conflicts_and_report_bounds_are_rejected() {
  invalid_started("wrong route", |started| {
    started.player_session_id = OTHER_ID.to_owned()
  });
  invalid_started("missing first sequence", |started| {
    started.first_log_sequence = None
  });
  invalid_started("two failures", |started| {
    let failure = PlayerInfrastructureFailure {
      code: ErrorCode::StartupProbeFailed,
      message: "probe failed".to_owned(),
    };
    started.startup_failure = Some(failure.clone());
    started.startup_log_failure = Some(failure);
  });
  invalid_started("bad startup hash", |started| {
    let StartupIdentity::Report(identity) = &mut started.identity else {
      unreachable!();
    };
    identity.startup_report.build_fingerprint = "A".repeat(64);
  });
  invalid_started("duplicate capability", |started| {
    let StartupIdentity::Report(identity) = &mut started.identity else {
      unreachable!();
    };
    identity.startup_report.capabilities.push(Capability::Png);
  });
  let cold: Started = serde_json::from_str(STARTED).unwrap();
  cold
    .validate(&job(), PLAYER_SESSION_ID, Some(PLAYER_SESSION_ID))
    .unwrap_err();
  let mut warm: Started = serde_json::from_str(STARTED).unwrap();
  warm.identity = StartupIdentity::Accepted(AcceptedPlayerSessionIdentity {
    accepted_player_session_id: PLAYER_SESSION_ID.to_owned(),
  });
  warm.validate(&job(), PLAYER_SESSION_ID, None).unwrap_err();
}

#[test]
fn scenario_completion_bounds_references_and_conditionals_are_rejected() {
  invalid_completion("scenario owner", |complete| {
    complete.scenario_id = OTHER_ID.to_owned()
  });
  invalid_completion("step count", |complete| {
    complete.steps.pop();
  });
  invalid_completion("step ownership", |complete| complete.steps[0].index = 1);
  invalid_completion("not-run payload", |complete| {
    complete.steps[0].status = StepStatus::NotRun
  });
  invalid_completion("unobserved step error", |complete| {
    complete.steps[4].error_refs[0] = "P0002".to_owned()
  });
  invalid_completion("unobserved primary", |complete| {
    complete.primary_error_ref = Some("P0002".to_owned())
  });
  invalid_completion("artifact mismatch", |complete| {
    complete.steps[1].screenshot_artifact_id = Some(OTHER_ID.to_owned())
  });
  invalid_completion("failure kind mismatch", |complete| {
    complete.artifacts[1].kind = ArtifactKind::Screenshot {
      checkpoint: "snap".to_owned(),
    }
  });
  invalid_completion("video start mismatch", |complete| {
    complete.video_inputs[0].start_step_index = 3
  });
  invalid_completion("video hash", |complete| {
    complete.video_inputs[0].sha256 = "bad".to_owned()
  });
  invalid_completion("duration bound", |complete| {
    complete.execution_duration_ms = 10_000
  });
  let mut artifacts = completion();
  artifacts.artifacts = vec![artifacts.artifacts[0].clone(); 129];
  assert!(artifacts.validate(&job(), &["P0001".to_owned()]).is_err());
  let mut videos = completion();
  videos.video_inputs = vec![videos.video_inputs[0].clone(); 65];
  assert!(videos.validate(&job(), &["P0001".to_owned()]).is_err());
  let mut refs = completion();
  refs.steps[4].error_refs = (1..=17).map(|index| format!("P{index:04}")).collect();
  let observed = refs.steps[4].error_refs.clone();
  assert!(refs.validate(&job(), &observed).is_err());
}

#[test]
fn decisions_terminal_accounting_acknowledgements_and_http_errors_are_closed() {
  let mut continue_with_error = ScenarioDecision {
    action: NextAction::Continue,
    completed_failures: 0,
    error_id: Some("E0001".to_owned()),
    error_code: Some(ErrorCode::StartupMismatch),
    message: Some("wrong display".to_owned()),
  };
  assert!(continue_with_error.validate().is_err());
  continue_with_error.action = NextAction::Stop;
  continue_with_error.validate().unwrap();
  continue_with_error.action = NextAction::Relaunch;
  continue_with_error.validate().unwrap();
  continue_with_error.error_id = None;
  assert!(continue_with_error.validate().is_err());

  let mut terminal: JobComplete = serde_json::from_value(json!({
    "job_id":JOB_ID,"last_log_sequence":90,"executed_scenario_ids":[],
    "unstarted_scenarios":[{"scenario_id":SCENARIO_ID,"reason":"bail"}],
    "reason":"bail","execution_duration_ms":10
  }))
  .unwrap();
  terminal.validate(&job()).unwrap();
  terminal.reason = TerminalReason::Completed;
  assert!(terminal.validate(&job()).is_err());
  terminal.reason = TerminalReason::Bail;
  terminal.unstarted_scenarios[0].scenario_id = OTHER_ID.to_owned();
  assert!(terminal.validate(&job()).is_err());

  assert!(
    LogBatchAck {
      player_session_id: OTHER_ID.to_owned(),
      next_sequence: 83,
    }
    .validate(PLAYER_SESSION_ID, 83)
    .is_err()
  );
  assert!(
    ArtifactAck {
      artifact_id: SCREENSHOT_ID.to_owned(),
      sha256: HASH_A.to_owned(),
    }
    .validate(SCREENSHOT_ID, HASH_B)
    .is_err()
  );
  assert!(
    JobCompleteAck {
      job_id: OTHER_ID.to_owned(),
    }
    .validate(JOB_ID)
    .is_err()
  );

  let mut error = HttpError {
    error_id: "E0003".to_owned(),
    code: ErrorCode::TransportLogGap,
    message: "log sequence gap".to_owned(),
    expected_sequence: Some(82),
    related_run_id: None,
  };
  error.validate().unwrap();
  error.code = ErrorCode::TransportRequestFailed;
  assert!(error.validate().is_err());
  error.expected_sequence = None;
  error.error_id = "P0001".to_owned();
  assert!(error.validate().is_err());
}

#[test]
fn ndjson_rejects_byte_sequence_and_ownership_failures() {
  let job = job();
  assert!(decode_ndjson(NDJSON.trim_end().as_bytes(), &job, PLAYER_SESSION_ID, 81).is_err());
  assert!(
    decode_ndjson(
      NDJSON.replace('\n', "\n\n").as_bytes(),
      &job,
      PLAYER_SESSION_ID,
      81
    )
    .is_err()
  );
  assert!(decode_ndjson(NDJSON.as_bytes(), &job, PLAYER_SESSION_ID, 80).is_err());
  assert!(decode_ndjson(NDJSON.as_bytes(), &job, OTHER_ID, 81).is_err());
  assert!(
    decode_ndjson(
      NDJSON.replace("\"schema\":1", "\"schema\":2").as_bytes(),
      &job,
      PLAYER_SESSION_ID,
      81
    )
    .is_err()
  );
  assert!(
    decode_ndjson(
      NDJSON.replace(SCENARIO_ID, OTHER_ID).as_bytes(),
      &job,
      PLAYER_SESSION_ID,
      81
    )
    .is_err()
  );
  let oversize = format!("{{\"message\":\"{}\"}}\n", "x".repeat(1024 * 1024));
  assert!(decode_ndjson(oversize.as_bytes(), &job, PLAYER_SESSION_ID, 0).is_err());
}

fn invalid_started(description: &str, mutate: impl FnOnce(&mut Started)) {
  let mut value: Started = serde_json::from_str(STARTED).unwrap();
  mutate(&mut value);
  assert!(
    value.validate(&job(), PLAYER_SESSION_ID, None).is_err(),
    "expected invalid {description}"
  );
}

fn invalid_completion(description: &str, mutate: impl FnOnce(&mut ScenarioComplete)) {
  let mut value = completion();
  mutate(&mut value);
  assert!(
    value.validate(&job(), &["P0001".to_owned()]).is_err(),
    "expected invalid {description}"
  );
}

fn reject_unknown<T>(value: T, pointer: &str)
where
  T: DeserializeOwned + Serialize,
{
  let mut value = serde_json::to_value(value).unwrap();
  let (parent, field) = pointer.rsplit_once('/').unwrap();
  value
    .pointer_mut(parent)
    .unwrap()
    .as_object_mut()
    .unwrap()
    .insert(field.to_owned(), json!(true));
  assert!(serde_json::from_value::<T>(value).is_err());
}

fn round_trip<T>(values: &[T])
where
  T: Clone + Debug + DeserializeOwned + PartialEq + Serialize,
{
  let encoded = serde_json::to_string(values).unwrap();
  let decoded: Vec<T> = serde_json::from_str(&encoded).unwrap();
  assert_eq!(decoded, values);
}

fn value_round_trip<T>(value: Value)
where
  T: DeserializeOwned + Serialize,
{
  let parsed: T = serde_json::from_value(value.clone()).unwrap();
  assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

fn shared_ndjson(records: &Value) -> Vec<u8> {
  let mut bytes = records
    .as_array()
    .unwrap()
    .iter()
    .map(|record| serde_json::to_string(record).unwrap())
    .collect::<Vec<_>>()
    .join("\n")
    .into_bytes();
  bytes.push(b'\n');
  bytes
}

fn mutate(value: &mut Value, pointer: &str, replacement: Value) {
  if let Some(target) = value.pointer_mut(pointer) {
    *target = replacement;
    return;
  }
  let (parent, field) = pointer.rsplit_once('/').unwrap();
  value
    .pointer_mut(parent)
    .unwrap()
    .as_object_mut()
    .unwrap()
    .insert(field.to_owned(), replacement);
}

fn job() -> Job {
  let job: Job = serde_json::from_str(JOB).unwrap();
  job.validate().unwrap();
  job
}

fn completion() -> ScenarioComplete {
  serde_json::from_str(SCENARIO_COMPLETE).unwrap()
}

fn context_bodies() -> Vec<Value> {
  vec![
    json!({"context":"job-started","run_id":RUN_ID}),
    json!({"context":"job-ended","reason":"completed"}),
    json!({"context":"engine-started","engine_session_id":ENGINE_ID,"scenario_id":SCENARIO_ID}),
    json!({"context":"engine-ended","engine_session_id":ENGINE_ID,"status":"passed"}),
    json!({"context":"scenario-started","scenario_id":SCENARIO_ID}),
    json!({"context":"scenario-ended","scenario_id":SCENARIO_ID,"execution_status":"failed",
      "failure_frame":null,"video_inputs":[],"execution_duration_ms":5,
      "startup_duration_ms":1,"settle_duration_ms":2,"capture_duration_ms":3,
      "boundary":{"status":"passed","duration_ms":2},
      "primary_error_ref":"P0001"}),
    json!({"context":"step-started","scenario_id":SCENARIO_ID,"step_index":0}),
    json!({"context":"step-ended","scenario_id":SCENARIO_ID,"result":
      serde_json::from_str::<Value>(SCENARIO_COMPLETE).unwrap()["steps"][0]}),
    json!({"context":"artifact-accepted","scenario_id":SCENARIO_ID,"step_index":1,
      "artifact_id":SCREENSHOT_ID,"artifact_kind":{"kind":"screenshot","checkpoint":"snap"}}),
    json!({"context":"error-observed","scenario_id":SCENARIO_ID,"step_index":4,
      "error_ref":"P0001","code":"assertion.failed","source":"ditto-player",
      "record_sequence":80,"battlement_error_id":null}),
  ]
}

const JOB_ID: &str = "0197b35f-6c59-7b98-b1f0-a39f5ee54db8";
const RUN_ID: &str = "0197b35f-6c59-7b98-b1f0-a39f5ee54db8";
const PLAYER_SESSION_ID: &str = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";
const ENGINE_ID: &str = "0197b35f-6dab-7a01-91d0-d9cc0e9ba811";
const SCENARIO_ID: &str = "0197b35f-6e24-75d8-9482-aa6c22a15133";
const SCREENSHOT_ID: &str = "0197b35f-6ef0-78df-8b96-b31bc9959181";
const FAILURE_ID: &str = "0197b35f-6ef0-78df-8b96-b31bc9959182";
const OTHER_ID: &str = "0197b35f-6ef0-78df-8b96-b31bc9959999";
const HASH_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HASH_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

const JOB: &str = r#"{
  "job_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "run_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "remaining_run_timeout_ms":20000,"log_redactions":[],"command":"run",
  "profile":{"name":"macos-local","platform":"macos",
    "display":{"width":1280,"height":720,"scale":1.0,"orientation":null,"safe_area":[0,0,1280,720]},
    "build_fingerprint":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "source_fingerprint":"fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "capabilities":["png","video"]},
  "scenarios":[{"id":"0197b35f-6e24-75d8-9482-aa6c22a15133","run_index":0,
    "name":"lifecycle","motion":"controlled","timeout_ms":10000,"steps":[
      {"index":0,"name":null,"timeout_ms":1000,"action":{"assert":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"visible"}}},
      {"index":1,"name":null,"timeout_ms":1000,"action":{"screenshot":{"name":"snap","comparison":{"threshold":"0.05","anti_alias":false,"max_changed_percent":"0"}}}},
      {"index":2,"name":null,"timeout_ms":1000,"action":{"video":{"action":"start","name":"clip","motion":"real-time","max_duration_ms":5000}}},
      {"index":3,"name":null,"timeout_ms":1000,"action":{"video":{"action":"stop"}}},
      {"index":4,"name":null,"timeout_ms":1000,"action":{"assert":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"enabled"}}}
    ]}]
}"#;

const STARTED: &str = r#"{
  "job_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "run_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "player_session_id":"0197b35f-6d12-71ac-b370-0bb2cbced1b2",
  "first_log_sequence":81,"startup_failure":null,"startup_log_failure":null,
  "identity":{"startup_report":{"platform":"macos","capture_adapter":"unity-async-readback-png",
    "build_fingerprint":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "source_fingerprint":"fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "unity_version":"6000.0.56f1","diagnostics":true,
    "display":{"width":1280,"height":720,"scale":1.0,"orientation":null,"safe_area":[0,0,1280,720]},
    "capabilities":["png","video"]}}
}"#;

const SCENARIO_COMPLETE: &str = r#"{
  "scenario_id":"0197b35f-6e24-75d8-9482-aa6c22a15133","execution_status":"failed",
  "steps":[
    {"index":0,"name":null,"kind":"assert","status":"passed","duration_ms":1,
      "expired_deadline":null,"error_refs":[],"assertion":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"visible","expected":true,"observed":true,"passed":true},
      "screenshot_artifact_id":null,"video_input_id":null},
    {"index":1,"name":null,"kind":"screenshot","status":"passed","duration_ms":1,
      "expired_deadline":null,"error_refs":[],"assertion":null,
      "screenshot_artifact_id":"0197b35f-6ef0-78df-8b96-b31bc9959181","video_input_id":null},
    {"index":2,"name":null,"kind":"video","status":"passed","duration_ms":1,
      "expired_deadline":null,"error_refs":[],"assertion":null,"screenshot_artifact_id":null,
      "video_input_id":"0197b35f-6ef0-78df-8b96-b31bc9959183"},
    {"index":3,"name":null,"kind":"video","status":"passed","duration_ms":1,
      "expired_deadline":null,"error_refs":[],"assertion":null,"screenshot_artifact_id":null,"video_input_id":null},
    {"index":4,"name":null,"kind":"assert","status":"failed","duration_ms":1,
      "expired_deadline":null,"error_refs":["P0001"],"assertion":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"enabled","expected":true,"observed":false,"passed":false},
      "screenshot_artifact_id":null,"video_input_id":null}
  ],
  "artifacts":[
    {"artifact_id":"0197b35f-6ef0-78df-8b96-b31bc9959181","step_index":1,"kind":{"kind":"screenshot","checkpoint":"snap"}},
    {"artifact_id":"0197b35f-6ef0-78df-8b96-b31bc9959182","step_index":4,"kind":{"kind":"failure-frame"}}
  ],
  "failure_frame":{"status":"captured","artifact_id":"0197b35f-6ef0-78df-8b96-b31bc9959182"},
  "video_inputs":[{"input_id":"0197b35f-6ef0-78df-8b96-b31bc9959183","start_step_index":2,
    "path":"videos/clip.raw","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "width":1280,"height":720,"frame_count":30,"truncated":false}],
  "last_log_sequence":90,"execution_duration_ms":5,"startup_duration_ms":1,
  "settle_duration_ms":2,"capture_duration_ms":3,
  "boundary":{"status":"passed","duration_ms":2},"primary_error_ref":"P0001"
}"#;

const NDJSON: &str = concat!(
  "{\"schema\":1,\"job_id\":\"0197b35f-6c59-7b98-b1f0-a39f5ee54db8\",\"player_session_id\":\"0197b35f-6d12-71ac-b370-0bb2cbced1b2\",\"sequence\":81,\"timestamp_unix_us\":1787953800000000,\"source\":\"rust\",\"severity\":\"information\",\"event_name\":\"chess.move\",\"message\":\"move applied\",\"fields\":{\"piece\":\"knight\"},\"exception\":null,\"stack_trace\":null}\n",
  "{\"schema\":1,\"job_id\":\"0197b35f-6c59-7b98-b1f0-a39f5ee54db8\",\"player_session_id\":\"0197b35f-6d12-71ac-b370-0bb2cbced1b2\",\"sequence\":82,\"timestamp_unix_us\":1787953800000100,\"source\":\"ditto-player\",\"severity\":\"information\",\"event_name\":\"ditto.context\",\"message\":\"scenario started\",\"body\":{\"context\":\"scenario-started\",\"scenario_id\":\"0197b35f-6e24-75d8-9482-aa6c22a15133\"}}\n"
);

const ERROR_CODES: &[&str] = &[
  "configuration.invalid",
  "build.failed",
  "launch.failed",
  "simulator.boot-failed",
  "startup.mismatch",
  "startup.probe-failed",
  "assertion.failed",
  "input.unreachable",
  "condition.unsupported",
  "image.mismatch",
  "image.missing-baseline",
  "image.capture-failed",
  "image.comparison-failed",
  "baseline.download-failed",
  "baseline.hash-mismatch",
  "baseline.store-conflict",
  "runtime.unity-error",
  "runtime.unity-assert",
  "runtime.unity-exception",
  "runtime.fatal",
  "runtime.panic",
  "runtime.process-exit",
  "runtime.reset-failed",
  "runtime.destroy-failed",
  "deadline.expired",
  "transport.request-failed",
  "transport.log-buffer-overflow",
  "transport.log-record-oversize",
  "transport.log-gap",
  "transport.log-conflict",
  "transport.artifact-conflict",
  "media.insufficient-space",
  "media.recording-failed",
  "media.ffmpeg-failed",
  "durability.failed",
  "durability.result-commit-failed",
  "baseline.lock-stale",
  "baseline.manifest-write-failed",
  "baseline.publish-failed",
  "baseline.lease-lost",
  "baseline.cleanup-failed",
];
