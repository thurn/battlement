//! Loopback-only read access to one immutable Ditto run.

use std::{
  collections::BTreeSet,
  fs,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  path::{Path, PathBuf},
  sync::atomic::{AtomicBool, Ordering},
  time::Duration,
};

use anyhow::{Context, Result, ensure};
use percent_encoding::percent_decode_str;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::wire::result::RunResult;

const INDEX: &str = include_str!("review/index.html");
const STYLES: &str = include_str!("review/app.css");
const SCRIPT: &str = include_str!("review/app.js");
const SLIDER: &str = include_str!("review/vendor/img-comparison-slider.js");
const PANZOOM: &str = include_str!("review/vendor/panzoom.min.js");

/// One read-only review application bound to explicit IPv4 loopback.
pub struct ReviewServer {
  server: Server,
  address: SocketAddrV4,
  directory: PathBuf,
  result: Vec<u8>,
  artifacts: BTreeSet<String>,
}

struct Reply {
  status: u16,
  content_type: &'static str,
  body: Vec<u8>,
}

impl ReviewServer {
  /// Binds a review server for one validated terminal result.
  pub fn bind(directory: impl Into<PathBuf>, result: RunResult) -> Result<Self> {
    result.validate()?;
    let directory = directory.into();
    ensure!(directory.is_dir(), "review run directory does not exist");
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
      directory,
      result: result.to_canonical_json()?,
      artifacts: result.artifacts.into_iter().collect(),
    })
  }

  /// Returns the loopback URL for this review application.
  pub fn url(&self) -> String {
    format!("http://{}/", self.address)
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

  fn respond(&self, request: Request) {
    let reply = if request.method() != &Method::Get {
      Reply::text(405, "method is not allowed")
    } else {
      self.dispatch(request.url())
    };
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

  fn dispatch(&self, url: &str) -> Reply {
    match url {
      "/" | "/index.html" => Reply::asset("text/html; charset=utf-8", INDEX),
      "/app.css" => Reply::asset("text/css; charset=utf-8", STYLES),
      "/app.js" => Reply::asset("text/javascript; charset=utf-8", SCRIPT),
      "/vendor/img-comparison-slider.js" => Reply::asset("text/javascript; charset=utf-8", SLIDER),
      "/vendor/panzoom.min.js" => Reply::asset("text/javascript; charset=utf-8", PANZOOM),
      "/api/result" => Reply {
        status: 200,
        content_type: "application/json",
        body: self.result.clone(),
      },
      _ => self.artifact(url),
    }
  }

  fn artifact(&self, url: &str) -> Reply {
    let Some(encoded) = url.strip_prefix("/artifact/") else {
      return Reply::text(404, "not found");
    };
    let Ok(path) = percent_decode_str(encoded).decode_utf8() else {
      return Reply::text(400, "artifact path is malformed");
    };
    if !self.artifacts.contains(path.as_ref()) {
      return Reply::text(404, "artifact is not part of this result");
    }
    let absolute = self.directory.join(path.as_ref());
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
}

fn header(name: &str, value: &str) -> Header {
  Header::from_bytes(name, value).expect("static review header is valid")
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
