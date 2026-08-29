//! Isolated loopback routes for one launched Ditto player.

use std::{
  fs,
  io::Read,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  path::{Component, Path, PathBuf},
  sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

pub use crate::session_mutations::{
  PlayerSessionDurableState, PlayerSessionHandler, PlayerSessionTerminal,
};

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
const NEXT_JOB_WAIT: Duration = Duration::from_secs(30);

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
  state: Arc<SharedState>,
  shutdown: Arc<AtomicBool>,
  worker: Option<JoinHandle<()>>,
  address: SocketAddrV4,
  route_token: String,
  player_session_id: String,
}

struct SharedState {
  value: Mutex<State>,
  changed: Condvar,
}

struct State {
  job: Job,
  job_bytes: Vec<u8>,
  requirements: PlayerSessionRequirements,
  startup: Option<StoredStartup>,
  started_at: Option<Instant>,
  expired: bool,
  next_error: u32,
  warm: bool,
  accepted_report: Option<StartupReport>,
  waiting_for_next_job: bool,
  mutations: MutationState,
  web_root: Option<PathBuf>,
}

struct StoredStartup {
  request_bytes: Vec<u8>,
  fact: StartupFact,
  response_bytes: Vec<u8>,
}

struct Reply {
  status: u16,
  body: Vec<u8>,
  content_type: &'static str,
  content_encoding: Option<&'static str>,
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
    Self::bind_with_identity(job, requirements, Uuid::new_v4().to_string(), handler)
  }

  /// Binds one same-origin WebGL launcher, build, and player API.
  pub fn bind_webgl(
    job: Job,
    requirements: PlayerSessionRequirements,
    web_root: &Path,
    handler: Arc<dyn PlayerSessionHandler>,
  ) -> Result<Self> {
    Self::bind_webgl_with_identity(
      job,
      requirements,
      Uuid::new_v4().to_string(),
      handler,
      web_root,
    )
  }

  pub(crate) fn bind_with_identity(
    job: Job,
    requirements: PlayerSessionRequirements,
    player_session_id: String,
    handler: Arc<dyn PlayerSessionHandler>,
  ) -> Result<Self> {
    Self::bind_inner(job, requirements, player_session_id, handler, None)
  }

  pub(crate) fn bind_webgl_with_identity(
    job: Job,
    requirements: PlayerSessionRequirements,
    player_session_id: String,
    handler: Arc<dyn PlayerSessionHandler>,
    web_root: &Path,
  ) -> Result<Self> {
    ensure!(
      web_root.join("index.html").is_file(),
      "WebGL launcher is missing"
    );
    Self::bind_inner(
      job,
      requirements,
      player_session_id,
      handler,
      Some(web_root.canonicalize()?),
    )
  }

  fn bind_inner(
    job: Job,
    mut requirements: PlayerSessionRequirements,
    player_session_id: String,
    handler: Arc<dyn PlayerSessionHandler>,
    web_root: Option<PathBuf>,
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
    if web_root.is_some() {
      requirements.origin = Some(format!("http://{address}"));
    }
    let server = Arc::new(
      Server::from_listener(listener, None)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("start player session server")?,
    );
    let route_token = Uuid::new_v4().simple().to_string();
    let mutations = MutationState::new(
      job.clone(),
      player_session_id.clone(),
      requirements.storage_directory.clone(),
      handler,
    )?;
    let state = Arc::new(SharedState {
      value: Mutex::new(State {
        job_bytes: serde_json::to_vec(&job)?,
        job,
        requirements,
        startup: None,
        started_at: None,
        expired: false,
        next_error: 1,
        warm: false,
        accepted_report: None,
        waiting_for_next_job: false,
        mutations,
        web_root,
      }),
      changed: Condvar::new(),
    });
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

  /// Returns the loopback origin shared by the launcher and HTTP API.
  pub fn origin(&self) -> String {
    format!("http://{}", self.address)
  }

  /// Returns the isolated launcher URL for this WebGL session.
  pub fn launcher_url(&self) -> String {
    format!("{}/launcher", self.base_url())
  }

  /// Returns the pending player-session identity assigned to this route.
  pub fn player_session_id(&self) -> &str {
    &self.player_session_id
  }

  /// Invalidates the route without stopping the listener.
  pub fn expire(&self) {
    self.state.value.lock().unwrap().expired = true;
    self.state.changed.notify_all();
  }

  /// Installs the next immutable job after the current job is durably terminal.
  pub fn install_job(
    &self,
    job: Job,
    storage_directory: PathBuf,
    handler: Arc<dyn PlayerSessionHandler>,
  ) -> Result<()> {
    job.validate()?;
    let mut state = self.state.value.lock().unwrap();
    ensure!(!state.route_expired(), "player session is stale");
    ensure!(
      state.mutations.durable_state().terminal.is_some(),
      "current watch job is not terminal"
    );
    ensure!(
      state
        .startup
        .as_ref()
        .is_some_and(|startup| startup.fact.decision.action == NextAction::Continue),
      "current player session was not accepted"
    );
    ensure!(
      job.job_id != state.job.job_id,
      "watch job ID was not replaced"
    );
    let report = state
      .accepted_report
      .as_ref()
      .context("accepted player startup report is unavailable")?;
    ensure!(
      job.profile.build_fingerprint == report.build_fingerprint,
      "watch job requires a replacement build"
    );
    ensure!(
      job.profile.source_fingerprint == report.source_fingerprint,
      "watch job requires a replacement source"
    );
    state.mutations = MutationState::new(
      job.clone(),
      self.player_session_id.clone(),
      storage_directory.clone(),
      handler,
    )?;
    state.requirements.storage_directory = storage_directory;
    state.job_bytes = serde_json::to_vec(&job)?;
    state.job = job;
    state.startup = None;
    state.started_at = None;
    state.next_error = 1;
    state.warm = true;
    drop(state);
    self.state.changed.notify_all();
    Ok(())
  }

  /// Waits until the accepted player is polling for another immutable job.
  pub fn wait_for_next_job(&self, timeout: Duration) -> Result<()> {
    let state = self.state.value.lock().unwrap();
    let (state, result) = self
      .state
      .changed
      .wait_timeout_while(state, timeout, |state| {
        !state.route_expired() && !state.waiting_for_next_job
      })
      .unwrap();
    ensure!(!state.route_expired(), "player session is stale");
    ensure!(
      !result.timed_out() && state.waiting_for_next_job,
      "player did not become ready for the next watch job"
    );
    Ok(())
  }

  /// Returns startup facts and route lifetime state.
  pub fn snapshot(&self) -> PlayerSessionSnapshot {
    let state = self.state.value.lock().unwrap();
    PlayerSessionSnapshot {
      startup: state.startup.as_ref().map(|value| value.fact.clone()),
      started_at: state.started_at,
      expired: state.route_expired(),
    }
  }

  /// Returns only bytes and completions already acknowledged by durable storage.
  pub fn durable_state(&self) -> PlayerSessionDurableState {
    self.state.value.lock().unwrap().mutations.durable_state()
  }
}

impl Drop for PlayerSessionServer {
  fn drop(&mut self) {
    self.shutdown.store(true, Ordering::Release);
    self.state.changed.notify_all();
    self.server.unblock();
    if let Some(worker) = self.worker.take() {
      worker.join().unwrap();
    }
  }
}

impl State {
  fn route_expired(&self) -> bool {
    self.expired
      || (self.mutations.durable_state().terminal.is_none()
        && self.started_at.is_some_and(|start| {
          start.elapsed().as_millis() >= self.job.remaining_run_timeout_ms.into()
        }))
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
          content_type: "application/json",
          content_encoding: None,
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
      content_type: "application/json",
      content_encoding: None,
    }
  }

  fn bytes(status: u16, body: Vec<u8>, content_type: &'static str) -> Self {
    Self {
      status,
      body,
      content_type,
      content_encoding: None,
    }
  }

  fn empty(status: u16) -> Self {
    Self::bytes(status, Vec::new(), "application/json")
  }
}

fn serve(mut request: Request, shared: &SharedState, route_token: &str, player_session_id: &str) {
  let prefix = format!("/ditto/{route_token}/next-job?");
  let reply = if request.url().starts_with(&prefix) {
    next_job(&request, shared, &prefix)
  } else {
    dispatch(
      &mut request,
      &mut shared.value.lock().unwrap(),
      route_token,
      player_session_id,
    )
  };
  let content_type = Header::from_bytes("Content-Type", reply.content_type).unwrap();
  let content_length = Header::from_bytes("Content-Length", reply.body.len().to_string()).unwrap();
  let session = Header::from_bytes("X-Ditto-Player-Session-Id", player_session_id).unwrap();
  let opener = Header::from_bytes("Cross-Origin-Opener-Policy", "same-origin").unwrap();
  let embedder = Header::from_bytes("Cross-Origin-Embedder-Policy", "require-corp").unwrap();
  let resource = Header::from_bytes("Cross-Origin-Resource-Policy", "same-origin").unwrap();
  let mut response = Response::from_data(reply.body)
    .with_status_code(StatusCode(reply.status))
    .with_header(content_type)
    .with_header(content_length)
    .with_header(session)
    .with_header(opener)
    .with_header(embedder)
    .with_header(resource);
  if let Some(encoding) = reply.content_encoding {
    response.add_header(Header::from_bytes("Content-Encoding", encoding).unwrap());
  }
  let _ = request.respond(response);
}

fn next_job(request: &Request, shared: &SharedState, prefix: &str) -> Reply {
  if request.method() != &Method::Get {
    return shared
      .value
      .lock()
      .unwrap()
      .error(405, "method is not allowed for the next-job route");
  }
  let Some(after) = request
    .url()
    .strip_prefix(prefix)
    .and_then(|query| query.strip_prefix("after="))
    .filter(|value| !value.is_empty() && !value.contains('&'))
  else {
    return shared
      .value
      .lock()
      .unwrap()
      .error(400, "next-job route requires one after query");
  };
  let mut state = shared.value.lock().unwrap();
  if !origin_allowed(request, state.requirements.origin.as_deref()) {
    return state.error(403, "request origin does not own this player route");
  }
  if after != state.job.job_id {
    return Reply::bytes(200, state.job_bytes.clone(), "application/json");
  }
  state.waiting_for_next_job = true;
  shared.changed.notify_all();
  let (mut state, _) = shared
    .changed
    .wait_timeout_while(state, NEXT_JOB_WAIT, |state| {
      !state.route_expired() && after == state.job.job_id
    })
    .unwrap();
  state.waiting_for_next_job = false;
  if state.route_expired() {
    Reply::empty(410)
  } else if after != state.job.job_id {
    Reply::bytes(200, state.job_bytes.clone(), "application/json")
  } else {
    Reply::empty(204)
  }
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
  if path == "/launcher" && query.is_none() {
    let session_url = format!(
      "{}{prefix}",
      state
        .requirements
        .origin
        .as_deref()
        .expect("WebGL launcher has an origin")
    );
    return launcher_asset(request, state, &session_url);
  }
  if path == "/job" {
    return if request.method() == &Method::Get {
      Reply::bytes(200, state.job_bytes.clone(), "application/json")
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
  if request.method() == &Method::Get
    && query.is_none()
    && let Some(relative) = normalized_asset_path(path)
  {
    return static_asset(request, state, &relative);
  }
  state.error(404, "unknown player route")
}

fn normalized_asset_path(path: &str) -> Option<PathBuf> {
  let relative = Path::new(path.strip_prefix('/')?);
  (!relative.as_os_str().is_empty()
    && relative
      .components()
      .all(|component| matches!(component, Component::Normal(_))))
  .then(|| relative.to_owned())
}

fn static_asset(request: &Request, state: &mut State, relative: &Path) -> Reply {
  if request.method() != &Method::Get {
    return state.error(405, "method is not allowed for WebGL assets");
  }
  let Some(root) = &state.web_root else {
    return state.error(404, "unknown player route");
  };
  let path = root.join(relative);
  let Ok(metadata) = fs::metadata(&path) else {
    return state.error(404, "unknown WebGL asset");
  };
  if !metadata.is_file() {
    return state.error(404, "unknown WebGL asset");
  }
  let Ok(body) = fs::read(&path) else {
    return state.error(500, "WebGL asset could not be read");
  };
  let name = relative.to_string_lossy();
  let mut reply = Reply::bytes(200, body, asset_content_type(&name));
  if name.ends_with(".unityweb") || name.ends_with(".gz") {
    reply.content_encoding = Some("gzip");
  }
  reply
}

fn launcher_asset(request: &Request, state: &mut State, session_url: &str) -> Reply {
  let mut reply = static_asset(request, state, Path::new("index.html"));
  if reply.status != 200 {
    return reply;
  }
  let Ok(html) = String::from_utf8(reply.body) else {
    return state.error(500, "WebGL launcher is not UTF-8");
  };
  if !html.contains("id=\"unity-canvas\"") {
    return state.error(500, "WebGL launcher is missing the Unity canvas");
  }
  let insertion = html.rfind("</body>").unwrap_or(html.len());
  let display = &state.job.profile.display;
  let configure = format!(
    "<script>const dittoCanvas=document.getElementById(\"unity-canvas\");\
     dittoCanvas.width={};dittoCanvas.height={};\
     dittoCanvas.style.width=\"{}px\";dittoCanvas.style.height=\"{}px\";\
     config.arguments=[\"--battlement-ditto-url\",\"{}\"];</script>",
    display.width, display.height, display.width, display.height, session_url
  );
  let mut body = String::with_capacity(html.len() + configure.len());
  body.push_str(&html[..insertion]);
  body.push_str(&configure);
  body.push_str(&html[insertion..]);
  reply.body = body.into_bytes();
  reply
}

fn asset_content_type(name: &str) -> &'static str {
  if name.ends_with(".html") {
    "text/html; charset=utf-8"
  } else if name.ends_with(".css") {
    "text/css; charset=utf-8"
  } else if name.ends_with(".js") || name.ends_with(".js.unityweb") || name.ends_with(".js.gz") {
    "application/javascript"
  } else if name.ends_with(".wasm")
    || name.ends_with(".wasm.unityweb")
    || name.ends_with(".wasm.gz")
  {
    "application/wasm"
  } else if name.ends_with(".png") {
    "image/png"
  } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
    "image/jpeg"
  } else {
    "application/octet-stream"
  }
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
      Reply::bytes(200, stored.response_bytes.clone(), "application/json")
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
    .validate(
      &state.job,
      player_session_id,
      state.warm.then_some(player_session_id),
    )
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
  if decision.action == NextAction::Continue
    && let StartupIdentity::Report(identity) = &started.identity
  {
    state.accepted_report = Some(identity.startup_report.clone());
  }
  state.startup = Some(StoredStartup {
    request_bytes: body,
    fact: StartupFact { started, decision },
    response_bytes: response_bytes.clone(),
  });
  Reply::bytes(200, response_bytes, "application/json")
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
    StartupIdentity::Accepted(_) if !state.warm => Some("new player did not report startup facts"),
    StartupIdentity::Accepted(_) => None,
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
  if !display_matches(profile.platform, &profile.display, &report.display) {
    return Some("wrong display");
  }
  if report.capabilities != profile.capabilities {
    return Some("wrong capabilities");
  }
  None
}

fn display_matches(
  platform: crate::wire::job::Platform,
  expected: &crate::wire::job::Display,
  actual: &crate::wire::job::Display,
) -> bool {
  if platform != crate::wire::job::Platform::IosSimulator {
    return actual == expected;
  }
  actual.width == expected.width
    && actual.height == expected.height
    && actual.scale == expected.scale
    && actual.orientation == expected.orientation
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
