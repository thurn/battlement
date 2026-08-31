//! Production macOS build, launch, materialization, and baseline update flow.

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  io::Write,
  path::{Path, PathBuf},
  sync::{Arc, atomic::AtomicBool},
  time::{Duration, Instant},
};

use anyhow::{Context, Result};
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildCache, DEFAULT_BUILD_CACHE_BYTES},
  build_identity::{CaptureAdapter, NativeInput},
  discovery::{HostDiscovery, Tool},
  host::{Host, SystemHost},
  macos_build::{self, MacosBuildOutcome, MacosBuildRequest, MacosBuildResult, MacosBuildTools},
};

use crate::{
  baseline_manifest::{BaselineManifest, ManifestSnapshot},
  baseline_store::BaselineStore,
  baseline_update::{
    self, BaselineProposal, BaselineUpdateRequest, ScenarioUpdate, ScenarioUpdateStatus,
  },
  cli::BuildOptions,
  config::model::{Baseline, Profile, StepKind, Suite, Target, VideoStep},
  execution_materializer::{self, ExecutionMaterializer},
  image_comparison::OdiffPool,
  job_resolution,
  macos_capture::{self, ImmutableMacosLauncher, MacosCaptureRequest, MacosCaptureTimeouts},
  macos_watch_capture::WarmMacosPlayer,
  maintenance_commands, native_video, run_progress,
  selection::{Disposition, Selection},
  session_server::PlayerSessionRequirements,
  storage_commands,
  wire::{
    common::{ErrorCode, ErrorSource, StepStatus},
    job::Command as JobCommand,
    result::{
      BuildDisposition, BuildResult, ComparisonOutcome, ErrorOccurrence, PhaseName, PhaseResult,
      PhaseStatus, ResultCommand, RunResult, RunStatus, ScenarioResult, ScenarioStatus,
      ScreenshotResult, StepResult,
    },
    run_storage::ActiveRun,
  },
};

pub(crate) fn build(suite: &Suite, options: BuildOptions, stdout: &mut dyn Write) -> Result<u8> {
  let profile_name = options.profile.as_deref().unwrap_or(&suite.default_profile);
  let profile = suite
    .profiles
    .get(profile_name)
    .with_context(|| format!("profile {profile_name:?} does not exist"))?;
  anyhow::ensure!(
    profile.target() == Target::Macos,
    "build currently supports macOS profiles"
  );
  let discovery = HostDiscovery::inspect(
    &SystemHost,
    &maintenance_commands::discovery_request(suite, Target::Macos)?,
  )?;
  let selected = macos_build::select_macos_player(&build_request(suite, &discovery)?, true)?;
  let (build, disposition) = match selected {
    MacosBuildResult::Ready { build, outcome } => (
      build,
      match outcome {
        MacosBuildOutcome::Created => "created",
        MacosBuildOutcome::Reused => "reused",
      },
    ),
    MacosBuildResult::Required { .. } => unreachable!("builds are allowed"),
    MacosBuildResult::Failed(failure) => anyhow::bail!(failure.message),
  };
  let value = serde_json::json!({
    "schema": 1,
    "suite": suite.name,
    "profile": profile_name,
    "source_fingerprint": build.metadata().identity.source_fingerprint,
    "build_fingerprint": build.metadata().identity.fingerprint,
    "disposition": disposition,
    "player_path": build.path(),
  });
  let encoded = serde_json::to_string_pretty(&value)? + "\n";
  if let Some(path) = options.output {
    fs::write(path, &encoded)?;
  }
  if options.json {
    write!(stdout, "{encoded}")?;
  } else {
    writeln!(
      stdout,
      "{} player {disposition}: {}",
      suite.name,
      build.path().display()
    )?;
  }
  Ok(0)
}

pub(crate) struct Options {
  pub command: ResultCommand,
  pub bail_after: Option<u32>,
  pub no_build: bool,
  pub update: bool,
  pub filtered: bool,
}

/// Warm process resources retained only by one watch invocation.
#[derive(Default)]
pub(crate) struct WatchRuntime {
  player: Option<WarmMacosPlayer>,
  player_fingerprint: Option<String>,
  odiff: Arc<OdiffPool>,
}

pub(crate) struct BaselineInputs {
  pub manifest: Option<BaselineManifest>,
  pub store: Option<Box<dyn BaselineStore>>,
  pub lock_sha256: Option<String>,
}

pub(crate) fn execute(
  suite: &Suite,
  selection: &Selection,
  options: Options,
  result: &mut RunResult,
  active: &ActiveRun,
  interrupted: &AtomicBool,
  progress: &mut dyn Write,
) -> Result<()> {
  execute_inner(
    suite,
    selection,
    options,
    result,
    active,
    interrupted,
    progress,
    None,
  )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_watch(
  suite: &Suite,
  selection: &Selection,
  options: Options,
  result: &mut RunResult,
  active: &ActiveRun,
  interrupted: &AtomicBool,
  progress: &mut dyn Write,
  runtime: &mut WatchRuntime,
) -> Result<()> {
  execute_inner(
    suite,
    selection,
    options,
    result,
    active,
    interrupted,
    progress,
    Some(runtime),
  )
}

#[allow(clippy::too_many_arguments)]
fn execute_inner(
  suite: &Suite,
  selection: &Selection,
  options: Options,
  result: &mut RunResult,
  active: &ActiveRun,
  interrupted: &AtomicBool,
  progress: &mut dyn Write,
  mut runtime: Option<&mut WatchRuntime>,
) -> Result<()> {
  writeln!(progress, "DITTO_PHASE=discovery")?;
  let discovery = HostDiscovery::inspect(
    &SystemHost,
    &maintenance_commands::discovery_request(suite, Target::Macos)?,
  )?;
  let video_requirement = video_requirement(selection)?;
  if let Some(required) = video_requirement {
    let available = SystemHost.available_bytes(active.path())?;
    if let Err(error) = native_video::ensure_available(required, available) {
      fail_media_preflight(result, &error.to_string());
      return Ok(());
    }
  }
  let ffmpeg = video_requirement
    .map(|_| required_tool(&discovery.ffmpeg))
    .transpose()?;
  let request = build_request(suite, &discovery)?;
  let build_started = Instant::now();
  let selected = macos_build::select_macos_player(&request, !options.no_build)?;
  let build_duration = build_started.elapsed().as_millis() as u64;
  let (build, disposition) = match selected {
    MacosBuildResult::Ready { build, outcome } => (
      build,
      match outcome {
        MacosBuildOutcome::Created => BuildDisposition::Created,
        MacosBuildOutcome::Reused => BuildDisposition::Reused,
      },
    ),
    MacosBuildResult::Required { identity, nearest } => {
      writeln!(progress, "DITTO_BUILD=required-by-no-build")?;
      let message = run_progress::no_build_message(&identity.fingerprint, nearest.as_ref());
      result.build = Some(BuildResult {
        source_fingerprint: identity.source_fingerprint,
        fingerprint: identity.fingerprint,
        disposition: BuildDisposition::RequiredByNoBuild,
        duration_ms: build_duration,
        log_path: None,
      });
      fail_build(result, &message, None, build_duration);
      if runtime.is_some() {
        result.scenarios.clear();
      }
      return Ok(());
    }
    MacosBuildResult::Failed(failure) => {
      writeln!(progress, "DITTO_BUILD=failed")?;
      let relative = "build/build.log".to_owned();
      copy_file(&failure.log_path, &active.path().join(&relative))?;
      result.build = Some(BuildResult {
        source_fingerprint: failure.identity.source_fingerprint,
        fingerprint: failure.identity.fingerprint,
        disposition: BuildDisposition::Failed,
        duration_ms: build_duration,
        log_path: Some(relative.clone()),
      });
      fail_build(result, &failure.message, Some(relative), build_duration);
      if runtime.is_some() {
        result.scenarios.clear();
      }
      return Ok(());
    }
  };
  writeln!(
    progress,
    "DITTO_BUILD={}",
    run_progress::build_label(disposition)
  )?;
  let build_log = if disposition == BuildDisposition::Created {
    let relative = "build/build.log".to_owned();
    copy_file(
      &build.path().join(BUILD_LOG_FILE),
      &active.path().join(&relative),
    )?;
    Some(relative)
  } else {
    None
  };
  result.build = Some(BuildResult {
    source_fingerprint: build.metadata().identity.source_fingerprint.clone(),
    fingerprint: build.metadata().identity.fingerprint.clone(),
    disposition,
    duration_ms: build_duration,
    log_path: build_log.clone(),
  });
  let baseline = baseline_inputs(suite, options.command, selection_has_screenshots(selection))?;
  result.lock_sha256 = baseline.lock_sha256;
  let job = job_resolution::resolve(
    selection,
    &suite.aliases,
    match options.command {
      ResultCommand::Run => JobCommand::Run,
      ResultCommand::Capture => JobCommand::Capture,
      ResultCommand::ComparisonOnly => unreachable!(),
    },
    &result.run_id,
    &build.metadata().identity.fingerprint,
    &build.metadata().identity.source_fingerprint,
    suite.timeouts.run.as_millis(),
  )?;
  let roots = maintenance_commands::cache_roots(suite)?;
  let materializer = Arc::new(ExecutionMaterializer::new(
    execution_materializer::Options {
      run_directory: active.path().to_path_buf(),
      profile: selection.profile_name.clone(),
      command: options.command,
      manifest: baseline.manifest,
      store: baseline.store,
      baseline_cache: roots.baselines,
      odiff_binary: discovery.odiff.path.clone(),
      odiff: runtime.as_ref().map(|runtime| runtime.odiff.clone()),
      comparison_timeout: Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      source_fingerprint: build.metadata().identity.source_fingerprint.clone(),
      ffmpeg_binary: ffmpeg,
      video_resolver: None,
    },
  ));
  writeln!(progress, "DITTO_PHASE=scenarios")?;
  let capture_request = MacosCaptureRequest {
    build: &build,
    job,
    requirements: PlayerSessionRequirements {
      origin: None,
      capture_adapter: "native-screen-capture".to_owned(),
      unity_version: request.tools.unity_version.clone(),
      diagnostics: true,
      storage_directory: active.path().to_path_buf(),
    },
    orchestration_path: active.path().join("orchestration.json"),
    player_log_source: active.path().join(".player.log"),
    bail_after: options.bail_after,
    timeouts: MacosCaptureTimeouts {
      launch: Duration::from_millis(suite.timeouts.launch.as_millis()),
      startup: Duration::from_millis(suite.timeouts.launch.as_millis()),
      shutdown: Duration::from_secs(10),
      interrupt_grace: Duration::from_secs(2),
      poll_interval: Duration::from_millis(10),
    },
  };
  let capture = match runtime.as_mut() {
    Some(runtime) => runtime.capture(capture_request, materializer.clone(), interrupted)?,
    None => macos_capture::capture_macos(
      capture_request,
      &ImmutableMacosLauncher,
      materializer.clone(),
      interrupted,
    )?,
  };
  capture.apply_to(result);
  merge_scenarios(result, capture.orchestration.scenarios);
  result.errors = materializer.errors();
  result.phases.insert(
    0,
    PhaseResult {
      name: PhaseName::Build,
      status: PhaseStatus::Passed,
      duration_ms: build_duration,
      expired_deadline: None,
      log_path: build_log,
      error_ids: Vec::new(),
    },
  );
  if options.update && !materializer.proposals().is_empty() {
    apply_update(
      suite,
      selection,
      options.filtered,
      materializer.proposals(),
      result,
    )?;
  }
  reduce_status(result);
  Ok(())
}

impl WatchRuntime {
  pub(crate) fn odiff(&self) -> Arc<OdiffPool> {
    self.odiff.clone()
  }

  fn capture(
    &mut self,
    request: MacosCaptureRequest<'_>,
    materializer: Arc<dyn crate::scenario_orchestration::ScenarioMaterializer>,
    interrupted: &AtomicBool,
  ) -> Result<macos_capture::MacosCaptureOutcome> {
    let fingerprint = request.job.profile.build_fingerprint.clone();
    if self.player_fingerprint.as_deref() != Some(&fingerprint) {
      if let Some(player) = self.player.take() {
        player.shutdown();
      }
      self.player_fingerprint = None;
    }
    if let Some(player) = self.player.as_mut()
      && !player.is_alive()?
    {
      self.player.take();
      self.player_fingerprint = None;
    }
    if let Some(mut player) = self.player.take() {
      return match player.execute(request, materializer, interrupted) {
        Ok(outcome) => {
          self.player = Some(player);
          Ok(outcome)
        }
        Err(error) => {
          player.shutdown();
          self.player_fingerprint = None;
          Err(error)
        }
      };
    }
    let launched =
      WarmMacosPlayer::launch(request, &ImmutableMacosLauncher, materializer, interrupted)?;
    if launched.player.is_some() {
      self.player_fingerprint = Some(fingerprint);
    }
    self.player = launched.player;
    Ok(launched.outcome)
  }
}

impl Drop for WatchRuntime {
  fn drop(&mut self) {
    if let Some(player) = self.player.take() {
      player.shutdown();
    }
  }
}

fn build_request(suite: &Suite, discovery: &HostDiscovery) -> Result<MacosBuildRequest> {
  let unity_editor = required_tool(&discovery.unity)?;
  let cargo = SystemHost
    .find_executable("cargo")
    .context("Cargo was not found")?;
  let rustc = SystemHost
    .find_executable("rustc")
    .context("rustc was not found")?;
  let xcrun = discovery
    .apple
    .iter()
    .find(|tool| tool.name == "xcrun")
    .context("xcrun discovery is missing")?;
  let xcodebuild = discovery
    .apple
    .iter()
    .find(|tool| tool.name == "xcodebuild")
    .context("xcodebuild discovery is missing")?;
  Ok(MacosBuildRequest {
    repository: suite.repository.clone(),
    unity_project: suite.player.unity_project.clone(),
    rust_manifest: suite.player.rust_manifest.clone(),
    scene: suite.player.scene.clone(),
    suite: suite.name.clone(),
    diagnostics: true,
    generated_inputs: Vec::new(),
    native_inputs: Vec::<NativeInput>::new(),
    capture_adapter: CaptureAdapter {
      name: "native-screen-capture".to_owned(),
      version: "1".to_owned(),
    },
    tools: MacosBuildTools {
      unity_editor,
      unity_version: maintenance_commands::unity_version(&suite.player.unity_project)?,
      cargo: cargo.clone(),
      cargo_version: SystemHost.command_output(&cargo, &["--version"])?,
      rustc_version: SystemHost.command_output(&rustc, &["--version"])?,
      architecture: SystemHost.architecture(),
      xcode_version: xcodebuild
        .version
        .clone()
        .context("xcodebuild version is unavailable")?,
      sdk_version: SystemHost.command_output(
        &required_tool(xcrun)?,
        &["--sdk", "macosx", "--show-sdk-version"],
      )?,
    },
    resource_slots: discovery.caches.resource_slots.clone(),
    cache: BuildCache::open(&discovery.caches.builds, DEFAULT_BUILD_CACHE_BYTES)?,
  })
}

pub(crate) fn required_tool(tool: &Tool) -> Result<PathBuf> {
  tool.path.clone().filter(|_| tool.ready()).with_context(|| {
    format!(
      "{} is unavailable: {}",
      tool.name,
      tool.problem.as_deref().unwrap_or("not found")
    )
  })
}

pub(crate) fn baseline_inputs(
  suite: &Suite,
  command: ResultCommand,
  selection_has_screenshots: bool,
) -> Result<BaselineInputs> {
  if command == ResultCommand::Capture || !selection_has_screenshots {
    return Ok(BaselineInputs {
      manifest: None,
      store: None,
      lock_sha256: None,
    });
  }
  let snapshot = ManifestSnapshot::read(&lock_path(suite))?;
  let store = suite
    .baseline
    .as_ref()
    .map(|_| storage_commands::read_store(suite))
    .transpose()?;
  Ok(BaselineInputs {
    manifest: snapshot.manifest,
    store,
    lock_sha256: snapshot.sha256,
  })
}

pub(crate) fn selection_has_screenshots(selection: &Selection) -> bool {
  selection.scenarios.iter().any(|selected| {
    selected.scenario.steps.iter().any(|step| {
      matches!(step.action, StepKind::Screenshot(_))
        && selected.disposition == Disposition::Runnable
    })
  })
}

fn video_requirement(selection: &Selection) -> Result<Option<u64>> {
  let Profile::Macos { display } = &selection.profile else {
    return Ok(None);
  };
  let durations = selection
    .scenarios
    .iter()
    .filter(|selected| selected.disposition == Disposition::Runnable)
    .flat_map(|selected| &selected.scenario.steps)
    .filter_map(|step| match &step.action {
      StepKind::Video(VideoStep::Start { max_duration, .. }) => Some(max_duration.as_millis()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let mut required: Option<u64> = None;
  for duration in durations {
    required = Some(
      required
        .unwrap_or_default()
        .max(native_video::required_bytes(
          display.width,
          display.height,
          duration,
        )?),
    );
  }
  Ok(required)
}

fn fail_media_preflight(result: &mut RunResult, message: &str) {
  let error_id = "E0001".to_owned();
  result.errors.push(ErrorOccurrence {
    id: error_id.clone(),
    code: ErrorCode::MediaInsufficientSpace,
    source: ErrorSource::Filesystem,
    message: message.to_owned(),
    job_id: None,
    player_session_id: None,
    scenario_id: None,
    step_index: None,
    log_sequence: None,
  });
  result.phases.push(PhaseResult {
    name: PhaseName::Scenarios,
    status: PhaseStatus::Failed,
    duration_ms: 0,
    expired_deadline: Some(crate::wire::common::DeadlineKind::Media),
    log_path: None,
    error_ids: vec![error_id],
  });
  result.status = RunStatus::InfrastructureError;
  result.exit_code = 2;
}

pub(crate) fn apply_update(
  suite: &Suite,
  selection: &Selection,
  filtered: bool,
  proposals: Vec<BaselineProposal>,
  result: &mut RunResult,
) -> Result<()> {
  let baseline = suite
    .baseline
    .as_ref()
    .context("--update requires a baseline store")?;
  let namespace = match baseline {
    Baseline::Filesystem { namespace, .. } | Baseline::R2 { namespace, .. } => namespace,
  };
  let proposals = proposals.into_iter().fold(
    BTreeMap::<String, Vec<_>>::new(),
    |mut grouped, proposal| {
      grouped
        .entry(proposal.scenario.clone())
        .or_default()
        .push(proposal);
      grouped
    },
  );
  let updates = result
    .scenarios
    .iter()
    .map(|scenario| ScenarioUpdate {
      name: scenario.name.clone(),
      status: update_status(scenario.status, &scenario.steps),
      proposals: proposals.get(&scenario.name).cloned().unwrap_or_default(),
    })
    .collect::<Vec<_>>();
  let authored = suite
    .scenarios
    .iter()
    .map(|scenario| {
      (
        scenario.name.clone(),
        scenario
          .steps
          .iter()
          .filter_map(|step| match &step.action {
            StepKind::Screenshot(value) => Some(value.name.clone()),
            _ => None,
          })
          .collect::<BTreeSet<_>>(),
      )
    })
    .collect::<BTreeMap<_, _>>();
  let applied = baseline_update::apply(
    storage_commands::write_store(suite)?.as_ref(),
    BaselineUpdateRequest {
      lock_path: &lock_path(suite),
      starting_lock_sha256: result.lock_sha256.clone(),
      suite: &suite.name,
      namespace,
      profile: &selection.profile_name,
      filtered,
      authored_checkpoints: &authored,
      scenarios: &updates,
    },
  )
  .map_err(anyhow::Error::from)?;
  result.lock_sha256 = Some(applied.lock_sha256);
  result.baseline_writes = applied.writes;
  mark_updated(result);
  Ok(())
}

fn update_status(status: ScenarioStatus, steps: &[StepResult]) -> ScenarioUpdateStatus {
  if matches!(status, ScenarioStatus::Skipped | ScenarioStatus::NotRun) {
    ScenarioUpdateStatus::RuntimeSkipped
  } else if steps
    .iter()
    .filter(|step| step.status != StepStatus::Passed)
    .all(|step| matches!(step.screenshot, Some(ScreenshotResult::Captured { .. })))
  {
    ScenarioUpdateStatus::Eligible
  } else {
    ScenarioUpdateStatus::Failed
  }
}

fn mark_updated(result: &mut RunResult) {
  for scenario in &mut result.scenarios {
    for step in &mut scenario.steps {
      let Some(ScreenshotResult::Captured {
        comparison,
        matched_before_update,
        updated,
        ..
      }) = &mut step.screenshot
      else {
        continue;
      };
      let matched = matches!(comparison, Some(ComparisonOutcome::Passed { .. }));
      *matched_before_update = Some(matched);
      *updated = Some(!matched);
      if !matched {
        step.status = StepStatus::Passed;
        step.error_ids.clear();
      }
    }
    if scenario
      .steps
      .iter()
      .all(|step| step.status == StepStatus::Passed)
    {
      scenario.status = ScenarioStatus::Passed;
    }
  }
}

pub(crate) fn fail_build(
  result: &mut RunResult,
  message: &str,
  log_path: Option<String>,
  duration_ms: u64,
) {
  result.errors.push(ErrorOccurrence {
    id: "E0001".to_owned(),
    code: ErrorCode::BuildFailed,
    source: ErrorSource::Ditto,
    message: message.to_owned(),
    job_id: None,
    player_session_id: None,
    scenario_id: None,
    step_index: None,
    log_sequence: None,
  });
  result.phases.push(PhaseResult {
    name: PhaseName::Build,
    status: PhaseStatus::Failed,
    duration_ms,
    expired_deadline: None,
    log_path,
    error_ids: vec!["E0001".to_owned()],
  });
  result.status = RunStatus::InfrastructureError;
  result.exit_code = 2;
}

pub(crate) fn merge_scenarios(result: &mut RunResult, reached: Vec<ScenarioResult>) {
  for scenario in reached {
    if let Some(current) = result
      .scenarios
      .iter_mut()
      .find(|current| current.name == scenario.name)
    {
      *current = scenario;
    }
  }
}

pub(crate) fn reduce_status(result: &mut RunResult) {
  if result.status == RunStatus::Interrupted {
    result.exit_code = 130;
  } else if result.status == RunStatus::InfrastructureError {
    result.exit_code = 2;
  } else if result
    .scenarios
    .iter()
    .any(|scenario| scenario.status == ScenarioStatus::InfrastructureError)
  {
    result.status = RunStatus::InfrastructureError;
    result.exit_code = 2;
  } else if result
    .scenarios
    .iter()
    .any(|scenario| scenario.status == ScenarioStatus::Failed)
  {
    result.status = RunStatus::Failed;
    result.exit_code = 1;
  } else {
    result.status = RunStatus::Passed;
    result.exit_code = 0;
  }
}

pub(crate) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
  if let Some(parent) = destination.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::copy(source, destination)?;
  Ok(())
}

fn lock_path(suite: &Suite) -> PathBuf {
  suite
    .source
    .parent()
    .expect("suite source has a parent")
    .join("ditto.lock")
}

#[cfg(test)]
#[path = "macos_run_tests.rs"]
mod tests;
