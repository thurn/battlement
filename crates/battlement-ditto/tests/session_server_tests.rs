use std::{
  io::{Read, Write},
  net::{Shutdown, TcpStream},
  time::Duration,
};

use battlement_ditto::{
  session_server::{PlayerSessionRequirements, PlayerSessionServer},
  wire::{
    common::ErrorCode,
    job::Job,
    lifecycle::{HttpError, NextAction, ScenarioDecision},
  },
};
use serde_json::Value;

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

fn server() -> PlayerSessionServer {
  PlayerSessionServer::bind(
    job(),
    PlayerSessionRequirements {
      origin: Some("http://launcher.test".to_owned()),
      capture_adapter: "unity-async-readback-png".to_owned(),
      unity_version: "6000.0.56f1".to_owned(),
      diagnostics: true,
    },
  )
  .unwrap()
}

fn job() -> Job {
  serde_json::from_value::<Job>(serde_json::from_str::<Value>(FIXTURE).unwrap()["job"].clone())
    .unwrap()
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
  if method == "POST" {
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
    let length =
      usize::from_str_radix(std::str::from_utf8(&bytes[..line_end]).unwrap(), 16).unwrap();
    if length == 0 {
      return decoded;
    }
    bytes = &bytes[line_end + 2..];
    decoded.extend_from_slice(&bytes[..length]);
    bytes = &bytes[length + 2..];
  }
}
