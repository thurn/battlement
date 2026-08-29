//! Isolated loopback routes for one launched Ditto player.

use std::{
  io::Read,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

use crate::wire::{
  common::ErrorCode,
  job::Job,
  lifecycle::{
    NextAction, PlayerInfrastructureFailure, ScenarioDecision, Started, StartupIdentity,
  },
};

const MAXIMUM_JSON_BYTES: usize = 1024 * 1024;

/// Host facts that a newly launched player must report exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerSessionRequirements {
  pub origin: Option<String>,
  pub capture_adapter: String,
  pub unity_version: String,
  pub diagnostics: bool,
}

/// One accepted startup payload and the durable decision returned for it.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupFact {
  pub started: Started,
  pub decision: ScenarioDecision,
}

/// Read-only state retained for orchestration and result construction.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSessionSnapshot {
  pub startup: Option<StartupFact>,
  pub started_at: Option<Instant>,
  pub expired: bool,
}

/// A single-player HTTP/1.1 server bound to explicit IPv4 loopback.
pub struct PlayerSessionServer {
  server: Arc<Server>,
  state: Arc<Mutex<State>>,
  shutdown: Arc<AtomicBool>,
  worker: Option<JoinHandle<()>>,
  address: SocketAddrV4,
  route_token: String,
  player_session_id: String,
}

struct State {
  job: Job,
  job_bytes: Vec<u8>,
  requirements: PlayerSessionRequirements,
  startup: Option<StoredStartup>,
  started_at: Option<Instant>,
  expired: bool,
  next_error: u32,
}

struct StoredStartup {
  request_bytes: Vec<u8>,
  fact: StartupFact,
  response_bytes: Vec<u8>,
}

struct Reply {
  status: u16,
  body: Vec<u8>,
}

impl PlayerSessionServer {
  /// Binds a loopback server and installs one immutable job.
  pub fn bind(job: Job, requirements: PlayerSessionRequirements) -> Result<Self> {
    job.validate()?;
    ensure!(
      !requirements.capture_adapter.is_empty(),
      "capture adapter is required"
    );
    ensure!(
      !requirements.unity_version.is_empty(),
      "Unity version is required"
    );
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
    let address = match listener.local_addr()? {
      SocketAddr::V4(address) => address,
      SocketAddr::V6(_) => unreachable!("an IPv4 listener returned an IPv6 address"),
    };
    let server = Arc::new(
      Server::from_listener(listener, None)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("start player session server")?,
    );
    let route_token = Uuid::new_v4().simple().to_string();
    let player_session_id = Uuid::new_v4().to_string();
    let state = Arc::new(Mutex::new(State {
      job_bytes: serde_json::to_vec(&job)?,
      job,
      requirements,
      startup: None,
      started_at: None,
      expired: false,
      next_error: 1,
    }));
    let worker_server = Arc::clone(&server);
    let worker_state = Arc::clone(&state);
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_shutdown = Arc::clone(&shutdown);
    let worker_token = route_token.clone();
    let worker_session = player_session_id.clone();
    let worker = thread::spawn(move || {
      loop {
        if worker_shutdown.load(Ordering::Acquire) {
          break;
        }
        match worker_server.recv_timeout(Duration::from_millis(50)) {
          Ok(Some(request)) => serve(request, &worker_state, &worker_token, &worker_session),
          Ok(None) => {}
          Err(_) => break,
        }
      }
    });
    Ok(Self {
      server,
      state,
      shutdown,
      worker: Some(worker),
      address,
      route_token,
      player_session_id,
    })
  }

  /// Returns the unguessable base URL assigned to this launch.
  pub fn base_url(&self) -> String {
    format!("http://{}/ditto/{}", self.address, self.route_token)
  }

  /// Returns the pending player-session identity assigned to this route.
  pub fn player_session_id(&self) -> &str {
    &self.player_session_id
  }

  /// Invalidates the route without stopping the listener.
  pub fn expire(&self) {
    self.state.lock().unwrap().expired = true;
  }

  /// Returns startup facts and route lifetime state.
  pub fn snapshot(&self) -> PlayerSessionSnapshot {
    let state = self.state.lock().unwrap();
    PlayerSessionSnapshot {
      startup: state.startup.as_ref().map(|value| value.fact.clone()),
      started_at: state.started_at,
      expired: state.route_expired(),
    }
  }
}

impl Drop for PlayerSessionServer {
  fn drop(&mut self) {
    self.shutdown.store(true, Ordering::Release);
    self.server.unblock();
    if let Some(worker) = self.worker.take() {
      worker.join().unwrap();
    }
  }
}

impl State {
  fn route_expired(&self) -> bool {
    self.expired
      || self.started_at.is_some_and(|start| {
        start.elapsed().as_millis() >= self.job.remaining_run_timeout_ms.into()
      })
  }

  fn error(&mut self, status: u16, message: &str) -> Reply {
    let error = crate::wire::lifecycle::HttpError {
      error_id: format!("E{:04}", self.next_error),
      code: ErrorCode::TransportRequestFailed,
      message: message.to_owned(),
      expected_sequence: None,
      related_run_id: None,
    };
    self.next_error += 1;
    Reply::json(status, &error)
  }
}

impl Reply {
  fn json<T: serde::Serialize>(status: u16, value: &T) -> Self {
    Self {
      status,
      body: serde_json::to_vec(value).expect("validated wire value must serialize"),
    }
  }
}

fn serve(mut request: Request, shared: &Mutex<State>, route_token: &str, player_session_id: &str) {
  let reply = dispatch(
    &mut request,
    &mut shared.lock().unwrap(),
    route_token,
    player_session_id,
  );
  let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();
  let _ = request.respond(
    Response::from_data(reply.body)
      .with_status_code(StatusCode(reply.status))
      .with_header(content_type),
  );
}

fn dispatch(
  request: &mut Request,
  state: &mut State,
  route_token: &str,
  player_session_id: &str,
) -> Reply {
  let prefix = format!("/ditto/{route_token}");
  let Some(path) = request.url().strip_prefix(&prefix) else {
    return state.error(404, "unknown player route");
  };
  if !matches!(path.chars().next(), Some('/') | None) {
    return state.error(404, "unknown player route");
  }
  if state.route_expired() {
    return state.error(404, "expired player route");
  }
  if !origin_allowed(request, state.requirements.origin.as_deref()) {
    return state.error(403, "request origin does not own this player route");
  }
  if path == "/job" {
    return if request.method() == &Method::Get {
      Reply {
        status: 200,
        body: state.job_bytes.clone(),
      }
    } else {
      state.error(405, "method is not allowed for the job route")
    };
  }
  let started_path = format!("/jobs/{}/started", state.job.job_id);
  if path != started_path {
    return state.error(404, "unknown player route");
  }
  if request.method() != &Method::Post {
    return state.error(405, "method is not allowed for the started route");
  }
  if header(request, "Content-Type") != Some("application/json") {
    return state.error(415, "started requires application/json");
  }
  if request
    .body_length()
    .is_some_and(|length| length > MAXIMUM_JSON_BYTES)
  {
    return state.error(413, "JSON body exceeds 1 MiB");
  }
  let mut body = Vec::new();
  if request
    .as_reader()
    .take((MAXIMUM_JSON_BYTES + 1) as u64)
    .read_to_end(&mut body)
    .is_err()
  {
    return state.error(400, "request body could not be read");
  }
  if body.len() > MAXIMUM_JSON_BYTES {
    return state.error(413, "JSON body exceeds 1 MiB");
  }
  if let Some(stored) = &state.startup {
    return if stored.request_bytes == body {
      Reply {
        status: 200,
        body: stored.response_bytes.clone(),
      }
    } else {
      state.error(409, "conflicting startup request")
    };
  }
  let Ok(started) = serde_json::from_slice::<Started>(&body) else {
    return state.error(400, "started body is malformed");
  };
  if started.job_id != state.job.job_id
    || started.run_id != state.job.run_id
    || started.player_session_id != player_session_id
  {
    return state.error(409, "started body belongs to another session");
  }
  if started
    .validate(&state.job, player_session_id, None)
    .is_err()
  {
    return state.error(400, "started body violates the wire contract");
  }
  let decision = startup_decision(state, &started);
  let response_bytes = serde_json::to_vec(&decision).expect("startup decision must serialize");
  state.started_at = Some(Instant::now());
  state.startup = Some(StoredStartup {
    request_bytes: body,
    fact: StartupFact { started, decision },
    response_bytes: response_bytes.clone(),
  });
  Reply {
    status: 200,
    body: response_bytes,
  }
}

fn startup_decision(state: &mut State, started: &Started) -> ScenarioDecision {
  let failure = started
    .startup_log_failure
    .as_ref()
    .or(started.startup_failure.as_ref());
  let mismatch = match &started.identity {
    StartupIdentity::Report(identity) => startup_mismatch(state, &identity.startup_report),
    StartupIdentity::Accepted(_) => Some("new player did not report startup facts"),
  };
  let (code, message) = match failure {
    Some(PlayerInfrastructureFailure { code, message }) => (*code, message.clone()),
    None if mismatch.is_some() => (ErrorCode::StartupMismatch, mismatch.unwrap().to_owned()),
    None => {
      return ScenarioDecision {
        action: NextAction::Continue,
        completed_failures: 0,
        error_id: None,
        error_code: None,
        message: None,
      };
    }
  };
  let error_id = format!("E{:04}", state.next_error);
  state.next_error += 1;
  ScenarioDecision {
    action: NextAction::Stop,
    completed_failures: 0,
    error_id: Some(error_id),
    error_code: Some(code),
    message: Some(message),
  }
}

fn startup_mismatch<'a>(
  state: &'a State,
  report: &'a crate::wire::lifecycle::StartupReport,
) -> Option<&'a str> {
  let profile = &state.job.profile;
  if report.platform != profile.platform {
    return Some("wrong platform");
  }
  if report.capture_adapter != state.requirements.capture_adapter {
    return Some("wrong capture adapter");
  }
  if report.build_fingerprint != profile.build_fingerprint {
    return Some("wrong build fingerprint");
  }
  if report.source_fingerprint != profile.source_fingerprint {
    return Some("wrong source fingerprint");
  }
  if report.unity_version != state.requirements.unity_version {
    return Some("wrong Unity version");
  }
  if report.diagnostics != state.requirements.diagnostics {
    return Some("wrong diagnostics setting");
  }
  if report.display != profile.display {
    return Some("wrong display");
  }
  if report.capabilities != profile.capabilities {
    return Some("wrong capabilities");
  }
  None
}

fn origin_allowed(request: &Request, expected: Option<&str>) -> bool {
  match header(request, "Origin") {
    None => true,
    Some(origin) => expected == Some(origin),
  }
}

fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
  request
    .headers()
    .iter()
    .find(|header| header.field.to_string().eq_ignore_ascii_case(name))
    .map(|header| header.value.as_str())
}
