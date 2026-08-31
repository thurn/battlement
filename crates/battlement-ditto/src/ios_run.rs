//! Production iOS Simulator build, launch, materialization, and baseline flow.

use std::{
  io::Write,
  path::PathBuf,
  sync::{Arc, Mutex, atomic::AtomicBool},
  time::{Duration, Instant},
};

use anyhow::{Context, Result};
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildCache, DEFAULT_BUILD_CACHE_BYTES},
  build_identity::{CaptureAdapter, NativeInput},
  discovery::HostDiscovery,
  host::{Host, SystemHost},
  ios_build::{self, IosBuildOutcome, IosBuildRequest, IosBuildResult, IosBuildTools},
};

use crate::{
  config::model::{Profile, StepKind, Suite, Target, VideoStep},
  execution_materializer::{self, ExecutionMaterializer, NativeVideoResolver},
  image_comparison::OdiffPool,
  ios_capture::{self, IosCaptureRequest, IosCaptureTimeouts},
  ios_simulator::{self, IosSimulator, SimulatorTools},
  job_resolution, macos_run, maintenance_commands, native_video, reactant_assets, run_progress,
  selection::{Disposition, Selection},
  session_server::PlayerSessionRequirements,
  wire::{
    common::{DeadlineKind, ErrorCode, ErrorSource},
    job::Command as JobCommand,
    lifecycle::NativeVideoInput,
    result::{
      BuildDisposition, BuildResult, ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus,
      ResultCommand, RunResult, RunStatus,
    },
    run_storage::ActiveRun,
  },
};

struct SimulatorVideoResolver {
  simulator: Arc<Mutex<IosSimulator>>,
  directory: PathBuf,
}

impl NativeVideoResolver for SimulatorVideoResolver {
  fn resolve(&self, input: &NativeVideoInput) -> Result<NativeVideoInput> {
    let destination = self.directory.join(format!("{}.raw", input.input_id));
    self
      .simulator
      .lock()
      .unwrap()
      .copy_recording(&input.path, &destination)?;
    let mut resolved = input.clone();
    resolved.path = destination.to_string_lossy().into_owned();
    Ok(resolved)
  }
}

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
    &maintenance_commands::discovery_request(suite, Target::IosSimulator)?,
  )?;
  let request = self::build_request(suite, &discovery)?;
  let build_started = Instant::now();
  let selected = ios_build::select_ios_player(&request, !options.no_build)?;
  let build_duration = build_started.elapsed().as_millis() as u64;
  let (build, disposition) = match selected {
    IosBuildResult::Ready { build, outcome } => (
      build,
      match outcome {
        IosBuildOutcome::Created => BuildDisposition::Created,
        IosBuildOutcome::Reused => BuildDisposition::Reused,
      },
    ),
    IosBuildResult::Required { identity, nearest } => {
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
    IosBuildResult::Failed(failure) => {
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

  let (device, orientation) = match &selection.profile {
    Profile::IosSimulator {
      device,
      orientation,
    } => (device.as_str(), *orientation),
    _ => unreachable!("iOS run received another profile"),
  };
  let xcrun = self::apple_tool(&discovery, "xcrun")?;
  let boot_started = Instant::now();
  let (simulator, facts) = match IosSimulator::create(
    SimulatorTools {
      xcrun: xcrun.clone(),
      plutil: PathBuf::from("/usr/bin/plutil"),
      command_timeout: Duration::from_secs(30),
      boot_timeout: Duration::from_millis(suite.timeouts.launch.as_millis()),
    },
    device,
    &result.run_id,
  ) {
    Ok(value) => value,
    Err(error) => {
      self::fail_adapter(
        result,
        PhaseName::SimulatorBoot,
        &format!("{error:#}"),
        build_duration,
        build_log,
      );
      return Ok(());
    }
  };
  let facts = ios_simulator::orient_display(facts, orientation);
  let boot_duration = boot_started.elapsed().as_millis() as u64;
  let simulator = Arc::new(Mutex::new(simulator));
  let required = self::video_requirement(selection, facts.display.width, facts.display.height)?;
  if let Some(required) = required {
    let available = SystemHost.available_bytes(active.path())?;
    if let Err(error) = native_video::ensure_available(required, available) {
      self::fail_adapter(
        result,
        PhaseName::Scenarios,
        &error.to_string(),
        build_duration,
        build_log,
      );
      return Ok(());
    }
  }
  let ffmpeg = required
    .map(|_| macos_run::required_tool(&discovery.ffmpeg))
    .transpose()?;
  let baseline = macos_run::baseline_inputs(
    suite,
    options.command,
    macos_run::selection_has_screenshots(selection),
  )?;
  result.lock_sha256 = baseline.lock_sha256;
  let job = job_resolution::resolve_ios(
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
    facts.display,
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
      ffmpeg_binary: ffmpeg,
      video_resolver: Some(Arc::new(SimulatorVideoResolver {
        simulator: simulator.clone(),
        directory: active.path().join(".simulator-video"),
      })),
    },
  ));
  let capture = ios_capture::capture_ios(
    IosCaptureRequest {
      build: &build,
      simulator,
      orientation,
      job,
      requirements: PlayerSessionRequirements {
        origin: None,
        capture_adapter: "native-screen-capture".to_owned(),
        unity_version: request.tools.unity_version.clone(),
        diagnostics: true,
        storage_directory: active.path().to_path_buf(),
      },
      orchestration_path: active.path().join("orchestration.json"),
      bail_after: options.bail_after,
      timeouts: IosCaptureTimeouts {
        startup: Duration::from_millis(suite.timeouts.launch.as_millis()),
        shutdown: Duration::from_secs(10),
        interrupt_grace: Duration::from_secs(2),
        poll_interval: Duration::from_millis(25),
      },
    },
    materializer.clone(),
    interrupted,
  );
  let capture = match capture {
    Ok(capture) => capture,
    Err(error) => {
      self::fail_adapter(
        result,
        PhaseName::Launch,
        &format!("{error:#}"),
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
      name: PhaseName::SimulatorBoot,
      status: PhaseStatus::Passed,
      duration_ms: boot_duration,
      expired_deadline: None,
      log_path: None,
      error_ids: Vec::new(),
    },
  );
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
  if capture.exit_code == 0 {
    macos_run::reduce_status(result);
  }
  Ok(())
}

fn build_request(suite: &Suite, discovery: &HostDiscovery) -> Result<IosBuildRequest> {
  reactant_assets::generate(suite)?;
  let unity_editor = macos_run::required_tool(&discovery.unity)?;
  let cargo = SystemHost
    .find_executable("cargo")
    .context("Cargo was not found")?;
  let rustc = SystemHost
    .find_executable("rustc")
    .context("rustc was not found")?;
  let xcrun = self::apple_tool(discovery, "xcrun")?;
  let xcodebuild = discovery
    .apple
    .iter()
    .find(|tool| tool.name == "xcodebuild")
    .context("xcodebuild discovery is missing")?;
  Ok(IosBuildRequest {
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
    tools: IosBuildTools {
      unity_editor,
      unity_version: maintenance_commands::unity_version(&suite.player.unity_project)?,
      cargo: cargo.clone(),
      cargo_version: SystemHost.command_output(&cargo, &["--version"])?,
      rustc_version: SystemHost.command_output(&rustc, &["--version"])?,
      architecture: SystemHost.architecture(),
      xcodebuild: macos_run::required_tool(xcodebuild)?,
      xcode_version: xcodebuild
        .version
        .clone()
        .context("xcodebuild version is unavailable")?,
      sdk_version: SystemHost
        .command_output(&xcrun, &["--sdk", "iphonesimulator", "--show-sdk-version"])?,
    },
    resource_slots: discovery.caches.resource_slots.clone(),
    cache: BuildCache::open(&discovery.caches.builds, DEFAULT_BUILD_CACHE_BYTES)?,
  })
}

fn apple_tool(discovery: &HostDiscovery, name: &str) -> Result<PathBuf> {
  discovery
    .apple
    .iter()
    .find(|tool| tool.name == name)
    .with_context(|| format!("{name} discovery is missing"))
    .and_then(macos_run::required_tool)
}

fn video_requirement(selection: &Selection, width: u32, height: u32) -> Result<Option<u64>> {
  let mut required: Option<u64> = None;
  for duration in selection
    .scenarios
    .iter()
    .filter(|selected| selected.disposition == Disposition::Runnable)
    .flat_map(|selected| &selected.scenario.steps)
    .filter_map(|step| match &step.action {
      StepKind::Video(VideoStep::Start { max_duration, .. }) => Some(max_duration.as_millis()),
      _ => None,
    })
  {
    required = Some(
      required
        .unwrap_or_default()
        .max(native_video::required_bytes(width, height, duration)?),
    );
  }
  Ok(required)
}

fn fail_adapter(
  result: &mut RunResult,
  phase: PhaseName,
  message: &str,
  build_duration: u64,
  build_log: Option<String>,
) {
  let error_id = "E0001".to_owned();
  result.errors.push(ErrorOccurrence {
    id: error_id.clone(),
    code: if phase == PhaseName::SimulatorBoot {
      ErrorCode::SimulatorBootFailed
    } else {
      ErrorCode::LaunchFailed
    },
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
      name: phase,
      status: PhaseStatus::Failed,
      duration_ms: 0,
      expired_deadline: Some(if phase == PhaseName::SimulatorBoot {
        DeadlineKind::SimulatorBoot
      } else {
        DeadlineKind::Launch
      }),
      log_path: None,
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
