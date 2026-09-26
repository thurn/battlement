use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  sync::{
    Arc, Barrier,
    atomic::{AtomicU32, AtomicUsize, Ordering},
  },
  time::Duration,
};

use crate::ditto::{
  macos_capture::{MacosCaptureRequest, MacosCaptureTimeouts, MacosPlayerLauncher},
  native_execution::NativeExecution,
  scenario_orchestration::{MaterializedScenario, ScenarioMaterializer},
  session_server::PlayerSessionRequirements,
  wire::{
    common::StepStatus,
    job::{
      Capability, Command as JobCommand, Display, InputTarget, Job, Motion, Platform,
      ResolvedProfile, ResolvedScenario, ResolvedStep, StepKind,
    },
    lifecycle::{PlayerStepResult, ScenarioBoundaryOutcome, ScenarioComplete},
    result::{
      LogSpan, Recovery, ResultCommand, RunResult, RunStatus, ScenarioResult, ScenarioStatus,
      ScenarioTimings, StepResult,
    },
  },
};
use anyhow::Result;
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildAccess, BuildCache, BuildHandle, SOURCE_MANIFEST_FILE},
  build_identity::{
    AppleToolchain, BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, RustToolchain,
  },
  fingerprint::SourceManifest,
  macos_build::{MacosStartupIdentity, STARTUP_IDENTITY_FILE},
};
use tempfile::TempDir;
use uuid::Uuid;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub(crate) struct FixtureLauncher {
  script: PathBuf,
  log: PathBuf,
  setup: PathBuf,
  override_value: String,
  mode: &'static str,
  pub(crate) count: Arc<AtomicUsize>,
  pub(crate) pid: AtomicU32,
  launch_barrier: Option<Arc<Barrier>>,
}

impl FixtureLauncher {
  pub(crate) fn new(run: &Path, override_value: serde_json::Value, mode: &'static str) -> Self {
    let script = run.join("player.py");
    fs::write(&script, PLAYER).unwrap();
    Self {
      script,
      log: run.join("source-player.log"),
      setup: run.join("setup"),
      override_value: serde_json::to_string(&override_value).unwrap(),
      mode,
      count: Arc::new(AtomicUsize::new(0)),
      pid: AtomicU32::new(0),
      launch_barrier: None,
    }
  }

  pub(crate) fn with_launch_barrier(mut self, barrier: Arc<Barrier>) -> Self {
    self.launch_barrier = Some(barrier);
    self
  }
}

impl MacosPlayerLauncher for FixtureLauncher {
  fn launch(
    &self,
    _executable: &Path,
    session_url: &str,
    log_path: &Path,
    _width: u32,
    _height: u32,
    _native_execution_id: Option<&str>,
  ) -> Result<Child> {
    fs::write(log_path, b"fixture player log\n")?;
    self.count.fetch_add(1, Ordering::SeqCst);
    if let Some(barrier) = &self.launch_barrier {
      barrier.wait();
    }
    let child = Command::new(env::var_os("BATTLEMENT_PYTHON").unwrap_or_else(|| {
      if cfg!(windows) {
        "python3".into()
      } else {
        "/usr/bin/python3".into()
      }
    }))
    .arg(&self.script)
    .arg(session_url)
    .env("DITTO_FIXTURE_LOG", &self.log)
    .env("DITTO_FIXTURE_SETUP", &self.setup)
    .env("DITTO_FIXTURE_OVERRIDE", &self.override_value)
    .env("DITTO_FIXTURE_MODE", self.mode)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()?;
    self.pid.store(child.id(), Ordering::SeqCst);
    Ok(child)
  }
}

pub(crate) struct FixtureBuild {
  _temporary: TempDir,
  pub(crate) handle: BuildHandle,
}

impl FixtureBuild {
  pub(crate) fn new(diagnostics: bool) -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let cache = BuildCache::open(temporary.path().join("cache"), 10_000_000).unwrap();
    let identity = BuildIdentity::derive(&BuildIdentityRequest {
      source_fingerprint: HASH.to_owned(),
      target: BuildTarget::Macos,
      unity_version: "6000.0.56f1".to_owned(),
      rust: RustToolchain {
        rustc_version: "rustc fixture".to_owned(),
        cargo_version: "cargo fixture".to_owned(),
        target: "aarch64-apple-darwin".to_owned(),
      },
      apple: Some(AppleToolchain {
        xcode_version: "Xcode fixture".to_owned(),
        sdk_version: "macOS fixture".to_owned(),
      }),
      diagnostics,
      capture_adapter: CaptureAdapter {
        name: "native-screen-capture".to_owned(),
        version: "1".to_owned(),
      },
      native_inputs: vec![],
      options: BTreeMap::new(),
    })
    .unwrap();
    let BuildAccess::Build(pending) = cache
      .acquire(&temporary.path().to_string_lossy(), "fixture", &identity, 1)
      .unwrap()
    else {
      panic!("new fixture unexpectedly reused a build")
    };
    fs::write(pending.path().join(BUILD_LOG_FILE), b"fixture build\n").unwrap();
    SourceManifest {
      fingerprint: HASH.to_owned(),
      entries: vec![],
    }
    .write(&pending.path().join(SOURCE_MANIFEST_FILE))
    .unwrap();
    let startup = MacosStartupIdentity {
      platform: "macos".to_owned(),
      capture_adapter: "native-screen-capture".to_owned(),
      build_fingerprint: identity.fingerprint.clone(),
      source_fingerprint: HASH.to_owned(),
      unity_version: "6000.0.56f1".to_owned(),
      diagnostics,
    };
    fs::write(
      pending.path().join(STARTUP_IDENTITY_FILE),
      serde_json::to_vec(&startup).unwrap(),
    )
    .unwrap();
    let player = pending.path().join("BattlementDitto.app/Contents/MacOS");
    fs::create_dir_all(&player).unwrap();
    fs::write(player.join("BattlementDitto"), b"fixture\n").unwrap();
    let handle = pending
      .publish(Path::new("BattlementDitto.app"), 1)
      .unwrap()
      .build;
    Self {
      _temporary: temporary,
      handle,
    }
  }
}

pub(crate) struct PassMaterializer;

impl ScenarioMaterializer for PassMaterializer {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> Result<MaterializedScenario> {
    let expected = job
      .scenarios
      .iter()
      .find(|scenario| scenario.id == complete.scenario_id)
      .unwrap();
    Ok(MaterializedScenario {
      result: ScenarioResult {
        id: expected.id.clone(),
        name: expected.name.clone(),
        status: ScenarioStatus::Passed,
        status_reason: None,
        motion: expected.motion,
        duration_ms: complete.execution_duration_ms,
        expired_deadline: None,
        timings: ScenarioTimings {
          startup_ms: Some(complete.startup_duration_ms),
          reset_ms: Some(match complete.boundary {
            ScenarioBoundaryOutcome::Passed { duration_ms }
            | ScenarioBoundaryOutcome::Failed { duration_ms, .. } => duration_ms,
          }),
          durability_ms: Some(0),
          ..ScenarioTimings::default()
        },
        steps: complete.steps.iter().map(step_result).collect(),
        logs: Some(LogSpan {
          job_id: job.job_id.clone(),
          player_session_id: String::new(),
          first_sequence: complete.last_log_sequence,
          last_sequence: complete.last_log_sequence,
          complete: true,
          path: "logs/events.jsonl".to_owned(),
        }),
        failure_frame: None,
        recovery,
        performance_attempt: expected.performance.clone(),
      },
      primary_failure: None,
    })
  }
}

fn step_result(player: &PlayerStepResult) -> StepResult {
  StepResult {
    index: player.index,
    name: player.name.clone(),
    kind: player.kind,
    status: StepStatus::Passed,
    status_reason: None,
    duration_ms: player.duration_ms,
    expired_deadline: None,
    error_ids: vec![],
    assertion: None,
    screenshot: None,
    video: None,
    performance: player.performance.clone(),
    input_trace: player.input_trace.clone(),
  }
}

pub(crate) fn request<'a>(
  build: &'a BuildHandle,
  run: &Path,
  count: u32,
) -> MacosCaptureRequest<'a> {
  let native_execution = Arc::new(NativeExecution::create());
  let native_execution_id = native_execution.id().to_owned();
  MacosCaptureRequest {
    build,
    job: job(build, count, &native_execution_id),
    requirements: PlayerSessionRequirements {
      origin: None,
      capture_adapter: "native-screen-capture".to_owned(),
      unity_version: "6000.0.56f1".to_owned(),
      diagnostics: true,
      storage_directory: run.to_owned(),
      native_execution_id: Some(native_execution_id),
    },
    orchestration_path: run.join("orchestration.json"),
    player_log_source: run.join("source-player.log"),
    resource_slots: run.join("resource-slots"),
    bail_after: None,
    timeouts: MacosCaptureTimeouts {
      launch: Duration::from_secs(2),
      startup: Duration::from_secs(2),
      shutdown: Duration::from_secs(1),
      interrupt_grace: Duration::from_millis(500),
      poll_interval: Duration::from_millis(5),
    },
    native_execution,
  }
}

fn job(build: &BuildHandle, count: u32, native_execution_id: &str) -> Job {
  Job {
    job_id: Uuid::new_v4().to_string(),
    run_id: Uuid::new_v4().to_string(),
    remaining_run_timeout_ms: 2_000,
    log_redactions: vec![],
    command: JobCommand::Capture,
    profile: ResolvedProfile {
      name: "macos-local".to_owned(),
      platform: Platform::Macos,
      display: Display {
        width: 1280,
        height: 720,
        scale: 1.0,
        orientation: None,
        safe_area: [0, 0, 1280, 720],
      },
      build_fingerprint: build.metadata().identity.fingerprint.clone(),
      source_fingerprint: HASH.to_owned(),
      capabilities: vec![Capability::Click],
      determinism_contract: "ditto-v3".to_owned(),
      native_execution_id: Some(native_execution_id.to_owned()),
    },
    scenarios: (0..count)
      .map(|index| ResolvedScenario {
        id: Uuid::new_v4().to_string(),
        run_index: index,
        name: format!("scenario {index}"),
        fixture: None,
        motion: Motion::Instant,
        timeout_ms: 500,
        performance: None,
        steps: vec![ResolvedStep {
          index: 0,
          name: None,
          timeout_ms: 100,
          measure: false,
          action: StepKind::Click {
            target: InputTarget::Object("4aac8ca0-af3d-409e-958e-62954e6cb3d1".to_owned()),
          },
        }],
      })
      .collect(),
  }
}

pub(crate) fn empty_result() -> RunResult {
  RunResult {
    run_id: Uuid::new_v4().to_string(),
    source_run_id: None,
    lock_sha256: None,
    command: ResultCommand::Capture,
    source_command: None,
    cycle: 1,
    suite: Some("fixture".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-29T00:00:00Z".to_owned(),
    duration_ms: 1,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: vec![],
    player_sessions: vec![],
    jobs: vec![],
    scenarios: vec![],
    warnings: vec![],
    errors: vec![],
    baseline_writes: vec![],
    artifacts: vec![],
    performance: None,
  }
}

const PLAYER: &str = r#"import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request

base = sys.argv[1].rstrip('/')
log_path = os.environ['DITTO_FIXTURE_LOG']
open(log_path, 'w').write('fixture player log\n')
if os.environ['DITTO_FIXTURE_MODE'] == 'exit-before-startup':
    sys.exit(7)
if os.environ['DITTO_FIXTURE_MODE'] == 'no-startup':
    open(os.environ['DITTO_FIXTURE_SETUP'] + '-startup', 'w').write('waiting')
    while True:
        time.sleep(0.01)

def send(method, path, value=None, content_type='application/json', headers=None):
    body = None if value is None else (value if isinstance(value, bytes) else json.dumps(value).encode())
    request = urllib.request.Request(base + '/' + path, data=body, method=method)
    if body is not None:
        request.add_header('Content-Type', content_type)
    for name, content in (headers or {}).items():
        request.add_header(name, content)
    return urllib.request.urlopen(request, timeout=1)

response = send('GET', 'job')
session = response.headers['X-Ditto-Player-Session-Id']
job = json.load(response)
report = {
    'platform': 'macos',
    'capture_adapter': 'native-screen-capture',
    'build_fingerprint': job['profile']['build_fingerprint'],
    'source_fingerprint': job['profile']['source_fingerprint'],
    'unity_version': '6000.0.56f1',
    'diagnostics': True,
    'display': job['profile']['display'],
    'capabilities': job['profile']['capabilities'],
    'determinism_contract': job['profile']['determinism_contract'],
    'native_execution_id': job['profile']['native_execution_id'],
}
report.update(json.loads(os.environ['DITTO_FIXTURE_OVERRIDE']))
mode = os.environ['DITTO_FIXTURE_MODE']
for cycle in range(2 if mode.startswith('warm-') else 1):
    if cycle == 1:
        boundary = 'before' if '-before-' in mode else 'after'
        if boundary == 'after':
            job = json.load(send('GET', 'next-job?after=' + job['job_id']))
        open(os.environ['DITTO_FIXTURE_SETUP'] + '-warm', 'w').write('waiting')
        if mode.endswith('-exit'):
            sys.exit(7)
        if mode != 'warm-complete':
            while True:
                time.sleep(0.01)
    started = {
        'job_id': job['job_id'],
        'run_id': job['run_id'],
        'player_session_id': session,
        'first_log_sequence': 0,
        'startup_failure': None,
        'startup_log_failure': None,
        'identity': {'startup_report': report} if cycle == 0 else {'accepted_player_session_id': session},
    }
    decision = json.load(send('POST', 'jobs/' + job['job_id'] + '/started', started))
    if decision['action'] != 'continue':
        sys.exit(0)
    open(os.environ['DITTO_FIXTURE_SETUP'], 'w').write('setup\n')
    if os.environ['DITTO_FIXTURE_MODE'] == 'idle':
        while True:
            try:
                send('GET', 'job').close()
                time.sleep(0.02)
            except urllib.error.HTTPError:
                sys.exit(0)

    executed = []
    for sequence, scenario in enumerate(job['scenarios']):
        event = {
            'schema': 1,
            'job_id': job['job_id'],
            'player_session_id': session,
            'sequence': sequence,
            'timestamp_unix_us': 1787953800000000 + sequence,
            'source': 'ditto-player',
            'severity': 'information',
            'event_name': 'fixture.step',
            'message': 'fixture step completed',
            'fields': {},
            'exception': None,
            'stack_trace': None,
        }
        events = (json.dumps(event, separators=(',', ':')) + '\n').encode()
        send(
            'PUT',
            'jobs/' + job['job_id'] + '/logs/' + session + '?first_sequence=' + str(sequence),
            events,
            'application/x-ndjson',
            {'X-Ditto-SHA256': hashlib.sha256(events).hexdigest()},
        ).close()
        step = scenario['steps'][0]
        complete = {
            'scenario_id': scenario['id'],
            'execution_status': 'passed',
            'steps': [{
                'index': 0,
                'name': step['name'],
                'kind': 'click',
                'status': 'passed',
                'duration_ms': 1,
                'expired_deadline': None,
                'error_refs': [],
                'assertion': None,
                'screenshot_artifact_id': None,
                'video_input_id': None,
            }],
            'artifacts': [],
            'failure_frame': None,
            'video_inputs': [],
            'last_log_sequence': sequence,
            'execution_duration_ms': 1,
            'startup_duration_ms': 1,
            'settle_duration_ms': 2,
            'capture_duration_ms': 3,
            'boundary': {'status': 'passed', 'duration_ms': 1},
            'primary_error_ref': None,
        }
        decision = json.load(send('POST', 'jobs/' + job['job_id'] + '/scenarios/' + scenario['id'] + '/complete', complete))
        if decision['action'] != 'continue':
            raise RuntimeError('unexpected scenario decision')
        executed.append(scenario['id'])

    complete = {
        'job_id': job['job_id'],
        'last_log_sequence': len(job['scenarios']) - 1,
        'executed_scenario_ids': executed,
        'unstarted_scenarios': [],
        'reason': 'completed',
        'execution_duration_ms': len(job['scenarios']),
    }
    send('POST', 'jobs/' + job['job_id'] + '/complete', complete).close()
if mode == 'warm-complete':
    try:
        send('GET', 'next-job?after=' + job['job_id']).close()
    except urllib.error.HTTPError:
        pass

"#;
