use std::{
  fs,
  io::{self, Read, Write},
  path::{Path, PathBuf},
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
  macos_run, maintenance_commands, review_commands, run_progress,
  selection::{self, Disposition},
  watch_commands,
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

#[derive(Clone)]
pub(crate) struct ExecuteOptions {
  pub selection: SelectionOptions,
  pub command: ResultCommand,
  pub update: bool,
  pub bail_after: Option<u32>,
  pub no_build: bool,
  pub filtered: bool,
  pub json: bool,
  pub output: Option<PathBuf>,
  pub review: bool,
  pub watch: bool,
  pub base_source: PathBuf,
  pub fragment_source: Option<PathBuf>,
}

pub(crate) struct CompletedCycle {
  pub result: RunResult,
  pub result_path: PathBuf,
  pub directory: PathBuf,
}

pub(crate) fn run(
  config_path: Option<&Path>,
  options: RunOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let filtered = !options.selection.includes.is_empty() || !options.selection.excludes.is_empty();
  execute(
    suite.clone(),
    ExecuteOptions {
      selection: options.selection,
      command: ResultCommand::Run,
      update: options.update,
      bail_after: options.bail_after,
      no_build: options.no_build,
      filtered,
      json: options.json,
      output: options.output,
      review: options.review,
      watch: options.watch,
      base_source: suite.source,
      fragment_source: None,
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
  let base_source = base.source.clone();
  let (suite, fragment_source) = match options.fragment {
    Some(path) if path == Path::new("-") => {
      let mut source = String::new();
      io::stdin().read_to_string(&mut source)?;
      (
        config::load_fragment(
          &base,
          FragmentInput::StandardInput { source, name: None },
          options.watch,
        )?,
        None,
      )
    }
    Some(path) => {
      let suite = config::load_fragment(&base, FragmentInput::File(path), options.watch)?;
      let source = suite.source.clone();
      (suite, Some(source))
    }
    None => (base, None),
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
      output: options.output,
      review: options.review,
      watch: options.watch,
      base_source,
      fragment_source,
    },
    stdout,
    stderr,
    interrupted,
  )
}

fn execute(
  suite: Suite,
  options: ExecuteOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  if options.watch {
    return watch_commands::execute(suite, options, stdout, stderr, interrupted);
  }
  let completed = execute_cycle(suite.clone(), &options, 1, stdout, stderr, interrupted)?;
  if options.review {
    review_commands::serve(&suite, Some(&completed.result.run_id), stderr, interrupted)?;
  }
  Ok(completed.result.exit_code)
}

pub(crate) fn execute_cycle(
  suite: Suite,
  options: &ExecuteOptions,
  cycle: u32,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<CompletedCycle> {
  execute_cycle_inner(suite, options, cycle, stdout, stderr, interrupted, None)
}

pub(crate) fn execute_watch_cycle(
  suite: Suite,
  options: &ExecuteOptions,
  cycle: u32,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
  runtime: &mut macos_run::WatchRuntime,
) -> Result<CompletedCycle> {
  execute_cycle_inner(
    suite,
    options,
    cycle,
    stdout,
    stderr,
    interrupted,
    Some(runtime),
  )
}

#[allow(clippy::too_many_arguments)]
fn execute_cycle_inner(
  suite: Suite,
  options: &ExecuteOptions,
  cycle: u32,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
  runtime: Option<&mut macos_run::WatchRuntime>,
) -> Result<CompletedCycle> {
  let started = Instant::now();
  let selection = selection::resolve(&suite, &selection_options(&options.selection))?;
  let now = unix_time()?;
  let mut store = RunStore::open(&maintenance_commands::cache_roots(&suite)?.runs)?;
  let mut result = RunResult {
    run_id: Uuid::new_v4().to_string(),
    source_run_id: None,
    lock_sha256: None,
    command: options.command,
    source_command: None,
    cycle,
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
    let execution = match (selection.profile.target(), runtime) {
      (Target::Macos, Some(runtime)) => macos_run::execute_watch(
        &suite,
        &selection,
        macos_options(options),
        &mut result,
        &active,
        interrupted,
        stderr,
        runtime,
      ),
      (Target::Macos, None) => macos_run::execute(
        &suite,
        &selection,
        macos_options(options),
        &mut result,
        &active,
        interrupted,
        stderr,
      ),
      (Target::Webgl, None) => crate::webgl_run::execute(
        &suite,
        &selection,
        macos_options(options),
        &mut result,
        &active,
        interrupted,
        stderr,
      ),
      (Target::Webgl, Some(_)) => Err(anyhow::anyhow!(
        "WebGL watch execution is not available yet"
      )),
      (Target::IosSimulator, None) => crate::ios_run::execute(
        &suite,
        &selection,
        macos_options(options),
        &mut result,
        &active,
        interrupted,
        stderr,
      ),
      (Target::IosSimulator, Some(_)) => Err(anyhow::anyhow!(
        "iOS Simulator watch execution is not available"
      )),
    };
    if let Err(error) = execution {
      infrastructure_error(&mut result, &format!("{error:#}"));
    }
  }
  result.duration_ms = started.elapsed().as_millis() as u64;
  let path = store.finalize(&mut active, result.clone(), now)?;
  result = serde_json::from_slice(&fs::read(&path)?)?;
  emit(&result, &path, options, stdout, stderr)?;
  Ok(CompletedCycle {
    directory: active.path().to_path_buf(),
    result,
    result_path: path,
  })
}

fn macos_options(options: &ExecuteOptions) -> macos_run::Options {
  macos_run::Options {
    command: options.command,
    bail_after: options.bail_after,
    no_build: options.no_build,
    update: options.update,
    filtered: options.filtered,
  }
}

pub(crate) fn emit(
  result: &RunResult,
  result_path: &Path,
  options: &ExecuteOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
) -> Result<()> {
  if let Some(output) = &options.output {
    replace_output(result_path, output)?;
  }
  if options.json {
    stdout.write_all(&result.to_canonical_json_line()?)?;
    stdout.flush()?;
  }
  run_progress::write_handoff(stderr, result, result_path)
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

fn selection_options(options: &SelectionOptions) -> selection::Options {
  selection::Options {
    profile: options.profile.clone(),
    includes: options.includes.clone(),
    excludes: options.excludes.clone(),
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
    StepKind::AccessibilityAssert(_) => StepName::AccessibilityAssert,
    StepKind::AccessibilityAction { .. } => StepName::AccessibilityAction,
    StepKind::Screenshot(_) => StepName::Screenshot,
    StepKind::Video(_) => StepName::Video,
  }
}

pub(crate) fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn replace_output(source: &Path, destination: &Path) -> Result<()> {
  let parent = destination.parent().unwrap_or_else(|| Path::new("."));
  fs::create_dir_all(parent)?;
  let temporary = parent.join(format!(
    ".{}.{}.tmp",
    destination
      .file_name()
      .context("output path has no file name")?
      .to_string_lossy(),
    Uuid::new_v4()
  ));
  fs::copy(source, &temporary)
    .with_context(|| format!("copy result to {}", destination.display()))?;
  fs::OpenOptions::new()
    .write(true)
    .open(&temporary)?
    .sync_all()?;
  fs::rename(&temporary, destination)?;
  self::sync_directory(parent)?;
  Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<()> {
  fs::File::open(path)?.sync_all()?;
  Ok(())
}

#[cfg(windows)]
fn sync_directory(_path: &Path) -> Result<()> {
  Ok(())
}
