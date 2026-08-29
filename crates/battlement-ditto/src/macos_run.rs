//! Production macOS build, launch, materialization, and baseline update flow.

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
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
  config::model::{Baseline, StepKind, Suite, Target},
  execution_materializer::{self, ExecutionMaterializer},
  job_resolution,
  macos_capture::{self, ImmutableMacosLauncher, MacosCaptureRequest, MacosCaptureTimeouts},
  maintenance_commands,
  selection::Selection,
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

pub(crate) struct Options {
  pub command: ResultCommand,
  pub bail_after: Option<u32>,
  pub no_build: bool,
  pub update: bool,
  pub filtered: bool,
}

struct BaselineInputs {
  manifest: Option<BaselineManifest>,
  store: Option<Box<dyn BaselineStore>>,
  lock_sha256: Option<String>,
}

pub(crate) fn execute(
  suite: &Suite,
  selection: &Selection,
  options: Options,
  result: &mut RunResult,
  active: &ActiveRun,
  interrupted: &AtomicBool,
) -> Result<()> {
  let discovery = HostDiscovery::inspect(
    &SystemHost,
    &maintenance_commands::discovery_request(suite, Target::Macos)?,
  )?;
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
    MacosBuildResult::Required(identity) => {
      result.build = Some(BuildResult {
        source_fingerprint: identity.source_fingerprint,
        fingerprint: identity.fingerprint,
        disposition: BuildDisposition::RequiredByNoBuild,
        duration_ms: build_duration,
        log_path: None,
      });
      fail_build(
        result,
        "the exact player build is not cached and --no-build was supplied",
        None,
        build_duration,
      );
      return Ok(());
    }
    MacosBuildResult::Failed(failure) => {
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
      return Ok(());
    }
  };
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
  let baseline = baseline_inputs(suite, options.command)?;
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
      comparison_timeout: Duration::from_millis(suite.timeouts.baseline_download.as_millis()),
      source_fingerprint: build.metadata().identity.source_fingerprint.clone(),
    },
  ));
  let capture = macos_capture::capture_macos(
    MacosCaptureRequest {
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
    },
    &ImmutableMacosLauncher,
    materializer.clone(),
    interrupted,
  )?;
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
  if options.update {
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

fn required_tool(tool: &Tool) -> Result<PathBuf> {
  tool.path.clone().filter(|_| tool.ready()).with_context(|| {
    format!(
      "{} is unavailable: {}",
      tool.name,
      tool.problem.as_deref().unwrap_or("not found")
    )
  })
}

fn baseline_inputs(suite: &Suite, command: ResultCommand) -> Result<BaselineInputs> {
  if command == ResultCommand::Capture {
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

fn apply_update(
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

fn fail_build(result: &mut RunResult, message: &str, log_path: Option<String>, duration_ms: u64) {
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

fn merge_scenarios(result: &mut RunResult, reached: Vec<ScenarioResult>) {
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

fn reduce_status(result: &mut RunResult) {
  if result.status == RunStatus::Interrupted {
    result.exit_code = 130;
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

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
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
