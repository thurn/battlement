use std::{
  fs,
  io::{self, Read, Write},
  path::Path,
  sync::atomic::AtomicBool,
  time::Instant,
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
  cli::{CaptureOptions, RunOptions, SelectionOptions},
  config::{
    self, FragmentInput,
    model::{Motion as AuthoredMotion, Scenario, StepKind, Suite, Target},
  },
  macos_run, maintenance_commands, run_progress,
  selection::{self, Disposition},
  wire::{
    common::{ErrorCode, ErrorSource, StepName, StepStatus},
    job::Motion,
    result::{
      ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus, Recovery, ResultCommand, RunResult,
      RunStatus, ScenarioResult, ScenarioStatus, ScenarioTimings, StepResult,
    },
    run_storage::RunStore,
  },
};

struct ExecuteOptions<'a> {
  selection: SelectionOptions,
  command: ResultCommand,
  update: bool,
  bail_after: Option<u32>,
  no_build: bool,
  filtered: bool,
  json: bool,
  output: Option<&'a Path>,
}

pub(crate) fn run(
  config_path: Option<&Path>,
  options: RunOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let filtered = !options.selection.includes.is_empty() || !options.selection.excludes.is_empty();
  execute(
    config::load(config_path)?,
    ExecuteOptions {
      selection: options.selection,
      command: ResultCommand::Run,
      update: options.update,
      bail_after: options.bail_after,
      no_build: options.no_build,
      filtered,
      json: options.json,
      output: options.output.as_deref(),
    },
    stdout,
    stderr,
    interrupted,
  )
}

pub(crate) fn capture(
  config_path: Option<&Path>,
  options: CaptureOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let base = config::load(config_path)?;
  let suite = match options.fragment {
    Some(path) if path == Path::new("-") => {
      let mut source = String::new();
      io::stdin().read_to_string(&mut source)?;
      config::load_fragment(
        &base,
        FragmentInput::StandardInput { source, name: None },
        false,
      )?
    }
    Some(path) => config::load_fragment(&base, FragmentInput::File(path), false)?,
    None => base,
  };
  let filtered = !options.selection.includes.is_empty() || !options.selection.excludes.is_empty();
  execute(
    suite,
    ExecuteOptions {
      selection: options.selection,
      command: ResultCommand::Capture,
      update: false,
      bail_after: options.bail_after,
      no_build: options.no_build,
      filtered,
      json: options.json,
      output: options.output.as_deref(),
    },
    stdout,
    stderr,
    interrupted,
  )
}

fn execute(
  suite: Suite,
  options: ExecuteOptions<'_>,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let started = Instant::now();
  let selection = selection::resolve(&suite, &selection_options(options.selection))?;
  let now = unix_time()?;
  let mut store = RunStore::open(&maintenance_commands::cache_roots(&suite)?.runs)?;
  let mut result = RunResult {
    run_id: Uuid::new_v4().to_string(),
    source_run_id: None,
    lock_sha256: None,
    command: options.command,
    source_command: None,
    cycle: 1,
    suite: Some(suite.name.clone()),
    profile: Some(selection.profile_name.clone()),
    started_at: OffsetDateTime::from_unix_timestamp(now as i64)?.format(&Rfc3339)?,
    duration_ms: 0,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: Vec::new(),
    player_sessions: Vec::new(),
    jobs: Vec::new(),
    scenarios: selection
      .scenarios
      .iter()
      .cloned()
      .map(selected_result)
      .collect::<Result<_>>()?,
    warnings: Vec::new(),
    errors: Vec::new(),
    baseline_writes: Vec::new(),
    artifacts: Vec::new(),
  };
  let mut active = store.begin(result.clone(), stderr, now)?;
  writeln!(stderr, "DITTO_SELECTED={}", selection.scenarios.len())?;
  store.index_identity(&active, &suite.repository, &suite.name, now)?;
  if selection
    .scenarios
    .iter()
    .any(|scenario| scenario.disposition == Disposition::Runnable)
  {
    if selection.profile.target() != Target::Macos {
      infrastructure_error(
        &mut result,
        "the selected platform execution adapter is not available yet",
      );
    } else if let Err(error) = macos_run::execute(
      &suite,
      &selection,
      macos_run::Options {
        command: options.command,
        bail_after: options.bail_after,
        no_build: options.no_build,
        update: options.update,
        filtered: options.filtered,
      },
      &mut result,
      &active,
      interrupted,
      stderr,
    ) {
      infrastructure_error(&mut result, &format!("{error:#}"));
    }
  }
  result.duration_ms = started.elapsed().as_millis() as u64;
  let path = store.finalize(&mut active, result.clone(), now)?;
  result = serde_json::from_slice(&fs::read(&path)?)?;
  if let Some(output) = options.output {
    fs::copy(&path, output).with_context(|| format!("copy result to {}", output.display()))?;
  }
  if options.json {
    stdout.write_all(&result.to_canonical_json_line()?)?;
  }
  run_progress::write_handoff(stderr, &result, &path)?;
  Ok(result.exit_code)
}

fn selected_result(scenario: selection::MaterializedScenario) -> Result<ScenarioResult> {
  match scenario.disposition {
    Disposition::Skipped { reason } => unstarted_result(
      scenario.scenario,
      scenario.run_index,
      ScenarioStatus::Skipped,
      reason,
    ),
    Disposition::Runnable => unstarted_result(
      scenario.scenario,
      scenario.run_index,
      ScenarioStatus::NotRun,
      "run-infrastructure-error".to_owned(),
    ),
  }
}

fn unstarted_result(
  scenario: Scenario,
  _run_index: u32,
  status: ScenarioStatus,
  reason: String,
) -> Result<ScenarioResult> {
  Ok(ScenarioResult {
    id: Uuid::new_v4().to_string(),
    name: scenario.name,
    status,
    status_reason: Some(reason.clone()),
    motion: motion(scenario.motion),
    duration_ms: 0,
    expired_deadline: None,
    timings: ScenarioTimings::default(),
    steps: scenario
      .steps
      .into_iter()
      .enumerate()
      .map(|(index, step)| StepResult {
        index: index as u32,
        name: step.name,
        kind: step_name(&step.action),
        status: StepStatus::NotRun,
        status_reason: Some(reason.clone()),
        duration_ms: 0,
        expired_deadline: None,
        error_ids: Vec::new(),
        assertion: None,
        screenshot: None,
        video: None,
      })
      .collect(),
    logs: None,
    failure_frame: None,
    recovery: Recovery::None,
  })
}

fn infrastructure_error(result: &mut RunResult, message: &str) {
  let error_id = format!("E{:04}", result.errors.len() + 1);
  result.errors.push(ErrorOccurrence {
    id: error_id.clone(),
    code: ErrorCode::LaunchFailed,
    source: ErrorSource::Ditto,
    message: message.to_owned(),
    job_id: None,
    player_session_id: None,
    scenario_id: None,
    step_index: None,
    log_sequence: None,
  });
  result.phases.push(PhaseResult {
    name: PhaseName::Cleanup,
    status: PhaseStatus::Failed,
    duration_ms: 0,
    expired_deadline: None,
    log_path: None,
    error_ids: vec![error_id],
  });
  result.status = RunStatus::InfrastructureError;
  result.exit_code = 2;
}

fn selection_options(options: SelectionOptions) -> selection::Options {
  selection::Options {
    profile: options.profile,
    includes: options.includes,
    excludes: options.excludes,
    allow_empty: options.allow_empty,
  }
}

fn motion(value: AuthoredMotion) -> Motion {
  match value {
    AuthoredMotion::Instant => Motion::Instant,
    AuthoredMotion::Controlled => Motion::Controlled,
    AuthoredMotion::RealTime => Motion::RealTime,
  }
}

fn step_name(value: &StepKind) -> StepName {
  match value {
    StepKind::Click { .. } => StepName::Click,
    StepKind::Hover { .. } => StepName::Hover,
    StepKind::Drag { .. } => StepName::Drag,
    StepKind::Key { .. } => StepName::Key,
    StepKind::Wait(_) => StepName::Wait,
    StepKind::Assert(_) => StepName::Assert,
    StepKind::Screenshot(_) => StepName::Screenshot,
    StepKind::Video(_) => StepName::Video,
  }
}

fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
