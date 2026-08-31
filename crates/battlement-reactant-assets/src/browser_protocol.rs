use std::{
  fs,
  net::TcpStream,
  path::Path,
  process::{Child, Command, Stdio},
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tungstenite::{Message, WebSocket, connect, stream::MaybeTlsStream};

use crate::{
  WorkReport, browser::BrowserIdentity, incremental::FileFingerprint,
  renderer_document::RenderDocument,
};

pub(crate) struct BrowserSession {
  protocol: Protocol,
  child: Option<Child>,
  _profile: TempDir,
  context_id: String,
  session_id: String,
}

struct Protocol {
  socket: WebSocket<MaybeTlsStream<TcpStream>>,
  next_id: u64,
}

impl BrowserSession {
  pub(crate) fn launch(
    executable: &Path,
    executable_fingerprint: FileFingerprint,
    cached_hash: Option<&str>,
    explicit: bool,
    report: &mut WorkReport,
  ) -> Result<(Self, BrowserIdentity)> {
    let profile = tempfile::tempdir().context("failed to create isolated browser profile")?;
    let mut child = Command::new(executable)
      .args([
        "--headless=new",
        "--remote-debugging-port=0",
        "--remote-allow-origins=*",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-domain-reliability",
        "--disable-extensions",
        "--disable-sync",
        "--metrics-recording-only",
        "--no-pings",
        "--force-color-profile=srgb",
        "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost",
        "--proxy-server=direct://",
        "--lang=en-US",
      ])
      .arg(format!("--user-data-dir={}", profile.path().display()))
      .arg("about:blank")
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
      .with_context(|| format!("failed to launch browser {}", executable.display()))?;
    report.browser_launches += 1;
    report.subprocesses_started += 1;
    let endpoint = match self::wait_for_endpoint(&mut child, &profile, explicit, report) {
      Ok(endpoint) => endpoint,
      Err(error) => {
        self::terminate_child(&mut child);
        return Err(error);
      }
    };
    let connection = connect(endpoint.as_str())
      .with_context(|| format!("failed to connect to browser protocol at {endpoint}"));
    let (mut socket, _) = match connection {
      Ok(connection) => connection,
      Err(error) => {
        self::terminate_child(&mut child);
        return Err(error);
      }
    };
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
      stream.set_read_timeout(Some(Duration::from_secs(10)))?;
      stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    }
    let mut session = Self {
      protocol: Protocol { socket, next_id: 1 },
      child: Some(child),
      _profile: profile,
      context_id: String::new(),
      session_id: String::new(),
    };
    let version = session
      .protocol
      .command("Browser.getVersion", json!({}), None)?;
    let reported_product = self::required_string(&version, "product")?;
    if !self::supported_product(&reported_product) {
      bail!(
        "browser {} reported unsupported product {reported_product}; expected stable Chrome or Chromium",
        executable.display()
      );
    }
    let (product, browser_version) = reported_product
      .split_once('/')
      .filter(|(product, version)| !product.is_empty() && !version.is_empty())
      .context("browser reported an invalid product identity")?;
    let executable_sha256 = if let Some(hash) = cached_hash {
      hash.to_owned()
    } else {
      let bytes = fs::read(executable)
        .with_context(|| format!("failed to read browser executable {}", executable.display()))?;
      report.files_opened += 1;
      report.browser_executable_opens += 1;
      report.bytes_read += bytes.len() as u64;
      self::hex(&Sha256::digest(bytes))
    };
    let identity = BrowserIdentity {
      executable_path: self::normalized(executable),
      executable_fingerprint,
      executable_sha256,
      product: product.to_owned(),
      version: browser_version.to_owned(),
      protocol_version: self::required_string(&version, "protocolVersion")?,
      revision: self::required_string(&version, "revision")?,
      user_agent: self::required_string(&version, "userAgent")?,
      javascript_version: self::required_string(&version, "jsVersion")?,
    };
    session.configure(report)?;
    Ok((session, identity))
  }

  pub(crate) fn render(
    &mut self,
    document: &RenderDocument,
    subject: battlement_reactant_asset_syntax::LogicalRect,
  ) -> Result<Vec<u8>> {
    self.protocol.command(
      "Emulation.setDeviceMetricsOverride",
      json!({
        "width": document.viewport_width,
        "height": document.viewport_height,
        "deviceScaleFactor": document.scale,
        "mobile": false
      }),
      Some(&self.session_id),
    )?;
    self.protocol.command(
      "Page.navigate",
      json!({"url": document.data_url()}),
      Some(&self.session_id),
    )?;
    self.wait_for_document(&document.key)?;
    let expression = document.setup_expression(subject)?;
    self.evaluate(&expression, true)?;
    let captured = self.protocol.command(
      "Page.captureScreenshot",
      json!({
        "format": "png",
        "fromSurface": true,
        "captureBeyondViewport": false,
        "clip": {"x": 0, "y": 0, "width": document.width, "height": document.height, "scale": 1}
      }),
      Some(&self.session_id),
    );
    let cleanup = self.evaluate(RenderDocument::cleanup_expression(), false);
    let captured = captured?;
    cleanup?;
    let encoded = self::required_string(&captured, "data")?;
    base64::engine::general_purpose::STANDARD
      .decode(encoded)
      .context("browser returned invalid screenshot bytes")
  }

  fn wait_for_document(&mut self, key: &str) -> Result<()> {
    let expected = serde_json::to_string(key)?;
    let expression = format!(
      "document.readyState==='complete'&&document.querySelector('meta[name=reactant-key]')?.content==={expected}"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      if self.evaluate(&expression, false).ok() == Some(Value::Bool(true)) {
        return Ok(());
      }
      if Instant::now() >= deadline {
        bail!("browser did not load the isolated render document within 10 seconds");
      }
      thread::sleep(Duration::from_millis(10));
    }
  }

  fn evaluate(&mut self, expression: &str, await_promise: bool) -> Result<Value> {
    let response = self.protocol.command(
      "Runtime.evaluate",
      json!({
        "expression": expression,
        "awaitPromise": await_promise,
        "returnByValue": true
      }),
      Some(&self.session_id),
    )?;
    if let Some(exception) = response.get("exceptionDetails") {
      let description = response
        .pointer("/result/description")
        .and_then(Value::as_str)
        .unwrap_or("browser render script failed");
      bail!("{description}: {exception}");
    }
    Ok(
      response
        .pointer("/result/value")
        .cloned()
        .unwrap_or(Value::Null),
    )
  }

  pub(crate) fn finish(mut self) -> Result<()> {
    self.protocol.command(
      "Target.disposeBrowserContext",
      json!({"browserContextId": self.context_id}),
      None,
    )?;
    let _ = self.protocol.command("Browser.close", json!({}), None);
    self::stop_child(&mut self.child);
    Ok(())
  }

  fn configure(&mut self, report: &mut WorkReport) -> Result<()> {
    let context = self.protocol.command(
      "Target.createBrowserContext",
      json!({"disposeOnDetach": true, "proxyServer": "direct://"}),
      None,
    )?;
    self.context_id = self::required_string(&context, "browserContextId")?;
    report.browser_contexts_created += 1;
    let target = self.protocol.command(
      "Target.createTarget",
      json!({"url": "about:blank", "browserContextId": self.context_id}),
      None,
    )?;
    let target_id = self::required_string(&target, "targetId")?;
    let attached = self.protocol.command(
      "Target.attachToTarget",
      json!({"targetId": target_id, "flatten": true}),
      None,
    )?;
    self.session_id = self::required_string(&attached, "sessionId")?;
    let session = Some(self.session_id.as_str());
    for (method, parameters) in [
      ("Page.enable", json!({})),
      ("Network.enable", json!({})),
      ("Network.setCacheDisabled", json!({"cacheDisabled": true})),
      ("Network.setBypassServiceWorker", json!({"bypass": true})),
      (
        "Network.setBlockedURLs",
        json!({"urls": ["http://*", "https://*", "ftp://*", "ws://*", "wss://*"]}),
      ),
      ("ServiceWorker.disable", json!({})),
      ("Emulation.setLocaleOverride", json!({"locale": "en-US"})),
      (
        "Emulation.setTimezoneOverride",
        json!({"timezoneId": "UTC"}),
      ),
      (
        "Emulation.setEmulatedMedia",
        json!({
          "media": "screen",
          "features": [
            {"name": "prefers-color-scheme", "value": "light"},
            {"name": "prefers-reduced-motion", "value": "reduce"}
          ]
        }),
      ),
      (
        "Emulation.setDeviceMetricsOverride",
        json!({"width": 8, "height": 8, "deviceScaleFactor": 1, "mobile": false}),
      ),
      (
        "Emulation.setDefaultBackgroundColorOverride",
        json!({"color": {"r": 0, "g": 0, "b": 0, "a": 0}}),
      ),
    ] {
      self.protocol.command(method, parameters, session)?;
    }
    let source = "document.documentElement.style.fontSize='16px';document.documentElement.style.colorScheme='light';";
    self.protocol.command(
      "Page.addScriptToEvaluateOnNewDocument",
      json!({"source": source}),
      session,
    )?;
    self.protocol.command(
      "Runtime.evaluate",
      json!({"expression": source, "awaitPromise": true}),
      session,
    )?;
    Ok(())
  }
}

impl Drop for BrowserSession {
  fn drop(&mut self) {
    self::stop_child(&mut self.child);
  }
}

impl Protocol {
  fn command(&mut self, method: &str, params: Value, session: Option<&str>) -> Result<Value> {
    let id = self.next_id;
    self.next_id += 1;
    let mut request = json!({"id": id, "method": method, "params": params});
    if let Some(session) = session {
      request["sessionId"] = Value::String(session.to_owned());
    }
    self
      .socket
      .send(Message::Text(request.to_string().into()))?;
    loop {
      let message = self.socket.read()?;
      let Message::Text(text) = message else {
        continue;
      };
      let response: Value = serde_json::from_str(&text)?;
      if response.get("id").and_then(Value::as_u64) != Some(id) {
        continue;
      }
      if let Some(error) = response.get("error") {
        bail!("browser protocol {method} failed: {error}");
      }
      return response
        .get("result")
        .cloned()
        .with_context(|| format!("browser protocol {method} omitted its result"));
    }
  }
}

fn wait_for_endpoint(
  child: &mut Child,
  profile: &TempDir,
  explicit: bool,
  report: &mut WorkReport,
) -> Result<String> {
  let active_port = profile.path().join("DevToolsActivePort");
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    if let Ok(contents) = fs::read_to_string(&active_port) {
      report.files_opened += 1;
      report.bytes_read += contents.len() as u64;
      if let Some(endpoint) = self::debugging_endpoint(&contents) {
        return Ok(endpoint);
      }
    }
    if let Some(status) = child.try_wait()? {
      let selection = if explicit {
        "explicit browser"
      } else {
        "browser"
      };
      bail!(
        "{selection} exited with {status} before reporting a Chrome or Chromium debugging endpoint"
      );
    }
    if Instant::now() >= deadline {
      bail!("browser did not report its debugging endpoint within 10 seconds");
    }
    thread::sleep(Duration::from_millis(20));
  }
}

fn debugging_endpoint(contents: &str) -> Option<String> {
  let mut lines = contents.lines();
  let port = lines.next()?.parse::<u16>().ok()?;
  let path = lines.next()?;
  if path.is_empty() || !path.starts_with('/') {
    return None;
  }
  Some(format!("ws://127.0.0.1:{port}{path}"))
}

fn stop_child(child: &mut Option<Child>) {
  let Some(mut child) = child.take() else {
    return;
  };
  let deadline = Instant::now() + Duration::from_secs(3);
  while Instant::now() < deadline {
    if child.try_wait().ok().flatten().is_some() {
      return;
    }
    thread::sleep(Duration::from_millis(20));
  }
  let _ = child.kill();
  let _ = child.wait();
}

fn terminate_child(child: &mut Child) {
  let _ = child.kill();
  let _ = child.wait();
}

fn required_string(value: &Value, field: &str) -> Result<String> {
  value
    .get(field)
    .and_then(Value::as_str)
    .map(str::to_owned)
    .with_context(|| format!("browser protocol response omitted {field}"))
}

fn supported_product(product: &str) -> bool {
  ["Chrome/", "HeadlessChrome/", "Chromium/"]
    .iter()
    .any(|prefix| product.starts_with(prefix))
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

fn hex(bytes: &[u8]) -> String {
  const DIGITS: &[u8; 16] = b"0123456789abcdef";

  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
  }
  output
}
