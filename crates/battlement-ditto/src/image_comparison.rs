//! Image comparison through one persistent ODiff server.

use std::{
  fs::{self, File},
  io::{BufRead, BufReader, BufWriter, Write},
  path::Path,
  process::{Child, ChildStdin, Command, Stdio},
  str::FromStr,
  sync::{
    Mutex,
    mpsc::{self, Receiver},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Number, json};
use sha2::{Digest, Sha256};

use crate::wire::{
  job::Comparison,
  result::{ComparisonOutcome, ImageFile},
};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Inputs for one comparison performed by a warm ODiff process.
pub struct ImageComparisonRequest<'a> {
  pub baseline: &'a Path,
  pub actual: &'a Path,
  pub diff: &'a Path,
  pub settings: Comparison,
  pub timeout: Duration,
}

/// A durable comparison outcome and any generated red difference mask.
#[derive(Debug, PartialEq)]
pub struct ImageComparison {
  pub outcome: ComparisonOutcome,
  pub diff: Option<ImageFile>,
}

/// One persistent ODiff server owned by a Ditto run.
pub struct OdiffServer {
  child: Child,
  input: BufWriter<ChildStdin>,
  output: Receiver<String>,
  next_request_id: u64,
  failed: bool,
}

/// Lazily owns one ODiff process that may be shared across immutable watch cycles.
#[derive(Default)]
pub struct OdiffPool {
  server: Mutex<Option<OdiffServer>>,
}

impl OdiffPool {
  /// Compares through the existing process, starting or replacing it when necessary.
  pub fn compare(
    &self,
    binary: &Path,
    diagnostic: &Path,
    startup_timeout: Duration,
    request: ImageComparisonRequest<'_>,
  ) -> Result<ImageComparison> {
    self.with_server(binary, diagnostic, startup_timeout, |server| {
      server.compare(request)
    })
  }

  /// Runs one operation with the retained process, replacing it after a failure.
  pub fn with_server<T>(
    &self,
    binary: &Path,
    diagnostic: &Path,
    startup_timeout: Duration,
    operation: impl FnOnce(&mut OdiffServer) -> Result<T>,
  ) -> Result<T> {
    let mut server = self.server.lock().unwrap();
    if server.is_none() {
      if let Some(parent) = diagnostic.parent() {
        fs::create_dir_all(parent)?;
      }
      *server = Some(OdiffServer::start(binary, diagnostic, startup_timeout)?);
    }
    let result = operation(server.as_mut().unwrap());
    if result.is_err() {
      server.take();
    }
    result
  }
}

impl OdiffServer {
  /// Starts and verifies an ODiff v4.5.0 server.
  pub fn start(binary: &Path, diagnostic: &Path, timeout: Duration) -> Result<Self> {
    ensure!(!timeout.is_zero(), "ODiff startup timeout must be positive");
    verify_version(binary, timeout)?;
    let diagnostic = File::create(diagnostic).context("create ODiff diagnostic log")?;
    let mut child = Command::new(binary)
      .arg("--server")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::from(diagnostic))
      .spawn()
      .context("start ODiff server")?;
    let input = child.stdin.take().context("ODiff server omitted stdin")?;
    let output = child.stdout.take().context("ODiff server omitted stdout")?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
      for line in BufReader::new(output).lines() {
        if sender.send(line.unwrap_or_default()).is_err() {
          break;
        }
      }
    });
    let ready = receiver.recv_timeout(timeout);
    if !matches!(ready.as_deref(), Ok(r#"{"ready":true}"#)) {
      let _ = child.kill();
      let _ = child.wait();
      let ready = ready.context("ODiff server startup timed out")?;
      bail!("ODiff server did not become ready: {ready}");
    }
    Ok(Self {
      child,
      input: BufWriter::new(input),
      output: receiver,
      next_request_id: 1,
      failed: false,
    })
  }

  /// Compares exact-size PNGs and retains a red mask for every nonzero difference.
  pub fn compare(&mut self, request: ImageComparisonRequest<'_>) -> Result<ImageComparison> {
    ensure!(!self.failed, "ODiff server is unavailable");
    ensure!(
      !request.timeout.is_zero(),
      "ODiff comparison timeout must be positive"
    );
    let baseline = png_dimensions(request.baseline).context("read baseline PNG")?;
    let actual = png_dimensions(request.actual).context("read actual PNG")?;
    ensure!(baseline == actual, "PNG dimensions differ");
    let total_pixels = u64::from(actual.0) * u64::from(actual.1);
    if request.diff.exists() {
      fs::remove_file(request.diff).context("remove stale ODiff mask")?;
    }
    let request_id = self.next_request_id;
    self.next_request_id += 1;
    let threshold = Number::from_str(&request.settings.threshold)
      .context("comparison threshold is not a JSON decimal")?;
    ensure!(
      threshold
        .as_f64()
        .is_some_and(|value| (0.0..=1.0).contains(&value)),
      "comparison threshold is outside 0 through 1"
    );
    let command = json!({
      "requestId": request_id,
      "type": "file",
      "base": request.baseline,
      "compare": request.actual,
      "output": request.diff,
      "options": {
        "threshold": threshold,
        "antialiasing": request.settings.anti_alias,
        "failOnLayoutDiff": true,
        "outputDiffMask": true,
        "diffColor": "#ff0000"
      }
    });
    serde_json::to_writer(&mut self.input, &command)?;
    self.input.write_all(b"\n")?;
    self.input.flush()?;
    let response = match self.output.recv_timeout(request.timeout) {
      Ok(value) => value,
      Err(error) => {
        self.failed = true;
        let _ = self.child.kill();
        bail!("ODiff comparison failed: {error}");
      }
    };
    let response: OdiffResponse =
      serde_json::from_str(&response).context("decode ODiff response")?;
    ensure!(
      response.request_id == Some(request_id),
      "ODiff response identity mismatch"
    );
    if let Some(error) = response.error {
      bail!("ODiff comparison failed: {error}");
    }
    if response.matched == Some(true) {
      return Ok(ImageComparison {
        outcome: ComparisonOutcome::Passed {
          changed_pixels: 0,
          total_pixels,
          settings: request.settings,
        },
        diff: None,
      });
    }
    ensure!(
      response.reason.as_deref() == Some("pixel-diff"),
      "ODiff rejected image layout"
    );
    let changed_pixels = response.diff_count.context("ODiff omitted diffCount")?;
    ensure!(
      changed_pixels <= total_pixels,
      "ODiff returned an impossible diffCount"
    );
    let diff = image_file(request.diff).context("read ODiff red mask")?;
    ensure!(
      (diff.width, diff.height) == actual,
      "ODiff mask dimensions differ"
    );
    let passed = within_limit(
      changed_pixels,
      total_pixels,
      &request.settings.max_changed_percent,
    )?;
    let outcome = if passed {
      ComparisonOutcome::Passed {
        changed_pixels,
        total_pixels,
        settings: request.settings,
      }
    } else {
      ComparisonOutcome::Mismatch {
        changed_pixels,
        total_pixels,
        settings: request.settings,
        diff: diff.clone(),
      }
    };
    Ok(ImageComparison {
      outcome,
      diff: Some(diff),
    })
  }
}

impl Drop for OdiffServer {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OdiffResponse {
  request_id: Option<u64>,
  #[serde(rename = "match")]
  matched: Option<bool>,
  reason: Option<String>,
  diff_count: Option<u64>,
  error: Option<String>,
}

fn verify_version(binary: &Path, timeout: Duration) -> Result<()> {
  let mut child = Command::new(binary)
    .arg("--version")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .context("query ODiff version")?;
  let started = Instant::now();
  while child.try_wait()?.is_none() {
    if started.elapsed() >= timeout {
      let _ = child.kill();
      let _ = child.wait();
      bail!("ODiff version probe timed out");
    }
    thread::sleep(Duration::from_millis(5));
  }
  let output = child.wait_with_output()?;
  ensure!(output.status.success(), "ODiff version probe failed");
  let text = format!(
    "{}{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
  ensure!(text.contains("4.5.0"), "ODiff is not version 4.5.0");
  Ok(())
}

fn png_dimensions(path: &Path) -> Result<(u32, u32)> {
  let bytes = fs::read(path)?;
  ensure!(bytes.len() >= 24, "PNG is truncated");
  ensure!(&bytes[..8] == PNG_SIGNATURE, "PNG signature is invalid");
  ensure!(&bytes[12..16] == b"IHDR", "PNG omits its IHDR chunk");
  let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
  let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
  ensure!(width > 0 && height > 0, "PNG dimensions are empty");
  Ok((width, height))
}

fn image_file(path: &Path) -> Result<ImageFile> {
  let bytes = fs::read(path)?;
  let (width, height) = png_dimensions(path)?;
  Ok(ImageFile {
    path: path.to_string_lossy().into_owned(),
    sha256: format!("{:x}", Sha256::digest(bytes)),
    width,
    height,
  })
}

fn within_limit(changed: u64, total: u64, percent: &str) -> Result<bool> {
  let (whole, fraction) = percent.split_once('.').unwrap_or((percent, ""));
  ensure!(
    !whole.is_empty()
      && whole.bytes().all(|value| value.is_ascii_digit())
      && fraction.bytes().all(|value| value.is_ascii_digit()),
    "changed-pixel percentage is invalid"
  );
  let scale = 10_u128
    .checked_pow(fraction.len().try_into()?)
    .context("changed-pixel percentage is too precise")?;
  let fractional = if fraction.is_empty() {
    0
  } else {
    fraction.parse::<u128>()?
  };
  let numerator = whole
    .parse::<u128>()?
    .checked_mul(scale)
    .and_then(|value| value.checked_add(fractional))
    .context("changed-pixel percentage overflow")?;
  ensure!(
    numerator <= 100 * scale,
    "changed-pixel percentage exceeds 100"
  );
  let changed_side = u128::from(changed)
    .checked_mul(100)
    .and_then(|value| value.checked_mul(scale))
    .context("changed-pixel comparison overflow")?;
  let total_side = u128::from(total)
    .checked_mul(numerator)
    .context("changed-pixel comparison overflow")?;
  Ok(changed_side <= total_side)
}
