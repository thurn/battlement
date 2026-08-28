use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use crate::wire::{
  common::DeadlineKind,
  lifecycle_validation,
  result::{
    BuildDisposition, ErrorOccurrence, JobResult, PlayerSessionResult, Recovery, ResultCommand,
    RunResult, RunStatus, ScenarioResult, ScenarioStatus,
  },
  result_format, result_nested_validation, validation,
};

pub(super) fn validate_run_result(result: &RunResult) -> Result<()> {
  validation::identifier("run_id", &result.run_id)?;
  command_identity(result)?;
  ensure!(result.cycle > 0, "cycle must be positive");
  paired_identity(result.suite.as_deref(), result.profile.as_deref())?;
  result_format::timestamp("started_at", &result.started_at)?;
  ensure!(
    result.exit_code == expected_exit_code(result.status),
    "exit_code does not match status"
  );
  artifacts(&result.artifacts)?;
  let artifacts: BTreeSet<&str> = result.artifacts.iter().map(String::as_str).collect();
  let errors = errors(result)?;
  build(result, &artifacts)?;
  phases(result, &errors, &artifacts)?;
  let sessions = player_sessions(result, &artifacts)?;
  let jobs = jobs(result, &sessions)?;
  scenarios(result, &errors, &artifacts, &jobs, &sessions)?;
  result_nested_validation::baseline_writes(result)?;
  bounded_strings("warning", &result.warnings, 4096)?;
  Ok(())
}

pub(super) fn error_reference<'a>(
  field: &str,
  value: &str,
  errors: &BTreeMap<&'a str, &'a ErrorOccurrence>,
) -> Result<()> {
  ensure!(
    errors.contains_key(value),
    "{field} references an unknown error"
  );
  Ok(())
}

fn command_identity(result: &RunResult) -> Result<()> {
  match result.command {
    ResultCommand::Run => executed_identity(result),
    ResultCommand::Capture => {
      executed_identity(result)?;
      ensure!(result.lock_sha256.is_none(), "capture must not load a lock");
      ensure!(
        result.baseline_writes.is_empty(),
        "capture must not write baselines"
      );
      Ok(())
    }
    ResultCommand::ComparisonOnly => {
      let Some(source_run_id) = &result.source_run_id else {
        anyhow::bail!("comparison-only result requires source_run_id");
      };
      validation::identifier("source_run_id", source_run_id)?;
      ensure!(
        source_run_id != &result.run_id,
        "source_run_id must differ from run_id"
      );
      ensure!(
        matches!(
          result.source_command,
          Some(ResultCommand::Run | ResultCommand::Capture)
        ),
        "comparison-only result requires an executed source_command"
      );
      Ok(())
    }
  }?;
  if let Some(lock_sha256) = &result.lock_sha256 {
    validation::sha256("lock_sha256", lock_sha256)?;
  }
  Ok(())
}

fn executed_identity(result: &RunResult) -> Result<()> {
  ensure!(
    result.source_run_id.is_none(),
    "executed result must not have source_run_id"
  );
  ensure!(
    result.source_command.is_none(),
    "executed result must not have source_command"
  );
  Ok(())
}

fn paired_identity(suite: Option<&str>, profile: Option<&str>) -> Result<()> {
  ensure!(
    suite.is_some() == profile.is_some(),
    "suite and profile must resolve together"
  );
  if let (Some(suite), Some(profile)) = (suite, profile) {
    validation::name("suite", suite)?;
    validation::name("profile", profile)?;
  }
  Ok(())
}

fn expected_exit_code(status: RunStatus) -> u8 {
  match status {
    RunStatus::Passed => 0,
    RunStatus::Failed => 1,
    RunStatus::InfrastructureError => 2,
    RunStatus::Interrupted => 130,
  }
}

fn artifacts(values: &[String]) -> Result<()> {
  ensure!(
    values.windows(2).all(|pair| pair[0] < pair[1]),
    "artifacts must be unique and lexically sorted"
  );
  for value in values {
    result_format::artifact_path("artifact", value)?;
    ensure!(
      value != "result.json" && value != "partial-result.json",
      "internal result files are not retained artifacts"
    );
  }
  Ok(())
}

fn errors(result: &RunResult) -> Result<BTreeMap<&str, &ErrorOccurrence>> {
  let mut errors = BTreeMap::new();
  for (index, error) in result.errors.iter().enumerate() {
    let expected = format!("E{:04}", index + 1);
    ensure!(
      error.id == expected,
      "error IDs must be contiguous in allocation order"
    );
    ensure!(
      errors.insert(error.id.as_str(), error).is_none(),
      "error IDs must be unique"
    );
    ensure!(!error.message.is_empty(), "error message must not be empty");
    ensure!(error.message.len() <= 4096, "error message is too long");
    validate_error_ownership(error, result)?;
  }
  Ok(errors)
}

fn validate_error_ownership(error: &ErrorOccurrence, result: &RunResult) -> Result<()> {
  if let Some(job_id) = &error.job_id {
    validation::identifier("error.job_id", job_id)?;
    ensure!(
      result.jobs.iter().any(|job| job.job_id == *job_id),
      "error references an unknown job"
    );
  }
  if let Some(session_id) = &error.player_session_id {
    validation::identifier("error.player_session_id", session_id)?;
    ensure!(
      result
        .player_sessions
        .iter()
        .any(|session| session.player_session_id == *session_id),
      "error references an unknown player session"
    );
  }
  if let Some(scenario_id) = &error.scenario_id {
    validation::identifier("error.scenario_id", scenario_id)?;
    let Some(scenario) = result
      .scenarios
      .iter()
      .find(|scenario| scenario.id == *scenario_id)
    else {
      anyhow::bail!("error references an unknown scenario");
    };
    if let Some(step_index) = error.step_index {
      ensure!(
        scenario.steps.iter().any(|step| step.index == step_index),
        "error references an unknown step"
      );
    }
  } else {
    ensure!(
      error.step_index.is_none(),
      "step_index requires scenario_id"
    );
  }
  if error.log_sequence.is_some() {
    ensure!(error.job_id.is_some(), "log_sequence requires job_id");
    ensure!(
      error.player_session_id.is_some(),
      "log_sequence requires player_session_id"
    );
  }
  if let (Some(job_id), Some(session_id)) = (&error.job_id, &error.player_session_id) {
    let job = result.jobs.iter().find(|job| job.job_id == *job_id);
    ensure!(
      job.is_some_and(|job| job.player_session_id == *session_id),
      "error job and player session disagree"
    );
  }
  Ok(())
}

fn build(result: &RunResult, artifacts: &BTreeSet<&str>) -> Result<()> {
  let Some(build) = &result.build else {
    return Ok(());
  };
  validation::sha256("build.source_fingerprint", &build.source_fingerprint)?;
  validation::sha256("build.fingerprint", &build.fingerprint)?;
  let ran = matches!(
    build.disposition,
    BuildDisposition::Created | BuildDisposition::Failed
  );
  ensure!(
    build.log_path.is_some() == ran,
    "build log_path must identify exactly a build that ran"
  );
  if let Some(path) = &build.log_path {
    retained_path("build.log_path", path, artifacts)?;
  }
  Ok(())
}

fn phases(
  result: &RunResult,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  for phase in &result.phases {
    unique_error_references("phase.error_ids", &phase.error_ids, errors)?;
    if let Some(path) = &phase.log_path {
      retained_path("phase.log_path", path, artifacts)?;
    }
  }
  Ok(())
}

fn player_sessions<'a>(
  result: &'a RunResult,
  artifacts: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>> {
  let mut sessions = BTreeSet::new();
  for session in &result.player_sessions {
    validate_session(session, artifacts)?;
    ensure!(
      sessions.insert(session.player_session_id.as_str()),
      "player session IDs must be unique"
    );
  }
  Ok(sessions)
}

fn validate_session(session: &PlayerSessionResult, artifacts: &BTreeSet<&str>) -> Result<()> {
  validation::identifier("player_session_id", &session.player_session_id)?;
  lifecycle_validation::startup_report(&session.startup_report)?;
  for path in &session.diagnostic_paths {
    retained_path("player diagnostic path", path, artifacts)?;
  }
  Ok(())
}

fn jobs<'a>(
  result: &'a RunResult,
  sessions: &BTreeSet<&str>,
) -> Result<BTreeMap<&'a str, &'a JobResult>> {
  let mut jobs = BTreeMap::new();
  for job in &result.jobs {
    validation::identifier("job_id", &job.job_id)?;
    validation::identifier("job.player_session_id", &job.player_session_id)?;
    ensure!(
      sessions.contains(job.player_session_id.as_str()),
      "job references an unknown player session"
    );
    ensure!(
      jobs.insert(job.job_id.as_str(), job).is_none(),
      "job IDs must be unique"
    );
    match (job.first_scenario_index, job.last_scenario_index) {
      (None, None) => {}
      (Some(first), Some(last)) => {
        ensure!(first <= last, "job scenario range is reversed");
        ensure!(
          last < result.scenarios.len() as u32,
          "job scenario range exceeds result"
        );
      }
      _ => anyhow::bail!("job scenario range endpoints must be paired"),
    }
  }
  Ok(jobs)
}

fn scenarios(
  result: &RunResult,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
  jobs: &BTreeMap<&str, &JobResult>,
  sessions: &BTreeSet<&str>,
) -> Result<()> {
  let mut ids = BTreeSet::new();
  let mut names = BTreeSet::new();
  for scenario in &result.scenarios {
    validation::identifier("scenario.id", &scenario.id)?;
    validation::name("scenario.name", &scenario.name)?;
    ensure!(
      ids.insert(scenario.id.as_str()),
      "scenario IDs must be unique"
    );
    ensure!(
      names.insert(scenario.name.as_str()),
      "scenario names must be unique"
    );
    scenario_state(scenario)?;
    result_nested_validation::scenario(scenario, result.command, errors, artifacts)?;
    if let Some(logs) = &scenario.logs {
      ensure!(
        jobs.contains_key(logs.job_id.as_str()),
        "log span references an unknown job"
      );
      ensure!(
        sessions.contains(logs.player_session_id.as_str()),
        "log span references an unknown player session"
      );
      ensure!(
        logs.first_sequence <= logs.last_sequence,
        "log span sequence range is reversed"
      );
      retained_path("log span path", &logs.path, artifacts)?;
      let job = jobs[logs.job_id.as_str()];
      ensure!(
        job.player_session_id == logs.player_session_id,
        "log span job and session disagree"
      );
    }
  }
  Ok(())
}

fn scenario_state(scenario: &ScenarioResult) -> Result<()> {
  let unstarted = matches!(
    scenario.status,
    ScenarioStatus::Skipped | ScenarioStatus::NotRun
  );
  ensure!(
    scenario.status_reason.is_some() == unstarted,
    "scenario status_reason does not match status"
  );
  if unstarted {
    validate_status_reason(scenario.status, scenario.status_reason.as_deref())?;
    ensure!(
      scenario.duration_ms == 0,
      "unstarted scenario duration must be zero"
    );
    ensure!(
      scenario.expired_deadline.is_none(),
      "unstarted scenario has no expired deadline"
    );
    ensure!(
      scenario.logs.is_none(),
      "unstarted scenario has no log span"
    );
    ensure!(
      scenario.failure_frame.is_none(),
      "unstarted scenario has no failure frame"
    );
    ensure!(
      scenario.recovery == Recovery::None,
      "unstarted scenario has no recovery"
    );
    ensure!(
      all_timings_none(scenario),
      "unstarted scenario timings must be null"
    );
  }
  ensure!(
    scenario
      .expired_deadline
      .is_none_or(|deadline| matches!(deadline, DeadlineKind::Scenario | DeadlineKind::Run)),
    "scenario expired_deadline must be scenario or run"
  );
  Ok(())
}

fn validate_status_reason(status: ScenarioStatus, reason: Option<&str>) -> Result<()> {
  let Some(reason) = reason else {
    return Ok(());
  };
  ensure!(reason.len() <= 4096, "scenario status_reason is too long");
  match status {
    ScenarioStatus::Skipped => ensure!(
      reason.starts_with("unsupported-input:") || reason.starts_with("unsupported-step:"),
      "skipped scenario requires an unsupported capability reason"
    ),
    ScenarioStatus::NotRun => ensure!(
      reason == "bail" || reason == "run-infrastructure-error",
      "not-run scenario has an unknown reason"
    ),
    _ => unreachable!("reached statuses have no reason"),
  }
  Ok(())
}

fn all_timings_none(scenario: &ScenarioResult) -> bool {
  let timings = &scenario.timings;
  [
    timings.startup_ms,
    timings.reset_ms,
    timings.baseline_download_ms,
    timings.comparison_ms,
    timings.media_ms,
    timings.durability_ms,
  ]
  .iter()
  .all(Option::is_none)
}

pub(super) fn unique_error_references(
  field: &str,
  values: &[String],
  errors: &BTreeMap<&str, &ErrorOccurrence>,
) -> Result<()> {
  let mut unique = BTreeSet::new();
  for value in values {
    error_reference(field, value, errors)?;
    ensure!(unique.insert(value), "{field} must not contain duplicates");
  }
  Ok(())
}

pub(super) fn retained_path(field: &str, path: &str, artifacts: &BTreeSet<&str>) -> Result<()> {
  result_format::artifact_path(field, path)?;
  ensure!(
    artifacts.contains(path),
    "{field} is missing from artifacts"
  );
  Ok(())
}

fn bounded_strings(field: &str, values: &[String], maximum: usize) -> Result<()> {
  for value in values {
    ensure!(!value.is_empty(), "{field} must not be empty");
    ensure!(value.len() <= maximum, "{field} is too long");
  }
  Ok(())
}
