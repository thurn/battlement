//! Production WebGL build, launch, materialization, and baseline update flow.

use std::{
  fs,
  io::Write,
  path::Path,
  sync::{Arc, atomic::AtomicBool},
  time::{Duration, Instant},
};

use anyhow::{Context, Result};
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildCache, DEFAULT_BUILD_CACHE_BYTES},
  build_identity::{CaptureAdapter, NativeInput},
  discovery::HostDiscovery,
  host::{Host, SystemHost},
  webgl_build::{self, WebglBuildOutcome, WebglBuildRequest, WebglBuildResult, WebglBuildTools},
};

use crate::{
  config::model::{Profile, Suite, Target},
  execution_materializer::{self, ExecutionMaterializer},
  image_comparison::OdiffPool,
  job_resolution, macos_run, maintenance_commands, reactant_assets, run_progress,
  selection::Selection,
  session_server::PlayerSessionRequirements,
  webgl_capture::{self, LocalWebglLauncher, WebglCaptureRequest, WebglCaptureTimeouts},
  wire::{
    common::{DeadlineKind, ErrorCode, ErrorSource},
    job::Command as JobCommand,
    result::{
      BuildDisposition, BuildResult, ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus,
      ResultCommand, RunResult, RunStatus,
    },
    run_storage::ActiveRun,
  },
};

pub(crate) fn execute(
  suite: &Suite,
  selection: &Selection,
  options: macos_run::Options,
  result: &mut RunResult,
  active: &ActiveRun,
  interrupted: &AtomicBool,
  progress: &mut dyn Write,
) -> Result<()> {
  writeln!(progress, "DITTO_PHASE=discovery")?;
  let discovery = HostDiscovery::inspect(
    &SystemHost,
    &maintenance_commands::discovery_request(suite, Target::Webgl)?,
  )?;
  let request = self::build_request(suite, &discovery)?;
  let build_started = Instant::now();
  let selected = webgl_build::select_webgl_player(&request, !options.no_build)?;
  let build_duration = build_started.elapsed().as_millis() as u64;
  let (build, disposition) = match selected {
    WebglBuildResult::Ready { build, outcome } => (
      build,
      match outcome {
        WebglBuildOutcome::Created => BuildDisposition::Created,
        WebglBuildOutcome::Reused => BuildDisposition::Reused,
      },
    ),
    WebglBuildResult::Required { identity, nearest } => {
      writeln!(progress, "DITTO_BUILD=required-by-no-build")?;
      let message = run_progress::no_build_message(&identity.fingerprint, nearest.as_ref());
      result.build = Some(BuildResult {
        source_fingerprint: identity.source_fingerprint,
        fingerprint: identity.fingerprint,
        disposition: BuildDisposition::RequiredByNoBuild,
        duration_ms: build_duration,
        log_path: None,
      });
      macos_run::fail_build(result, &message, None, build_duration);
      return Ok(());
    }
    WebglBuildResult::Failed(failure) => {
      writeln!(progress, "DITTO_BUILD=failed")?;
      let relative = "build/build.log".to_owned();
      macos_run::copy_file(&failure.log_path, &active.path().join(&relative))?;
      result.build = Some(BuildResult {
        source_fingerprint: failure.identity.source_fingerprint,
        fingerprint: failure.identity.fingerprint,
        disposition: BuildDisposition::Failed,
        duration_ms: build_duration,
        log_path: Some(relative.clone()),
      });
      macos_run::fail_build(result, &failure.message, Some(relative), build_duration);
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
    macos_run::copy_file(
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
  let baseline = macos_run::baseline_inputs(
    suite,
    options.command,
    macos_run::selection_has_screenshots(selection),
  )?;
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
      odiff: Some(Arc::new(OdiffPool::default())),
      comparison_timeout: Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      source_fingerprint: build.metadata().identity.source_fingerprint.clone(),
      ffmpeg_binary: None,
      video_resolver: None,
    },
  ));
  let headless_command = match &selection.profile {
    Profile::Webgl {
      headless_command, ..
    } => headless_command.as_deref(),
    _ => unreachable!("WebGL run received another profile"),
  };
  writeln!(progress, "DITTO_PHASE=scenarios")?;
  let browser_log_source = active.path().join(".browser.log");
  let capture = webgl_capture::capture_webgl(
    WebglCaptureRequest {
      build: &build,
      job,
      requirements: PlayerSessionRequirements {
        origin: None,
        capture_adapter: "webgl-canvas-png".to_owned(),
        unity_version: request.tools.unity_version.clone(),
        diagnostics: true,
        storage_directory: active.path().to_path_buf(),
      },
      orchestration_path: active.path().join("orchestration.json"),
      browser_log_source: browser_log_source.clone(),
      bail_after: options.bail_after,
      headless_command,
      timeouts: WebglCaptureTimeouts {
        launch: Duration::from_millis(suite.timeouts.launch.as_millis()),
        shutdown: Duration::from_secs(2),
        interrupt_grace: Duration::from_secs(2),
        poll_interval: Duration::from_millis(10),
      },
    },
    &LocalWebglLauncher,
    materializer.clone(),
    interrupted,
  );
  let capture = match capture {
    Ok(capture) => capture,
    Err(error) => {
      self::fail_capture(
        result,
        &format!("{error:#}"),
        self::retain_failed_browser_log(active, &browser_log_source),
        build_duration,
        build_log,
      );
      return Ok(());
    }
  };
  capture.apply_to(result);
  macos_run::merge_scenarios(result, capture.orchestration.scenarios);
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
    macos_run::apply_update(
      suite,
      selection,
      options.filtered,
      materializer.proposals(),
      result,
    )?;
  }
  macos_run::reduce_status(result);
  Ok(())
}

fn retain_failed_browser_log(active: &ActiveRun, source: &Path) -> Option<String> {
  if !source.is_file() {
    return None;
  }
  let relative = "logs/browser-launch.log".to_owned();
  fs::copy(source, active.path().join(&relative))
    .ok()
    .map(|_| relative)
}

fn fail_capture(
  result: &mut RunResult,
  message: &str,
  log_path: Option<String>,
  build_duration: u64,
  build_log: Option<String>,
) {
  let error_id = "E0001".to_owned();
  result.errors.push(ErrorOccurrence {
    id: error_id.clone(),
    code: ErrorCode::LaunchFailed,
    source: ErrorSource::Ditto,
    message: message.chars().take(4096).collect(),
    job_id: None,
    player_session_id: None,
    scenario_id: None,
    step_index: None,
    log_sequence: None,
  });
  result.phases.extend([
    PhaseResult {
      name: PhaseName::Build,
      status: PhaseStatus::Passed,
      duration_ms: build_duration,
      expired_deadline: None,
      log_path: build_log,
      error_ids: Vec::new(),
    },
    PhaseResult {
      name: PhaseName::Launch,
      status: PhaseStatus::Failed,
      duration_ms: 0,
      expired_deadline: message
        .contains("launch deadline expired")
        .then_some(DeadlineKind::Launch),
      log_path,
      error_ids: vec![error_id],
    },
    PhaseResult {
      name: PhaseName::Cleanup,
      status: PhaseStatus::Passed,
      duration_ms: 0,
      expired_deadline: None,
      log_path: None,
      error_ids: Vec::new(),
    },
  ]);
  result.status = RunStatus::InfrastructureError;
  result.exit_code = 2;
}

fn build_request(suite: &Suite, discovery: &HostDiscovery) -> Result<WebglBuildRequest> {
  reactant_assets::generate(suite)?;
  let unity_editor = macos_run::required_tool(&discovery.unity)?;
  let cargo = SystemHost
    .find_executable("cargo")
    .context("Cargo was not found")?;
  let rustc = SystemHost
    .find_executable("rustc")
    .context("rustc was not found")?;
  Ok(WebglBuildRequest {
    repository: suite.repository.clone(),
    unity_project: suite.player.unity_project.clone(),
    rust_manifest: suite.player.rust_manifest.clone(),
    scene: suite.player.scene.clone(),
    suite: suite.name.clone(),
    diagnostics: true,
    generated_inputs: Vec::new(),
    native_inputs: Vec::<NativeInput>::new(),
    capture_adapter: CaptureAdapter {
      name: "webgl-canvas-png".to_owned(),
      version: "1".to_owned(),
    },
    tools: WebglBuildTools {
      unity_editor,
      unity_version: maintenance_commands::unity_version(&suite.player.unity_project)?,
      cargo: cargo.clone(),
      cargo_version: SystemHost.command_output(&cargo, &["--version"])?,
      rustc_version: SystemHost.command_output(&rustc, &["--version"])?,
    },
    resource_slots: discovery.caches.resource_slots.clone(),
    cache: BuildCache::open(&discovery.caches.builds, DEFAULT_BUILD_CACHE_BYTES)?,
  })
}
