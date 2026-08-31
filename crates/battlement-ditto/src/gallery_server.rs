//! Loopback-only gallery for one suite and its canonical screenshots.

use std::{
  collections::BTreeMap,
  fs,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
  time::Duration,
};

use anyhow::{Context, Result};
use serde::Serialize;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const INDEX: &str = include_str!("gallery/index.html");
const STYLES: &str = include_str!("gallery/app.css");
const SCRIPT: &str = include_str!("gallery/app.js");

/// Source and canonical screenshots displayed by one gallery.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GalleryDocument {
  pub suite: String,
  pub profile: String,
  pub filename: String,
  pub source: String,
  pub checkpoints: Vec<GalleryCheckpoint>,
}

/// One screenshot placement within the suite source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GalleryCheckpoint {
  pub after_line: usize,
  pub scenario: String,
  pub checkpoint: String,
  pub image: Option<String>,
  pub width: Option<u32>,
  pub height: Option<u32>,
}

/// Serves a gallery and its allow-listed PNG objects on IPv4 loopback.
pub struct GalleryServer {
  server: Server,
  address: SocketAddrV4,
  document: Vec<u8>,
  images: BTreeMap<String, PathBuf>,
}

struct Reply {
  status: u16,
  content_type: &'static str,
  body: Vec<u8>,
}

impl GalleryServer {
  /// Binds a gallery to the requested port or an available port when omitted.
  pub fn bind(
    document: GalleryDocument,
    images: BTreeMap<String, PathBuf>,
    port: Option<u16>,
  ) -> Result<Self> {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port.unwrap_or(0)))?;
    let address = match listener.local_addr()? {
      SocketAddr::V4(address) => address,
      SocketAddr::V6(_) => unreachable!("an IPv4 listener returned an IPv6 address"),
    };
    Ok(Self {
      server: Server::from_listener(listener, None)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("start gallery server")?,
      address,
      document: serde_json::to_vec(&document)?,
      images,
    })
  }

  /// Returns the gallery's loopback URL.
  pub fn url(&self) -> String {
    format!("http://{}/", self.address)
  }

  /// Serves requests until interrupted.
  pub fn serve(&self, interrupted: &AtomicBool) -> Result<()> {
    while !interrupted.load(Ordering::Acquire) {
      if let Some(request) = self.server.recv_timeout(Duration::from_millis(50))? {
        self.respond(request);
      }
    }
    Ok(())
  }

  fn respond(&self, request: Request) {
    let reply = self.dispatch(&request);
    let response = Response::from_data(reply.body)
      .with_status_code(StatusCode(reply.status))
      .with_header(header("Content-Type", reply.content_type))
      .with_header(header("Cache-Control", "no-store"))
      .with_header(header("X-Content-Type-Options", "nosniff"))
      .with_header(header(
        "Content-Security-Policy",
        "default-src 'self'; img-src 'self'; style-src 'self'; script-src 'self'",
      ));
    let _ = request.respond(response);
  }

  fn dispatch(&self, request: &Request) -> Reply {
    if request.method() != &Method::Get {
      return Reply::text(405, "method is not allowed");
    }
    match request.url() {
      "/" | "/index.html" => Reply::asset("text/html; charset=utf-8", INDEX),
      "/app.css" => Reply::asset("text/css; charset=utf-8", STYLES),
      "/app.js" => Reply::asset("text/javascript; charset=utf-8", SCRIPT),
      "/api/gallery" => Reply {
        status: 200,
        content_type: "application/json",
        body: self.document.clone(),
      },
      route => self.image(route),
    }
  }

  fn image(&self, route: &str) -> Reply {
    let Some(path) = self.images.get(route) else {
      return Reply::text(404, "not found");
    };
    match fs::read(path) {
      Ok(body) => Reply {
        status: 200,
        content_type: "image/png",
        body,
      },
      Err(_) => Reply::text(404, "canonical screenshot is unavailable"),
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
  Header::from_bytes(name, value).expect("static gallery header is valid")
}
