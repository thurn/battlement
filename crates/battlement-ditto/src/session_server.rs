//! Isolated loopback routes for one launched Ditto player.

use std::{
  io::Read,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

pub use crate::session_mutations::{PlayerSessionDurableState, PlayerSessionHandler};

use crate::{
  session_mutations::{ContinueSessionHandler, MutationError, MutationReply, MutationState},
  wire::{
    common::ErrorCode,
    job::Job,
    lifecycle::{
      HttpError, NextAction, PlayerInfrastructureFailure, ScenarioDecision, Started,
      StartupIdentity, StartupReport,
    },
  },
};

const MAXIMUM_JSON_BYTES: usize = 1024 * 1024;
const MAXIMUM_PNG_BYTES: usize = 64 * 1024 * 1024;

/// Host facts that a newly launched player must report exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerSessionRequirements {
  pub origin: Option<String>,
  pub capture_adapter: String,
  pub unity_version: String,
  pub diagnostics: bool,
  pub storage_directory: PathBuf,
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
  mutations: MutationState,
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

enum RequestBodyError {
  Media,
  Length,
  Oversize,
  Read,
}

impl PlayerSessionServer {
  /// Binds a loopback server and installs one immutable job.
  pub fn bind(job: Job, requirements: PlayerSessionRequirements) -> Result<Self> {
    Self::bind_with_handler(job, requirements, Arc::new(ContinueSessionHandler))
  }

  /// Binds a loopback server with host callbacks for committed results.
  pub fn bind_with_handler(
    job: Job,
    requirements: PlayerSessionRequirements,
    handler: Arc<dyn PlayerSessionHandler>,
  ) -> Result<Self> {
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
    let mutations = MutationState::new(
      job.clone(),
      player_session_id.clone(),
      requirements.storage_directory.clone(),
      handler,
    )?;
    let state = Arc::new(Mutex::new(State {
      job_bytes: serde_json::to_vec(&job)?,
      job,
      requirements,
      startup: None,
      started_at: None,
      expired: false,
      next_error: 1,
      mutations,
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

  /// Returns only bytes and completions already acknowledged by durable storage.
  pub fn durable_state(&self) -> PlayerSessionDurableState {
    self.state.lock().unwrap().mutations.durable_state()
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
    self.typed_error(status, ErrorCode::TransportRequestFailed, message, None)
  }

  fn mutation_reply(&mut self, result: Result<MutationReply, MutationError>) -> Reply {
    match result {
      Ok(reply) => {
        if reply.used_error_id {
          self.next_error += 1;
        }
        Reply {
          status: reply.status,
          body: reply.body,
        }
      }
      Err(error) => {
        if error.terminal {
          self.expired = true;
        }
        self.typed_error(
          error.status,
          error.code,
          &error.message,
          error.expected_sequence,
        )
      }
    }
  }

  fn typed_error(
    &mut self,
    status: u16,
    code: ErrorCode,
    message: &str,
    expected_sequence: Option<u64>,
  ) -> Reply {
    let error = HttpError {
      error_id: self.next_error_id(),
      code,
      message: message.to_owned(),
      expected_sequence,
      related_run_id: None,
    };
    self.next_error += 1;
    Reply::json(status, &error)
  }

  fn next_error_id(&self) -> String {
    format!("E{:04}", self.next_error)
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
  let request_url = request.url().to_owned();
  let Some(path) = request_url.strip_prefix(&prefix) else {
    return state.error(404, "unknown player route");
  };
  if !matches!(path.chars().next(), Some('/') | None) {
    return state.error(404, "unknown player route");
  }
  let (path, query) = path
    .split_once('?')
    .map_or((path, None), |(path, query)| (path, Some(query)));
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
  if path == started_path && query.is_none() {
    return started(request, state, player_session_id);
  }
  let job_path = format!("/jobs/{}", state.job.job_id);
  if let Some(session) = path.strip_prefix(&format!("{job_path}/logs/")) {
    return logs(request, state, session, query, player_session_id);
  }
  if let Some(artifact_id) = path.strip_prefix(&format!("{job_path}/artifacts/")) {
    return artifact(request, state, artifact_id, query);
  }
  if let Some(value) = path.strip_prefix(&format!("{job_path}/scenarios/"))
    && let Some(scenario_id) = value.strip_suffix("/complete")
  {
    return scenario(request, state, scenario_id, query);
  }
  if path == format!("{job_path}/complete") && query.is_none() {
    return terminal(request, state, false);
  }
  if path == format!("{job_path}/failed") && query.is_none() {
    return terminal(request, state, true);
  }
  state.error(404, "unknown player route")
}

fn started(request: &mut Request, state: &mut State, player_session_id: &str) -> Reply {
  if request.method() != &Method::Post {
    return state.error(405, "method is not allowed for the started route");
  }
  let body = match request_body(request, "application/json", MAXIMUM_JSON_BYTES) {
    Ok(body) => body,
    Err(error) => return request_body_error(state, error, "application/json"),
  };
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
  if let Err(error) =
    state
      .mutations
      .accept_startup(started.first_log_sequence, &body, &response_bytes)
  {
    state.expired = true;
    return state.typed_error(
      500,
      ErrorCode::DurabilityFailed,
      &format!("durable startup storage failed: {error}"),
      None,
    );
  }
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

fn logs(
  request: &mut Request,
  state: &mut State,
  session: &str,
  query: Option<&str>,
  player_session_id: &str,
) -> Reply {
  if request.method() != &Method::Put {
    return state.error(405, "method is not allowed for the log route");
  }
  if session != player_session_id {
    return state.error(409, "log request belongs to another player session");
  }
  let Some(first) = query
    .and_then(|value| value.strip_prefix("first_sequence="))
    .filter(|value| !value.contains('&'))
    .and_then(|value| value.parse::<u64>().ok())
  else {
    return state.error(400, "log route requires one first_sequence query");
  };
  let body = match request_body(request, "application/x-ndjson", MAXIMUM_JSON_BYTES) {
    Ok(body) => body,
    Err(error) => return request_body_error(state, error, "application/x-ndjson"),
  };
  if !valid_hash(request, &body) {
    return state.error(400, "log SHA-256 does not match its body");
  }
  let result = state.mutations.logs(first, &body);
  state.mutation_reply(result)
}

fn artifact(
  request: &mut Request,
  state: &mut State,
  artifact_id: &str,
  query: Option<&str>,
) -> Reply {
  if request.method() != &Method::Put {
    return state.error(405, "method is not allowed for the artifact route");
  }
  if query.is_some() || Uuid::parse_str(artifact_id).is_err() {
    return state.error(400, "artifact route is malformed");
  }
  let body = match request_body(request, "image/png", MAXIMUM_PNG_BYTES) {
    Ok(body) => body,
    Err(error) => return request_body_error(state, error, "image/png"),
  };
  let Some(hash) = header(request, "X-Ditto-SHA256") else {
    return state.error(400, "artifact SHA-256 header is required");
  };
  let Some(width) = integer_header(request, "X-Ditto-Width") else {
    return state.error(400, "artifact width header is invalid");
  };
  let Some(height) = integer_header(request, "X-Ditto-Height") else {
    return state.error(400, "artifact height header is invalid");
  };
  let result = state
    .mutations
    .artifact(artifact_id, &body, hash, width, height);
  state.mutation_reply(result)
}

fn scenario(
  request: &mut Request,
  state: &mut State,
  scenario_id: &str,
  query: Option<&str>,
) -> Reply {
  if request.method() != &Method::Post {
    return state.error(405, "method is not allowed for scenario completion");
  }
  if query.is_some() || Uuid::parse_str(scenario_id).is_err() {
    return state.error(400, "scenario completion route is malformed");
  }
  let body = match request_body(request, "application/json", MAXIMUM_JSON_BYTES) {
    Ok(body) => body,
    Err(error) => return request_body_error(state, error, "application/json"),
  };
  let result = state.mutations.scenario(scenario_id, &body);
  state.mutation_reply(result)
}

fn terminal(request: &mut Request, state: &mut State, failed: bool) -> Reply {
  if request.method() != &Method::Post {
    return state.error(405, "method is not allowed for terminal routes");
  }
  let body = match request_body(request, "application/json", MAXIMUM_JSON_BYTES) {
    Ok(body) => body,
    Err(error) => return request_body_error(state, error, "application/json"),
  };
  let result = if failed {
    let existing = state
      .startup
      .as_ref()
      .and_then(|startup| startup.fact.decision.error_id.clone());
    let error_id = existing.clone().unwrap_or_else(|| state.next_error_id());
    state.mutations.failed(&body, &error_id, existing.is_none())
  } else {
    state.mutations.complete(&body)
  };
  state.mutation_reply(result)
}

fn request_body(
  request: &mut Request,
  content_type: &str,
  maximum: usize,
) -> Result<Vec<u8>, RequestBodyError> {
  if header(request, "Content-Type") != Some(content_type) {
    return Err(RequestBodyError::Media);
  }
  if request.body_length().is_none() {
    return Err(RequestBodyError::Length);
  }
  if request.body_length().is_some_and(|length| length > maximum) {
    return Err(RequestBodyError::Oversize);
  }
  let mut body = Vec::new();
  request
    .as_reader()
    .take((maximum + 1) as u64)
    .read_to_end(&mut body)
    .map_err(|_| RequestBodyError::Read)?;
  if body.len() > maximum {
    return Err(RequestBodyError::Oversize);
  }
  Ok(body)
}

fn request_body_error(state: &mut State, error: RequestBodyError, content_type: &str) -> Reply {
  match error {
    RequestBodyError::Media => state.error(415, &format!("route requires {content_type}")),
    RequestBodyError::Length => state.error(411, "request requires Content-Length"),
    RequestBodyError::Oversize => state.error(413, "request body exceeds its size limit"),
    RequestBodyError::Read => state.error(400, "request body could not be read"),
  }
}

fn valid_hash(request: &Request, body: &[u8]) -> bool {
  header(request, "X-Ditto-SHA256").is_some_and(|declared| {
    let actual: String = Sha256::digest(body)
      .iter()
      .map(|byte| format!("{byte:02x}"))
      .collect();
    declared == actual
  })
}

fn integer_header(request: &Request, name: &str) -> Option<u32> {
  header(request, name)?
    .parse()
    .ok()
    .filter(|value| *value > 0)
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

fn startup_mismatch<'a>(state: &'a State, report: &'a StartupReport) -> Option<&'a str> {
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
