use std::{
  collections::BTreeMap,
  fs,
  path::Path,
  sync::{Arc, atomic::AtomicBool},
  time::Duration,
};

use battlement_ditto::{
  scenario_orchestration::{MaterializedScenario, ScenarioMaterializer},
  session_server::PlayerSessionRequirements,
  webgl_capture::{
    LocalWebglLauncher, WebglCaptureRequest, WebglCaptureTimeouts, WebglLaunch,
    WebglPlayerLauncher, capture_webgl,
  },
  wire::{
    common::StepStatus,
    job::{
      Capability, Command as JobCommand, Display, InputTarget, Job, Motion, Platform,
      ResolvedProfile, ResolvedScenario, ResolvedStep, StepKind,
    },
    lifecycle::{PlayerStepResult, ScenarioBoundaryOutcome, ScenarioComplete},
    result::{LogSpan, Recovery, ScenarioResult, ScenarioStatus, ScenarioTimings, StepResult},
  },
};
use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildAccess, BuildCache, BuildHandle, SOURCE_MANIFEST_FILE},
  build_identity::{
    BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, RustToolchain,
  },
  fingerprint::SourceManifest,
  webgl_build::{STARTUP_IDENTITY_FILE, WebglStartupIdentity},
};
use tempfile::TempDir;
use uuid::Uuid;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn configured_headless_player_completes_through_the_same_origin_launcher() {
  let build = FixtureBuild::new();
  let run = tempfile::tempdir().unwrap();
  let script = run.path().join("browser.py");
  fs::write(&script, PLAYER).unwrap();
  let command = vec![
    "/usr/bin/python3".to_owned(),
    script.to_string_lossy().into_owned(),
    "{url}".to_owned(),
  ];
  let outcome = capture_webgl(
    request(&build.handle, run.path(), Some(&command)),
    &LocalWebglLauncher,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap();

  assert_eq!(outcome.exit_code, 0);
  assert_eq!(outcome.orchestration.scenarios.len(), 1);
  assert_eq!(outcome.player_exit.unwrap().code, Some(0));
  let session = outcome.player_session.unwrap();
  assert!(session.accepted);
  assert_eq!(session.startup_report.platform, Platform::Webgl);
  assert_eq!(session.diagnostic_paths.len(), 1);
  assert!(
    fs::read_to_string(run.path().join(&session.diagnostic_paths[0]))
      .unwrap()
      .contains("responsive browser fixture")
  );
}

#[test]
fn supervised_exit_and_unobservable_launch_deadline_are_bounded() {
  let build = FixtureBuild::new();
  let exited = tempfile::tempdir().unwrap();
  let command = vec!["/usr/bin/false".to_owned(), "{url}".to_owned()];
  let error = capture_webgl(
    request(&build.handle, exited.path(), Some(&command)),
    &LocalWebglLauncher,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap_err();
  assert!(error.to_string().contains("exited before startup"));

  let deadline = tempfile::tempdir().unwrap();
  let mut timed = request(&build.handle, deadline.path(), None);
  timed.timeouts.launch = Duration::from_millis(30);
  let error = capture_webgl(
    timed,
    &UnobservableLauncher,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap_err();
  assert!(error.to_string().contains("launch deadline expired"));
}

struct UnobservableLauncher;

impl WebglPlayerLauncher for UnobservableLauncher {
  fn launch(&self, _: &str, _: Option<&[String]>, _: &Path) -> anyhow::Result<WebglLaunch> {
    Ok(WebglLaunch::operating_system())
  }
}

struct FixtureBuild {
  _temporary: TempDir,
  handle: BuildHandle,
}

impl FixtureBuild {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let cache = BuildCache::open(temporary.path().join("cache"), 10_000_000).unwrap();
    let identity = BuildIdentity::derive(&BuildIdentityRequest {
      source_fingerprint: HASH.to_owned(),
      target: BuildTarget::Webgl,
      unity_version: "6000.0.56f1".to_owned(),
      rust: RustToolchain {
        rustc_version: "rustc fixture".to_owned(),
        cargo_version: "cargo fixture".to_owned(),
        target: "wasm32-unknown-emscripten".to_owned(),
      },
      apple: None,
      diagnostics: true,
      capture_adapter: CaptureAdapter {
        name: "webgl-canvas-png".to_owned(),
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
    fs::write(
      pending.path().join(STARTUP_IDENTITY_FILE),
      serde_json::to_vec(&WebglStartupIdentity {
        platform: "webgl".to_owned(),
        capture_adapter: "webgl-canvas-png".to_owned(),
        build_fingerprint: identity.fingerprint.clone(),
        source_fingerprint: HASH.to_owned(),
        unity_version: "6000.0.56f1".to_owned(),
        diagnostics: true,
      })
      .unwrap(),
    )
    .unwrap();
    let player = pending.path().join("player/Build");
    fs::create_dir_all(&player).unwrap();
    fs::write(
      pending.path().join("player/index.html"),
      b"<canvas id=\"unity-canvas\"></canvas>",
    )
    .unwrap();
    fs::write(player.join("fixture.loader.js"), b"fixture").unwrap();
    fs::write(player.join("fixture.wasm.unityweb"), b"fixture").unwrap();
    let handle = pending.publish(Path::new("player"), 1).unwrap().build;
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
  ) -> anyhow::Result<MaterializedScenario> {
    let expected = &job.scenarios[0];
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
        steps: complete.steps.iter().map(self::step_result).collect(),
        logs: Some(LogSpan {
          job_id: job.job_id.clone(),
          player_session_id: String::new(),
          first_sequence: 0,
          last_sequence: 0,
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

fn request<'a>(
  build: &'a BuildHandle,
  run: &Path,
  headless_command: Option<&'a [String]>,
) -> WebglCaptureRequest<'a> {
  WebglCaptureRequest {
    build,
    job: job(build),
    requirements: PlayerSessionRequirements {
      origin: None,
      capture_adapter: "webgl-canvas-png".to_owned(),
      unity_version: "6000.0.56f1".to_owned(),
      diagnostics: true,
      storage_directory: run.to_owned(),
    },
    orchestration_path: run.join("orchestration.json"),
    browser_log_source: run.join("browser.log"),
    bail_after: None,
    headless_command,
    timeouts: WebglCaptureTimeouts {
      launch: Duration::from_secs(2),
      shutdown: Duration::from_secs(1),
      interrupt_grace: Duration::from_millis(200),
      poll_interval: Duration::from_millis(5),
    },
  }
}

fn job(build: &BuildHandle) -> Job {
  Job {
    job_id: Uuid::new_v4().to_string(),
    run_id: Uuid::new_v4().to_string(),
    remaining_run_timeout_ms: 2_000,
    log_redactions: vec![],
    command: JobCommand::Capture,
    profile: ResolvedProfile {
      name: "web-ci".to_owned(),
      platform: Platform::Webgl,
      display: Display {
        width: 640,
        height: 360,
        scale: 1.0,
        orientation: None,
        safe_area: [0, 0, 640, 360],
      },
      build_fingerprint: build.metadata().identity.fingerprint.clone(),
      source_fingerprint: HASH.to_owned(),
      capabilities: vec![Capability::Click],
    },
    scenarios: vec![ResolvedScenario {
      id: Uuid::new_v4().to_string(),
      run_index: 0,
      name: "focused web adapter".to_owned(),
      motion: Motion::Controlled,
      timeout_ms: 1_000,
      steps: vec![ResolvedStep {
        index: 0,
        name: None,
        timeout_ms: 100,
        action: StepKind::Click {
          target: InputTarget::Coordinates([0.5, 0.5]),
        },
      }],
    }],
  }
}

const PLAYER: &str = r#"import hashlib
import json
import sys
import urllib.request

launcher = sys.argv[1]
response = urllib.request.urlopen(launcher, timeout=1)
assert response.headers['Cross-Origin-Opener-Policy'] == 'same-origin'
base = launcher.rsplit('/launcher', 1)[0]

def send(method, path, value=None, content_type='application/json', headers=None):
    body = None if value is None else json.dumps(value, separators=(',', ':')).encode()
    request = urllib.request.Request(base + '/' + path, data=body, method=method)
    if body is not None:
        request.add_header('Content-Type', content_type)
    for name, content in (headers or {}).items():
        request.add_header(name, content)
    return urllib.request.urlopen(request, timeout=1)

job_response = send('GET', 'job')
session = job_response.headers['X-Ditto-Player-Session-Id']
job = json.load(job_response)
started = {
    'job_id': job['job_id'],
    'run_id': job['run_id'],
    'player_session_id': session,
    'first_log_sequence': 0,
    'startup_failure': None,
    'startup_log_failure': None,
    'identity': {'startup_report': {
        'platform': 'webgl',
        'capture_adapter': 'webgl-canvas-png',
        'build_fingerprint': job['profile']['build_fingerprint'],
        'source_fingerprint': job['profile']['source_fingerprint'],
        'unity_version': '6000.0.56f1',
        'diagnostics': True,
        'display': job['profile']['display'],
        'capabilities': job['profile']['capabilities'],
    }},
}
assert json.load(send('POST', 'jobs/' + job['job_id'] + '/started', started))['action'] == 'continue'
event = {
    'schema': 1,
    'job_id': job['job_id'],
    'player_session_id': session,
    'sequence': 0,
    'timestamp_unix_us': 1787953800000000,
    'source': 'ditto-player',
    'severity': 'information',
    'event_name': 'fixture.step',
    'message': 'responsive browser fixture',
    'fields': {},
    'exception': None,
    'stack_trace': None,
}
events = (json.dumps(event, separators=(',', ':')) + '\n').encode()
request = urllib.request.Request(
    base + '/jobs/' + job['job_id'] + '/logs/' + session + '?first_sequence=0',
    data=events,
    method='PUT',
    headers={
        'Content-Type': 'application/x-ndjson',
        'X-Ditto-SHA256': hashlib.sha256(events).hexdigest(),
    },
)
urllib.request.urlopen(request, timeout=1).close()
scenario = job['scenarios'][0]
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
    'last_log_sequence': 0,
    'execution_duration_ms': 1,
    'startup_duration_ms': 1,
    'boundary': {'status': 'passed', 'duration_ms': 1},
    'primary_error_ref': None,
}
assert json.load(send('POST', 'jobs/' + job['job_id'] + '/scenarios/' + scenario['id'] + '/complete', complete))['action'] == 'continue'
send('POST', 'jobs/' + job['job_id'] + '/complete', {
    'job_id': job['job_id'],
    'last_log_sequence': 0,
    'executed_scenario_ids': [scenario['id']],
    'unstarted_scenarios': [],
    'reason': 'completed',
    'execution_duration_ms': 1,
}).close()
print('responsive browser fixture')
"#;
