use std::{
  fs::{self, File},
  io::{Read, Write},
  net::{Shutdown, TcpStream},
  ops::Deref,
  path::PathBuf,
  str,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  thread,
  time::Duration,
};

use battlement_ditto::{
  session_server::{PlayerSessionHandler, PlayerSessionRequirements, PlayerSessionServer},
  wire::{
    common::ErrorCode,
    job::Job,
    lifecycle::{
      HttpError, JobComplete, JobCompleteAck, JobFailed, JobFailedAck, NextAction,
      ScenarioComplete, ScenarioDecision,
    },
  },
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const FIXTURE: &str = include_str!(
  "../../../Packages/com.battlement.client/Tests/Fixtures/Ditto/lifecycle-contract.json"
);

#[test]
fn accepted_native_startup_is_replayable_and_retained() {
  let server = server();
  let job_response = exchange("GET", &format!("{}/job", server.base_url()), &[], &[], None);
  assert_eq!(job_response.status, 200);
  assert_eq!(
    serde_json::from_slice::<Job>(&job_response.body).unwrap(),
    job()
  );

  let body = started_body(&server);
  let url = format!("{}/jobs/{}/started", server.base_url(), job().job_id);
  let first = exchange(
    "POST",
    &url,
    &[("Content-Type", "application/json")],
    &body,
    None,
  );
  let replay = exchange(
    "POST",
    &url,
    &[("Content-Type", "application/json")],
    &body,
    None,
  );

  assert_eq!(first.status, 200);
  assert_eq!(first.body, replay.body);
  let decision: ScenarioDecision = serde_json::from_slice(&first.body).unwrap();
  assert_eq!(decision.action, NextAction::Continue);
  let snapshot = server.snapshot();
  assert_eq!(
    snapshot.startup.unwrap().started,
    serde_json::from_slice(&body).unwrap()
  );
  assert!(snapshot.started_at.is_some());

  let mut conflict: Value = serde_json::from_slice(&body).unwrap();
  conflict["first_log_sequence"] = Value::from(82);
  assert_http_error(
    exchange(
      "POST",
      &url,
      &[("Content-Type", "application/json")],
      &serde_json::to_vec(&conflict).unwrap(),
      None,
    ),
    409,
  );
}

#[test]
fn routes_reject_unknown_expired_cross_session_origin_media_and_size() {
  let server = server();
  let base = server.base_url();
  let address = base.split("/ditto/").next().unwrap();
  assert_http_error(
    exchange(
      "GET",
      &format!("{address}/ditto/unknown/job"),
      &[],
      &[],
      None,
    ),
    404,
  );
  assert_http_error(
    exchange(
      "GET",
      &format!("{base}/job"),
      &[("Origin", "http://unrelated.test")],
      &[],
      None,
    ),
    403,
  );
  assert_http_error(
    exchange("POST", &format!("{base}/job"), &[], &[], None),
    405,
  );

  let url = format!("{base}/jobs/{}/started", job().job_id);
  assert_http_error(exchange("POST", &url, &[], b"{}", None), 415);
  assert_http_error(
    exchange(
      "POST",
      &url,
      &[("Content-Type", "application/json")],
      &[],
      Some(1024 * 1024 + 1),
    ),
    413,
  );
  let mut cross_session: Value = serde_json::from_slice(&started_body(&server)).unwrap();
  cross_session["player_session_id"] = Value::String(uuid::Uuid::new_v4().to_string());
  assert_http_error(
    exchange(
      "POST",
      &url,
      &[("Content-Type", "application/json")],
      &serde_json::to_vec(&cross_session).unwrap(),
      None,
    ),
    409,
  );

  server.expire();
  assert_http_error(exchange("GET", &format!("{base}/job"), &[], &[], None), 404);
}

#[test]
fn rejected_web_startup_returns_stop_and_retains_reported_facts() {
  let server = server();
  let mut started: Value = serde_json::from_slice(&started_body(&server)).unwrap();
  started["identity"]["startup_report"]["display"]["width"] = Value::from(640);
  started["identity"]["startup_report"]["display"]["safe_area"][2] = Value::from(640);
  let body = serde_json::to_vec(&started).unwrap();
  let response = exchange(
    "POST",
    &format!("{}/jobs/{}/started", server.base_url(), job().job_id),
    &[
      ("Content-Type", "application/json"),
      ("Origin", "http://launcher.test"),
    ],
    &body,
    None,
  );

  assert_eq!(
    response.status,
    200,
    "{}",
    String::from_utf8_lossy(&response.body)
  );
  let decision: ScenarioDecision = serde_json::from_slice(&response.body).unwrap();
  assert_eq!(decision.action, NextAction::Stop);
  assert_eq!(decision.error_code, Some(ErrorCode::StartupMismatch));
  assert_eq!(decision.message.as_deref(), Some("wrong display"));
  assert_eq!(server.snapshot().startup.unwrap().decision, decision);
}

#[test]
fn mutating_routes_replay_exact_requests_without_repeating_host_work() {
  let storage = tempfile::tempdir().unwrap();
  let handler = Arc::new(CountingHandler::default());
  let server = PlayerSessionServer::bind_with_handler(
    job(),
    requirements(storage.path().to_owned()),
    handler.clone(),
  )
  .unwrap();
  let started = started_body(&server);
  let started_url = format!("{}/jobs/{}/started", server.base_url(), job().job_id);
  assert_eq!(json_request("POST", &started_url, &started).status, 200);

  let events = event_bytes(&server);
  let log_url = format!(
    "{}/jobs/{}/logs/{}?first_sequence=81",
    server.base_url(),
    job().job_id,
    server.player_session_id()
  );
  let log_hash = hash(&events);
  let log_headers = [
    ("Content-Type", "application/x-ndjson"),
    ("X-Ditto-SHA256", log_hash.as_str()),
  ];
  let first_log = exchange("PUT", &log_url, &log_headers, &events, None);
  let replay_log = exchange("PUT", &log_url, &log_headers, &events, None);
  assert_eq!((first_log.status, first_log.body), (200, replay_log.body));
  assert_eq!(
    fs::read(storage.path().join("logs/events.jsonl")).unwrap(),
    events
  );

  let gap_url = log_url.replace("first_sequence=81", "first_sequence=84");
  let gap = exchange("PUT", &gap_url, &log_headers, &events, None);
  assert_eq!(gap.status, 409);
  let gap_error: HttpError = serde_json::from_slice(&gap.body).unwrap();
  assert_eq!(gap_error.code, ErrorCode::TransportLogGap);
  assert_eq!(gap_error.expected_sequence, Some(83));
  let mut changed_events: Value =
    serde_json::from_slice(events.split(|byte| *byte == b'\n').next().unwrap()).unwrap();
  changed_events["message"] = Value::String("changed".to_owned());
  let changed = format!("{}\n", serde_json::to_string(&changed_events).unwrap()).into_bytes();
  let changed_hash = hash(&changed);
  let conflict = exchange(
    "PUT",
    &log_url,
    &[
      ("Content-Type", "application/x-ndjson"),
      ("X-Ditto-SHA256", &changed_hash),
    ],
    &changed,
    None,
  );
  assert_eq!(
    serde_json::from_slice::<HttpError>(&conflict.body)
      .unwrap()
      .code,
    ErrorCode::TransportLogConflict
  );

  let values = fixture();
  let screenshot_id = values["scenario_complete"]["artifacts"][0]["artifact_id"]
    .as_str()
    .unwrap();
  let artifact_url = format!(
    "{}/jobs/{}/artifacts/{screenshot_id}",
    server.base_url(),
    job().job_id
  );
  let png = include_bytes!("../../../samples/ui/Assets/Original/Signal Sprite.png");
  let png_hash = hash(png);
  let artifact_headers = [
    ("Content-Type", "image/png"),
    ("X-Ditto-SHA256", png_hash.as_str()),
    ("X-Ditto-Width", "128"),
    ("X-Ditto-Height", "96"),
  ];
  assert_eq!(
    exchange("PUT", &artifact_url, &artifact_headers, png, None).status,
    201
  );
  assert_eq!(
    exchange("PUT", &artifact_url, &artifact_headers, png, None).status,
    200
  );
  let other = include_bytes!("../../../samples/ui/Assets/Original/Signal Cursor.png");
  let other_hash = hash(other);
  let artifact_conflict = exchange(
    "PUT",
    &artifact_url,
    &[
      ("Content-Type", "image/png"),
      ("X-Ditto-SHA256", &other_hash),
      ("X-Ditto-Width", "128"),
      ("X-Ditto-Height", "96"),
    ],
    other,
    None,
  );
  assert_eq!(artifact_conflict.status, 409);

  let scenario_body = passed_scenario();
  let scenario_id = values["scenario_complete"]["scenario_id"]
    .as_str()
    .unwrap()
    .to_owned();
  let scenario_url = format!(
    "{}/jobs/{}/scenarios/{scenario_id}/complete",
    server.base_url(),
    job().job_id
  );
  let first_scenario = json_request("POST", &scenario_url, &scenario_body);
  let replay_scenario = json_request("POST", &scenario_url, &scenario_body);
  assert_eq!(
    (first_scenario.status, first_scenario.body),
    (200, replay_scenario.body)
  );
  assert_eq!(handler.scenarios.load(Ordering::SeqCst), 1);
  let mut changed_scenario: Value = serde_json::from_slice(&scenario_body).unwrap();
  changed_scenario["execution_duration_ms"] = Value::from(6);
  assert_eq!(
    json_request(
      "POST",
      &scenario_url,
      &serde_json::to_vec(&changed_scenario).unwrap(),
    )
    .status,
    409
  );
  assert_eq!(handler.scenarios.load(Ordering::SeqCst), 1);

  let complete = serde_json::json!({
    "job_id": job().job_id,
    "last_log_sequence": 82,
    "executed_scenario_ids": [scenario_id],
    "unstarted_scenarios": [],
    "reason": "completed",
    "execution_duration_ms": 12
  });
  let complete_body = serde_json::to_vec(&complete).unwrap();
  let complete_url = format!("{}/jobs/{}/complete", server.base_url(), job().job_id);
  let first_complete = json_request("POST", &complete_url, &complete_body);
  let replay_complete = json_request("POST", &complete_url, &complete_body);
  assert_eq!(
    (first_complete.status, first_complete.body),
    (200, replay_complete.body)
  );
  assert_eq!(handler.completions.load(Ordering::SeqCst), 1);
  let mut changed_complete = complete;
  changed_complete["execution_duration_ms"] = Value::from(13);
  assert_eq!(
    json_request(
      "POST",
      &complete_url,
      &serde_json::to_vec(&changed_complete).unwrap(),
    )
    .status,
    409
  );
  assert_eq!(handler.completions.load(Ordering::SeqCst), 1);
  let failed_url = format!("{}/jobs/{}/failed", server.base_url(), job().job_id);
  assert_eq!(json_request("POST", &failed_url, b"{}").status, 409);
}

#[test]
fn failed_terminal_replays_once_and_durable_storage_failure_returns_500() {
  let storage = tempfile::tempdir().unwrap();
  let handler = Arc::new(CountingHandler::default());
  let server = PlayerSessionServer::bind_with_handler(
    job(),
    requirements(storage.path().to_owned()),
    handler.clone(),
  )
  .unwrap();
  assert_eq!(
    json_request(
      "POST",
      &format!("{}/jobs/{}/started", server.base_url(), job().job_id),
      &started_body(&server),
    )
    .status,
    200
  );
  let failed = serde_json::to_vec(&serde_json::json!({
    "job_id": job().job_id,
    "failure": {"code": "runtime.process-exit", "message": "fixture exit"},
    "last_log_sequence": null,
    "executed_scenario_ids": [],
    "unstarted_scenarios": [{
      "scenario_id": job().scenarios[0].id,
      "reason": "run-infrastructure-error"
    }]
  }))
  .unwrap();
  let failed_url = format!("{}/jobs/{}/failed", server.base_url(), job().job_id);
  let first = json_request("POST", &failed_url, &failed);
  let replay = json_request("POST", &failed_url, &failed);
  assert_eq!((first.status, first.body), (200, replay.body));
  assert_eq!(handler.failures.load(Ordering::SeqCst), 1);

  let broken_storage = tempfile::tempdir().unwrap();
  let broken =
    PlayerSessionServer::bind(job(), requirements(broken_storage.path().to_owned())).unwrap();
  assert_eq!(
    json_request(
      "POST",
      &format!("{}/jobs/{}/started", broken.base_url(), job().job_id),
      &started_body(&broken),
    )
    .status,
    200
  );
  fs::remove_dir_all(broken_storage.path().join(".requests")).unwrap();
  File::create(broken_storage.path().join(".requests")).unwrap();
  let events = event_bytes(&broken);
  let events_hash = hash(&events);
  let response = exchange(
    "PUT",
    &format!(
      "{}/jobs/{}/logs/{}?first_sequence=81",
      broken.base_url(),
      job().job_id,
      broken.player_session_id()
    ),
    &[
      ("Content-Type", "application/x-ndjson"),
      ("X-Ditto-SHA256", &events_hash),
    ],
    &events,
    None,
  );
  assert_eq!(response.status, 500);
  assert_eq!(
    serde_json::from_slice::<HttpError>(&response.body)
      .unwrap()
      .code,
    ErrorCode::DurabilityFailed
  );
}

#[test]
fn terminal_player_long_poll_receives_one_warm_job() {
  let first_storage = tempfile::tempdir().unwrap();
  let server = Arc::new(
    PlayerSessionServer::bind(job(), requirements(first_storage.path().to_owned())).unwrap(),
  );
  assert_eq!(
    json_request(
      "POST",
      &format!("{}/jobs/{}/started", server.base_url(), job().job_id),
      &started_body(&server),
    )
    .status,
    200
  );
  let failed = serde_json::to_vec(&serde_json::json!({
    "job_id": job().job_id,
    "failure": {"code": "runtime.process-exit", "message": "fixture exit"},
    "last_log_sequence": null,
    "executed_scenario_ids": [],
    "unstarted_scenarios": [{
      "scenario_id": job().scenarios[0].id,
      "reason": "run-infrastructure-error"
    }]
  }))
  .unwrap();
  assert_eq!(
    json_request(
      "POST",
      &format!("{}/jobs/{}/failed", server.base_url(), job().job_id),
      &failed,
    )
    .status,
    200
  );

  let base = server.base_url();
  let first_job_id = job().job_id;
  let waiter = thread::spawn(move || {
    exchange(
      "GET",
      &format!("{base}/next-job?after={first_job_id}"),
      &[("Origin", "http://launcher.test")],
      &[],
      None,
    )
  });
  thread::sleep(Duration::from_millis(30));
  let second_storage = tempfile::tempdir().unwrap();
  let mut next = job();
  next.job_id = uuid::Uuid::new_v4().to_string();
  next.run_id = uuid::Uuid::new_v4().to_string();
  let mut stale = next.clone();
  stale.profile.source_fingerprint = "a".repeat(64);
  assert!(
    server
      .install_job(
        stale,
        second_storage.path().to_owned(),
        Arc::new(CountingHandler::default()),
      )
      .unwrap_err()
      .to_string()
      .contains("replacement source")
  );
  server
    .install_job(
      next.clone(),
      second_storage.path().to_owned(),
      Arc::new(CountingHandler::default()),
    )
    .unwrap();
  let response = waiter.join().unwrap();
  assert_eq!(response.status, 200);
  assert_eq!(serde_json::from_slice::<Job>(&response.body).unwrap(), next);

  let warm_started = serde_json::to_vec(&serde_json::json!({
    "job_id": next.job_id,
    "run_id": next.run_id,
    "player_session_id": server.player_session_id(),
    "first_log_sequence": 83,
    "startup_failure": null,
    "startup_log_failure": null,
    "identity": {"accepted_player_session_id": server.player_session_id()}
  }))
  .unwrap();
  assert_eq!(
    json_request(
      "POST",
      &format!("{}/jobs/{}/started", server.base_url(), next.job_id),
      &warm_started,
    )
    .status,
    200
  );
}

fn server() -> TestServer {
  let storage = tempfile::tempdir().unwrap();
  let server = PlayerSessionServer::bind(job(), requirements(storage.path().to_owned())).unwrap();
  TestServer {
    server,
    _storage: storage,
  }
}

fn requirements(storage_directory: PathBuf) -> PlayerSessionRequirements {
  PlayerSessionRequirements {
    origin: Some("http://launcher.test".to_owned()),
    capture_adapter: "unity-async-readback-png".to_owned(),
    unity_version: "6000.0.56f1".to_owned(),
    diagnostics: true,
    storage_directory,
  }
}

fn job() -> Job {
  serde_json::from_value::<Job>(fixture()["job"].clone()).unwrap()
}

fn fixture() -> Value {
  serde_json::from_str(FIXTURE).unwrap()
}

fn started_body(server: &PlayerSessionServer) -> Vec<u8> {
  let mut started = serde_json::from_str::<Value>(FIXTURE).unwrap()["started"].clone();
  started["player_session_id"] = Value::String(server.player_session_id().to_owned());
  serde_json::to_vec(&started).unwrap()
}

fn assert_http_error(response: HttpResponse, expected_status: u16) {
  assert_eq!(response.status, expected_status);
  serde_json::from_slice::<HttpError>(&response.body)
    .unwrap()
    .validate()
    .unwrap();
}

fn exchange(
  method: &str,
  url: &str,
  headers: &[(&str, &str)],
  body: &[u8],
  declared_length: Option<usize>,
) -> HttpResponse {
  let endpoint = url.strip_prefix("http://").unwrap();
  let (authority, path) = endpoint.split_once('/').unwrap();
  let mut stream = TcpStream::connect(authority).unwrap();
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .unwrap();
  write!(
    stream,
    "{method} /{path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n"
  )
  .unwrap();
  for (name, value) in headers {
    write!(stream, "{name}: {value}\r\n").unwrap();
  }
  if matches!(method, "POST" | "PUT") {
    write!(
      stream,
      "Content-Length: {}\r\n",
      declared_length.unwrap_or(body.len())
    )
    .unwrap();
  }
  stream.write_all(b"\r\n").unwrap();
  stream.write_all(body).unwrap();
  stream.shutdown(Shutdown::Write).unwrap();
  let mut bytes = Vec::new();
  stream.read_to_end(&mut bytes).unwrap();
  HttpResponse::parse(bytes)
}

fn json_request(method: &str, url: &str, body: &[u8]) -> HttpResponse {
  exchange(
    method,
    url,
    &[("Content-Type", "application/json")],
    body,
    None,
  )
}

fn event_bytes(server: &PlayerSessionServer) -> Vec<u8> {
  let mut bytes = Vec::new();
  for mut event in fixture()["events"].as_array().unwrap().iter().cloned() {
    event["player_session_id"] = Value::String(server.player_session_id().to_owned());
    bytes.extend(serde_json::to_vec(&event).unwrap());
    bytes.push(b'\n');
  }
  bytes
}

fn passed_scenario() -> Vec<u8> {
  let mut complete = fixture()["scenario_complete"].clone();
  complete["execution_status"] = Value::String("passed".to_owned());
  complete["last_log_sequence"] = Value::from(82);
  complete["primary_error_ref"] = Value::Null;
  complete["failure_frame"] = Value::Null;
  complete["artifacts"].as_array_mut().unwrap().truncate(1);
  let last = complete["steps"]
    .as_array_mut()
    .unwrap()
    .last_mut()
    .unwrap();
  last["status"] = Value::String("passed".to_owned());
  last["error_refs"] = Value::Array(Vec::new());
  last["assertion"]["observed"] = Value::Bool(true);
  last["assertion"]["passed"] = Value::Bool(true);
  serde_json::to_vec(&complete).unwrap()
}

fn hash(bytes: &[u8]) -> String {
  Sha256::digest(bytes)
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect()
}

struct HttpResponse {
  status: u16,
  body: Vec<u8>,
}

impl HttpResponse {
  fn parse(bytes: Vec<u8>) -> Self {
    let boundary = bytes
      .windows(4)
      .position(|value| value == b"\r\n\r\n")
      .unwrap();
    let headers = String::from_utf8(bytes[..boundary].to_vec()).unwrap();
    let status = headers
      .lines()
      .next()
      .unwrap()
      .split_whitespace()
      .nth(1)
      .unwrap()
      .parse()
      .unwrap();
    let raw_body = &bytes[boundary + 4..];
    let body = if headers
      .to_ascii_lowercase()
      .contains("transfer-encoding: chunked")
    {
      decode_chunks(raw_body)
    } else {
      raw_body.to_vec()
    };
    Self { status, body }
  }
}

fn decode_chunks(mut bytes: &[u8]) -> Vec<u8> {
  let mut decoded = Vec::new();
  loop {
    let line_end = bytes.windows(2).position(|value| value == b"\r\n").unwrap();
    let length = usize::from_str_radix(str::from_utf8(&bytes[..line_end]).unwrap(), 16).unwrap();
    if length == 0 {
      return decoded;
    }
    bytes = &bytes[line_end + 2..];
    decoded.extend_from_slice(&bytes[..length]);
    bytes = &bytes[length + 2..];
  }
}

struct TestServer {
  server: PlayerSessionServer,
  _storage: TempDir,
}

impl Deref for TestServer {
  type Target = PlayerSessionServer;

  fn deref(&self) -> &Self::Target {
    &self.server
  }
}

#[derive(Default)]
struct CountingHandler {
  scenarios: AtomicUsize,
  completions: AtomicUsize,
  failures: AtomicUsize,
}

impl PlayerSessionHandler for CountingHandler {
  fn scenario_complete(&self, _: &ScenarioComplete) -> anyhow::Result<ScenarioDecision> {
    self.scenarios.fetch_add(1, Ordering::SeqCst);
    Ok(ScenarioDecision {
      action: NextAction::Continue,
      completed_failures: 0,
      error_id: None,
      error_code: None,
      message: None,
    })
  }

  fn job_complete(&self, complete: &JobComplete) -> anyhow::Result<JobCompleteAck> {
    self.completions.fetch_add(1, Ordering::SeqCst);
    Ok(JobCompleteAck {
      job_id: complete.job_id.clone(),
    })
  }

  fn job_failed(&self, failed: &JobFailed, error_id: &str) -> anyhow::Result<JobFailedAck> {
    self.failures.fetch_add(1, Ordering::SeqCst);
    Ok(JobFailedAck {
      job_id: failed.job_id.clone(),
      error_id: error_id.to_owned(),
    })
  }
}
