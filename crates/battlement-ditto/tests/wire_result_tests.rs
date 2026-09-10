use std::fmt::Debug;

use battlement_ditto::wire::{
  baseline_state::{BaselineStoreState, BaselineTombstone},
  common::{AssertionResult, DeadlineKind, ErrorCode, ErrorSource, StepName, StepStatus},
  job::{
    Capability, Comparison, Display, Motion, ObjectState, PerformanceAttempt, PerformancePass,
    Platform,
  },
  lifecycle::{ObserverFrameTiming, PacingConfiguration, StartupReport, StepPerformance},
  result::{
    BaselineOutcome, BaselineWriteResult, BaselineWriteStatus, BuildDisposition, BuildResult,
    ComparisonOutcome, Distribution, ErrorOccurrence, ImageFile, JobResult, JobStatus, LogSpan,
    MeasuredStepAttempt, MeasuredStepPerformance, MediaCapture, ObserverPassSummary,
    ObserverStageSummary, PerformanceAttemptSummary, PerformanceHotspot, PerformanceResult,
    PhaseName, PhaseResult, PhaseStatus, PlayerSessionResult, Recovery, ResultCommand, RunResult,
    RunStatus, ScenarioResult, ScenarioStatus, ScenarioTimings, ScreenshotResult, StepResult,
    VideoResult,
  },
  review::{
    ReviewAcceptance, ReviewAcceptanceResult, ReviewEvent, ReviewEventBody, ReviewSelection,
  },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

#[test]
fn complete_result_review_exchange_validates() {
  let result = complete_result();
  result.validate().unwrap();

  let snapshot = ReviewEvent {
    id: 1,
    body: ReviewEventBody::Snapshot {
      result: result.clone(),
    },
  };
  snapshot.validate().unwrap();
  for body in [
    ReviewEventBody::LogBatch {
      player_session_id: PLAYER_SESSION_ID.to_owned(),
      first_sequence: 1,
      last_sequence: 9,
    },
    ReviewEventBody::ScenarioCompleted {
      scenario_id: SCENARIO_ID.to_owned(),
    },
    ReviewEventBody::RunCompleted {
      run_id: RUN_ID.to_owned(),
    },
  ] {
    ReviewEvent { id: 2, body }.validate().unwrap();
  }

  let acceptance = acceptance();
  acceptance.validate(&result).unwrap();
  ReviewAcceptanceResult {
    comparison_run_id: COMPARISON_RUN_ID.to_owned(),
    lock_sha256: HASH_D.to_owned(),
  }
  .validate()
  .unwrap();
}

#[test]
fn retained_legacy_result_remains_readable() {
  let mut result = complete_result();
  let report = &mut result.player_sessions[0].startup_report;
  report.determinism_contract = "ditto-v1".to_owned();
  report.capabilities.insert(1, Capability::Key);
  result.validate().unwrap();
}

#[test]
fn canonical_result_and_baseline_state_bytes_are_stable() {
  let result = complete_result();
  let bytes = result.to_canonical_json().unwrap();
  assert_eq!(bytes.last(), Some(&b'\n'));
  let text = String::from_utf8(bytes).unwrap();
  assert!(text.starts_with("{\n  \"artifacts\":"));
  assert!(text.contains("\n  \"build\":"));
  assert!(text.find("\"artifacts\"") < text.find("\"build\""));
  assert!(!text.contains("\n    \n"));

  let line = result.to_canonical_json_line().unwrap();
  assert_eq!(line.last(), Some(&b'\n'));
  assert_eq!(line.iter().filter(|byte| **byte == b'\n').count(), 1);

  let state = initial_state();
  state.validate(None).unwrap();
  assert_eq!(
    String::from_utf8(state.to_canonical_json().unwrap()).unwrap(),
    format!(
      concat!(
        "{{\n",
        "  \"cleanup_applied_at\": null,\n",
        "  \"generation\": 1,\n",
        "  \"live_sha256\": [\n",
        "    \"{}\"\n",
        "  ],\n",
        "  \"lock_sha256\": \"{}\",\n",
        "  \"published_at\": \"2026-08-28T20:00:00Z\",\n",
        "  \"tombstones\": [\n",
        "    {{\n",
        "      \"removed_at\": \"2026-08-27T20:00:00Z\",\n",
        "      \"sha256\": \"{}\"\n",
        "    }}\n",
        "  ]\n",
        "}}\n"
      ),
      HASH_A, HASH_C, HASH_B
    )
  );
}

#[test]
fn every_result_and_review_variant_round_trips() {
  round_trip(&[
    ResultCommand::Run,
    ResultCommand::Capture,
    ResultCommand::Profile,
    ResultCommand::ComparisonOnly,
  ]);
  round_trip(&[
    RunStatus::Passed,
    RunStatus::Failed,
    RunStatus::InfrastructureError,
    RunStatus::Interrupted,
  ]);
  round_trip(&[
    BuildDisposition::Created,
    BuildDisposition::Reused,
    BuildDisposition::RequiredByNoBuild,
    BuildDisposition::Failed,
  ]);
  round_trip(&[
    PhaseName::Discovery,
    PhaseName::Build,
    PhaseName::Launch,
    PhaseName::Startup,
    PhaseName::Scenarios,
    PhaseName::Cleanup,
    PhaseName::SimulatorBoot,
    PhaseName::Reset,
    PhaseName::BaselineDownload,
    PhaseName::Comparison,
    PhaseName::Media,
    PhaseName::Durability,
  ]);
  round_trip(&[
    PhaseStatus::Passed,
    PhaseStatus::Failed,
    PhaseStatus::Interrupted,
  ]);
  round_trip(&[
    JobStatus::Passed,
    JobStatus::Failed,
    JobStatus::InfrastructureError,
    JobStatus::Interrupted,
  ]);
  round_trip(&[
    ScenarioStatus::Passed,
    ScenarioStatus::Failed,
    ScenarioStatus::Skipped,
    ScenarioStatus::NotRun,
    ScenarioStatus::InfrastructureError,
    ScenarioStatus::Interrupted,
  ]);
  round_trip(&[
    Recovery::None,
    Recovery::Reset,
    Recovery::Relaunch,
    Recovery::RelaunchFailed,
  ]);
  round_trip(&[
    BaselineWriteStatus::Proposed,
    BaselineWriteStatus::UploadedUnreferenced,
    BaselineWriteStatus::Published,
  ]);

  let image = image("actual/checkpoint.png", HASH_A);
  round_trip(&[
    BaselineOutcome::NotLoaded,
    BaselineOutcome::Missing,
    BaselineOutcome::Loaded {
      image: image.clone(),
    },
  ]);
  round_trip(&[
    ComparisonOutcome::Passed {
      changed_pixels: 0,
      total_pixels: 4,
      settings: comparison(),
    },
    ComparisonOutcome::Mismatch {
      changed_pixels: 1,
      total_pixels: 4,
      settings: comparison(),
      diff: image.clone(),
    },
  ]);
  round_trip(&[
    ScreenshotResult::Captured {
      checkpoint: "checkpoint".to_owned(),
      actual: image.clone(),
      baseline: BaselineOutcome::Missing,
      comparison: None,
      matched_before_update: Some(false),
      updated: Some(true),
    },
    ScreenshotResult::Unavailable {
      reason: "capture failed".to_owned(),
      error_id: "E0001".to_owned(),
    },
  ]);
  round_trip(&[
    VideoResult::Encoded {
      path: "video/out.mp4".to_owned(),
      sha256: HASH_A.to_owned(),
      width: 2,
      height: 2,
      frame_rate: 30,
      duration_ms: 100,
      truncated: false,
    },
    VideoResult::Failed {
      error_id: "E0001".to_owned(),
      diagnostic_paths: vec!["video/raw.log".to_owned()],
    },
  ]);
  round_trip(&[
    MediaCapture::Captured { image },
    MediaCapture::Unavailable {
      reason: "not attempted".to_owned(),
      error_id: None,
    },
  ]);

  for body in [
    ReviewEventBody::LogBatch {
      player_session_id: PLAYER_SESSION_ID.to_owned(),
      first_sequence: 1,
      last_sequence: 1,
    },
    ReviewEventBody::ScenarioCompleted {
      scenario_id: SCENARIO_ID.to_owned(),
    },
    ReviewEventBody::RunCompleted {
      run_id: RUN_ID.to_owned(),
    },
  ] {
    round_trip(&[body]);
  }
  round_trip(&[ReviewEventBody::Snapshot {
    result: complete_result(),
  }]);
}

#[test]
fn profile_result_validates_headline_attempts_and_hotspots() {
  let summary = PerformanceResult {
    headline: "Missed 60 Hz interaction deadlines".to_owned(),
    timing_proxy: "unity-wait-for-end-of-frame".to_owned(),
    timing_limitation: "not input-to-photon".to_owned(),
    target_fps: 60,
    missed_interaction_deadlines: 0,
    goal: 0,
    measured_score_attempts: 5,
    score_attempt_values: vec![0, 0, 0, 0, 0],
    pacing_configurations: vec![pacing_configuration()],
    warmup_attempts: vec![
      performance_attempt_summary(PerformancePass::Score, 0, true),
      performance_attempt_summary(PerformancePass::Detail, 0, true),
    ],
    score_attempts: (1..=5)
      .map(|iteration| performance_attempt_summary(PerformancePass::Score, iteration, false))
      .collect(),
    measured_steps: vec![MeasuredStepPerformance {
      name: "Open settings".to_owned(),
      attempts: 5,
      no_visual_response_attempts: 0,
      missed_interaction_deadlines: 0,
      response_missed_deadlines: 0,
      pacing_missed_deadlines: 0,
      median_response_latency_ns: 10,
      p95_response_latency_ns: 10,
      maximum_response_latency_ns: 10,
      median_semantic_completion_latency_ns: None,
      median_settled_completion_latency_ns: None,
      worst_presentation_interval_ns: 10,
      presentation_interval_distribution: test_distribution(5, 10),
      over_budget_presentation_intervals: 0,
      longest_over_budget_sequence: 0,
      managed_allocated_bytes: Some(0),
      attempt_values: (1..=5)
        .map(|iteration| MeasuredStepAttempt {
          iteration,
          response_latency_ns: 10,
          semantic_completion_latency_ns: None,
          settled_completion_latency_ns: None,
          response_missed_deadlines: 0,
          pacing_missed_deadlines: 0,
          missed_interaction_deadlines: 0,
          no_visual_response: false,
          presented_frames: 2,
          presentation_interval_distribution: test_distribution(1, 10),
        })
        .collect(),
    }],
    observer_passes: vec![
      observer_pass_summary(PerformancePass::Score),
      observer_pass_summary(PerformancePass::Detail),
    ],
    detail_hotspots: vec![PerformanceHotspot {
      component: "settings::Settings".to_owned(),
      calls: 100,
      total_self_duration_us: 20_000,
      maximum_self_duration_us: 1_000,
      total_inclusive_duration_us: 50_000,
    }],
  };
  let mut result = RunResult {
    run_id: RUN_ID.to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command: ResultCommand::Profile,
    source_command: None,
    cycle: 1,
    suite: Some("suite".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-28T20:00:00Z".to_owned(),
    duration_ms: 450,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: vec![],
    player_sessions: vec![],
    jobs: vec![],
    scenarios: vec![ScenarioResult {
      id: SCENARIO_ID.to_owned(),
      name: "profile attempt".to_owned(),
      status: ScenarioStatus::Passed,
      status_reason: None,
      motion: Motion::Controlled,
      duration_ms: 20,
      expired_deadline: None,
      timings: empty_timings(),
      steps: vec![StepResult {
        index: 0,
        name: Some("Open settings".to_owned()),
        kind: StepName::PointerAction,
        status: StepStatus::Passed,
        status_reason: None,
        duration_ms: 20,
        expired_deadline: None,
        error_ids: vec![],
        assertion: None,
        screenshot: None,
        video: None,
        performance: Some(StepPerformance {
          target_fps: 60,
          timing_proxy: "unity-wait-for-end-of-frame".to_owned(),
          pacing_configuration: pacing_configuration(),
          response_latency_ns: 10,
          response_missed_deadlines: 0,
          pacing_missed_deadlines: 0,
          missed_interaction_deadlines: 0,
          no_visual_response: false,
          has_activation: false,
          semantic_completion_latency_ns: None,
          settled_completion_latency_ns: None,
          presentation_timestamps_ns: vec![10, 20],
          presentation_intervals_ns: vec![10],
          managed_allocation_deltas: Some(vec![0, 0]),
          observer_timings: vec![observer_frame_timing(), observer_frame_timing()],
        }),
      }],
      logs: None,
      failure_frame: None,
      recovery: Recovery::None,
      performance_attempt: Some(PerformanceAttempt {
        pass: PerformancePass::Score,
        warmup: false,
        iteration: 1,
        target_fps: 60,
        idle_frames: 120,
      }),
    }],
    warnings: vec![],
    errors: vec![],
    baseline_writes: vec![],
    artifacts: vec![],
    performance: Some(summary),
  };
  let template = result.scenarios[0].clone();
  result.scenarios = [PerformancePass::Score, PerformancePass::Detail]
    .into_iter()
    .flat_map(|pass| {
      let template = template.clone();
      (0..=5).map(move |iteration| {
        let mut scenario = template.clone();
        let (pass_name, pass_offset) = match pass {
          PerformancePass::Score => ("score", 0),
          PerformancePass::Detail => ("detail", 10),
        };
        scenario.id = format!("00000000-0000-4000-8000-{:012}", iteration + pass_offset);
        scenario.name = format!("profile {pass_name} {iteration}");
        scenario.performance_attempt = Some(PerformanceAttempt {
          pass,
          warmup: iteration == 0,
          iteration,
          target_fps: 60,
          idle_frames: 120,
        });
        scenario
      })
    })
    .collect();
  result.validate().unwrap();
  rejects(&result, |result| {
    result
      .performance
      .as_mut()
      .unwrap()
      .missed_interaction_deadlines = 1
  });
  rejects(&result, |result| {
    result.performance.as_mut().unwrap().score_attempt_values[4] = 1
  });
  rejects(&result, |result| {
    result.performance.as_mut().unwrap().measured_steps[0].attempts = 4
  });
  rejects(&result, |result| {
    result.scenarios.pop();
  });
  rejects(&result, |result| {
    let raw = result.scenarios[1].steps[0].performance.as_mut().unwrap();
    raw.pacing_missed_deadlines = 1;
    raw.missed_interaction_deadlines = 1;
  });
  rejects(&result, |result| result.performance = None);
  rejects(&result, |result| {
    result.status = RunStatus::Failed;
    result.exit_code = 1;
  });
  rejects(&result, |result| {
    result.scenarios[0].steps[0]
      .performance
      .as_mut()
      .unwrap()
      .presentation_intervals_ns[0] = 9;
  });
  result.performance = None;
  result.scenarios[0].status = ScenarioStatus::NotRun;
  result.scenarios[0].status_reason = Some("run-infrastructure-error".to_owned());
  result.scenarios[0].duration_ms = 0;
  result.scenarios[0].steps[0].status = StepStatus::NotRun;
  result.scenarios[0].steps[0].status_reason = Some("run-infrastructure-error".to_owned());
  result.scenarios[0].steps[0].duration_ms = 0;
  result.scenarios[0].steps[0].performance = None;
  result.validate().unwrap();
}

fn pacing_configuration() -> PacingConfiguration {
  PacingConfiguration {
    v_sync_count: 0,
    target_frame_rate: 60,
    display_refresh_hz: Some(60.0),
    width: 100,
    height: 100,
    debug_build: false,
  }
}

fn performance_attempt_summary(
  pass: PerformancePass,
  iteration: u32,
  warmup: bool,
) -> PerformanceAttemptSummary {
  PerformanceAttemptSummary {
    pass,
    iteration,
    warmup,
    missed_interaction_deadlines: 0,
    response_missed_deadlines: 0,
    pacing_missed_deadlines: 0,
    no_visual_response_attempts: 0,
  }
}

fn test_distribution(count: u64, value: u64) -> Distribution {
  Distribution {
    count,
    minimum_ns: value,
    p50_ns: value,
    p95_ns: value,
    p99_ns: value,
    maximum_ns: value,
  }
}

fn observer_frame_timing() -> ObserverFrameTiming {
  ObserverFrameTiming {
    motion_prepare_ns: 0,
    runner_frame_ns: 0,
    native_frame_complete_ns: 0,
    input_release_and_sync_transport_ns: 0,
    response_decode_ns: 0,
    response_apply_ns: 0,
    end_of_frame_wait_ns: 0,
    texture_setup_ns: 0,
    capture_request_cpu_ns: 0,
    synchronous_readback_ns: 0,
    cpu_hash_ns: 0,
    observed_pixels: 1,
    layout_observation_ns: 0,
    recorder_bookkeeping_ns: 0,
  }
}

fn observer_pass_summary(pass: PerformancePass) -> ObserverPassSummary {
  ObserverPassSummary {
    pass,
    attempts: 5,
    observed_frames: 10,
    observed_pixels: 10,
    stages: [
      "motion-prepare",
      "runner-frame",
      "native-frame-complete",
      "input-release-and-sync-transport",
      "response-decode",
      "response-apply",
      "end-of-frame-wait",
      "texture-setup",
      "capture-request-cpu",
      "synchronous-readback",
      "cpu-hash",
      "layout-observation",
      "recorder-bookkeeping",
    ]
    .into_iter()
    .map(|stage| ObserverStageSummary {
      stage: stage.to_owned(),
      total_ns: 0,
      p95_ns: 0,
      maximum_ns: 0,
    })
    .collect(),
  }
}

#[test]
fn unknown_fields_and_malformed_closed_unions_are_rejected() {
  let mut value = serde_json::to_value(complete_result()).unwrap();
  value
    .as_object_mut()
    .unwrap()
    .insert("extra".to_owned(), json!(true));
  assert!(serde_json::from_value::<RunResult>(value).is_err());
  assert!(
    serde_json::from_value::<ScreenshotResult>(json!({
      "status":"captured",
      "checkpoint":"checkpoint",
      "actual":{"path":"actual/a.png","sha256":HASH_A,"width":2,"height":2},
      "baseline":{"status":"missing"},
      "comparison":null,
      "matched_before_update":null,
      "updated":null,
      "extra":true
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<BaselineOutcome>(json!({
      "status":"missing","image":{"path":"a.png","sha256":HASH_A,"width":2,"height":2}
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<VideoResult>(json!({
      "status":"encoded","error_id":"E0001"
    }))
    .is_err()
  );
  assert!(
    serde_json::from_value::<ReviewEventBody>(json!({
      "event":"run-completed","run_id":RUN_ID,"extra":true
    }))
    .is_err()
  );
  let mut state = serde_json::to_value(initial_state()).unwrap();
  state
    .as_object_mut()
    .unwrap()
    .insert("extra".to_owned(), json!(true));
  assert!(serde_json::from_value::<BaselineStoreState>(state).is_err());
}

#[test]
fn result_conditionals_reject_inconsistent_states() {
  let valid = complete_result();
  rejects(&valid, |result| result.exit_code = 0);
  rejects(&valid, |result| {
    result.source_run_id = Some(COMPARISON_RUN_ID.to_owned())
  });
  rejects(&valid, |result| {
    result.started_at = "2026-08-28T20:00:00.1Z".to_owned()
  });
  rejects(&valid, |result| result.artifacts.swap(0, 1));
  rejects(&valid, |result| {
    result.scenarios[0].steps[0].assertion = None
  });
  rejects(&valid, |result| {
    result.scenarios[0].steps[1].status_reason = Some("bad".to_owned())
  });
  rejects(&valid, |result| {
    result.scenarios[0].expired_deadline = Some(DeadlineKind::Step)
  });
  rejects(&valid, |result| result.errors[0].id = "E0002".to_owned());
  rejects(&valid, |result| {
    let ScreenshotResult::Captured {
      comparison: Some(ComparisonOutcome::Mismatch { settings, .. }),
      ..
    } = result.scenarios[0].steps[1].screenshot.as_mut().unwrap()
    else {
      panic!("fixture screenshot must contain a mismatch");
    };
    settings.threshold = "00.1".to_owned();
  });
}

#[test]
fn unresolved_errors_and_artifact_mismatches_are_rejected() {
  let valid = complete_result();
  rejects(&valid, |result| {
    result.scenarios[0].steps[1].error_ids[0] = "E9999".to_owned()
  });
  rejects(&valid, |result| {
    result
      .artifacts
      .retain(|path| path != "actual/checkpoint.png")
  });
  rejects(&valid, |result| {
    result.errors[0].player_session_id = Some(COMPARISON_RUN_ID.to_owned())
  });
  rejects(&valid, |result| {
    let ScreenshotResult::Captured { comparison, .. } =
      result.scenarios[0].steps[1].screenshot.as_mut().unwrap()
    else {
      panic!("fixture screenshot must be captured");
    };
    *comparison = None;
  });
}

#[test]
fn skipped_and_capture_results_enforce_closed_conditions() {
  let mut skipped = complete_result();
  skipped.status = RunStatus::Passed;
  skipped.exit_code = 0;
  skipped.errors.clear();
  skipped.phases[1].error_ids.clear();
  skipped.scenarios[0].status = ScenarioStatus::Skipped;
  skipped.scenarios[0].status_reason = Some("unsupported-input:hover".to_owned());
  skipped.scenarios[0].duration_ms = 0;
  skipped.scenarios[0].timings = empty_timings();
  skipped.scenarios[0].logs = None;
  skipped.scenarios[0].failure_frame = None;
  skipped.scenarios[0].recovery = Recovery::None;
  for step in &mut skipped.scenarios[0].steps {
    step.status = StepStatus::NotRun;
    step.status_reason = Some("unsupported-input:hover".to_owned());
    step.duration_ms = 0;
    step.error_ids.clear();
    step.assertion = None;
    step.screenshot = None;
    step.video = None;
  }
  skipped.validate().unwrap();
  skipped.scenarios[0].steps[0].status = StepStatus::Passed;
  assert!(skipped.validate().is_err());

  let mut capture = complete_result();
  capture.command = ResultCommand::Capture;
  capture.lock_sha256 = None;
  let ScreenshotResult::Captured {
    baseline,
    comparison,
    ..
  } = capture.scenarios[0].steps[1].screenshot.as_mut().unwrap()
  else {
    panic!("fixture screenshot must be captured");
  };
  *baseline = BaselineOutcome::NotLoaded;
  *comparison = None;
  capture.validate().unwrap();
  capture.lock_sha256 = Some(HASH_C.to_owned());
  assert!(capture.validate().is_err());
}

#[test]
fn comparison_only_and_update_results_validate_their_distinct_fields() {
  let mut comparison_only = complete_result();
  comparison_only.run_id = COMPARISON_RUN_ID.to_owned();
  comparison_only.source_run_id = Some(RUN_ID.to_owned());
  comparison_only.command = ResultCommand::ComparisonOnly;
  comparison_only.source_command = Some(ResultCommand::Run);
  comparison_only.validate().unwrap();

  let mut update = complete_result();
  update.status = RunStatus::Passed;
  update.exit_code = 0;
  update.scenarios[0].status = ScenarioStatus::Passed;
  update.scenarios[0].steps[1].status = StepStatus::Passed;
  let ScreenshotResult::Captured {
    matched_before_update,
    updated,
    ..
  } = update.scenarios[0].steps[1].screenshot.as_mut().unwrap()
  else {
    panic!("fixture screenshot must be captured");
  };
  *matched_before_update = Some(false);
  *updated = Some(true);
  update.baseline_writes.push(BaselineWriteResult {
    sha256: HASH_A.to_owned(),
    profile: "macos-local".to_owned(),
    scenario: "scenario".to_owned(),
    checkpoint: "checkpoint".to_owned(),
    status: BaselineWriteStatus::Published,
  });
  update.validate().unwrap();

  let mut wrong_hash = update.clone();
  wrong_hash.baseline_writes[0].sha256 = HASH_B.to_owned();
  assert!(wrong_hash.validate().is_err());
  let mut comparison_update = update;
  comparison_update.command = ResultCommand::ComparisonOnly;
  comparison_update.source_run_id = Some(COMPARISON_RUN_ID.to_owned());
  comparison_update.source_command = Some(ResultCommand::Run);
  assert!(comparison_update.validate().is_err());
}

#[test]
fn review_acceptance_rejects_stale_duplicate_and_mismatched_selections() {
  let result = complete_result();
  let valid = acceptance();
  let mut stale = valid.clone();
  stale.lock_sha256 = Some(HASH_B.to_owned());
  assert!(stale.validate(&result).is_err());

  let mut duplicate = valid.clone();
  duplicate.selections.push(duplicate.selections[0].clone());
  assert!(duplicate.validate(&result).is_err());

  let mut mismatch = valid;
  mismatch.selections[0].actual_sha256 = HASH_B.to_owned();
  assert!(mismatch.validate(&result).is_err());
}

#[test]
fn baseline_state_generation_order_and_timestamps_are_enforced() {
  let initial = initial_state();
  let mut next = initial.clone();
  next.generation = 2;
  next.published_at = "2026-08-29T20:00:00Z".to_owned();
  next.validate(Some(&initial)).unwrap();

  next.generation = 3;
  assert!(next.validate(Some(&initial)).is_err());
  let mut unsorted = initial.clone();
  unsorted.live_sha256 = vec![HASH_C.to_owned(), HASH_A.to_owned()];
  assert!(unsorted.validate(None).is_err());
  let mut overlap = initial.clone();
  overlap.live_sha256 = vec![HASH_B.to_owned()];
  assert!(overlap.validate(None).is_err());
  let mut fractional = initial;
  fractional.published_at = "2026-08-28T20:00:00.000Z".to_owned();
  assert!(fractional.validate(None).is_err());
}

fn complete_result() -> RunResult {
  RunResult {
    run_id: RUN_ID.to_owned(),
    source_run_id: None,
    lock_sha256: Some(HASH_C.to_owned()),
    command: ResultCommand::Run,
    source_command: None,
    cycle: 1,
    suite: Some("suite".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-28T20:00:00Z".to_owned(),
    duration_ms: 450,
    status: RunStatus::Failed,
    exit_code: 1,
    build: Some(BuildResult {
      source_fingerprint: HASH_A.to_owned(),
      fingerprint: HASH_B.to_owned(),
      disposition: BuildDisposition::Created,
      duration_ms: 100,
      log_path: Some("build/build.log".to_owned()),
    }),
    phases: vec![
      PhaseResult {
        name: PhaseName::Build,
        status: PhaseStatus::Passed,
        duration_ms: 100,
        expired_deadline: None,
        log_path: Some("build/build.log".to_owned()),
        error_ids: vec![],
      },
      PhaseResult {
        name: PhaseName::Scenarios,
        status: PhaseStatus::Failed,
        duration_ms: 350,
        expired_deadline: None,
        log_path: None,
        error_ids: vec!["E0001".to_owned()],
      },
    ],
    player_sessions: vec![PlayerSessionResult {
      player_session_id: PLAYER_SESSION_ID.to_owned(),
      accepted: true,
      startup_report: startup_report(),
      diagnostic_paths: vec!["diagnostics/player.log".to_owned()],
    }],
    jobs: vec![JobResult {
      job_id: JOB_ID.to_owned(),
      player_session_id: PLAYER_SESSION_ID.to_owned(),
      status: JobStatus::Failed,
      first_scenario_index: Some(0),
      last_scenario_index: Some(0),
    }],
    scenarios: vec![scenario()],
    warnings: vec!["comparison found changed pixels".to_owned()],
    errors: vec![
      ErrorOccurrence {
        id: "E0001".to_owned(),
        code: ErrorCode::ImageMismatch,
        source: ErrorSource::ODiff,
        message: "image mismatch".to_owned(),
        job_id: Some(JOB_ID.to_owned()),
        player_session_id: Some(PLAYER_SESSION_ID.to_owned()),
        scenario_id: Some(SCENARIO_ID.to_owned()),
        step_index: Some(1),
        log_sequence: Some(5),
      },
      ErrorOccurrence {
        id: "E0002".to_owned(),
        code: ErrorCode::ImageCaptureFailed,
        source: ErrorSource::DittoPlayer,
        message: "failure frame unavailable".to_owned(),
        job_id: Some(JOB_ID.to_owned()),
        player_session_id: Some(PLAYER_SESSION_ID.to_owned()),
        scenario_id: Some(SCENARIO_ID.to_owned()),
        step_index: None,
        log_sequence: Some(9),
      },
    ],
    baseline_writes: vec![],
    artifacts: vec![
      "actual/checkpoint.png".to_owned(),
      "baseline/checkpoint.png".to_owned(),
      "build/build.log".to_owned(),
      "diagnostics/player.log".to_owned(),
      "diff/checkpoint.png".to_owned(),
      "logs/scenario.ndjson".to_owned(),
      "video/out.mp4".to_owned(),
    ],
    performance: None,
  }
}

fn scenario() -> ScenarioResult {
  ScenarioResult {
    id: SCENARIO_ID.to_owned(),
    name: "scenario".to_owned(),
    status: ScenarioStatus::Failed,
    status_reason: None,
    motion: Motion::Controlled,
    duration_ms: 350,
    expired_deadline: None,
    timings: ScenarioTimings {
      startup_ms: Some(50),
      settle_ms: Some(30),
      capture_ms: Some(20),
      baseline_read_ms: Some(10),
      odiff_ms: Some(20),
      reset_ms: Some(20),
      baseline_download_ms: Some(10),
      comparison_ms: Some(20),
      media_ms: Some(30),
      durability_ms: Some(10),
    },
    steps: vec![
      assertion_step(),
      screenshot_step(),
      video_step(),
      StepResult {
        index: 3,
        name: None,
        kind: StepName::Video,
        status: StepStatus::Passed,
        status_reason: None,
        duration_ms: 1,
        expired_deadline: None,
        error_ids: vec![],
        assertion: None,
        screenshot: None,
        video: None,
        performance: None,
      },
    ],
    logs: Some(LogSpan {
      job_id: JOB_ID.to_owned(),
      player_session_id: PLAYER_SESSION_ID.to_owned(),
      first_sequence: 1,
      last_sequence: 9,
      complete: true,
      path: "logs/scenario.ndjson".to_owned(),
    }),
    failure_frame: Some(MediaCapture::Unavailable {
      reason: "capture failed".to_owned(),
      error_id: Some("E0002".to_owned()),
    }),
    recovery: Recovery::Reset,
    performance_attempt: None,
  }
}

fn assertion_step() -> StepResult {
  StepResult {
    index: 0,
    name: Some("ready".to_owned()),
    kind: StepName::Assert,
    status: StepStatus::Passed,
    status_reason: None,
    duration_ms: 5,
    expired_deadline: None,
    error_ids: vec![],
    assertion: Some(AssertionResult {
      object: OBJECT_ID.to_owned(),
      state: ObjectState::Visible,
      expected: true,
      observed: true,
      passed: true,
    }),
    screenshot: None,
    video: None,
    performance: None,
  }
}

fn screenshot_step() -> StepResult {
  StepResult {
    index: 1,
    name: Some("checkpoint".to_owned()),
    kind: StepName::Screenshot,
    status: StepStatus::Failed,
    status_reason: None,
    duration_ms: 20,
    expired_deadline: None,
    error_ids: vec!["E0001".to_owned()],
    assertion: None,
    screenshot: Some(ScreenshotResult::Captured {
      checkpoint: "checkpoint".to_owned(),
      actual: image("actual/checkpoint.png", HASH_A),
      baseline: BaselineOutcome::Loaded {
        image: image("baseline/checkpoint.png", HASH_B),
      },
      comparison: Some(ComparisonOutcome::Mismatch {
        changed_pixels: 1,
        total_pixels: 4,
        settings: comparison(),
        diff: image("diff/checkpoint.png", HASH_D),
      }),
      matched_before_update: None,
      updated: None,
    }),
    video: None,
    performance: None,
  }
}

fn video_step() -> StepResult {
  StepResult {
    index: 2,
    name: Some("motion".to_owned()),
    kind: StepName::Video,
    status: StepStatus::Passed,
    status_reason: None,
    duration_ms: 100,
    expired_deadline: None,
    error_ids: vec![],
    assertion: None,
    screenshot: None,
    video: Some(VideoResult::Encoded {
      path: "video/out.mp4".to_owned(),
      sha256: HASH_D.to_owned(),
      width: 2,
      height: 2,
      frame_rate: 30,
      duration_ms: 100,
      truncated: false,
    }),
    performance: None,
  }
}

fn startup_report() -> StartupReport {
  StartupReport {
    platform: Platform::Macos,
    capture_adapter: "native".to_owned(),
    build_fingerprint: HASH_B.to_owned(),
    source_fingerprint: HASH_A.to_owned(),
    unity_version: "6000.0.50f1".to_owned(),
    diagnostics: true,
    display: Display {
      width: 2,
      height: 2,
      scale: 1.0,
      orientation: None,
      safe_area: [0, 0, 2, 2],
    },
    capabilities: vec![Capability::Click, Capability::Png, Capability::Video],
    determinism_contract: "ditto-v3".to_owned(),
    native_execution_id: None,
    observer_baseline: None,
  }
}

fn acceptance() -> ReviewAcceptance {
  ReviewAcceptance {
    request_id: REQUEST_ID.to_owned(),
    run_id: RUN_ID.to_owned(),
    lock_sha256: Some(HASH_C.to_owned()),
    selections: vec![ReviewSelection {
      profile: "macos-local".to_owned(),
      scenario: "scenario".to_owned(),
      checkpoint: "checkpoint".to_owned(),
      width: 2,
      height: 2,
      actual_sha256: HASH_A.to_owned(),
    }],
  }
}

fn initial_state() -> BaselineStoreState {
  BaselineStoreState {
    generation: 1,
    lock_sha256: HASH_C.to_owned(),
    published_at: "2026-08-28T20:00:00Z".to_owned(),
    live_sha256: vec![HASH_A.to_owned()],
    tombstones: vec![BaselineTombstone {
      sha256: HASH_B.to_owned(),
      removed_at: "2026-08-27T20:00:00Z".to_owned(),
    }],
    cleanup_applied_at: None,
  }
}

fn comparison() -> Comparison {
  Comparison {
    threshold: "0.1".to_owned(),
    anti_alias: true,
    max_changed_percent: "0.5".to_owned(),
  }
}

fn image(path: &str, sha256: &str) -> ImageFile {
  ImageFile {
    path: path.to_owned(),
    sha256: sha256.to_owned(),
    width: 2,
    height: 2,
  }
}

fn empty_timings() -> ScenarioTimings {
  ScenarioTimings {
    startup_ms: None,
    settle_ms: None,
    capture_ms: None,
    baseline_read_ms: None,
    odiff_ms: None,
    reset_ms: None,
    baseline_download_ms: None,
    comparison_ms: None,
    media_ms: None,
    durability_ms: None,
  }
}

fn rejects(valid: &RunResult, mutation: impl FnOnce(&mut RunResult)) {
  let mut invalid = valid.clone();
  mutation(&mut invalid);
  assert!(invalid.validate().is_err());
}

fn round_trip<T>(values: &[T])
where
  T: Debug + DeserializeOwned + PartialEq + Serialize,
{
  for value in values {
    let encoded = serde_json::to_value(value).unwrap();
    let decoded: T = serde_json::from_value(encoded).unwrap();
    assert_eq!(&decoded, value);
  }
}

const RUN_ID: &str = "0197b35f-6c59-7b98-b1f0-a39f5ee54db8";
const PLAYER_SESSION_ID: &str = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";
const JOB_ID: &str = "0197b35f-6e24-75d8-9482-aa6c22a15133";
const SCENARIO_ID: &str = "0197b35f-6f36-76e9-a593-bb7d33b26244";
const OBJECT_ID: &str = "0197b35f-7048-77fa-b6a4-cc8e44c37355";
const REQUEST_ID: &str = "0197b35f-715a-780b-c7b5-dd9f55d48466";
const COMPARISON_RUN_ID: &str = "0197b35f-726c-791c-d8c6-eea066e59577";
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
