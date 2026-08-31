use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::Result;
use battlement_ditto::{
  macos_capture::{MacosCaptureRequest, MacosCaptureTimeouts, MacosPlayerLauncher, capture_macos},
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
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildAccess, BuildCache, BuildHandle, SOURCE_MANIFEST_FILE},
  build_identity::{
    AppleToolchain, BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, RustToolchain,
  },
  fingerprint::SourceManifest,
  macos_build::{MacosStartupIdentity, STARTUP_IDENTITY_FILE},
};
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn fixture_completes_three_scenarios_and_writes_a_valid_result() {
  let build = FixtureBuild::new(true);
  let run = tempfile::tempdir().unwrap();
  let launcher = FixtureLauncher::new(run.path(), json!({}), "complete");
  let outcome = capture_macos(
    request(&build.handle, run.path(), 3),
    &launcher,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap();

  assert_eq!(launcher.count.load(Ordering::SeqCst), 1);
  assert_eq!(outcome.exit_code, 0);
  assert_eq!(outcome.orchestration.scenarios.len(), 3);
  assert_eq!(outcome.orchestration.jobs.len(), 1);
  assert!(outcome.player_exit.unwrap().code == Some(0));
  let diagnostic = outcome.player_session.as_ref().unwrap().diagnostic_paths[0].clone();
  assert!(run.path().join(&diagnostic).is_file());

  let mut result = empty_result();
  outcome.apply_to(&mut result);
  result.artifacts = vec!["logs/events.jsonl".to_owned(), diagnostic];
  result.validate().unwrap();
  fs::write(
    run.path().join("result.json"),
    result.to_canonical_json().unwrap(),
  )
  .unwrap();
  let retained: RunResult =
    serde_json::from_slice(&fs::read(run.path().join("result.json")).unwrap()).unwrap();
  assert_eq!(retained.status, RunStatus::Passed);
  assert_eq!(retained.scenarios.len(), 3);
  assert_eq!(retained.player_sessions.len(), 1);
}

#[test]
fn every_startup_mismatch_stops_before_scenario_setup() {
  let build = FixtureBuild::new(true);
  let profile = job(&build.handle, 1).profile;
  let cases = [
    (
      "display",
      json!({"display": {"width": 640, "height": 720, "scale": 1.0, "orientation": null, "safe_area": [0, 0, 640, 720]}}),
    ),
    (
      "build",
      json!({"build_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}),
    ),
    (
      "source",
      json!({"source_fingerprint": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}),
    ),
    ("diagnostics", json!({"diagnostics": false})),
    ("adapter", json!({"capture_adapter": "wrong-adapter"})),
    ("capability", json!({"capabilities": []})),
    ("unity", json!({"unity_version": "6000.0.99f1"})),
  ];
  for (name, override_value) in cases {
    let run = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(run.path(), override_value, "complete");
    let mut capture = request(&build.handle, run.path(), 1);
    capture.job.profile = profile.clone();
    let outcome = capture_macos(
      capture,
      &launcher,
      Arc::new(PassMaterializer),
      &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(outcome.exit_code, 2, "{name}");
    assert!(!outcome.player_session.unwrap().accepted, "{name}");
    assert!(outcome.orchestration.jobs.is_empty(), "{name}");
    assert!(!run.path().join("setup").exists(), "{name}");
  }
}

#[test]
fn diagnostics_disabled_build_never_starts_and_interrupt_is_bounded() {
  let disabled = FixtureBuild::new(false);
  let rejected_run = tempfile::tempdir().unwrap();
  let rejected = FixtureLauncher::new(rejected_run.path(), json!({}), "complete");
  let error = capture_macos(
    request(&disabled.handle, rejected_run.path(), 1),
    &rejected,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap_err();
  assert!(error.to_string().contains("diagnostics disabled"));
  assert_eq!(rejected.count.load(Ordering::SeqCst), 0);

  let build = FixtureBuild::new(true);
  let run = tempfile::tempdir().unwrap();
  let launcher = FixtureLauncher::new(run.path(), json!({}), "idle");
  let interrupted = Arc::new(AtomicBool::new(false));
  let signal = interrupted.clone();
  thread::spawn(move || {
    thread::sleep(Duration::from_millis(100));
    signal.store(true, Ordering::Release);
  });
  let started = Instant::now();
  let outcome = capture_macos(
    request(&build.handle, run.path(), 1),
    &launcher,
    Arc::new(PassMaterializer),
    interrupted.as_ref(),
  )
  .unwrap();
  assert_eq!(outcome.exit_code, 130);
  assert!(started.elapsed() < Duration::from_secs(2));
  assert!(run.path().join("logs").read_dir().unwrap().count() == 1);
}

struct FixtureLauncher {
  script: PathBuf,
  log: PathBuf,
  setup: PathBuf,
  override_value: String,
  mode: &'static str,
  count: Arc<AtomicUsize>,
}

impl FixtureLauncher {
  fn new(run: &Path, override_value: serde_json::Value, mode: &'static str) -> Self {
    let script = run.join("player.py");
    fs::write(&script, PLAYER).unwrap();
    Self {
      script,
      log: run.join("source-player.log"),
      setup: run.join("setup"),
      override_value: serde_json::to_string(&override_value).unwrap(),
      mode,
      count: Arc::new(AtomicUsize::new(0)),
    }
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
  ) -> Result<Child> {
    fs::write(log_path, b"fixture player log\n")?;
    self.count.fetch_add(1, Ordering::SeqCst);
    Ok(
      Command::new("/usr/bin/python3")
        .arg(&self.script)
        .arg(session_url)
        .env("DITTO_FIXTURE_LOG", &self.log)
        .env("DITTO_FIXTURE_SETUP", &self.setup)
        .env("DITTO_FIXTURE_OVERRIDE", &self.override_value)
        .env("DITTO_FIXTURE_MODE", self.mode)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?,
    )
  }
}

struct FixtureBuild {
  _temporary: TempDir,
  handle: BuildHandle,
}

impl FixtureBuild {
  fn new(diagnostics: bool) -> Self {
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

struct PassMaterializer;

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
  }
}

fn request<'a>(build: &'a BuildHandle, run: &Path, count: u32) -> MacosCaptureRequest<'a> {
  MacosCaptureRequest {
    build,
    job: job(build, count),
    requirements: PlayerSessionRequirements {
      origin: None,
      capture_adapter: "native-screen-capture".to_owned(),
      unity_version: "6000.0.56f1".to_owned(),
      diagnostics: true,
      storage_directory: run.to_owned(),
    },
    orchestration_path: run.join("orchestration.json"),
    player_log_source: run.join("source-player.log"),
    bail_after: None,
    timeouts: MacosCaptureTimeouts {
      launch: Duration::from_secs(2),
      startup: Duration::from_secs(2),
      shutdown: Duration::from_secs(1),
      interrupt_grace: Duration::from_millis(500),
      poll_interval: Duration::from_millis(5),
    },
  }
}

fn job(build: &BuildHandle, count: u32) -> Job {
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
    },
    scenarios: (0..count)
      .map(|index| ResolvedScenario {
        id: Uuid::new_v4().to_string(),
        run_index: index,
        name: format!("scenario {index}"),
        fixture: None,
        motion: Motion::Instant,
        timeout_ms: 500,
        steps: vec![ResolvedStep {
          index: 0,
          name: None,
          timeout_ms: 100,
          action: StepKind::Click {
            target: InputTarget::Coordinates([0.5, 0.5]),
            settle: true,
          },
        }],
      })
      .collect(),
  }
}

fn empty_result() -> RunResult {
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
}
report.update(json.loads(os.environ['DITTO_FIXTURE_OVERRIDE']))
started = {
    'job_id': job['job_id'],
    'run_id': job['run_id'],
    'player_session_id': session,
    'first_log_sequence': 0,
    'startup_failure': None,
    'startup_log_failure': None,
    'identity': {'startup_report': report},
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
"#;
