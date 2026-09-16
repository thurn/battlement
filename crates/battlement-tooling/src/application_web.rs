use std::{
  fs,
  path::{Component, Path, PathBuf},
  process::Command,
  sync::atomic::{AtomicBool, Ordering},
  time::Duration,
};

use anyhow::{Context, Result, bail};
use percent_encoding::percent_decode_str;
use tiny_http::{Header, Request, Response, Server, StatusCode};

pub(crate) fn serve(output: &Path, port: u16, interrupted: &AtomicBool) -> Result<()> {
  let server = Server::http(("127.0.0.1", port))
    .map_err(|error| anyhow::anyhow!("failed to bind Web server to port {port}: {error}"))?;
  let url = format!("http://127.0.0.1:{port}/");
  println!("Running Reactant Web application at {url}");
  println!("Press Ctrl-C to stop.");
  #[cfg(windows)]
  let status = Command::new("rundll32.exe")
    .args(["url.dll,FileProtocolHandler", &url])
    .status();
  #[cfg(not(windows))]
  let status = Command::new("open").arg(&url).status();
  let status = status.context("failed to open the Web application in a browser")?;
  if !status.success() {
    bail!("browser opener exited with status {status}");
  }
  while !interrupted.load(Ordering::SeqCst) {
    if let Some(request) = server.recv_timeout(Duration::from_millis(50))? {
      respond(request, output)?;
    }
  }
  Ok(())
}

fn respond(request: Request, output: &Path) -> Result<()> {
  let url = request.url().split('?').next().unwrap_or("/");
  let decoded = percent_decode_str(url)
    .decode_utf8()
    .context("Web request path is not UTF-8")?;
  let mut relative = PathBuf::new();
  for component in Path::new(decoded.trim_start_matches('/')).components() {
    match component {
      Component::Normal(value) => relative.push(value),
      Component::CurDir => {}
      _ => {
        request.respond(Response::empty(StatusCode(404)))?;
        return Ok(());
      }
    }
  }
  if relative.as_os_str().is_empty() {
    relative.push("index.html");
  }
  let path = output.join(relative);
  let bytes = match fs::read(&path) {
    Ok(bytes) => bytes,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
      request.respond(Response::empty(StatusCode(404)))?;
      return Ok(());
    }
    Err(error) => return Err(error).with_context(|| format!("failed to read {}", path.display())),
  };
  let name = path
    .file_name()
    .and_then(|value| value.to_str())
    .unwrap_or("");
  let content_type = if name.ends_with(".html") {
    "text/html; charset=utf-8"
  } else if name.ends_with(".js") || name.ends_with(".js.unityweb") {
    "application/javascript"
  } else if name.ends_with(".wasm") || name.ends_with(".wasm.unityweb") {
    "application/wasm"
  } else if name.ends_with(".json") {
    "application/json"
  } else {
    "application/octet-stream"
  };
  let mut response = Response::from_data(bytes)
    .with_header(header("Content-Type", content_type)?)
    .with_header(header("Cross-Origin-Opener-Policy", "same-origin")?)
    .with_header(header("Cross-Origin-Embedder-Policy", "require-corp")?)
    .with_header(header("Cross-Origin-Resource-Policy", "same-origin")?);
  if name.ends_with(".unityweb") {
    response.add_header(header("Content-Encoding", "gzip")?);
  }
  request.respond(response)?;
  Ok(())
}

fn header(name: &str, value: &str) -> Result<Header> {
  Header::from_bytes(name, value).map_err(|_| anyhow::anyhow!("invalid Web response header"))
}
