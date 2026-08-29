//! Loopback-only read access to one immutable Ditto run.

use std::{
  collections::{BTreeSet, VecDeque},
  fs,
  io::Read,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  path::{Path, PathBuf},
  sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use anyhow::{Context, Result, ensure};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

use crate::{
  review_acceptance::ReviewAcceptanceService,
  wire::{
    result::RunResult,
    review::{ReviewEvent, ReviewEventBody},
  },
};

const INDEX: &str = include_str!("review/index.html");
const STYLES: &str = include_str!("review/app.css");
const SCRIPT: &str = include_str!("review/app.js");
const ACCEPTANCE_SCRIPT: &str = include_str!("review/acceptance.js");
const SLIDER: &str = include_str!("review/vendor/img-comparison-slider.js");
const PANZOOM: &str = include_str!("review/vendor/panzoom.min.js");
const RETAINED_EVENTS: usize = 64;

/// One read-only review application bound to explicit IPv4 loopback.
pub struct ReviewServer {
  server: Server,
  address: SocketAddrV4,
  state: Mutex<ReviewState>,
}

struct ReviewState {
  directory: PathBuf,
  result: Vec<u8>,
  artifacts: BTreeSet<String>,
  acceptance: Option<ReviewAcceptanceService>,
  acceptance_token: String,
  disabled_reason: Option<String>,
  events: VecDeque<ReviewEvent>,
  next_event_id: u64,
}

struct Reply {
  status: u16,
  content_type: &'static str,
  body: Vec<u8>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceCapability<'a> {
  enabled: bool,
  token: Option<&'a str>,
  reason: Option<&'a str>,
}

impl ReviewServer {
  /// Binds a review server for one validated terminal result.
  pub fn bind(directory: impl Into<PathBuf>, result: RunResult) -> Result<Self> {
    Self::bind_state(
      directory.into(),
      result,
      None,
      Some("Baseline write credentials are unavailable.".to_owned()),
    )
  }

  /// Binds a review server with atomic baseline acceptance enabled.
  pub fn bind_accepting(
    directory: impl Into<PathBuf>,
    result: RunResult,
    acceptance: ReviewAcceptanceService,
  ) -> Result<Self> {
    Self::bind_state(directory.into(), result, Some(acceptance), None)
  }

  /// Binds a readable review server with an explicit disabled explanation.
  pub fn bind_disabled(
    directory: impl Into<PathBuf>,
    result: RunResult,
    reason: String,
  ) -> Result<Self> {
    Self::bind_state(directory.into(), result, None, Some(reason))
  }

  fn bind_state(
    directory: PathBuf,
    result: RunResult,
    acceptance: Option<ReviewAcceptanceService>,
    disabled_reason: Option<String>,
  ) -> Result<Self> {
    result.validate()?;
    ensure!(directory.is_dir(), "review run directory does not exist");
    let initial_event = ReviewEvent {
      id: 1,
      body: ReviewEventBody::Snapshot {
        result: result.clone(),
      },
    };
    initial_event.validate()?;
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
    let address = match listener.local_addr()? {
      SocketAddr::V4(address) => address,
      SocketAddr::V6(_) => unreachable!("an IPv4 listener returned an IPv6 address"),
    };
    Ok(Self {
      server: Server::from_listener(listener, None)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("start review server")?,
      address,
      state: Mutex::new(ReviewState {
        directory,
        result: result.to_canonical_json()?,
        artifacts: result.artifacts.into_iter().collect(),
        acceptance,
        acceptance_token: Uuid::new_v4().to_string(),
        disabled_reason,
        events: VecDeque::from([initial_event]),
        next_event_id: 2,
      }),
    })
  }

  /// Returns the loopback URL for this review application.
  pub fn url(&self) -> String {
    format!("http://{}/", self.address)
  }

  /// Switches the live review state and emits one replayable snapshot event.
  pub fn publish(&self, directory: impl Into<PathBuf>, result: RunResult) -> Result<()> {
    result.validate()?;
    let directory = directory.into();
    ensure!(directory.is_dir(), "review run directory does not exist");
    self.state.lock().unwrap().replace(directory, result)
  }

  /// Returns the immutable result currently displayed by the live tab.
  pub fn current_result(&self) -> Result<RunResult> {
    Ok(serde_json::from_slice(&self.state.lock().unwrap().result)?)
  }

  /// Serves review requests until the caller signals interruption.
  pub fn serve(&self, interrupted: &AtomicBool) -> Result<()> {
    while !interrupted.load(Ordering::Acquire) {
      if let Some(request) = self.server.recv_timeout(Duration::from_millis(50))? {
        self.respond(request);
      }
    }
    Ok(())
  }

  fn respond(&self, mut request: Request) {
    let reply = self.dispatch(&mut request);
    let response = Response::from_data(reply.body)
      .with_status_code(StatusCode(reply.status))
      .with_header(header("Content-Type", reply.content_type))
      .with_header(header("Cache-Control", "no-store"))
      .with_header(header("X-Content-Type-Options", "nosniff"))
      .with_header(header(
        "Content-Security-Policy",
        "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self'",
      ));
    let _ = request.respond(response);
  }

  fn dispatch(&self, request: &mut Request) -> Reply {
    let url = request.url().to_owned();
    if request.method() == &Method::Post {
      return self.accept(request, &url);
    }
    if request.method() != &Method::Get {
      return Reply::text(405, "method is not allowed");
    }
    match url.as_str() {
      "/" | "/index.html" => Reply::asset("text/html; charset=utf-8", INDEX),
      "/app.css" => Reply::asset("text/css; charset=utf-8", STYLES),
      "/app.js" => Reply::asset("text/javascript; charset=utf-8", SCRIPT),
      "/acceptance.js" => Reply::asset("text/javascript; charset=utf-8", ACCEPTANCE_SCRIPT),
      "/vendor/img-comparison-slider.js" => Reply::asset("text/javascript; charset=utf-8", SLIDER),
      "/vendor/panzoom.min.js" => Reply::asset("text/javascript; charset=utf-8", PANZOOM),
      "/api/result" => Reply {
        status: 200,
        content_type: "application/json",
        body: self.state.lock().unwrap().result.clone(),
      },
      "/api/acceptance" => self.capability(),
      "/api/events" => self.events(request),
      _ => self.artifact(&url),
    }
  }

  fn capability(&self) -> Reply {
    let state = self.state.lock().unwrap();
    Reply::json(
      200,
      &AcceptanceCapability {
        enabled: state.acceptance.is_some(),
        token: state
          .acceptance
          .as_ref()
          .map(|_| state.acceptance_token.as_str()),
        reason: state.disabled_reason.as_deref(),
      },
    )
  }

  fn events(&self, request: &Request) -> Reply {
    let state = self.state.lock().unwrap();
    let last = match request_header(request, "Last-Event-ID") {
      Some(value) => match value.parse::<u64>() {
        Ok(value) => Some(value),
        Err(_) => return Reply::text(400, "Last-Event-ID is malformed"),
      },
      None => None,
    };
    let oldest = state.events.front().map_or(0, |event| event.id);
    let replay_lost = last.is_some_and(|id| id.saturating_add(1) < oldest);
    let events: Vec<_> = if replay_lost || last.is_none() {
      state.events.back().into_iter().collect()
    } else {
      state
        .events
        .iter()
        .filter(|event| event.id > last.unwrap())
        .collect()
    };
    let mut body = b"retry: 250\n\n".to_vec();
    for event in events {
      body.extend_from_slice(format!("id: {}\nevent: review\ndata: ", event.id).as_bytes());
      body.extend_from_slice(&serde_json::to_vec(event).expect("review event serializes"));
      body.extend_from_slice(b"\n\n");
    }
    Reply {
      status: 200,
      content_type: "text/event-stream; charset=utf-8",
      body,
    }
  }

  fn accept(&self, request: &mut Request, url: &str) -> Reply {
    if !url.starts_with("/api/accept/") {
      return Reply::text(405, "method is not allowed");
    }
    let mut state = self.state.lock().unwrap();
    let expected = format!("/api/accept/{}", state.acceptance_token);
    if url != expected {
      return Reply::text(404, "not found");
    }
    let Some(acceptance) = &mut state.acceptance else {
      return Reply::text(403, "baseline acceptance is disabled");
    };
    let mut body = Vec::new();
    if request
      .as_reader()
      .take(1024 * 1024)
      .read_to_end(&mut body)
      .is_err()
    {
      return Reply::text(400, "acceptance body could not be read");
    }
    let accepted = acceptance.accept(&body);
    if let Some((directory, result)) = accepted.replacement {
      state
        .replace(directory, result)
        .expect("accepted result remains valid");
    }
    Reply {
      status: accepted.status,
      content_type: "application/json",
      body: accepted.body,
    }
  }

  fn artifact(&self, url: &str) -> Reply {
    let Some(encoded) = url.strip_prefix("/artifact/") else {
      return Reply::text(404, "not found");
    };
    let Ok(path) = percent_decode_str(encoded).decode_utf8() else {
      return Reply::text(400, "artifact path is malformed");
    };
    let state = self.state.lock().unwrap();
    if !state.artifacts.contains(path.as_ref()) {
      return Reply::text(404, "artifact is not part of this result");
    }
    let absolute = state.directory.join(path.as_ref());
    let Ok(metadata) = fs::symlink_metadata(&absolute) else {
      return Reply::text(404, "retained artifact is unavailable");
    };
    if !metadata.file_type().is_file() {
      return Reply::text(404, "retained artifact is unavailable");
    }
    match fs::read(&absolute) {
      Ok(body) => Reply {
        status: 200,
        content_type: content_type(&absolute),
        body,
      },
      Err(_) => Reply::text(404, "retained artifact is unavailable"),
    }
  }
}

impl ReviewState {
  fn replace(&mut self, directory: PathBuf, result: RunResult) -> Result<()> {
    let event = ReviewEvent {
      id: self.next_event_id,
      body: ReviewEventBody::Snapshot {
        result: result.clone(),
      },
    };
    event.validate()?;
    if let Some(acceptance) = &mut self.acceptance {
      acceptance.replace_reviewed(result.clone(), directory.clone())?;
    }
    self.directory = directory;
    self.artifacts = result.artifacts.iter().cloned().collect();
    self.result = result.to_canonical_json()?;
    self.events.push_back(event);
    self.next_event_id += 1;
    if self.events.len() > RETAINED_EVENTS {
      self.events.pop_front();
    }
    Ok(())
  }
}

impl Reply {
  fn asset(content_type: &'static str, source: &str) -> Self {
    Self {
      status: 200,
      content_type,
      body: source.as_bytes().to_vec(),
    }
  }

  fn text(status: u16, message: &str) -> Self {
    Self {
      status,
      content_type: "text/plain; charset=utf-8",
      body: message.as_bytes().to_vec(),
    }
  }

  fn json(status: u16, value: &impl Serialize) -> Self {
    Self {
      status,
      content_type: "application/json",
      body: serde_json::to_vec(value).expect("review response serializes"),
    }
  }
}

fn header(name: &str, value: &str) -> Header {
  Header::from_bytes(name, value).expect("static review header is valid")
}

fn request_header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
  request
    .headers()
    .iter()
    .find(|header| header.field.equiv(name))
    .map(|header| header.value.as_str())
}

fn content_type(path: &Path) -> &'static str {
  match path.extension().and_then(|extension| extension.to_str()) {
    Some("png") => "image/png",
    Some("json") => "application/json",
    Some("jsonl") => "application/x-ndjson; charset=utf-8",
    Some("log" | "txt") => "text/plain; charset=utf-8",
    Some("mp4") => "video/mp4",
    _ => "application/octet-stream",
  }
}
